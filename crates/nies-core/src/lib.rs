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
pub mod ppu;
pub mod snapshot;

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_smoke() {
        assert_eq!(2 + 2, 4);
    }
}
