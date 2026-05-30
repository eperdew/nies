//! `NesRenderer` — uploads the 256×240 palette-index framebuffer into an
//! R8Uint texture and draws it through the palette-LUT shader, integer
//! scaled and centered. Platform-agnostic: owns GPU resources but not the
//! surface, device, queue, or window — the binary passes those in.

use crate::palette::fbx_smooth;
use crate::scaling::{NES_H, NES_W};
use wgpu::util::DeviceExt;

pub struct NesRenderer {
    index_tex: wgpu::Texture,
    // Used in render(); suppressed until Task 10 adds that method.
    #[allow(dead_code)]
    pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    bind_group: wgpu::BindGroup,
}

impl NesRenderer {
    pub fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        let index_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nes-index-tex"),
            size: wgpu::Extent3d {
                width: NES_W,
                height: NES_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let index_view = index_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let pal = fbx_smooth();
        let mut lut = [0f32; 64 * 4];
        for (i, [r, g, b]) in pal.iter().enumerate() {
            lut[i * 4] = *r as f32 / 255.0;
            lut[i * 4 + 1] = *g as f32 / 255.0;
            lut[i * 4 + 2] = *b as f32 / 255.0;
            lut[i * 4 + 3] = 1.0;
        }
        let lut_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("nes-palette-lut"),
            contents: bytemuck::cast_slice(&lut),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nes-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nes-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&index_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: lut_buf.as_entire_binding(),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nes-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("nes.wgsl").into()),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nes-pl"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nes-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            index_tex,
            pipeline,
            bind_group,
        }
    }

    /// Upload one 256×240 palette-index frame into the GPU texture.
    pub fn upload_frame(&self, queue: &wgpu::Queue, frame: &[u8; 256 * 240]) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.index_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            frame,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(NES_W),
                rows_per_image: Some(NES_H),
            },
            wgpu::Extent3d {
                width: NES_W,
                height: NES_H,
                depth_or_array_layers: 1,
            },
        );
    }
}
