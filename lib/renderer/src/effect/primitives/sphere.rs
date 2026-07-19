use crate::camera::Camera;
use crate::device::DEPTH_FORMAT;
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind, view_z};
use crate::effect::{EffectDrawList, EffectPrimitiveDraw};
use crate::sprite::SpriteVertex;

pub struct SphereRenderer {
    pub pipeline_alpha: wgpu::RenderPipeline,
    pub pipeline_additive: wgpu::RenderPipeline,
    pub pipeline_alpha_no_depth: wgpu::RenderPipeline,
    pub pipeline_additive_no_depth: wgpu::RenderPipeline,
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
            wgpu::CompareFunction::LessEqual,
        );
        let (pipeline_alpha_no_depth, pipeline_additive_no_depth) = Self::build_pipelines(
            device,
            surface_format,
            camera_bind_group_layout,
            texture_bind_group_layout,
            include_str!("../../shaders/effect_sphere.wgsl"),
            wgpu::CompareFunction::Always,
        );
        Self {
            pipeline_alpha,
            pipeline_additive,
            pipeline_alpha_no_depth,
            pipeline_additive_no_depth,
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
            wgpu::CompareFunction::LessEqual,
        );
        self.pipeline_alpha = alpha;
        self.pipeline_additive = additive;
        let (alpha_nd, additive_nd) = Self::build_pipelines(
            device,
            surface_format,
            camera_bind_group_layout,
            texture_bind_group_layout,
            shader_source,
            wgpu::CompareFunction::Always,
        );
        self.pipeline_alpha_no_depth = alpha_nd;
        self.pipeline_additive_no_depth = additive_nd;
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
            label: Some("effect_sphere"),
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

pub fn prepare_sphere_records<'tex>(
    list: &EffectDrawList,
    camera: &Camera,
    fallback_texture: &'tex wgpu::BindGroup,
    texture_lookup: impl Fn(&str) -> Option<&'tex wgpu::BindGroup>,
) -> Vec<DrawRecord<'tex>> {
    let mut records: Vec<DrawRecord<'tex>> = Vec::new();
    for (emission, prim) in list.primitives.iter().enumerate() {
        let EffectPrimitiveDraw::Sphere {
            center,
            radius,
            sides_lat,
            sides_lon,
            longitude_offset,
            longitude_arc,
            uv_repeat,
            texture,
            color,
            blend,
            no_depth,
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

        let lat_count = lat_segs + 1;
        let lon_count = lon_segs + 1;

        let mut vertices: Vec<SpriteVertex> = Vec::with_capacity((lat_count * lon_count) as usize);
        let mut indices: Vec<u32> = Vec::with_capacity((lat_segs * lon_segs * 6) as usize);

        for lat in 0..lat_count {
            let v = lat as f32 / lat_segs as f32;
            let phi = -std::f32::consts::FRAC_PI_2 + v * std::f32::consts::PI;
            let (sin_phi, cos_phi) = phi.sin_cos();
            let tv = v * uv_repeat[1];
            for lon in 0..lon_count {
                let u = lon as f32 / lon_segs as f32;
                let theta = u * *longitude_arc + *longitude_offset;
                let (sin_theta, cos_theta) = theta.sin_cos();
                let px = center[0] + radius * cos_phi * cos_theta;
                let py = center[1] - radius * sin_phi;
                let pz = center[2] + radius * cos_phi * sin_theta;
                let tu = u * uv_repeat[0];
                vertices.push(SpriteVertex {
                    position: [px, py, pz],
                    tex_coord: [tu, tv],
                    color: *color,
                });
            }
        }

        for lat in 0..lat_segs {
            for lon in 0..lon_segs {
                let row0 = lat * lon_count;
                let row1 = (lat + 1) * lon_count;
                let a = row0 + lon;
                let b = row0 + lon + 1;
                let c = row1 + lon;
                let d = row1 + lon + 1;
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }

        let bucket = match (BlendBucket::from_blend_kind(*blend), *no_depth) {
            (BlendBucket::Alpha, true) => BlendBucket::AlphaNoDepth,
            (BlendBucket::Additive, true) => BlendBucket::AdditiveNoDepth,
            (bucket, _) => bucket,
        };
        records.push(DrawRecord::new(
            view_z(camera, *center),
            emission as u32,
            bucket,
            PipelineKind::Sphere,
            vertices,
            indices,
            texture_bg,
        ));
    }
    records
}
