//! Frustum primitive — vertical "tube" between two coaxial rings.
//!
//! Used by effects whose silhouette is a vertical band of texture: Magnum
//! Break's explosion cone, Sanctuary / Magnus pillars, Bottom-Sanc rotating
//! pillar. Geometry is a closed triangle strip from a bottom polygon
//! (`bottom_size` radius) up to a top polygon (`top_size` radius). When the
//! two radii are equal it's a cylinder; when `top_size == 0` it's a cone.
//! `sides == 4` gives a square pillar (Bottom-Sanc), high `sides` approximate
//! a smooth circular tube.
//!
//! The texture wraps once around the lateral surface unless `rotation` shifts
//! the seam; `v = 0` at the bottom, `v = 1` at the top.

use crate::camera::Camera;
use crate::device::DEPTH_FORMAT;
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FrustumVertex {
    position: [f32; 3],
    tex_coord: [f32; 2],
    color: [f32; 4],
}

impl FrustumVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Float32x4,
    ];

    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &Self::ATTRIBS,
    };
}

const INITIAL_VERTEX_CAPACITY: usize = 512;
const INITIAL_INDEX_CAPACITY: usize = 1024;

pub struct FrustumRenderer {
    pipeline_alpha: wgpu::RenderPipeline,
    pipeline_additive: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    index_capacity: usize,
}

impl FrustumRenderer {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let (pipeline_alpha, pipeline_additive) = Self::build_pipelines(
            device,
            surface_format,
            camera_bind_group_layout,
            texture_bind_group_layout,
            include_str!("../../shaders/effect_frustum.wgsl"),
        );

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frustum_vertices"),
            size: (INITIAL_VERTEX_CAPACITY * std::mem::size_of::<FrustumVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frustum_indices"),
            size: (INITIAL_INDEX_CAPACITY * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline_alpha,
            pipeline_additive,
            vertex_buffer,
            index_buffer,
            vertex_capacity: INITIAL_VERTEX_CAPACITY,
            index_capacity: INITIAL_INDEX_CAPACITY,
        }
    }

    /// Rebuild both pipelines from a runtime-supplied WGSL source. Used by
    /// the effect viewer's hot-reload path; production code calls `new()`
    /// once with the `include_str!`'d source.
    pub fn recreate_pipelines(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        shader_source: &str,
    ) {
        let (alpha, additive) = Self::build_pipelines(
            device,
            surface_format,
            camera_bind_group_layout,
            texture_bind_group_layout,
            shader_source,
        );
        self.pipeline_alpha = alpha;
        self.pipeline_additive = additive;
    }

    fn build_pipelines(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        shader_source: &str,
    ) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("effect_frustum"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_frustum"),
            bind_group_layouts: &[camera_bind_group_layout, texture_bind_group_layout],
            immediate_size: 0,
        });

        let alpha = wgpu::BlendState::ALPHA_BLENDING;
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let pipeline_alpha =
            Self::create_pipeline(device, surface_format, &pipeline_layout, &shader, alpha);
        let pipeline_additive =
            Self::create_pipeline(device, surface_format, &pipeline_layout, &shader, additive);
        (pipeline_alpha, pipeline_additive)
    }

    fn create_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        pipeline_layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        blend: wgpu::BlendState,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("effect_frustum"),
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[FrustumVertex::LAYOUT],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    pub fn render<'a>(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_bind_group: &wgpu::BindGroup,
        _camera: &Camera,
        list: &EffectDrawList,
        fallback_texture: &'a wgpu::BindGroup,
        texture_lookup: impl Fn(&str) -> Option<&'a wgpu::BindGroup>,
    ) {
        let mut verts: Vec<FrustumVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        struct DrawSpan<'a> {
            texture: &'a wgpu::BindGroup,
            additive: bool,
            index_start: u32,
            index_count: u32,
        }
        let mut spans: Vec<DrawSpan<'_>> = Vec::new();

        for prim in &list.primitives {
            let EffectPrimitiveDraw::Frustum {
                base,
                bottom_size,
                top_size,
                height,
                sides,
                rotation,
                uv_repeat,
                uv_scroll,
                wave_amplitude,
                wave_frequency,
                wave_phase,
                texture,
                color,
                blend,
            } = prim
            else {
                continue;
            };

            let sides = (*sides).max(3);
            if *bottom_size <= 0.0 && *top_size <= 0.0 {
                continue;
            }
            if height.abs() <= 0.0 {
                continue;
            }

            let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);
            let additive = blend_is_additive(blend);
            let index_start = indices.len() as u32;
            let vert_base = verts.len() as u32;

            let bottom_y = base[1];
            // Native RO coordinates: -Y is up.
            let top_y = base[1] - *height;
            let full_span = std::f32::consts::TAU;
            // `rotation` rotates the geometry around the vertical axis. The
            // texture's u-coord is keyed to the segment index (`t`), not to
            // world angle, so the texture pattern travels with the rotating
            // geometry — the four flame stripes in `ring_white.tga`
            // (LandProtector) and the four faces of a square pillar
            // (BottomSanc) rotate as a whole instead of staying pinned to
            // world cardinals while the mesh spins beneath them.
            let geom_rotation = *rotation;
            let uv_rep = *uv_repeat;
            let scroll_v = uv_scroll[1];

            // Per-segment wave displaces the top vertex along the cone's
            // tilt direction (the unit vector from bottom-rim to top-rim).
            // Matches the original game's `Rx = cos(rise) * height`,
            // `Ry = sin(rise) * height` — taller wave peaks reach further
            // outward as well as further up. For a cylinder (`top == bottom`)
            // the radial component is zero so the wave only affects Y, which
            // is what BottomSanc wants.
            let delta_r = *top_size - *bottom_size;
            let tilt_len = (delta_r * delta_r + height * height).sqrt();
            let (radial_unit, vert_unit) = if tilt_len > 0.0 {
                (delta_r / tilt_len, height / tilt_len)
            } else {
                (0.0, 1.0)
            };

            for s in 0..=sides {
                let t = s as f32 / sides as f32;
                let local_angle = t * full_span;
                let world_angle = local_angle + geom_rotation;
                let (sin_a, cos_a) = world_angle.sin_cos();
                let u = t * uv_rep + uv_scroll[0];

                // Wave uses LOCAL angle so peaks stay locked to the cone's
                // surface and visibly travel with the rotation. If we used
                // `world_angle` here the wave's argument would advance at
                // `wave_frequency × rotation_rate` per frame, making peaks
                // flicker past every vertex many times per revolution.
                let wave = *wave_amplitude
                    * (local_angle * *wave_frequency + *wave_phase).sin();
                let seg_top_size = top_size + wave * radial_unit;
                let seg_top_y = top_y - wave * vert_unit; // -Y is up

                verts.push(FrustumVertex {
                    position: [
                        base[0] + bottom_size * cos_a,
                        bottom_y,
                        base[2] + bottom_size * sin_a,
                    ],
                    tex_coord: [u, 1.0 + scroll_v],
                    color: *color,
                });
                verts.push(FrustumVertex {
                    position: [
                        base[0] + seg_top_size * cos_a,
                        seg_top_y,
                        base[2] + seg_top_size * sin_a,
                    ],
                    tex_coord: [u, 0.0 + scroll_v],
                    color: *color,
                });
            }

            for s in 0..sides {
                let b0 = vert_base + 2 * s;
                let t0 = b0 + 1;
                let b1 = vert_base + 2 * (s + 1);
                let t1 = b1 + 1;
                indices.extend_from_slice(&[b0, t0, b1, t0, t1, b1]);
            }

            let index_count = indices.len() as u32 - index_start;
            spans.push(DrawSpan {
                texture: texture_bg,
                additive,
                index_start,
                index_count,
            });
        }

        if verts.is_empty() {
            return;
        }

        if verts.len() > self.vertex_capacity {
            self.vertex_capacity = verts.len().next_power_of_two();
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("frustum_vertices"),
                size: (self.vertex_capacity * std::mem::size_of::<FrustumVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if indices.len() > self.index_capacity {
            self.index_capacity = indices.len().next_power_of_two();
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("frustum_indices"),
                size: (self.index_capacity * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("effect_frustum"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        pass.set_bind_group(0, camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        for span in spans {
            let pipeline = if span.additive {
                &self.pipeline_additive
            } else {
                &self.pipeline_alpha
            };
            pass.set_pipeline(pipeline);
            pass.set_bind_group(1, span.texture, &[]);
            pass.draw_indexed(span.index_start..span.index_start + span.index_count, 0, 0..1);
        }
    }
}

fn blend_is_additive(blend: &BlendKind) -> bool {
    match blend {
        BlendKind::Additive => true,
        BlendKind::Alpha | BlendKind::Multiply => false,
        BlendKind::Raw { src: _, dst } => *dst != 6,
    }
}
