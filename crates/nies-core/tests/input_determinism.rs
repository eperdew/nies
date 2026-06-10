//! M4 input determinism gate (design spec §6.3): a scripted input
//! sequence over the input_demo micro-ROM produces a stable, pinned
//! state hash. The same constant is asserted under wasm32 in
//! crates/nies-web/tests/determinism_wasm.rs; keep the two in sync.

use nies_core::input_demo;

/// Ring-buffer encoding: the NMI handler's LSR/ROL loop shifts the
/// first-read bit (A) into bit 7, so ring bytes are bit-reversed
/// relative to `Buttons`.
const R_A: u8 = 0x80;
const R_RIGHT: u8 = 0x01;
const R_START: u8 = 0x10;
const R_DOWN: u8 = 0x04;

/// Structural check, asserted before any hash is pinned: the ring
/// buffer records exactly the scripted progression of port-0 states,
/// and port 1 always reads released. Debuggable on failure, unlike a
/// hash mismatch.
#[test]
fn ring_buffer_records_the_scripted_sequence() {
    let nes = input_demo::run_input_demo();
    let ram = nes.ram();
    let idx = ram[0x02] as usize;
    assert_eq!(idx % 2, 0, "ring entries come in (p0, p1) pairs");
    assert!(
        idx >= 2 * 15,
        "expected ≥15 NMI polls in {} frames, got {}",
        input_demo::DEMO_FRAMES,
        idx / 2
    );
    let entries = &ram[0x0300..0x0300 + idx];

    let p1: Vec<u8> = entries.iter().skip(1).step_by(2).copied().collect();
    assert!(
        p1.iter().all(|&b| b == 0),
        "port 1 must always poll released, got {p1:02X?}"
    );

    let p0: Vec<u8> = entries.iter().step_by(2).copied().collect();
    let mut dedup: Vec<u8> = vec![p0[0]];
    for &b in &p0[1..] {
        if b != *dedup.last().unwrap() {
            dedup.push(b);
        }
    }
    assert_eq!(
        dedup,
        vec![
            0x00,          // released at first poll (pre-script)
            R_A,           // frame 6
            R_A | R_RIGHT, // frame 8
            0x00,          // frame 10
            R_START,       // frame 12
            0x00,          // frame 14
            R_DOWN,        // frame 16
            0x00,          // frame 19; the frame-20 SELECT press+release
                           // is journaled but never polled
        ],
        "deduped port-0 poll sequence"
    );
}

/// The journal records every script step — including the SELECT
/// press+release pair the polls never see.
#[test]
fn journal_records_all_script_steps() {
    let nes = input_demo::run_input_demo();
    let script = input_demo::script();
    let log = nes.input_log();
    assert_eq!(log.len(), script.len());
    for (event, step) in log.iter().zip(&script) {
        assert_eq!(event.port, step.port);
        assert_eq!(event.buttons, step.buttons);
    }
}

#[test]
fn input_demo_is_self_deterministic() {
    assert_eq!(
        input_demo::run_and_hash(),
        input_demo::run_and_hash(),
        "state hash differs across identical scripted runs"
    );
}

/// Pinned golden hash (RAM + index framebuffer) of the scripted run.
/// MUST equal the constant in crates/nies-web/tests/determinism_wasm.rs.
/// Like the M3 demo_ntsc hash, this encodes pre-M5 instruction-boundary
/// NMI timing and will need re-pinning when M5's per-cycle interrupt
/// polling lands (design spec §8). A change at any other time is a real
/// determinism regression — debug via ring_buffer_records_the_scripted_
/// sequence before touching it.
const GOLDEN_INPUT_HASH: u64 = 0x94A4_5621_A5A7_FCF4;

#[test]
fn input_demo_matches_golden_hash() {
    assert_eq!(
        input_demo::run_and_hash(),
        GOLDEN_INPUT_HASH,
        "input demo state hash drifted from the pinned golden value"
    );
}
