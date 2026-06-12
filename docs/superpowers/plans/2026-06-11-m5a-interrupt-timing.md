# M5a — Cycle-Accurate Interrupt Timing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. **All units are agent-dispatched; the user reviews diffs at per-unit checkpoints.** Pause at each `--- CHECKPOINT ---`.

**Goal:** CPU interrupt servicing moves to penultimate-cycle semantics (via a one-cycle-delayed shadow in the Bus) and PPU register writes land at the hardware-correct dot — turning the 8 deferred blargg timing ROMs green and re-pinning both golden hashes once.

**Architecture:** A two-stage interrupt pipeline in `Bus`: every cycle starts by propagating the live NMI edge / IRQ level into `polled_*` shadow fields, then the PPU steps. The CPU's boundary decision reads the shadow — which is exactly "state as of the penultimate cycle" — so no opcode handler changes and the SingleStepTests corpus is unaffected by construction. Two targeted quirks: the branch helper records its poll at operand-fetch time, and BRK/IRQ vector fetches re-check the shadow for NMI hijacking. Separately, `Bus::write`/`read` for PPU registers split their tick so the register access lands mid-cycle at a test-pinned dot.

**Tech Stack:** Rust 2024, no new dependencies anywhere. Pure `nies-core` until the final docs task.

**Predecessor design spec:** [`docs/superpowers/specs/2026-06-10-m5a-interrupt-timing-design.md`](../specs/2026-06-10-m5a-interrupt-timing-design.md) — read it before this plan. The spec is rationale; this plan is execution.

---

## Reference: spec section ↔ task mapping

| Design spec § | Requirement | Tasks |
|---|---|---|
| §9 (hash churn discipline) | Goldens guarded during the re-pin window, moved once | 1, 10 |
| §4 | Interrupt shadow in `Bus` + `BusLike` surface | 2 |
| §4.2, §4.3 | `Cpu::step` decides from the shadow; IRQ line wired in | 3 |
| §5.1 | Branch interrupt quirk | 4 |
| §5.2 | NMI vector hijacking (BRK + IRQ) | 5 |
| §2 gate | `ppu_vbl_nmi` 05/07/08 green | 6 |
| §6 (read side) | `06-suppression` green (read alignment only if needed) | 7 |
| §6 | PPU write alignment; `10-even_odd_timing` green | 8 |
| §2 gate | sprite_hit 07/09/10 green | 9 |
| §2 gate, §7 | Re-pin 4 golden constants; comment updates; manual line check | 10 |
| §1, §10 | Global spec §7.8/§8 amendments; CLAUDE.md refresh | 11 |

---

## Notes for the implementer

- **Working directory** `/Users/eperdew/Software/nies`. Spec + this plan are on `master`. **First action:** `git checkout -b m5a-interrupt-timing master`, push, and open a draft PR after Task 1's commit so CI runs.
- **Current interrupt flow (verified):** `Bus::tick` (`bus.rs:71`) steps 3 PPU dots, latches NMI edges into `pending_nmi`, steps APU, increments `cycle`, services DMC. `Bus::stall` (`bus.rs:94`) is a tick-without-DMC loop. `Cpu::step` (`cpu/mod.rs:67`) drains `bus.take_pending_nmi()` at the instruction boundary, then checks `self.irq_pending` (a CPU-local test hook — **the bus IRQ line is currently never consulted**; `BusLike::mapper_irq_pending` exists but is unwired). `Bus::read`/`write` tick **first**, then decode (`read_no_tick`/`write_no_tick`). Every CPU cycle is exactly one bus access.
- **TDD throughout.** Failing test → predicted failure → implement → pass. A test failing *differently* than predicted = stop and investigate.
- **Stop-the-line rule (spec §9):** any SingleStepTests diff is a bug in your change, full stop. The corpus checks per-cycle bus traces and nothing in this plan changes any handler's access pattern.
- **Per-task commits**, single-quoted multi-line messages (no heredocs), `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`. `cargo fmt --all` + `cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings` clean before each commit.
- **Suite commands:** native `cargo test --workspace --exclude nies-web`; blargg ROM gates run via `cargo test -p nies-core --test test_roms <name> -- --ignored` while still ignored, then un-ignored in their gate task. wasm suite: `wasm-pack test --headless --chrome crates/nies-web`.
- **Blargg ROM harness:** `assert_rom_passes(path, max_cycles)` in `crates/nies-core/tests/test_roms.rs` runs the ROM and asserts the blargg result-byte protocol. Un-ignoring = deleting the `#[ignore = ...]` line (and fixing any now-stale comment above it).

---

## Task 1: Guard the golden hashes for the re-pin window

The shadow work (Tasks 2-5) changes NMI service timing, which changes the
`demo_ntsc` framebuffer and the input-demo RAM state — so all four golden
assertions would go red mid-milestone. Spec §9 wants the constants moved
**once**. Guard them now, before any behavior changes; Task 10 re-pins and
removes the guards. The run-twice self-determinism tests stay active
throughout (they are the structural determinism check and must keep
passing at every commit).

**Files:**
- Modify: `crates/nies-core/tests/ppu_determinism.rs`
- Modify: `crates/nies-core/tests/input_determinism.rs`
- Modify: `crates/nies-web/tests/determinism_wasm.rs`

- [ ] **Step 1: Ignore the two native golden tests**

In `ppu_determinism.rs`, on `demo_ntsc_framebuffer_matches_golden_hash`:

```rust
#[test]
#[ignore = "M5a re-pin window: NMI timing is changing; re-pinned in the M5a re-pin task"]
fn demo_ntsc_framebuffer_matches_golden_hash() {
```

Same attribute (same string) on `input_demo_matches_golden_hash` in
`input_determinism.rs`.

- [ ] **Step 2: Guard the two wasm golden tests**

`#[ignore]` interplay with `wasm_bindgen_test` is version-sensitive, so
use an explicit early return there instead. In
`crates/nies-web/tests/determinism_wasm.rs`, above the constants:

```rust
/// M5a re-pin window: NMI timing is changing on the m5a branch; both
/// golden constants get re-pinned in the M5a re-pin task, which flips
/// this back to false. The native self-determinism tests still guard
/// the determinism property meanwhile.
const REPIN_PENDING_M5A: bool = true;
```

First line of both `#[wasm_bindgen_test]` bodies:

```rust
    if REPIN_PENDING_M5A {
        return;
    }
```

- [ ] **Step 3: Verify**

Run: `cargo test -p nies-core --test ppu_determinism --test input_determinism`
Expected: golden tests show `ignored`; self-determinism + ring-buffer +
journal tests still pass.

- [ ] **Step 4: fmt, clippy, commit**

```bash
cargo fmt --all
cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
git add crates/nies-core/tests/ppu_determinism.rs crates/nies-core/tests/input_determinism.rs crates/nies-web/tests/determinism_wasm.rs
git commit -m 'test: guard golden hashes for the M5a re-pin window

NMI service timing changes during M5a, moving both golden constants.
Per the M5a spec (hash churn discipline) they move exactly once, at
the end: native goldens are #[ignore]d and the wasm twins early-return
behind REPIN_PENDING_M5A until the re-pin task. Self-determinism
checks stay active throughout.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>'
```

---

## Task 2: Interrupt shadow in `Bus`

**Files:**
- Modify: `crates/nies-core/src/bus.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `bus.rs`:

```rust
#[test]
fn nmi_edge_is_invisible_to_the_shadow_until_next_cycle() {
    let mut bus = fake_bus();
    // Simulate the PPU raising an edge mid-cycle: the live latch is set...
    bus.pending_nmi = true;
    // ...but the shadow (what the CPU's boundary decision sees) is empty.
    assert!(!bus.peek_polled_nmi());
    assert!(!bus.take_pending_nmi());
    // One cycle later (any bus access ticks), the edge has propagated.
    let _ = bus.read(0x0000);
    assert!(bus.peek_polled_nmi());
    assert!(bus.take_pending_nmi()); // drains the shadow...
    assert!(!bus.take_pending_nmi()); // ...exactly once
}

#[test]
fn shadow_propagation_consumes_the_live_edge() {
    let mut bus = fake_bus();
    bus.pending_nmi = true;
    let _ = bus.read(0x0000);
    // The edge moved into the shadow; the live latch is clear, so a
    // second cycle does not duplicate it.
    assert!(!bus.pending_nmi);
    let _ = bus.read(0x0000);
    assert!(bus.take_pending_nmi());
    assert!(!bus.take_pending_nmi());
}

#[test]
fn polled_irq_level_is_false_for_nrom() {
    let mut bus = fake_bus();
    let _ = bus.read(0x0000);
    // NROM never raises IRQ; the delayed level must stay false.
    assert!(!bus.polled_irq_level());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p nies-core --lib bus`
Expected: compile error — `peek_polled_nmi` / `polled_irq_level` not found.

- [ ] **Step 3: Implement the shadow**

In the `Bus` struct, after `pending_nmi`:

```rust
    /// Interrupt shadow (M5a, spec §4): the state of the interrupt lines
    /// as of the end of the *previous* CPU cycle. `begin_cycle` propagates
    /// the live latch/level here before the PPU steps, so a boundary
    /// decision reading the shadow has penultimate-cycle semantics.
    pub polled_nmi: bool,
    /// Delayed IRQ level (mapper now; APU frame/DMC IRQs OR in at M5b).
    pub polled_irq: bool,
```

Initialize both to `false` in `Bus::new`.

Refactor `tick` and `stall` onto two helpers (this also pre-stages Task
7/8's split ticks). Replace the current `tick` body and `stall` with:

```rust
    /// Start-of-cycle bookkeeping: propagate the live interrupt state
    /// into the shadow. Runs before this cycle's PPU dots, so the shadow
    /// holds "state as of the end of the previous cycle."
    fn begin_cycle(&mut self) {
        if self.pending_nmi {
            self.polled_nmi = true;
            self.pending_nmi = false;
        }
        self.polled_irq = {
            use crate::mapper::MapperImpl;
            self.mapper.irq_pending()
        };
    }

    /// Step the PPU `n` dots, latching any NMI edge into the live latch.
    fn step_ppu_dots(&mut self, n: u32) {
        for _ in 0..n {
            self.ppu.step(&mut self.mapper);
            if self.ppu.take_nmi() {
                self.pending_nmi = true;
            }
        }
    }

    /// Tick the rest of the system one CPU cycle. Called from every public
    /// `read`/`write`. See spec §3.3.
    fn tick(&mut self) {
        self.begin_cycle();
        // 3 PPU dots per CPU cycle (NTSC).
        self.step_ppu_dots(3);
        // 1 APU step per CPU cycle.
        self.apu.step(&mut self.mapper);
        self.cycle = self.cycle.wrapping_add(1);
        // Service any pending DMC fetch. The fetch performs a no-tick read
        // from CPU memory, delivers the sample to DMC, and adds the
        // configured stall by recursively ticking. M1 DMC is always idle
        // (pending_fetch is None), so the body is unreachable at M1; this
        // is here so M5's DMC code can land without bus changes.
        if let Some(addr) = self.apu.dmc.take_pending_fetch() {
            let val = self.read_no_tick(addr);
            self.apu.dmc.deliver_sample(val);
            self.stall(self.apu.dmc.stall_cycles());
        }
    }

    fn stall(&mut self, cycles: u32) {
        for _ in 0..cycles {
            self.begin_cycle();
            self.step_ppu_dots(3);
            self.apu.step(&mut self.mapper);
            self.cycle = self.cycle.wrapping_add(1);
        }
    }
```

Change `take_pending_nmi` to drain the **shadow**, and add the two new
accessors:

```rust
    /// Drain the *shadow* NMI latch — the edge as visible to a
    /// penultimate-cycle poll. An edge raised during the current cycle
    /// stays in `pending_nmi` until the next cycle's `begin_cycle`.
    pub fn take_pending_nmi(&mut self) -> bool {
        let v = self.polled_nmi;
        self.polled_nmi = false;
        v
    }

    /// Read the shadow NMI latch without draining (branch-quirk capture,
    /// vector hijack check).
    pub fn peek_polled_nmi(&self) -> bool {
        self.polled_nmi
    }

    /// The IRQ line level as of the end of the previous cycle.
    pub fn polled_irq_level(&self) -> bool {
        self.polled_irq
    }
```

Update the doc comment on the `pending_nmi` field: it is now the **live**
latch feeding the shadow, drained by `begin_cycle`, not by the CPU.

- [ ] **Step 4: Extend `BusLike`**

In the `BusLike` trait (defaults keep `FlatBus`/SingleStepTests
untouched — `FlatBus` has no interrupt sources):

```rust
    /// Shadow NMI peek (non-draining). Production: the M5a interrupt
    /// shadow. Tests: always false.
    fn peek_polled_nmi(&self) -> bool {
        false
    }
    /// Delayed IRQ line level (penultimate-cycle semantics).
    /// Production: mapper (and APU from M5b). Tests: always false.
    fn polled_irq_level(&self) -> bool {
        false
    }
```

And in `impl BusLike for Bus`:

```rust
    fn peek_polled_nmi(&self) -> bool {
        Bus::peek_polled_nmi(self)
    }
    fn polled_irq_level(&self) -> bool {
        Bus::polled_irq_level(self)
    }
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p nies-core --lib bus`
Expected: 3 new tests pass; all existing bus tests pass.

- [ ] **Step 6: Full suite — expect ONE behavioral note**

Run: `cargo test --workspace --exclude nies-web`
Expected: **all green.** The CPU still calls `take_pending_nmi` at
boundaries; the drain now lags edges by one cycle, which shifts NMI
service by one instruction at most. `nestest` never enables NMI;
`cpu_instrs`-style ROMs self-report via the result byte and tolerate
this. If anything fails here, STOP — do not proceed to Task 3 with a red
suite; report the failure.

- [ ] **Step 7: fmt, clippy, commit**

```bash
cargo fmt --all
cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
git add crates/nies-core/src/bus.rs
git commit -m 'feat(bus): two-stage interrupt shadow (penultimate-cycle poll)

begin_cycle propagates the live NMI edge / mapper IRQ level into
polled_* shadows before the PPU steps each cycle, so a boundary
decision reading the shadow samples the line state as of the
penultimate cycle (spec 2026-06-10 m5a §4). take_pending_nmi drains
the shadow; peek_polled_nmi and polled_irq_level added for the branch
quirk and vector hijack. tick/stall refactored onto
begin_cycle/step_ppu_dots, pre-staging the M5a split-tick work.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>'
```

---

## Task 3: `Cpu::step` — IRQ line wired in, shadow semantics tested

`Cpu::step`'s NMI path already reads the (now-delayed) shadow via
`take_pending_nmi`. This task wires the bus IRQ line into the service
decision for the first time (spec §4.3) and adds CPU-level tests with a
scripted bus double.

**Files:**
- Modify: `crates/nies-core/src/cpu/mod.rs`

- [ ] **Step 1: Write the failing tests**

The existing tests in `cpu/mod.rs` use a local flat test bus. Add a
scripted double and tests to the `tests` module (adapt the double's name
if one with this shape already exists — extend rather than duplicate):

```rust
    /// Flat 64 KiB bus with directly controllable interrupt shadow.
    struct ShadowBus {
        mem: Vec<u8>,
        polled_nmi: bool,
        polled_irq: bool,
    }

    impl ShadowBus {
        fn new() -> Self {
            ShadowBus {
                mem: vec![0; 0x10000],
                polled_nmi: false,
                polled_irq: false,
            }
        }
    }

    impl crate::bus::BusLike for ShadowBus {
        fn read(&mut self, addr: u16) -> u8 {
            self.mem[addr as usize]
        }
        fn write(&mut self, addr: u16, val: u8) {
            self.mem[addr as usize] = val;
        }
        fn take_pending_nmi(&mut self) -> bool {
            let v = self.polled_nmi;
            self.polled_nmi = false;
            v
        }
        fn peek_polled_nmi(&self) -> bool {
            self.polled_nmi
        }
        fn polled_irq_level(&self) -> bool {
            self.polled_irq
        }
    }

    #[test]
    fn bus_irq_level_triggers_service_when_i_clear() {
        let mut bus = ShadowBus::new();
        // IRQ vector -> $9000; program at $8000 is NOPs.
        bus.mem[0xFFFE] = 0x00;
        bus.mem[0xFFFF] = 0x90;
        bus.mem[0x8000] = 0xEA;
        let mut cpu = Cpu::new();
        cpu.pc = 0x8000;
        cpu.p = 0x24; // I clear
        bus.polled_irq = true;
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x9000, "bus IRQ level must be serviced");
        assert_ne!(cpu.p & flags::FLAG_I, 0);
    }

    #[test]
    fn bus_irq_level_masked_by_i_flag() {
        let mut bus = ShadowBus::new();
        bus.mem[0x8000] = 0xEA;
        let mut cpu = Cpu::new();
        cpu.pc = 0x8000;
        cpu.p = 0x24 | flags::FLAG_I;
        bus.polled_irq = true;
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x8001, "masked IRQ must not service");
    }

    #[test]
    fn nmi_beats_irq_at_the_same_boundary() {
        let mut bus = ShadowBus::new();
        bus.mem[0xFFFA] = 0x00;
        bus.mem[0xFFFB] = 0xA0; // NMI -> $A000
        bus.mem[0xFFFE] = 0x00;
        bus.mem[0xFFFF] = 0x90; // IRQ -> $9000
        let mut cpu = Cpu::new();
        cpu.pc = 0x8000;
        cpu.p = 0x24;
        bus.polled_nmi = true;
        bus.polled_irq = true;
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0xA000, "NMI has priority over IRQ");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p nies-core --lib cpu`
Expected: `bus_irq_level_triggers_service_when_i_clear` and
`nmi_beats_irq_at_the_same_boundary` may already... NO — expected:
`bus_irq_level_triggers_service_when_i_clear` FAILS (pc advances to
$8001: the bus IRQ line is not consulted yet). The other two pass
(NMI path already works; masked case trivially passes). Confirm exactly
that pattern.

- [ ] **Step 3: Wire the IRQ line**

In `Cpu::step`, change the IRQ condition (currently
`if self.irq_pending && (self.p & flags::FLAG_I) == 0`):

```rust
        if (self.irq_pending || bus.polled_irq_level()) && (self.p & flags::FLAG_I) == 0 {
            self.service_irq(bus);
            return;
        }
```

Update `service_irq`'s doc comment: the level now comes from the CPU
test hook **or** the bus's delayed line (mapper today, APU at M5b).

- [ ] **Step 4: Run tests**

Run: `cargo test -p nies-core --lib cpu` → all pass.

- [ ] **Step 5: Full suite, fmt, clippy, commit**

```bash
cargo test --workspace --exclude nies-web
cargo fmt --all
cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
git add crates/nies-core/src/cpu/mod.rs
git commit -m 'feat(cpu): service IRQs from the delayed bus line

Cpu::step now ORs bus.polled_irq_level() (the M5a shadow: mapper
today, APU frame/DMC at M5b) into the IRQ service decision — the
first time the bus IRQ line reaches the CPU. NMI keeps priority.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>'
```

---

## Task 4: Branch interrupt quirk

A taken branch with no page crossing polls interrupts only during its
operand-fetch cycle; an interrupt asserted during its final cycle waits
one extra instruction (spec §5.1). With the shadow: right after the
operand fetch's tick, the shadow holds state as of the end of the opcode
fetch — exactly the hardware's poll. Record it; `step` consumes it.

**Files:**
- Modify: `crates/nies-core/src/cpu/mod.rs` (field + `step` + `reset`)
- Modify: `crates/nies-core/src/cpu/instructions.rs` (`branch_if`, ~line 1403)

- [ ] **Step 1: Write the failing tests**

Append to `cpu/mod.rs` tests (using `ShadowBus` from Task 3):

```rust
    #[test]
    fn taken_branch_no_cross_delays_late_nmi_by_one_instruction() {
        let mut bus = ShadowBus::new();
        bus.mem[0xFFFA] = 0x00;
        bus.mem[0xFFFB] = 0xA0; // NMI -> $A000
        // $8000: BNE +2 (taken, no page cross) ; $8004: NOP at target
        bus.mem[0x8000] = 0xD0;
        bus.mem[0x8001] = 0x02;
        bus.mem[0x8004] = 0xEA;
        let mut cpu = Cpu::new();
        cpu.pc = 0x8000;
        cpu.p = 0x24; // Z clear -> BNE taken
        // Shadow is CLEAR at the branch's operand fetch (poll point)...
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x8004, "branch lands at target");
        // ...the edge becomes visible only after the branch retired:
        bus.polled_nmi = true;
        // Hardware: the branch already polled (negative); the NMI is
        // serviced only after the NEXT instruction.
        cpu.step(&mut bus); // executes the NOP at $8004
        assert_eq!(cpu.pc, 0xA000, "NMI serviced after the following instruction");
    }

    #[test]
    fn taken_branch_no_cross_services_nmi_seen_at_its_poll() {
        let mut bus = ShadowBus::new();
        bus.mem[0xFFFA] = 0x00;
        bus.mem[0xFFFB] = 0xA0;
        bus.mem[0x8000] = 0xD0;
        bus.mem[0x8001] = 0x02;
        let mut cpu = Cpu::new();
        cpu.pc = 0x8000;
        cpu.p = 0x24;
        bus.polled_nmi = true; // visible at the branch's poll point
        cpu.step(&mut bus); // branch executes...
        cpu.step(&mut bus); // ...and the boundary services the NMI
        assert_eq!(cpu.pc, 0xA000);
    }

    #[test]
    fn untaken_branch_polls_normally() {
        let mut bus = ShadowBus::new();
        bus.mem[0xFFFA] = 0x00;
        bus.mem[0xFFFB] = 0xA0;
        bus.mem[0x8000] = 0xD0; // BNE, NOT taken (Z set)
        bus.mem[0x8001] = 0x02;
        let mut cpu = Cpu::new();
        cpu.pc = 0x8000;
        cpu.p = 0x24 | flags::FLAG_Z;
        cpu.step(&mut bus); // untaken branch retires at $8002
        bus.polled_nmi = true;
        cpu.step(&mut bus); // normal boundary poll services immediately
        assert_eq!(cpu.pc, 0xA000);
    }
```

Note on the first test: with a `ShadowBus` (no ticking), "the edge
becomes visible after the branch" is modeled by setting `polled_nmi`
between `step` calls. The production per-cycle propagation is what
blargg 05/07/08 verify; these unit tests pin the override plumbing.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p nies-core --lib cpu`
Expected: `taken_branch_no_cross_delays_late_nmi_by_one_instruction`
FAILS (the NMI is serviced right after the branch — pc $A000 one step
early, so the assert on `0x8004`... read the failure: the second
`cpu.step` call services the NMI *instead of* executing the NOP, leaving
pc at $A000 — the test still ends at $A000 — **so write the assert
trail carefully**: the failure manifests on a THIRD observation. Replace
intuition with this: add after the branch's `cpu.step`:
`assert_eq!(cpu.pc, 0x8004)` (passes), then `bus.polled_nmi = true;`,
then `cpu.step`, then assert the NOP retired by checking
`assert_eq!(cpu.pc, 0xA000)` — without the quirk this passes too!).

**Correction — the discriminating test:** the difference is *which*
instruction the NMI interrupts. Use the return address pushed on the
stack:

```rust
    #[test]
    fn taken_branch_no_cross_delays_late_nmi_by_one_instruction() {
        let mut bus = ShadowBus::new();
        bus.mem[0xFFFA] = 0x00;
        bus.mem[0xFFFB] = 0xA0;
        bus.mem[0x8000] = 0xD0; // BNE +2 -> $8004
        bus.mem[0x8001] = 0x02;
        bus.mem[0x8004] = 0xEA; // NOP
        let mut cpu = Cpu::new();
        cpu.pc = 0x8000;
        cpu.p = 0x24;
        cpu.step(&mut bus); // branch retires; its recorded poll was clear
        bus.polled_nmi = true;
        cpu.step(&mut bus); // MUST execute the NOP (quirk), not service
        assert_eq!(cpu.pc, 0x8005, "NOP must retire before the NMI");
        cpu.step(&mut bus); // now the boundary services
        assert_eq!(cpu.pc, 0xA000);
    }
```

Without the quirk, the second `step` services immediately (pc $A000) —
clean failure. Use THIS version; the prose above documents why.

- [ ] **Step 3: Implement**

In `Cpu` (struct, `cpu/mod.rs`):

```rust
    /// Branch quirk (M5a spec §5.1): a taken branch with no page cross
    /// polls interrupts only at its operand-fetch cycle. The branch
    /// helper records (nmi, irq) here; the next boundary decision uses
    /// the recording instead of the live shadow. None = poll normally.
    pub(crate) branch_poll_override: Option<(bool, bool)>,
```

Initialize to `None` in `Default`/`new` and clear in `reset`.

In `Cpu::step`, replace the boundary block (the `take_pending_nmi` +
NMI + IRQ checks) with:

```rust
        // Boundary interrupt decision. A taken/no-cross branch recorded
        // its poll at operand-fetch time (spec §5.1); otherwise read the
        // shadow (penultimate-cycle state).
        let (nmi_now, irq_level) = match self.branch_poll_override.take() {
            Some((nmi, irq)) => {
                if nmi {
                    // Consume the edge we're about to service.
                    let _ = bus.take_pending_nmi();
                }
                (nmi, irq)
            }
            None => (bus.take_pending_nmi(), bus.polled_irq_level()),
        };
        if nmi_now {
            self.nmi_pending = true;
        }

        if self.jammed {
            // KIL/JAM/HLT halts the CPU until reset; just keep ticking
            // the bus so PPU/APU continue running.
            let _ = bus.read(self.pc);
            return;
        }

        // Interrupt servicing happens at instruction boundaries.
        if self.nmi_pending {
            self.service_nmi(bus);
            return;
        }
        if (self.irq_pending || irq_level) && (self.p & flags::FLAG_I) == 0 {
            self.service_irq(bus);
            return;
        }
```

(The `irq_level` from the match replaces the direct
`bus.polled_irq_level()` call from Task 3.)

In `instructions.rs`, replace `branch_if`:

```rust
/// issues a dummy read of the unmodified PC, computes the new PC, and
/// — if the branch crosses a page boundary — issues a second dummy read
/// at the unmasked-PC address before updating PC.
///
/// Interrupt quirk (M5a spec §5.1): a taken branch with no page cross
/// polls interrupts only during its operand-fetch cycle. At that point
/// the shadow holds state as of the opcode fetch — the hardware's poll —
/// so we record it for the boundary; the final cycle never re-polls.
fn branch_if<B: BusLike>(cpu: &mut Cpu, bus: &mut B, taken: bool) {
    let offset = addr::relative(cpu, bus);
    if taken {
        let new_pc = (cpu.pc as i32 + offset as i32) as u16;
        let crossed = (cpu.pc & 0xFF00) != (new_pc & 0xFF00);
        if !crossed {
            cpu.branch_poll_override =
                Some((bus.peek_polled_nmi(), bus.polled_irq_level()));
        }
        let _ = bus.read(cpu.pc); // dummy read at unmodified PC
        if crossed {
            // Page-crossed: extra dummy read at the unmasked-PC address.
            let _ = bus.read((cpu.pc & 0xFF00) | (new_pc & 0x00FF));
        }
        cpu.pc = new_pc;
    }
}
```

(Bus access order is unchanged: operand fetch, dummy read, optional
page-cross dummy — the corpus traces stay identical.)

- [ ] **Step 4: Run tests**

Run: `cargo test -p nies-core --lib cpu` → all pass, including Task 3's.

- [ ] **Step 5: Full suite, fmt, clippy, commit**

```bash
cargo test --workspace --exclude nies-web
cargo fmt --all
cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
git add crates/nies-core/src/cpu/mod.rs crates/nies-core/src/cpu/instructions.rs
git commit -m 'feat(cpu): branch interrupt quirk (taken/no-cross polls early)

A taken branch without page crossing polls interrupts only at its
operand-fetch cycle; an interrupt asserted during its final cycle
waits one extra instruction. branch_if records the shadow at the poll
point; the next boundary consumes the recording. Bus access order is
unchanged, so SingleStepTests traces are identical.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>'
```

---

## Task 5: NMI vector hijacking

**Files:**
- Modify: `crates/nies-core/src/cpu/mod.rs` (`service_interrupt`, helper)
- Modify: `crates/nies-core/src/cpu/instructions.rs` (BRK, ~line 691)

- [ ] **Step 1: Extend `ShadowBus`, then write the failing tests**

A subtlety drives the test design: `Cpu::step`'s boundary check runs
*before* the instruction executes, so a `polled_nmi` that is already
true at the boundary gets serviced instead of letting BRK/IRQ entry
begin. The shadow must rise **mid-sequence**. `ShadowBus` isn't
tick-coupled, so give it a one-shot fuse keyed on bus-access count.

Extend `ShadowBus` (from Task 3) with two fields and update both trait
methods:

```rust
    struct ShadowBus {
        mem: Vec<u8>,
        polled_nmi: bool,
        polled_irq: bool,
        /// One-shot fuse: raise `polled_nmi` after the Nth bus access.
        raise_polled_nmi_after: Option<u32>,
        accesses: u32,
    }
```

(initialize the new fields to `None` / `0` in `ShadowBus::new`), and as
the first lines of **both** `read` and `write`:

```rust
            self.accesses += 1;
            if Some(self.accesses) == self.raise_polled_nmi_after {
                self.polled_nmi = true;
            }
```

Then the tests. IRQ entry's bus accesses are: dummy, dummy, push PCH,
push PCL, push P, vector lo, vector hi — a fuse at 2 fires after the
second dummy, well before the vector fetch. BRK's are: padding fetch,
push, push, push, vector lo, vector hi — fuse at 3 fires after the
second push.

```rust
    #[test]
    fn nmi_hijacks_irq_vector_fetch() {
        let mut bus = ShadowBus::new();
        bus.mem[0xFFFA] = 0x00;
        bus.mem[0xFFFB] = 0xA0; // NMI -> $A000
        bus.mem[0xFFFE] = 0x00;
        bus.mem[0xFFFF] = 0x90; // IRQ -> $9000
        bus.mem[0xA000] = 0xEA; // NOP at the NMI handler
        let mut cpu = Cpu::new();
        cpu.pc = 0x8000;
        cpu.p = 0x24;
        cpu.irq_pending = true; // boundary check services IRQ...
        bus.raise_polled_nmi_after = Some(2); // ...NMI arrives mid-entry
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0xA000, "NMI must hijack the IRQ vector");
        assert!(!bus.polled_nmi, "the hijacking NMI edge is consumed");
        // The hijack satisfied the NMI; the next step must execute the
        // NOP, not re-service.
        cpu.irq_pending = false;
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0xA001);
    }

    #[test]
    fn nmi_hijacks_brk_vector_fetch() {
        let mut bus = ShadowBus::new();
        bus.mem[0xFFFA] = 0x00;
        bus.mem[0xFFFB] = 0xA0;
        bus.mem[0xFFFE] = 0x00;
        bus.mem[0xFFFF] = 0x90;
        bus.mem[0x8000] = 0x00; // BRK
        let mut cpu = Cpu::new();
        cpu.pc = 0x8000;
        cpu.p = 0x24;
        let sp_before = cpu.sp;
        bus.raise_polled_nmi_after = Some(3); // mid-entry, pre-vector
        cpu.step(&mut bus); // executes BRK; entry must land at $A000
        assert_eq!(cpu.pc, 0xA000, "NMI must hijack the BRK vector");
        assert!(!bus.polled_nmi, "the hijacking NMI edge is consumed");
        // BRK semantics preserved despite the hijack: P was pushed with
        // the B flag set (third push: at S_initial - 2).
        let pushed_p = bus.mem[(0x0100 | (sp_before as u16).wrapping_sub(2)) as usize];
        assert_ne!(pushed_p & flags::FLAG_B, 0, "B flag pushed by BRK");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p nies-core --lib cpu`
Expected: both hijack tests FAIL with pc = $9000 (vector not hijacked).

- [ ] **Step 3: Implement**

In `cpu/mod.rs`, a **module-level free function** (NOT inside
`impl Cpu` — `instructions.rs` calls it as `crate::cpu::hijackable_vector`,
and it takes no `self`). Place it above `impl Cpu`:

```rust
/// Late vector selection with NMI hijacking (M5a spec §5.2): BRK and
/// IRQ entries choose their handler address only at the vector-fetch
/// cycles; an NMI edge visible in the shadow by then steals the
/// vector (and is consumed by doing so). NMI entry ($FFFA) cannot be
/// hijacked.
pub(crate) fn hijackable_vector<B: crate::bus::BusLike>(bus: &mut B, base: u16) -> u16 {
    if base == 0xFFFE && bus.peek_polled_nmi() {
        let _ = bus.take_pending_nmi();
        0xFFFA
    } else {
        base
    }
}
```

In `service_interrupt`, immediately before the vector reads (after the
pushes and I-flag set), insert:

```rust
        let vector = hijackable_vector(bus, vector);
```

(make the parameter binding `mut`-free by shadowing as shown).

In `instructions.rs` BRK (0x00 arm), replace the two vector reads:

```rust
            // Read the handler vector — $FFFE, unless an NMI edge arrived
            // during the entry sequence and hijacks it (M5a spec §5.2).
            let vector = crate::cpu::hijackable_vector(bus, 0xFFFE);
            let lo = bus.read(vector) as u16;
            let hi = bus.read(vector + 1) as u16;
            cpu.pc = (hi << 8) | lo;
```

(Adjust the path to however `hijackable_vector` resolves from
`instructions.rs` — it's the same module tree; `super::` or
`crate::cpu::` per the existing imports.)

- [ ] **Step 4: Run tests**

Run: `cargo test -p nies-core --lib cpu` → all pass.

- [ ] **Step 5: Full suite, fmt, clippy, commit**

```bash
cargo test --workspace --exclude nies-web
cargo fmt --all
cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
git add crates/nies-core/src/cpu/mod.rs crates/nies-core/src/cpu/instructions.rs
git commit -m 'feat(cpu): NMI vector hijacking during BRK/IRQ entry

An NMI edge visible in the shadow by the vector-fetch cycles steals
the vector ($FFFA replaces $FFFE) and is consumed; the in-flight
entry sequence (pushes, flags) is otherwise unchanged. BRK shares the
hijackable_vector helper.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>'
```

---

## Task 6: Gate — `ppu_vbl_nmi` 05/07/08

**Files:**
- Modify: `crates/nies-core/tests/test_roms.rs` (lines ~318-360)

- [ ] **Step 1: Run the three ROMs while still ignored**

```bash
cargo test -p nies-core --test test_roms blargg_ppu_vbl_nmi_05 -- --ignored
cargo test -p nies-core --test test_roms blargg_ppu_vbl_nmi_07 -- --ignored
cargo test -p nies-core --test test_roms blargg_ppu_vbl_nmi_08 -- --ignored
```

Expected: ALL PASS. If one fails: the blargg on-screen/result-byte error
code identifies the failing sub-check — debug at the shadow/quirk/hijack
level (Tasks 2-5), not by fudging constants. The likely knobs, in order:
the hijack window (Task 5 checks the shadow once before vector fetch —
05/07 probe its exact boundary), the branch-quirk capture point, and the
NMI-edge propagation ordering in `begin_cycle`. If the failure resists a
few focused attempts, STOP and report with the result byte and your
analysis. Do not weaken any passing test.

- [ ] **Step 2: Un-ignore the three tests**

Delete the `#[ignore = ...]` lines from
`blargg_ppu_vbl_nmi_05_nmi_timing`, `blargg_ppu_vbl_nmi_07_nmi_on_timing`,
`blargg_ppu_vbl_nmi_08_nmi_off_timing` (06 stays ignored until Task 7).
Rewrite the now-stale comment block above test 05 (it currently says
"every opcode handler needs to participate" — wrong since approach A):

```rust
// 05/07/08 measure NMI dispatch timing at single-cycle precision. M5a's
// interrupt shadow gives the CPU penultimate-cycle sampling (see the M5a
// design spec §4), with the branch quirk and vector hijacking handled in
// the CPU; these went green at M5a. 06 (suppression) is gated separately
// on read-side alignment — see the M5a spec §6.
```

- [ ] **Step 3: Full suite**

Run: `cargo test --workspace --exclude nies-web`
Expected: all green, the three tests now counted in the run (not ignored).

- [ ] **Step 4: fmt, clippy, commit**

```bash
cargo fmt --all
cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
git add crates/nies-core/tests/test_roms.rs
git commit -m 'test(rom): un-ignore ppu_vbl_nmi 05/07/08 — NMI timing green

Penultimate-cycle polling (interrupt shadow + branch quirk + vector
hijacking) satisfies blargg NMI dispatch timing.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>'
```

--- CHECKPOINT 1 (Unit 1: shadow + quirks + NMI timing gates) — pause for user diff review ---

---

## Task 7: Gate — `06-suppression` (read alignment only if needed)

**Files:**
- Modify: `crates/nies-core/tests/test_roms.rs`
- Possibly modify: `crates/nies-core/src/bus.rs` (read split — only if Step 1 is red)

- [ ] **Step 1: Run 06 while still ignored**

```bash
cargo test -p nies-core --test test_roms blargg_ppu_vbl_nmi_06 -- --ignored
```

If PASS → skip to Step 4. If FAIL → the read-side alignment is needed
(the $2002 read currently samples PPU state after all 3 dots of its
cycle; the suppression race wants the mid-cycle value). Continue with
Step 2.

- [ ] **Step 2 (conditional): Split-tick PPU reads**

In `bus.rs`, add next to `tick` (using the Task 2 helpers):

```rust
    /// How many of a cycle's 3 PPU dots run before a PPU register
    /// access lands (M5a spec §6). The data-bus transaction happens
    /// mid-cycle on hardware; the exact dot is pinned by blargg
    /// 06-suppression (reads) / 10-even_odd_timing (writes).
    const PPU_REG_ACCESS_DOT: u32 = 2;

    /// One CPU cycle in which a PPU register *read* lands mid-cycle.
    fn tick_with_ppu_read(&mut self, addr: u16) -> u8 {
        self.begin_cycle();
        self.step_ppu_dots(Self::PPU_REG_ACCESS_DOT);
        let val = self.ppu.cpu_read(&mut self.mapper, addr);
        self.step_ppu_dots(3 - Self::PPU_REG_ACCESS_DOT);
        self.apu.step(&mut self.mapper);
        self.cycle = self.cycle.wrapping_add(1);
        if let Some(a) = self.apu.dmc.take_pending_fetch() {
            let v = self.read_no_tick(a);
            self.apu.dmc.deliver_sample(v);
            self.stall(self.apu.dmc.stall_cycles());
        }
        val
    }
```

And route it in `Bus::read`:

```rust
    pub fn read(&mut self, addr: u16) -> u8 {
        let val = if (0x2000..=0x3FFF).contains(&addr) {
            self.tick_with_ppu_read(addr)
        } else {
            self.tick();
            self.read_no_tick(addr)
        };
        self.open_bus = val;
        val
    }
```

**Watch for double-stepping:** `read_no_tick`'s $2000-$3FFF arm still
exists for the DMC path and the no-tick debugger callers — the routed
`read` above no longer goes through it for PPU registers, so no PPU
register access happens twice. Add a bus unit test pinning dot
accounting:

```rust
    #[test]
    fn ppu_register_read_costs_exactly_one_cycle_and_three_dots() {
        let mut bus = fake_bus();
        let cycle_before = bus.cycle;
        let dots_before = bus.ppu.state.dot;
        let _ = bus.read(0x2002);
        assert_eq!(bus.cycle, cycle_before + 1);
        assert_eq!(bus.ppu.state.dot, dots_before + 3);
    }
```

(If `state.dot` wraps at scanline end, pick a fresh `fake_bus` — dot 0
start makes +3 safe.)

Re-run 06 (`-- --ignored`); if still red, try `PPU_REG_ACCESS_DOT = 1`.
If neither value passes, STOP and report (result byte + analysis) — do
not try exotic splits without review.

**Regression guard:** the full M2 PPU matrix must stay green after this
change (`cargo test -p nies-core --test test_roms`): the vbl set/clear
times in 01-04 read $2002 too. Any newly-red M2 test means the split
placement is wrong (spec §9) — fix here, don't touch the PPU.

- [ ] **Step 3 (conditional): also confirm 05/07/08 still green**

```bash
cargo test -p nies-core --test test_roms blargg_ppu_vbl_nmi
```

- [ ] **Step 4: Un-ignore 06, full suite**

Delete its `#[ignore = ...]` line. Run the full native suite → green.

- [ ] **Step 5: fmt, clippy, commit**

```bash
cargo fmt --all
cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
git add crates/nies-core/tests/test_roms.rs crates/nies-core/src/bus.rs
git commit -m 'test(rom): un-ignore ppu_vbl_nmi 06 — suppression green

[Adjust body to match reality: either "passes with the shadow alone —
no read-side alignment needed" (bus.rs not staged), or "PPU register
reads land mid-cycle (dot K) via a split tick; M2 vbl suites stay
green".]

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>'
```

--- CHECKPOINT 2 (Unit 2: suppression) — pause for user diff review ---

---

## Task 8: PPU write alignment + `10-even_odd_timing`

**Files:**
- Modify: `crates/nies-core/src/bus.rs`
- Modify: `crates/nies-core/tests/test_roms.rs`

- [ ] **Step 1: Write the failing dot-accounting test**

Append to `bus.rs` tests:

```rust
    #[test]
    fn ppu_register_write_costs_exactly_one_cycle_and_three_dots() {
        let mut bus = fake_bus();
        let cycle_before = bus.cycle;
        let dots_before = bus.ppu.state.dot;
        bus.write(0x2001, 0x1E);
        assert_eq!(bus.cycle, cycle_before + 1);
        assert_eq!(bus.ppu.state.dot, dots_before + 3);
    }
```

(This passes today too — it's the invariant that must SURVIVE the split.
Commit it with the split, not before.)

- [ ] **Step 2: Implement the write split**

Mirroring Task 7's read split (and sharing `PPU_REG_ACCESS_DOT` — if
Task 7 was skipped because 06 passed without it, introduce the constant
here; if Task 7 introduced it with value 1, keep both paths on the same
constant unless test results force them apart, in which case split into
`PPU_REG_READ_DOT`/`PPU_REG_WRITE_DOT` with a comment naming which test
pinned each):

```rust
    /// One CPU cycle in which a PPU register *write* lands mid-cycle
    /// (M5a spec §6 — fixes 10-even_odd_timing's complaint that a
    /// PPUMASK write coincident with the odd-frame skip dot landed a
    /// full cycle late).
    fn tick_with_ppu_write(&mut self, addr: u16, val: u8) {
        self.begin_cycle();
        self.step_ppu_dots(Self::PPU_REG_ACCESS_DOT);
        self.open_bus = val;
        self.ppu.cpu_write(&mut self.mapper, addr, val);
        self.step_ppu_dots(3 - Self::PPU_REG_ACCESS_DOT);
        self.apu.step(&mut self.mapper);
        self.cycle = self.cycle.wrapping_add(1);
        if let Some(a) = self.apu.dmc.take_pending_fetch() {
            let v = self.read_no_tick(a);
            self.apu.dmc.deliver_sample(v);
            self.stall(self.apu.dmc.stall_cycles());
        }
    }
```

Route in `Bus::write`:

```rust
    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x4014 => {
                self.tick();
                self.do_oamdma(val);
            }
            0x2000..=0x3FFF => self.tick_with_ppu_write(addr, val),
            _ => {
                self.tick();
                self.write_no_tick(addr, val);
            }
        }
    }
```

($4014 keeps its existing path — OAMDMA is CPU-side, not a PPU register
in the $2000-$3FFF decode. `write_no_tick` still handles $2000-$3FFF for
the debugger force-write path; the routed `write` no longer reaches it
for those addresses, so nothing double-applies.)

- [ ] **Step 3: Run bus tests + the M2 matrix**

```bash
cargo test -p nies-core --lib bus
cargo test -p nies-core --test test_roms
```

Expected: all green (the M2 vbl/sprite/OAM suites are the blast-radius
guard, spec §9).

- [ ] **Step 4: Run 10-even_odd_timing while ignored; pin the dot**

```bash
cargo test -p nies-core --test test_roms blargg_ppu_vbl_nmi_10 -- --ignored
```

If red with `PPU_REG_ACCESS_DOT = 2`, try `1` (re-run Step 3's matrix
AND 06 after any change). If neither passes, STOP and report. Then
delete test 10's `#[ignore = ...]` line (and its stale comment about
"sub-step register granularity deferred").

- [ ] **Step 5: Full suite, fmt, clippy, commit**

```bash
cargo test --workspace --exclude nies-web
cargo fmt --all
cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
git add crates/nies-core/src/bus.rs crates/nies-core/tests/test_roms.rs
git commit -m 'feat(bus): PPU register writes land mid-cycle; vbl_nmi 10 green

Split tick for $2000-$3FFF writes: K dots, apply, 3-K dots (K pinned
by 10-even_odd_timing). Total dots/cycle and APU/DMC ordering are
invariant; the M2 vbl/sprite/OAM matrix guards the blast radius.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>'
```

---

## Task 9: Gate — sprite_hit 07/09/10

**Files:**
- Modify: `crates/nies-core/tests/test_roms.rs` (lines ~480-520)

- [ ] **Step 1: Run the trio while ignored**

```bash
cargo test -p nies-core --test test_roms sprite_hit -- --ignored
```

Expected: PASS — these measure sprite-0-hit observability via precisely
timed $2002 reads, which is the shadow (Tasks 2-5) plus read alignment
(Task 7, if taken). If red AND Task 7 was skipped (06 passed without the
read split): implement Task 7 Step 2's read split now and re-run
everything (06, 05/07/08, M2 matrix, this trio). If red with the read
split in place, STOP and report per the spec §9 failure-analysis policy.

- [ ] **Step 2: Un-ignore all three; full suite**

Delete the three `#[ignore = ...]` lines (07.screen_bottom,
09.timing_basics, 10.timing_order) and update their M2-deferral comments
to record the M5a resolution. Run the full native suite → green; the
sprite-hit suite is now 11/11 and ppu_vbl_nmi 10/10.

- [ ] **Step 3: fmt, clippy, commit**

```bash
cargo fmt --all
cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
git add crates/nies-core/tests/test_roms.rs crates/nies-core/src/bus.rs
git commit -m 'test(rom): un-ignore sprite_hit 07/09/10 — timing trio green

Penultimate-cycle polling plus mid-cycle PPU register access timing
satisfies the sprite-0-hit observability tests. ppu_vbl_nmi 10/10,
sprite_hit 11/11.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>'
```

(Drop `bus.rs` from the `git add` if Step 1 needed no code.)

--- CHECKPOINT 3 (Unit 3: write alignment + sprite trio; all 8 ROMs green) — pause for user diff review ---

---

## Task 10: Re-pin the golden hashes

**Files:**
- Modify: `crates/nies-core/tests/ppu_determinism.rs`
- Modify: `crates/nies-core/tests/input_determinism.rs`
- Modify: `crates/nies-web/tests/determinism_wasm.rs`

- [ ] **Step 1: Compute the new constants**

Remove the two native `#[ignore = ...]` guards (Task 1). Run:

```bash
cargo test -p nies-core --test ppu_determinism --test input_determinism
```

Both golden tests FAIL printing actual vs pinned. Copy the two actual
values into `GOLDEN_FB_HASH` and `GOLDEN_INPUT_HASH`. Re-run → PASS.

- [ ] **Step 2: Update the stale timing comments**

`ppu_determinism.rs`: the `KNOWN PRE-M5 TIMING` paragraph on
`GOLDEN_FB_HASH` is now resolved — replace it with:

```rust
/// Re-pinned at M5a: per-cycle interrupt polling moved demo_ntsc's
/// NMI-synchronized line to its cycle-accurate position (the pre-M5a
/// value encoded instruction-boundary NMI dispatch; see the M5a design
/// spec). A change from HERE on is a real determinism regression.
```

`input_determinism.rs`: same treatment for its golden comment (drop the
"will need re-pinning at M5" sentence; note the M5a re-pin).

- [ ] **Step 3: Update the wasm twins**

In `determinism_wasm.rs`: set both constants to the new values, flip
`REPIN_PENDING_M5A` handling by **deleting** the const and both
early-return blocks (the guard was a one-milestone scaffold, not a
keeper), and update the "MUST match" comments. Run:

```bash
wasm-pack test --headless --chrome crates/nies-web
```

Expected: 2/2 pass with the new constants.

- [ ] **Step 4: Manual check — demo_ntsc line position** *(user-assisted)*

```bash
RUST_LOG=info cargo run -p nies-app
```

The NMI-synchronized middle line should now sit at its cycle-accurate
position (vs. the documented pre-M5a leftward shift). The user eyeballs
this at the checkpoint; flag it in the checkpoint summary.

- [ ] **Step 5: Full suite, wasm clippy, fmt, clippy, commit**

```bash
cargo test --workspace --exclude nies-web
cargo fmt --all
cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
cargo clippy -p nies-web --target wasm32-unknown-unknown --all-targets -- -D warnings
git add crates/nies-core/tests/ppu_determinism.rs crates/nies-core/tests/input_determinism.rs crates/nies-web/tests/determinism_wasm.rs
git commit -m 'test: re-pin both golden hashes for cycle-accurate NMI timing

The one planned re-pin (documented since M3/M4): per-cycle interrupt
polling moves demo_ntsc'"'"'s NMI-synced line to its accurate position
and shifts the input-demo poll cycles. Native + wasm constants updated
together; re-pin guards removed.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>'
```

(Note the `'"'"'` escape for the apostrophe in `demo_ntsc's` — or
reword the body to avoid the apostrophe entirely, e.g. "moves the
demo_ntsc NMI-synced line"; prefer rewording.)

---

## Task 11: Documentation close-out

**Files:**
- Modify: `docs/superpowers/specs/2026-05-02-nes-emulator-design.md` (§7.8, §8)
- Modify: `CLAUDE.md`

- [ ] **Step 1: Global spec §8 — record the M5 split**

Update the roadmap line:

```text
M0  → M1 → M2 → M3 → M4 → M5a → M5b → 🎯 M6 → M7 → M8 → M9 → M10 → (post-v1: M11+)
```

Replace the `### M5 — APU + audio` heading and intro with two entries:

```markdown
### M5a — Cycle-accurate interrupt timing

CPU interrupt sampling moves to penultimate-cycle semantics; PPU
register accesses land mid-cycle. Split from M5 (2026-06-10) so the
CPU refactor is validated in isolation before the APU depends on it.

- Two-stage interrupt shadow in the bus; branch quirk; NMI vector
  hijacking; bus IRQ line wired into the CPU service decision.
- PPU register access dot alignment (split tick).

**Gate:** `ppu_vbl_nmi` 10/10 and `sprite_hit` 11/11 (the 8 §7.8
deferrals re-enabled); all prior suites green; both golden hashes
re-pinned once.

> **Status (M5a complete):** [fill in at close: approach (bus shadow),
> the pinned PPU_REG_ACCESS_DOT value, whether the read split was
> needed, new hash constants by reference]. See
> [`2026-06-10-m5a-interrupt-timing-design.md`](2026-06-10-m5a-interrupt-timing-design.md).

### M5b — APU + audio

Sound works; pacing switches to audio-driven. (Spec to be written
after M5a lands.)
```

…and keep the original M5 bullet list (channels, frame counter, DMC,
resampler, cpal, volume hooks) under M5b, dropping any per-cycle-polling
phrasing (now done in M5a). The original M5 gate text (apu_test,
dmc_tests, apu_mixer, instr_timing un-ignores) stays under M5b.

- [ ] **Step 2: Global spec §7.8 — close the deferred rows**

For the 8 rows deferring to "M5" (and the M3-observation paragraph at
the section's end): annotate each as resolved, e.g. change the
"Re-enable at" cell to `**M5a — done**` and append one sentence to the
M3-observation paragraph: "Resolved at M5a: per-cycle polling landed and
the framebuffer hash was re-pinned." Do not delete the rows — they are
the historical record of *why* the deferrals existed.

- [ ] **Step 3: CLAUDE.md refresh**

- Intro line → `Current state: M5a (cycle-accurate interrupt timing)
  complete; next is M5b (APU + audio).`
- Plans line → most recent `2026-06-11-m5a-interrupt-timing.md`
  (complete); M5b has no spec/plan yet.
- "What's intentionally NOT in scope" → retitle the list `For M5b
  specifically (next up):` and drop the bullet about replay/journal if
  unchanged, keeping the list current (gamepad/rebinding M10, volume
  UI M10, save states M7, replay M8, debugger M9, non-NROM mappers
  M11+).
- Known-gotchas hash bullet: replace the "will be re-pinned at M5"
  sentence with "both constants were re-pinned at M5a (cycle-accurate
  NMI timing); any change from here on is a regression."

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-05-02-nes-emulator-design.md CLAUDE.md
git commit -m 'docs: record M5a completion; split M5 into M5a/M5b in roadmap

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>'
```

- [ ] **Step 5: CI + merge**

- Push `m5a-interrupt-timing`; all five CI jobs green on the draft PR.
- Mark ready for review; the **user merges** (master pushes need
  explicit authorization per CLAUDE.md).

--- CHECKPOINT 4 (Unit 4: re-pins + docs + CI) ---

---

## Risks recap (from the design spec §9)

- **A ROM resists** → STOP at its gate task with the result byte and an
  analysis; the failure-analysis-into-spec path needs user signoff —
  never weaken a test to get green.
- **SingleStepTests diff** → stop-the-line; nothing here changes handler
  access patterns, so a corpus diff is an accidental behavior change.
- **One re-pin only** → goldens guarded in Task 1, moved in Task 10;
  no constants change in between.
- **Split-tick blast radius** → the M2 matrix runs inside Tasks 7/8/9;
  any newly-red M2 test means the split is wrong — fix the split.
