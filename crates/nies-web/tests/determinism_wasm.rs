//! Cross-platform determinism gate: the demo_ntsc index framebuffer hashes
//! to the same pinned constant under wasm32 as it does natively
//! (crates/nies-core/tests/ppu_determinism.rs). Proves nies-core is
//! bit-identical on the web target. Run via:
//!   wasm-pack test --headless --chrome crates/nies-web

use nies_core::Nes;
use std::hash::{DefaultHasher, Hasher};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

const N_FRAMES: u64 = 200;
/// MUST match GOLDEN_FB_HASH in crates/nies-core/tests/ppu_determinism.rs.
const GOLDEN_FB_HASH: u64 = 0x886769044cc33914;

#[wasm_bindgen_test]
fn demo_ntsc_framebuffer_matches_golden_hash_on_wasm() {
    let mut nes = Nes::from_rom_bytes(nies_core::demo_rom_bytes()).expect("build Nes");
    for _ in 0..N_FRAMES {
        nes.run_frame();
    }
    let mut h = DefaultHasher::new();
    // `Hasher::write`, not `frame().hash()`: the slice Hash impl prepends a
    // `usize` length prefix whose width (8 bytes native / 4 bytes wasm32)
    // makes `.hash()` non-portable. See ppu_determinism.rs for the detail.
    h.write(nes.frame());
    assert_eq!(
        h.finish(),
        GOLDEN_FB_HASH,
        "wasm framebuffer hash != native golden"
    );
}
