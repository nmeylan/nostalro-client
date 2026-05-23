use crate::camera::Camera;
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind};
use crate::effect_sprite::project_billboard;
use crate::sprite::SpriteVertex;

use super::super::{EffectDrawList, EffectPrimitiveDraw};

#[cfg(test)]
use super::super::BlendKind;

/// Convert every `Billboard` / `BillboardDisc` in `list` into a
/// [`DrawRecord`] dispatched through the sprite pipeline.
///
/// `texture_lookup` resolves the per-primitive texture name (bare
/// filename, e.g. `ring_yellow.tga`) against the caller's texture cache;
/// missing textures fall back to `fallback_texture`.
///
/// Vertices are in *screen space* — anchor produced by `project_billboard`
/// plus per-corner pixel offsets — and depth is the NDC z returned by the
/// same projection. The sprite pipeline interprets `position.xy` as screen
/// pixels and `position.z` as NDC depth.
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
            let Some((anchor, ndc_z, ppu)) =
                project_billboard(camera, *pos, screen_w, screen_h)
            else {
                continue;
            };
            let r = radius * ppu;
            let n = (*segments).max(8);
            let z = ndc_z - 0.001;
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

        let Some((anchor, ndc_z, ppu)) = project_billboard(camera, *pos, screen_w, screen_h)
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
        let z = ndc_z - 0.001;
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
