//! PPU register file: external CPU-facing registers ($2000-$2007) and
//! Loopy internal registers (v, t, x, w).
//!
//! Decomposition of v/t (15 bits each):
//!   yyy NN YYYYY XXXXX
//!   ||| || ||||| +++++- coarse X scroll (5 bits)
//!   ||| || +++++------- coarse Y scroll (5 bits)
//!   ||| ++------------- nametable select (2 bits)
//!   +++---------------- fine Y scroll (3 bits)
//!
//! See nesdev wiki: https://www.nesdev.org/wiki/PPU_scrolling

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registers {
    /// PPUCTRL ($2000) — write only, latched.
    pub ctrl: u8,
    /// PPUMASK ($2001) — write only, latched.
    pub mask: u8,
    /// PPUSTATUS ($2002) — read clears bit 7 and the w toggle. Bits 5-7 only.
    pub status: u8,
    /// OAMADDR ($2003) — write only.
    pub oamaddr: u8,
    /// Internal: current VRAM address (15 bits).
    pub v: u16,
    /// Internal: temporary VRAM address (15 bits, staged via $2005/$2006).
    pub t: u16,
    /// Internal: fine X scroll (3 bits).
    pub x: u8,
    /// Internal: write toggle (1 bit).
    pub w: bool,
    /// Internal: PPUDATA read buffer for $0000-$3EFF reads.
    pub read_buffer: u8,
    /// Internal: open-bus latch for register reads (for unimplemented bits).
    pub open_bus: u8,
}

impl Registers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset state per Mesen power-on snapshot. PPUCTRL/MASK/STATUS/OAMADDR
    /// all 0; v/t/x/w/buffers all 0.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// PPUCTRL ($2000) write. Latches the byte and updates t bits 10-11
    /// from byte bits 0-1 (nametable select).
    pub fn write_ppuctrl(&mut self, val: u8) {
        self.ctrl = val;
        let nt_bits = (val as u16 & 0b11) << 10;
        self.t = (self.t & !0b0000_1100_0000_0000) | nt_bits;
    }

    /// PPUCTRL accessors.
    pub fn nmi_enabled(&self) -> bool {
        self.ctrl & 0x80 != 0
    }
    pub fn sprite_size_8x16(&self) -> bool {
        self.ctrl & 0x20 != 0
    }
    pub fn bg_pattern_table_base(&self) -> u16 {
        if self.ctrl & 0x10 != 0 {
            0x1000
        } else {
            0x0000
        }
    }
    pub fn sprite_pattern_table_base(&self) -> u16 {
        if self.ctrl & 0x08 != 0 {
            0x1000
        } else {
            0x0000
        }
    }
    pub fn vram_increment(&self) -> u16 {
        if self.ctrl & 0x04 != 0 { 32 } else { 1 }
    }

    pub fn write_ppumask(&mut self, val: u8) {
        self.mask = val;
    }
    pub fn show_bg(&self) -> bool {
        self.mask & 0x08 != 0
    }
    pub fn show_sprites(&self) -> bool {
        self.mask & 0x10 != 0
    }
    pub fn rendering_enabled(&self) -> bool {
        self.show_bg() || self.show_sprites()
    }
    pub fn show_bg_left8(&self) -> bool {
        self.mask & 0x02 != 0
    }
    pub fn show_sprites_left8(&self) -> bool {
        self.mask & 0x04 != 0
    }
    pub fn greyscale(&self) -> bool {
        self.mask & 0x01 != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_default_is_all_zero() {
        let r = Registers::new();
        assert_eq!(r.ctrl, 0);
        assert_eq!(r.mask, 0);
        assert_eq!(r.status, 0);
        assert_eq!(r.oamaddr, 0);
        assert_eq!(r.v, 0);
        assert_eq!(r.t, 0);
        assert_eq!(r.x, 0);
        assert!(!r.w);
        assert_eq!(r.read_buffer, 0);
    }

    #[test]
    fn reset_clears_all_state() {
        let mut r = Registers::new();
        r.ctrl = 0x80;
        r.v = 0x2400;
        r.t = 0x2400;
        r.x = 5;
        r.w = true;
        r.read_buffer = 0x42;
        r.reset();
        assert_eq!(r, Registers::default());
    }

    #[test]
    fn ppuctrl_write_sets_nametable_bits_in_t() {
        let mut r = Registers::new();
        r.write_ppuctrl(0b0000_0011); // nametable select = 3
        assert_eq!(r.ctrl, 0b0000_0011);
        assert_eq!(r.t & 0b0000_1100_0000_0000, 0b0000_1100_0000_0000);
    }

    #[test]
    fn ppuctrl_write_preserves_other_t_bits() {
        let mut r = Registers::new();
        r.t = 0b0111_0011_1111_1111; // some unrelated bits set
        r.write_ppuctrl(0b0000_0001); // nametable = 1
        // Bits 10-11 of t should be 01; other bits unchanged.
        assert_eq!(r.t & 0b0000_1100_0000_0000, 0b0000_0100_0000_0000);
        assert_eq!(
            r.t & !0b0000_1100_0000_0000,
            0b0111_0011_1111_1111 & !0b0000_1100_0000_0000
        );
    }

    #[test]
    fn ppumask_write_latches_byte() {
        let mut r = Registers::new();
        r.write_ppumask(0b0001_1110);
        assert_eq!(r.mask, 0b0001_1110);
    }

    #[test]
    fn ppumask_rendering_enabled_iff_bg_or_sprite_bit_set() {
        let mut r = Registers::new();
        assert!(!r.rendering_enabled());
        r.write_ppumask(0b0000_1000); // bg only
        assert!(r.rendering_enabled());
        r.write_ppumask(0b0001_0000); // sprite only
        assert!(r.rendering_enabled());
        r.write_ppumask(0b0001_1000); // both
        assert!(r.rendering_enabled());
        r.write_ppumask(0b0000_0001); // greyscale only, no rendering
        assert!(!r.rendering_enabled());
    }
}
