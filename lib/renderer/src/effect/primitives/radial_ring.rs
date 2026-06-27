use std::f32::consts::TAU;

use crate::camera::Camera;
use crate::device::DEPTH_FORMAT;
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind, view_z};
use crate::effect::{EffectDrawList, EffectPrimitiveDraw};
use crate::sprite::SpriteVertex;

pub struct RadialRingRenderer {
    pub pipeline_alpha: wgpu::RenderPipeline,
    pub pipeline_additive: wgpu::RenderPipeline,
}

impl RadialRingRenderer {
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
            label: Some("effect_radial_ring"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_radial_ring"),
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
            label: Some("effect_radial_ring"),
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

pub fn prepare_radial_ring_records<'tex>(
    list: &EffectDrawList,
    camera: &Camera,
    fallback_texture: &'tex wgpu::BindGroup,
    texture_lookup: impl Fn(&str) -> Option<&'tex wgpu::BindGroup>,
) -> Vec<DrawRecord<'tex>> {
    let mut records: Vec<DrawRecord<'tex>> = Vec::new();
    for (emission, prim) in list.primitives.iter().enumerate() {
        let EffectPrimitiveDraw::RadialRing {
            center,
            distance,
            rise_angle_rad,
            rot_start_rad,
            full_arc_rad,
            segments,
            height_scale,
            heights,
            texture,
            color,
            blend,
        } = prim
        else {
            continue;
        };

        let segments_n = *segments;
        if segments_n == 0 || *distance <= 0.0 {
            continue;
        }

        let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);

        let arc = if *full_arc_rad == 0.0 {
            TAU
        } else {
            *full_arc_rad
        };
        let closed = (arc - TAU).abs() < 1e-4;
        let (sin_r, cos_r) = rise_angle_rad.sin_cos();

        let position_count = segments_n + 1;
        let mut positions: Vec<([f32; 3], [f32; 3])> = Vec::with_capacity(position_count as usize);
        for i in 0..position_count {
            let idx = if closed && i == segments_n { 0 } else { i };
            let frac = idx as f32 / segments_n as f32;
            let theta = rot_start_rad + frac * arc;
            let (sin_t, cos_t) = theta.sin_cos();

            let h = heights[(idx as usize) % heights.len()] * height_scale;
            let top_radial = h * cos_r;
            let top_up = h * sin_r;

            let bottom = [
                center[0] + distance * cos_t,
                center[1],
                center[2] + distance * sin_t,
            ];
            let top = [
                bottom[0] + top_radial * cos_t,
                bottom[1] - top_up,
                bottom[2] + top_radial * sin_t,
            ];
            positions.push((bottom, top));
        }

        let mut vertices: Vec<SpriteVertex> = Vec::with_capacity((segments_n as usize) * 4);
        let mut indices: Vec<u32> = Vec::with_capacity((segments_n as usize) * 6);

        for seg in 0..segments_n {
            let (prev_bot, prev_top) = positions[seg as usize];
            let (this_bot, this_top) = positions[seg as usize + 1];
            let h_prev = heights[(seg as usize) % heights.len()];
            let h_this = heights[((seg as usize + 1)
                % if closed {
                    segments_n as usize
                } else {
                    position_count as usize
                })
                % heights.len()];
            if h_prev * height_scale <= 0.0 && h_this * height_scale <= 0.0 {
                continue;
            }

            let u_prev = seg as f32 / segments_n as f32;
            let u_this = (seg + 1) as f32 / segments_n as f32;
            let base_idx = vertices.len() as u32;

            vertices.push(SpriteVertex {
                position: prev_bot,
                tex_coord: [u_prev, 1.0],
                color: *color,
            });
            vertices.push(SpriteVertex {
                position: this_bot,
                tex_coord: [u_this, 1.0],
                color: *color,
            });
            vertices.push(SpriteVertex {
                position: this_top,
                tex_coord: [u_this, 0.0],
                color: *color,
            });
            vertices.push(SpriteVertex {
                position: prev_top,
                tex_coord: [u_prev, 0.0],
                color: *color,
            });

            indices.extend_from_slice(&[
                base_idx,
                base_idx + 1,
                base_idx + 2,
                base_idx,
                base_idx + 2,
                base_idx + 3,
            ]);
        }

        records.push(DrawRecord::new(
            view_z(camera, *center),
            emission as u32,
            BlendBucket::from_blend_kind(*blend),
            PipelineKind::RadialRing,
            vertices,
            indices,
            texture_bg,
        ));
    }
    records
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use ragnarok_game::effect::radial_emitter::RADIAL_EMITTER_DIVISION;

    use super::*;
    use crate::camera::Camera;
    use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw};

    fn dummy_camera() -> Camera {
        Camera::default()
    }

    fn dummy_bg() -> &'static wgpu::BindGroup {
        // SAFETY: texture is never dereferenced in these tests.
        unsafe {
            let ptr = std::ptr::NonNull::<wgpu::BindGroup>::dangling().as_ptr();
            &*ptr
        }
    }

    #[test]
    fn closed_ring_connects_segments_into_a_strip() {
        let mut heights = [0.0; RADIAL_EMITTER_DIVISION];
        heights[0] = 5.0;
        heights[1] = 7.0;
        heights[2] = 3.0;
        heights[3] = 1.0;

        let mut list = EffectDrawList::new();
        list.push(EffectPrimitiveDraw::RadialRing {
            center: [0.0, 0.0, 0.0],
            distance: 10.0,
            rise_angle_rad: FRAC_PI_2,
            rot_start_rad: 0.0,
            full_arc_rad: 0.0, // sentinel → TAU (closed loop)
            segments: 4,
            height_scale: 1.0,
            heights,
            texture: "ring_yellow.tga",
            color: [1.0, 1.0, 1.0, 1.0],
            blend: BlendKind::Alpha,
        });

        let records = prepare_radial_ring_records(&list, &dummy_camera(), dummy_bg(), |_| None);
        assert_eq!(records.len(), 1);
        let r = &records[0];
        assert_eq!(r.vertices.len(), 16);
        assert_eq!(r.indices.len(), 24);

        let seg0_prev_bot = r.vertices[0].position;
        let seg0_this_bot = r.vertices[1].position;
        let seg0_this_top = r.vertices[2].position;
        let seg0_prev_top = r.vertices[3].position;

        assert!((seg0_prev_bot[0] - 10.0).abs() < 1e-4 && seg0_prev_bot[2].abs() < 1e-4);
        assert!((seg0_prev_top[1] + 5.0).abs() < 1e-4);
        assert!(seg0_this_bot[0].abs() < 1e-4 && (seg0_this_bot[2] - 10.0).abs() < 1e-4);
        assert!((seg0_this_top[1] + 7.0).abs() < 1e-4);

        let seg3_this_bot = r.vertices[13].position;
        let seg3_this_top = r.vertices[14].position;
        assert!((seg3_this_bot[0] - 10.0).abs() < 1e-4 && seg3_this_bot[2].abs() < 1e-4);
        assert!((seg3_this_top[1] + 5.0).abs() < 1e-4);
    }
}
