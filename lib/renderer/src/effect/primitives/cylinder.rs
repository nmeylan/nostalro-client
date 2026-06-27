use crate::camera::Camera;
use crate::device::DEPTH_FORMAT;
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind, view_z};
use crate::effect::{EffectDrawList, EffectPrimitiveDraw};
use crate::sprite::SpriteVertex;

pub struct CylinderRenderer {
    pub pipeline_alpha: wgpu::RenderPipeline,
    pub pipeline_additive: wgpu::RenderPipeline,
}

impl CylinderRenderer {
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
            include_str!("../../shaders/effect_cylinder.wgsl"),
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
            label: Some("effect_cylinder"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_cylinder"),
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
            label: Some("effect_cylinder"),
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

pub fn prepare_cylinder_records<'tex>(
    list: &EffectDrawList,
    camera: &Camera,
    fallback_texture: &'tex wgpu::BindGroup,
    texture_lookup: impl Fn(&str) -> Option<&'tex wgpu::BindGroup>,
) -> Vec<DrawRecord<'tex>> {
    let mut records: Vec<DrawRecord<'tex>> = Vec::new();
    for (emission, prim) in list.primitives.iter().enumerate() {
        let EffectPrimitiveDraw::Cylinder {
            base,
            bottom_size,
            top_size,
            height,
            sides,
            rotation,
            tilt_x_rad,
            rotation_y_rad,
            uv_scroll,
            texture,
            color,
            alpha_bottom,
            blend,
        } = prim
        else {
            continue;
        };

        let sides_n = (*sides).max(3);
        if *bottom_size <= 0.0 && *top_size <= 0.0 {
            continue;
        }
        if height.abs() <= 0.0 {
            continue;
        }

        let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);

        let (sin_tx, cos_tx) = tilt_x_rad.sin_cos();
        let (sin_ry, cos_ry) = rotation_y_rad.sin_cos();
        let transform_local = |lx: f32, ly: f32, lz: f32| -> [f32; 3] {
            let x1 = lx;
            let y1 = ly * cos_tx + lz * sin_tx;
            let z1 = -ly * sin_tx + lz * cos_tx;
            let x2 = x1 * cos_ry - z1 * sin_ry;
            let y2 = y1;
            let z2 = x1 * sin_ry + z1 * cos_ry;
            [base[0] + x2, base[1] + y2, base[2] + z2]
        };

        let bottom_local_y: f32 = 0.0;
        let top_local_y: f32 = -*height;
        let full_span = std::f32::consts::TAU;
        let scroll_u = uv_scroll[0];
        let scroll_v = uv_scroll[1];

        let mut vertices: Vec<SpriteVertex> = Vec::with_capacity(((sides_n + 1) * 2) as usize);
        let mut indices: Vec<u32> = Vec::with_capacity((sides_n * 6) as usize);

        for s in 0..=sides_n {
            let t = s as f32 / sides_n as f32;
            let local_angle = t * full_span + *rotation;
            let (sin_a, cos_a) = local_angle.sin_cos();

            let u_raw = s as f32 * 0.25 + scroll_u;

            vertices.push(SpriteVertex {
                position: transform_local(bottom_size * cos_a, bottom_local_y, bottom_size * sin_a),
                tex_coord: [u_raw, 1.0 + scroll_v],
                color: [color[0], color[1], color[2], *alpha_bottom],
            });
            vertices.push(SpriteVertex {
                position: transform_local(top_size * cos_a, top_local_y, top_size * sin_a),
                tex_coord: [u_raw, 0.0 + scroll_v],
                color: *color,
            });
        }

        for s in 0..sides_n {
            let b0 = 2 * s;
            let t0 = b0 + 1;
            let b1 = 2 * (s + 1);
            let t1 = b1 + 1;
            indices.extend_from_slice(&[b0, t0, b1, t0, t1, b1]);
        }

        let depth_anchor = [base[0], base[1] - height * 0.5, base[2]];

        records.push(DrawRecord::new(
            view_z(camera, depth_anchor),
            emission as u32,
            BlendBucket::from_blend_kind(*blend),
            PipelineKind::Cylinder,
            vertices,
            indices,
            texture_bg,
        ));
    }
    records
}
