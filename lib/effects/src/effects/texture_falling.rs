//! `EF_TEXTURE_FALLING` — id 1031.
//!
//! Drops a single camera-facing
//! `sword.bmp` billboard from above onto the target, leaving a short fading
//! motion trail behind it. One lead emitter plus
//! three "ghost" slots: each frame slot N+1 copies slot N's position with a
//! reduced alpha (−100, −25, −25 of 255), so the four stamps read as a
//! comet-like streak. The lead fades in over the first 10 frames, falls for
//! ~32 frames, then fades out.
//!
//! Modeled here as a small param-set struct: one falling lead whose recent
//! positions are kept in a short history and re-drawn with decaying alpha.
//! The original also shakes the camera on impact — not a
//! rendered primitive, so it is out of scope here.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

/// Number of trail stamps (lead + 3 ghosts), matching the original's 4
/// trail slots.
const TRAIL_LEN: usize = 4;
/// Alpha drop from each stamp to the next (`−100, −25, −25` of 255).
const TRAIL_ALPHA_DROP: [f32; TRAIL_LEN] = [0.0, 100.0 / 255.0, 125.0 / 255.0, 150.0 / 255.0];

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub texture: &'static str,
    /// Height above the anchor the lead starts at (world units; it descends
    /// to the anchor plane). Native RO has −Y up, so the start sits at
    /// `anchor.y − start_height`.
    pub start_height: f32,
    /// Descent per frame (world units), applied for `fall_frames`.
    pub fall_speed: f32,
    pub fall_frames: f32,
    pub size: f32,
    /// Constant texture roll (degrees); the original holds a fixed
    /// 45° tilt.
    pub rotation_deg: f32,
    pub color: [f32; 3],
    pub peak_alpha: f32,
    pub fade_in_frames: f32,
    /// Alpha lost per frame once the lead has finished falling (`−5` of 255).
    pub fade_out_rate: f32,
}

pub const TEXTURE_FALLING: Params = Params {
    texture: "sword.bmp",
    start_height: 12.0,
    fall_speed: 0.31,
    fall_frames: 32.0,
    size: 6.0,
    rotation_deg: 45.0,
    color: [1.0, 1.0, 1.0],
    peak_alpha: 200.0 / 255.0,
    fade_in_frames: 10.0,
    fade_out_rate: 5.0 / 255.0,
};

pub const TEXTURES: &[&str] = &[TEXTURE_FALLING.texture];

/// Wall-clock end: fade-in/fall (~32 frames) + the fade-out tail
/// (`peak_alpha / fade_out_rate`).
pub const fn total_duration_ms(p: &Params) -> u32 {
    let fade_out = p.peak_alpha / p.fade_out_rate;
    (((p.fall_frames + fade_out) / FPS) * 1000.0) as u32
}

pub struct FallingTrailEffect {
    anchor: [f32; 3],
    params: Params,
    age_frames: f32,
    /// Most-recent-first ring of recent lead positions.
    history: Vec<[f32; 3]>,
}

impl FallingTrailEffect {
    pub fn new(world_pos: [f32; 3], params: Params) -> Self {
        Self {
            anchor: world_pos,
            params,
            age_frames: 0.0,
            history: Vec::with_capacity(TRAIL_LEN),
        }
    }

    fn lead_pos(&self) -> [f32; 3] {
        let fallen = (self.age_frames * self.params.fall_speed)
            .min(self.params.fall_frames * self.params.fall_speed);
        [
            self.anchor[0],
            self.anchor[1] - self.params.start_height + fallen,
            self.anchor[2],
        ]
    }

    fn lead_alpha(&self) -> f32 {
        if self.age_frames < self.params.fade_in_frames {
            self.params.peak_alpha * (self.age_frames / self.params.fade_in_frames)
        } else if self.age_frames <= self.params.fall_frames {
            self.params.peak_alpha
        } else {
            (self.params.peak_alpha
                - (self.age_frames - self.params.fall_frames) * self.params.fade_out_rate)
                .max(0.0)
        }
    }

    fn total_frames(&self) -> f32 {
        self.params.fall_frames + self.params.peak_alpha / self.params.fade_out_rate
    }
}

impl Effect for FallingTrailEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        self.history.insert(0, self.lead_pos());
        self.history.truncate(TRAIL_LEN);
        if self.age_frames >= self.total_frames() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let lead_alpha = self.lead_alpha();
        if lead_alpha <= 0.0 {
            return;
        }
        let rotation = self.params.rotation_deg.to_radians();
        let [r, g, b] = self.params.color;
        for (i, pos) in self.history.iter().enumerate() {
            let alpha = (lead_alpha - TRAIL_ALPHA_DROP[i]).max(0.0);
            if alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos: *pos,
                size: [self.params.size, self.params.size],
                uv: UNIT_UV,
                rotation,
                texture: self.params.texture,
                color: [r, g, b, alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut FallingTrailEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None, caster_yaw: None,
        })
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn draws(e: &FallingTrailEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn ys(prims: &[EffectPrimitiveDraw]) -> Vec<f32> {
        prims
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard { pos, .. } => pos[1],
                other => panic!("expected Billboard, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn lead_descends_from_above_toward_the_anchor() {
        // Native RO: −Y is up, so the lead starts above (smaller y) and its y
        // increases toward the anchor as it falls.
        let mut e = FallingTrailEffect::new([3.0, 0.0, 5.0], TEXTURE_FALLING);
        step(&mut e, 1.0);
        let y_early = ys(&draws(&e))[0];
        step(&mut e, 20.0);
        let y_late = ys(&draws(&e))[0];
        assert!(y_early < 0.0, "starts above the anchor: {y_early}");
        assert!(y_late > y_early, "descends over time: {y_early} -> {y_late}");
    }

    #[test]
    fn builds_a_fading_trail_of_up_to_four_stamps() {
        // After a few frames the history fills to 4 stamps, each dimmer than
        // the last (additive comet streak).
        let mut e = FallingTrailEffect::new([0.0, 0.0, 0.0], TEXTURE_FALLING);
        // One stamp is recorded per frame-tick update; drive 12 single-frame
        // ticks (past fade-in) so the history fills to its 4-stamp cap.
        for _ in 0..12 {
            step(&mut e, 1.0);
        }
        let prims = draws(&e);
        assert_eq!(prims.len(), TRAIL_LEN, "lead + 3 ghosts");
        let alphas: Vec<f32> = prims
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard { color, .. } => color[3],
                _ => unreachable!(),
            })
            .collect();
        for w in alphas.windows(2) {
            assert!(w[0] > w[1], "trail alpha decays: {alphas:?}");
        }
    }

    #[test]
    fn fades_in_then_out_and_dies() {
        let mut e = FallingTrailEffect::new([0.0, 0.0, 0.0], TEXTURE_FALLING);
        step(&mut e, 1.0);
        let a_in = e.lead_alpha();
        step(&mut e, TEXTURE_FALLING.fade_in_frames);
        let a_peak = e.lead_alpha();
        assert!(a_in < a_peak, "fades in: {a_in} -> {a_peak}");
        assert!((a_peak - TEXTURE_FALLING.peak_alpha).abs() < 1e-3);

        let total = e.total_frames();
        let status = step(&mut e, total);
        assert_eq!(status, EffectStatus::Dead);
        assert!(e.lead_alpha() <= 0.0, "fully faded by death");
    }
}
