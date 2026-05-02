# M0 — Project Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the existing single-crate `cargo new` scaffolding into a four-crate Cargo workspace (`nies-core`, `nies-ui`, `nies-app`, `nies-web`) where the native binary opens a window with a sentinel-colored clear, the WASM binary does the same in a `<canvas>`, and CI verifies builds on macOS / Linux / Windows / WASM with formatting checks.

**Architecture:** Cargo workspace at the repo root. `nies-core` and `nies-ui` are plain libraries (empty at M0). `nies-app` is a native binary using winit + wgpu. `nies-web` is a `cdylib` built with Trunk and a pinned `wasm-opt` post-processing pass. CI uses GitHub Actions with a job matrix.

**Tech Stack:** Rust 2024 edition, winit 0.30, wgpu 29, wasm-bindgen, Trunk, GitHub Actions.

---

## Reference: spec section ↔ task mapping

| Spec § | Requirement | Tasks |
|---|---|---|
| §3.1 | Workspace layout (4 crates under `crates/`) | 1, 6, 8, 11, 16 |
| §3.1 | `nies-core` is no-I/O library | 6 |
| §5.8 / §10.2 | Trunk + pinned wasm-opt | 18 |
| §5.1 | Crate stack (winit 0.30, wgpu 29, etc.) | 12, 17 |
| §7.4 | CI matrix (5 jobs) | 21 |
| §8 (M0 gate) | `cargo build --workspace` and `trunk build --release` succeed | 9, 19, 24 |
| §8 (M0 gate) | `LICENSES.md`, `README.md`, `rust-toolchain.toml`, `Trunk.toml` | 3, 18, 22, 23 |

---

## Notes for the implementer

- **Working directory** is `/Users/eperdew/Software/nies`. Run all commands from there unless otherwise noted.
- **Existing state:** the repo currently has a default `cargo new`-style scaffolding (`src/main.rs` "Hello, world!", `Cargo.toml` with no dependencies, `Cargo.lock`, basic `.gitignore`). One commit so far (the design spec). No git remote is assumed.
- **Manual verification steps** are called out explicitly. Some steps can't be automated in CI (interactive window display); verify on the local Mac.
- **Trunk + wasm-opt:** wasm-opt versions are referenced by Trunk's convention, e.g. `version_120`. If the pinned version fails to install on a runner, bump to a newer one and update both `Trunk.toml` and the CI job.
- **Rust toolchain:** plan pins to a specific stable Rust version pre-1.87 to sidestep the wasm-opt bulk-memory issue noted in the spec (§10.2). If you find a newer version with verified working wasm-opt, you may bump.
- **Lockfile:** workspace `Cargo.lock` should be committed.
- **WASM dependencies in CI:** the native build matrix excludes `nies-web` (`--exclude nies-web`) because it's a wasm-only crate. The wasm job builds it explicitly.

---

## Task 1: Tear down single-crate Cargo.toml

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Replace Cargo.toml with workspace declaration**

Overwrite `Cargo.toml` with:

```toml
[workspace]
resolver = "3"
members = [
    "crates/nies-core",
    "crates/nies-ui",
    "crates/nies-app",
    "crates/nies-web",
]

[workspace.package]
version = "0.0.0"
edition = "2024"
rust-version = "1.86"
authors = ["Eric Perdew"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/eperdew/nies"
publish = false

[workspace.dependencies]
# Pinned across all crates
log = "0.4"
```

- [ ] **Step 2: Verify the file parses**

Run: `cargo metadata --no-deps --format-version 1 >/dev/null`
Expected: exits 0 with no output. (It will warn that the listed members don't exist yet — that's expected; we create them in later tasks.)

If `cargo metadata` errors because workspace members don't exist, that's *expected* until later tasks. Skip this verification for now and accept: the file is syntactically valid TOML.

To verify TOML syntax instead, run: `python3 -c "import tomllib; tomllib.loads(open('Cargo.toml').read())"`
Expected: exits 0 with no output.

---

## Task 2: Remove old single-crate sources

**Files:**
- Delete: `src/main.rs`
- Delete: `src/` (directory)
- Delete: `Cargo.lock`

- [ ] **Step 1: Delete the old src/ tree**

```bash
rm -r src/
rm Cargo.lock
```

- [ ] **Step 2: Verify they're gone**

```bash
ls src/ 2>&1 | head -1   # expect: "ls: src/: No such file or directory"
ls Cargo.lock 2>&1 | head -1  # expect: "ls: Cargo.lock: No such file or directory"
```

---

## Task 3: Create rust-toolchain.toml

**Files:**
- Create: `rust-toolchain.toml`

- [ ] **Step 1: Write the toolchain pin**

```toml
[toolchain]
channel = "1.86.0"
components = ["rustfmt", "clippy"]
targets = ["wasm32-unknown-unknown"]
profile = "default"
```

- [ ] **Step 2: Verify rustup picks it up**

Run: `rustup show active-toolchain`
Expected: a line like `1.86.0-aarch64-apple-darwin (overridden by '/Users/eperdew/Software/nies/rust-toolchain.toml')`

If rustup says the toolchain is not installed, run: `rustup toolchain install 1.86.0`

---

## Task 4: Update .gitignore

**Files:**
- Modify: `.gitignore`

- [ ] **Step 1: Replace .gitignore contents**

```
/target
/dist
.DS_Store
*.swp
```

- [ ] **Step 2: Verify it parses (no command needed; just visual check)**

```bash
cat .gitignore
```

Expected output: the four lines above.

---

## Task 5: Commit workspace scaffolding teardown

**Files:** (commit only)

- [ ] **Step 1: Stage and commit**

```bash
git add Cargo.toml rust-toolchain.toml .gitignore
git rm -r --cached src/ 2>/dev/null || true   # in case src/ was tracked
git status
```

Expected: `Cargo.toml`, `rust-toolchain.toml`, `.gitignore` show as added/modified; `src/main.rs` and `Cargo.lock` show as deleted (or not appearing if never tracked).

```bash
git commit -m "$(cat <<'EOF'
chore: restructure as Cargo workspace

Replace the cargo-new single-crate scaffolding with an empty workspace
declaration and supporting files (rust-toolchain pin, .gitignore for
trunk dist output). Member crates are added in subsequent commits.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 2: Verify commit**

Run: `git log --oneline -2`
Expected: two lines — the new commit and the original spec commit.

---

## Task 6: Create nies-core crate

**Files:**
- Create: `crates/nies-core/Cargo.toml`
- Create: `crates/nies-core/src/lib.rs`

- [ ] **Step 1: Create the crate directory and Cargo.toml**

```bash
mkdir -p crates/nies-core/src
```

Write `crates/nies-core/Cargo.toml`:

```toml
[package]
name = "nies-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
log.workspace = true

[lints.rust]
unused_must_use = "deny"
```

- [ ] **Step 2: Write the lib.rs with a smoke test**

Write `crates/nies-core/src/lib.rs`:

```rust
//! `nies-core` — NES emulator backend (CPU, PPU, APU, mappers, save state, debugger).
//!
//! No I/O dependencies: this crate must remain free of `std::fs`, `std::time::SystemTime`,
//! audio/video device access, and threading. The deterministic emulator core lives here.

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_smoke() {
        assert_eq!(2 + 2, 4);
    }
}
```

- [ ] **Step 3: Verify the crate builds and the test passes**

Run: `cargo test -p nies-core`
Expected: `test tests::workspace_smoke ... ok`, `1 passed; 0 failed`.

---

## Task 7: Create nies-ui crate

**Files:**
- Create: `crates/nies-ui/Cargo.toml`
- Create: `crates/nies-ui/src/lib.rs`

- [ ] **Step 1: Create the crate**

```bash
mkdir -p crates/nies-ui/src
```

Write `crates/nies-ui/Cargo.toml`:

```toml
[package]
name = "nies-ui"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
nies-core = { path = "../nies-core" }
log.workspace = true

[lints.rust]
unused_must_use = "deny"
```

- [ ] **Step 2: Write the lib.rs**

Write `crates/nies-ui/src/lib.rs`:

```rust
//! `nies-ui` — egui panels shared by both native and WASM frontends.
//!
//! At M0 this is a placeholder. Game viewport, debugger panels, and settings UI
//! are added in later milestones.
```

- [ ] **Step 3: Verify the workspace still builds**

Run: `cargo build --workspace`
Expected: `Compiling nies-core …`, `Compiling nies-ui …`, finishes successfully.

---

## Task 8: Commit nies-core and nies-ui

**Files:** (commit only)

- [ ] **Step 1: Stage and commit**

```bash
git add crates/nies-core crates/nies-ui
git commit -m "$(cat <<'EOF'
scaffold: add nies-core and nies-ui empty crates

Both are placeholder libraries for now. nies-core is documented as
the no-I/O emulator backend; nies-ui will hold shared egui panels.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 2: Verify**

Run: `git log --oneline -3`
Expected: three commits.

---

## Task 9: Create nies-app crate with stub main

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `crates/nies-app/Cargo.toml`
- Create: `crates/nies-app/src/main.rs`

> **Note (deviation from earlier plan revision):** Cargo loads the full workspace manifest before applying `-p` filters, so listing yet-to-exist crates in `members` breaks every cargo command. As a result, `crates/nies-app` and `crates/nies-web` were removed from the workspace `members` list in Unit 2's commit. This task **adds `crates/nies-app` back**.

- [ ] **Step 1: Add `crates/nies-app` back to workspace members**

Edit `Cargo.toml` (workspace root). The `members` list currently reads:

```toml
members = [
    "crates/nies-core",
    "crates/nies-ui",
]
```

Update to:

```toml
members = [
    "crates/nies-core",
    "crates/nies-ui",
    "crates/nies-app",
]
```

- [ ] **Step 2: Create the crate directory**

```bash
mkdir -p crates/nies-app/src
```

Write `crates/nies-app/Cargo.toml`:

```toml
[package]
name = "nies-app"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
nies-core = { path = "../nies-core" }
nies-ui = { path = "../nies-ui" }
log.workspace = true
env_logger = "0.11"
winit = "0.30"
wgpu = "29"
pollster = "0.4"

[lints.rust]
unused_must_use = "deny"
```

- [ ] **Step 3: Write a stub main.rs that compiles**

Write `crates/nies-app/src/main.rs`:

```rust
fn main() {
    env_logger::init();
    log::info!("nies-app starting");
}
```

- [ ] **Step 4: Verify the crate builds**

Run: `cargo build -p nies-app`
Expected: builds successfully (will download winit/wgpu/etc. on first run).

- [ ] **Step 5: Verify the stub runs**

Run: `RUST_LOG=info cargo run -p nies-app`
Expected: prints `[INFO  nies_app] nies-app starting` then exits cleanly.

---

## Task 10: Implement winit + wgpu window in nies-app

**Files:**
- Modify: `crates/nies-app/src/main.rs`

- [ ] **Step 1: Replace main.rs with winit + wgpu rendering**

Write `crates/nies-app/src/main.rs`:

```rust
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

const SENTINEL_CLEAR: wgpu::Color = wgpu::Color {
    r: 0.6,
    g: 0.05,
    b: 0.6,
    a: 1.0,
};

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .expect("create wgpu surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("request adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("request device");

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        Self {
            surface,
            device,
            queue,
            config,
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(SENTINEL_CLEAR),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("nies")
                        .with_inner_size(winit::dpi::LogicalSize::new(640, 480)),
                )
                .expect("create window"),
        );
        let gpu = pollster::block_on(GpuState::new(window.clone()));
        self.window = Some(window);
        self.gpu = Some(gpu);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => gpu.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                if let Err(e) = gpu.render() {
                    log::error!("render error: {e:?}");
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();
    log::info!("nies-app starting");
    let event_loop = EventLoop::new().expect("create event loop");
    let mut app = App::default();
    event_loop
        .run_app(&mut app)
        .expect("run event loop");
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p nies-app`
Expected: builds successfully. Warnings about unused imports are acceptable; warnings should not include errors.

- [ ] **Step 3: Manually verify it renders the sentinel color**

Run: `RUST_LOG=info cargo run -p nies-app`
Expected: a 640×480 window opens titled "nies" with a magenta-purple background. Closing the window terminates the process cleanly.

If the window doesn't open or the color is wrong, debug before proceeding. Common failures: wrong wgpu adapter on macOS (try `WGPU_BACKEND=metal` env var), surface format issues (check the `caps.formats[0]` selection).

---

## Task 11: Commit nies-app

**Files:** (commit only)

- [ ] **Step 1: Stage and commit**

```bash
git add crates/nies-app Cargo.lock
git commit -m "$(cat <<'EOF'
feat(app): native window with sentinel-colored clear

Implements a minimal winit + wgpu setup for the native binary.
Opens a 640x480 window and clears it to a sentinel magenta color
so subsequent rendering work has a baseline to build on.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 2: Verify**

Run: `git log --oneline -4`
Expected: four commits.

---

## Task 12: Create nies-web crate (cdylib + WASM entry)

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `crates/nies-web/Cargo.toml`
- Create: `crates/nies-web/src/lib.rs`

> **Note (deviation from earlier plan revision):** see the same note on Task 9. The workspace `members` list was reduced in Unit 2; this task adds `crates/nies-web` back.

- [ ] **Step 1: Add `crates/nies-web` back to workspace members**

Edit `Cargo.toml` (workspace root). The `members` list currently reads:

```toml
members = [
    "crates/nies-core",
    "crates/nies-ui",
    "crates/nies-app",
]
```

Update to:

```toml
members = [
    "crates/nies-core",
    "crates/nies-ui",
    "crates/nies-app",
    "crates/nies-web",
]
```

- [ ] **Step 2: Create the crate**

```bash
mkdir -p crates/nies-web/src
```

Write `crates/nies-web/Cargo.toml`:

```toml
[package]
name = "nies-web"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
nies-core = { path = "../nies-core" }
nies-ui = { path = "../nies-ui" }
log.workspace = true
console_log = "1"
console_error_panic_hook = "0.1"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
winit = { version = "0.30", features = ["rwh_06"] }
wgpu = { version = "29", default-features = false, features = ["webgpu", "webgl"] }
web-sys = { version = "0.3", features = [
    "Document",
    "Window",
    "Element",
    "HtmlCanvasElement",
] }

[lints.rust]
unused_must_use = "deny"
```

- [ ] **Step 3: Write a placeholder lib.rs**

Write `crates/nies-web/src/lib.rs`:

```rust
//! `nies-web` — WASM frontend for the nies emulator.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    log::info!("nies-web starting");
}
```

- [ ] **Step 4: Verify it builds for the WASM target**

Run: `cargo build -p nies-web --target wasm32-unknown-unknown`
Expected: builds successfully.

---

## Task 13: Implement winit + wgpu rendering in nies-web

**Files:**
- Modify: `crates/nies-web/src/lib.rs`

WASM bootstrapping wgpu is async, but winit's event handler runs synchronously. We bridge using winit's `EventLoopProxy` + user-event pattern: `resumed()` spawns an async task that creates the `GpuState`, then sends it back to the event loop as a user event, which the synchronous handler stores. From then on, rendering works the same as the native code path.

- [ ] **Step 1: Replace lib.rs with the WASM rendering setup**

Write `crates/nies-web/src/lib.rs`:

```rust
//! `nies-web` — WASM frontend for the nies emulator.

use std::sync::Arc;

use wasm_bindgen::prelude::*;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
use winit::window::{Window, WindowId};

const SENTINEL_CLEAR: wgpu::Color = wgpu::Color {
    r: 0.6,
    g: 0.05,
    b: 0.6,
    a: 1.0,
};

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .expect("create wgpu surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("request adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .expect("request device");

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        Self {
            surface,
            device,
            queue,
            config,
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(SENTINEL_CLEAR),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}

/// User events posted from async tasks back into the synchronous event handler.
enum UserEvent {
    GpuReady(GpuState),
}

struct App {
    proxy: EventLoopProxy<UserEvent>,
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
}

impl App {
    fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            proxy,
            window: None,
            gpu: None,
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("nies-canvas"))
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("find #nies-canvas in document");

        let attrs = Window::default_attributes()
            .with_title("nies")
            .with_canvas(Some(canvas));

        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        self.window = Some(window.clone());

        let proxy = self.proxy.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let gpu = GpuState::new(window).await;
            let _ = proxy.send_event(UserEvent::GpuReady(gpu));
        });
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::GpuReady(gpu) => {
                self.gpu = Some(gpu);
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => gpu.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                if let Err(e) = gpu.render() {
                    log::error!("render error: {e:?}");
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    log::info!("nies-web starting");

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let proxy = event_loop.create_proxy();
    let app = App::new(proxy);
    event_loop.spawn_app(app);
    Ok(())
}
```

Key points:

- The event loop is built with `EventLoop::<UserEvent>::with_user_event().build()` so it can deliver user-typed events back to the handler.
- `App::resumed` creates the `Window`, then spawns an async task. The task awaits `GpuState::new`, then sends `UserEvent::GpuReady` back through the proxy.
- `App::user_event` receives the ready GPU state on the synchronous side, stores it, and requests a redraw.
- From `window_event::RedrawRequested` onward, rendering works identically to the native version.

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p nies-web --target wasm32-unknown-unknown`
Expected: builds successfully (warnings acceptable).

If `with_user_event()` is not found, your winit version may have a different builder API; check `winit` docs at the version pinned in `Cargo.toml`. As of winit 0.30.x the construction is `EventLoop::<UserEvent>::with_user_event().build()`.

---

## Task 14: Create the index.html for nies-web

**Files:**
- Create: `crates/nies-web/index.html`

- [ ] **Step 1: Write the HTML host page**

Write `crates/nies-web/index.html`:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>nies</title>
    <link data-trunk rel="rust" data-bin="nies-web" data-type="main" />
    <style>
        html, body {
            margin: 0;
            padding: 0;
            background: #111;
            color: #ddd;
            font-family: system-ui, sans-serif;
            height: 100%;
        }
        body {
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
        }
        canvas {
            display: block;
            width: 640px;
            height: 480px;
            border: 1px solid #333;
        }
    </style>
</head>
<body>
    <canvas id="nies-canvas" width="640" height="480"></canvas>
</body>
</html>
```

> **Implementer note:** Trunk's `data-bin` attribute names the workspace member to compile. Since `nies-web` is a `cdylib`, Trunk treats the cdylib output as the wasm artifact and generates the necessary glue. If Trunk complains about `data-bin` on a cdylib, remove the attribute and let Trunk auto-detect from `Cargo.toml`'s `[lib]` section.

---

## Task 15: Create Trunk.toml at workspace root

**Files:**
- Create: `Trunk.toml`

- [ ] **Step 1: Write Trunk.toml**

Write `Trunk.toml`:

```toml
[build]
target = "crates/nies-web/index.html"
release = false
dist = "dist"

[tools]
# Pin wasm-bindgen-cli to a recent compatible version. Bump deliberately
# alongside wasm-bindgen in Cargo.toml.
wasm-bindgen = "0.2.99"
# Pin wasm-opt for: (a) externref correctness — externref processor must run
# before wasm-opt; (b) sidestepping the Rust 1.87+ "Bulk memory operations
# require bulk memory" issue by pinning to a known-good binaryen.
wasm-opt = "version_120"

[serve]
address = "127.0.0.1"
port = 8080
```

- [ ] **Step 2: Install trunk if not already present**

Check: `trunk --version`
If not installed, run: `cargo install trunk --version 0.21.14 --locked`
Expected after install: `trunk --version` prints `trunk 0.21.14`.

- [ ] **Step 3: Verify Trunk can build the WASM crate**

Run: `trunk build`
Expected: builds successfully; output ends with something like `success` and a `dist/` directory is created at the workspace root containing `index.html`, `*.wasm`, and `*.js`.

If trunk errors on `wasm-opt` install, try removing the `wasm-opt` line from `Trunk.toml` temporarily, rebuild, then re-add and bump the version. Some networks block GitHub-released binaries; the `WASM_BINDGEN_DEBUG=1` and `TRUNK_TOOLS_WASM_OPT=...` env vars are escape hatches.

- [ ] **Step 4: Verify the release build (with wasm-opt)**

Run: `trunk build --release`
Expected: builds successfully. The output `.wasm` file is meaningfully smaller than the debug build's. Record the size for later regression baselines.

```bash
ls -lh dist/*.wasm
```

Expected: a single `.wasm` file under, say, ~5 MB at this stage.

---

## Task 16: Manually verify nies-web runs in a browser

**Files:** (no file changes)

- [ ] **Step 1: Run trunk serve**

Run: `trunk serve --release` (run in background or in a separate terminal)

- [ ] **Step 2: Open a browser to http://127.0.0.1:8080**

Expected: the page loads. After a brief moment (while the async GPU bootstrap completes), the canvas fills with the magenta-purple sentinel color. The browser DevTools Console should show `nies-web starting` (info log).

If the page errors with `WebGL2 unsupported` or similar, your browser may need WebGPU enabled. nies-web is built with both `webgpu` and `webgl` features so it should fall back; if it doesn't, capture the console error for M3 follow-up but continue with the rest of M0.

If the canvas never fills (stays the default white/transparent) but no error appears, check that `with_user_event()` is being called and that `App::user_event` is reached — add a `log::info!("user_event: gpu ready")` for debugging, then remove it before committing.

---

## Task 17: Commit nies-web and Trunk config

**Files:** (commit only)

- [ ] **Step 1: Stage and commit**

```bash
git add crates/nies-web Trunk.toml Cargo.lock
git commit -m "$(cat <<'EOF'
feat(web): WASM canvas with sentinel-colored clear via Trunk

Adds the nies-web cdylib with a winit + wgpu setup that bootstraps
the GPU asynchronously and bridges back to the synchronous event
loop via EventLoopProxy + UserEvent::GpuReady. The canvas fills
with the same sentinel color as the native binary.

Trunk.toml pins wasm-bindgen-cli and wasm-opt versions to ensure
deterministic WASM builds and to avoid the known Rust 1.87+
wasm-opt bulk-memory issue.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 2: Verify**

Run: `git log --oneline -5`
Expected: five commits.

---

## Task 18: Add CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create the CI workflow**

```bash
mkdir -p .github/workflows
```

Write `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main, master]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings

jobs:
  fmt:
    name: Format check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: 1.86.0
          components: rustfmt
      - run: cargo fmt --all -- --check

  native:
    name: Native (${{ matrix.os }})
    strategy:
      fail-fast: false
      matrix:
        os: [macos-latest, ubuntu-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: 1.86.0
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - name: Linux build deps
        if: matrix.os == 'ubuntu-latest'
        run: sudo apt-get update && sudo apt-get install -y libasound2-dev libudev-dev pkg-config
      - run: cargo build --workspace --exclude nies-web
      - run: cargo test --workspace --exclude nies-web
      - run: cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings

  wasm:
    name: WASM
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: 1.86.0
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - name: Install trunk
        run: cargo install trunk --version 0.21.14 --locked
      - run: cargo build --target wasm32-unknown-unknown -p nies-web
      - run: trunk build --release
      - name: WASM size report
        run: ls -lh dist/*.wasm
```

- [ ] **Step 2: Lint the YAML locally**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`
Expected: exits 0 with no output.

If `yaml` is not installed: `python3 -c "import json, sys, re; ..."` is overkill — install via `pip3 install pyyaml` or skip syntactic verification (GitHub will tell you on push).

---

## Task 19: Add LICENSES.md

**Files:**
- Create: `LICENSES.md`

- [ ] **Step 1: Write LICENSES.md**

Write `LICENSES.md`:

```markdown
# Licenses

## nies (this project)

`nies` is dual-licensed under the MIT License and the Apache License 2.0
at the user's option. Both licenses are reproduced in `LICENSE-MIT` and
`LICENSE-APACHE` (to be added).

The license decision is provisional and may change before v1 ships.
See `docs/superpowers/specs/2026-05-02-nes-emulator-design.md` §9 for context.

## Test ROMs

Test ROMs vendored under `crates/nies-core/tests/roms/` are added in
later milestones (M1+). Each will be enumerated here with its source
and license. None will be commercial ROMs.

## Third-party crates

Third-party Rust crate licenses are surfaced via `cargo about`
(or equivalent) prior to v1 release. None are included verbatim here.
```

> **Implementer note:** the actual `LICENSE-MIT` and `LICENSE-APACHE` files are not added at M0 because the spec (§9) flags the license decision as an open question. We add `LICENSES.md` as a placeholder and revisit at M10 (polish).

---

## Task 20: Update README.md

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write the README**

Write `README.md`:

```markdown
# nies

An NES emulator written in Rust. Targets native macOS / Linux / Windows
and the web (WebAssembly).

## Status

In active development. See
[`docs/superpowers/specs/2026-05-02-nes-emulator-design.md`](docs/superpowers/specs/2026-05-02-nes-emulator-design.md)
for the full design and milestone roadmap.

Current milestone: **M0 — Project Skeleton**. The native binary opens a
window with a sentinel color clear; the WASM build loads in a browser
canvas. No emulation yet.

## Workspace layout

- `crates/nies-core` — emulator backend (CPU, PPU, APU, mapper, save
  state, debugger). No I/O dependencies.
- `crates/nies-ui` — egui panels shared by both frontends.
- `crates/nies-app` — native binary (winit + wgpu).
- `crates/nies-web` — WASM library, bundled with Trunk.

## Build

Native:

```bash
cargo run -p nies-app
```

WASM (requires `trunk` installed via `cargo install trunk --version 0.21.14 --locked`):

```bash
trunk serve --release
# Then open http://127.0.0.1:8080 in a WebGPU-capable browser.
```

## Testing

```bash
cargo test --workspace --exclude nies-web
```

## License

Dual-licensed under MIT or Apache-2.0. See [`LICENSES.md`](LICENSES.md) for details.
```

---

## Task 21: Final verification — full workspace build + tests

**Files:** (no file changes)

- [ ] **Step 1: Clean and full rebuild**

```bash
cargo clean
cargo build --workspace --exclude nies-web
```

Expected: builds successfully on Mac.

- [ ] **Step 2: Run all tests**

```bash
cargo test --workspace --exclude nies-web
```

Expected: at least one test (`workspace_smoke`) passes; no failures.

- [ ] **Step 3: WASM build**

```bash
cargo build --target wasm32-unknown-unknown -p nies-web
trunk build --release
```

Expected: both succeed. The `dist/*.wasm` file exists.

- [ ] **Step 4: Lints and format**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
```

Expected: both exit 0.

If `cargo fmt --check` complains, run `cargo fmt --all` (without `--check`) to auto-fix, then re-verify.

---

## Task 22: Final commit and (optional) push

**Files:** (commit only)

- [ ] **Step 1: Stage all remaining files**

```bash
git add .github/workflows/ci.yml LICENSES.md README.md
git status
```

Expected: only the three new files are staged (CI workflow, LICENSES, README).

- [ ] **Step 2: Commit**

```bash
git commit -m "$(cat <<'EOF'
ci+docs: add multi-platform CI matrix, README, and license placeholder

CI verifies fmt, native build/test/clippy on macOS/Linux/Windows,
and the WASM build via Trunk. README documents current status,
workspace layout, and build commands. LICENSES.md is a placeholder
pending the §9 license decision.

This completes M0 (project skeleton).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 3: (Optional) push to remote**

If a GitHub remote is configured:

```bash
git remote -v   # verify a remote is set
git push -u origin master   # or main, depending on default branch
```

Expected: push succeeds; CI runs on the remote.

If no remote yet, skip. The CI workflow file exists and will run on first push.

- [ ] **Step 4: (If pushed) verify CI green**

Run: `gh pr checks` or visit the GitHub Actions tab.
Expected: all 5 jobs (fmt, native ×3, wasm) green.

If any job is red, fix the cause and commit a fix; do not merge until all are green.

---

## Acceptance checklist for M0

- [ ] `cargo build --workspace --exclude nies-web` succeeds on macOS.
- [ ] `cargo test --workspace --exclude nies-web` passes.
- [ ] `cargo fmt --check` and `cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings` succeed.
- [ ] `trunk build --release` produces a `dist/*.wasm` file.
- [ ] `cargo run -p nies-app` opens a window with a magenta-purple sentinel color.
- [ ] `trunk serve --release` loads the page at http://127.0.0.1:8080 and the canvas fills with the magenta-purple sentinel color.
- [ ] CI workflow file exists at `.github/workflows/ci.yml`.
- [ ] `README.md` and `LICENSES.md` exist at the workspace root.
- [ ] All M0 commits are clean (no half-finished work, no `git status` debris).

When all of these are checked, M0 is done. Move on to writing the M1 plan.
