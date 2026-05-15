//! QuadHorn primitive — square-based pyramid spike (icicle, ice shard,
//! stalagmite). Matches the original game's quad-horn behavior from
//! `Render3DQuadHorn` exactly. Used by Stormgust's ice shards and eight
//! other effects in the original game.
//!
//! Local frame matches original game's `Render3DQuadHorn`
//!   * base square on the local XY plane, corners at `(±size, ±size, 0)`;
//!   * apex at `(0, 0, height)` along local +Z.
//!
//! Rotations match the original game's X and Y axis rotations
//! under row-vector multiplication, so callers
//! can pass latitude/longitude literals directly:
//!   * `tilt_x_deg` = 0° → apex along world +Z (horizontal)
//!   * `tilt_x_deg` = 90° → apex along world -Y (straight UP — native RO)
//!   * `tilt_x_deg` = 100° → near-vertical, slight backward lean (original game
//!     Stormgust's latitude 100)
//!   * `tilt_x_deg` = 270° → apex along world +Y (straight DOWN)
//!   * `rotation_y_deg` rotates the tilted spike around the world up-axis
//!     to face a compass heading.
//!
//! UV layout per original game: each triangle is a vertical strip; base verts at
//! `v=1`, apex at `v=0`; `u` advances by 0.2 per triangle (so the four
//! strips cover `[0.0..0.8]` — the `[0.8..1.0]` strip of the texture is
//! intentionally unused).
//!
//! The vertex layout and shader are identical to Frustum's so we re-use
//! `effect_frustum.wgsl` instead of duplicating it.

use crate::camera::Camera;
use crate::device::DEPTH_FORMAT;
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct QuadHornVertex {
    position: [f32; 3],
    tex_coord: [f32; 2],
    color: [f32; 4],
}

impl QuadHornVertex {
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

const INITIAL_VERTEX_CAPACITY: usize = 256;
const INITIAL_INDEX_CAPACITY: usize = 256;

pub struct QuadHornRenderer {
    pipeline_alpha: wgpu::RenderPipeline,
    pipeline_additive: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    index_capacity: usize,
}

impl QuadHornRenderer {
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
            label: Some("quad_horn_vertices"),
            size: (INITIAL_VERTEX_CAPACITY * std::mem::size_of::<QuadHornVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad_horn_indices"),
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
            label: Some("effect_quad_horn"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_quad_horn"),
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
            label: Some("effect_quad_horn"),
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[QuadHornVertex::LAYOUT],
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
        let mut verts: Vec<QuadHornVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        struct DrawSpan<'a> {
            texture: &'a wgpu::BindGroup,
            additive: bool,
            index_start: u32,
            index_count: u32,
        }
        let mut spans: Vec<DrawSpan<'_>> = Vec::new();

        for prim in &list.primitives {
            let EffectPrimitiveDraw::QuadHorn {
                base,
                size,
                height,
                tilt_x_deg,
                rotation_y_deg,
                texture,
                color,
                blend,
            } = prim
            else {
                continue;
            };

            if *size <= 0.0 || height.abs() <= 0.0 {
                continue;
            }

            let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);
            let additive = blend_is_additive(blend);
            let index_start = indices.len() as u32;
            let vert_base = verts.len() as u32;

            // Local frame matches original game `Render3DQuadHorn` exactly: base
            // square on the local XY plane (z=0), apex at (0, 0, height)
            // along local +Z. 
            // world output is just `base + rotated_local`
            // (both original game and our codebase use native RO `-Y = up`, so the
            // local Y maps directly to world Y).
            //
            // Caller convention for `tilt_x_deg` therefore matches original game's
            // latitude literally:
            //   0°   → apex points along world +Z (horizontal)
            //   90°  → apex points along world -Y (straight UP)
            //   100° → mostly up, slight backward lean (original game stormgust)
            //   180° → apex along world -Z (horizontal, opposite)
            //   270° → apex along world +Y (straight DOWN)
            let s = *size;
            let h = *height;
            // (base_a, apex, base_b) per triangle. Winding order matches
            // original game Render3DQuadHorn for consistent face orientation.
            let locals: [[[f32; 3]; 3]; 4] = [
                [[-s, -s, 0.0], [0.0, 0.0, h], [s, -s, 0.0]],
                [[-s, -s, 0.0], [0.0, 0.0, h], [-s, s, 0.0]],
                [[-s, s, 0.0], [0.0, 0.0, h], [s, s, 0.0]],
                [[s, s, 0.0], [0.0, 0.0, h], [s, -s, 0.0]],
            ];

            let tilt = tilt_x_deg.to_radians();
            let yaw = rotation_y_deg.to_radians();
            let (sin_t, cos_t) = tilt.sin_cos();
            let (sin_y, cos_y) = yaw.sin_cos();

            // X-rotation by `tilt_x_deg`, then Y-rotation by
            // `rotation_y_deg`, both using original game's row-vector matrix layout
            let transform = |p: [f32; 3]| -> [f32; 3] {
                let (lx, ly, lz) = (p[0], p[1], p[2]);
                // X-rotation: rotates the (y, z) pair.
                let x1 = lx;
                let y1 = ly * cos_t - lz * sin_t;
                let z1 = ly * sin_t + lz * cos_t;
                // Y-rotation: rotates the (x, z) pair.
                let x2 = x1 * cos_y + z1 * sin_y;
                let y2 = y1;
                let z2 = -x1 * sin_y + z1 * cos_y;
                [base[0] + x2, base[1] + y2, base[2] + z2]
            };

            let mut u_left = 0.0f32;
            for face in &locals {
                let world = [transform(face[0]), transform(face[1]), transform(face[2])];
                verts.push(QuadHornVertex {
                    position: world[0],
                    tex_coord: [u_left, 1.0],
                    color: *color,
                });
                verts.push(QuadHornVertex {
                    position: world[1],
                    tex_coord: [u_left, 0.0],
                    color: *color,
                });
                verts.push(QuadHornVertex {
                    position: world[2],
                    tex_coord: [u_left + 0.2, 1.0],
                    color: *color,
                });
                u_left += 0.2;
            }

            for face in 0..4u32 {
                let v0 = vert_base + face * 3;
                indices.extend_from_slice(&[v0, v0 + 1, v0 + 2]);
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
                label: Some("quad_horn_vertices"),
                size: (self.vertex_capacity * std::mem::size_of::<QuadHornVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if indices.len() > self.index_capacity {
            self.index_capacity = indices.len().next_power_of_two();
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("quad_horn_indices"),
                size: (self.index_capacity * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("effect_quad_horn"),
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
