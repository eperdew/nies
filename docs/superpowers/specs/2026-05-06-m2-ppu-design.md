# M2 PPU — Design

**Status:** Draft for implementation planning
**Date:** 2026-05-06
**Author:** Eric Perdew (with Claude Code as co-designer)
**Predecessor:** [`2026-05-02-nes-emulator-design.md`](2026-05-02-nes-emulator-design.md) §3.5, §8 M2

## 1. Purpose and scope

This document captures the design decisions specific to milestone M2 (PPU implementation), refining the PPU section of the global design spec with the choices made during M2 brainstorming. It exists separately from the global spec because M2 surfaces enough non-obvious tradeoffs (mirroring location, NMI signal shape, golden-hash deferral, dispatch granularity) that recording the rationale once, here, beats spreading it across the implementation plan or the global spec.

The M2 implementation plan (`docs/superpowers/plans/2026-05-06-m2-ppu.md`, to be authored next via the `superpowers:writing-plans` skill) operationalizes this design into per-task steps with commit messages.

### 1.1 What M2 ships

The 2C02 PPU as a per-dot state machine, with NROM CHR access and nametable mirroring complete. Per spec §3.5: per-dot state, background fetch pipeline, sprite evaluation, vblank/NMI per spec, A12 line plumbed (no-op for NROM), `[u8; 256 * 240]` palette-index framebuffer per frame.

OAMDMA at $4014 — currently a stub in [`crates/nies-core/src/bus.rs`](../../../crates/nies-core/src/bus.rs) — is filled in here as well, since it's PPU-adjacent and one of the gate ROMs needs it.

### 1.2 What M2 does not ship

- Frontend rendering (M3) — the framebuffer remains a palette-index buffer inside `nies-core`; nothing is drawn on screen.
- Controller strobe (M4), APU sample generation (M5).
- MMC3-specific A12 timing — A12 is plumbed and the code path exercised, but no assertions on filter behavior. NROM ignores `notify_a12`.
- Save state coverage of new PPU fields (M7) — `SnapshotComponent` shape exists; golden snapshot tests come at M7.
- Runtime-mutable mirroring (MMC5, Namco 163, post-v1) — the design admits a clean extension point, but M2 doesn't build it.

### 1.3 M2 acceptance gate

- `blargg/ppu_vbl_nmi/01..10` all green (10 sub-tests, $6000 status protocol).
- `sprite_hit_tests_2005.10.05/*` all green (10 sub-tests, $6000 status protocol).
- `oam_read.nes` and `oam_stress.nes` green ($6000 status protocol).
- `nmi_sync/demo_ntsc.nes` produces a *self-deterministic* framebuffer hash (run twice, hashes equal — determinism check, not correctness; correctness check is deferred to M3).
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings` clean.
- `cargo clippy -p nies-web --target wasm32-unknown-unknown --all-targets -- -D warnings` clean.
- Branch merged to `master` via the `superpowers:finishing-a-development-branch` skill.

## 2. Module layout inside `nies-core`

The 59-line stub at [`crates/nies-core/src/ppu/mod.rs`](../../../crates/nies-core/src/ppu/mod.rs) is replaced by a focused module tree:

```
ppu/
  mod.rs              // pub Ppu, top-level step()/cpu_read/cpu_write, frame buffer
  state.rs            // PpuState: dot, scanline, frame_parity, internal latches
  registers.rs        // PPUCTRL/MASK/STATUS/OAMADDR/OAMDATA/PPUSCROLL/PPUADDR/PPUDATA
                      // Plus the v/t/x/w internal registers (per Loopy's docs)
  vram.rs             // 2KB nametable RAM + nametable mirroring helper
  oam.rs              // primary OAM (256B) + secondary OAM (32B)
  background.rs       // 8-cycle fetch pipeline + 16-bit shifters
  sprite.rs           // sprite eval (dots 65–256), fetch (257–320), sprite-0 hit
  palette.rs          // 32-byte palette RAM + index lookup (with $3F10/$14/$18/$1C mirrors)
```

Each file stays focused; per the global spec's design-for-isolation principle, every file should be under ~250 lines and answer one question (mirroring rules, fetch pipeline, sprite eval, etc.). When a file outgrows that, the work to do is splitting responsibilities, not accepting larger files.

## 3. The PPU↔Bus↔CPU surface

```rust
impl Ppu {
    pub fn new() -> Self;
    pub fn step(&mut self, mapper: &mut MapperKind);
    pub fn cpu_read(&mut self, mapper: &mut MapperKind, addr: u16) -> u8;  // $2000-$2007
    pub fn cpu_write(&mut self, mapper: &mut MapperKind, addr: u16, val: u8);
    pub fn cpu_peek(&self, mapper: &MapperKind, addr: u16) -> u8;          // debugger
    pub fn take_nmi(&mut self) -> bool;        // edge-triggered: returns true once on rising edge
    pub fn frame(&self) -> &[u8; 256 * 240];   // current framebuffer (palette indices 0..63)
    pub fn frame_parity(&self) -> bool;        // for tests / debugger
}
```

### 3.1 NMI wiring (take-on-read semantics)

`Bus::tick` calls `ppu.step(mapper)`, then checks `ppu.take_nmi()`; on `true` it sets `cpu.nmi_pending = true`. The CPU samples `nmi_pending` at instruction boundaries (existing M1 code).

**Why take-on-read, not a level signal:** NMI is edge-triggered. The CPU is mid-instruction when vblank arrives most of the time, and we don't want a level-held line to re-fire repeatedly during the long string of `Bus::tick` calls inside one CPU instruction. `take_nmi()` returns `true` exactly once per rising edge of (PPUCTRL bit 7 ∧ vblank flag), then resets internally. The fact that the CPU may not service the NMI until several instructions later is fine — `Cpu::nmi_pending` accumulates the latch.

The dot-precise PPUSTATUS-read suppression behavior (a $2002 read at the dot-0 / dot-1 boundary of scanline 241 suppresses NMI for that frame) is implemented inside `Ppu`, not in the bus or CPU. The suppression decides whether to *raise* the NMI edge in the first place; the bus simply transports whatever edge the PPU produces.

### 3.2 OAMDMA at $4014

`Bus::write` $4014 case — currently a TODO — fills in the 256-byte transfer. Spec §3.6 already prescribed the shape: 256 reads from CPU memory + 256 writes to $2004, with 1 alignment cycle on even CPU cycles or 2 on odd, totaling 513 or 514 cycles. Implementation uses `read_cpu_memory_no_tick` + `ppu.cpu_write(0x2004, ...)`. The bus advances the cycle counter and ticks the PPU/APU appropriately.

OAMDMA goes into the same dispatch unit as the background fetch pipeline (unit 3); it's hot-path code adjacent to bus tick discipline and trivial relative to the rest of the unit.

### 3.3 Mirroring lives in the PPU

The 2KB nametable RAM is owned by `Ppu`, not by the mapper. For PPU reads/writes to $2000–$2FFF, the PPU computes the mirroring index itself using `mapper.mirroring()` (the existing `Mirroring` enum from [`crates/nies-core/src/cartridge.rs`](../../../crates/nies-core/src/cartridge.rs): `Vertical`, `Horizontal`, `SingleScreen`, `FourScreen`) and indexes into its own array. The mapper trait's `ppu_read`/`ppu_write` only ever handle $0000–$1FFF (CHR); requests in the $2000+ range are not forwarded.

**Why this choice (chosen over mirroring-in-mapper):**

1. **Hardware fidelity.** The 2KB nametable RAM physically lives inside the NES console, not the cartridge. Mirroring-in-PPU matches that.
2. **Save state shape.** PPU's nametable RAM goes in the PPU snapshot section; mapper-specific RAM (CHR-RAM, PRG-RAM, four-screen extension) goes in the mapper snapshot. Clean separation when M7 lands.
3. **Mapper trait stays small.** Today the trait is `cpu_read/cpu_peek/cpu_write/ppu_read/ppu_write/mirroring/notify_a12/irq_pending/debug_dump`. Pushing nametable handling into the mapper would inflate it.
4. **Performance.** Direct array indexing is faster than enum-dispatch into mapper code that does the same indexing. Negligible in practice but free.

**The post-v1 cost (acknowledged):** four-screen mirroring (Gauntlet, Rad Racer 2, M11+) and runtime-mutable mirroring (MMC5, Namco 163, post-v1) want the mapper to participate. The plan when those land is to add an *optional* mapper hook — `nametable_read(addr) -> u8` and `nametable_write(addr, val)` with default impls that preserve current behavior — and have the PPU call it for $2000–$2FFF accesses. That's a real refactor of the hot path but a localized one. The user has noted the refactor will be instructive in its own right; we are not optimizing M2 to avoid it.

### 3.4 Frame buffer

A single `[u8; 256 * 240]` owned by the PPU. Filled in-place during rendering (no double-buffering at M2). Tests can read it at any point; the *meaningful* read is post-vblank. M3 may add double-buffering for tearing-free display, but that's a renderer concern.

## 4. Test ROM strategy

### 4.1 ROMs vendored at M2

| Path | Source | Purpose | Dispatch unit |
|---|---|---|---|
| `tests/roms/blargg/ppu_vbl_nmi/01-vbl_basics.nes` … `10-even_odd_timing.nes` | blargg `ppu_vbl_nmi.zip` | Vblank/NMI timing | 2 |
| `tests/roms/blargg/sprite_hit_tests_2005.10.05/01..10.nes` | blargg `sprite_hit_tests_2005.10.05.zip` | Sprite-0 hit | 4 |
| `tests/roms/blargg/oam_read.nes` | blargg `oam_read.zip` | OAM read | 5 |
| `tests/roms/blargg/oam_stress.nes` | blargg `oam_stress.zip` | OAM edge cases | 5 |
| `tests/roms/nmi_sync/demo_ntsc.nes` | blargg/nmi_sync archive | Determinism check | 3 |

23 ROMs total; combined size well under 1 MB. Not vendored via Git LFS (LFS is reserved for the SingleStepTests corpus per the existing convention). `LICENSES.md` gains a section enumerating each with source URL, author, redistribution permission, and SHA-256 hash, in the same shape as the M1 entries.

### 4.2 Test harness reuse

[`crates/nies-core/tests/test_roms.rs`](../../../crates/nies-core/tests/test_roms.rs) already implements the $6000 status protocol via `run_test_rom(path, max_cycles)`. `ppu_vbl_nmi`, `sprite_hit_tests`, `oam_read`, and `oam_stress` all use that protocol — they slot in identically to the M1 `cpu_instrs` sub-tests. One `#[test]` per ROM, named `blargg_ppu_vbl_nmi_01_basics`, etc.

### 4.3 The `nmi_sync/demo_ntsc.nes` determinism check

`demo_ntsc.nes` does *not* use the $6000 status protocol — it's a pure framebuffer artifact. The original M2 gate text in spec §8 called for "matches golden framebuffer hash," but at M2 we have no renderer, no frame-diff tool, and no debugger UI. A hash mismatch would yield "expected X, got Y" with zero diagnostic signal: we'd be guessing whether the bug is a wrong tile, an off-by-one scanline, a sprite priority issue, or a palette miss.

The M2 gate is therefore a *self-determinism* check, not a correctness check:

```rust
#[test]
fn nmi_sync_demo_ntsc_is_deterministic() {
    let h1 = run_and_hash_frame(N_FRAMES, "nmi_sync/demo_ntsc.nes");
    let h2 = run_and_hash_frame(N_FRAMES, "nmi_sync/demo_ntsc.nes");
    assert_eq!(h1, h2, "framebuffer hash differs across two identical runs");
}
```

`run_and_hash_frame` boots the ROM, runs N frames (likely 100–300; final value chosen during implementation), reads `bus.ppu().frame()`, hashes with `std::hash::DefaultHasher` (no new dep). The hash is *not* compared to a reference value.

This catches non-determinism — a real bug class for us, given the determinism contract from spec §4.1 — without inventing a correctness oracle we can't debug.

The correctness check (golden hash captured from Mesen 2 reference) is deferred to M3, where the renderer + frame-diff tooling will exist to make a hash mismatch debuggable.

### 4.4 Spec amendments (committed in dispatch unit 5)

The global design spec gets two coordinated edits:

- **§7.8 deferrals table** gains a row: `nmi_sync/demo_ntsc.nes` golden hash → re-enable at M3, reason "no renderer or frame-diff tooling at M2 to debug a hash mismatch."
- **§8 M2 gate** changes "matches golden framebuffer hash" → "produces a self-deterministic framebuffer."

§8 M3's existing gate text ("`nmi_sync/demo_ntsc.nes` framebuffer matches golden hash on Mac and WASM") is unchanged; the M3 implementation plan will specify the capture-from-Mesen 2 procedure when it's authored.

**Additional amendment (committed in dispatch unit 2, Task 23):** the global spec §7.8 deferrals table also gains rows for `ppu_vbl_nmi/05-nmi_timing`, `06-suppression`, `07-nmi_on_timing`, `08-nmi_off_timing`, and `10-even_odd_timing`. These five sub-tests measure NMI dispatch latency and the odd-frame skip at single-cycle precision; passing them requires per-cycle interrupt polling (penultimate-cycle 6502 sampling) — a CPU-wide refactor where every opcode handler participates in the poll. Deferred to M5 alongside the APU frame-counter IRQ work that needs the same infrastructure. The M2 §1.3 acceptance gate's "`ppu_vbl_nmi/01..10` all green" is amended via this §7.8 mechanism to "5 of 10 sub-tests green at M2 (01, 02, 03, 04, 09); 5 deferred per §7.8."

## 5. Dispatch units (the 5-unit decomposition)

Each unit ends in a clean integration-test gate. Per the user's preference for higher involvement on unfamiliar domains, units 1–2 are user-driven (Claude as reviewer/pair); units 4–5 default to agent dispatch; unit 3's mode is a real decision at the unit-2 → unit-3 checkpoint, not a rubber stamp.

Every task in the implementation plan begins with a Background section: what this subsystem does, where it sits in the bigger picture, the relevant nesdev wiki link, and the trickiest gotcha. Between dispatch units we pause for an explicit checkpoint: Claude summarizes the diff and points at test output; the user reads the code; we discuss before moving on.

### Unit 1 — Skeleton: state machine, register file, VRAM/OAM/palette/mirroring (user-driven)

Replaces the 59-line stub. Defines `PpuState`, the per-dot state machine (262 × 341, four phases, frame parity), the register file (PPUCTRL/MASK/STATUS/OAMADDR/OAMDATA/PPUSCROLL/PPUADDR/PPUDATA + the v/t/x/w internal registers from Loopy's docs), 2KB nametable RAM with mirroring, primary OAM (256B), 32B palette RAM with the $3F10/$14/$18/$1C mirror behavior, and CHR access through the existing mapper.

**Gate:** unit tests for dot/scanline counting, register read/write side effects (including PPUDATA buffered read for $0000–$3EFF, palette mirror semantics), nametable mirroring math. No real test ROMs run yet.

**Why user-driven:** the Loopy v/t/x/w internal-register dance is the single most disorienting thing in the PPU; building it by hand pays off forever.

### Unit 2 — Vblank/NMI timing → green ppu_vbl_nmi/01..10 (user-driven)

Sets PPUSTATUS vblank flag at dot 1 of scanline 241; drives `take_nmi()` rising edge from PPUCTRL bit 7. Implements PPUSTATUS-read suppression. Wires PPU NMI through the bus into `Cpu::nmi_pending`. Walks the 10 sub-tests in numeric order: `01-vbl_basics`, `02-vbl_set_time`, `03-vbl_clear_time`, `04-nmi_control`, `05-nmi_timing`, `06-suppression`, `07-nmi_on_timing`, `08-nmi_off_timing`, `09-even_odd_frames`, `10-even_odd_timing`. Each ROM is its own `#[test]` ending in a $6000 PASS.

**Gate:** all 10 sub-tests green; ROMs vendored in this unit.

**Why user-driven:** vblank/NMI timing is the second mental-model unit — the dot-precise suppression behavior is famously subtle, and the test ROMs are *built* to teach what's right.

### Unit 3 — Background fetch pipeline + OAMDMA (decision deferred to checkpoint)

8-cycle repeating fetch (NT byte → AT byte → pat lo → pat hi), 16-bit pattern shifters, 8-bit attribute latches, fine-X scroll selection, per-dot pixel emit during dots 1–256 of visible scanlines. v-register increments (coarse-X every 8 cycles, fine-Y/coarse-Y at dot 256, horizontal copy at dot 257, vertical copy on pre-render dots 280–304). OAMDMA filled in at $4014.

**Gate:** unit tests for fetch timing and shifter content; framebuffer becomes meaningful but no real-ROM gate yet (no M2 test ROM exercises background-only correctness without sprite involvement). `nmi_sync/demo_ntsc.nes` self-determinism check lands here (the determinism check doesn't need sprites).

**Mode:** decided at the unit-2 → unit-3 checkpoint. Default if nothing's said: agent-dispatched. The user has flagged a likely preference to drive this unit too, given that the fetch pipeline is also a mental-model unit even if it's mechanical to write.

### Unit 4 — Sprite eval + sprite-0 hit → green sprite_hit_tests_2005.10.05/* (agent-dispatched by default)

Secondary OAM fill during dots 65–256, sprite fetches during dots 257–320, per-pixel sprite/background priority resolution, sprite 0 hit detection latched into PPUSTATUS bit 6, sprite overflow flag (bit 5) with the documented hardware bug. Walks all 10 `sprite_hit_tests` sub-tests.

**Gate:** all sprite_hit_tests green.

### Unit 5 — OAM stress + A12 + cleanup → green oam_read.nes, oam_stress.nes (agent-dispatched by default)

OAM read/write edge cases, OAMADDR corruption during rendering (the documented hardware bug), A12 line plumbed through `mapper.notify_a12(level)` from inside `Ppu::step` (no-op for NROM but exercised), spec §8 M2 gate amendment, spec §7.8 deferral row for `nmi_sync` golden hash, `LICENSES.md` provenance for the 23 newly vendored ROMs, M2 wrap-up commit, branch merge.

**Gate:** `oam_read.nes` and `oam_stress.nes` green; `cargo fmt`, `cargo clippy -D warnings` clean across the workspace.

### Per-unit checkpoint protocol

After each dispatch unit lands on the feature branch, before starting the next:

1. Claude posts a recap: what changed (file list with one-line summaries), test output (real-ROM passes, unit-test counts), and any subtleties that came up (cycle-count surprises, spec ambiguities resolved one way, deferred sub-issues).
2. User reads the diff, runs the new tests locally if desired, reads any docs the recap pointed at.
3. Discussion until both parties agree the unit is on-pattern.
4. Decide the next unit's dispatch mode (especially for unit 3) and any plan amendments.

Especially important between units 2 and 3 — the user-driven → agent-dispatched boundary deserves a careful look at what patterns to carry forward.

## 6. Risk register

Risks that are likely to bite, with planned mitigations:

1. **Loopy v/t/x/w internal-register dance is the trickiest thing in unit 1.** The increments at dot 256 (fine-Y/coarse-Y), dot 257 (horizontal copy from t into v), pre-render vertical copy at dots 280–304, and how PPUSCROLL/PPUADDR writes interact with the `w` toggle — all easy to get subtly wrong. *Mitigation:* unit 1 has unit tests for each register operation against documented behavior; unit 2's `02-vbl_set_time.nes` will catch many timing slips even though it nominally tests vblank.

2. **PPUSTATUS read suppression in unit 2.** A read of $2002 right at the dot-0 / dot-1 boundary of scanline 241 has documented suppression behavior. `06-suppression.nes`, `07-nmi_on_timing.nes`, and `08-nmi_off_timing.nes` are *built* to test this. *Mitigation:* this is one of the unit 2 sub-tests; failure modes are diagnostic via the $6000 protocol. Plan task gets explicit nesdev wiki references.

3. **Sprite-0 hit edge cases in unit 4.** Hit detection has documented edges: at x=255 it's not detected; the rightmost 8 pixels can be clipped via PPUMASK; hit doesn't fire if sprite or background is fully transparent at that pixel. *Mitigation:* the 10 `sprite_hit_tests` sub-tests cover these explicitly.

4. **OAMADDR corruption during rendering.** Documented hardware bug: OAMADDR writes during rendering corrupt OAM in a specific pattern. *Mitigation:* `oam_stress.nes` exercises this; we implement the bug, not paper over it. Specific dot ranges are pinned down in the implementation plan task with the relevant nesdev wiki citation.

5. **OAMDMA cycle alignment.** 513 cycles on even alignment, 514 on odd. *Mitigation:* unit 3 has a unit test against the cycle counter delta.

6. **User-driven units take longer than dispatched ones, by design.** Unit 1 may take 2–3 work sessions. *Mitigation:* explainer Background sections in the plan let the user pick up after a break without re-reading the spec.

7. **A12 plumbing being inert is hard to test.** No mapper at M2 actually uses `notify_a12`. *Mitigation:* unit 5 includes a small `#[cfg(test)]` test mapper that records A12 transitions, so the test asserts the PPU calls `notify_a12` at the documented dot positions. This is the only "fixture mapper" we add at M2.

### What's deliberately not in the risk register

- **Performance.** No tuning at M2. Spec §6.8 sets a perf contract for the debugger, not the PPU. Native cycles/frame matters first at M6 (SMB1 milestone).
- **Cross-emulator correctness.** The test ROMs are the spec. We don't compare framebuffers to Mesen at M2 — that's M3's job once a renderer exists.
- **WASM divergence.** Nothing PPU-specific. WASM build keeps working because PPU is pure compute.

## 7. Authoritative references

Each plan task names the relevant section. Master list:

- nesdev wiki — *PPU rendering* — https://www.nesdev.org/wiki/PPU_rendering
- nesdev wiki — *PPU scrolling* (Loopy's doc) — https://www.nesdev.org/wiki/PPU_scrolling
- nesdev wiki — *PPU registers* — https://www.nesdev.org/wiki/PPU_registers
- nesdev wiki — *Sprite zero hits* — https://www.nesdev.org/wiki/Sprite_zero_hits
- nesdev wiki — *PPU OAM* — https://www.nesdev.org/wiki/PPU_OAM
- nesdev wiki — *NMI* — https://www.nesdev.org/wiki/NMI
- nesdev wiki — *PPU pattern tables* — https://www.nesdev.org/wiki/PPU_pattern_tables
- nesdev wiki — *PPU palettes* — https://www.nesdev.org/wiki/PPU_palettes
- blargg's `ppu_vbl_nmi.txt` accompanying notes (vendored alongside the ROMs)
- Project: [`docs/superpowers/specs/2026-05-02-nes-emulator-design.md`](2026-05-02-nes-emulator-design.md) §3.5

## 8. Out-of-scope reminders

In line with the global spec's "don't preempt later milestones" principle, M2 explicitly does not:

- Render the framebuffer to screen — that is M3.
- Implement controller strobe or $4016/$4017 read shape — M4.
- Generate APU samples — M5.
- Add save state golden tests for new PPU fields — M7 (the snapshot derive is in place but not exercised against a wire-format reference).
- Implement MMC3 A12 IRQ filter — M11+. The A12 *call* is in place; the *consumer* arrives later.
- Build debugger UI panels for PPU inspection — M9.

The trait shapes are designed to absorb later milestones (e.g., the take-on-read NMI semantics work unchanged for time-travel snapshots in M8; A12's call site is already where MMC3 will hook in). Resist preemptive abstraction.
