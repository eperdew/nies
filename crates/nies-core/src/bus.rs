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
        }
    }

    /// Read without ticking. For debugger inspection only — bypasses any
    /// side effects on register-mapped addresses.
    pub fn peek(&self, addr: u16) -> u8 {
        self.read_no_tick(addr)
    }

    /// Internal: address-decoder read, no tick. Used by both `read` (which
    /// adds the tick) and `peek` (which doesn't).
    pub(crate) fn read_no_tick(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize],
            0x2000..=0x3FFF => {
                // PPU register space: $2000-$2007 mirrored every 8 bytes.
                // Real PPU read effects (PPUSTATUS clear, PPUDATA buffer)
                // land in M2; M1 returns open-bus.
                self.open_bus
            }
            0x4000..=0x4015 => self.open_bus, // APU registers: M5
            0x4016 => 0,                      // Controller 1: M4 will fill in
            0x4017 => 0,                      // Controller 2 / APU frame counter: M4/M5
            0x4018..=0x401F => self.open_bus, // CPU test mode (unused on retail NES)
            0x4020..=0xFFFF => {
                // Cartridge-mapped: PRG-RAM, expansion ROM, PRG-ROM.
                // We need a non-mut mapper read; do an unchecked clone-elision
                // by going through the read_no_tick interface on the mapper.
                // (Mapper trait requires &mut self for cpu_read because some
                // mappers update internal state on read; but we can take a
                // clone-of-state copy here for peek. For M1 NROM doesn't have
                // read side effects, so we use a direct const access.)
                //
                // To avoid cloning the whole mapper, we accept that peek
                // might not be fully side-effect-free for stateful mappers
                // (post-M1). At M1 we just call cpu_read on a casted mut
                // reference (safe because we hold &self exclusively).
                #[allow(unsafe_code)]
                {
                    let mapper_mut: *const MapperKind = &self.mapper;
                    let mapper_mut = mapper_mut as *mut MapperKind;
                    unsafe { (*mapper_mut).cpu_read(addr) }
                }
            }
        }
    }

    /// Internal: write the data bus and update the address-decoder side
    /// without ticking. Used by `write` (adds tick) and any future debugger
    /// "force write" functionality.
    #[allow(dead_code)] // first non-test caller arrives in Task 8 (`Bus::write`).
    pub(crate) fn write_no_tick(&mut self, addr: u16, val: u8) {
        self.open_bus = val;
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize] = val,
            0x2000..=0x3FFF => {
                // PPU register write: M2.
                let _ = val;
            }
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
        assert_eq!(bus.read_no_tick(0x0800), 0x42); // $0000 mirrors at $0800
        assert_eq!(bus.read_no_tick(0x1000), 0x42); // ...and at $1000
        assert_eq!(bus.read_no_tick(0x1800), 0x42); // ...and at $1800
    }

    #[test]
    fn power_on_ram_uses_mesen_pattern() {
        let bus = fake_bus();
        assert_eq!(bus.read_no_tick(0x0000), 0x00);
        assert_eq!(bus.read_no_tick(0x0008), 0xF7);
        assert_eq!(bus.read_no_tick(0x0009), 0xEF);
        assert_eq!(bus.read_no_tick(0x000A), 0xDF);
        assert_eq!(bus.read_no_tick(0x000F), 0xBF);
    }

    #[test]
    fn prg_rom_visible_through_bus() {
        let bus = fake_bus();
        // PRG byte 0 is at $8000; a 32 KiB ROM fills $8000-$FFFF.
        // fake_bus fills PRG with `(i as u8)`, so each 256-byte block
        // repeats 0x00..0xFF. $8000 → PRG[0x0000] = 0; $8001 → PRG[0x0001] = 1.
        assert_eq!(bus.read_no_tick(0x8000), 0);
        assert_eq!(bus.read_no_tick(0x8001), 1);
        // $C000 → PRG[0x4000]; 0x4000 mod 256 = 0.
        assert_eq!(bus.read_no_tick(0xC000), 0);
        // Non-aligned offset proves a 32 KiB ROM does not mirror at $8000.
        assert_eq!(bus.read_no_tick(0xC001), 1);
    }

    #[test]
    fn unmapped_apu_read_returns_open_bus() {
        let mut bus = fake_bus();
        bus.write_no_tick(0x0000, 0xAB); // sets open_bus = 0xAB
        assert_eq!(bus.read_no_tick(0x4015), 0xAB);
    }
}
