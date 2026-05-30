//! 2KB nametable RAM owned by the PPU + mirroring helper.
//!
//! The cartridge tells the PPU how the 4KB virtual space ($2000-$2FFF)
//! maps onto the physical 2KB. Four-screen mirroring (cartridge supplies
//! extra RAM) is post-v1.

use crate::cartridge::Mirroring;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

const VRAM_SIZE: usize = 2 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vram {
    #[serde(with = "BigArray")]
    pub bytes: [u8; VRAM_SIZE],
}

impl Default for Vram {
    fn default() -> Self {
        Self {
            bytes: [0; VRAM_SIZE],
        }
    }
}

impl Vram {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&self, addr: u16, mirroring: Mirroring) -> u8 {
        self.bytes[nametable_index(addr, mirroring)]
    }

    pub fn write(&mut self, addr: u16, val: u8, mirroring: Mirroring) {
        self.bytes[nametable_index(addr, mirroring)] = val;
    }
}

/// Map a PPU address in $2000-$3EFF into a 0..2048 byte index in the
/// 2KB nametable RAM, applying the cartridge's mirroring mode.
///
/// Panics on `FourScreen` — that mode requires cartridge-supplied extra
/// RAM and is post-v1 (see M2 design spec §3.3).
pub fn nametable_index(addr: u16, mirroring: Mirroring) -> usize {
    let offset = (addr & 0x0FFF) as usize;
    match mirroring {
        Mirroring::Vertical => offset & 0x7FF,
        Mirroring::Horizontal => ((offset & 0x800) >> 1) | (offset & 0x3FF),
        Mirroring::SingleScreen => offset & 0x3FF,
        Mirroring::FourScreen => {
            panic!("FourScreen mirroring not supported at M2 (post-v1)")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_maps_nt0_and_nt2_to_same_kb() {
        assert_eq!(nametable_index(0x2000, Mirroring::Vertical), 0);
        assert_eq!(nametable_index(0x2800, Mirroring::Vertical), 0);
        assert_eq!(nametable_index(0x2400, Mirroring::Vertical), 0x400);
        assert_eq!(nametable_index(0x2C00, Mirroring::Vertical), 0x400);
    }

    #[test]
    fn horizontal_maps_nt0_and_nt1_to_same_kb() {
        assert_eq!(nametable_index(0x2000, Mirroring::Horizontal), 0);
        assert_eq!(nametable_index(0x2400, Mirroring::Horizontal), 0);
        assert_eq!(nametable_index(0x2800, Mirroring::Horizontal), 0x400);
        assert_eq!(nametable_index(0x2C00, Mirroring::Horizontal), 0x400);
    }

    #[test]
    fn single_screen_maps_all_to_first_kb() {
        for nt in [0x2000, 0x2400, 0x2800, 0x2C00] {
            assert_eq!(nametable_index(nt, Mirroring::SingleScreen), 0);
        }
    }

    #[test]
    fn vram_read_after_write_round_trips() {
        let mut v = Vram::new();
        v.write(0x2055, 0xAB, Mirroring::Vertical);
        assert_eq!(v.read(0x2055, Mirroring::Vertical), 0xAB);
    }
}
