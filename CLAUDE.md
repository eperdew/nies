# Project context for Claude

`nies` is a Rust NES emulator. Current state: end of milestone M0 (project skeleton merged), M1 in progress on branch `m1-cpu-cartridge` (CPU + cartridge + test infrastructure).

## Authoritative documents

- **Design spec:** [`docs/superpowers/specs/2026-05-02-nes-emulator-design.md`](docs/superpowers/specs/2026-05-02-nes-emulator-design.md). Architecture decisions, milestone roadmap, accuracy tier (tier 2 / bus-tick), crate audit. Update via the brainstorming skill if scope or architecture changes.
- **Implementation plans:** `docs/superpowers/plans/`. Each milestone gets its own plan written incrementally (we don't pre-write all of them). Active plan: `2026-05-02-m1-cpu-cartridge.md`.

## Workspace layout

Four crates under `crates/`:

| Crate | Role | Notes |
|---|---|---|
| `nies-core` | Emulator backend (CPU, PPU, APU, mappers, save state, debugger). | **No I/O dependencies allowed** — no `std::fs`, no `std::time::SystemTime`, no audio/video device access, no threading. Determinism contract from spec §4.1. |
| `nies-ui` | Shared egui panels (game viewport, debugger UI). | Placeholder until M3+. |
| `nies-app` | Native binary (winit + wgpu). | M0 ships a sentinel-color clear; real rendering lands at M3. |
| `nies-web` | WASM `cdylib`, bundled with Trunk. | Same shape as `nies-app` but with async GPU bootstrap via `EventLoopProxy<UserEvent::GpuReady>`. |

## Toolchain

Pinned in `rust-toolchain.toml` to **Rust 1.94.0**. Workspace MSRV is **1.87** (the floor required by `wgpu = "29"`). The 1.94.0 / 1.87 split is decoupled on purpose:

- 1.94.0 is what we develop and CI against (recent stable)
- 1.87 is what `Cargo.toml` declares as the minimum supported version

**Don't bump these without a reason.** History: an earlier plan revision pinned 1.86.0 for a wasm-opt compatibility issue; that was incompatible with wgpu 29 (MSRV 1.87) so we bumped. The wasm-opt mitigation now lives in `Trunk.toml` (pinned binaryen).

## Git LFS

The SingleStepTests/65x02 corpus (`crates/nies-core/tests/data/65x02.tar.zst`, ~138 MB) is stored via Git LFS. The corpus drives ~2.5M per-opcode test cases used by the M1 CPU validation.

**Fresh clones need:**
```bash
git lfs install   # one-time per machine
git lfs pull      # fetch the corpus tarball
```

Without `git lfs pull`, the file at the path above is a 134-byte LFS pointer and any test that decompresses it will fail with a bad-data error.

CI handles this via `actions/cache@v4` keyed on the LFS pointer's hash. First CI run after a corpus update consumes LFS bandwidth; subsequent runs hit the cache and consume zero.

## Build / test commands

```bash
# Native build + test (excludes the wasm-only crate)
cargo build --workspace --exclude nies-web
cargo test --workspace --exclude nies-web

# WASM build
cargo build --target wasm32-unknown-unknown -p nies-web
trunk build --release   # produces dist/*.wasm

# Lints and fmt (must remain clean)
cargo fmt --all -- --check
cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
cargo clippy -p nies-web --target wasm32-unknown-unknown --all-targets -- -D warnings

# Run native binary (opens a window; close to exit)
RUST_LOG=info cargo run -p nies-app

# Run web build locally
trunk serve --release   # http://127.0.0.1:8080
```

## CI (GitHub Actions)

Workflow at [.github/workflows/ci.yml](.github/workflows/ci.yml). Five jobs:

- `Format check` (Linux): `cargo fmt --all -- --check`
- `Native (macOS / Linux / Windows)`: `cargo build` + `cargo test` + `cargo clippy`. **Excludes `nies-web`.** Requires the LFS corpus (cached after first run).
- `WASM` (Linux): `cargo build --target wasm32-unknown-unknown -p nies-web` + `trunk build --release`

Triggers on push to `main`/`master` and on pull requests. Feature branches need a (draft) PR for CI to run.

## Workflow conventions

### Never push to `master` without explicit permission

**Hard rule.** `master` is the integration branch. Pushes to it must be a deliberate action the user has authorized for that specific push.

- ✅ Push freely to feature branches (`m1-cpu-cartridge`, future `m2-…`, etc.) — these drive draft PRs and CI.
- ✅ Push to `master` only when the user has explicitly said to do so for the current change.
- ❌ Don't push to `master` as a side effect of "establishing the remote," "syncing branches," "cleaning up," etc.

Merging a feature branch into `master` locally is fine; the resulting `master` push must still be explicitly authorized.

### Commit messages

**Use single-quoted multi-line strings, not `$(cat <<'EOF' ... EOF)` heredoc subshells.** The shell command Claude Code's permission matcher sees is the literal command line, and heredoc subshells break the `Bash(git commit -m *)` allow rule. Single-quoted form:

```bash
git commit -m 'subject line

Body line 1.
Body line 2.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>'
```

Single-quoted strings can't contain literal `'`. Our commit messages don't use them; if one ever needs to, fall back to `"..."` and escape any `$` as `\$`.

### Per-task commits

The implementation plans group work into **plan tasks**. Each task ends in a single commit with a specific message body the plan supplies. Don't bundle tasks; reviewability matters.

The exception: prep commits (e.g., toolchain bumps) and plan corrections land as their own small commits, separate from the feature work.

### TDD discipline (CPU work)

For every CPU opcode: add the `#[test] fn opcode_NN_*()` entry first, watch it fail with the dispatch table's panic, then implement the opcode, then re-run and watch it pass. The SingleStepTests corpus checks both register/RAM final state AND the per-cycle bus access trace, so it catches missing dummy reads, wrong addressing-mode variants, off-by-one cycle counts, etc.

### CI gating

CI is the merge gate. Don't merge if any job is red. The fmt/clippy `-D warnings` settings are strict by design; fix at the source rather than weakening the lints.

## Known gotchas

- **`wgpu` 29.0.x API differences from older docs.** `Instance::new` takes the `InstanceDescriptor` **by value** (use `InstanceDescriptor::new_without_display_handle_from_env()`); `Surface::get_current_texture` returns `CurrentSurfaceTexture` enum (not `Result`); `RenderPassColorAttachment` requires `depth_slice: None`; `RenderPassDescriptor` requires `multiview_mask: None`. See `crates/nies-app/src/main.rs` for the canonical setup.
- **`Bus::peek` for debugger inspection** uses an `unsafe` cast to call `&mut self` mapper methods from a `&self` context. Safe at M1 (NROM is read-side-effect-free); revisit when stateful mappers land.
- **Workspace `members` list and crate-creation order.** Cargo eagerly validates the manifest before applying `-p` filtering. If you need to create a crate that isn't in `members` yet, add it to the list AND create the directory in the same commit. Never list non-existent members.
- **`bincode` is deprecated** (Dec 2025; maintainer ceased due to harassment). We use **`postcard`** for save state serialization (M7+). Don't add bincode.
- **`evalexpr` is AGPL-3.0** — incompatible with the project's MIT-or-Apache-2.0 license intent. Conditional breakpoints (M9) are deferred until we pick a non-AGPL solution or roll our own.
- **Plan source bugs** are possible — when `assert_eq!` constants in plan tests look wrong, check the underlying byte arithmetic. Past examples: `(i / 256) as u8` repeating produces 0 at offset 0x4000, not 0x80.

## Subagent dispatching

Per the user's preference (option ii from M1 mid-stream check-in):

- **Bigger units, lighter reviews.** Group multiple plan tasks per implementer dispatch (e.g., a whole opcode-family group). Use the SingleStepTests corpus as the spec compliance check; only dispatch full subagent spec/code reviews for foundational or judgment-heavy units.
- **No re-review loops on cosmetic findings.** If the only issues are commit-message escapes, doc nits, or unused-dep cleanups, fix inline and move on.

## What's intentionally NOT in scope (current milestone)

For M1 specifically:
- PPU register effects (read clear, PPUDATA buffer) — M2.
- APU sample generation — M5.
- Controller strobe/shift register — M4.
- All non-NROM mappers — M11+ post-v1.
- Save state machinery (postcard wire format with header) — M7.
- Debugger UI panels — M9.
- Snapshot/restore round-trip integration — M7.

The trait shapes are designed against these later milestones (e.g., `Bus::tick`'s DMC fetch service path is wired but inert; `MapperImpl` has `notify_a12` with default no-op for MMC3-readiness). When working on M1, **don't preempt later milestones** — placeholder shapes are deliberate.

