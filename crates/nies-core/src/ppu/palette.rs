//! 32-byte palette RAM with the $3F1x→$3F0x mirror semantics.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    pub bytes: [u8; 32],
}

impl Default for Palette {
    fn default() -> Self {
        // Power-on palette per Mesen. Tests don't depend on specific values.
        Self {
            bytes: [
                0x09, 0x01, 0x00, 0x01, 0x00, 0x02, 0x02, 0x0D, 0x08, 0x10, 0x08, 0x24, 0x00, 0x00,
                0x04, 0x2C, 0x09, 0x01, 0x34, 0x03, 0x00, 0x04, 0x00, 0x14, 0x08, 0x3A, 0x00, 0x02,
                0x00, 0x20, 0x2C, 0x08,
            ],
        }
    }
}

impl Palette {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.bytes[palette_index(addr)]
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        self.bytes[palette_index(addr)] = val;
    }

    /// Read with greyscale mask applied (PPUMASK bit 0): low 4 bits forced 0.
    pub fn read_masked(&self, addr: u16, greyscale: bool) -> u8 {
        let v = self.read(addr);
        if greyscale { v & 0x30 } else { v }
    }
}

/// Map a $3F00-$3FFF address to a 0..32 index, applying the $3F1x→$3F0x
/// mirror for the four "transparent" indices.
pub fn palette_index(addr: u16) -> usize {
    let mut idx = (addr & 0x1F) as usize;
    if idx & 0b1_0011 == 0b1_0000 {
        idx &= !0b1_0000;
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_address_mirrors_3f10_to_3f00() {
        assert_eq!(palette_index(0x3F10), 0x00);
        assert_eq!(palette_index(0x3F14), 0x04);
        assert_eq!(palette_index(0x3F18), 0x08);
        assert_eq!(palette_index(0x3F1C), 0x0C);
    }

    #[test]
    fn palette_address_does_not_mirror_3f01_3f02_3f03() {
        assert_eq!(palette_index(0x3F11), 0x11);
        assert_eq!(palette_index(0x3F15), 0x15);
    }

    #[test]
    fn write_to_3f10_is_readable_at_3f00() {
        let mut p = Palette::new();
        p.write(0x3F10, 0x2A);
        assert_eq!(p.read(0x3F00), 0x2A);
    }

    #[test]
    fn read_masked_applies_greyscale() {
        let mut p = Palette::new();
        p.write(0x3F01, 0x2F);
        assert_eq!(p.read_masked(0x3F01, false), 0x2F);
        assert_eq!(p.read_masked(0x3F01, true), 0x20);
    }

    #[test]
    fn upper_3f20_3fff_mirrors_3f00_3f1f() {
        assert_eq!(palette_index(0x3F20), palette_index(0x3F00));
        assert_eq!(palette_index(0x3FFF), palette_index(0x3F1F));
    }
}
