//! Per-dot PPU clock: dot 0..340, scanline 0..261, frame parity.
//!
//! At M2 only the counters live here. Rendering side effects are added by
//! background.rs and sprite.rs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PpuState {
    /// Current dot within the scanline: 0..=340.
    pub dot: u16,
    /// Current scanline: 0..=261. 0..=239 visible, 240 post-render,
    /// 241..=260 vblank, 261 pre-render.
    pub scanline: u16,
    /// Even (`false`) / odd (`true`) frame parity. Toggles at the end of each frame.
    pub frame_parity: bool,
    /// Total frames completed since power-on (for tests / debugger).
    pub frames: u64,
}

impl PpuState {
    pub fn new() -> Self {
        Self {
            dot: 0,
            scanline: 0,
            frame_parity: false,
            frames: 0,
        }
    }

    /// Advance one dot. Wraps to next scanline at dot 340 and to next
    /// frame at scanline 261. Frame parity flips on every wrap to scanline 0.
    pub fn advance_dot(&mut self) {
        self.dot += 1;
        if self.dot > 340 {
            self.dot = 0;
            self.scanline += 1;
            if self.scanline > 261 {
                self.scanline = 0;
                self.frame_parity = !self.frame_parity;
                self.frames += 1;
            }
        }
    }

    /// Advance one dot, honoring the odd-frame skip when rendering is
    /// enabled: at (scanline 261, dot 339, odd frame, rendering_enabled),
    /// the next dot is (0, 0) instead of (261, 340).
    pub fn advance_dot_with_rendering(&mut self, rendering_enabled: bool) {
        if rendering_enabled && self.frame_parity && self.scanline == 261 && self.dot == 339 {
            self.dot = 0;
            self.scanline = 0;
            self.frame_parity = !self.frame_parity;
            self.frames += 1;
            return;
        }
        self.advance_dot();
    }
}

impl Default for PpuState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_dot_wraps_to_next_scanline_at_341() {
        let mut s = PpuState::new();
        for _ in 0..341 {
            s.advance_dot();
        }
        assert_eq!(s.dot, 0);
        assert_eq!(s.scanline, 1);
    }

    #[test]
    fn advance_dot_wraps_to_next_frame_after_262_scanlines() {
        let mut s = PpuState::new();
        for _ in 0..(341 * 262) {
            s.advance_dot();
        }
        assert_eq!(s.dot, 0);
        assert_eq!(s.scanline, 0);
        assert_eq!(s.frames, 1);
        assert!(s.frame_parity);
    }

    #[test]
    fn frame_parity_alternates_each_frame() {
        let mut s = PpuState::new();
        for _ in 0..(341 * 262 * 2) {
            s.advance_dot();
        }
        assert_eq!(s.frames, 2);
        assert!(!s.frame_parity);
    }

    #[test]
    fn odd_frame_skips_dot_339_when_rendering_enabled() {
        let mut s = PpuState::new();
        s.frame_parity = true;
        s.scanline = 261;
        s.dot = 339;
        s.advance_dot_with_rendering(true);
        assert_eq!(s.scanline, 0);
        assert_eq!(s.dot, 0);
        assert_eq!(s.frames, 1);
        assert!(!s.frame_parity); // toggled by frame wrap
    }

    #[test]
    fn even_frame_does_not_skip_dot_339() {
        let mut s = PpuState::new();
        s.frame_parity = false;
        s.scanline = 261;
        s.dot = 339;
        s.advance_dot_with_rendering(true);
        assert_eq!(s.scanline, 261);
        assert_eq!(s.dot, 340);
        assert_eq!(s.frames, 0);
    }

    #[test]
    fn odd_frame_does_not_skip_when_rendering_disabled() {
        let mut s = PpuState::new();
        s.frame_parity = true;
        s.scanline = 261;
        s.dot = 339;
        s.advance_dot_with_rendering(false);
        assert_eq!(s.scanline, 261);
        assert_eq!(s.dot, 340);
    }
}
