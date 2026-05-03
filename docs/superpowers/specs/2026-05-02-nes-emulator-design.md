# nies — NES Emulator Design

**Status:** Draft for implementation planning
**Date:** 2026-05-02
**Author:** Eric Perdew (with Claude Code as co-designer)

## 1. Overview

`nies` is a Nintendo Entertainment System (NES) emulator written in Rust, targeting native macOS / Linux / Windows desktops and the web (WebAssembly). The first concrete milestone is to play *Super Mario Bros.* (SMB1) end-to-end with sound, save states, rebindable controls, and a built-in debugger. The architecture is deliberately chosen to support broad NES compatibility long-term without an architectural rewrite.

### 1.1 Goals (v1)

- Play SMB1 correctly with sound on macOS native and in the browser via WASM.
- Native app shell (winit + wgpu, egui UI) with rebindable keyboard and gamepad controls.
- Audio output with cross-platform crate stack (cpal).
- Save states: 10 numbered slots + quicksave, with hotkeys, persisted per platform.
- Per-instruction time-travel debugging via deterministic snapshot + replay.
- A built-in debugger covering CPU, PPU, APU, and mapper inspection, with PC breakpoints, watchpoints, step controls, and a trace ring buffer.
- Determinism guaranteed by core design: same ROM + same input sequence ⇒ same state at every cycle, on every supported platform.

### 1.2 Non-goals (v1)

- Netplay / online multiplayer.
- Conditional breakpoints (deferred until needed; see §6.4).
- NTSC composite filter, CRT shader, scanlines.
- Mappers beyond NROM. (MMC1, UxROM, CNROM, MMC3 land in post-v1 milestones; the `Mapper` trait is designed against MMC3's needs from day one to avoid retrofitting.)
- Tier-3 (sub-instruction master-clock-accurate) emulation. Tier 2 is the chosen target (§3.1).
- Reading or supporting commercial ROM dumps. The user supplies their own copy of any commercial ROM at runtime.

### 1.3 Audience

A solo developer (the author) building this as a learning and craft project, with the intent that the architecture supports significant future expansion (more games, more mappers, possibly netplay, possibly tier-3 accuracy).

## 2. Accuracy tier

Three meaningful accuracy levels exist:

1. **Instruction-accurate CPU + scanline-accurate PPU.** Plays maybe 70-80% of popular NES library; breaks on sprite-0-hit-sensitive games (Punch-Out!!, Battletoads), mid-scanline PPU writes (Rad Racer, Marble Madness), and fine MMC3 IRQ timing.
2. **Cycle-stepped "bus-tick" architecture.** CPU executes per-instruction, but every memory read/write inside the instruction ticks the PPU 3 dots and APU 1 cycle. PPU runs per-dot. Mappers see PPU A12 transitions per cycle. Plays 95%+ of the commercial library correctly. This is what Mesen-S, ares' NES core, and Nintendulator-style emulators run.
3. **Sub-instruction / master-clock cycle-accurate.** Every component clocked at the master clock. Buys you the last few percent (open-bus quirks, exact sprite-overflow timing, demoscene ROMs). Mesen, Nintendulator, ares.

**`nies` targets tier 2.** It is the right balance of correctness, compatibility, and implementation cost for a project whose stated ambition is "broad compatibility on par with serious emulators." Going from tier 2 to tier 3 buys ~last 5% for substantially more implementation work and a much harder testing burden; the project can choose to climb that ladder later if desired.

Building tier 2 from day one is barely more work than tier 1 for the SMB1 milestone, and saves a multi-day rewrite the first time a game needs better accuracy.

## 3. Architecture

### 3.1 Workspace and crates

A Cargo workspace at the repo root with four crates:

```
nies/
├── Cargo.toml          (workspace root)
├── crates/
│   ├── nies-core/      (library — emulator + debugger backend, no I/O)
│   ├── nies-ui/        (library — egui panels shared by both binaries)
│   ├── nies-app/       (binary — native frontend)
│   └── nies-web/       (binary — WASM frontend)
├── docs/superpowers/specs/
└── Trunk.toml
```

**`nies-core`** is the bulk of the project. It compiles to a pure library with no I/O dependencies — no `std::fs`, no `std::time::SystemTime`, no audio device access, no threading. It exposes `Nes`, `Cartridge`, `Mapper`, `Snapshot`, `InputEvent`, `Debugger`, `TimeTravel`, plus `frame()` / `step_instruction()` / `step_cycle()` driver methods. It runs deterministically given (ROM bytes, initial state, input event sequence).

**`nies-ui`** holds platform-agnostic egui code: game viewport, debugger panels, settings dialogs. Depends on `egui`, `egui-wgpu`, and `nies-core`. Knows nothing about file systems, audio devices, or platform clocks.

**`nies-app`** is the native binary. Owns: winit window, wgpu rendering, cpal audio, file dialogs (rfd), save-state file I/O, config persistence (TOML in platform save dir), hotkey/rebinding logic. Implements `Platform` (§3.6).

**`nies-web`** is the WASM binary. Owns: HTML file picker (gloo-file), IndexedDB for save states (gloo-storage), cpal-via-WebAudio with the `audioworklet` backend preferred (falling back to default), `web_time::Instant` for frame pacing. Built via Trunk with a pinned `wasm-opt` pass (§3.7). Implements `Platform`.

### 3.2 Module layout inside `nies-core`

```
src/
├── lib.rs
├── nes.rs              (Nes struct, top-level frame/step driver)
├── bus.rs              (CPU bus: ticks PPU/APU on every access)
├── cpu/
│   ├── mod.rs
│   ├── instructions.rs (opcode table + execution)
│   └── addressing.rs   (addressing modes)
├── ppu/
│   ├── mod.rs          (per-dot state machine)
│   ├── render.rs       (background fetch pipeline)
│   ├── sprites.rs      (sprite evaluation, sprite 0 hit)
│   └── registers.rs    (PPUCTRL, PPUMASK, etc.)
├── apu/
│   ├── mod.rs
│   ├── pulse.rs
│   ├── triangle.rs
│   ├── noise.rs
│   ├── dmc.rs
│   └── mixer.rs
├── mapper/
│   ├── mod.rs          (Mapper trait + MapperKind enum dispatch)
│   ├── nrom.rs         (mapper 0; v1)
│   ├── mmc1.rs         (mapper 1; post-v1)
│   ├── uxrom.rs        (mapper 2; post-v1)
│   ├── cnrom.rs        (mapper 3; post-v1)
│   └── mmc3.rs         (mapper 4; post-v1)
├── cartridge.rs        (iNES/NES2.0 parser)
├── input.rs            (controller state, input events)
├── snapshot.rs         (postcard-based full-state serialization)
├── timetravel.rs       (snapshot ring + replay)
└── debugger.rs         (breakpoints, watchpoints, trace, step control)
```

Only NROM is required for the SMB1 milestone. The trait is designed against MMC3 needs (PPU A12 hook, scanline IRQ counter) so the other mappers slot in without trait churn.

### 3.3 The `Nes` struct and bus tick discipline

```rust
struct Nes {
    cpu: Cpu,
    bus: Bus,
}

struct Bus {
    ram: [u8; 2048],
    ppu: Ppu,
    apu: Apu,
    mapper: MapperKind,         // enum dispatch (see §5.1)
    controllers: [Controller; 2],
    cycle: u64,                 // master CPU cycle counter
}
```

The architectural rule:

> The only way to read or write memory inside the core is `Bus::read(addr)` / `Bus::write(addr, val)`. Both methods unconditionally tick the rest of the system one CPU cycle (= 3 PPU dots + 1 APU step + DMC service if pending).

```rust
impl Bus {
    pub fn read(&mut self, addr: u16) -> u8 { /* … self.tick(); … */ }
    pub fn write(&mut self, addr: u16, val: u8) { /* … self.tick(); … */ }
    pub fn peek(&self, addr: u16) -> u8 { /* read logic, no tick, no side effects */ }

    fn tick(&mut self) {
        for _ in 0..3 { self.ppu.step(&mut self.mapper); }
        self.apu.step(&mut self.mapper);
        self.cycle += 1;
        if let Some(addr) = self.apu.dmc.take_pending_fetch() {
            let val = self.read_cpu_memory_no_tick(addr);
            self.apu.dmc.deliver_sample(val);
            self.stall(self.apu.dmc.stall_cycles());
        }
    }
}
```

`Cpu::step(&mut self, bus: &mut Bus)` executes one instruction. CPU has no permanent reference to the bus. There is no other path from CPU to memory. "I forgot to tick" is structurally impossible because tick happens inside read/write. Split borrows on `Bus`'s named fields let `tick()` simultaneously borrow PPU and mapper without recursion.

`peek` is a *non-ticking* read path used by the debugger for memory inspection. Reads of register-mapped addresses ($2000-$3FFF, $4000-$401F) return open-bus-style 0 instead of triggering real hardware reads.

### 3.4 CPU

- 256-entry opcode dispatch table → instruction handler.
- All official opcodes plus illegal opcodes per the policy below. Each handler runs to completion via `bus.read` / `bus.write`. Cycle correctness comes from the bus ticking on each access.
- IRQ/NMI lines sampled at the appropriate cycle of the current instruction.
- Decimal mode (D flag) is settable but BCD arithmetic is not implemented (NES 6502 omits it).
- Dummy reads (page-cross timing on absolute,X reads; read-modify-write dummy reads) are explicit in the opcode handlers and verified by `cpu_dummy_reads.nes`.

**Illegal opcode policy.** Five practical groups:

1. **Stable illegals** (LAX, SAX, DCP, ISC, SLO, RLA, SRE, RRA, ANC, ALR, AXS, etc., ~20 opcodes): implemented to the canonical Nintendulator/blargg spec. These are deterministic across documented 6502s and are the ones commercial games actually use.
2. **"Magic constant" instructions** (XAA/ANE, LXA): real-hardware behavior varies with chip, temperature, bus state. Implemented to the most-cited reference behavior; comment documents the variance.
3. **`SHX`/`SHY`/`SHA`/`TAS` family**: notoriously buggy page-cross behavior. Implemented to the most-cited reference behavior.
4. **`JAM`/`KIL`/`HLT`**: hang the CPU until reset.
5. No commercial game depends on (2) or (3).

### 3.5 PPU

Per-dot state machine. NTSC frame: 262 scanlines × 341 dots; visible 240 × 256.

- Background fetch: 8-cycle repeating fetch pipeline (nametable / attribute / pattern lo / pattern hi) shifted into 16-bit registers, output 1 pixel per dot.
- Sprite evaluation: secondary OAM filled during dots 65–256 of visible scanline; sprite fetches during dots 257–320.
- Sprite 0 hit: detected per-pixel during rendering, latched to PPUSTATUS.
- A12 line: per-cycle level signal, observed by the mapper via `mapper.notify_a12(level)` from inside `Ppu::step`. MMC3's IRQ counter clocks on rising A12 with the standard filter.
- Vblank/NMI: vblank set on dot 1 of scanline 241; NMI fires if PPUCTRL bit 7 set. PPUSTATUS read at dot 0 of scanline 241 has documented suppression behavior.
- Output: `[u8; 256 * 240]` of palette indices (0..63). Frontend converts to RGB via configurable palette.

### 3.6 APU and DMC bus driving

Stepped one CPU cycle at a time. 2× pulse, 1 triangle, 1 noise, 1 DMC. Frame counter in 4-step or 5-step mode controlled by $4017. Mixer per the Nesdev formula. Sample produced per CPU cycle (~1.789 MHz), resampled in the frontend to host rate.

The DMC channel reads samples from CPU memory ($C000-$FFFF) and stalls the CPU 1-4 cycles. Implementation: DMC sets a pending-fetch flag; `Bus::tick` services the flag after stepping APU, performing `read_cpu_memory_no_tick` and adding stall cycles via additional ticking. No recursion required.

OAMDMA ($4014 write) is CPU-initiated and handled inside `Bus::write`'s $4014 case (256 reads from CPU memory, 256 writes to $2004; ticks accordingly).

### 3.7 Mapper trait

```rust
trait MapperImpl: snapshot::SnapshotComponent {
    fn cpu_read(&mut self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, val: u8);
    fn ppu_read(&mut self, addr: u16) -> u8;
    fn ppu_write(&mut self, addr: u16, val: u8);
    fn notify_a12(&mut self, level: bool) {}      // default: no-op
    fn irq_pending(&self) -> bool { false }
    fn mirroring(&self) -> Mirroring;
    fn debug_dump(&self) -> Vec<(&'static str, u32)> { vec![] }
}
```

Dispatched through an enum `MapperKind` (variants: `Nrom(NromState)`, `Mmc1(Mmc1State)`, …) rather than `Box<dyn>`. Reasons: (a) serializes naturally via serde derive, avoiding `typetag` (which is awkward on WASM); (b) faster than vtable dispatch (irrelevant in practice but free); (c) makes the closed set of mappers explicit at the type level.

`notify_a12` is called every PPU dot from inside the PPU step. NROM ignores it. `irq_pending` is polled by the CPU at IRQ sample points.

### 3.8 The `Platform` trait

The thin abstraction shared by both binaries:

```rust
trait Platform {
    fn pick_rom(&mut self) -> Option<RomBytes>;
    fn save_state_write(&mut self, key: &SaveKey, bytes: &[u8]) -> Result<()>;
    fn save_state_read(&mut self, key: &SaveKey) -> Result<Vec<u8>>;
    fn load_config(&self) -> Config;
    fn save_config(&mut self, config: &Config);
    fn now(&self) -> Instant;       // wall-clock; frontend-only, never reaches the core
}
```

`nies-app` and `nies-web` each implement this against their platform's primitives. Everything else in `nies-ui` is platform-agnostic.

## 4. Determinism, save states, time travel

### 4.1 Determinism contract

The core is deterministic in the strict sense: given (initial state, ROM bytes, sequence of timestamped input events), running the emulator produces bit-identical state at every cycle, on every supported platform.

Discipline required to maintain this:

- No clock reads inside the core. The only "time" is `cycle: u64`.
- No OS RNG inside the core. If pseudo-randomness is ever needed, a seeded PRNG is used and the seed is part of state.
- No threading inside the core.
- No `HashMap` iteration order dependence in any logic that affects state.
- Floating point only in the APU mixer, where standard IEEE 754 ops produce identical results across platforms.

Frontend timing (audio device pacing, vsync) is not deterministic, but the frontend is downstream of the core: it consumes outputs and only feeds back via timestamped input events.

### 4.2 Power-on state

The emulator always starts from a canonical power-on state. Policy: **the Mesen pattern** ($00 except $0008/$0009/$000A/$000F set to $F7/$EF/$DF/$BF). This matches what many real chips produce and is broadly compatible. CPU registers reset state per spec (`P=0x34, S=0xFD, PC=read_word(reset_vector)`); PPU and APU reset state per spec.

### 4.3 Save state format

- Serializer: `postcard` with `serde` derives on every state type. (Replaces bincode, which is deprecated.)
- Mapper state serializes via `MapperKind` enum (§3.7).
- File header:
  ```
  magic:        b"NIES"   (4 bytes)
  version:      u16       (format version)
  rom_hash:     [u8; 32]  (SHA-256 of cartridge ROM data)
  cycle:        u64       (master CPU cycle at snapshot)
  payload_len:  u32
  payload:      [u8; payload_len]   (postcard-encoded Nes state)
  ```
- Loading rejects mismatched magic, unknown version, or wrong ROM hash with a clean error.
- No compression. A snapshot is ~10-15 KB uncompressed for NROM, ~30 KB for the largest mappers.
- Not migration-tolerant. Format version bumps invalidate older saves; documented up front.

### 4.4 Save state slots (frontend)

- 10 numbered slots (0-9), per-game (keyed by ROM hash).
- 1 quicksave slot, distinct from numbered slots; last-touched.
- Hotkeys: F1-F10 (save), Shift+F1-F10 (load), Ctrl+S (quicksave), Ctrl+L (quickload). Rebindable.
- Native: stored as files under a per-game directory in the OS save-data path (via `directories` crate).
- WASM: stored in IndexedDB keyed by ROM hash + slot (via `gloo-storage`).

### 4.5 Time travel

Two related features sharing one mechanism:

1. **Game rewind** — hold a hotkey, the game scrubs backward at frame granularity.
2. **Debugger backward step** — in the debugger, "step back one instruction" jumps to the state one instruction prior.

#### Snapshot ring with adaptive thinning

- Last 10 seconds: every frame (600 snapshots).
- 10s–1min: every 10th frame (300 snapshots).
- 1min–10min: every second (540 snapshots).
- Older than 10 minutes: dropped.

Total bound: ~1500 snapshots × ~15 KB ≈ ~22 MB in steady state. The input event log is truncated whenever the oldest snapshot is dropped.

#### Replay

```rust
fn replay_to(&mut self, target_cycle: u64) {
    let snap = self.snapshot_ring.nearest_at_or_before(target_cycle);
    self.nes.apply_snapshot(snap);
    let mut next_event = self.input_log.iter().find(|e| e.cycle >= snap.cycle);
    while self.nes.cycle < target_cycle {
        while let Some(e) = next_event && e.cycle == self.nes.cycle {
            self.nes.apply_input(e);
            next_event = next_event_after(e);
        }
        self.nes.step_cycle();
    }
}
```

Headless replay (no rendering, no audio output) runs ~50-100× realtime on a modern machine, so step-back from any point in the rewind window completes in ≤20 ms.

#### Debugger backward step

1. Determine cycle of start of current instruction (recorded in trace).
2. Compute target cycle = (start of current instruction) − (cycle length of previous instruction).
3. `replay_to(target_cycle)`.

#### Game rewind

User holds rewind hotkey. Frontend repeatedly: pick `current_cycle - 1 frame`, `replay_to(target)`, render that frame. Audio is muted during rewind.

### 4.6 Determinism testing

Built into CI:

- **Replay determinism**: predefined input sequence over N seconds, end-state hashed. Replay from initial state with same input log; verify same hash.
- **Snapshot–restore–replay**: snapshot, run M frames, hash. Restart, snapshot, restore, run M frames, hash. Match required.
- **Cross-platform determinism**: same replay test on Mac, Linux, Windows native, and WASM (via `wasm-bindgen-test` in headless Chrome). All four must produce identical state hashes.

## 5. Frontend

### 5.1 Crate stack

| Concern | Crate | Native | WASM |
|---|---|---|---|
| Window + event loop | `winit 0.30.x` | ✓ | ✓ (canvas) |
| GPU | `wgpu 29.x` | Metal/Vulkan/DX12 | WebGPU/WebGL2 |
| UI | `egui` + `egui-wgpu` | ✓ | ✓ |
| Audio | `cpal` | CoreAudio/ALSA/WASAPI | `audioworklet` preferred, `wasm-bindgen` (WebAudio) fallback |
| Gamepad | `gilrs` | ✓ | Gamepad API |
| File dialog | `rfd` | ✓ | (use `<input type=file>`) |
| Save data | filesystem + `directories` | — | — |
| Save data | — | — | `gloo-storage` (IndexedDB) |
| File input | — | — | `gloo-file` |
| Time | `std::time` | ✓ | `web_time` |
| Save state format | `postcard` (replaces deprecated bincode) | ✓ | ✓ |
| Native config | `toml` | ✓ | — |
| WASM bundling | `trunk` + pinned `wasm-opt` | — | ✓ |

### 5.2 Frame pacing: audio-driven

NES NTSC runs at 60.0988 Hz, not 60. On a 60 Hz monitor with vsync-driven pacing, vsync drift is audible (~17-second skip cadence) or causes audio pitch shifts. Audio glitches are far more perceptible than visual ones.

`nies` is **audio-driven**. The audio device requests N samples; the frontend asks the core for that many APU samples worth of emulation. The core runs CPU/PPU/APU until N samples are produced. Frames produced as a side effect (PPU completes 89,342 dots per frame) are queued for next vsync.

### 5.3 Audio specifics

- APU produces one raw sample per CPU cycle (~1.789 MHz). Resampled to host rate (typically 48 kHz) using a Blackman-windowed sinc FIR (~64 taps) plus linear interpolation.
- Buffer target: ~50 ms latency. Configurable.
- Underrun: log, skip ahead. Don't try to catch up.
- Per-channel mute toggles in the UI.
- Master volume slider.

### 5.4 Video pipeline

- Core emits `[u8; 256 * 240]` of palette indices per frame.
- Frontend uploads as R8 texture via `wgpu::Queue::write_texture` (one persistent texture, written each frame). At 256×240×R8 = 61 KiB at 60 fps = 3.7 MiB/s upload bandwidth — trivial on any modern GPU.
- Fragment shader samples the texture, looks up palette RGB from a 64-entry uniform buffer, outputs RGB.
- Final blit: integer-scale to viewport, optional pixel aspect ratio correction (8:7).
- Default palette: FBX "Smooth" (subject to selection during M3 implementation).
- Defer to post-v1: NTSC composite filter, CRT shader, scanlines, overscan crop.

### 5.5 Input

Two emulated controllers, each mapped to keyboard or gamepad.

**Default keyboard mapping (controller 1):**
- D-Pad: Arrow keys
- A: X · B: Z
- Start: Enter · Select: RShift

**Default gamepad mapping:** first detected gamepad → controller 1, second → controller 2. Buttons map to obvious face/dpad correspondences.

**Rebinding UX:**
- **Primary action** is a *wizard*: prompts for each NES button in sequence ("Press Up… Down… Left… Right… Select… Start… B… A"). One button click in settings to start. This is the default flow.
- **Per-button rebind** is secondary: click any individual binding slot to rebind just that one.

**Hotkey bindings (rebindable):**
- Save state F1-F10 / Shift+F1-F10 to load · Ctrl+S quicksave · Ctrl+L quickload
- Pause: Space
- Reset: Ctrl+R (soft) · Ctrl+Shift+R (hard / power-cycle)
- Rewind: hold Backspace
- Fast-forward: hold Tab
- Toggle debugger: F12
- Fullscreen: F11

**Input event handling:** host events are timestamped to the next CPU cycle when observed and inserted into the input event log. The core polls controller state from this log on $4016/$4017 reads. Keeps input deterministic and replay-correct.

### 5.6 UI shell

Single window, three logical regions:

- **Top: menu bar.** File (Open ROM, Recent, Save State, Load State, Quit), Emulation (Pause, Reset, Speed, Mute), View (Window Scale, Aspect Correction, Fullscreen, Show Debugger), Help (About).
- **Center: game viewport.**
- **Right (or floating): debugger panels.** Only visible when toggled on (F12). Dockable via egui.

Settings panel (modal): input rebinding, audio config, palette selection, save-data location.

### 5.7 Config persistence

```rust
struct Config {
    input_bindings: InputBindings,    // per-port + hotkeys
    window: WindowConfig,             // size, aspect mode, palette
    audio: AudioConfig,                // sample rate, latency, master volume, channel mutes
    recent_roms: Vec<RecentRom>,
}
```

Native: `~/Library/Application Support/nies/config.toml` (or platform equivalent). WASM: localStorage as JSON.

### 5.8 WASM build pipeline

- Build via `trunk build --release`.
- **`wasm-opt -O3` pass on every release build.** Pinned binaryen version in `Trunk.toml` to avoid drift. Addresses the externref interaction (externref processor must run before wasm-opt) and gives ~30-50% size reduction. Also sidesteps known Rust-1.87+ wasm-opt compatibility issues by pinning a known-good toolchain.
- ROM loading: `<input type=file>` triggered from egui; bytes fed to core.
- Save state I/O: IndexedDB via `gloo-storage`.
- Time: `web_time::Instant` for frame pacing in the frontend; the core never reads time.
- Threading: none. Main JS thread + WebAudio worklet.
- CI uploads built `dist/` as a deployable artifact.

## 6. Debugger

### 6.1 Backend: `Debugger` struct

```rust
struct Debugger {
    nes: Nes,
    state: DebugState,
    breakpoints: Vec<Breakpoint>,
    watchpoints: Vec<Watchpoint>,
    trace: VecDeque<TraceEntry>,    // ring buffer, default cap 4096
    trace_capacity: usize,
    trace_log: Option<TraceLogSink>, // optional: write trace to disk
    timetravel: TimeTravel,
}

enum DebugState { Running, Paused, Halted(HaltReason) }
enum HaltReason { Breakpoint(BreakpointId), Watchpoint(WatchpointId), StepCompleted, Trap(TrapKind) }
```

The Debugger owns the `Nes`. Frontends interact with the system through the Debugger; release builds incur zero overhead when no breakpoints/watchpoints are active.

### 6.2 Step controls

`run`, `pause`, `step_instruction`, `step_over` (one-shot BP at PC+3 if JSR), `step_out` (one-shot BP at return address), `step_frame` (until next vblank-NMI), `step_scanline`, `step_back` (time-travel mechanism), `run_to_cursor(addr)` (one-shot BP). One-shot breakpoints share machinery with regular breakpoints via an `auto_remove_on_hit` flag.

### 6.3 Breakpoints (v1)

```rust
enum BreakpointKind {
    Pc(u16),       // halt when PC == addr at instruction boundary
    NmiEntry,
    IrqEntry,
    Brk,           // halt on BRK opcode (common in-rom debug-break convention)
}

struct Breakpoint {
    id: BreakpointId,
    enabled: bool,
    kind: BreakpointKind,
    auto_remove_on_hit: bool,
    hit_count: u64,
}
```

Checked after each instruction step. Fast path: `breakpoints.is_empty()` short-circuits to one branch.

### 6.4 Watchpoints (v1)

```rust
struct Watchpoint {
    id: WatchpointId,
    enabled: bool,
    address_range: RangeInclusive<u16>,
    access: WatchAccess,            // Read | Write | ReadWrite
    bus: WatchBus,                  // CpuBus | PpuBus
    hit_count: u64,
}
```

`Bus::read` / `Bus::write` (and the analogous PPU bus paths inside the mapper) check `debugger.has_watchpoints()` (one bool) before any work. If true, accesses run through a small filter; matches set a `halt_after_instruction` flag. The Debugger halts at the *next instruction boundary* — never mid-instruction, which would leave the CPU in an indeterminate state.

When no watchpoints are set, the path is one branch — zero cost on the bus tick hot path.

### 6.5 Conditional breakpoints — DEFERRED

Conditional breakpoints (e.g., `A == $05 && scanline > 30`) are deferred until the v1 debugger has been used in practice and the need is concrete. Two reasons to delay:

- The clear permissive-license expression-evaluator landscape is sparse. `evalexpr` is the closest match technically but is AGPL-3.0-only, which would force `nies` to AGPL too. Other crates (`meval`, `fasteval`) are math-only. `rhai` is permissive but heavyweight.
- A hand-rolled grammar (~150-200 lines) is a real option, *because* the grammar is closed (fixed identifier set, one indexed accessor, no functions/strings/scopes). But it's not free, and it's premature.

When the feature is needed, this design will be revisited with a license decision for `nies` alongside.

### 6.6 Trace ring buffer

```rust
struct TraceEntry {
    cycle: u64,
    pc: u16,
    opcode: u8,
    operand_bytes: [u8; 2],
    a: u8, x: u8, y: u8, sp: u8, p: u8,    // CPU regs after the instruction
    ppu_scanline: u16,
    ppu_dot: u16,
    nmi_pending: bool,
    irq_pending: bool,
}
```

Per-entry: ~24 bytes. Default capacity 4096 (~96 KB). Configurable up to ~1M (~24 MB). Optional disk logging to a postcard-encoded file for offline analysis.

### 6.7 UI panels (egui, dockable)

All panels toggleable from the **Debug** menu. Layout uses egui docking; user can rearrange.

1. **Control panel.** Run/Pause/Step buttons; current state; cycle counter; FPS.
2. **CPU registers.** A, X, Y, PC, SP, P (with N V - B D I Z C broken out). Editable when paused.
3. **Disassembly.** Vertical list around current PC. Address, hex bytes, mnemonic, decoded operand. Click line → toggle PC breakpoint. Right-click → "Run to cursor."
4. **Memory viewer.** Hex+ASCII view of CPU bus. Tabs for CPU bus and PPU bus. Right-click byte → "Set watchpoint here."
5. **PPU state.** Sub-tabs:
   - Nametables: 2×2 grid with current scroll overlay.
   - Pattern tables: two 128×128 CHR banks rendered with selected palette.
   - Palette: 32-entry swatch view.
   - OAM: scrollable sprite list with mini render preview, sprite 0 highlighted.
6. **APU state.** Per-channel: enabled, period, volume, length counter. Oscilloscope per channel (last ~512 samples).
7. **Mapper state.** Generic key/value view fed by `MapperImpl::debug_dump()`.
8. **Breakpoints / Watchpoints panel.** List with checkboxes, edit/delete, hit count. Add via modal.
9. **Trace.** Most-recent-first scroll view. Search box (filter by PC, opcode). Right-click → "Jump to here."
10. **Time travel scrubber.** Slider over the snapshot ring; drag backward to scrub, "Restore here" to drop the future and resume.

Heavy panels (nametables, pattern tables) repaint at 30 Hz instead of 60 Hz to reduce GPU cost.

### 6.8 Performance contract

- No breakpoints + no watchpoints + zero-capacity trace: zero overhead vs running the core directly.
- With breakpoints/watchpoints: ~5-15% slowdown.
- With disk trace logging: ~30% slowdown plus disk bandwidth.

## 7. Testing strategy

Three layers, all enforced as hard merge gates:

### 7.1 Unit tests inside `nies-core`

Standard `#[cfg(test)] mod tests`. Targets:

- **CPU per-opcode tests via SingleStepTests/65x02 corpus** (vendored, ~50 MB). ~10K randomized cases per opcode covering every addressing mode, illegal opcode, dummy read, and page-cross. *This is the highest-leverage CPU correctness gate.*
- **PPU register tests.**
- **Mapper tests** (NROM in v1).
- **APU channel tests.**
- **Snapshot round-trip tests** (`apply_snapshot(snapshot()) == identity`) per-component and whole-`Nes`.

### 7.2 Test-ROM integration tests

Curated public-domain ROMs vendored under `crates/nies-core/tests/roms/`. Standard test protocol: ROM writes to $6000-$60FF (status byte at $6000 + ASCII status string at $6004+). Test harness runs each ROM headlessly until completion or 60s timeout (~3000 emulated frames at ~50× realtime is ≤1 sec).

**CPU (gates SMB1):**
- `nestest.nes` (compared to canonical Nintendulator log).
- `blargg/cpu_instrs/*`, `instr_misc.nes`, `instr_timing.nes`, `cpu_dummy_reads.nes`.

**PPU (gates SMB1):**
- `blargg/ppu_vbl_nmi/01-10*.nes` (10 sub-tests).
- `blargg/sprite_hit_tests_2005.10.05/*.nes`.
- `blargg/oam_read.nes`, `oam_stress.nes`.
- `nmi_sync/demo_ntsc.nes` (golden framebuffer hash).

**APU (gates audio in SMB1):**
- `blargg/apu_test/*.nes`.
- `blargg/dmc_tests/*.nes`.
- `blargg/apu_mixer/*.nes`.

**Mapper (per-mapper milestones, post-v1):**
- MMC1/MMC3: `blargg/mmc3_test_2/*.nes`, plus a custom MMC3 IRQ-edge test.

**Determinism:**
- `nes_instr_test`-style timing-sensitive ROM with golden state hash.

Each ROM has a sibling `<rom>.toml` describing expected outcome (success code, status string substring or framebuffer hash, max emulated frames, max wall-clock seconds). Test runner emits one `#[test]` per ROM.

### 7.3 Determinism / replay tests

- **Replay determinism** test (predefined input sequence, end-state hash match).
- **Snapshot–restore–replay** test.
- **Cross-platform determinism**: same replay on Mac/Linux/Windows native + WASM (via `wasm-bindgen-test` in headless Chrome) — all produce identical state hash.

### 7.4 CI

| Job | OS | Toolchain | Steps |
|---|---|---|---|
| native-macos | macos-latest | stable | `cargo build`, `cargo test --workspace`, `cargo clippy -- -D warnings` |
| native-linux | ubuntu-latest | stable | same |
| native-windows | windows-latest | stable | same |
| wasm | ubuntu-latest | stable + wasm32 | `cargo build --target wasm32-unknown-unknown -p nies-web`, `wasm-pack test --headless --chrome`, `trunk build --release` (with pinned wasm-opt) |
| fmt | ubuntu-latest | stable | `cargo fmt --check` |

All jobs are hard merge gates.

### 7.5 Test ROM licensing

Blargg's test ROMs: explicitly public domain. Nestest: by kevtris, customarily redistributed. nmi_sync, dmc_dma_during_read4: redistribution allowed by their authors. A `LICENSES.md` in the test ROM directory enumerates every ROM's source and license. No commercial ROMs are vendored.

### 7.6 Out of scope for automated testing

- Audio output quality (manual listening).
- UI layout (manual click-through).
- Frame pacing under real audio drivers (no audio device on CI runners).
- Game compatibility breadth (manual; tracked in a compat list).

### 7.7 TDD discipline

Every implementation task in the writing-plans-generated plan ties to specific test ROMs and SingleStepTests cases that must pass. PPU implementation walks blargg's `ppu_vbl_nmi` sub-tests in numeric order (pass `01-vbl_basics.nes` first, then `02-vbl_set_time.nes`, etc.). APU implementation walks `blargg/apu_test` similarly.

### 7.8 Documented test deferrals

Some vendored test ROMs cannot pass at the milestone where they're first relevant; they're vendored anyway so they can be enabled later without re-vendoring. Each is `#[ignore = "..."]` in the integration test suite with a clear reason; this list tracks when they're expected to start passing.

**M1 — vendored, marked `#[ignore]`, expected to pass at the listed milestone:**

| Test | Ignored at M1 because | Re-enable at |
|---|---|---|
| `blargg/cpu_instrs/03-immediate.nes` | The single failing case is opcode `$AB` (ATX/LXA), an unstable magic-constant illegal. SingleStepTests/65x02 uses magic constant `0xEE`; blargg's checksum was generated against a different (undocumented) constant. We match SingleStepTests, which is the more rigorous corpus. The other 64 immediate-mode opcodes in `03-immediate.nes` pass. No commercial NES game uses `$AB`. | **Permanent.** Documented as accepted divergence; the only resolution would be to make our `$AB` selectable per the corpus, which is not worth the complexity. |
| `blargg/instr_timing/1-instr_timing.nes` | Depends on the APU length counter to time instruction-by-instruction execution. APU is a stub at M1. | **M5** (APU implementation). |
| `blargg/instr_timing/2-branch_timing.nes` | Same dependency on APU length counter. | **M5** (APU implementation). |
| `blargg/cpu_instrs/cpu_instrs.nes` (combined) | Uses MMC1 (mapper 1). M1 ships only NROM. The 16 individual NROM sub-tests (`01-basics.nes` through `16-special.nes`) cover the same opcode-level content. | **M11** (MMC1). |
| `blargg/instr_misc.nes` | Uses MMC1. | **M11** (MMC1). |
| `blargg/instr_timing/instr_timing.nes` (combined) | Uses MMC1. | **M11**, then enabled together with the sub-tests once both M5 (APU) and M11 (MMC1) have shipped. |
| `blargg/cpu_dummy_reads.nes` | Uses CNROM (mapper 3). M1 ships only NROM. SingleStepTests' per-cycle bus-trace check already validates dummy-read behavior for every opcode at higher precision than `cpu_dummy_reads.nes` provides. | **M11+** (CNROM). |

The aggregate signal at M1: 256/256 SingleStepTests opcodes pass + 16/16 NROM-compatible blargg cpu_instrs sub-tests pass + nestest's automated mode matches Nintendulator (M1 Task 46).

## 8. Milestones

```
M0  → M1 → M2 → M3 → M4 → M5 → 🎯 M6 → M7 → M8 → M9 → M10 → (post-v1: M11+)
```

### M0 — Project skeleton

All four crates build; both binaries launch a blank window; CI green.

- Cargo workspace; placeholder `lib.rs` and `main.rs`.
- `nies-app`: winit window, wgpu clears to sentinel color.
- `nies-web`: same in `<canvas>` via Trunk.
- CI matrix (4 jobs) green.
- `LICENSES.md`, `README.md`, `rust-toolchain.toml`, `Trunk.toml` with pinned wasm-opt.

**Gate:** `cargo build --workspace` and `trunk build --release` succeed; CI green.

### M1 — Cartridge + CPU

The 6502 CPU is correct in isolation, including illegals.

- `Cartridge` parses iNES + NES 2.0; rejects malformed cleanly.
- `Cpu` implements all official + illegal opcodes per §3.4 policy.
- `Bus` skeleton with RAM and stub mapper (NROM CPU side only); PPU/APU placeholder.
- Tick discipline enforced.

**Gate:** SingleStepTests/65x02 corpus passes for every opcode (all 256). The 16 NROM sub-tests of `blargg/cpu_instrs/` (`01-basics.nes` through `16-special.nes`) all green, *except* `03-immediate.nes` is `#[ignore]`d due to the documented `$AB` corpus disagreement (see §7.8). `nestest.nes` automated mode matches the Nintendulator log byte-for-byte (M1 Task 46). The MMC1/CNROM-requiring combined runners (`cpu_instrs.nes`, `instr_misc.nes`, `instr_timing.nes`, `cpu_dummy_reads.nes`) are vendored but `#[ignore]`d at M1 — they get re-enabled at M11 when those mappers ship. The two `instr_timing` sub-tests are also `#[ignore]`d at M1 pending APU (M5).

### M2 — PPU

Per-dot rendering correct; NROM CHR / mirroring complete.

- Per-dot state machine, all four phases.
- Background fetch pipeline.
- Sprite evaluation including secondary OAM and sprite 0 hit.
- Vblank/NMI per spec, including suppression.
- A12 line plumbed (no-op for NROM).
- Palette-index framebuffer per frame.

**Gate:** `blargg/ppu_vbl_nmi/*`, `sprite_hit_tests_2005.10.05/*`, `oam_read.nes`, `oam_stress.nes` green. `nmi_sync/demo_ntsc.nes` matches golden framebuffer hash.

### M3 — Frontend rendering

Emulator framebuffer on screen at 60 Hz, native and WASM.

- wgpu pipeline, palette LUT shader, integer-scaled blit.
- Default palette shipped (FBX Smooth or Nostalgia, pick at implementation).
- Frame pacing temporarily vsync-driven.
- Both `nies-app` and `nies-web` render correctly.

**Gate:** `nmi_sync/demo_ntsc.nes` framebuffer matches golden hash on Mac and WASM. WASM build size + load time recorded for regression.

### M4 — Input

Two NES controllers driven from keyboard/gamepad; deterministic.

- Controller state, $4016/$4017 register handling, strobe.
- Default mappings (§5.5).
- Input event log integration.

**Gate:** Predefined input sequence applied to a test ROM produces same state hash on every run. Manual: SMB1 boots, title responds, level 1-1 plays visually correctly (silent).

### M5 — APU + audio

Sound works; pacing switches to audio-driven.

- All five channels per spec.
- 4-step / 5-step frame counter.
- DMC sample fetch via Bus pending-fetch flag.
- Resampler (Blackman sinc + linear interp).
- cpal integration: native (CoreAudio/ALSA/WASAPI), WASM (`audioworklet` preferred, `wasm-bindgen` fallback).
- Volume/mute hooks (UI in M10).

**Gate:** `blargg/apu_test/*`, `dmc_tests/*`, `apu_mixer/*` green. Additionally, the M1-deferred `blargg/instr_timing/1-instr_timing.nes` and `2-branch_timing.nes` un-`#[ignore]` and pass — those tests measure instruction timing using the APU length counter, which doesn't exist until M5 (see §7.8).

### 🎯 M6 — SMB1 milestone

The user's primary stated goal. SMB1 plays end-to-end with sound and input.

- Manual playthrough verification: title, levels 1-1 through 1-4, warps, music + SFX.
- Compat list document started; SMB1 marked playable.
- A pre-recorded "Mario walks right and jumps" replay captured as a regression smoke test.

**Gate:** all earlier gates still green; manual smoke test on Mac native and WASM.

### M7 — Save states

Save and restore arbitrary game state across sessions.

- serde derives on all `Nes` state types; `MapperKind` enum with per-variant Serialize.
- postcard wire format with file header.
- File I/O native; IndexedDB WASM.
- 10 numbered slots + 1 quicksave; hotkeys.

**Gate:** snapshot round-trip determinism test green; cross-platform determinism test green.

### M8 — Time travel

Rewind during play; debugger backward step API working.

- Snapshot ring with adaptive thinning.
- Input event log keyed by master cycle.
- Headless replay (`replay_to(target_cycle)`).
- Rewind hotkey behavior.

**Gate:** snapshot+restore+replay test green. Manual: rewind during SMB1 reverses Mario, releases seamlessly.

### M9 — Debugger

All debugger functionality from §6, minus conditional breakpoints.

- Backend: `Debugger`, breakpoints, watchpoints, one-shot, trace ring (in-memory + optional disk), full step controls.
- UI panels (§6.7): control, registers, disassembly, memory, PPU sub-tabs, APU, mapper, breakpoints/watchpoints, trace, time travel scrubber.
- Heavy panels repaint throttled to 30 Hz.
- F12 toggle.

**Gate:** manual exercise — set PC breakpoint inside SMB1, hit it, step over a JSR, step out, step back ~50 instructions, set watchpoint on player Y, observe firing on jump.

### M10 — Polish

Ship-quality v1.

- Rebinding wizard UX.
- Settings panel: audio config, palette, save-data location, integer scaling, PAR toggle.
- Recent ROMs.
- Per-channel mute UI.
- WASM build pipeline finalized; size budget asserted in CI.
- README documents controls, save-state hotkeys, debugger overview.

**Gate:** all earlier gates green; WASM bundle within size budget; both binaries deployable end-to-end.

### v1 ships at the end of M10.

### Post-v1 (out of v1 scope, listed for context)

- **M11:** MMC1 + UxROM mappers; compat list grows. Re-enables M1-deferred `blargg/cpu_instrs/cpu_instrs.nes`, `instr_misc.nes`, `instr_timing/instr_timing.nes` (combined runners use MMC1).
- **M12:** MMC3; confirms A12 IRQ machinery from day one.
- **M13:** CNROM, AxROM, FME-7, others as games of interest demand. Re-enables M1-deferred `blargg/cpu_dummy_reads.nes` (CNROM).
- **M14:** Optional NTSC composite filter, CRT shader, scanlines.
- **M15:** Reconsider conditional breakpoints (license decision + evalexpr vs roll-our-own).
- **M16:** Possibly netplay (its own project).

## 9. Open questions / future decisions

Non-blocking items to revisit later:

- **License for `nies`**: not chosen yet. Affects M15 (conditional breakpoints) and any future dependency choices that have copyleft-vs-permissive implications.
- **Default palette**: pick between FBX Smooth and Nostalgia during M3.
- **Audio sample rate**: default to 48 kHz, but config allows others. Validate behavior at 44.1 kHz and 96 kHz when M5 is done.
- **Trunk vs alternative WASM bundlers**: Trunk chosen; alternative `wasm-pack` + custom HTML is a fallback if Trunk pinning becomes painful.
- **Sub-instruction (tier 3) accuracy**: explicitly out of scope for v1; revisit if/when the project ambition extends to "compete with Mesen."

## 10. Appendix

### 10.1 Crate audit (May 2026)

Verified at design time:

- `bincode` v3.0.0: **deprecated** (Dec 2025, maintainer ceased due to harassment). Repo archived Aug 2025. README directs users to alternatives. **Replaced by postcard.**
- `postcard` v1.1.3: active, serde-based, no_std-friendly. **Adopted for save state format.**
- `winit` v0.31 in beta; **pinned to 0.30.x stable**.
- `wgpu` v29: active.
- `egui`/`egui-wgpu` v0.34: active.
- `cpal` v0.17.3 (Feb 2026): active. WebAudio backend via `wasm-bindgen`; lower-latency `audioworklet` backend on dedicated thread. **`audioworklet` preferred on WASM**.
- `gilrs` v0.11: active, supports WASM Gamepad API.
- `rfd` v0.17: active.
- `trunk` v0.21: active. `Trunk.toml` supports pinned wasm-opt invocation.
- `gloo-storage`/`gloo-file` v0.4: active.
- `web-time` v1.1: active, drop-in for `std::time::Instant` in browsers.
- `evalexpr` v13.1.0: active but **AGPL-3.0-only** — defers conditional breakpoint feature pending license decision.

### 10.2 wasm-opt pinning

Required because:

1. The `externref` processor must run before wasm-opt; without pinning, a breaking binaryen update could subtly corrupt externref handling.
2. Rust 1.87+ shipped wasm output that some wasm-opt versions rejected with "Bulk memory operations require bulk memory."

Pin a known-good binaryen version in `Trunk.toml`. Bump deliberately, test before committing.

### 10.3 Why bus-tick (Approach 1) over async-CPU (Approach 2)

The async-CPU approach (CPU as `async fn` that `.await`s a `tick()` future per cycle) makes "I forgot to tick" structurally impossible, mirroring hardware exactly. Trade-offs:

- Pro (async): cleaner correctness story; no discipline burden.
- Con (async): async-in-non-IO Rust is awkward; generators partially stable; debugging async stack frames meaningfully harder; integration with synchronous step-control debugger UI is messier.

Bus-tick achieves the same correctness property with a synchronous design by making `Bus::read` and `Bus::write` the only ways to access memory and ticking unconditionally inside both. Industry standard for serious Rust NES emulators (kpcyrd's nes, mvdnes, et al.).

### 10.4 Why enum-dispatch over `Box<dyn Mapper>`

- Serde-friendly without typetag (which is awkward on WASM).
- Slightly faster (no vtable indirection) — irrelevant in practice but free.
- Closed mapper set is explicit at the type level — adding a mapper requires an enum variant, surfacing at every match site.
- The post-v1 mapper roadmap (MMC1, UxROM, CNROM, MMC3 first) is bounded enough that a closed enum is comfortable.
