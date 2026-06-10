use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

enum RenderOutcome {
    Presented,
    Reconfigured,
    Occluded,
}

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: nies_ui::NesRenderer,
    // Boxed for parity with the web frontend, where moving the ~66 KB Nes
    // (inline PPU framebuffer) by value overflows the wasm shadow stack.
    nes: Box<nies_core::Nes>,
    keyboard: nies_ui::input::KeyboardState,
}

impl GpuState {
    async fn new(window: Arc<Window>, rom_bytes: &[u8]) -> Self {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
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

        let renderer = nies_ui::NesRenderer::new(&device, &queue, config.format);
        let nes = Box::new(
            nies_core::Nes::from_rom_bytes(rom_bytes).unwrap_or_else(|e| {
                log::error!("ROM failed to parse ({e:?}); falling back to embedded demo");
                nies_core::Nes::from_rom_bytes(nies_core::demo_rom_bytes())
                    .expect("demo ROM builds")
            }),
        );

        Self {
            surface,
            device,
            queue,
            config,
            renderer,
            nes,
            keyboard: nies_ui::input::KeyboardState::default(),
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

    fn render(&mut self) -> RenderOutcome {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return RenderOutcome::Reconfigured;
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                return RenderOutcome::Occluded;
            }
            other => {
                log::warn!("surface acquire returned {other:?}");
                return RenderOutcome::Occluded;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.nes.run_frame();
        self.renderer.upload_frame(&self.queue, self.nes.frame());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        self.renderer
            .render(&mut encoder, &view, (self.config.width, self.config.height));
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        RenderOutcome::Presented
    }
}

struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
    rom_bytes: Vec<u8>,
}

impl App {
    fn new(rom_bytes: Vec<u8>) -> Self {
        Self {
            window: None,
            gpu: None,
            rom_bytes,
        }
    }
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
        let gpu = pollster::block_on(GpuState::new(window.clone(), &self.rom_bytes));
        self.window = Some(window);
        self.gpu = Some(gpu);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => gpu.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                match gpu.render() {
                    RenderOutcome::Presented | RenderOutcome::Reconfigured => {
                        if let Some(w) = &self.window {
                            w.request_redraw();
                        }
                    }
                    RenderOutcome::Occluded => {
                        // Don't self-trigger — wait for the OS to tell us we're visible again
                        // via WindowEvent::Occluded(false) below.
                    }
                }
            }
            WindowEvent::Occluded(false) => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(state) =
                    gpu.keyboard
                        .on_key(event.physical_key, event.state, event.repeat)
                {
                    gpu.nes.set_buttons(0, state);
                }
            }
            WindowEvent::Focused(false) => {
                // Key-up events won't arrive while unfocused; release
                // everything so buttons don't stick across focus loss.
                if let Some(state) = gpu.keyboard.release_all() {
                    gpu.nes.set_buttons(0, state);
                }
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();
    log::info!("nies-app starting");

    let rom_bytes: Vec<u8> = match std::env::args().nth(1) {
        Some(path) => match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) => {
                log::error!("failed to read ROM '{path}': {e}; using embedded demo");
                nies_core::demo_rom_bytes().to_vec()
            }
        },
        None => nies_core::demo_rom_bytes().to_vec(),
    };

    let event_loop = EventLoop::new().expect("create event loop");
    let mut app = App::new(rom_bytes);
    event_loop.run_app(&mut app).expect("run event loop");
}
