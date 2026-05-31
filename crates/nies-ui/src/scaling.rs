//! Pure integer-scaling math for the game viewport. Keeps the geometry
//! testable without a GPU.

/// NES visible resolution.
pub const NES_W: u32 = 256;
pub const NES_H: u32 = 240;

/// A centered, integer-scaled viewport rectangle within `target`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Largest integer-scaled, centered NES rectangle that fits in
/// `target_w × target_h`. Scale is at least 1 (the image may overflow a
/// tiny window rather than vanish). The surrounding area is letterboxed.
pub fn integer_scale(target_w: u32, target_h: u32) -> Viewport {
    let scale = (target_w / NES_W).min(target_h / NES_H).max(1);
    let w = NES_W * scale;
    let h = NES_H * scale;
    let x = target_w.saturating_sub(w) / 2;
    let y = target_h.saturating_sub(h) / 2;
    Viewport { x, y, w, h }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_fit_scale_1() {
        assert_eq!(
            integer_scale(256, 240),
            Viewport {
                x: 0,
                y: 0,
                w: 256,
                h: 240
            }
        );
    }

    #[test]
    fn picks_largest_integer_scale_and_centers() {
        assert_eq!(
            integer_scale(800, 600),
            Viewport {
                x: (800 - 512) / 2,
                y: (600 - 480) / 2,
                w: 512,
                h: 480
            }
        );
    }

    #[test]
    fn tiny_window_clamps_to_scale_1() {
        assert_eq!(
            integer_scale(100, 100),
            Viewport {
                x: 0,
                y: 0,
                w: 256,
                h: 240
            }
        );
    }
}
