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

    /// PPUSCROLL ($2005) write — two-write sequence controlled by w.
    /// First write: fine X → x register, coarse X → t bits 0-4. Second
    /// write: fine Y → t bits 12-14, coarse Y → t bits 5-9.
    pub fn write_ppuscroll(&mut self, val: u8) {
        if !self.w {
            self.x = val & 0b111;
            self.t = (self.t & !0b0000_0000_0001_1111) | ((val as u16) >> 3);
        } else {
            let fine_y = (val as u16 & 0b111) << 12;
            let coarse_y = ((val as u16) & 0b1111_1000) << 2;
            self.t = (self.t & !0b0111_0011_1110_0000) | fine_y | coarse_y;
        }
        self.w = !self.w;
    }

    /// PPUADDR ($2006) write — two-write sequence controlled by w.
    /// First write: high 6 bits → t[13:8], t[14] cleared. Second write:
    /// low 8 bits → t[7:0]; then v ← t.
    pub fn write_ppuaddr(&mut self, val: u8) {
        if !self.w {
            let hi = (val as u16 & 0b0011_1111) << 8;
            self.t = (self.t & 0x00FF) | hi;
        } else {
            self.t = (self.t & 0xFF00) | (val as u16);
            self.v = self.t;
        }
        self.w = !self.w;
    }

    /// PPUDATA ($2007) read with a closure-supplied VRAM bus. Used both
    /// by `Ppu::cpu_read` (provides a real bus closure) and by unit tests.
    /// The address is `v & 0x3FFF` (PPU bus is 14 bits).
    pub fn read_ppudata_with<F: FnMut(u16) -> u8>(&mut self, mut bus_read: F) -> u8 {
        let addr = self.v & 0x3FFF;
        let val = if addr < 0x3F00 {
            let prev = self.read_buffer;
            self.read_buffer = bus_read(addr);
            prev
        } else {
            self.read_buffer = bus_read(addr - 0x1000);
            bus_read(addr)
        };
        self.v = (self.v.wrapping_add(self.vram_increment())) & 0x7FFF;
        val
    }

    /// PPUDATA ($2007) write with a closure-supplied VRAM bus.
    pub fn write_ppudata_with<F: FnMut(u16, u8)>(&mut self, val: u8, mut bus_write: F) {
        let addr = self.v & 0x3FFF;
        bus_write(addr, val);
        self.v = (self.v.wrapping_add(self.vram_increment())) & 0x7FFF;
    }

    /// PPUSTATUS ($2002) read. Returns bits 5-7 from the latched status
    /// or'd with bits 0-4 from the open-bus latch. Side effects: clears
    /// bit 7 (vblank) and the w toggle.
    pub fn read_ppustatus(&mut self) -> u8 {
        let v = (self.status & 0xE0) | (self.open_bus & 0x1F);
        self.status &= 0x7F;
        self.w = false;
        // Reads also load the open-bus latch with the high 3 bits.
        self.open_bus = (self.open_bus & 0x1F) | (v & 0xE0);
        v
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

    #[test]
    fn ppustatus_read_clears_bit7_and_w() {
        let mut r = Registers::new();
        r.status = 0b1010_0000;
        r.w = true;
        let v = r.read_ppustatus();
        assert_eq!(v & 0xE0, 0b1010_0000); // bits 5-7 returned as-set BEFORE clear
        assert_eq!(r.status & 0x80, 0); // vblank bit cleared after read
        assert!(!r.w); // w toggle cleared
    }

    #[test]
    fn ppustatus_read_low_bits_come_from_open_bus() {
        let mut r = Registers::new();
        r.status = 0xE0;
        r.open_bus = 0x1F;
        let v = r.read_ppustatus();
        assert_eq!(v, 0xE0 | 0x1F);
    }

    #[test]
    fn ppuscroll_first_write_sets_fine_x_and_coarse_x_in_t() {
        let mut r = Registers::new();
        r.write_ppuscroll(0b1010_1101); // coarse_x = 0b10101, fine_x = 0b101
        assert_eq!(r.x, 0b101);
        assert_eq!(r.t & 0b0000_0000_0001_1111, 0b10101);
        assert!(r.w);
    }

    #[test]
    fn ppuscroll_second_write_sets_fine_y_and_coarse_y_in_t() {
        let mut r = Registers::new();
        r.w = true;
        r.write_ppuscroll(0b1010_1011); // coarse_y = 0b10101, fine_y = 0b011
        assert_eq!((r.t >> 12) & 0b111, 0b011);
        assert_eq!((r.t >> 5) & 0b1_1111, 0b10101);
        assert!(!r.w);
    }

    #[test]
    fn ppuaddr_first_write_sets_t_high_and_clears_bit14() {
        let mut r = Registers::new();
        r.t = 0xFFFF;
        r.write_ppuaddr(0b1011_1100);
        // Byte 0b1011_1100 has low 6 bits 0b11_1100; that goes into t bits 8-13.
        // (t >> 8) & 0x3F retrieves them. Bit 14 of t is forced 0 by the write.
        assert_eq!((r.t >> 8) & 0b0011_1111, 0b11_1100);
        assert_eq!((r.t >> 14) & 1, 0);
        assert!(r.w);
    }

    #[test]
    fn ppuaddr_second_write_sets_t_low_then_copies_t_to_v() {
        let mut r = Registers::new();
        r.t = 0x2A00;
        r.w = true;
        r.write_ppuaddr(0x55);
        assert_eq!(r.t, 0x2A55);
        assert_eq!(r.v, 0x2A55);
        assert!(!r.w);
    }

    #[test]
    fn ppudata_read_below_3f00_is_buffered() {
        let mut r = Registers::new();
        r.v = 0x2000;
        r.read_buffer = 0x42;
        let v_first = r.read_ppudata_with(|_addr| 0x99);
        assert_eq!(v_first, 0x42);
        assert_eq!(r.read_buffer, 0x99);
        assert_eq!(r.v, 0x2001);
    }

    #[test]
    fn ppudata_read_at_palette_is_immediate_and_buffer_holds_mirror() {
        let mut r = Registers::new();
        r.v = 0x3F00;
        r.read_buffer = 0x42;
        let v = r.read_ppudata_with(|addr| if addr == 0x3F00 { 0x33 } else { 0x77 });
        assert_eq!(v, 0x33);
        assert_eq!(r.read_buffer, 0x77);
    }

    #[test]
    fn ppudata_increment_is_32_when_ppuctrl_bit2_set() {
        let mut r = Registers::new();
        r.write_ppuctrl(0b0000_0100);
        r.v = 0x2000;
        r.read_ppudata_with(|_| 0);
        assert_eq!(r.v, 0x2020);
    }
}
