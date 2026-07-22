//! Camera-facing billboards: the `Billboard`, `BillboardFlash`,
//! `BillboardDepthAnchored`, `BillboardDisc` and `BillboardRing` draw variants.
//!
//! Geometry is built in screen space. We project the world `pos` to a
//! screen-space anchor and lay out vertices in pixel units (world `size` and
//! `radius` are multiplied by the projection's pixels-per-unit), so a quad
//! always faces the camera and keeps a constant on-screen size for a given
//! world size. `rotation` is a screen-space counter-clockwise angle in radians.
//! `Billboard` is a rotatable quad, `BillboardDisc` a triangle fan, and
//! `BillboardRing` a two-ring triangle strip. `BillboardFlash` and the
//! disc/ring pin vertex Z to 0 so they draw as a near overlay; `Billboard` and
//! `BillboardDepthAnchored` write a projected NDC depth into vertex Z.
//!
//! These records carry `PipelineKind::Sprite` because they share the sprite
//! pipeline (which binds the group-0 sprite uniform). They are dispatched
//! straight from `build_effect_records`, not through the primitive registry,
//! and unlike the other primitives this builder needs the screen dimensions.
//! Per-record blend comes from the draw's `BlendKind`. All of these are emitted
//! by `EffectSpec::Custom` effects (for example Bash for the disc, Magnum Break
//! for the ring).

use crate::camera::Camera;
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind};
use crate::effect_sprite::{
    project_billboard, project_billboard_biased, project_billboard_depth_anchored,
};
use crate::sprite::SpriteVertex;

use super::super::{EffectDrawList, EffectPrimitiveDraw};

#[cfg(test)]
use super::super::BlendKind;

pub fn prepare_billboard_records<'tex>(
    list: &EffectDrawList,
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
    fallback_texture: &'tex wgpu::BindGroup,
    texture_lookup: impl Fn(&str) -> Option<&'tex wgpu::BindGroup>,
) -> Vec<DrawRecord<'tex>> {
    let mut records: Vec<DrawRecord<'tex>> = Vec::new();
    for (emission, prim) in list.primitives.iter().enumerate() {
        if let EffectPrimitiveDraw::BillboardSpriteDisc {
            pos,
            size,
            segments,
            rotation,
            texture,
            color,
            blend,
        } = prim
        {
            let Some((anchor, ndc_z, ppu)) =
                project_billboard_biased(camera, *pos, screen_w, screen_h)
            else {
                continue;
            };
            let r = size[0] * ppu * 0.5;
            let n = (*segments).max(8);
            let (sin_r, cos_r) = rotation.sin_cos();
            let z = ndc_z;
            let mut vertices: Vec<SpriteVertex> = Vec::with_capacity(n as usize + 2);
            vertices.push(SpriteVertex {
                position: [anchor[0], anchor[1], z],
                tex_coord: [0.5, 0.5],
                color: *color,
            });
            for s in 0..=n {
                let theta = (s as f32 / n as f32) * std::f32::consts::TAU;
                let (sin_t, cos_t) = theta.sin_cos();
                let dx = r * cos_t;
                let dy = r * sin_t;
                vertices.push(SpriteVertex {
                    position: [
                        anchor[0] + dx * cos_r - dy * sin_r,
                        anchor[1] + dx * sin_r + dy * cos_r,
                        z,
                    ],
                    tex_coord: [0.5 + 0.5 * cos_t, 0.5 + 0.5 * sin_t],
                    color: *color,
                });
            }
            let mut indices: Vec<u32> = Vec::with_capacity(n as usize * 3);
            for s in 0..n {
                indices.push(0);
                indices.push(1 + s);
                indices.push(2 + s);
            }
            let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);
            records.push(DrawRecord::new(
                super::super::queue::view_z(camera, *pos),
                emission as u32,
                BlendBucket::from_blend_kind(*blend),
                PipelineKind::Sprite,
                vertices,
                indices,
                texture_bg,
            ));
            continue;
        }

        if let EffectPrimitiveDraw::BillboardDisc {
            pos,
            radius,
            segments,
            uv_repeat,
            texture,
            color,
            blend,
        } = prim
        {
            let Some((anchor, _ndc_z, ppu)) = project_billboard(camera, *pos, screen_w, screen_h)
            else {
                continue;
            };
            let r = radius * ppu;
            let n = (*segments).max(8);
            let z = 0.0;
            let mut vertices: Vec<SpriteVertex> = Vec::with_capacity(n as usize + 2);
            vertices.push(SpriteVertex {
                position: [anchor[0], anchor[1], z],
                tex_coord: [0.5, 1.0],
                color: *color,
            });
            for s in 0..=n {
                let t = s as f32 / n as f32;
                let theta = t * std::f32::consts::TAU;
                let (sin_t, cos_t) = theta.sin_cos();
                vertices.push(SpriteVertex {
                    position: [anchor[0] + r * cos_t, anchor[1] + r * sin_t, z],
                    tex_coord: [t * uv_repeat, 0.0],
                    color: *color,
                });
            }
            let mut indices: Vec<u32> = Vec::with_capacity(n as usize * 3);
            for s in 0..n {
                indices.push(0);
                indices.push(1 + s);
                indices.push(2 + s);
            }
            let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);
            records.push(DrawRecord::new(
                super::super::queue::view_z(camera, *pos),
                emission as u32,
                BlendBucket::from_blend_kind(*blend),
                PipelineKind::Sprite,
                vertices,
                indices,
                texture_bg,
            ));
            continue;
        }

        if let EffectPrimitiveDraw::BillboardRing {
            pos,
            radius,
            thickness,
            segments,
            uv_repeat,
            texture,
            color,
            blend,
        } = prim
        {
            let Some((anchor, _ndc_z, ppu)) = project_billboard(camera, *pos, screen_w, screen_h)
            else {
                continue;
            };
            let r_outer = radius * ppu;
            let r_inner = (radius - thickness).max(0.0) * ppu;
            if r_outer <= 0.0 {
                continue;
            }
            let n = (*segments).max(8);
            let z = 0.0;
            let mut vertices: Vec<SpriteVertex> = Vec::with_capacity((n as usize + 1) * 2);
            for s in 0..=n {
                let t = s as f32 / n as f32;
                let theta = t * std::f32::consts::TAU;
                let (sin_t, cos_t) = theta.sin_cos();
                let u = t * uv_repeat;
                vertices.push(SpriteVertex {
                    position: [anchor[0] + r_outer * cos_t, anchor[1] + r_outer * sin_t, z],
                    tex_coord: [u, 0.0],
                    color: *color,
                });
                vertices.push(SpriteVertex {
                    position: [anchor[0] + r_inner * cos_t, anchor[1] + r_inner * sin_t, z],
                    tex_coord: [u, 1.0],
                    color: *color,
                });
            }
            let mut indices: Vec<u32> = Vec::with_capacity(n as usize * 6);
            for s in 0..n {
                let i = s * 2;
                indices.push(i);
                indices.push(i + 1);
                indices.push(i + 2);
                indices.push(i + 2);
                indices.push(i + 1);
                indices.push(i + 3);
            }
            let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);
            records.push(DrawRecord::new(
                super::super::queue::view_z(camera, *pos),
                emission as u32,
                BlendBucket::from_blend_kind(*blend),
                PipelineKind::Sprite,
                vertices,
                indices,
                texture_bg,
            ));
            continue;
        }

        if let EffectPrimitiveDraw::BillboardDepthAnchored {
            pos,
            depth_pos,
            size,
            uv,
            rotation,
            texture,
            color,
            blend,
        } = prim
        {
            let Some((anchor, ndc_z, ppu, view_depth)) =
                project_billboard_depth_anchored(camera, *pos, *depth_pos, screen_w, screen_h)
            else {
                continue;
            };
            let half_w = size[0] * ppu * 0.5;
            let half_h = size[1] * ppu * 0.5;
            let (sin_r, cos_r) = rotation.sin_cos();
            let rotate = |dx: f32, dy: f32| -> [f32; 2] {
                [
                    anchor[0] + dx * cos_r - dy * sin_r,
                    anchor[1] + dx * sin_r + dy * cos_r,
                ]
            };
            let corners = [
                (rotate(-half_w, -half_h), uv[0]),
                (rotate(half_w, -half_h), uv[1]),
                (rotate(-half_w, half_h), uv[2]),
                (rotate(half_w, half_h), uv[3]),
            ];
            let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);
            let vertices = corners
                .iter()
                .map(|(p, t)| SpriteVertex {
                    position: [p[0], p[1], ndc_z],
                    tex_coord: *t,
                    color: *color,
                })
                .collect();
            records.push(DrawRecord::new(
                view_depth,
                emission as u32,
                BlendBucket::from_blend_kind(*blend),
                PipelineKind::Sprite,
                vertices,
                vec![0, 1, 2, 1, 3, 2],
                texture_bg,
            ));
            continue;
        }

        if let EffectPrimitiveDraw::BillboardFlash {
            pos,
            size,
            uv,
            rotation,
            texture,
            color,
            blend,
        } = prim
        {
            let Some((anchor, _ndc_z, ppu)) = project_billboard(camera, *pos, screen_w, screen_h)
            else {
                continue;
            };
            let half_w = size[0] * ppu * 0.5;
            let half_h = size[1] * ppu * 0.5;
            let (sin_r, cos_r) = rotation.sin_cos();
            let rotate = |dx: f32, dy: f32| -> [f32; 2] {
                [
                    anchor[0] + dx * cos_r - dy * sin_r,
                    anchor[1] + dx * sin_r + dy * cos_r,
                ]
            };
            let corners = [
                (rotate(-half_w, -half_h), uv[0]),
                (rotate(half_w, -half_h), uv[1]),
                (rotate(-half_w, half_h), uv[2]),
                (rotate(half_w, half_h), uv[3]),
            ];
            let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);
            let vertices = corners
                .iter()
                .map(|(p, t)| SpriteVertex {
                    position: [p[0], p[1], 0.0],
                    tex_coord: *t,
                    color: *color,
                })
                .collect();
            records.push(DrawRecord::new(
                super::super::queue::view_z(camera, *pos),
                emission as u32,
                BlendBucket::from_blend_kind(*blend),
                PipelineKind::Sprite,
                vertices,
                vec![0, 1, 2, 1, 3, 2],
                texture_bg,
            ));
            continue;
        }

        let EffectPrimitiveDraw::Billboard {
            pos,
            size,
            uv,
            rotation,
            texture,
            color,
            blend,
        } = prim
        else {
            continue;
        };

        let Some((anchor, ndc_z, ppu)) = project_billboard_biased(camera, *pos, screen_w, screen_h)
        else {
            continue;
        };

        let half_w = size[0] * ppu * 0.5;
        let half_h = size[1] * ppu * 0.5;
        let (sin_r, cos_r) = rotation.sin_cos();
        let rotate = |dx: f32, dy: f32| -> [f32; 2] {
            [
                anchor[0] + dx * cos_r - dy * sin_r,
                anchor[1] + dx * sin_r + dy * cos_r,
            ]
        };
        let corners = [
            (rotate(-half_w, -half_h), uv[0]),
            (rotate(half_w, -half_h), uv[1]),
            (rotate(-half_w, half_h), uv[2]),
            (rotate(half_w, half_h), uv[3]),
        ];

        let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);
        let z = ndc_z;
        let vertices = corners
            .iter()
            .map(|(p, t)| SpriteVertex {
                position: [p[0], p[1], z],
                tex_coord: *t,
                color: *color,
            })
            .collect();

        records.push(DrawRecord::new(
            super::super::queue::view_z(camera, *pos),
            emission as u32,
            BlendBucket::from_blend_kind(*blend),
            PipelineKind::Sprite,
            vertices,
            vec![0, 1, 2, 1, 3, 2],
            texture_bg,
        ));
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_list_construction() {
        let mut list = EffectDrawList::new();
        list.push(EffectPrimitiveDraw::Billboard {
            pos: [0.0, 0.0, 0.0],
            size: [10.0, 10.0],
            uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            rotation: 0.0,
            texture: "missing",
            color: [1.0, 1.0, 1.0, 1.0],
            blend: BlendKind::Additive,
        });
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn blend_bucket_mapping() {
        assert_eq!(
            BlendBucket::from_blend_kind(BlendKind::Additive),
            BlendBucket::Additive
        );
        assert_eq!(
            BlendBucket::from_blend_kind(BlendKind::Alpha),
            BlendBucket::Alpha
        );
        assert_eq!(
            BlendBucket::from_blend_kind(BlendKind::Multiply),
            BlendBucket::Multiply
        );
        assert_eq!(
            BlendBucket::from_blend_kind(BlendKind::Raw { src: 5, dst: 6 }),
            BlendBucket::Alpha
        );
        assert_eq!(
            BlendBucket::from_blend_kind(BlendKind::Raw { src: 5, dst: 2 }),
            BlendBucket::Additive
        );
    }
}
