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
}
