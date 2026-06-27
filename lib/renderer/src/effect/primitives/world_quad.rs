use crate::camera::Camera;
use crate::device::DEPTH_FORMAT;
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_world_quad"),
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

        let pipeline_alpha = Self::create_pipeline(
            device,
            surface_format,
            &pipeline_layout,
            &shader,
            alpha,
            depth_compare,
        );
        let pipeline_additive = Self::create_pipeline(
            device,
            surface_format,
            &pipeline_layout,
            &shader,
            additive,
            depth_compare,
        );
        (pipeline_alpha, pipeline_additive)
    }

    fn create_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        pipeline_layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        blend: wgpu::BlendState,
        depth_compare: wgpu::CompareFunction,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("effect_world_quad"),
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
                depth_compare,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
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
