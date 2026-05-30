//! Background fetch pipeline: 8-dot repeating fetches feeding 16-bit
//! pattern shifters and 8-bit attribute latches.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Background {
    /// Latched nametable byte (current tile's pattern index).
    pub nt_byte: u8,
    /// Latched attribute byte (palette select for the 4×4-tile region).
    pub at_byte: u8,
    /// Latched pattern low byte (current tile, current row).
    pub pat_lo_latch: u8,
    /// Latched pattern high byte.
    pub pat_hi_latch: u8,
    /// 16-bit pattern shifters. High 8 bits hold current tile; low 8 bits hold next tile.
    pub shift_pat_lo: u16,
    pub shift_pat_hi: u16,
    /// 8-bit attribute shifters; each pixel sees its 2-bit palette select.
    pub shift_at_lo: u8,
    pub shift_at_hi: u8,
    /// Latched attribute bits for the *next* tile (loaded at dot 8 boundary).
    pub at_latch_lo: bool,
    pub at_latch_hi: bool,
}

impl Background {
    pub fn new() -> Self {
        Self::default()
    }

    /// Shift the pattern + attribute shifters one bit. Called every visible
    /// rendering dot.
    pub fn shift(&mut self) {
        self.shift_pat_lo <<= 1;
        self.shift_pat_hi <<= 1;
        self.shift_at_lo = (self.shift_at_lo << 1) | (self.at_latch_lo as u8);
        self.shift_at_hi = (self.shift_at_hi << 1) | (self.at_latch_hi as u8);
    }

    /// Load the next tile's latched values into the low half of the pattern
    /// shifters. Called at the dot-8 boundary of each fetch group. The
    /// at_latch_{lo,hi} are set by the caller before this call.
    pub fn reload_shifters(&mut self) {
        self.shift_pat_lo = (self.shift_pat_lo & 0xFF00) | self.pat_lo_latch as u16;
        self.shift_pat_hi = (self.shift_pat_hi & 0xFF00) | self.pat_hi_latch as u16;
    }
}
