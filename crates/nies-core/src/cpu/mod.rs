//! 6502 CPU implementation. See spec §3.4.

pub mod addressing;
pub mod flags;
pub mod instructions;

use crate::bus::Bus;

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
    pub fn reset(&mut self, bus: &mut Bus) {
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
    pub fn step(&mut self, bus: &mut Bus) {
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
        let handler = instructions::OPCODES[opcode as usize];
        handler(self, bus);
    }

    fn service_nmi(&mut self, _bus: &mut Bus) {
        // Filled in by Task 38.
        unimplemented!("NMI service in Task 38");
    }

    fn service_irq(&mut self, _bus: &mut Bus) {
        // Filled in by Task 39.
        unimplemented!("IRQ service in Task 39");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::cartridge::{Cartridge, Mirroring, NesFormat};
    use crate::mapper::MapperKind;

    fn bus_with_reset_vector(vector: u16) -> Bus {
        let mut prg = vec![0u8; 32 * 1024];
        // Reset vector lives at $FFFC-$FFFD; with 32 KiB PRG mapped to
        // $8000-$FFFF that's prg[0x7FFC..0x7FFE].
        prg[0x7FFC] = (vector & 0xFF) as u8;
        prg[0x7FFD] = (vector >> 8) as u8;
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
}
