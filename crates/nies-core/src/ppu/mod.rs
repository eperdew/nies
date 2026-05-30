//! PPU — Picture Processing Unit (RP2C02 NTSC variant).
//!
//! Per-dot state machine called from `Bus::tick` 3 times per CPU cycle.
//! Module layout per the M2 design spec §2:
//! - state.rs: dot/scanline counters, frame parity
//! - registers.rs (Task 4+): PPUCTRL/MASK/STATUS/etc. + Loopy v/t/x/w
//! - vram.rs (Task 10): 2KB nametable RAM + mirroring
//! - oam.rs (Task 11): 256B primary OAM + 32B secondary OAM
//! - palette.rs (Task 12): 32-byte palette RAM with $3F1x mirrors
//! - background.rs (Task 26+): 8-cycle fetch pipeline
//! - sprite.rs (Task 39+): sprite eval, fetch, sprite-0 hit

pub mod oam;
pub mod registers;
pub mod state;
pub mod vram;

use crate::mapper::MapperKind;
use oam::Oam;
use registers::Registers;
use serde::{Deserialize, Serialize};
use state::PpuState;
use vram::Vram;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ppu {
    pub state: PpuState,
    pub regs: Registers,
    pub vram: Vram,
    pub oam: Oam,
}

impl Ppu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the PPU by one dot. M2 unit 1: just advances the counter.
    /// `_mapper` will be used by later tasks for CHR access and notify_a12.
    pub fn step(&mut self, _mapper: &mut MapperKind) {
        let rendering_enabled = self.regs.rendering_enabled();
        self.state.advance_dot_with_rendering(rendering_enabled);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, Mirroring, NesFormat};

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
    fn step_advances_one_dot() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        ppu.step(&mut mapper);
        assert_eq!(ppu.state.dot, 1);
        assert_eq!(ppu.state.scanline, 0);
    }
}
