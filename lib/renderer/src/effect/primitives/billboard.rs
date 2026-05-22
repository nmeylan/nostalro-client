use crate::camera::Camera;
use crate::effect_sprite::project_billboard;
use crate::sprite::{SpriteBatch, SpriteVertex};

use super::super::{BlendKind, EffectDrawList, EffectPrimitiveDraw};

/// Convert every `EffectPrimitiveDraw::Billboard` entry into a `SpriteBatch`.
///
/// Resolution rules:
///   * Texture: callers resolve names via `texture_lookup` (typically a
///     closure over an App-owned `TextureCache`). Returning `None` falls
///     back to `fallback_texture`. Keeping the cache *out* of this function
///     lets callers control the borrow scope - the renderer's own
///     `TextureCache` field would otherwise hold an immutable borrow on the
///     renderer that prevents calling `Renderer::render(&mut self, …)`.
///   * Blend: `BlendKind::Additive` or `Raw { dst != Zero }` → additive
///     SpriteBatch; everything else → alpha. Full per-frame D3D blend
///     factors land when we drop the SpriteRenderer detour and add a
///     dedicated effect-primitive pipeline.
///   * Size is in world units; converted to screen pixels via `ppu`
///     (perspective-correct).
pub fn build_billboard_batches<'a>(
    list: &EffectDrawList,
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
    fallback_texture: &'a wgpu::BindGroup,
    texture_lookup: impl Fn(&str) -> Option<&'a wgpu::BindGroup>,
) -> Vec<SpriteBatch<'a>> {
    let mut batches = Vec::new();
    for prim in &list.primitives {
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

        let Some((anchor, depth, ppu)) = project_billboard(camera, *pos, screen_w, screen_h)
        else {
            continue;
        };

        let half_w = size[0] * ppu * 0.5;
        let half_h = size[1] * ppu * 0.5;
        // Rotate each corner offset around the anchor by `rotation` radians.
        // CCW in screen space (matches the original game roll convention used by
        // Hit2's lens-flare petals). When `rotation == 0` cos=1, sin=0 and
        // the transform is the identity, so existing axis-aligned callers
        // see no change.
        let (sin_r, cos_r) = rotation.sin_cos();
        let rotate = |dx: f32, dy: f32| -> [f32; 2] {
            [
                anchor[0] + dx * cos_r - dy * sin_r,
                anchor[1] + dx * sin_r + dy * cos_r,
            ]
        };
        // Vertex order: TL, TR, BL, BR; indices form a triangle strip.
        let corners = [
            (rotate(-half_w, -half_h), uv[0]),
            (rotate(half_w, -half_h), uv[1]),
            (rotate(-half_w, half_h), uv[2]),
            (rotate(half_w, half_h), uv[3]),
        ];

        let texture_bg = texture_lookup(texture).unwrap_or(fallback_texture);
        let z = depth - 0.001;
        let vertices = corners
            .iter()
            .map(|(p, t)| SpriteVertex {
                position: [p[0], p[1], z],
                tex_coord: *t,
                color: *color,
            })
            .collect();

        batches.push(SpriteBatch {
            vertices,
            indices: vec![0, 1, 2, 1, 3, 2],
            texture: texture_bg,
            additive: blend_is_additive(blend),
        });
    }
    batches
}

fn blend_is_additive(blend: &BlendKind) -> bool {
    match blend {
        BlendKind::Additive => true,
        BlendKind::Alpha | BlendKind::Multiply => false,
        // Heuristic for raw D3D factors: anything that doesn't write the
        // standard `SrcAlpha, OneMinusSrcAlpha` pair lands in additive.
        BlendKind::Raw { src: _, dst } => *dst != 6,
    }
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
    fn blend_classification() {
        assert!(blend_is_additive(&BlendKind::Additive));
        assert!(!blend_is_additive(&BlendKind::Alpha));
        assert!(!blend_is_additive(&BlendKind::Multiply));
        assert!(!blend_is_additive(&BlendKind::Raw { src: 5, dst: 6 }));
        assert!(blend_is_additive(&BlendKind::Raw { src: 5, dst: 2 }));
    }
}
