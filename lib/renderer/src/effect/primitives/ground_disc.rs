//! GroundDisc primitive — flat-on-ground textured annulus or partial-arc
//! wedge rendered in world space.

use crate::camera::Camera;
use crate::device::DEPTH_FORMAT;
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind, view_z};
use crate::effect::{EffectDrawList, EffectPrimitiveDraw};
use crate::sprite::SpriteVertex;

const FULL_DISC_SEGMENTS: u32 = 32;

pub struct GroundDiscRenderer {
    pub pipeline_alpha: wgpu::RenderPipeline,
    pub pipeline_additive: wgpu::RenderPipeline,
}

impl GroundDiscRenderer {
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
            include_str!("../../shaders/effect_ground_disc.wgsl"),
        );
        Self {
            pipeline_alpha,
            pipeline_additive,
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
            label: Some("effect_ground_disc"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_ground_disc"),
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
            label: Some("effect_ground_disc"),
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[SpriteVertex::LAYOUT],
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
}

/// Build one [`DrawRecord`] per `EffectPrimitiveDraw::GroundDisc` entry.
pub fn prepare_ground_disc_records<'tex>(
    list: &EffectDrawList,
    camera: &Camera,
    fallback_texture: &'tex wgpu::BindGroup,
    texture_lookup: impl Fn(&str) -> Option<&'tex wgpu::BindGroup>,
) -> Vec<DrawRecord<'tex>> {
    let mut records: Vec<DrawRecord<'tex>> = Vec::new();
    for (emission, prim) in list.primitives.iter().enumerate() {
        let EffectPrimitiveDraw::GroundDisc {
            center,
            radius,
            thickness,
            rotation,
            arc_angle_deg,
            uv_repeat,
            texture,
            color,
            blend,
        } = prim
        else {
            continue;
        };

        let outer = *radius;
        let inner = (outer - *thickness).max(0.0);
        let full_span_rad = std::f32::consts::TAU;
        let arc_span_rad = arc_angle_deg.to_radians().clamp(0.0, full_span_rad);
        if arc_span_rad <= 0.0 || outer <= 0.0 {
            continue;
        }
        let segments = ((arc_span_rad / full_span_rad) * FULL_DISC_SEGMENTS as f32)
            .ceil()
            .max(3.0) as u32;
        let uv_repeat_value = *uv_repeat;
        let u_offset = rotation / std::f32::consts::TAU;
        let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);

        let mut vertices: Vec<SpriteVertex> = Vec::with_capacity(((segments + 1) * 2) as usize);
        let mut indices: Vec<u32> = Vec::with_capacity((segments * 6) as usize);

        for s in 0..=segments {
            let t = s as f32 / segments as f32;
            let angle = t * arc_span_rad;
            let (sin_a, cos_a) = angle.sin_cos();
            let outer_lx = outer * cos_a;
            let outer_lz = outer * sin_a;
            let inner_lx = inner * cos_a;
            let inner_lz = inner * sin_a;
            let u = t * (arc_span_rad / full_span_rad) * uv_repeat_value + u_offset;
            vertices.push(SpriteVertex {
                position: [center[0] + outer_lx, center[1], center[2] + outer_lz],
                tex_coord: [u, 0.0],
                color: *color,
            });
            vertices.push(SpriteVertex {
                position: [center[0] + inner_lx, center[1], center[2] + inner_lz],
                tex_coord: [u, 1.0],
                color: *color,
            });
        }

        for s in 0..segments {
            let o0 = 2 * s;
            let i0 = o0 + 1;
            let o1 = 2 * (s + 1);
            let i1 = o1 + 1;
            indices.extend_from_slice(&[o0, i0, o1, i0, i1, o1]);
        }

        records.push(DrawRecord::new(
            view_z(camera, *center),
            emission as u32,
            BlendBucket::from_blend_kind(*blend),
            PipelineKind::GroundDisc,
            vertices,
            indices,
            texture_bg,
        ));
    }
    records
}
