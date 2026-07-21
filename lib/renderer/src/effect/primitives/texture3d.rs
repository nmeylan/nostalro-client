use crate::camera::Camera;
use crate::effect::blend::ADDITIVE_BLEND;
use crate::effect::pipeline::{PipelineOpts, build_pipeline, effect_pipeline_layout};
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind, view_z};
use crate::effect::{EffectDrawList, EffectPrimitiveDraw};
use crate::sprite::SpriteVertex;
use ragnarok_game::effect::draw::QuadPlane;

pub struct Texture3DRenderer {
    pub pipeline_alpha: wgpu::RenderPipeline,
    pub pipeline_additive: wgpu::RenderPipeline,
}

impl Texture3DRenderer {
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

    fn build_pipelines(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        shader_source: &str,
    ) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("effect_texture3d"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        let layout = effect_pipeline_layout(
            device,
            "effect_texture3d",
            camera_bind_group_layout,
            texture_bind_group_layout,
        );
        let opts = |blend| PipelineOpts {
            label: "effect_texture3d",
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
        let pipeline_additive =
            build_pipeline(device, surface_format, &layout, &shader, &opts(ADDITIVE_BLEND));
        (pipeline_alpha, pipeline_additive)
    }
}

pub fn prepare_texture3d_records<'tex>(
    list: &EffectDrawList,
    camera: &Camera,
    fallback_texture: &'tex wgpu::BindGroup,
    texture_lookup: impl Fn(&str) -> Option<&'tex wgpu::BindGroup>,
) -> Vec<DrawRecord<'tex>> {
    let mut records: Vec<DrawRecord<'tex>> = Vec::new();
    for (emission, prim) in list.primitives.iter().enumerate() {
        let EffectPrimitiveDraw::Texture3D {
            center,
            size,
            plane,
            uv,
            texture,
            color,
            blend,
        } = prim
        else {
            continue;
        };

        let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);
        let corners = QuadPlane::corners(*plane, *center, *size);

        let mut vertices: Vec<SpriteVertex> = Vec::with_capacity(4);
        for i in 0..4 {
            vertices.push(SpriteVertex {
                position: corners[i],
                tex_coord: uv[i],
                color: *color,
            });
        }
        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        records.push(DrawRecord::new(
            view_z(camera, *center),
            emission as u32,
            BlendBucket::from_blend_kind(*blend),
            PipelineKind::Texture3D,
            vertices,
            indices,
            texture_bg,
        ));
    }
    records
}
