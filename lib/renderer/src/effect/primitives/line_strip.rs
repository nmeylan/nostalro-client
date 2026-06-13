//! LineStrip / Spline primitive — a camera-facing textured ribbon following
//! a polyline. Matches the original game's strip-style beams and curved
//! trails. A `Spline` is first tessellated from its Catmull-Rom control
//! points into a polyline, then fed through the same ribbon builder as a
//! plain `LineStrip`.
//!
//! Reuses `effect_frustum.wgsl` so the camera bind group layout and vertex
//! format match the other world-space primitives exactly.

use crate::camera::Camera;
use crate::device::DEPTH_FORMAT;
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind, view_z};
use crate::effect::{EffectDrawList, EffectPrimitiveDraw};
use crate::sprite::SpriteVertex;

pub struct LineStripRenderer {
    pub pipeline_alpha: wgpu::RenderPipeline,
    pub pipeline_additive: wgpu::RenderPipeline,
}

impl LineStripRenderer {
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
            label: Some("effect_line_strip"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_line_strip"),
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
            label: Some("effect_line_strip"),
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

/// Sample a uniform Catmull-Rom curve through `control` into `segments + 1`
/// points. Endpoints are clamped by duplicating the first / last control
/// point, so the curve starts and ends exactly on the outer control points.
fn tessellate_catmull_rom(control: &[[f32; 3]], segments: u32) -> Vec<[f32; 3]> {
    let n = control.len();
    if n < 2 || segments == 0 {
        return control.to_vec();
    }
    let spans = (n - 1) as f32;
    let count = segments + 1;
    let mut out = Vec::with_capacity(count as usize);
    let at = |i: isize| -> glam::Vec3 {
        let idx = i.clamp(0, (n - 1) as isize) as usize;
        glam::Vec3::from(control[idx])
    };
    for s in 0..count {
        // Global parameter in [0, spans]; integer part selects the span,
        // fractional part is the local t within it.
        let u = (s as f32 / segments as f32) * spans;
        let span = (u.floor() as isize).min((n - 2) as isize);
        let t = u - span as f32;
        let p0 = at(span - 1);
        let p1 = at(span);
        let p2 = at(span + 1);
        let p3 = at(span + 2);
        let t2 = t * t;
        let t3 = t2 * t;
        let pos = 0.5
            * ((2.0 * p1)
                + (-p0 + p2) * t
                + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3);
        out.push([pos.x, pos.y, pos.z]);
    }
    out
}

/// Build a camera-facing ribbon along `points`, appending vertices / indices.
/// Two vertices per point (offset `±half_width` perpendicular to the path and
/// the view direction); `uv_along` scales how fast the along-path texture
/// coordinate accumulates with length (V by default, U when `u_along`).
/// `colors` tints each path point individually; `None` → flat `color`.
fn build_ribbon(
    points: &[[f32; 3]],
    half_width: f32,
    uv_along: f32,
    u_along: bool,
    color: [f32; 4],
    colors: Option<&[[f32; 4]]>,
    eye: glam::Vec3,
    vertices: &mut Vec<SpriteVertex>,
    indices: &mut Vec<u32>,
) {
    let m = points.len();
    let mut cum_len = 0.0f32;
    for i in 0..m {
        let p = glam::Vec3::from(points[i]);
        let tangent = if i == 0 {
            glam::Vec3::from(points[1]) - p
        } else if i == m - 1 {
            p - glam::Vec3::from(points[m - 2])
        } else {
            glam::Vec3::from(points[i + 1]) - glam::Vec3::from(points[i - 1])
        };
        if i > 0 {
            cum_len += (p - glam::Vec3::from(points[i - 1])).length();
        }

        let view_dir = (eye - p).normalize_or_zero();
        let mut side = tangent.cross(view_dir);
        if side.length_squared() < 1e-8 {
            // Path points straight at the camera: fall back to a stable
            // world-up perpendicular so the ribbon keeps a finite width.
            side = tangent.cross(glam::Vec3::Y);
            if side.length_squared() < 1e-8 {
                side = glam::Vec3::X;
            }
        }
        let side = side.normalize() * half_width;
        let along = cum_len * uv_along;
        let (uv_left, uv_right) = if u_along {
            ([along, 0.0], [along, 1.0])
        } else {
            ([0.0, along], [1.0, along])
        };
        let color = colors.map_or(color, |c| c[i.min(c.len() - 1)]);

        let left = p - side;
        let right = p + side;
        vertices.push(SpriteVertex {
            position: [left.x, left.y, left.z],
            tex_coord: uv_left,
            color,
        });
        vertices.push(SpriteVertex {
            position: [right.x, right.y, right.z],
            tex_coord: uv_right,
            color,
        });
    }

    for i in 0..(m - 1) as u32 {
        let a = 2 * i;
        indices.extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
    }
}

pub fn prepare_line_strip_records<'tex>(
    list: &EffectDrawList,
    camera: &Camera,
    fallback_texture: &'tex wgpu::BindGroup,
    texture_lookup: impl Fn(&str) -> Option<&'tex wgpu::BindGroup>,
) -> Vec<DrawRecord<'tex>> {
    let eye = camera.eye();
    let mut records: Vec<DrawRecord<'tex>> = Vec::new();
    for (emission, prim) in list.primitives.iter().enumerate() {
        // Resolve both variants down to a polyline + shared ribbon params.
        let (points, half_width, uv_along, u_along, texture, color, colors, blend) = match prim {
            EffectPrimitiveDraw::LineStrip {
                points,
                uv_along,
                u_along,
                half_width,
                texture,
                color,
                colors,
                blend,
            } => (
                points.clone(),
                *half_width,
                *uv_along,
                *u_along,
                texture,
                color,
                colors.as_deref(),
                blend,
            ),
            EffectPrimitiveDraw::Spline {
                control_points,
                segments,
                half_width,
                texture,
                color,
                blend,
            } => (
                tessellate_catmull_rom(control_points, *segments),
                *half_width,
                1.0,
                false,
                texture,
                color,
                None,
                blend,
            ),
            _ => continue,
        };

        if points.len() < 2 || half_width <= 0.0 {
            continue;
        }

        let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);

        let mut vertices: Vec<SpriteVertex> = Vec::with_capacity(points.len() * 2);
        let mut indices: Vec<u32> = Vec::with_capacity((points.len() - 1) * 6);
        build_ribbon(
            &points,
            half_width,
            uv_along,
            u_along,
            *color,
            colors,
            eye,
            &mut vertices,
            &mut indices,
        );

        // Anchor depth at the path midpoint so the whole ribbon sorts as one.
        let mid = points[points.len() / 2];
        records.push(DrawRecord::new(
            view_z(camera, mid),
            emission as u32,
            BlendBucket::from_blend_kind(*blend),
            PipelineKind::LineStrip,
            vertices,
            indices,
            texture_bg,
        ));
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::BlendKind;

    fn dummy_bind_group() -> &'static wgpu::BindGroup {
        // The `prepare_*` path never dereferences the texture bind group, so
        // tests synthesise a dangling reference purely to satisfy the type.
        unsafe {
            let ptr = std::ptr::NonNull::<wgpu::BindGroup>::dangling().as_ptr();
            &*ptr
        }
    }

    fn prepare(list: &EffectDrawList) -> Vec<DrawRecord<'static>> {
        let camera = Camera::default();
        let fallback = dummy_bind_group();
        prepare_line_strip_records(list, &camera, fallback, |_| None)
    }

    #[test]
    fn line_strip_emits_two_verts_per_point() {
        let mut list = EffectDrawList::new();
        let points = vec![[0.0, 0.0, 0.0], [0.0, 0.0, 4.0], [0.0, 0.0, 8.0]];
        let n = points.len();
        list.push(EffectPrimitiveDraw::LineStrip {
            points,
            uv_along: 0.1,
            u_along: false,
            half_width: 0.5,
            texture: "x",
            color: [1.0, 1.0, 1.0, 1.0],
            colors: None,
            blend: BlendKind::Additive,
        });
        let records = prepare(&list);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].vertices.len(), 2 * n);
        assert_eq!(records[0].indices.len(), 6 * (n - 1));
    }

    #[test]
    fn spline_tessellates_to_segments_plus_one_cross_sections() {
        let mut list = EffectDrawList::new();
        let segments = 8u32;
        list.push(EffectPrimitiveDraw::Spline {
            control_points: vec![
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 3.0],
                [0.0, 0.0, 6.0],
                [0.0, 0.0, 9.0],
            ],
            segments,
            half_width: 0.5,
            texture: "x",
            color: [1.0, 1.0, 1.0, 1.0],
            blend: BlendKind::Additive,
        });
        let records = prepare(&list);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].vertices.len(), 2 * (segments as usize + 1));
    }

    #[test]
    fn straight_control_points_stay_collinear() {
        let control = [
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 3.0],
            [0.0, 0.0, 6.0],
            [0.0, 0.0, 9.0],
        ];
        let pts = tessellate_catmull_rom(&control, 12);
        for p in &pts {
            assert!(p[0].abs() < 1e-4 && p[1].abs() < 1e-4, "off-axis: {p:?}");
        }
        // Monotonically advancing along Z.
        for w in pts.windows(2) {
            assert!(w[1][2] >= w[0][2] - 1e-4);
        }
    }

    #[test]
    fn degenerate_single_point_produces_no_record() {
        let mut list = EffectDrawList::new();
        list.push(EffectPrimitiveDraw::LineStrip {
            points: vec![[0.0, 0.0, 0.0]],
            uv_along: 0.1,
            u_along: false,
            half_width: 0.5,
            texture: "x",
            color: [1.0, 1.0, 1.0, 1.0],
            colors: None,
            blend: BlendKind::Additive,
        });
        assert!(prepare(&list).is_empty());
    }
}
