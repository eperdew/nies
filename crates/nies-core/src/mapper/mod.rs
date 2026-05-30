//! Mapper trait + variant enum. NROM only at M1 (spec §3.7).
//!
//! Mappers are dispatched through the `MapperKind` enum rather than
//! `Box<dyn>`. This avoids vtable lookups, serializes cleanly via serde
//! (no `typetag` dependency), and makes the closed set of supported
//! mappers explicit at the type level.

pub mod nrom;

use crate::cartridge::{Cartridge, CartridgeError, Mirroring};
use nrom::NromState;

/// Per-mapper interface. Spec §3.7.
pub trait MapperImpl {
    /// CPU bus read with side effects. May mutate internal mapper state
    /// (for stateful mappers like MMC5 with read-triggered registers).
    fn cpu_read(&mut self, addr: u16) -> u8;
    /// Side-effect-free CPU bus read. Used by `Bus::peek` for debugger
    /// inspection. Stateful mappers must implement this to skip any
    /// read-triggered state changes; for read-side-effect-free mappers
    /// (NROM, etc.) the implementation is the same as `cpu_read`.
    fn cpu_peek(&self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, val: u8);
    fn ppu_read(&mut self, addr: u16) -> u8;
    /// Side-effect-free PPU bus read. Same intent as `cpu_peek` but on
    /// the PPU bus side; will be exercised by the M9 PPU debugger panels.
    fn ppu_peek(&self, addr: u16) -> u8;
    fn ppu_write(&mut self, addr: u16, val: u8);
    /// Per-cycle PPU A12 line level. Used by MMC3-class mappers; default
    /// no-op for mappers that don't track A12 transitions.
    fn notify_a12(&mut self, _level: bool) {}
    /// True if the mapper has an asserted IRQ line (e.g., MMC3 scanline counter).
    fn irq_pending(&self) -> bool {
        false
    }
    /// Current mirroring mode (some mappers can change this dynamically).
    fn mirroring(&self) -> Mirroring;
    /// Debug introspection. Default: empty list. Each mapper can return its
    /// internal register state for the debugger UI (M9).
    fn debug_dump(&self) -> Vec<(&'static str, u32)> {
        vec![]
    }
}

/// Closed set of supported mappers. Adding a new mapper means adding a
/// variant here and updating every `match` site in this file.
#[derive(Debug, Clone)]
pub enum MapperKind {
    Nrom(NromState),
}

impl MapperKind {
    /// Build a `MapperKind` from a parsed cartridge. Returns
    /// `CartridgeError::UnsupportedMapper` for any mapper id not yet implemented.
    pub fn from_cartridge(cart: &Cartridge) -> Result<Self, CartridgeError> {
        match cart.mapper_id {
            0 => Ok(MapperKind::Nrom(NromState::new(cart))),
            id => Err(CartridgeError::UnsupportedMapper(id)),
        }
    }
}

impl MapperImpl for MapperKind {
    fn cpu_read(&mut self, addr: u16) -> u8 {
        match self {
            MapperKind::Nrom(s) => s.cpu_read(addr),
        }
    }

    fn cpu_peek(&self, addr: u16) -> u8 {
        match self {
            MapperKind::Nrom(s) => s.cpu_peek(addr),
        }
    }

    fn cpu_write(&mut self, addr: u16, val: u8) {
        match self {
            MapperKind::Nrom(s) => s.cpu_write(addr, val),
        }
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        match self {
            MapperKind::Nrom(s) => s.ppu_read(addr),
        }
    }

    fn ppu_peek(&self, addr: u16) -> u8 {
        match self {
            MapperKind::Nrom(s) => s.ppu_peek(addr),
        }
    }

    fn ppu_write(&mut self, addr: u16, val: u8) {
        match self {
            MapperKind::Nrom(s) => s.ppu_write(addr, val),
        }
    }

    fn mirroring(&self) -> Mirroring {
        match self {
            MapperKind::Nrom(s) => s.mirroring(),
        }
    }

    fn notify_a12(&mut self, level: bool) {
        match self {
            MapperKind::Nrom(s) => s.notify_a12(level),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, NesFormat};

    fn fake_nrom_cart() -> Cartridge {
        Cartridge {
            format: NesFormat::INes,
            mapper_id: 0,
            submapper_id: 0,
            mirroring: Mirroring::Horizontal,
            has_battery: false,
            has_trainer: false,
            prg_rom: vec![0xAA; 16 * 1024],
            chr_rom: vec![0xBB; 8 * 1024],
            prg_ram_size: 8 * 1024,
            chr_ram_size: 0,
        }
    }

    #[test]
    fn dispatches_to_nrom() {
        let cart = fake_nrom_cart();
        let mut mapper = MapperKind::from_cartridge(&cart).unwrap();
        assert_eq!(mapper.cpu_read(0x8000), 0xAA);
        assert_eq!(mapper.ppu_read(0x0000), 0xBB);
        assert_eq!(mapper.mirroring(), Mirroring::Horizontal);
        assert!(!mapper.irq_pending());
    }

    #[test]
    fn rejects_unsupported_mapper() {
        let mut cart = fake_nrom_cart();
        cart.mapper_id = 4; // MMC3 — not implemented at M1
        let err = MapperKind::from_cartridge(&cart).unwrap_err();
        assert_eq!(err, CartridgeError::UnsupportedMapper(4));
    }
}
