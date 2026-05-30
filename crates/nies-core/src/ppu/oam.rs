//! Object Attribute Memory (OAM): 64 sprite entries × 4 bytes = 256B
//! primary OAM, plus 32B secondary OAM filled during rendering.

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Oam {
    /// 256B primary OAM. 64 sprite entries × 4 bytes.
    #[serde(with = "BigArray")]
    pub primary: [u8; 256],
    /// 32B secondary OAM filled during sprite eval (dots 65-256). Holds
    /// up to 8 sprites × 4 bytes for the next scanline. Cleared on dots
    /// 1-64 (read as $FF during that window).
    pub secondary: [u8; 32],
}

impl Default for Oam {
    fn default() -> Self {
        // Power-on: primary OAM is 0xFF per Mesen.
        Self {
            primary: [0xFF; 256],
            secondary: [0xFF; 32],
        }
    }
}

impl Oam {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&self, addr: u8) -> u8 {
        self.primary[addr as usize]
    }
    /// Write a byte into primary OAM. For attribute bytes (those at
    /// addresses where `addr & 3 == 2`), bits 2-4 are unimplemented in
    /// hardware and always read back as 0, so we mask them off at the
    /// storage layer. Per nesdev "PPU OAM": "the three unimplemented
    /// bits of each sprite's byte 2 do not exist in the PPU and always
    /// read back as 0".
    pub fn write(&mut self, addr: u8, val: u8) {
        let masked = if addr & 0x03 == 0x02 { val & 0xE3 } else { val };
        self.primary[addr as usize] = masked;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oam_power_on_is_all_ff() {
        let oam = Oam::new();
        assert!(oam.primary.iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn oam_read_after_write_round_trips() {
        let mut oam = Oam::new();
        oam.write(0x10, 0x42);
        assert_eq!(oam.read(0x10), 0x42);
    }
}
