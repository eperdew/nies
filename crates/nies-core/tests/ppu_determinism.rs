//! M3 PPU determinism: demo_ntsc.nes produces a stable framebuffer hash.
//! Task 4 keeps this as a self-determinism check (run-twice equality);
//! Task 5 upgrades it to a pinned golden constant.

use nies_core::Nes;
use std::hash::{DefaultHasher, Hasher};

const N_FRAMES: u64 = 200;

#[test]
fn demo_ntsc_framebuffer_is_self_deterministic() {
    let h1 = run_and_hash(N_FRAMES);
    let h2 = run_and_hash(N_FRAMES);
    assert_eq!(h1, h2, "framebuffer hash differs across identical runs");
}

/// Pinned golden hash of the demo_ntsc index framebuffer after N_FRAMES.
/// Computed via `run_and_hash` (portable `Hasher::write` — see its comment).
/// The same constant is asserted under wasm32 in
/// crates/nies-web/tests/determinism_wasm.rs; keep the two in sync. A change
/// here is a real determinism regression — render the frame and diff against
/// a reference before touching it.
///
/// KNOWN PRE-M5 TIMING: demo_ntsc draws an NMI-synchronized "middle line"
/// whose position encodes NMI-dispatch precision. Our CPU samples
/// interrupts at instruction boundaries, not the penultimate cycle (global
/// spec §7.8, deferred to M5), so the line renders shifted slightly left of
/// where a cycle-accurate emulator puts it. This is deterministic, so the
/// hash is stable and valid as a cross-platform gate — but the value will
/// need re-pinning once M5's per-cycle interrupt polling lands. (Ruled out
/// the M11 sprite-eval-collapse: demo_ntsc never reads $2004 mid-scanline.)
const GOLDEN_FB_HASH: u64 = 0x886769044cc33914;

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
    // Hash the raw bytes via `Hasher::write`, NOT `frame().hash()`. The slice
    // `Hash` impl prepends a `usize` length prefix, and `usize` is 8 bytes on
    // 64-bit native but 4 bytes on wasm32 — so `.hash()` yields a different
    // value per pointer width even for identical bytes, breaking the
    // cross-platform gate. `write` absorbs only the bytes; SipHash's u64/LE
    // internals are platform-independent.
    h.write(nes.frame());
    h.finish()
}
