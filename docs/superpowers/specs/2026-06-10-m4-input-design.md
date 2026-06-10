# M4 — Input Design

**Status:** Approved for implementation planning
**Date:** 2026-06-10
**Author:** Eric Perdew (with Claude Code as co-designer)
**Predecessor:** [`2026-05-02-nes-emulator-design.md`](2026-05-02-nes-emulator-design.md) §4.1 (determinism contract), §5.5 (input), §8 (M4 milestone). Read the global spec first; this document refines M4 only.

## 1. Goal and gate

Two emulated NES controllers driven from the keyboard, with input flowing
through a deterministic event journal. SMB1 becomes playable (silent).

**Gate (from global spec §8, made concrete here):**

- A predefined input sequence applied to a controller-polling test ROM
  produces a state hash equal to a **pinned golden constant**, asserted in a
  native test **and** a `wasm-bindgen-test` (extending the M3 cross-platform
  gate pattern).
- Manual: SMB1 (user-supplied ROM via the existing CLI path argument) boots,
  the title screen responds to Start, and level 1-1 plays correctly with
  keyboard controls (no audio — that's M5).

## 2. Scope

### 2.1 In scope

- Real `Controller` shift-register emulation in `nies-core` (replacing the M1
  stub): strobe latch, 8-bit serial reads, post-8 reads return 1.
- `$4016`/`$4017` read paths on `Bus` wired to the controllers, with
  open-bus-style upper bits; non-destructive `peek`.
- A `Buttons` newtype (hardware bit order) shared by core and frontends.
- `InputEvent` + an in-core input journal on `Nes`, with
  `Nes::set_buttons(port, buttons)` as the single apply primitive
  (architecture: apply-immediately + journal, see §4).
- Default keyboard mapping per global spec §5.5 (arrows, X=A, Z=B,
  Enter=Start, RShift=Select) driving **port 1** on both frontends, via a
  shared mapping helper in `nies-ui`.
- Input determinism gate: custom in-test micro-ROM, scripted input sequence,
  golden hash, native + wasm.

### 2.2 Out of scope (deferred, with milestone)

| Deferred | Milestone |
|---|---|
| Gamepad support (gilrs native / Gamepad API web) | M10 |
| Rebinding UI (wizard + per-button) | M10 |
| All hotkeys (save state, pause, reset, rewind, fast-forward, fullscreen, debugger toggle) | with their features (M7–M10) |
| Port-2 keyboard mapping | M10 (gamepads cover port 2; §5.5 defines no keyboard default for it) |
| `$4016` expansion-port bits (bits 1–4), Famicom mic | not planned for v1 |
| DMC DMA / `$4016` double-read conflict (`read_joy3` etc.) | M5 (needs the APU DMC) |
| Opposing-d-pad masking (Up+Down / Left+Right) | M10 settings concern; the core stays faithful and never masks |
| Replay machinery that *consumes* the journal | M8 (M4 only records) |

Port 2 is fully emulated and readable at M4 — it just always reports
all-released because nothing maps to it yet.

## 3. Core: `Controller` (`nies-core/src/input.rs`)

Replace the M1 stub with the 4021 shift-register model:

```rust
/// Button state in hardware bit order (bit 0 = A): A, B, Select, Start,
/// Up, Down, Left, Right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Buttons(pub u8);

impl Buttons {
    pub const A: Buttons;      // 0x01
    pub const B: Buttons;      // 0x02
    pub const SELECT: Buttons; // 0x04
    pub const START: Buttons;  // 0x08
    pub const UP: Buttons;     // 0x10
    pub const DOWN: Buttons;   // 0x20
    pub const LEFT: Buttons;   // 0x40
    pub const RIGHT: Buttons;  // 0x80
    // plus minimal set/clear/contains helpers and BitOr — hand-rolled,
    // no `bitflags` dependency.
}

pub struct Controller {
    buttons: Buttons, // live state, updated by Nes::set_buttons
    strobe: bool,     // $4016 bit 0
    shift: u8,        // serial shift register
}
```

Behavior (Nesdev "standard controller"):

- `write_strobe(val)`: `strobe = val & 1 != 0`. While strobe is high the
  shift register continuously tracks `buttons`; this is modeled by reloading
  `shift = buttons.0` on every read while strobed (and on the strobe-high
  write itself, so the falling edge latches the then-current state).
- `read()`: if strobed, reload first. Return `shift & 1`, then
  `shift = (shift >> 1) | 0x80` — after 8 reads, further reads return 1,
  matching official controllers.
- `peek(&self)`: the bit a `read` would return, with **no** mutation —
  `buttons.0 & 1` while strobed (a read would reload first), else
  `shift & 1`. Serves the debugger's non-destructive `Bus::peek` path.

`Bus` changes:

- `$4016` read → `0x40 | controllers[0].read()`; `$4017` read →
  `0x40 | controllers[1].read()`. The `0x40` models the typical open-bus
  upper bits seen on reads from this range (the bus address `$40xx` was the
  last value on the bus). Bit 0 is the only bit games may rely on at M4.
- `$4016` write already strobes both controllers (M1 plumbing) — unchanged.
- `Bus::peek` at `$4016`/`$4017` uses `Controller::peek`, which takes
  `&self` — the existing `unsafe` `&self`→`&mut self` cast in `Bus::peek`
  is not needed for these addresses.

The core never masks simultaneous opposing directions — real hardware
allows Up+Down, and faithful recording is what the journal stores. Any
masking policy is a frontend/M10 settings concern.

## 4. Core: `InputEvent` journal on `Nes`

Architecture decision: **apply-immediately + journal** (option A of three
considered). The alternative — deriving controller state from the log via a
cursor during normal execution — produces identical observable behavior for
any real host (events always arrive stamped "now") but adds cursor state
that the M7 snapshot would have to capture. The journal here is a passive
record; nothing reads it during normal execution.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputEvent {
    pub cycle: u64,      // Bus::cycle (master CPU cycle counter) at apply time
    pub port: u8,        // 0 or 1
    pub buttons: Buttons, // full state, not a delta
}

impl Nes {
    /// Apply a controller state change now: stamp it with the current
    /// master cycle, set the live controller state, append to the journal.
    /// `port` must be 0 or 1 (debug-asserted; out-of-range is a programmer
    /// error, not user input).
    pub fn set_buttons(&mut self, port: u8, buttons: Buttons);

    /// The journal, for tests now and replay (M8) later.
    pub fn input_log(&self) -> &[InputEvent];
}
```

Design notes:

- Events carry the **full button state**, not deltas — replay can apply any
  prefix of the log and be correct, and a dropped event corrupts one frame,
  not everything after it.
- `cycle` comes from `bus.cycle`, the master counter the tick discipline
  already maintains. In practice frontends call `set_buttons` between
  `run_frame()` calls, so stamps land on frame boundaries; the format
  supports mid-frame stamps for M8 time-travel without change.
- M8's replay walks this log calling the same apply primitive (global spec
  §4.4's `apply_input` sketch); M7's snapshot machinery serializes
  `Controller` (3 small fields) and the journal handling is defined there.
- The journal grows unboundedly at M4 (one event per host key transition —
  trivial memory). Truncation policy is tied to the snapshot ring and is
  defined at M7/M8 (global spec §4.4).

## 5. Frontends: keyboard → port 1

A shared helper in `nies-ui` (new `input.rs` module), used identically by
both binaries since both run winit event loops:

```rust
/// §5.5 default mapping: arrows = d-pad, X = A, Z = B,
/// Enter = Start, RShift = Select. Returns None for unmapped keys.
pub fn map_key(code: winit::keyboard::KeyCode) -> Option<Buttons>;

/// Folds key down/up transitions into the current port-1 button state.
#[derive(Default)]
pub struct KeyboardState { buttons: Buttons }

impl KeyboardState {
    /// Update from a winit key event. Returns Some(new_state) when the
    /// mapped state changed (caller forwards to nes.set_buttons), None
    /// for unmapped keys, key-repeat events, and no-op transitions.
    pub fn on_key(&mut self, event: &winit::event::KeyEvent) -> Option<Buttons>;
}
```

- Both `nies-app` and `nies-web` handle `WindowEvent::KeyboardInput`,
  call `keyboard.on_key(&event)`, and forward changes via
  `nes.set_buttons(0, state)`.
- Key-repeat events (`event.repeat`) are ignored — the latch already holds
  the pressed state.
- Returning `Some` only on *change* keeps the journal minimal (no event spam
  from repeats or unmapped keys).
- Port 2 is never written by the frontends at M4.

**Web caveat to verify manually:** arrow keys must not scroll the page and
the canvas must receive key events (focus). Winit's web backend calls
`preventDefault` on handled keys, but this gets an explicit manual check in
the plan; if focus is an issue the fix is canvas `tabindex` wiring in
`index.html`, not Rust code.

## 6. Testing and CI

TDD discipline applies (per project conventions): tests first for every
behavioral unit.

### 6.1 Controller unit tests (`input.rs`)

- Serial read order A, B, Select, Start, Up, Down, Left, Right (LSB first).
- Reads 9+ return 1.
- Strobe high: reads continuously reflect live `buttons` bit A without
  advancing the sequence.
- Strobe falling edge latches the then-current state; later `buttons`
  changes don't affect an in-progress read sequence.
- Re-strobe mid-sequence restarts from bit A.
- `peek` returns the current bit without shifting.

### 6.2 Bus tests

- `$4016` write strobes **both** ports.
- `$4016`/`$4017` reads return `0x40 | bit` and shift the correct port.
- `Bus::peek` at `$4016`/`$4017` is non-destructive (back-to-back peeks
  agree; a subsequent read returns the same bit).

### 6.3 Determinism gate (`tests/input_determinism.rs`)

- **Micro-ROM assembled in-test** (NROM, built as a byte vector like
  earlier plan-built ROMs): NMI handler strobes `$4016`, reads 8 bits from
  each port, packs them into two bytes, and appends to a RAM ring buffer;
  main loop idles. No vendored binary, no licensing, and it exercises
  exactly the strobe/latch/shift semantics under real CPU timing.
- The test scripts a multi-frame sequence of `set_buttons` calls
  interleaved with `run_frame()` (covering: single buttons, combinations,
  port 2 silence, a press held across frames, press+release within one
  poll interval).
- Run the identical script on **two fresh `Nes` instances**; hash CPU RAM
  and the framebuffer with `Hasher::write` over raw bytes (**never**
  `.hash()` — the documented usize-length-prefix gotcha); assert the two
  hashes match **and** equal a pinned golden constant.
- A `wasm-bindgen-test` runs the same script and asserts the same constant,
  extending the M3 cross-platform gate. The micro-ROM builder and script
  live in shared test-support code so native and wasm use identical bytes.

### 6.4 What is not tested automatically

- Keyboard event handling end-to-end (winit events can't be synthesized
  headlessly with confidence): `map_key`/`KeyboardState` get pure unit
  tests; the full path is covered by the manual SMB1 gate.
- Web canvas focus / scroll-prevention behavior (manual check).

### 6.5 CI

No workflow changes expected: the native tests are ordinary `cargo test`
additions, and the wasm test rides the existing `wasm-pack test` job.

## 7. Execution and review style

Same as M3: **agent-dispatched units, user reviews diffs at per-unit
checkpoints**, with a short pre-task background note per unit.

Anticipated unit breakdown (the implementation plan finalizes this):

1. **`Buttons` + `Controller`** — replace the stub, TDD against §6.1; wire
   `Bus` reads + peek (§6.2).
2. **`InputEvent` journal** — `set_buttons` / `input_log` on `Nes`, unit
   tests for stamping and full-state semantics.
3. **Determinism gate** — micro-ROM builder, scripted sequence, golden
   hash, native + wasm tests.
4. **Frontends** — `nies-ui` `map_key`/`KeyboardState` (unit-tested), wire
   both binaries, manual SMB1 + web focus checks.

## 8. Risks and mitigations

- **Micro-ROM authoring bugs** — hand-assembled 6502 in a test is easy to
  get subtly wrong. Mitigation: build it with the same helper style as
  earlier plan-built ROMs, keep the program tiny (strobe, 16 reads, store,
  loop), and validate the ring-buffer contents directly in a non-hash
  assertion before pinning the golden hash.
- **Open-bus upper bits** — `0x40 |` is the common-case approximation, not
  a full open-bus model. SMB1 and the M4 gate only depend on bit 0. If a
  later test ROM demands true open-bus, that's a deliberate revisit (noted
  here, not silently wrong).
- **Web key handling quirks** — confined to the manual check; the fix
  space (tabindex / preventDefault) is HTML-side and small.
- **Golden constant churn at M5** — the M3 `demo_ntsc` hash is already
  documented as needing re-pinning at M5 (per-cycle interrupt polling).
  The M4 micro-ROM polls in an NMI handler, so its hash is also
  timing-sensitive and may re-pin at M5. Acceptable: re-pinning is a
  one-constant change with the determinism property still asserted by the
  run-twice comparison.

## 9. Open questions resolved by this milestone

- None of the global spec §9 open questions are touched by M4. The §8 M4
  milestone entry gains a completion note when the milestone lands (final
  unit), recording the keyboard-only scope decision (gamepad → M10).
