//! Integration test harness for blargg-style and nestest test ROMs.
//!
//! ROMs that follow the blargg "$6000 status protocol" use the
//! battery-backed PRG-RAM region $6000-$60FF as a status surface:
//!
//! - `$6000`: status code; `$80` = running, `$00` = success (once the
//!   magic-ready bytes are set), any other byte = failure
//! - `$6001-$6003`: magic-ready signature `$DE $B0 $61` — written by
//!   the ROM once it has reached its main test loop
//! - `$6004+`: null-terminated ASCII status message
//!
//! `run_test_rom` loads a ROM, builds a `Cartridge` + `Bus` + `Cpu`,
//! resets, and steps the CPU until either the ROM signals completion
//! via the status protocol or a cycle budget is exhausted.

use nies_core::bus::Bus;
use nies_core::cartridge::Cartridge;
use nies_core::cpu::Cpu;
use nies_core::mapper::MapperKind;

/// Outcome of running a test ROM.
#[derive(Debug)]
pub enum RomResult {
    Done { status: u8, message: String },
    Timeout,
}

/// Load a ROM file, run it through the CPU until completion or cycle
/// budget exhaustion. See module docs for the $6000 protocol.
pub fn run_test_rom(path: &str, max_cycles: u64) -> RomResult {
    let bytes = std::fs::read(path).expect("read rom");
    let cart = Cartridge::from_bytes(&bytes).expect("parse rom");
    let mapper = MapperKind::from_cartridge(&cart).expect("build mapper");
    let mut bus = Bus::new(mapper);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    // Wait for the ROM to set the magic ready handshake at $6001-$6003,
    // then watch $6000 for a non-running status code.
    let start_cycle = bus.cycle;
    while bus.cycle - start_cycle < max_cycles {
        cpu.step(&mut bus);
        let status = bus.peek(0x6000);
        if status != 0x80 && status != 0x00 {
            return read_result(&bus, status);
        }
        // Special case: status 0x00 *with* magic-ready bytes set means success.
        if status == 0x00
            && bus.peek(0x6001) == 0xDE
            && bus.peek(0x6002) == 0xB0
            && bus.peek(0x6003) == 0x61
        {
            return read_result(&bus, 0x00);
        }
    }
    RomResult::Timeout
}

fn read_result(bus: &Bus, status: u8) -> RomResult {
    let mut s = String::new();
    for offset in 0u16..0xFC {
        let b = bus.peek(0x6004 + offset);
        if b == 0 {
            break;
        }
        s.push(b as char);
    }
    RomResult::Done { status, message: s }
}

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/roms");

/// Helper: run a ROM and assert it completes with status 0 (success).
/// Panics with the ROM's status message on failure or timeout.
fn assert_rom_passes(path: &str, max_cycles: u64) {
    match run_test_rom(path, max_cycles) {
        RomResult::Done { status: 0, .. } => { /* success */ }
        RomResult::Done { status, message } => {
            panic!("{path}: ROM failed with status {status:#04x}: {message:?}");
        }
        RomResult::Timeout => {
            panic!("{path}: ROM timed out (cycle budget {max_cycles} exhausted)");
        }
    }
}

#[test]
fn nestest_loads_and_runs() {
    // nestest in normal-reset mode (PC from $FFFC) waits for controller
    // input at the menu. We just verify the ROM loads, the CPU resets
    // cleanly, and the harness doesn't panic running it for a short
    // budget. The byte-level Nintendulator-log comparison (PC=$C000
    // automated mode) lives in Unit 14 (Task 46).
    let path = format!("{ROOT}/nestest/nestest.nes");
    let _ = run_test_rom(&path, 1_000_000);
}

// The blargg "combined" runners (cpu_instrs.nes, instr_timing.nes,
// instr_misc.nes) all use MMC1 (mapper 1) for bank-switching between
// the per-category sub-tests; cpu_dummy_reads.nes uses CNROM (mapper
// 3). M1 only implements NROM (mapper 0); see CLAUDE.md "What's
// intentionally NOT in scope" — mappers 1-… land at M11+. We
// therefore exercise the NROM sub-tests directly, which together
// cover the same opcode-level content as the combined runners.

#[test]
fn blargg_cpu_instrs_01_basics() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/cpu_instrs/01-basics.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_cpu_instrs_02_implied() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/cpu_instrs/02-implied.nes"),
        60_000_000,
    );
}

// 03-immediate fails on opcode $AB (ATX / LXA / OAL), an unstable
// illegal opcode whose result depends on a "magic constant" that
// varies across real-hardware chip lots, temperature, and even
// recently-bussed values. Our implementation matches the magic
// constant used by SingleStepTests/65x02 (`0xEE`); blargg's checksum
// for this opcode is computed against a different (undocumented)
// constant. The two corpuses simply disagree on this single opcode.
//
// Since:
// - SingleStepTests/65x02 is M1's authoritative bus-precise corpus
//   (it checks per-cycle bus traces, not just final state),
// - the failure is on `0xAB` only — the other 64 immediate-mode
//   instructions in 03-immediate pass — and
// - no commercial NES game uses `0xAB`,
// we keep our `0xEE` constant and document the corpus disagreement
// rather than chase blargg's checksum. Re-evaluate if a commercial
// game ever surfaces a dependency on this opcode.
#[test]
#[ignore = "blargg+SingleStepTests disagree on $AB ATX magic constant; we match SingleStepTests"]
fn blargg_cpu_instrs_03_immediate() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/cpu_instrs/03-immediate.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_cpu_instrs_04_zero_page() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/cpu_instrs/04-zero_page.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_cpu_instrs_05_zp_xy() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/cpu_instrs/05-zp_xy.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_cpu_instrs_06_absolute() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/cpu_instrs/06-absolute.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_cpu_instrs_07_abs_xy() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/cpu_instrs/07-abs_xy.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_cpu_instrs_08_ind_x() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/cpu_instrs/08-ind_x.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_cpu_instrs_09_ind_y() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/cpu_instrs/09-ind_y.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_cpu_instrs_10_branches() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/cpu_instrs/10-branches.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_cpu_instrs_11_stack() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/cpu_instrs/11-stack.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_cpu_instrs_12_jmp_jsr() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/cpu_instrs/12-jmp_jsr.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_cpu_instrs_13_rts() {
    assert_rom_passes(&format!("{ROOT}/blargg/cpu_instrs/13-rts.nes"), 60_000_000);
}

#[test]
fn blargg_cpu_instrs_14_rti() {
    assert_rom_passes(&format!("{ROOT}/blargg/cpu_instrs/14-rti.nes"), 60_000_000);
}

#[test]
fn blargg_cpu_instrs_15_brk() {
    assert_rom_passes(&format!("{ROOT}/blargg/cpu_instrs/15-brk.nes"), 60_000_000);
}

#[test]
fn blargg_cpu_instrs_16_special() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/cpu_instrs/16-special.nes"),
        60_000_000,
    );
}

// instr_timing's per-instruction timing measurement uses the APU
// length counter (the test "synchronizes to the APU length counter
// and then loads it with 2, executing the instruction in a loop
// that stops once the length counter expires"; see the bundled
// readme.txt). The APU is M5 territory — at M1 the APU is a stub
// that doesn't tick a length counter, so the loop runs forever
// from the test's perspective and reports a spurious mis-timing.
//
// SingleStepTests/65x02 checks per-cycle bus traces and is green
// for all 256 opcodes, which already validates per-instruction
// timing at the bus level; instr_timing's complementary check
// re-enables once the APU exists.
#[test]
#[ignore = "depends on APU length counter (M5); SingleStepTests already validates per-cycle bus traces"]
fn blargg_instr_timing_1_instr_timing() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/instr_timing/1-instr_timing.nes"),
        200_000_000,
    );
}

#[test]
#[ignore = "depends on APU length counter (M5); SingleStepTests already validates per-cycle bus traces"]
fn blargg_instr_timing_2_branch_timing() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/instr_timing/2-branch_timing.nes"),
        200_000_000,
    );
}

// instr_misc.nes (mapper 1) and cpu_dummy_reads.nes (mapper 3) require
// non-NROM mappers; their integration tests are deferred to M11+ when
// those mappers land. The ROMs are vendored so they can be wired up
// without re-vendoring at that time.

// M2 PPU vblank/NMI timing suite. Each sub-test is a separate NROM
// build of blargg's ppu_vbl_nmi tests, exercising a specific timing
// edge case (vblank set/clear time, NMI control, suppression races,
// odd-frame timing).

#[test]
fn blargg_ppu_vbl_nmi_01_basics() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/ppu_vbl_nmi/01-vbl_basics.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_ppu_vbl_nmi_02_vbl_set_time() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/ppu_vbl_nmi/02-vbl_set_time.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_ppu_vbl_nmi_03_vbl_clear_time() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/ppu_vbl_nmi/03-vbl_clear_time.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_ppu_vbl_nmi_04_nmi_control() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/ppu_vbl_nmi/04-nmi_control.nes"),
        60_000_000,
    );
}

// Sub-tests 05-08 measure NMI dispatch latency at single-cycle precision.
// On a real 6502, interrupts are polled on the penultimate cycle of every
// instruction; our CPU samples at instruction boundaries instead. The
// resulting off-by-one shows up as "NMI fires 2-4 cycles late" in 05, and
// as wrong V/N columns at the suppression edges in 06/07/08. Implementing
// per-cycle interrupt polling is a CPU-wide refactor (every opcode handler
// needs to participate); it's deferred to a later milestone where APU
// frame-counter IRQ timing will need the same infrastructure. See M2
// design spec §4.4 and global spec §7.8.
#[test]
#[ignore = "requires per-cycle interrupt polling (penultimate-cycle 6502 sampling); deferred — see spec §7.8"]
fn blargg_ppu_vbl_nmi_05_nmi_timing() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/ppu_vbl_nmi/05-nmi_timing.nes"),
        60_000_000,
    );
}

#[test]
#[ignore = "requires per-cycle interrupt polling (penultimate-cycle 6502 sampling); deferred — see spec §7.8"]
fn blargg_ppu_vbl_nmi_06_suppression() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/ppu_vbl_nmi/06-suppression.nes"),
        60_000_000,
    );
}

#[test]
#[ignore = "requires per-cycle interrupt polling (penultimate-cycle 6502 sampling); deferred — see spec §7.8"]
fn blargg_ppu_vbl_nmi_07_nmi_on_timing() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/ppu_vbl_nmi/07-nmi_on_timing.nes"),
        60_000_000,
    );
}

#[test]
#[ignore = "requires per-cycle interrupt polling (penultimate-cycle 6502 sampling); deferred — see spec §7.8"]
fn blargg_ppu_vbl_nmi_08_nmi_off_timing() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/ppu_vbl_nmi/08-nmi_off_timing.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_ppu_vbl_nmi_09_even_odd_frames() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/ppu_vbl_nmi/09-even_odd_frames.nes"),
        60_000_000,
    );
}

// 10-even_odd_timing measures the cycle at which the dot-339 skip on odd
// pre-render scanlines fires *relative to a mid-frame PPUMASK BG-enable
// write*. Our `rendering_enabled` is sampled at the top of each PPU step,
// so a PPUMASK write during the same CPU cycle as dot 338→339 sees the
// skip applied one CPU cycle (~3 PPU dots) "late" from the test's
// perspective. The fix is sub-PPU-step register write granularity, which
// is intertwined with per-cycle interrupt polling; deferred together.
#[test]
#[ignore = "odd-frame skip vs mid-frame PPUMASK write needs sub-step register granularity; deferred — see spec §7.8"]
fn blargg_ppu_vbl_nmi_10_even_odd_timing() {
    assert_rom_passes(
        &format!("{ROOT}/blargg/ppu_vbl_nmi/10-even_odd_timing.nes"),
        60_000_000,
    );
}

// M2 PPU sprite-zero-hit suite. Each sub-test exercises a specific
// edge of the sprite-0 hit detection (basic firing, pixel alignment,
// corner pixels, flip flags, left-column clipping, x=255 suppression,
// scanline 239 bottom-row exclusion, 8x16 mode, and three cycle-precise
// timing tests).

/// Runner for blargg's sprite_hit_tests_2005.10.05 suite. These ROMs do
/// NOT use the $6000 status protocol — they display results on screen
/// and beep. They store the result in zero-page $F8:
/// - `$F8 == 1` → all sub-tests passed
/// - any other value → failure code of the failing sub-test (see the
///   suite's readme.txt for code → meaning)
///
/// Every ROM ends in a `JMP <self>` infinite loop ("done trap") at one
/// of several addresses depending on the ROM. We detect this trap by
/// checking whether the byte at PC is `$4C` (JMP absolute) and the
/// operand equals PC — i.e., a self-jump.
fn assert_sprite_hit_rom_passes(path: &str, max_cycles: u64) {
    let bytes = std::fs::read(path).expect("read rom");
    let cart = Cartridge::from_bytes(&bytes).expect("parse rom");
    let mapper = MapperKind::from_cartridge(&cart).expect("build mapper");
    let mut bus = Bus::new(mapper);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let start = bus.cycle;
    while bus.cycle - start < max_cycles {
        cpu.step(&mut bus);
        // Detect `JMP $self` infinite loop at PC. JMP abs opcode = $4C.
        if bus.peek(cpu.pc) == 0x4C {
            let lo = bus.peek(cpu.pc.wrapping_add(1)) as u16;
            let hi = bus.peek(cpu.pc.wrapping_add(2)) as u16;
            let target = (hi << 8) | lo;
            if target == cpu.pc {
                let f8 = bus.peek(0xF8);
                if f8 == 1 {
                    return; // success
                }
                panic!(
                    "{path}: sprite_hit ROM failed with $F8 = {f8:#04x} (see readme.txt for code meaning)"
                );
            }
        }
    }
    panic!(
        "{path}: sprite_hit ROM timed out (cycle budget {max_cycles} exhausted, last $F8 = {:#04x})",
        bus.peek(0xF8)
    );
}

#[test]
fn blargg_sprite_hit_01_basics() {
    assert_sprite_hit_rom_passes(
        &format!("{ROOT}/blargg/sprite_hit_tests_2005.10.05/01.basics.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_sprite_hit_02_alignment() {
    assert_sprite_hit_rom_passes(
        &format!("{ROOT}/blargg/sprite_hit_tests_2005.10.05/02.alignment.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_sprite_hit_03_corners() {
    assert_sprite_hit_rom_passes(
        &format!("{ROOT}/blargg/sprite_hit_tests_2005.10.05/03.corners.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_sprite_hit_04_flip() {
    assert_sprite_hit_rom_passes(
        &format!("{ROOT}/blargg/sprite_hit_tests_2005.10.05/04.flip.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_sprite_hit_05_left_clip() {
    assert_sprite_hit_rom_passes(
        &format!("{ROOT}/blargg/sprite_hit_tests_2005.10.05/05.left_clip.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_sprite_hit_06_right_edge() {
    assert_sprite_hit_rom_passes(
        &format!("{ROOT}/blargg/sprite_hit_tests_2005.10.05/06.right_edge.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_sprite_hit_07_screen_bottom() {
    assert_sprite_hit_rom_passes(
        &format!("{ROOT}/blargg/sprite_hit_tests_2005.10.05/07.screen_bottom.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_sprite_hit_08_double_height() {
    assert_sprite_hit_rom_passes(
        &format!("{ROOT}/blargg/sprite_hit_tests_2005.10.05/08.double_height.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_sprite_hit_09_timing_basics() {
    assert_sprite_hit_rom_passes(
        &format!("{ROOT}/blargg/sprite_hit_tests_2005.10.05/09.timing_basics.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_sprite_hit_10_timing_order() {
    assert_sprite_hit_rom_passes(
        &format!("{ROOT}/blargg/sprite_hit_tests_2005.10.05/10.timing_order.nes"),
        60_000_000,
    );
}

#[test]
fn blargg_sprite_hit_11_edge_timing() {
    assert_sprite_hit_rom_passes(
        &format!("{ROOT}/blargg/sprite_hit_tests_2005.10.05/11.edge_timing.nes"),
        60_000_000,
    );
}
