use crate::device::DEPTH_FORMAT;
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_fullscreen"),
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
        Self {
            pipeline_alpha,
            pipeline_additive,
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
            label: Some("effect_fullscreen"),
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
                depth_compare: wgpu::CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
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
