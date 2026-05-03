//! APU stub. Real implementation lands in M5 (spec §3.6).
//!
//! At M1 this is a counter that records CPU-cycle steps. The DMC
//! sub-module exposes a no-op `pending_fetch` API that `Bus::tick`
//! calls into; the bus is the same shape it will be at M5.

pub mod dmc;

use crate::mapper::MapperKind;
use dmc::DmcChannel;

#[derive(Debug, Clone, Default)]
pub struct Apu {
    /// CPU cycles elapsed since power-on. M5 will replace this with full
    /// state: pulse/triangle/noise/DMC channels, frame counter, mixer.
    pub cycles: u64,
    pub dmc: DmcChannel,
}

impl Apu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn step(&mut self, _mapper: &mut MapperKind) {
        self.cycles = self.cycles.wrapping_add(1);
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
    fn step_increments_cycle_counter() {
        let mut apu = Apu::new();
        let mut mapper = fake_mapper();
        for _ in 0..50 {
            apu.step(&mut mapper);
        }
        assert_eq!(apu.cycles, 50);
        assert!(apu.dmc.take_pending_fetch().is_none());
    }
}
