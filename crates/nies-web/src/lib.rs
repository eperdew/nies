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
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Self {
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
        // WebGL2 enforces hard limits (max texture size, max bind groups, etc.)
        // well below desktop wgpu defaults. Downlevel here so the same render
        // code works under either the webgpu or webgl backend; the native
        // binary uses DeviceDescriptor::default() because Metal/Vulkan/DX12
        // have no equivalent constraint.
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
                multiview_mask: None,
            });
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        RenderOutcome::Presented
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
        // winit may emit Resumed multiple times in suspend/resume scenarios
        // (mobile / PWA). Re-bootstrapping would create a second Window and
        // a second GpuReady future; guard against it.
        if self.window.is_some() {
            return;
        }

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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => gpu.resize(size.width, size.height),
            WindowEvent::RedrawRequested => match gpu.render() {
                RenderOutcome::Presented | RenderOutcome::Reconfigured => {
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
                RenderOutcome::Occluded => {
                    // Don't self-trigger — wait for the OS to tell us we're visible again
                    // via WindowEvent::Occluded(false) below.
                }
            },
            WindowEvent::Occluded(false) => {
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
