//! Per-opcode integration tests driven by the SingleStepTests/65x02
//! corpus. Each opcode-implementation task adds a `#[test]` here that
//! calls `run_opcode_tests(0xNN)`.
//!
//! The corpus assumes a 64 KiB flat memory model (no PPU/APU sub-mapping),
//! so this test runner builds a `FlatBus` (declared below) that mirrors
//! `Bus`'s public read/write interface but stores all addresses in a
//! single 64 KiB `Vec<u8>` plus a recorded access list.

mod common;

use common::{load_opcode_cases, TestCase};
use nies_core::cpu::flags;

/// 64 KiB flat memory + cycle-by-cycle access trace.
pub struct FlatBus {
    pub mem: [u8; 0x10000],
    pub trace: Vec<(u16, u8, &'static str)>,
}

impl FlatBus {
    fn new() -> Self {
        FlatBus {
            mem: [0u8; 0x10000],
            trace: Vec::with_capacity(16),
        }
    }
}

/// Mini-Cpu compatible enough with `nies_core::cpu::Cpu` to drive the
/// dispatch table. Built so we can substitute the FlatBus without
/// changing the Cpu API.
///
/// We deliberately reuse the production opcode handlers; the SingleStepTests
/// corpus's purpose is to validate that production code is correct against
/// every documented case.
pub struct TestHarness {
    pub cpu: nies_core::cpu::Cpu,
    pub bus: FlatBus,
}

impl TestHarness {
    fn from_initial(state: &common::TestState) -> Self {
        let mut bus = FlatBus::new();
        for &(addr, val) in &state.ram {
            bus.mem[addr as usize] = val;
        }
        let mut cpu = nies_core::cpu::Cpu::new();
        cpu.pc = state.pc;
        cpu.sp = state.s;
        cpu.a = state.a;
        cpu.x = state.x;
        cpu.y = state.y;
        cpu.p = state.p;
        TestHarness { cpu, bus }
    }
}

/// Run all test cases for a specific opcode and assert state + cycle trace
/// match. Reports the first failing case if any.
pub fn run_opcode_tests(opcode: u8) {
    let cases = load_opcode_cases(opcode);
    let mut failures = 0usize;
    let mut first_failure: Option<String> = None;

    for case in &cases {
        match run_single_case(opcode, case) {
            Ok(()) => {}
            Err(msg) => {
                failures += 1;
                if first_failure.is_none() {
                    first_failure = Some(format!("case '{}': {msg}", case.name));
                }
            }
        }
    }

    if failures > 0 {
        panic!(
            "opcode {opcode:02X}: {failures}/{} cases failed. First failure: {}",
            cases.len(),
            first_failure.as_deref().unwrap_or("?")
        );
    }
}

fn run_single_case(_opcode: u8, case: &TestCase) -> Result<(), String> {
    let harness = TestHarness::from_initial(&case.initial);

    // Step one instruction. Production CPU expects a real `Bus`; our
    // FlatBus needs a thin adapter. The simplest approach for this
    // milestone: re-implement step against FlatBus by calling the
    // dispatch table directly with a bus-shaped trait. That requires
    // a `Bus`-like trait the production Cpu can consume. For M1 we
    // accept a small duplication: the test harness mirrors the bus's
    // public surface but operates on FlatBus.

    // ... (full step-against-FlatBus logic implemented in Task 13
    // as part of the LDA #imm bring-up. At Task 12 the runner exists
    // but cannot yet drive the CPU through opcodes — Task 13 introduces
    // a `BusLike` trait that both production Bus and FlatBus implement,
    // allowing the dispatch table to be polymorphic.)
    //
    // For Task 12 the runner is dead-coded.

    let _ = (case, &harness, flags::FLAG_C);
    Err("step-against-FlatBus not yet implemented; see Task 13".to_string())
}
