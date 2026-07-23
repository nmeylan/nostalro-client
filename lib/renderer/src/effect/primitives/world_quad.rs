//! Free world quad: the `WorldQuad` draw variant, `PipelineKind::WorldQuad`.
//!
//! We emit one textured quad from four world-space `corners` and their UVs
//! supplied directly by the effect, which has already positioned and oriented
//! them. This is the most general and most-used quad primitive; the effect owns
//! the geometry, the renderer only assembles vertices. The record sorts at the
//! corner centroid.
//!
//! Blend is per-record alpha or additive; `no_depth` promotes the bucket to the
//! `*NoDepth` variant (an `Always`-compare pipeline). No pipeline writes depth.
//! `WorldQuadRenderer` implements `EffectPrimitiveRenderer` and is registered
//! under this kind. Emitted by `EffectSpec::Custom` effects such as Basilica and
//! Bash.

use crate::camera::Camera;
use crate::effect::blend::ADDITIVE_BLEND;
use crate::effect::pipeline::{PipelineOpts, build_pipeline, effect_pipeline_layout};
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind, view_z};
use crate::effect::{EffectDrawList, EffectPrimitiveDraw};
use crate::sprite::SpriteVertex;

pub struct WorldQuadRenderer {
    pub pipeline_alpha: wgpu::RenderPipeline,
    pub pipeline_additive: wgpu::RenderPipeline,
    pub pipeline_alpha_no_depth: wgpu::RenderPipeline,
    pub pipeline_additive_no_depth: wgpu::RenderPipeline,
}

impl WorldQuadRenderer {
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
            wgpu::CompareFunction::LessEqual,
        );
        let (pipeline_alpha_no_depth, pipeline_additive_no_depth) = Self::build_pipelines(
            device,
            surface_format,
            camera_bind_group_layout,
            texture_bind_group_layout,
            include_str!("../../shaders/effect_ground_disc.wgsl"),
            wgpu::CompareFunction::Always,
        );
        Self {
            pipeline_alpha,
            pipeline_additive,
            pipeline_alpha_no_depth,
            pipeline_additive_no_depth,
        }
    }

    fn build_pipelines(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        shader_source: &str,
        depth_compare: wgpu::CompareFunction,
    ) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("effect_world_quad"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        let layout = effect_pipeline_layout(
            device,
            "effect_world_quad",
            camera_bind_group_layout,
            texture_bind_group_layout,
        );
        let opts = |blend| PipelineOpts {
            label: "effect_world_quad",
            blend,
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            depth_write: false,
            depth_compare,
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

pub fn prepare_world_quad_records<'tex>(
    list: &EffectDrawList,
    camera: &Camera,
    fallback_texture: &'tex wgpu::BindGroup,
    texture_lookup: impl Fn(&str) -> Option<&'tex wgpu::BindGroup>,
) -> Vec<DrawRecord<'tex>> {
    let mut records: Vec<DrawRecord<'tex>> = Vec::new();
    for (emission, prim) in list.primitives.iter().enumerate() {
        let EffectPrimitiveDraw::WorldQuad {
            corners,
            uv,
            texture,
            color,
            blend,
            no_depth,
        } = prim
        else {
            continue;
        };

        let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);

        let mut vertices: Vec<SpriteVertex> = Vec::with_capacity(4);
        for i in 0..4 {
            vertices.push(SpriteVertex {
                position: corners[i],
                tex_coord: uv[i],
                color: *color,
            });
        }
        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        let centroid = [
            (corners[0][0] + corners[1][0] + corners[2][0] + corners[3][0]) * 0.25,
            (corners[0][1] + corners[1][1] + corners[2][1] + corners[3][1]) * 0.25,
            (corners[0][2] + corners[1][2] + corners[2][2] + corners[3][2]) * 0.25,
        ];

        let bucket = match (BlendBucket::from_blend_kind(*blend), *no_depth) {
            (BlendBucket::Alpha, true) => BlendBucket::AlphaNoDepth,
            (BlendBucket::Additive, true) => BlendBucket::AdditiveNoDepth,
            (bucket, _) => bucket,
        };

        records.push(DrawRecord::new(
            view_z(camera, centroid),
            emission as u32,
            bucket,
            PipelineKind::WorldQuad,
            vertices,
            indices,
            texture_bg,
        ));
    }
    records
}
