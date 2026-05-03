//! PPU stub. Real implementation lands in M2 (spec §3.5).
//!
//! At M1 this is just a counter that records the number of dots advanced,
//! so the bus can call `Ppu::step(&mut self, mapper)` from inside its
//! tick loop without panicking.

use crate::mapper::MapperKind;

#[derive(Debug, Clone, Default)]
pub struct Ppu {
    /// PPU dots elapsed since power-on. M2 will replace this with full
    /// state: scanline, dot, register file, OAM, etc.
    pub dots: u64,
}

impl Ppu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the PPU by one dot. M1 stub: just increments the counter.
    /// `_mapper` is unused at M1 but reserved for M2's A12 hook.
    pub fn step(&mut self, _mapper: &mut MapperKind) {
        self.dots = self.dots.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, Mirroring, NesFormat};
    use crate::mapper::MapperKind;

    fn fake_mapper() -> MapperKind {
        let cart = Cartridge {
            format: NesFormat::INes,
            mapper_id: 0,
            submapper_id: 0,
            mirroring: Mirroring::Horizontal,
            has_battery: false,
            has_trainer: false,
            prg_rom: vec![0; 16 * 1024],
            chr_rom: vec![0; 8 * 1024],
            prg_ram_size: 0,
            chr_ram_size: 0,
        };
        MapperKind::from_cartridge(&cart).unwrap()
    }

    #[test]
    fn step_increments_dot_counter() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        for _ in 0..100 {
            ppu.step(&mut mapper);
        }
        assert_eq!(ppu.dots, 100);
    }
}
