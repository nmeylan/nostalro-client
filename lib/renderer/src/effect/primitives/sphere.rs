//! Sphere primitive — closed UV sphere mesh (or a partial spherical band).
//!
//! Latitude sweeps `-90°..+90°` over `sides_lat` segments, longitude sweeps
//! `0°..360°` over `sides_lon`, two triangles per cell. Matches original
//! game's `Render3DSphere`. `longitude_offset` shifts the longitude angle
//! used for both geometry and UVs — drive it over time to reproduce the
//! texture rotation speed.

use crate::camera::Camera;
use crate::device::DEPTH_FORMAT;
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SphereVertex {
    position: [f32; 3],
    tex_coord: [f32; 2],
    color: [f32; 4],
}

impl SphereVertex {
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

pub struct SphereRenderer {
    pipeline_alpha: wgpu::RenderPipeline,
    pipeline_additive: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    index_capacity: usize,
}

impl SphereRenderer {
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
            include_str!("../../shaders/effect_sphere.wgsl"),
        );

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sphere_vertices"),
            size: (INITIAL_VERTEX_CAPACITY * std::mem::size_of::<SphereVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sphere_indices"),
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
            label: Some("effect_sphere"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_sphere"),
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
            label: Some("effect_sphere"),
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[SphereVertex::LAYOUT],
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
        let mut verts: Vec<SphereVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        struct DrawSpan<'a> {
            texture: &'a wgpu::BindGroup,
            additive: bool,
            index_start: u32,
            index_count: u32,
        }
        let mut spans: Vec<DrawSpan<'_>> = Vec::new();

        for prim in &list.primitives {
            let EffectPrimitiveDraw::Sphere {
                center,
                radius,
                sides_lat,
                sides_lon,
                longitude_offset,
                uv_repeat,
                texture,
                color,
                blend,
            } = prim
            else {
                continue;
            };

            if *radius <= 0.0 {
                continue;
            }
            let lat_segs = (*sides_lat).max(2);
            let lon_segs = (*sides_lon).max(3);

            let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);
            let additive = blend_is_additive(blend);
            let index_start = indices.len() as u32;
            let vert_base = verts.len() as u32;

            // Build a (lat_segs+1) × (lon_segs+1) vertex grid. Latitude phi
            // sweeps -pi/2..pi/2 (south pole to north pole); longitude theta
            // sweeps 0..2*pi. Native RO coords: -Y is up, so the north pole
            // sits at center - radius * up.
            let lat_count = lat_segs + 1;
            let lon_count = lon_segs + 1;
            for lat in 0..lat_count {
                let v = lat as f32 / lat_segs as f32;
                let phi =
                    -std::f32::consts::FRAC_PI_2 + v * std::f32::consts::PI;
                let (sin_phi, cos_phi) = phi.sin_cos();
                let tv = v * uv_repeat[1];
                for lon in 0..lon_count {
                    let u = lon as f32 / lon_segs as f32;
                    let theta = u * std::f32::consts::TAU + *longitude_offset;
                    let (sin_theta, cos_theta) = theta.sin_cos();
                    // Position: ring at latitude phi has horizontal radius
                    // cos(phi)*r, vertical position sin(phi)*r along -Y.
                    let px = center[0] + radius * cos_phi * cos_theta;
                    let py = center[1] - radius * sin_phi;
                    let pz = center[2] + radius * cos_phi * sin_theta;
                    let tu = u * uv_repeat[0];
                    verts.push(SphereVertex {
                        position: [px, py, pz],
                        tex_coord: [tu, tv],
                        color: *color,
                    });
                }
            }

            // Two triangles per (lat, lon) cell.
            for lat in 0..lat_segs {
                for lon in 0..lon_segs {
                    let row0 = lat * lon_count;
                    let row1 = (lat + 1) * lon_count;
                    let a = vert_base + row0 + lon;
                    let b = vert_base + row0 + lon + 1;
                    let c = vert_base + row1 + lon;
                    let d = vert_base + row1 + lon + 1;
                    indices.extend_from_slice(&[a, c, b, b, c, d]);
                }
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
                label: Some("sphere_vertices"),
                size: (self.vertex_capacity * std::mem::size_of::<SphereVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if indices.len() > self.index_capacity {
            self.index_capacity = indices.len().next_power_of_two();
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sphere_indices"),
                size: (self.index_capacity * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("effect_sphere"),
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
