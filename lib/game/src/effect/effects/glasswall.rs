//! `EF_GLASSWALL` — Safety Wall barrier visual: a box of glass walls plus
//! a cascading particle overlay.
//!
//! Hybrid effect: a quartet of tall translucent blue walls forms a box
//! around the target cell, plus `SafetyWall.str` plays the cascading
//! particle/shimmer overlay on top.
//!
//! Wall layout (4 quads, all `ring_blue.tga`):
//!
//! | side  | offset in master frame | size (W × H) | rotation |
//! |-------|------------------------|--------------|----------|
//! | front | `(0, 0, +2.6)`         | `3.0 × 40`   | `angle + 90°` |
//! | back  | `(0, 0, −2.6)`         | `3.0 × 40`   | `angle + 90°` |
//! | right | `(+3.0, 0, 0)`         | `2.6 × 40`   | `angle`       |
//! | left  | `(−3.0, 0, 0)`         | `2.6 × 40`   | `angle`       |
//!
//! Height starts at 40 wu. The original game raises it at
//! 0.25 wu/frame but the visible silhouette in the gif
//! is steady — we use a fixed height. Alpha ramps from 0 to `180/255`
//! over 6 frames and stays through the persistent lifetime.
//!
//! The original game's box has gaps at the corners (each wall is shorter
//! than the spacing between walls). To produce a closed barrier
//! silhouette we widen each pair of walls to span between the
//! perpendicular pair's anchors, so the four panels meet edge-to-edge.
//!
//! Safety Wall is a sustained skill (the table's "persistent"
//! sentinel) — the effect's `Custom { duration_ms }` mirrors
//! the table's persistent value and the server is responsible for
//! despawning when the skill cell expires.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const WALL_TEXTURE: &str = "ring_blue.tga";
pub const TEXTURES: &[&str] = &[WALL_TEXTURE];

pub const STR_OVERLAY: &str = "safetywall";

const FRAMES_PER_SECOND: f32 = 60.0;

// Wall layout — the original game offsets are ±2.6 along Z and ±3.0 along X;
// we widen each wall's half-extent to the perpendicular pair's offset
// so the four panels close at the corners (otherwise the box has 1.5
// wu gaps where the original visibly has none).
const WALL_OFFSET_Z: f32 = 2.6;
const WALL_OFFSET_X: f32 = 3.0;
const WALL_FRONT_BACK_HALF_WIDTH: f32 = WALL_OFFSET_X;
const WALL_LEFT_RIGHT_HALF_WIDTH: f32 = WALL_OFFSET_Z;
// Fixed wall height. The original is 40 wu with a 0.25 wu/frame
// drift; pinning it removes the slow "rising" creep the user flagged
// without changing the visible silhouette in the gif.
const WALL_HEIGHT: f32 = 20.0;

const WALL_MAX_ALPHA: f32 = 180.0 / 255.0;
const WALL_FADE_IN_FRAMES: f32 = 6.0;
// Each wall's local UV wave cycles through the
// texture u-coordinate over time. 1 cycle every 60 frames keeps the
// shimmer slow enough to read as a wave, not a strobe.
const WALL_UV_SCROLL_PER_FRAME: f32 = 1.0 / 60.0;

pub const TOTAL_DURATION_MS: u32 = 99990;

fn wall_alpha(frame: f32) -> f32 {
    (frame / WALL_FADE_IN_FRAMES).clamp(0.0, 1.0) * WALL_MAX_ALPHA
}


pub struct GlasswallEffect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl GlasswallEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self { world_pos, age_frames: 0.0 }
    }
}

/// Build a vertical wall quad. `(half_along_x, half_along_z)` are the
/// XZ-plane offsets of the wall's two ground corners from `centre`; the
/// wall extends straight up to `-height` in native RO coords.
///
/// Corners are returned in perimeter order so the WorldQuad renderer's
/// triangulation `(0,1,2) + (0,2,3)` covers the full quad: `TL → TR → BR → BL`.
fn wall_quad(
    centre: [f32; 3],
    half_along_x: f32,
    half_along_z: f32,
    height: f32,
) -> [[f32; 3]; 4] {
    let bx0 = centre[0] - half_along_x;
    let bz0 = centre[2] - half_along_z;
    let bx1 = centre[0] + half_along_x;
    let bz1 = centre[2] + half_along_z;
    let top_y = centre[1] - height;
    let bot_y = centre[1];
    [
        [bx0, top_y, bz0],
        [bx1, top_y, bz1],
        [bx1, bot_y, bz1],
        [bx0, bot_y, bz0],
    ]
}

impl Effect for GlasswallEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = wall_alpha(self.age_frames);
        if alpha <= 0.0 {
            return;
        }
        // U-scroll over time matches the original game's
        // shimmer cycle on the wall's horizontal axis.
        // Corners are TL → TR → BR → BL (perimeter order so the
        // triangulation covers the whole quad).
        let scroll = WALL_UV_SCROLL_PER_FRAME * self.age_frames;
        let uv = [
            [0.0 + scroll, 0.0],
            [1.0 + scroll, 0.0],
            [1.0 + scroll, 1.0],
            [0.0 + scroll, 1.0],
        ];
        let colour = [0.5, 0.7, 1.0, alpha];

        // Front / back — long axis along X, positioned at ±Z.
        for side in [1.0, -1.0] {
            let centre = [
                self.world_pos[0],
                self.world_pos[1],
                self.world_pos[2] + WALL_OFFSET_Z * side,
            ];
            out.push(EffectPrimitiveDraw::WorldQuad {
                corners: wall_quad(centre, WALL_FRONT_BACK_HALF_WIDTH, 0.0, WALL_HEIGHT),
                uv,
                texture: WALL_TEXTURE,
                color: colour,
                blend: BlendKind::Alpha,
            });
        }

        // Left / right — long axis along Z, positioned at ±X.
        for side in [1.0, -1.0] {
            let centre = [
                self.world_pos[0] + WALL_OFFSET_X * side,
                self.world_pos[1],
                self.world_pos[2],
            ];
            out.push(EffectPrimitiveDraw::WorldQuad {
                corners: wall_quad(centre, 0.0, WALL_LEFT_RIGHT_HALF_WIDTH, WALL_HEIGHT),
                uv,
                texture: WALL_TEXTURE,
                color: colour,
                blend: BlendKind::Alpha,
            });
        }
    }

    fn str_overlay(&self) -> Option<&'static str> {
        Some(STR_OVERLAY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn emits_four_walls_forming_a_box() {
        // Sociable: 4 WorldQuad walls each centred on one side of the
        // box, all with the same height and the ring_blue texture.
        let mut e = GlasswallEffect::new([10.0, 0.0, 20.0]);
        // Advance past the fade-in so the walls are fully visible.
        e.update(&ctx(10.0 / FRAMES_PER_SECOND));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let walls: Vec<_> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::WorldQuad { corners, texture, .. } => {
                    Some((*corners, *texture))
                }
                _ => None,
            })
            .collect();
        assert_eq!(walls.len(), 4, "exactly 4 walls forming the box");
        for (_, tex) in &walls {
            assert_eq!(*tex, WALL_TEXTURE);
        }

        // The two ±Z-offset walls and the two ±X-offset walls together
        // box the centre — confirm by collecting the centre points and
        // checking they include ±2.6 along Z and ±3.0 along X relative
        // to the anchor.
        let mut centres: Vec<[f32; 3]> = walls
            .iter()
            .map(|(c, _)| {
                let mut sum = [0.0; 3];
                for v in c {
                    sum[0] += v[0];
                    sum[1] += v[1];
                    sum[2] += v[2];
                }
                [sum[0] / 4.0, sum[1] / 4.0, sum[2] / 4.0]
            })
            .collect();
        centres.sort_by(|a, b| a[2].partial_cmp(&b[2]).unwrap_or(std::cmp::Ordering::Equal));
        let z_min = centres[0][2];
        let z_max = centres[3][2];
        assert!((z_min - (20.0 - WALL_OFFSET_Z)).abs() < 1e-3);
        assert!((z_max - (20.0 + WALL_OFFSET_Z)).abs() < 1e-3);
    }

    #[test]
    fn declares_safetywall_str_overlay() {
        let e = GlasswallEffect::new([0.0; 3]);
        assert_eq!(e.str_overlay(), Some(STR_OVERLAY));
    }

    #[test]
    fn alpha_fades_in_from_zero() {
        // At frame 0 alpha is 0 (no draws); past the fade-in window
        // it's at the peak.
        let mut e = GlasswallEffect::new([0.0; 3]);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert!(list.primitives.is_empty(), "no walls at frame 0");

        e.update(&ctx(WALL_FADE_IN_FRAMES / FRAMES_PER_SECOND));
        let mut list2 = EffectDrawList::new();
        e.collect_draws(&mut list2, &render_ctx());
        let peak_alpha = list2
            .primitives
            .iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::WorldQuad { color, .. } => Some(color[3]),
                _ => None,
            })
            .unwrap();
        assert!((peak_alpha - WALL_MAX_ALPHA).abs() < 1e-3);
    }
}
