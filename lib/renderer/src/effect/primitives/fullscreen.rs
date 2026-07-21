use crate::effect::blend::ADDITIVE_BLEND;
use crate::effect::pipeline::{PipelineOpts, build_pipeline, effect_pipeline_layout};
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind};
use crate::effect::{EffectDrawList, EffectPrimitiveDraw};
use crate::sprite::SpriteVertex;

pub struct FullscreenOverlayRenderer {
    pub pipeline_alpha: wgpu::RenderPipeline,
    pub pipeline_additive: wgpu::RenderPipeline,
}

impl FullscreenOverlayRenderer {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("effect_fullscreen"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/effect_fullscreen.wgsl").into(),
            ),
        });
        let layout = effect_pipeline_layout(
            device,
            "effect_fullscreen",
            camera_bind_group_layout,
            texture_bind_group_layout,
        );
        let opts = |blend| PipelineOpts {
            label: "effect_fullscreen",
            blend,
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            depth_write: false,
            depth_compare: wgpu::CompareFunction::Always,
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
        Self {
            pipeline_alpha,
            pipeline_additive,
        }
    }
}

pub fn prepare_screen_quad_records<'tex>(
    list: &EffectDrawList,
    fallback_texture: &'tex wgpu::BindGroup,
    texture_lookup: impl Fn(&str) -> Option<&'tex wgpu::BindGroup>,
) -> Vec<DrawRecord<'tex>> {
    let mut records: Vec<DrawRecord<'tex>> = Vec::new();
    for (emission, prim) in list.primitives.iter().enumerate() {
        let (texture, blend, vertices, indices) = match prim {
            EffectPrimitiveDraw::ScreenQuad {
                texture,
                color,
                blend,
                corners,
                uvs,
            } => {
                let vertices: Vec<SpriteVertex> = (0..4)
                    .map(|i| SpriteVertex {
                        position: [corners[i][0], corners[i][1], 0.0],
                        tex_coord: uvs[i],
                        color: *color,
                    })
                    .collect();
                (texture, blend, vertices, vec![0, 1, 2, 0, 2, 3])
            }
            EffectPrimitiveDraw::ScreenMesh {
                texture,
                blend,
                vertices,
                indices,
            } => {
                let verts: Vec<SpriteVertex> = vertices
                    .iter()
                    .map(|(pos, color)| SpriteVertex {
                        position: [pos[0], pos[1], 0.0],
                        tex_coord: [0.5, 0.5],
                        color: *color,
                    })
                    .collect();
                (texture, blend, verts, indices.clone())
            }
            _ => continue,
        };

        let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);

        records.push(DrawRecord::new(
            f32::MAX,
            emission as u32,
            BlendBucket::from_blend_kind(*blend),
            PipelineKind::FullscreenOverlay,
            vertices,
            indices,
            texture_bg,
        ));
    }
    records
}
