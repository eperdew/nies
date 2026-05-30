//! CPU bus. Exposes `Bus::read` and `Bus::write`, both of which tick the
//! rest of the system (PPU/APU/mapper) one CPU cycle on every access.
//! See spec §3.3.

use crate::apu::Apu;
use crate::input::Controller;
use crate::mapper::{MapperImpl, MapperKind};
use crate::ppu::Ppu;

/// 2 KiB of CPU RAM, mirrored across $0000-$1FFF.
pub const CPU_RAM_BYTES: usize = 2048;

#[derive(Debug, Clone)]
pub struct Bus {
    pub ram: [u8; CPU_RAM_BYTES],
    pub ppu: Ppu,
    pub apu: Apu,
    pub mapper: MapperKind,
    pub controllers: [Controller; 2],
    /// Master CPU cycle counter since power-on.
    pub cycle: u64,
    /// Open-bus latch: last value transferred on the CPU data bus.
    /// Used as the read result for unmapped addresses (a real-hardware
    /// quirk that some test ROMs depend on).
    pub open_bus: u8,
    /// Pending NMI edge latched from the PPU. Drained by the CPU via
    /// `take_pending_nmi()` at the next instruction boundary.
    pub pending_nmi: bool,
}

impl Bus {
    pub fn new(mapper: MapperKind) -> Self {
        // Power-on RAM pattern: spec §4.2 says we use the Mesen pattern
        // ($00 except $0008/$0009/$000A/$000F set to $F7/$EF/$DF/$BF).
        let mut ram = [0u8; CPU_RAM_BYTES];
        ram[0x0008] = 0xF7;
        ram[0x0009] = 0xEF;
        ram[0x000A] = 0xDF;
        ram[0x000F] = 0xBF;
        Bus {
            ram,
            ppu: Ppu::new(),
            apu: Apu::new(),
            mapper,
            controllers: [Controller::new(), Controller::new()],
            cycle: 0,
            open_bus: 0,
            pending_nmi: false,
        }
    }

    /// Read without ticking and without side effects. For debugger
    /// inspection (M9). Register-mapped addresses ($2000-$3FFF PPU,
    /// $4000-$401F APU + I/O) return open-bus instead of triggering real
    /// hardware reads. Stateful-mapper read side effects are skipped via
    /// `MapperImpl::cpu_peek`.
    pub fn peek(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize],
            0x2000..=0x3FFF => self.open_bus,
            0x4000..=0x4015 => self.open_bus,
            0x4016 => 0,
            0x4017 => 0,
            0x4018..=0x401F => self.open_bus,
            0x4020..=0xFFFF => self.mapper.cpu_peek(addr),
        }
    }

    /// Tick the rest of the system one CPU cycle. Called from every public
    /// `read`/`write`. See spec §3.3.
    fn tick(&mut self) {
        // 3 PPU dots per CPU cycle (NTSC).
        for _ in 0..3 {
            self.ppu.step(&mut self.mapper);
            if self.ppu.take_nmi() {
                self.pending_nmi = true;
            }
        }
        // 1 APU step per CPU cycle.
        self.apu.step(&mut self.mapper);
        self.cycle = self.cycle.wrapping_add(1);
        // Service any pending DMC fetch. The fetch performs a no-tick read
        // from CPU memory, delivers the sample to DMC, and adds the
        // configured stall by recursively ticking. M1 DMC is always idle
        // (pending_fetch is None), so the body is unreachable at M1; this
        // is here so M5's DMC code can land without bus changes.
        if let Some(addr) = self.apu.dmc.take_pending_fetch() {
            let val = self.read_no_tick(addr);
            self.apu.dmc.deliver_sample(val);
            self.stall(self.apu.dmc.stall_cycles());
        }
    }

    fn stall(&mut self, cycles: u32) {
        for _ in 0..cycles {
            for _ in 0..3 {
                self.ppu.step(&mut self.mapper);
                if self.ppu.take_nmi() {
                    self.pending_nmi = true;
                }
            }
            self.apu.step(&mut self.mapper);
            self.cycle = self.cycle.wrapping_add(1);
        }
    }

    /// Drain the bus-level pending NMI latch. Called by the CPU at the
    /// next instruction boundary to learn that the PPU has raised an
    /// NMI edge since the last poll.
    pub fn take_pending_nmi(&mut self) -> bool {
        let v = self.pending_nmi;
        self.pending_nmi = false;
        v
    }

    /// Read a byte from the CPU bus. Ticks the system one CPU cycle.
    /// May trigger mapper read side effects (e.g., MMC5 expansion-RAM reads
    /// in future milestones); use `peek` for side-effect-free inspection.
    pub fn read(&mut self, addr: u16) -> u8 {
        self.tick();
        let val = self.read_no_tick(addr);
        self.open_bus = val;
        val
    }

    /// Internal: address-decoder read, no tick. Same side-effect profile as
    /// `read` (mapper reads may mutate state) — distinct from `peek`. Used
    /// by `read` (which adds the tick) and by the DMC fetch path inside
    /// `tick` (which has already ticked the cycle that triggered the fetch).
    pub(crate) fn read_no_tick(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize],
            0x2000..=0x3FFF => self.ppu.cpu_read(&mut self.mapper, addr),
            0x4000..=0x4015 => self.open_bus, // APU registers: M5
            0x4016 => 0,                      // Controller 1: M4 will fill in
            0x4017 => 0,                      // Controller 2 / APU frame counter: M4/M5
            0x4018..=0x401F => self.open_bus, // CPU test mode (unused on retail NES)
            0x4020..=0xFFFF => self.mapper.cpu_read(addr),
        }
    }

    /// Write a byte to the CPU bus. Ticks the system one CPU cycle.
    pub fn write(&mut self, addr: u16, val: u8) {
        self.tick();
        self.write_no_tick(addr, val);
    }

    /// Internal: write the data bus and update the address-decoder side
    /// without ticking. Used by `write` (adds tick) and any future debugger
    /// "force write" functionality.
    pub(crate) fn write_no_tick(&mut self, addr: u16, val: u8) {
        self.open_bus = val;
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize] = val,
            0x2000..=0x3FFF => self.ppu.cpu_write(&mut self.mapper, addr, val),
            0x4000..=0x4013 | 0x4015 => {
                // APU register write: M5.
                let _ = val;
            }
            0x4014 => {
                // OAMDMA: M2/M3 will implement the 256-byte transfer.
                let _ = val;
            }
            0x4016 => {
                self.controllers[0].write_strobe(val);
                self.controllers[1].write_strobe(val);
            }
            0x4017 => {
                // APU frame counter: M5.
                let _ = val;
            }
            0x4018..=0x401F => {} // CPU test mode (unused)
            0x4020..=0xFFFF => self.mapper.cpu_write(addr, val),
        }
    }
}

/// Bus interface required by the CPU's dispatch table.
///
/// **This trait exists for one reason: the SingleStepTests/65x02
/// integration tests.** Those tests operate against a 64 KiB flat memory
/// model with no PPU / APU / mapper structure, so they need a different
/// concrete bus type than production code uses. Parameterizing the CPU
/// over `BusLike` lets us re-use the production opcode handlers verbatim
/// against the `FlatBus` test harness in `crates/nies-core/tests/singlestep_tests.rs`,
/// instead of maintaining a parallel CPU implementation just for tests.
///
/// Production runtime code (the binaries, save states, debugger, etc.)
/// always uses the concrete `Bus`. There is no expected use case where a
/// non-test caller wants to swap in their own `BusLike` impl — if you
/// find yourself reaching for one, talk to the spec first.
///
/// The cost of the abstraction: opcode dispatch is a `match` over `u8`
/// rather than a static `[fn(...); 256]` table, because Rust generic
/// function pointers can't live in a static. LLVM compiles the match to
/// a jump table at the same cost as the function pointer table would have.
pub trait BusLike {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
    /// True when the mapper has asserted IRQ. Production: forwards to
    /// `mapper.irq_pending()`. Tests: always false.
    fn mapper_irq_pending(&self) -> bool {
        false
    }
    /// Drain any pending NMI edge raised by the PPU since the last poll.
    /// Production: forwards to `Bus::take_pending_nmi`. Tests: always
    /// false (SingleStepTests/FlatBus has no PPU).
    fn take_pending_nmi(&mut self) -> bool {
        false
    }
}

impl BusLike for Bus {
    fn read(&mut self, addr: u16) -> u8 {
        Bus::read(self, addr)
    }
    fn write(&mut self, addr: u16, val: u8) {
        Bus::write(self, addr, val)
    }
    fn mapper_irq_pending(&self) -> bool {
        use crate::mapper::MapperImpl;
        self.mapper.irq_pending()
    }
    fn take_pending_nmi(&mut self) -> bool {
        Bus::take_pending_nmi(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, Mirroring, NesFormat};

    fn fake_bus() -> Bus {
        let cart = Cartridge {
            format: NesFormat::INes,
            mapper_id: 0,
            submapper_id: 0,
            mirroring: Mirroring::Horizontal,
            has_battery: false,
            has_trainer: false,
            prg_rom: (0..(32 * 1024) as u32).map(|i| i as u8).collect(),
            chr_rom: vec![0; 8 * 1024],
            prg_ram_size: 8 * 1024,
            chr_ram_size: 0,
        };
        Bus::new(MapperKind::from_cartridge(&cart).unwrap())
    }

    #[test]
    fn ram_mirroring() {
        let mut bus = fake_bus();
        bus.write_no_tick(0x0000, 0x42);
        assert_eq!(bus.peek(0x0800), 0x42); // $0000 mirrors at $0800
        assert_eq!(bus.peek(0x1000), 0x42); // ...and at $1000
        assert_eq!(bus.peek(0x1800), 0x42); // ...and at $1800
    }

    #[test]
    fn power_on_ram_uses_mesen_pattern() {
        let bus = fake_bus();
        assert_eq!(bus.peek(0x0000), 0x00);
        assert_eq!(bus.peek(0x0008), 0xF7);
        assert_eq!(bus.peek(0x0009), 0xEF);
        assert_eq!(bus.peek(0x000A), 0xDF);
        assert_eq!(bus.peek(0x000F), 0xBF);
    }

    #[test]
    fn prg_rom_visible_through_bus() {
        let bus = fake_bus();
        // PRG byte 0 is at $8000; a 32 KiB ROM fills $8000-$FFFF.
        // fake_bus fills PRG with `(i as u8)`, so each 256-byte block
        // repeats 0x00..0xFF. $8000 → PRG[0x0000] = 0; $8001 → PRG[0x0001] = 1.
        assert_eq!(bus.peek(0x8000), 0);
        assert_eq!(bus.peek(0x8001), 1);
        // $C000 → PRG[0x4000]; 0x4000 mod 256 = 0.
        assert_eq!(bus.peek(0xC000), 0);
        // Non-aligned offset proves a 32 KiB ROM does not mirror at $8000.
        assert_eq!(bus.peek(0xC001), 1);
    }

    #[test]
    fn unmapped_apu_read_returns_open_bus() {
        let mut bus = fake_bus();
        bus.write_no_tick(0x0000, 0xAB); // sets open_bus = 0xAB
        assert_eq!(bus.peek(0x4015), 0xAB);
    }

    #[test]
    fn read_advances_cycle_counter() {
        let mut bus = fake_bus();
        let cycle_before = bus.cycle;
        let _ = bus.read(0x0000);
        assert_eq!(bus.cycle, cycle_before + 1);
    }

    #[test]
    fn write_advances_cycle_counter() {
        let mut bus = fake_bus();
        let cycle_before = bus.cycle;
        bus.write(0x0000, 0x42);
        assert_eq!(bus.cycle, cycle_before + 1);
    }

    #[test]
    fn read_advances_ppu_three_dots() {
        let mut bus = fake_bus();
        let dots_before = bus.ppu.state.dot;
        let _ = bus.read(0x0000);
        assert_eq!(bus.ppu.state.dot, dots_before + 3);
    }

    #[test]
    fn read_advances_apu_one_cycle() {
        let mut bus = fake_bus();
        let apu_cycles_before = bus.apu.cycles;
        let _ = bus.read(0x0000);
        assert_eq!(bus.apu.cycles, apu_cycles_before + 1);
    }

    #[test]
    fn peek_does_not_tick() {
        let bus = fake_bus();
        let cycle_before = bus.cycle;
        let _ = bus.peek(0x0000);
        // peek takes &self; cycle_before still valid from before
        assert_eq!(bus.cycle, cycle_before);
    }
}
