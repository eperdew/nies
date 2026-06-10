//! Cross-platform determinism gates: nies-core produces the same pinned
//! constants under wasm32 as it does natively. M3: the demo_ntsc index
//! framebuffer hash (crates/nies-core/tests/ppu_determinism.rs). M4: the
//! scripted input-demo state hash
//! (crates/nies-core/tests/input_determinism.rs). Proves nies-core is
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

/// M4 input determinism gate. MUST match GOLDEN_INPUT_HASH in
/// crates/nies-core/tests/input_determinism.rs.
const GOLDEN_INPUT_HASH: u64 = 0x94A4_5621_A5A7_FCF4;

#[wasm_bindgen_test]
fn input_demo_matches_golden_hash_on_wasm() {
    assert_eq!(
        nies_core::input_demo::run_and_hash(),
        GOLDEN_INPUT_HASH,
        "wasm input-demo state hash != native golden"
    );
}
