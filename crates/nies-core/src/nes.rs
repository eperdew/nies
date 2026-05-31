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

    /// Run the CPU until the PPU completes one frame. Executes whole
    /// instructions; the boundary is the first instruction that pushes
    /// the PPU frame counter over.
    ///
    /// Because we stop on an instruction boundary rather than mid-instruction,
    /// the terminating instruction may run a handful of PPU dots past the
    /// frame wrap, so up to a few pixels at the very start of row 0 can belong
    /// to frame N+1. This is bounded (≪ one scanline), fully deterministic,
    /// and captured by the golden hash; it is cosmetically invisible.
    pub fn run_frame(&mut self) {
        let target = self.bus.ppu.frames() + 1;
        while self.bus.ppu.frames() < target {
            self.cpu.step(&mut self.bus);
        }
    }

    /// Soft reset: re-run the CPU reset sequence. Does not rebuild the
    /// cartridge or clear the framebuffer.
    pub fn reset(&mut self) {
        self.cpu.reset(&mut self.bus);
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

    #[test]
    fn run_frame_advances_frame_count_by_one() {
        let mut nes = Nes::from_rom_bytes(demo_rom_bytes()).expect("build Nes");
        let before = nes.frame_count();
        nes.run_frame();
        assert_eq!(nes.frame_count(), before + 1);
    }

    #[test]
    fn run_frame_is_repeatable() {
        let mut nes = Nes::from_rom_bytes(demo_rom_bytes()).expect("build Nes");
        for _ in 0..10 {
            nes.run_frame();
        }
        assert_eq!(nes.frame_count(), 10);
    }
}
