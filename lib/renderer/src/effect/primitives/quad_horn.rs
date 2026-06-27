use crate::camera::Camera;
use crate::device::DEPTH_FORMAT;
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
