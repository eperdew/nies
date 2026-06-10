//! `nies-core` — NES emulator backend (CPU, PPU, APU, mappers, save state, debugger).
//!
//! No I/O dependencies: this crate must remain free of `std::fs`, `std::time::SystemTime`,
//! audio/video device access, and threading. The deterministic emulator core lives here.

pub mod apu;
pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod input;
pub mod mapper;
pub mod nes;
pub mod ppu;
pub mod snapshot;

// Convenience re-exports for the public API.
pub use bus::{Bus, BusLike};
pub use cartridge::{Cartridge, CartridgeError, Mirroring, NesFormat};
pub use cpu::Cpu;
pub use input::{Buttons, Controller, InputEvent};
pub use mapper::{MapperImpl, MapperKind};
pub use nes::{Nes, demo_rom_bytes};
pub use ppu::Ppu;

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_smoke() {
        assert_eq!(2 + 2, 4);
    }
}
