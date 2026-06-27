use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

pub const TEXTURE: &str = "magic_blue.tga";
pub const RING_BLUE: &str = "ring_blue.tga";

pub const TEXTURES: &[&str] = &[TEXTURE, RING_BLUE];

const RADIUS_INIT: f32 = 1.0;
const RADIUS_PER_FRAME: f32 = 1.0;
const DISTANCE_INIT: f32 = 2.0;
const ALPHA_PER_FRAME: f32 = 5.0;
const ALPHA_FADE_PER_FRAME: f32 = 2.0;
const FADE_START_RADIUS: f32 = 10.0;
const ALPHA_DIVISOR: f32 = 255.0;

const EMITTERS_PER_SET: usize = 4;
const SLICES: usize = 3;
const SLICE_STEP_DEG: f32 = 1.0;

const WORLD_SCALE: f32 = 1.0;

#[derive(Clone, Copy, Debug)]
pub struct SlashParams {
    pub emitter_sets: &'static [f32],
    pub distance_per_frame: f32,
    pub max_height_init: f32,
    pub max_height_cap: f32,
    pub max_height_per_frame: f32,
    pub alpha_rise_frames: f32,
    pub texture: &'static str,
}

impl SlashParams {
    fn peak_alpha(&self) -> f32 {
        ALPHA_PER_FRAME * self.alpha_rise_frames
    }
    fn fade_start_frame(&self) -> f32 {
        (FADE_START_RADIUS - RADIUS_INIT) / RADIUS_PER_FRAME
    }
    fn life_end_frame(&self) -> f32 {
        self.fade_start_frame() + self.peak_alpha() / ALPHA_FADE_PER_FRAME
    }
    pub fn total_duration_ms(&self) -> u32 {
        (self.life_end_frame() / FRAMES_PER_SECOND * 1000.0).ceil() as u32
    }
}

pub const KAIZEL: SlashParams = SlashParams {
    emitter_sets: &[0.0, 45.0],
    distance_per_frame: 2.0,
    max_height_init: 1.5,
    max_height_cap: 4.0,
    max_height_per_frame: 0.4,
    alpha_rise_frames: 7.0,
    texture: TEXTURE,
};

pub const SUPERANGEL_RING: SlashParams = SlashParams {
    emitter_sets: &[0.0, 45.0],
    distance_per_frame: 2.0,
    max_height_init: 1.5,
    max_height_cap: 4.0,
    max_height_per_frame: 0.4,
    alpha_rise_frames: 7.0,
    texture: RING_BLUE,
};

pub const STOPEFFECT: SlashParams = SlashParams {
    emitter_sets: &[0.0, 45.0],
    distance_per_frame: 1.0,
    max_height_init: 0.0,
    max_height_cap: 2.0,
    max_height_per_frame: 0.2,
    alpha_rise_frames: 5.0,
    texture: TEXTURE,
};

pub const TOTAL_DURATION_MS: u32 = 450;
pub const STOPEFFECT_DURATION_MS: u32 = 359;

pub struct SlashEffect {
    params: SlashParams,
    world_pos: [f32; 3],
    age_frames: f32,
}

impl SlashEffect {
    pub fn new(world_pos: [f32; 3], params: SlashParams) -> Self {
        Self {
            params,
            world_pos,
            age_frames: 0.0,
        }
    }

    fn radius(&self) -> f32 {
        RADIUS_INIT + RADIUS_PER_FRAME * self.age_frames
    }
    fn distance(&self) -> f32 {
        DISTANCE_INIT + self.params.distance_per_frame * self.age_frames
    }
    fn max_height(&self) -> f32 {
        (self.params.max_height_init + self.params.max_height_per_frame * self.age_frames)
            .min(self.params.max_height_cap)
    }
    fn alpha_b(&self) -> f32 {
        let t = self.age_frames;
        let rise = (ALPHA_PER_FRAME * t).min(self.params.peak_alpha());
        let fade = if t > self.params.fade_start_frame() {
            ALPHA_FADE_PER_FRAME * (t - self.params.fade_start_frame())
        } else {
            0.0
        };
        (rise - fade).max(0.0)
    }
}

impl Effect for SlashEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= self.params.life_end_frame() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha_b = self.alpha_b();
        if alpha_b <= 0.0 {
            return;
        }
        let [wx, wy, wz] = self.world_pos;
        let r_in = self.radius() * WORLD_SCALE;
        let r_out = (self.radius() + self.distance()) * WORLD_SCALE;
        let mh = self.max_height() * WORLD_SCALE;

        for &base in self.params.emitter_sets {
            for ec in 0..EMITTERS_PER_SET {
                let rot_start = base + ec as f32 * 90.0;
                for i in 0..SLICES {
                    let angle =
                        (rot_start - SLICE_STEP_DEG + i as f32 * SLICE_STEP_DEG).to_radians();
                    let (sn, cs) = angle.sin_cos();
                    let mid = i == 1;
                    // Top-edge lift (native −Y up): the middle slice is tallest.
                    let top_inner = if mid { -mh * 0.2 } else { -mh * 0.1 };
                    let top_outer = if mid { -mh } else { -mh * 0.4 };
                    let alpha = if mid { alpha_b } else { alpha_b / 3.0 } / ALPHA_DIVISOR;

                    let inner_bottom = [wx + cs * r_in, wy, wz + sn * r_in];
                    let inner_top = [wx + cs * r_in, wy + top_inner, wz + sn * r_in];
                    let outer_bottom = [wx + cs * r_out, wy, wz + sn * r_out];
                    let outer_top = [wx + cs * r_out, wy + top_outer, wz + sn * r_out];

                    out.push(EffectPrimitiveDraw::WorldQuad {
                        corners: [inner_bottom, outer_bottom, outer_top, inner_top],
                        uv: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
                        texture: self.params.texture,
                        color: [1.0, 1.0, 1.0, alpha],
                        blend: BlendKind::Additive,
                        no_depth: false,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step(e: &mut SlashEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn draws(e: &SlashEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn quad(p: &EffectPrimitiveDraw) -> ([[f32; 3]; 4], [f32; 4]) {
        match p {
            EffectPrimitiveDraw::WorldQuad {
                corners,
                color,
                texture,
                blend,
                ..
            } => {
                assert_eq!(*texture, TEXTURE);
                assert_eq!(*blend, BlendKind::Additive);
                (*corners, *color)
            }
            other => panic!("expected WorldQuad, got {other:?}"),
        }
    }

    #[test]
    fn eight_blades_three_slices_each() {
        let mut e = SlashEffect::new([0.0; 3], KAIZEL);
        step(&mut e, 2.0);
        assert_eq!(draws(&e).len(), 2 * EMITTERS_PER_SET * SLICES);
    }

    #[test]
    fn blade_grows_outward_and_rises() {
        let mut e = SlashEffect::new([0.0; 3], KAIZEL);
        step(&mut e, 2.0);
        let early = quad(&draws(&e)[1]).0;
        let early_out = (early[1][0].powi(2) + early[1][2].powi(2)).sqrt();
        step(&mut e, 6.0);
        let late = quad(&draws(&e)[1]).0;
        let late_out = (late[1][0].powi(2) + late[1][2].powi(2)).sqrt();
        assert!(
            late_out > early_out,
            "blade flies outward: {early_out} -> {late_out}"
        );
        assert!(late[2][1] < late[1][1], "outer edge rises");
    }

    #[test]
    fn alpha_ramps_in_then_out_and_effect_dies() {
        let mut e = SlashEffect::new([0.0; 3], KAIZEL);
        step(&mut e, 1.0);
        let a_early = quad(&draws(&e)[1]).1[3];
        step(&mut e, 6.0);
        let a_peak = quad(&draws(&e)[1]).1[3];
        assert!(a_peak > a_early, "fades in: {a_early} -> {a_peak}");

        let mut status = EffectStatus::Running;
        for _ in 0..60 {
            status = step(&mut e, 1.0);
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn stopeffect_starts_flat_and_grows_lift() {
        let mut e = SlashEffect::new([0.0; 3], STOPEFFECT);
        step(&mut e, 1.0);
        let early = quad(&draws(&e)[1]).0;
        let early_lift = (early[1][1] - early[2][1]).abs();
        step(&mut e, 6.0);
        let late = quad(&draws(&e)[1]).0;
        let late_lift = (late[1][1] - late[2][1]).abs();
        assert!(
            late_lift > early_lift,
            "lift grows in: {early_lift} -> {late_lift}"
        );
        assert_eq!(draws(&e).len(), 2 * EMITTERS_PER_SET * SLICES);
    }

    #[test]
    fn stopeffect_duration_const_matches_computed() {
        assert_eq!(STOPEFFECT.total_duration_ms(), STOPEFFECT_DURATION_MS);
    }

    #[test]
    fn middle_slice_brighter_than_outer_slices() {
        let mut e = SlashEffect::new([0.0; 3], KAIZEL);
        step(&mut e, 4.0);
        let prims = draws(&e);
        let outer0 = quad(&prims[0]).1[3];
        let middle = quad(&prims[1]).1[3];
        let outer2 = quad(&prims[2]).1[3];
        assert!(
            middle > outer0 && middle > outer2,
            "middle slice is the bright one"
        );
    }
}
