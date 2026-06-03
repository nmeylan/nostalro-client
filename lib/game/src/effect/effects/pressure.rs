//! A spinning item icon falls from the sky onto a ground point and a coloured
//! shockwave ring expands from the impact. Used by the Slim potion-throw
//! effects (497-499) and, with a `quake`-enabled param, by Pressure (365, a
//! sibling batch).
//!
//! The falling, spinning, fading icon quad carries a 4-slot ghost trail. The
//! flat ground ring is an [`EffectPrimitiveDraw::GroundDisc`] that starts after
//! a delay and grows ×1.07/frame, expanding outward from the impact point.
//! Only plain Pressure (F1 0) shakes the screen; the Slim effects (F1 1/2/3)
//! do not.
//!
//! Texture note: the slim-potion icons are the inventory item icons, stored in
//! the classic GRF under their Korean names (`레드슬림포션` / `옐로우슬림포션`
//! / `화이트슬림포션`). They are already the right colour, so the falling quad
//! is drawn untinted (white) and only the ground ring carries the red/yellow/
//! white grade. Ring texture `bbbb.bmp` is present. The icons stay white so
//! only the ring grades the colour. Reference distances (drop height 80, icon
//! radius 8, ring 15) are downscaled to the gif's ~½-character bottle and
//! ~1-character splash.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{CameraShake, Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &[
    "유저인터페이스/item/레드슬림포션.bmp",
    "유저인터페이스/item/옐로우슬림포션.bmp",
    "유저인터페이스/item/화이트슬림포션.bmp",
    "bbbb.bmp",
];

const RING_TEXTURE: &str = "bbbb.bmp";
const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

const FPS: f32 = 60.0;
const FRAME_DT: f32 = 1.0 / FPS;

/// Spawns at a random angle, then spins -15°/frame.
const SPIN_PER_FRAME_DEG: f32 = -15.0;
/// Icon fall speed (units/frame), downscaled from the reference 3/frame over
/// an 80-unit drop to the gif's short visible fall.
const FALL_SPEED: f32 = 0.8;
/// Icon alpha +20/frame fade-in, -5/frame fade-out after landing.
const ICON_FADE_IN_PER_FRAME: f32 = 20.0 / 255.0;
const ICON_FADE_OUT_PER_FRAME: f32 = 5.0 / 255.0;
const ICON_FADE_IN_UNTIL: i32 = 10;

/// `process = minus = -16` → the ring waits 16 frames (until the icon lands)
/// before expanding.
const RING_DELAY_FRAMES: i32 = 16;
/// Ring radius scales by 1.07 each frame.
const RING_GROWTH_PER_FRAME: f32 = 1.07;
const RING_FADE_IN_FRAMES: f32 = 6.0;
const RING_LIFE_FRAMES: f32 = 30.0;
const RING_PEAK_ALPHA: f32 = 0.8;
const RING_THICKNESS: f32 = 1.5;

const MAX_TOTAL_FRAMES: f32 = 90.0;
pub const PRESSURE_TOTAL_DURATION_MS: u32 = (MAX_TOTAL_FRAMES / FPS * 1000.0) as u32;

const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
const RING_RED: [f32; 3] = [1.0, 0.0, 0.0];
const RING_YELLOW: [f32; 3] = [1.0, 1.0, 0.0];

#[derive(Clone, Copy)]
pub struct PressureParams {
    /// Falling icon texture (already coloured for the slim potions).
    pub icon_texture: &'static str,
    /// Tint applied to the falling icon quad (white = show the icon as-is).
    pub icon_tint: [f32; 3],
    /// Tint applied to the ground ring.
    pub ring_tint: [f32; 3],
    /// Start height above the impact point (negative = up; −Y is up).
    pub drop_y: f32,
    /// Icon quad corner radius (side = `distance * √2`).
    pub icon_distance: f32,
    /// Ring starting radius.
    pub ring_start_radius: f32,
    /// Fire a one-shot camera shake when the icon lands (Pressure only).
    pub quake: bool,
}

// 497/498/499 Slim — the real coloured potion icon drops + a matching ground
// splash ring; no screen shake. Icon drawn untinted (already coloured).
pub const SLIM: PressureParams = PressureParams {
    icon_texture: "유저인터페이스/item/레드슬림포션.bmp",
    icon_tint: WHITE,
    ring_tint: RING_RED,
    drop_y: -18.0,
    icon_distance: 1.2,
    ring_start_radius: 3.0,
    quake: false,
};
pub const SLIM2: PressureParams = PressureParams {
    icon_texture: "유저인터페이스/item/옐로우슬림포션.bmp",
    icon_tint: WHITE,
    ring_tint: RING_YELLOW,
    drop_y: -18.0,
    icon_distance: 1.2,
    ring_start_radius: 3.0,
    quake: false,
};
pub const SLIM3: PressureParams = PressureParams {
    icon_texture: "유저인터페이스/item/화이트슬림포션.bmp",
    icon_tint: WHITE,
    ring_tint: WHITE,
    drop_y: -18.0,
    icon_distance: 1.2,
    ring_start_radius: 3.0,
    quake: false,
};

pub struct PressureEffect {
    params: PressureParams,
    impact: [f32; 3],
    process: i32,
    icon_pos: [f32; 3],
    icon_spin_deg: f32,
    icon_alpha: f32,
    icon_landed: bool,
    icon_done: bool,
    icon_history: Vec<[f32; 3]>,
    shake_fired: bool,
    time_accum: f32,
}

impl PressureEffect {
    pub fn new(impact: [f32; 3], params: PressureParams) -> Self {
        Self {
            params,
            impact,
            process: 0,
            icon_pos: [impact[0], impact[1] + params.drop_y, impact[2]],
            icon_spin_deg: 0.0,
            icon_alpha: 0.0,
            icon_landed: false,
            icon_done: false,
            icon_history: Vec::with_capacity(4),
            shake_fired: false,
            time_accum: 0.0,
        }
    }

    fn ring_age(&self) -> Option<f32> {
        let age = self.process - RING_DELAY_FRAMES;
        (age > 0).then_some(age as f32)
    }

    fn ring_done(&self) -> bool {
        self.ring_age().is_some_and(|age| age >= RING_LIFE_FRAMES)
    }

    fn tick(&mut self) {
        self.process += 1;
        self.icon_spin_deg = (self.icon_spin_deg + SPIN_PER_FRAME_DEG).rem_euclid(360.0);

        if !self.icon_done {
            if !self.icon_landed {
                self.icon_alpha = if self.process <= ICON_FADE_IN_UNTIL {
                    (self.icon_alpha + ICON_FADE_IN_PER_FRAME).min(1.0)
                } else {
                    self.icon_alpha
                };
                // −Y is up: falling toward the impact means y increases to impact[1].
                self.icon_pos[1] += FALL_SPEED;
                if self.icon_pos[1] >= self.impact[1] {
                    self.icon_pos[1] = self.impact[1];
                    self.icon_landed = true;
                }
            } else {
                self.icon_alpha -= ICON_FADE_OUT_PER_FRAME;
                if self.icon_alpha <= 0.0 {
                    self.icon_alpha = 0.0;
                    self.icon_done = true;
                }
            }
            self.icon_history.push(self.icon_pos);
            if self.icon_history.len() > 4 {
                self.icon_history.remove(0);
            }
        }
    }

    fn push_icon(&self, out: &mut EffectDrawList, pos: [f32; 3], alpha: f32) {
        if alpha <= 0.0 {
            return;
        }
        let side = self.params.icon_distance * std::f32::consts::SQRT_2;
        let t = self.params.icon_tint;
        out.push(EffectPrimitiveDraw::Billboard {
            pos,
            size: [side, side],
            uv: UNIT_UV,
            rotation: self.icon_spin_deg.to_radians(),
            texture: self.params.icon_texture,
            color: [t[0], t[1], t[2], alpha],
            blend: BlendKind::Alpha,
        });
    }
}

impl Effect for PressureEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.time_accum += ctx.delta;
        while self.time_accum >= FRAME_DT {
            self.time_accum -= FRAME_DT;
            self.tick();
        }
        if self.icon_done && self.ring_done() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        // Falling icon: lead + a short motion-blur trail.
        if !self.icon_done {
            const TRAIL_ALPHA: [f32; 3] = [0.4, 0.25, 0.1];
            for (k, factor) in TRAIL_ALPHA.iter().enumerate() {
                if let Some(&pos) = self.icon_history.len().checked_sub(2 + k).and_then(|i| self.icon_history.get(i)) {
                    self.push_icon(out, pos, self.icon_alpha * factor);
                }
            }
            self.push_icon(out, self.icon_pos, self.icon_alpha);
        }

        // Expanding ground shockwave ring (after the landing delay).
        if let Some(age) = self.ring_age() {
            if age < RING_LIFE_FRAMES {
                let radius = self.params.ring_start_radius * RING_GROWTH_PER_FRAME.powf(age);
                let alpha = if age < RING_FADE_IN_FRAMES {
                    RING_PEAK_ALPHA * (age / RING_FADE_IN_FRAMES)
                } else {
                    RING_PEAK_ALPHA * (1.0 - (age - RING_FADE_IN_FRAMES) / (RING_LIFE_FRAMES - RING_FADE_IN_FRAMES))
                };
                let t = self.params.ring_tint;
                out.push(EffectPrimitiveDraw::GroundDisc {
                    center: self.impact,
                    radius,
                    thickness: RING_THICKNESS,
                    rotation: 0.0,
                    arc_angle_deg: 360.0,
                    uv_repeat: 1.0,
                    texture: RING_TEXTURE,
                    color: [t[0], t[1], t[2], alpha.max(0.0)],
                    blend: BlendKind::Additive,
                });
            }
        }
    }

    fn take_camera_shake(&mut self) -> Option<CameraShake> {
        if self.params.quake && !self.shake_fired && self.icon_landed {
            self.shake_fired = true;
            Some(CameraShake { amplitude: 1.5, duration_ms: 350 })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut PressureEffect, frames: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..frames {
            s = e.update(&EffectUpdateCtx { delta: FRAME_DT, camera_target: None });
        }
        s
    }

    fn ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn list(e: &PressureEffect) -> Vec<EffectPrimitiveDraw> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &ctx());
        l.primitives
    }

    fn icon_y(e: &PressureEffect) -> Option<f32> {
        list(e).iter().find_map(|p| match p {
            EffectPrimitiveDraw::Billboard { pos, .. } => Some(pos[1]),
            _ => None,
        })
    }

    fn ring(e: &PressureEffect) -> Option<(f32, f32)> {
        list(e).iter().find_map(|p| match p {
            EffectPrimitiveDraw::GroundDisc { radius, color, .. } => Some((*radius, color[3])),
            _ => None,
        })
    }

    #[test]
    fn potion_falls_toward_the_ground() {
        // Impact at the origin; icon spawns drop_y above (−Y up) and descends.
        let mut e = PressureEffect::new([0.0, 0.0, 0.0], SLIM);
        step(&mut e, 1);
        let y0 = icon_y(&e).expect("icon visible");
        step(&mut e, 8);
        let y1 = icon_y(&e).expect("icon visible");
        assert!(y0 < 0.0, "icon starts above the impact (negative Y = up)");
        assert!(y1 > y0, "icon descends toward the impact over time");
    }

    #[test]
    fn ring_appears_after_delay_then_grows_and_fades() {
        let mut e = PressureEffect::new([0.0, 0.0, 0.0], SLIM);
        step(&mut e, RING_DELAY_FRAMES as u32);
        assert!(ring(&e).is_none(), "no ring before the landing delay");
        step(&mut e, 4);
        let (r_early, a_early) = ring(&e).expect("ring after delay");
        step(&mut e, 8);
        let (r_late, _) = ring(&e).expect("ring still expanding");
        assert!(r_late > r_early, "ring radius grows");
        assert!(a_early > 0.0, "ring is visible");
    }

    #[test]
    fn slim_variants_tint_red_yellow_white_and_never_shake() {
        for (params, ring_tint) in [(SLIM, RING_RED), (SLIM2, RING_YELLOW), (SLIM3, WHITE)] {
            let mut e = PressureEffect::new([0.0, 0.0, 0.0], params);
            step(&mut e, RING_DELAY_FRAMES as u32 + 3);
            let has_tinted_ring = list(&e).iter().any(|p| matches!(p,
                EffectPrimitiveDraw::GroundDisc { color, .. }
                    if color[0] == ring_tint[0] && color[1] == ring_tint[1] && color[2] == ring_tint[2]));
            assert!(has_tinted_ring, "ring carries the variant colour");
            assert!(e.take_camera_shake().is_none(), "Slim never shakes the screen");
        }
    }

    #[test]
    fn terminates_after_icon_and_ring_finish() {
        let mut e = PressureEffect::new([0.0, 0.0, 0.0], SLIM);
        let mut status = EffectStatus::Running;
        for _ in 0..(MAX_TOTAL_FRAMES as u32) {
            status = step(&mut e, 1);
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
