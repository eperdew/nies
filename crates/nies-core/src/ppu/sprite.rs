//! Sprite evaluation, fetch, and per-pixel sprite/background priority.
//!
//! - Dots 1-64: clear secondary OAM (write $FF).
//! - Dots 65-256: scan primary OAM, copy in-range sprites to secondary.
//! - Dots 257-320: fetch pattern data for the 8 sprites placed in
//!   secondary OAM into the sprite shifters.
//! - Dots 1-256 of the *next* scanline: emit sprite pixels alongside
//!   background pixels with priority resolution.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sprites {
    /// Per-sprite shifters and latches for the 8 sprites on the current
    /// scanline (loaded during dots 257-320 of the *previous* scanline).
    pub shifters: [SpriteSlot; 8],
    /// Number of sprites placed in secondary OAM for the current scanline.
    pub count: u8,
    /// Sprite-0 is among the placed sprites (used to gate sprite-0 hit).
    pub sprite_0_in_range: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteSlot {
    pub pat_lo: u8,
    pub pat_hi: u8,
    pub attr: u8,
    pub x: u8,
}

impl Sprites {
    pub fn new() -> Self {
        Self::default()
    }
}
