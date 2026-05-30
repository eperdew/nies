//! M3 PPU determinism: demo_ntsc.nes produces a stable framebuffer hash.
//! Task 4 keeps this as a self-determinism check (run-twice equality);
//! Task 5 upgrades it to a pinned golden constant.

use nies_core::Nes;
use std::hash::{DefaultHasher, Hash, Hasher};

const N_FRAMES: u64 = 200;

#[test]
fn demo_ntsc_framebuffer_is_self_deterministic() {
    let h1 = run_and_hash(N_FRAMES);
    let h2 = run_and_hash(N_FRAMES);
    assert_eq!(h1, h2, "framebuffer hash differs across identical runs");
}

/// Pinned golden hash of the demo_ntsc index framebuffer after N_FRAMES.
/// Recorded once from a known-good run (Task 5, Step 1). The same constant
/// is asserted under wasm32 in crates/nies-web/tests/determinism_wasm.rs;
/// keep the two in sync. A change here is a real determinism regression —
/// render the frame and diff against a reference before touching it.
const GOLDEN_FB_HASH: u64 = 0xdf3e45e98c8063b5;

#[test]
fn demo_ntsc_framebuffer_matches_golden_hash() {
    assert_eq!(
        run_and_hash(N_FRAMES),
        GOLDEN_FB_HASH,
        "demo_ntsc framebuffer hash drifted from the pinned golden value"
    );
}

fn run_and_hash(n_frames: u64) -> u64 {
    let mut nes = Nes::from_rom_bytes(nies_core::demo_rom_bytes()).expect("build Nes");
    for _ in 0..n_frames {
        nes.run_frame();
    }
    let mut h = DefaultHasher::new();
    nes.frame().hash(&mut h);
    h.finish()
}
