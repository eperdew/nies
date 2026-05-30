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

fn run_and_hash(n_frames: u64) -> u64 {
    let mut nes = Nes::from_rom_bytes(nies_core::demo_rom_bytes()).expect("build Nes");
    for _ in 0..n_frames {
        nes.run_frame();
    }
    let mut h = DefaultHasher::new();
    nes.frame().hash(&mut h);
    h.finish()
}
