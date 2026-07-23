//! Four-bladed spike: the `QuadHorn` draw variant, `PipelineKind::QuadHorn`.
//!
//! We emit four triangles that rise from the edges of a `size`-square base to a
//! common apex offset `height` along local Z. Positions are world space: each
//! triangle is transformed by a tilt about local X (`tilt_x_deg`) then a yaw
//! about local Y (`rotation_y_deg`) and translated to `base`. Vertex colour is
//! flat; U steps 0.2 per face across the four blades. The record sorts at
//! `base`.
//!
//! Blend is per-record alpha or additive, no depth write, compare `LessEqual`.
//! `QuadHornRenderer` implements `EffectPrimitiveRenderer` and is registered
//! under this kind. Emitted by `EffectSpec::Custom` effects such as Frost Diver,
//! Earth Spike and Gravitation.

use crate::camera::Camera;
use crate::effect::blend::ADDITIVE_BLEND;
use crate::effect::pipeline::{PipelineOpts, build_pipeline, effect_pipeline_layout};
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind, view_z};
use crate::effect::{EffectDrawList, EffectPrimitiveDraw};
use crate::sprite::SpriteVertex;

pub struct QuadHornRenderer {
    pub pipeline_alpha: wgpu::RenderPipeline,
    pub pipeline_additive: wgpu::RenderPipeline,
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
            label: Some("effect_quad_horn"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        let layout = effect_pipeline_layout(
            device,
            "effect_quad_horn",
            camera_bind_group_layout,
            texture_bind_group_layout,
        );
        let opts = |blend| PipelineOpts {
            label: "effect_quad_horn",
            blend,
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            depth_write: false,
            depth_compare: wgpu::CompareFunction::LessEqual,
        };
        let pipeline_alpha = build_pipeline(
            device,
            surface_format,
            &layout,
            &shader,
            &opts(wgpu::BlendState::ALPHA_BLENDING),
        );
        let pipeline_additive = build_pipeline(
            device,
            surface_format,
            &layout,
            &shader,
            &opts(ADDITIVE_BLEND),
        );
        (pipeline_alpha, pipeline_additive)
    }
}

pub fn prepare_quad_horn_records<'tex>(
    list: &EffectDrawList,
    camera: &Camera,
    fallback_texture: &'tex wgpu::BindGroup,
    texture_lookup: impl Fn(&str) -> Option<&'tex wgpu::BindGroup>,
) -> Vec<DrawRecord<'tex>> {
    let mut records: Vec<DrawRecord<'tex>> = Vec::new();
    for (emission, prim) in list.primitives.iter().enumerate() {
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

        let s = *size;
        let h = *height;
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

        let transform = |p: [f32; 3]| -> [f32; 3] {
            let (lx, ly, lz) = (p[0], p[1], p[2]);
            let x1 = lx;
            let y1 = ly * cos_t - lz * sin_t;
            let z1 = ly * sin_t + lz * cos_t;
            let x2 = x1 * cos_y + z1 * sin_y;
            let y2 = y1;
            let z2 = -x1 * sin_y + z1 * cos_y;
            [base[0] + x2, base[1] + y2, base[2] + z2]
        };

        let mut vertices: Vec<SpriteVertex> = Vec::with_capacity(12);
        let mut indices: Vec<u32> = Vec::with_capacity(12);

        let mut u_left = 0.0f32;
        for (face_idx, face) in locals.iter().enumerate() {
            let world = [transform(face[0]), transform(face[1]), transform(face[2])];
            let vert_base = (face_idx * 3) as u32;
            vertices.push(SpriteVertex {
                position: world[0],
                tex_coord: [u_left, 1.0],
                color: *color,
            });
            vertices.push(SpriteVertex {
                position: world[1],
                tex_coord: [u_left, 0.0],
                color: *color,
            });
            vertices.push(SpriteVertex {
                position: world[2],
                tex_coord: [u_left + 0.2, 1.0],
                color: *color,
            });
            indices.extend_from_slice(&[vert_base, vert_base + 1, vert_base + 2]);
            u_left += 0.2;
        }

        records.push(DrawRecord::new(
            view_z(camera, *base),
            emission as u32,
            BlendBucket::from_blend_kind(*blend),
            PipelineKind::QuadHorn,
            vertices,
            indices,
            texture_bg,
        ));
    }
    records
}
