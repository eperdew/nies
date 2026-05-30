//! 6502 CPU implementation. See spec §3.4.

pub mod addressing;
pub mod flags;
pub mod instructions;

/// 6502 CPU state.
#[derive(Debug, Clone, Copy)]
pub struct Cpu {
    pub a: u8,   // accumulator
    pub x: u8,   // X index
    pub y: u8,   // Y index
    pub pc: u16, // program counter
    pub sp: u8,  // stack pointer
    pub p: u8,   // status flags
    /// True when the CPU is halted by a JAM/KIL/HLT illegal opcode.
    pub jammed: bool,
    /// Pending NMI: latched when the NMI line is pulled low; serviced
    /// at the next instruction boundary. Set by the PPU; cleared after
    /// the NMI handler entry.
    pub nmi_pending: bool,
    /// Pending IRQ: level-sensitive (asserted while any IRQ source holds
    /// the line low). Sampled at instruction boundaries when I flag is clear.
    pub irq_pending: bool,
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: 0xFD,
            p: 0x34, // I=1, U=1, B=1 (the "B flag" bit in P is always set when read directly)
            jammed: false,
            nmi_pending: false,
            irq_pending: false,
        }
    }
}

impl Cpu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize CPU state per spec: A=X=Y=0, S=$FD, P=$34,
    /// PC=read_word(reset_vector $FFFC). Each of the two reset reads ticks
    /// the bus.
    pub fn reset<B: crate::bus::BusLike>(&mut self, bus: &mut B) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.p = 0x34;
        self.jammed = false;
        self.nmi_pending = false;
        self.irq_pending = false;
        let lo = bus.read(0xFFFC) as u16;
        let hi = bus.read(0xFFFD) as u16;
        self.pc = (hi << 8) | lo;
    }

    /// Execute one CPU instruction. Handles pending NMI/IRQ at the
    /// instruction boundary before fetching the next opcode.
    pub fn step<B: crate::bus::BusLike>(&mut self, bus: &mut B) {
        // Drain any PPU NMI edge raised since the last instruction boundary.
        if bus.take_pending_nmi() {
            self.nmi_pending = true;
        }

        if self.jammed {
            // KIL/JAM/HLT halts the CPU until reset; just keep ticking
            // the bus so PPU/APU continue running.
            let _ = bus.read(self.pc);
            return;
        }

        // Interrupt servicing happens at instruction boundaries.
        if self.nmi_pending {
            self.service_nmi(bus);
            return;
        }
        if self.irq_pending && (self.p & flags::FLAG_I) == 0 {
            self.service_irq(bus);
            return;
        }

        let opcode = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        instructions::dispatch(opcode, self, bus);
    }

    /// Push a byte onto the stack at $0100 + S, then decrement S.
    fn push<B: crate::bus::BusLike>(&mut self, bus: &mut B, val: u8) {
        bus.write(0x0100 | self.sp as u16, val);
        self.sp = self.sp.wrapping_sub(1);
    }

    /// Shared interrupt-entry sequence used by both NMI and IRQ.
    /// 7 bus cycles: two dummy reads of PC, push PCH/PCL/(P with B clear
    /// and U set), set the I flag, then read the two-byte handler address
    /// from `vector` / `vector+1` and jump to it.
    fn service_interrupt<B: crate::bus::BusLike>(&mut self, bus: &mut B, vector: u16) {
        let _ = bus.read(self.pc);
        let _ = bus.read(self.pc);
        let pch = (self.pc >> 8) as u8;
        let pcl = (self.pc & 0xFF) as u8;
        self.push(bus, pch);
        self.push(bus, pcl);
        let p_to_push = (self.p & !flags::FLAG_B) | flags::FLAG_U;
        self.push(bus, p_to_push);
        self.p |= flags::FLAG_I;
        let lo = bus.read(vector) as u16;
        let hi = bus.read(vector.wrapping_add(1)) as u16;
        self.pc = (hi << 8) | lo;
    }

    /// Service a pending NMI: shared interrupt sequence with the NMI
    /// vector at $FFFA/$FFFB. NMI is edge-triggered, so `nmi_pending`
    /// is cleared on entry to the handler.
    fn service_nmi<B: crate::bus::BusLike>(&mut self, bus: &mut B) {
        self.service_interrupt(bus, 0xFFFA);
        self.nmi_pending = false;
    }

    /// Service a pending IRQ: shared interrupt sequence with the IRQ
    /// vector at $FFFE/$FFFF. IRQ is level-sensitive, so `irq_pending`
    /// is intentionally NOT cleared here — the IRQ source (e.g. APU
    /// frame counter, MMC3 scanline counter) is responsible for
    /// de-asserting the line once the handler acknowledges. The CPU
    /// just sets I to mask further IRQs until RTI restores P.
    fn service_irq<B: crate::bus::BusLike>(&mut self, bus: &mut B) {
        self.service_interrupt(bus, 0xFFFE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::cartridge::{Cartridge, Mirroring, NesFormat};
    use crate::mapper::MapperKind;

    fn bus_with_vectors(reset: Option<u16>, nmi: Option<u16>, irq: Option<u16>) -> Bus {
        let mut prg = vec![0u8; 32 * 1024];
        // With 32 KiB PRG mapped to $8000-$FFFF, the vectors at
        // $FFFA-$FFFF live at prg[0x7FFA..0x8000].
        if let Some(v) = nmi {
            prg[0x7FFA] = (v & 0xFF) as u8;
            prg[0x7FFB] = (v >> 8) as u8;
        }
        if let Some(v) = reset {
            prg[0x7FFC] = (v & 0xFF) as u8;
            prg[0x7FFD] = (v >> 8) as u8;
        }
        if let Some(v) = irq {
            prg[0x7FFE] = (v & 0xFF) as u8;
            prg[0x7FFF] = (v >> 8) as u8;
        }
        let cart = Cartridge {
            format: NesFormat::INes,
            mapper_id: 0,
            submapper_id: 0,
            mirroring: Mirroring::Horizontal,
            has_battery: false,
            has_trainer: false,
            prg_rom: prg,
            chr_rom: vec![0; 8 * 1024],
            prg_ram_size: 0,
            chr_ram_size: 0,
        };
        Bus::new(MapperKind::from_cartridge(&cart).unwrap())
    }

    fn bus_with_reset_vector(vector: u16) -> Bus {
        bus_with_vectors(Some(vector), None, None)
    }

    fn bus_with_nmi_vector(vector: u16) -> Bus {
        bus_with_vectors(None, Some(vector), None)
    }

    fn bus_with_irq_vector(vector: u16) -> Bus {
        bus_with_vectors(None, None, Some(vector))
    }

    #[test]
    fn reset_loads_pc_from_vector() {
        let mut bus = bus_with_reset_vector(0xC000);
        let mut cpu = Cpu::new();
        cpu.reset(&mut bus);
        assert_eq!(cpu.pc, 0xC000);
        assert_eq!(cpu.sp, 0xFD);
        assert_eq!(cpu.p, 0x34);
        assert_eq!(cpu.a, 0);
        assert_eq!(cpu.x, 0);
        assert_eq!(cpu.y, 0);
    }

    #[test]
    fn reset_consumes_two_bus_cycles() {
        let mut bus = bus_with_reset_vector(0x8000);
        let mut cpu = Cpu::new();
        let cycle_before = bus.cycle;
        cpu.reset(&mut bus);
        assert_eq!(bus.cycle, cycle_before + 2);
    }

    #[test]
    fn irq_pushes_and_jumps_when_i_clear() {
        let mut bus = bus_with_irq_vector(0xD000);
        let mut cpu = Cpu::new();
        cpu.pc = 0x4321;
        cpu.p = 0; // I clear
        cpu.sp = 0xFF;
        cpu.irq_pending = true;
        cpu.step(&mut bus);

        assert_eq!(cpu.pc, 0xD000);
        assert_eq!(cpu.sp, 0xFC); // pushed 3 bytes
        assert!(cpu.p & flags::FLAG_I != 0);
        // IRQ is level-sensitive: irq_pending is NOT cleared here.
        // The IRQ source is responsible for de-asserting.
        assert!(cpu.irq_pending);
        // Verify pushed values: stack grows down from sp=0xFF.
        assert_eq!(bus.peek(0x01FF), 0x43); // PCH
        assert_eq!(bus.peek(0x01FE), 0x21); // PCL
        let pushed_p = bus.peek(0x01FD);
        assert_eq!(pushed_p & flags::FLAG_B, 0); // B clear
        assert!(pushed_p & flags::FLAG_U != 0); // U set
    }

    #[test]
    fn irq_is_ignored_when_i_set() {
        let mut bus = bus_with_irq_vector(0xD000);
        let mut cpu = Cpu::new();
        // Place a NOP ($EA) into RAM at $0421 and point PC there so
        // step() will dispatch a real NOP rather than fall through to
        // BRK ($00) at zeroed RAM/ROM.
        bus.ram[0x0421] = 0xEA;
        cpu.pc = 0x0421;
        cpu.p = flags::FLAG_I; // I set: mask IRQs
        cpu.sp = 0xFF;
        cpu.irq_pending = true;
        cpu.step(&mut bus);

        // Step should have fetched and executed the NOP at $0421
        // rather than entering the IRQ handler at $D000.
        assert_ne!(cpu.pc, 0xD000);
        assert_eq!(cpu.pc, 0x0422);
        // Stack pointer is untouched since NOP doesn't push.
        assert_eq!(cpu.sp, 0xFF);
        // irq_pending stays asserted (level-sensitive, source clears).
        assert!(cpu.irq_pending);
    }

    #[test]
    fn nmi_pushes_pc_and_p_then_jumps_to_vector() {
        let mut bus = bus_with_nmi_vector(0xE000);
        let mut cpu = Cpu::new();
        cpu.pc = 0x1234;
        cpu.p = flags::FLAG_C | flags::FLAG_Z; // arbitrary, I clear
        cpu.sp = 0xFF;
        cpu.nmi_pending = true;
        cpu.step(&mut bus); // services NMI

        assert_eq!(cpu.pc, 0xE000);
        assert_eq!(cpu.sp, 0xFC); // pushed 3 bytes
        assert!(cpu.p & flags::FLAG_I != 0);
        assert!(!cpu.nmi_pending);
        // Verify pushed values: stack grows down from initial sp=0xFF.
        assert_eq!(bus.peek(0x01FF), 0x12); // PCH
        assert_eq!(bus.peek(0x01FE), 0x34); // PCL
        let pushed_p = bus.peek(0x01FD);
        assert_eq!(pushed_p & flags::FLAG_B, 0); // B clear
        assert!(pushed_p & flags::FLAG_U != 0); // U set
        // The original C and Z flags survived in the pushed P.
        assert!(pushed_p & flags::FLAG_C != 0);
        assert!(pushed_p & flags::FLAG_Z != 0);
    }

    #[test]
    fn reset_returns_to_vector_from_arbitrary_state() {
        let mut bus = bus_with_reset_vector(0xC000);
        let mut cpu = Cpu::new();
        // Put CPU in an arbitrary post-init state.
        cpu.a = 0xAB;
        cpu.x = 0xCD;
        cpu.y = 0xEF;
        cpu.pc = 0x1234;
        cpu.sp = 0x10;
        cpu.p = 0xFF;
        cpu.jammed = true;
        cpu.nmi_pending = true;
        cpu.irq_pending = true;

        cpu.reset(&mut bus);

        // Reset clears all the special states and reloads PC from $FFFC/D.
        assert_eq!(cpu.pc, 0xC000);
        assert_eq!(cpu.a, 0);
        assert_eq!(cpu.x, 0);
        assert_eq!(cpu.y, 0);
        assert_eq!(cpu.sp, 0xFD);
        assert_eq!(cpu.p, 0x34);
        assert!(!cpu.jammed);
        assert!(!cpu.nmi_pending);
        assert!(!cpu.irq_pending);
    }
}
