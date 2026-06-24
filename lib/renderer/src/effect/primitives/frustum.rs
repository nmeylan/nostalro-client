//! Frustum primitive — vertical "tube" between two coaxial rings.
//!
//! Used by effects whose silhouette is a vertical band of texture: Magnum
//! Break's explosion cone, Sanctuary / Magnus pillars, Bottom-Sanc rotating
//! pillar. Geometry is a closed triangle strip from a bottom polygon
//! (`bottom_size` radius) up to a top polygon (`top_size` radius). When the
//! two radii are equal it's a cylinder; when `top_size == 0` it's a cone.
//! `sides == 4` gives a square pillar (Bottom-Sanc), high `sides` approximate
//! a smooth circular tube.
//!
//! The texture wraps once around the lateral surface unless `rotation` shifts
//! the seam; `v = 0` at the bottom, `v = 1` at the top.

use crate::camera::Camera;
use crate::device::DEPTH_FORMAT;
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind, view_z};
use crate::effect::{EffectDrawList, EffectPrimitiveDraw};
use crate::sprite::SpriteVertex;
use ragnarok_game::effect::draw::FrustumWaveMode;

/// Pipeline holder for the Frustum primitive. The unified effect-dispatch
/// path owns the per-frame vertex / index buffer; this struct just owns the
/// pipelines used to draw frustum records.
pub struct FrustumRenderer {
    pub pipeline_alpha: wgpu::RenderPipeline,
    pub pipeline_additive: wgpu::RenderPipeline,
}

impl FrustumRenderer {
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

    /// Rebuild both pipelines from a runtime-supplied WGSL source. Used by
    /// the effect viewer's hot-reload path; production code calls `new()`
    /// once with the `include_str!`'d source.
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
            label: Some("effect_frustum"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_frustum"),
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
            label: Some("effect_frustum"),
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

/// Build one [`DrawRecord`] per `EffectPrimitiveDraw::Frustum` entry in
/// `list`. The records reference `fallback_texture` when `texture_lookup`
/// returns `None`.
pub fn prepare_frustum_records<'tex>(
    list: &EffectDrawList,
    camera: &Camera,
    fallback_texture: &'tex wgpu::BindGroup,
    texture_lookup: impl Fn(&str) -> Option<&'tex wgpu::BindGroup>,
) -> Vec<DrawRecord<'tex>> {
    let mut records: Vec<DrawRecord<'tex>> = Vec::new();
    let eye = camera.eye();
    for (emission, prim) in list.primitives.iter().enumerate() {
        let EffectPrimitiveDraw::Frustum {
            base,
            bottom_size,
            top_size,
            height,
            sides,
            arc_angle_deg,
            rotation,
            uv_repeat,
            uv_scroll,
            wave_amplitude,
            wave_frequency,
            wave_phase,
            wave_mode,
            tilt_x_rad,
            rotation_y_rad,
            cull_back,
            base_alpha,
            texture,
            color,
            blend,
        } = prim
        else {
            continue;
        };

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

        let sides_n = (*sides).max(3);
        if *bottom_size <= 0.0 && *top_size <= 0.0 {
            continue;
        }
        if height.abs() <= 0.0 {
            continue;
        }

        let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);

        let bottom_local_y: f32 = 0.0;
        let top_local_y_base: f32 = -*height;
        let full_span = arc_angle_deg
            .to_radians()
            .clamp(0.0, std::f32::consts::TAU);
        let geom_rotation = *rotation;
        let uv_rep = *uv_repeat;
        let scroll_v = uv_scroll[1];

        let delta_r = *top_size - *bottom_size;
        let tilt_len = (delta_r * delta_r + height * height).sqrt();
        let (radial_unit, vert_unit) = if tilt_len > 0.0 {
            (delta_r / tilt_len, height / tilt_len)
        } else {
            (0.0, 1.0)
        };

        let radial_flare = (*top_size - *bottom_size).abs();
        let flatness = radial_flare / (radial_flare + height.abs()).max(1e-3);
        const FADE_ONSET: f32 = 0.2;
        const FADE_COMPLETE: f32 = 0.53;
        let fade_strength = if *cull_back {
            ((flatness - FADE_ONSET) / (FADE_COMPLETE - FADE_ONSET)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let eye_xz_x = eye.x - base[0];
        let eye_xz_z = eye.z - base[2];
        let eye_xz_len = (eye_xz_x * eye_xz_x + eye_xz_z * eye_xz_z)
            .sqrt()
            .max(1e-3);

        let mut vertices: Vec<SpriteVertex> = Vec::with_capacity(((sides_n + 1) * 2) as usize);
        let mut indices: Vec<u32> = Vec::with_capacity((sides_n * 6) as usize);

        for s in 0..=sides_n {
            let t = s as f32 / sides_n as f32;
            let local_angle = t * full_span;
            let world_angle = local_angle + geom_rotation;
            let (sin_a, cos_a) = world_angle.sin_cos();
            let u = t * uv_rep + uv_scroll[0];

            let wave = match wave_mode {
                FrustumWaveMode::Sine => {
                    *wave_amplitude * (local_angle * *wave_frequency + *wave_phase).sin()
                }
                FrustumWaveMode::SaintBell => {
                    // Per-segment height formula for the saint-casting bell,
                    // matching the original game's on-screen profile:
                    //   middle = (E_DIVISION-1)/2 = 10
                    //   m2     = 90 / middle      = 9     (degrees per segment)
                    //   SinLimit_i = 90° + (i - 10) * 9°  with i = 0..20
                    //   height[i] = max_h * (1 + sin(SinLimit_i) * 0.3 * sin(pr))
                    // SinLimit sweeps 0°..180° across the 360° cone surface,
                    // so the bell is `sin(local_angle / 2)` — a half-sine
                    // peaking at the segment opposite the seam (local_angle=π)
                    // and falling to **zero** at the seam itself (local_angle
                    // = 0 or 2π). Always non-negative. `wave_amplitude`
                    // carries the `sin(pr)` time pulse including its sign,
                    // so negative wave_amplitude pulls the bell-side
                    // inward instead of outward (the cone "breathes").
                    let bell = (local_angle * 0.5).sin();
                    *wave_amplitude * bell
                }
            };
            let seg_top_size = top_size + wave * radial_unit;
            let seg_top_local_y = top_local_y_base - wave * vert_unit;

            let outward_dot_xz = cos_a * eye_xz_x + sin_a * eye_xz_z;
            let front_factor = ((outward_dot_xz / eye_xz_len) + 1.0) * 0.5;
            let front_weight = front_factor * front_factor;
            let segment_alpha = 1.0 - fade_strength * (1.0 - front_weight);
            let mut seg_color = *color;
            seg_color[3] *= segment_alpha;
            // Fade the base ring of vertices so a tight, additively-overlapped
            // cone base doesn't bloom into a solid disc. Top rim stays full.
            let mut base_color = seg_color;
            base_color[3] *= *base_alpha;

            vertices.push(SpriteVertex {
                position: transform_local(
                    bottom_size * cos_a,
                    bottom_local_y,
                    bottom_size * sin_a,
                ),
                tex_coord: [u, 1.0 + scroll_v],
                color: base_color,
            });
            vertices.push(SpriteVertex {
                position: transform_local(
                    seg_top_size * cos_a,
                    seg_top_local_y,
                    seg_top_size * sin_a,
                ),
                tex_coord: [u, 0.0 + scroll_v],
                color: seg_color,
            });
        }

        for s in 0..sides_n {
            let b0 = 2 * s;
            let t0 = b0 + 1;
            let b1 = 2 * (s + 1);
            let t1 = b1 + 1;
            indices.extend_from_slice(&[b0, t0, b1, t0, t1, b1]);
        }

        // Depth at the cone's midpoint matches the original game's per-RP
        // `oow = 1/w` at the same anchor; close enough for back-to-front
        // sort within the alpha bucket.
        let depth_anchor = [
            base[0],
            base[1] - height * 0.5,
            base[2],
        ];

        records.push(DrawRecord::new(
            view_z(camera, depth_anchor),
            emission as u32,
            BlendBucket::from_blend_kind(*blend),
            PipelineKind::Frustum,
            vertices,
            indices,
            texture_bg,
        ));
    }
    records
}
