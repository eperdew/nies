# M5a — Cycle-Accurate Interrupt Timing Design

**Status:** Approved for implementation planning
**Date:** 2026-06-10
**Author:** Eric Perdew (with Claude Code as co-designer)
**Predecessor:** [`2026-05-02-nes-emulator-design.md`](2026-05-02-nes-emulator-design.md) §7.8 (deferred-test table), §8 (M5 milestone). Read the global spec first; this document refines M5a only.

## 1. The M5 split

The global spec's M5 bundles two large work packages: a CPU-wide interrupt
timing refactor and the full APU + audio stack. This milestone splits them
(decision 2026-06-10):

- **M5a (this spec):** per-cycle interrupt polling + PPU write alignment.
  Pure `nies-core`; no new dependencies.
- **M5b (separate spec, written after M5a lands):** APU channels, frame
  counter, DMC, mixer, resampler, cpal native + WebAudio wasm,
  audio-driven pacing, volume/mute hooks.

Sequencing rationale: blargg's `apu_test` measures frame-counter IRQ
*service* timing, so the APU inherits whatever interrupt-timing slop the
CPU has. Landing the CPU refactor first means it is validated in
isolation (SingleStepTests cycle traces + the 8 deferred PPU/sprite
ROMs) before the APU depends on it, and an APU-era timing bug bisects
cleanly. The global spec §8 M5 entry and the §7.8 table rows are amended
to record the split (final unit of this milestone).

## 2. Goal and gate

The CPU samples interrupt lines with penultimate-cycle semantics instead
of instruction-boundary semantics; PPU register writes take effect at
the hardware-correct dot within their CPU cycle.

**Gate:**

- The 8 deferred ROMs un-`#[ignore]` and pass:
  - `blargg/ppu_vbl_nmi/05-nmi_timing.nes`, `06-suppression.nes`,
    `07-nmi_on_timing.nes`, `08-nmi_off_timing.nes`,
    `10-even_odd_timing.nes` → the `ppu_vbl_nmi` suite goes 10/10.
  - `blargg/sprite_hit_tests/07.screen_bottom.nes`,
    `09.timing_basics.nes`, `10.timing_order.nes` → the sprite-hit
    suite goes 11/11.
- Every existing suite stays green: 256 SingleStepTests opcodes,
  `nestest` log compare, `cpu_instrs` sub-tests, all currently-passing
  PPU/OAM tests, the M4 input determinism gate.
- Both golden hashes re-pinned — 4 constants: `GOLDEN_FB_HASH` and
  `GOLDEN_INPUT_HASH` natively (`ppu_determinism.rs`,
  `input_determinism.rs`) plus their wasm twins
  (`crates/nies-web/tests/determinism_wasm.rs`) — asserted on both
  platforms. This is the re-pin both test files and CLAUDE.md have
  documented since M3/M4.
- Manual: `demo_ntsc`'s NMI-synchronized middle line renders at its
  cycle-accurate position (it currently sits shifted left; global spec
  §7.8 "M3 observation").

## 3. Scope

### 3.1 In scope

- Two-stage interrupt shadow in `Bus` (`tick` + `stall`), with `BusLike`
  default impls keeping `FlatBus` untouched.
- `Cpu::step` boundary decision reads the shadow (penultimate-cycle
  semantics).
- Branch-quirk handling (taken, no page cross: poll at operand fetch,
  none at final cycle).
- Interrupt hijacking (NMI steals the BRK/IRQ vector during the entry
  sequence).
- IRQ line gets the same shadow treatment (mapper IRQ now; APU frame
  IRQ consumes it at M5b).
- PPU register **write** alignment: explicit mid-cycle application
  point within the 3-dot window. Read alignment only if tests 06 /
  sprite-hit trio demand it.
- Un-ignoring the 8 ROMs; re-pinning the 4 hash constants; global spec
  §7.8/§8 amendments.

### 3.2 Out of scope (deferred, with milestone)

| Deferred | Milestone |
|---|---|
| APU, audio output, audio-driven pacing | M5b |
| `instr_timing` 1 & 2 un-ignore (needs APU length counter) | M5b |
| Sprite eval/fetch per-dot spreading (dots 65-256 / 257-320) | M11 (MMC3) |
| Cycle-stepped (micro-coded) CPU rewrite | not planned (tier-2 charter) |

## 4. The interrupt shadow

### 4.1 Mechanism

A two-stage pipeline in `Bus`:

```text
            live latch                shadow (what the CPU sees)
PPU edge ─▶ pending_nmi ──[tick start]──▶ polled_nmi ──take──▶ CPU
mapper   ─▶ irq level   ──[tick start]──▶ polled_irq ──peek──▶ CPU
```

At the **start** of every `tick()` (and every `stall()` iteration —
stall cycles are cycles), the live state propagates into the shadow;
then the PPU steps its 3 dots and may raise new edges into the live
latch. The shadow therefore always holds "the interrupt state as of the
end of the **previous** cycle."

`Bus::take_pending_nmi()` drains the shadow (`polled_nmi`), not the live
latch. A new `polled_irq` carries the delayed IRQ level (sourced from
`mapper.irq_pending()`; M5b ORs in the APU frame/DMC IRQs here — that
future single point of integration is why M5a builds the IRQ side now).

### 4.2 Why this equals penultimate-cycle sampling

`Cpu::step`'s boundary check runs immediately after the instruction's
final cycle. The shadow at that moment holds state as of the end of the
final cycle *minus one* — the penultimate cycle. An NMI edge raised
during an instruction's last cycle is still in the live latch, not the
shadow, so it waits exactly one more instruction — matching real
hardware. No opcode handler changes; the 2.5M-case SingleStepTests
corpus (which checks per-cycle bus traces) is unaffected by
construction.

### 4.3 Trait surface

`BusLike` gains the shadow accessors with defaults preserving current
test behavior (`FlatBus` has no interrupt sources):

- `take_pending_nmi()` — semantics change to "drain shadow" in the
  production impl; `FlatBus` default stays `false`.
- `polled_irq_level() -> bool` — default `false`; production returns
  the delayed mapper-IRQ level. Note: today `Cpu::step` never consults
  the bus IRQ line at all (`mapper_irq_pending` exists on `BusLike` but
  is unwired — NROM never raises it). M5a wires the line into the
  service decision for the first time, already shadow-delayed, so M5b's
  APU IRQs arrive on a path with correct timing from day one. The
  un-delayed `mapper_irq_pending` accessor remains for the debugger.

`Cpu`'s own `irq_pending` field (used by unit tests to inject IRQs)
keeps working: the service condition becomes
`(self.irq_pending || bus.polled_irq_level()) && !I-flag`.

## 5. The two 6502 quirks

### 5.1 Branch interrupt quirk

A taken branch with **no page crossing** polls interrupts only during
its operand-fetch cycle; an interrupt asserted during its final cycle
waits one extra instruction. Untaken branches and page-crossing taken
branches poll normally (boundary shadow). Implementation: the single
branch helper in `instructions.rs` records the shadow state at fetch
time into a small `Cpu` field (e.g. `boundary_poll_override:
Option<(bool, bool)>`); `Cpu::step` consumes the override — if set, it
decides from the recorded values instead of the current shadow. One
helper + one field; no per-opcode edits. `ppu_vbl_nmi` 05/07/08
arbitrate the exact behavior.

### 5.2 Interrupt hijacking

An NMI edge that becomes visible during the early cycles of a BRK/IRQ
entry sequence **hijacks** the vector: the in-flight entry completes
but fetches $FFFA instead of $FFFE (and the NMI is consumed).
`service_interrupt` re-checks the shadow immediately before its
vector-fetch cycles and swaps the vector if an NMI has arrived. BRK's
handler shares `service_interrupt`'s tail, so it inherits the behavior.
The hijack window's exact boundary is pinned by `05-nmi_timing` /
`07-nmi_on_timing`.

## 6. PPU register write alignment

`Bus::write` currently runs the full tick (3 PPU dots) and then applies
the register write — so a PPUMASK write coincident with the odd-frame
skip dot lands ~3 dots late from the ROM's perspective
(`10-even_odd_timing`'s complaint, global spec §7.8).

Change: for PPU-register addresses ($2000-$3FFF), the tick is split —
step `k` dots, apply the write, step `3-k` dots. `k` is a named
constant pinned during implementation against test 10 (hardware places
the data-bus transaction mid-cycle; expect `k = 1` or `2`). All other
addresses keep the existing tick-then-apply order, and the APU step +
DMC service point in `tick` are invariant. If `06-suppression` or the
sprite-hit trio prove to need the same alignment on the **read** side
($2002/$2004 mid-cycle sampling), the identical split applies to
`Bus::read` for PPU registers — taken only if a test demands it, with
the full currently-green M2 suites as the regression guard.

## 7. Testing

- **Re-enabled gates:** remove `#[ignore]` from the 8 ROMs in
  `crates/nies-core/tests/test_roms.rs`; their pass criteria are the
  blargg result-byte protocol already used by the green tests.
- **New unit tests:** shadow propagation (edge in cycle N visible to
  boundary decisions only after cycle N+1; stall cycles propagate);
  branch-quirk override (taken/no-cross delays a final-cycle NMI by one
  instruction; untaken and page-cross don't); hijack (NMI during IRQ
  entry fetches $FFFA, NMI consumed); split-tick dot accounting (3
  dots per cycle, write lands after dot `k`).
- **Regression suites:** the full existing matrix, unchanged commands.
  SingleStepTests must stay green with zero trace diffs.
- **Hash re-pins:** run each determinism test, copy the new constants,
  update the 4 sites (2 native + 2 wasm), re-run native + wasm. The
  run-twice self-determinism assertions stay as the structural check.
- **Manual:** `demo_ntsc` line position eyeballed against a reference
  (cycle-accurate emulator screenshot or Nesdev description).

## 8. Execution and review style

Agent-dispatched units, user reviews diffs at per-unit checkpoints
(M3/M4 style), short pre-unit background notes. Anticipated units (the
implementation plan finalizes this):

1. **Shadow + step + quirks** — Bus pipeline, `BusLike` surface,
   `Cpu::step`, branch override, hijacking; unit tests; un-ignore and
   pass `ppu_vbl_nmi` 05/07/08.
2. **Suppression** — `06-suppression` green (read-side alignment only
   if required).
3. **Write alignment** — split tick, `10-even_odd_timing` + sprite-hit
   07/09/10 green.
4. **Re-pins + docs** — 4 hash constants, global spec §7.8/§8
   amendments (M5 → M5a/M5b split), CLAUDE.md state refresh.

## 9. Risks

- **A ROM still fails after a faithful implementation.** These tests
  are calibrated for exactly this work, so the expectation is all 8
  pass. If one resists: write the failure analysis into this spec's
  amendment (the M2 §7.8 pattern) rather than chasing tier-3 accuracy
  inline — but only after the analysis shows the root cause is outside
  penultimate-cycle polling / write alignment.
- **Subtle SingleStepTests regressions.** The corpus checks per-cycle
  bus traces; approach A doesn't change any handler's bus access
  pattern, so a corpus diff means an accidental behavior change — treat
  any corpus red as a stop-the-line bug.
- **Hash churn discipline.** Exactly one re-pin, at unit 4, after all 8
  ROMs are green — not incrementally per unit, so the constants move
  once and the diff history stays readable.
- **Split-tick blast radius.** The `Bus::write` ordering change affects
  every PPU register write; the M2 suites (vbl/sprite/OAM, all green
  today) are the guard. Any unexpected M2 failure means the chosen `k`
  or the split placement is wrong — fix there, don't patch the PPU.

## 10. Open questions resolved by this milestone

- None of the global spec §9 items. The §8 M5 entry is restructured
  into M5a/M5b; §7.8 deferred rows referencing "M5" become "M5a".
