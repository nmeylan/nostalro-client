//! `EF_SAINTWING` (id 345) — the angel wings that unfurl behind the caster.
//!
//! The wings use `wing003.bmp` with
//! two emitters (one per wing). Each wing is built from **10
//! feather quads**: a root edge spread `±65°` at radius `distance = 2.0`, each
//! feather reaching out along the wing heading by `Rx = cos(add)·Rz` and up by
//! `Ry = sin(add)·Rz`. The feathers split into two banks — the lower five
//! (`add = 3·i + flap`) flap with `flap =
//! 20·sin(rise) + 20`, the upper five sit fixed at `45..57°`. Per feather the
//! alpha steps up (`·0.7` per rank) toward a bright tip every fifth feather,
//! and the colour ramps blue `(105,105,255)` → white `(205,205,255)`.
//! The flap phase advances `+1°/frame` (the slow flap) and points
//! the two wings `roty + 135°` / `roty + 45°` (behind the caster).
//!
//! Persistent effect. No reference gif — validated against observed
//! original-game behaviour.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
/// Persistent (effectively unbounded); clamps to 5 s in the exporter.
pub const TOTAL_DURATION_MS: u32 = 99990;

pub const TEXTURES: &[&str] = &["wing003.bmp"];

/// Small source literals → ~0.7× (a sprite is ~5–8 world units; the wings
/// stand a little taller).
const WORLD_SCALE: f32 = 0.7;
const DISTANCE: f32 = 2.0 * WORLD_SCALE;
const MAX_HEIGHT: f32 = 10.0 * WORLD_SCALE;
/// Wing-root height above the caster's feet (native `-Y = up`).
const Y_OFFSET: f32 = -9.0 * WORLD_SCALE;
const FEATHERS_PER_WING: usize = 10;

/// Wing heading = `roty + 135°` (wing 0) / `roty + 45°` (wing 1).
const WING_HEADING_DEG: [f32; 2] = [135.0, 45.0];

const ALPHA_BASE: f32 = 200.0 / 255.0;
const COLOR_DIM: [f32; 3] = [105.0 / 255.0, 105.0 / 255.0, 1.0];
const COLOR_BRIGHT: [f32; 3] = [205.0 / 255.0, 205.0 / 255.0, 1.0];

pub struct SaintwingEffect {
    world_pos: [f32; 3],
    /// Caster heading (degrees) captured from the update context.
    caster_yaw_deg: f32,
    /// Slow flap phase, advances 1°/frame.
    rise_angle: f32,
}

impl SaintwingEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self { world_pos, caster_yaw_deg: 0.0, rise_angle: 0.0 }
    }
}

fn polar(radius: f32, angle_deg: f32) -> (f32, f32) {
    let a = angle_deg.to_radians();
    (radius * a.cos(), radius * a.sin())
}

impl Effect for SaintwingEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        if let Some(yaw) = ctx.caster_yaw {
            self.caster_yaw_deg = yaw.to_degrees();
        }
        self.rise_angle = (self.rise_angle + ctx.delta * FRAMES_PER_SECOND) % 360.0;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [cx, cy, cz] = self.world_pos;
        let flap = 20.0 * self.rise_angle.to_radians().sin() + 20.0;
        // Upper feathers grow a hair as the wing flaps.
        let tip_grow = (0.5 * self.rise_angle.to_radians().sin() + 0.5) * WORLD_SCALE;

        for (wing, &heading) in WING_HEADING_DEG.iter().enumerate() {
            let rot = self.caster_yaw_deg + heading;
            let (l_x, l_z) = polar(DISTANCE, rot - 65.0);
            let (r_x, r_z) = polar(DISTANCE, rot + 65.0);
            let root_l = [cx + l_x, cy + Y_OFFSET, cz + l_z];
            let root_r = [cx + r_x, cy + Y_OFFSET, cz + r_z];
            let (heading_cos, heading_sin) = {
                let a = rot.to_radians();
                (a.cos(), a.sin())
            };

            for i in 0..FEATHERS_PER_WING {
                let bank = i / 5;
                let rank = i % 5;
                let rz = if bank == 0 { MAX_HEIGHT } else { MAX_HEIGHT + tip_grow };
                let add = if bank == 0 {
                    rank as f32 * 3.0 + flap
                } else {
                    45.0 + rank as f32 * 3.0
                };
                let add_rad = add.to_radians();
                let reach = add_rad.cos() * rz;
                let lift = add_rad.sin() * rz;
                let off = [heading_cos * reach, -lift, heading_sin * reach];
                let tip_l = [root_l[0] + off[0], root_l[1] + off[1], root_l[2] + off[2]];
                let tip_r = [root_r[0] + off[0], root_r[1] + off[1], root_r[2] + off[2]];

                let (color, alpha) = if rank < 4 {
                    // Dim coverts: alpha steps up toward the bright tip.
                    let a = ALPHA_BASE * 0.7f32.powi((4 - rank) as i32);
                    (COLOR_DIM, a)
                } else {
                    (COLOR_BRIGHT, ALPHA_BASE)
                };
                let [r, g, b] = color;
                // Mirror winding between the two wings.
                let corners = if wing == 0 {
                    [root_r, tip_r, tip_l, root_l]
                } else {
                    [root_l, tip_l, tip_r, root_r]
                };
                out.push(EffectPrimitiveDraw::WorldQuad {
                    corners,
                    uv: [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
                    texture: "wing003.bmp",
                    color: [r, g, b, alpha],
                    blend: BlendKind::Additive,
                    no_depth: false,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn tick(e: &mut SaintwingEffect, frames: u32) {
        for _ in 0..frames {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
    }

    fn quads(e: &SaintwingEffect) -> Vec<EffectPrimitiveDraw> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        l.primitives
    }

    #[test]
    fn emits_twenty_feather_quads() {
        let e = SaintwingEffect::new([0.0; 3]);
        let q = quads(&e);
        assert_eq!(q.len(), FEATHERS_PER_WING * 2);
        assert!(q.iter().all(|p| matches!(p, EffectPrimitiveDraw::WorldQuad { .. })));
    }

    #[test]
    fn alpha_ramps_toward_bright_tip_feathers() {
        let e = SaintwingEffect::new([0.0; 3]);
        let alphas: Vec<f32> = quads(&e).iter().filter_map(|p| match p {
            EffectPrimitiveDraw::WorldQuad { color, .. } => Some(color[3]),
            _ => None,
        }).collect();
        // Within the first bank, alpha increases toward the bright 5th feather.
        assert!(alphas[0] < alphas[3], "coverts dim → bright ({} < {})", alphas[0], alphas[3]);
        assert!(alphas[4] >= alphas[3], "tip feather is brightest");
    }

    #[test]
    fn flap_animates_lower_feather_geometry() {
        let mut e = SaintwingEffect::new([0.0; 3]);
        let before = quads(&e);
        tick(&mut e, 30); // quarter of the rise_angle cycle
        let after = quads(&e);
        // The lower (flapping) feather's tip moves between frames.
        let tip = |v: &[EffectPrimitiveDraw], idx: usize| match &v[idx] {
            EffectPrimitiveDraw::WorldQuad { corners, .. } => corners[1],
            _ => panic!(),
        };
        let a = tip(&before, 0);
        let b = tip(&after, 0);
        assert!((a[0] - b[0]).abs() + (a[1] - b[1]).abs() > 1e-4, "feather flaps");
    }

    #[test]
    fn never_self_terminates() {
        let mut e = SaintwingEffect::new([0.0; 3]);
        for _ in 0..120 {
            assert_eq!(
                e.update(&EffectUpdateCtx { delta: 0.1, camera_target: None, caster_yaw: None }),
                EffectStatus::Running
            );
        }
    }
}
