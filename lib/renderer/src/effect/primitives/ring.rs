//! Ring primitive — flat-on-ground textured annulus rendered in world space.
//!
//! Replaces the Billboard fallback for the GroundRing family (Warp, Magnum
//! Break, Sanctuary base, Pneuma, Land Protector, …). Geometry is built on
//! the CPU each frame as a triangle strip wrapped around the y-axis; vertices
//! are projected via the global camera uniform (group 0).

use crate::camera::Camera;
use crate::device::DEPTH_FORMAT;
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw};

/// Vertex layout for the ring pipeline — world-space position + UV + tint.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RingVertex {
    position: [f32; 3],
    tex_coord: [f32; 2],
    color: [f32; 4],
}

impl RingVertex {
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
const DEFAULT_SEGMENTS: u32 = 32;

pub struct RingRenderer {
    pipeline_alpha: wgpu::RenderPipeline,
    pipeline_additive: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    index_capacity: usize,
}

impl RingRenderer {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("effect_ring"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/effect_ring.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_ring"),
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

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ring_vertices"),
            size: (INITIAL_VERTEX_CAPACITY * std::mem::size_of::<RingVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ring_indices"),
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

    fn create_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        pipeline_layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        blend: wgpu::BlendState,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("effect_ring"),
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[RingVertex::LAYOUT],
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

    /// Emit GPU draws for every `EffectPrimitiveDraw::Ring` entry in `list`.
    ///
    /// `camera_bind_group` must match the layout passed at construction.
    /// `texture_lookup` resolves the per-ring `&'static str` texture name
    /// against the caller's texture cache; falling back to
    /// `fallback_texture` when missing. `_camera` is reserved for billboard-style
    /// orientation features in later slices; current rings are flat on XZ.
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
        // Build CPU mesh: per-ring, push a triangle list spanning `segments`
        // wedges of the annulus. Group consecutive rings sharing texture+blend
        // into the same draw call.
        let mut verts: Vec<RingVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        struct DrawSpan<'a> {
            texture: &'a wgpu::BindGroup,
            additive: bool,
            index_start: u32,
            index_count: u32,
        }
        let mut spans: Vec<DrawSpan<'_>> = Vec::new();

        for prim in &list.primitives {
            let EffectPrimitiveDraw::Ring {
                center,
                radius,
                thickness,
                rotation,
                texture,
                color,
                blend,
            } = prim
            else {
                continue;
            };

            // Filled disc when thickness >= radius; otherwise inner radius is
            // max(0, radius - thickness). Tilt 0; rings are flat on XZ.
            let outer = *radius;
            let inner = (outer - *thickness).max(0.0);
            let segments = DEFAULT_SEGMENTS;
            // Planar UV: the texture maps once onto the disc footprint and
            // rotates around the center as `rotation` advances. Per-vertex
            // local (lx, lz) ∈ [-radius, +radius] is rotated then normalised
            // into [0, 1].
            let (sin_r, cos_r) = rotation.sin_cos();
            let planar_uv = |lx: f32, lz: f32| -> [f32; 2] {
                let rx = lx * cos_r - lz * sin_r;
                let rz = lx * sin_r + lz * cos_r;
                [(rx / (2.0 * outer)) + 0.5, (rz / (2.0 * outer)) + 0.5]
            };

            let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);
            let additive = blend_is_additive(blend);
            let index_start = indices.len() as u32;
            let vert_base = verts.len() as u32;

            for s in 0..=segments {
                let t = s as f32 / segments as f32;
                let angle = t * std::f32::consts::TAU;
                let (sin_a, cos_a) = angle.sin_cos();
                let outer_lx = outer * cos_a;
                let outer_lz = outer * sin_a;
                let inner_lx = inner * cos_a;
                let inner_lz = inner * sin_a;
                verts.push(RingVertex {
                    position: [center[0] + outer_lx, center[1], center[2] + outer_lz],
                    tex_coord: planar_uv(outer_lx, outer_lz),
                    color: *color,
                });
                verts.push(RingVertex {
                    position: [center[0] + inner_lx, center[1], center[2] + inner_lz],
                    tex_coord: planar_uv(inner_lx, inner_lz),
                    color: *color,
                });
            }

            // Indices: per segment, two triangles forming a quad between
            // segment `s` and `s+1`. Vertex layout per segment in `verts`:
            //   2*s+0 → outer at s
            //   2*s+1 → inner at s
            for s in 0..segments {
                let o0 = vert_base + 2 * s;
                let i0 = o0 + 1;
                let o1 = vert_base + 2 * (s + 1);
                let i1 = o1 + 1;
                indices.extend_from_slice(&[o0, i0, o1, i0, i1, o1]);
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
                label: Some("ring_vertices"),
                size: (self.vertex_capacity * std::mem::size_of::<RingVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if indices.len() > self.index_capacity {
            self.index_capacity = indices.len().next_power_of_two();
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ring_indices"),
                size: (self.index_capacity * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("effect_ring"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_classification() {
        assert!(blend_is_additive(&BlendKind::Additive));
        assert!(!blend_is_additive(&BlendKind::Alpha));
        assert!(!blend_is_additive(&BlendKind::Multiply));
        assert!(blend_is_additive(&BlendKind::Raw { src: 5, dst: 2 }));
        assert!(!blend_is_additive(&BlendKind::Raw { src: 5, dst: 6 }));
    }
}
