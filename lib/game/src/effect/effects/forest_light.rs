//! `EF_ITEM_LIGHT` — `ForestLight("effect\\cloud11.tga", 11)`.
//!
//! The forest primitive draws, per emitter column, a thin pentagonal
//! tube of additive greenish light. Each column has two horizontal pentagon
//! rings of equal radius — a top ring centred on the caster (`vecT_now`) and a
//! bottom ring centred far below-and-aside (`vecT_pre = caster + (-70,-300,-70)`)
//! — joined by five quads. Because both rings stay horizontal while their
//! centres are offset on all three axes, the tube is *sheared*, not tilted, so
//! it's built from raw `WorldQuad` corners (same approach as the Bottom_Light
//! ribbon).
//!
//! The `F1 = 11` (ITEM_LIGHT) variant of the forest light beam:
//!   * four columns are launched, radii `4, 6, 8, 4` with rot start = 25*ec;
//!   * column 3 starts at `process = 180` (= duration), so its alpha never
//!     leaves 0 and it never renders — only columns 0..2 are visible;
//!   * per-frame alpha ramps `0 → 40` over the first 40 frames, holds, then
//!     ramps back to 0 over the last 40 frames (out of 255 — a faint glow);
//!   * columns 1 and 3 breathe their radius by `±0.5` on a slow sine.
//!
//! Colour is `(230, 255, 230)` additive.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
/// `SET_DURATION(180)` — the alpha ramp keys off this exact frame count.
const DURATION_FRAMES: u32 = 180;
const ALPHA_RAMP_FRAMES: u32 = 40;
const ALPHA_FADE_START: u32 = 140;
/// Peak alpha reached at the end of the fade-in (out of 255).
const PEAK_ALPHA: f32 = ALPHA_RAMP_FRAMES as f32;
/// Pentagon: angle samples every 72°, six points closing the loop → five quads.
const SEGMENTS: usize = 5;
/// `vecT_pre = caster + (-70, -300, -70)` — the far ring centre. Native RO
/// coords (-Y up), so `-300` on Y lifts it well above the caster.
const BOTTOM_OFFSET: [f32; 3] = [-70.0, -300.0, -70.0];

#[derive(Clone, Copy, Debug)]
pub struct ForestLightParams {
    pub texture: &'static str,
    /// RGB tint (0..1) from the textured-rect render's `(r,g,b)` args.
    pub color_rgb: [f32; 3],
}

/// `EF_ITEM_LIGHT` → `ForestLight("effect\\cloud11.tga", 11)`, colour
/// `(230, 255, 230)`.
pub const ITEM_LIGHT: ForestLightParams = ForestLightParams {
    texture: "cloud11.tga",
    color_rgb: [230.0 / 255.0, 1.0, 230.0 / 255.0],
};

pub const TEXTURES: &[&str] = &["cloud11.tga"];

/// Per-column constants for the `F1 = 11` launch.
struct Column {
    /// Base radius (`max_height` in the original).
    radius: f32,
    rot_start_deg: f32,
    /// Frame the column's `process` counter starts at.
    process_start: u32,
    /// Columns 1 and 3 breathe their radius on a slow sine.
    breathes: bool,
}

const COLUMNS: [Column; 4] = [
    Column { radius: 4.0, rot_start_deg: 0.0, process_start: 0, breathes: false },
    Column { radius: 6.0, rot_start_deg: 25.0, process_start: 0, breathes: true },
    Column { radius: 8.0, rot_start_deg: 50.0, process_start: 0, breathes: false },
    Column { radius: 4.0, rot_start_deg: 75.0, process_start: DURATION_FRAMES, breathes: true },
];

pub struct ForestLightEffect {
    world_pos: [f32; 3],
    params: ForestLightParams,
    age: f32,
    frames: u32,
}

impl ForestLightEffect {
    pub fn new(world_pos: [f32; 3], params: ForestLightParams) -> Self {
        Self { world_pos, params, age: 0.0, frames: 0 }
    }
}

/// start alpha for a column whose process is at `frame`: ramp up over the first
/// 40 frames, hold, ramp back down over the last 40. Clamped to `>= 0`.
fn column_alpha(process: u32) -> f32 {
    if process <= ALPHA_RAMP_FRAMES {
        process as f32
    } else if process > ALPHA_FADE_START {
        (PEAK_ALPHA - (process - ALPHA_FADE_START) as f32).max(0.0)
    } else {
        PEAK_ALPHA
    }
}

/// Radius for a column at `process`, with the `±0.5` sine breathing applied to
/// the breathing columns. Matches `sinp = (process % 720) / 2`.
fn column_radius(col: &Column, process: u32) -> f32 {
    if col.breathes {
        let sinp = (process % 720) as f32 * 0.5;
        col.radius + sinp.to_radians().sin() * 0.5
    } else {
        col.radius
    }
}

impl Effect for ForestLightEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        self.frames = (self.age * FRAMES_PER_SECOND) as u32;
        if self.frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [tr, tg, tb] = self.params.color_rgb;
        let top_center = self.world_pos;
        let bottom_center = [
            self.world_pos[0] + BOTTOM_OFFSET[0],
            self.world_pos[1] + BOTTOM_OFFSET[1],
            self.world_pos[2] + BOTTOM_OFFSET[2],
        ];

        for col in &COLUMNS {
            let process = self.frames + col.process_start;
            let alpha = column_alpha(process);
            if alpha <= 0.0 {
                continue;
            }
            let radius = column_radius(col, process);

            // Six angle samples (0,72,..,360) → the last closes the pentagon.
            let ring_point = |center: [f32; 3], i: usize| {
                let angle_deg = (i as f32 * 72.0 + col.rot_start_deg) % 360.0;
                let (s, c) = angle_deg.to_radians().sin_cos();
                [center[0] + radius * c, center[1], center[2] + radius * s]
            };

            for i in 1..=SEGMENTS {
                let prev_top = ring_point(top_center, i - 1);
                let cur_top = ring_point(top_center, i);
                let cur_bottom = ring_point(bottom_center, i);
                let prev_bottom = ring_point(bottom_center, i - 1);
                out.push(EffectPrimitiveDraw::WorldQuad {
                    corners: [prev_top, cur_top, cur_bottom, prev_bottom],
                    uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    texture: self.params.texture,
                    color: [tr, tg, tb, alpha / 255.0],
                    blend: BlendKind::Additive,
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

    fn draws_after(secs: f32) -> Vec<EffectPrimitiveDraw> {
        let mut e = ForestLightEffect::new([10.0, 0.0, 20.0], ITEM_LIGHT);
        e.update(&EffectUpdateCtx { delta: secs, camera_target: None });
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn item_light_emits_three_additive_columns_that_fade_and_die() {
        // Sociable: mid-life it draws three visible pentagon tubes (column 3 is
        // suppressed by its process_start), each a five-quad additive ribbon of
        // greenish light anchored above the caster's XZ column.
        let prims = draws_after(1.0); // frame 60 — past fade-in, full hold alpha.
        assert_eq!(prims.len(), 3 * SEGMENTS, "3 visible columns × 5 quads");

        for p in &prims {
            let EffectPrimitiveDraw::WorldQuad { corners, color, blend, texture, .. } = p else {
                panic!("expected WorldQuad, got {p:?}");
            };
            assert_eq!(*blend, BlendKind::Additive);
            assert_eq!(*texture, "cloud11.tga");
            // Greenish: G is the brightest channel.
            assert!(color[1] >= color[0] && color[1] >= color[2]);
            // Hold-phase alpha is the 40/255 peak.
            assert!((color[3] - PEAK_ALPHA / 255.0).abs() < 1e-3, "hold alpha");
            // The bottom ring sits well above the top ring (native -Y up): the
            // tube leans up and aside, not flat.
            let top_y = corners[0][1];
            let bottom_y = corners[3][1];
            assert!(top_y - bottom_y > 100.0, "tube spans the vertical offset");
        }

        // Fade-in: at frame 10 the alpha is a tenth of the hold value.
        let early = draws_after(10.0 / 60.0);
        let EffectPrimitiveDraw::WorldQuad { color, .. } = &early[0] else { panic!() };
        assert!(color[3] < PEAK_ALPHA / 255.0, "still fading in");

        // Self-terminates once the 180-frame duration elapses.
        let mut e = ForestLightEffect::new([0.0; 3], ITEM_LIGHT);
        let mut status = EffectStatus::Running;
        for _ in 0..DURATION_FRAMES + 5 {
            status = e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None });
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
