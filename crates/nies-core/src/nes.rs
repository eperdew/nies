//! `Nes` — top-level emulator driver (CPU + Bus), and the embedded demo ROM.
//!
//! Pure logic: honors the crate's no-I/O contract. `include_bytes!` is a
//! compile-time data embed, not runtime I/O.

use crate::bus::Bus;
use crate::cartridge::{Cartridge, CartridgeError};
use crate::cpu::Cpu;
use crate::mapper::MapperKind;

/// Top-level NES driver: owns the CPU and the bus (which owns PPU, APU,
/// mapper, RAM). The single entry point a frontend uses to run the
/// emulator. No rendering/audio/input here — those belong to later
/// milestones; M3 needs only "run a frame, give me the framebuffer".
pub struct Nes {
    // cpu is used in run_frame (Task 3); suppress until then.
    #[allow(dead_code)]
    cpu: Cpu,
    bus: Bus,
}

impl Nes {
    /// Parse an iNES / NES 2.0 image, build the mapper and bus, and run
    /// the CPU reset sequence. Returns the cartridge parse/mapper error
    /// on malformed or unsupported ROMs.
    pub fn from_rom_bytes(bytes: &[u8]) -> Result<Self, CartridgeError> {
        let cart = Cartridge::from_bytes(bytes)?;
        let mapper = MapperKind::from_cartridge(&cart)?;
        let mut bus = Bus::new(mapper);
        let mut cpu = Cpu::new();
        cpu.reset(&mut bus);
        Ok(Self { cpu, bus })
    }

    /// The current frame's palette-index framebuffer (one byte per pixel,
    /// value 0..=63), row-major, 256×240.
    pub fn frame(&self) -> &[u8; 256 * 240] {
        self.bus.ppu.frame()
    }

    /// Total frames completed since power-on (monotonic).
    pub fn frame_count(&self) -> u64 {
        self.bus.ppu.frames()
    }
}

/// Bytes of the bundled `nmi_sync/demo_ntsc.nes` test ROM. Single source
/// shared by both frontends and the golden-hash tests (spec §5.3).
pub fn demo_rom_bytes() -> &'static [u8] {
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/roms/nmi_sync/demo_ntsc.nes"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::Cartridge;

    #[test]
    fn demo_rom_parses_as_cartridge() {
        let bytes = demo_rom_bytes();
        assert!(
            bytes.len() > 16,
            "demo ROM should be larger than an iNES header"
        );
        Cartridge::from_bytes(bytes).expect("demo ROM parses as a cartridge");
    }

    #[test]
    fn new_nes_has_zero_frames_and_full_framebuffer() {
        let nes = Nes::from_rom_bytes(demo_rom_bytes()).expect("build Nes");
        assert_eq!(nes.frame_count(), 0);
        assert_eq!(nes.frame().len(), 256 * 240);
    }

    #[test]
    fn from_rom_bytes_rejects_garbage() {
        assert!(Nes::from_rom_bytes(&[0, 1, 2, 3]).is_err());
    }
}
