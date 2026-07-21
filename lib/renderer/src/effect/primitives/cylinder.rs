//! Truncated-cone band: the `Cylinder` draw variant, `PipelineKind::Cylinder`.
//!
//! We emit one open (un-capped) tube as a triangle strip of `sides` quads
//! joining a bottom ring of radius `bottom_size` to a top ring of radius
//! `top_size`. Positions are world space; the ring plane is local XZ and the
//! height runs along negative local Y, matching the convention that up is
//! negative Y. The local frame is tilted about X (`tilt_x_rad`) then yawed about
//! Y (`rotation_y_rad`) and translated to `base`. U advances a quarter per side
//! plus `uv_scroll.x`, V spans bottom-to-top plus `uv_scroll.y`; the bottom ring
//! takes `alpha_bottom`. The record sorts at the mid-height point.
//!
//! Blend is per-record alpha or additive; both pipelines disable depth writes
//! and compare `LessEqual`. `CylinderRenderer` implements
//! `EffectPrimitiveRenderer` and lives in the registry under this kind. Emitted
//! by `EffectSpec::Custom` effects such as Magnus Exorcismus and Sanctuary
//! pillars.

use crate::camera::Camera;
use crate::effect::blend::ADDITIVE_BLEND;
use crate::effect::pipeline::{PipelineOpts, build_pipeline, effect_pipeline_layout};
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
        let layout = effect_pipeline_layout(
            device,
            "effect_cylinder",
            camera_bind_group_layout,
            texture_bind_group_layout,
        );
        let opts = |blend| PipelineOpts {
            label: "effect_cylinder",
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
