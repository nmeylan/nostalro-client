use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{CameraShake, Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &[
    "유저인터페이스/item/레드슬림포션.bmp",
    "유저인터페이스/item/옐로우슬림포션.bmp",
    "유저인터페이스/item/화이트슬림포션.bmp",
    "bbbb.bmp",
    "cross_old.bmp",
    "explosive_1_128.bmp",
];

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

const FPS: f32 = 60.0;
const FRAME_DT: f32 = 1.0 / FPS;

const SPIN_PER_FRAME_DEG: f32 = -15.0;
const FALL_SPEED: f32 = 3.0;
const ICON_FADE_IN_PER_FRAME: f32 = 20.0 / 255.0;
const ICON_FADE_OUT_PER_FRAME: f32 = 5.0 / 255.0;
const ICON_FADE_IN_UNTIL: i32 = 10;
/// Every variant holds its landed pose until this frame, then fades.
const ICON_ACTIVE_FRAMES: i32 = 32;

/// The four corners of the shockwave sit at `0.9 ×` its distance, which grows
/// 7% a frame from `RING_START_DISTANCE`.
const RING_GROWTH_PER_FRAME: f32 = 1.07;
const RING_CORNER_FACTOR: f32 = 0.9;
const RING_START_DISTANCE: f32 = 15.0;
const RING_Y_OFFSET: f32 = -5.0;
const RING_FADE_IN_FRAMES: f32 = 10.0;
const RING_LIFE_FRAMES: f32 = 30.0;
const RING_PEAK_ALPHA: f32 = 20.0 / 255.0;

const MAX_TOTAL_FRAMES: f32 = 90.0;
pub const PRESSURE_TOTAL_DURATION_MS: u32 = (MAX_TOTAL_FRAMES / FPS * 1000.0) as u32;

/// The cross stops falling and the ground shakes one frame later.
pub const PROJECTILE_FLIGHT: crate::effect_queue::ProjectileFlight =
    crate::effect_queue::ProjectileFlight::FixedFrames(PRESSURE.fall_frames as f32 + 1.0);

const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
const RING_RED: [f32; 3] = [1.0, 0.0, 0.0];
const RING_YELLOW: [f32; 3] = [1.0, 1.0, 0.0];

#[derive(Clone, Copy)]
pub struct PressureParams {
    pub icon_texture: &'static str,
    pub icon_tint: [f32; 3],
    pub ring_tint: [f32; 3],
    /// Start height above the impact point (negative = up; −Y is up).
    pub drop_y: f32,
    pub icon_distance: f32,
    /// How long the icon descends and spins. It stops short of the ground and
    /// holds there until [`ICON_ACTIVE_FRAMES`] rather than landing.
    pub fall_frames: i32,
    pub ring_texture: &'static str,
    pub ring_delay_frames: i32,
    /// The red potion stacks the shockwave quad on itself.
    pub ring_draws: u32,
    pub quake: bool,
}

pub const SLIM: PressureParams = PressureParams {
    icon_texture: "유저인터페이스/item/레드슬림포션.bmp",
    icon_tint: WHITE,
    ring_tint: RING_RED,
    drop_y: -80.0,
    icon_distance: 8.0,
    fall_frames: 24,
    ring_texture: "bbbb.bmp",
    ring_delay_frames: 16,
    ring_draws: 2,
    quake: false,
};
pub const SLIM2: PressureParams = PressureParams {
    icon_texture: "유저인터페이스/item/옐로우슬림포션.bmp",
    icon_tint: WHITE,
    ring_tint: RING_YELLOW,
    drop_y: -80.0,
    icon_distance: 8.0,
    fall_frames: 24,
    ring_texture: "bbbb.bmp",
    ring_delay_frames: 16,
    ring_draws: 1,
    quake: false,
};
pub const SLIM3: PressureParams = PressureParams {
    icon_texture: "유저인터페이스/item/화이트슬림포션.bmp",
    icon_tint: WHITE,
    ring_tint: WHITE,
    drop_y: -80.0,
    icon_distance: 8.0,
    fall_frames: 24,
    ring_texture: "bbbb.bmp",
    ring_delay_frames: 16,
    ring_draws: 1,
    quake: false,
};

pub const PRESSURE: PressureParams = PressureParams {
    icon_texture: "cross_old.bmp",
    icon_tint: WHITE,
    ring_tint: WHITE,
    drop_y: -115.0,
    icon_distance: 12.0,
    fall_frames: 32,
    ring_texture: "explosive_1_128.bmp",
    ring_delay_frames: 28,
    ring_draws: 1,
    quake: true,
};

struct IconGhost {
    pos: [f32; 3],
    spin_deg: f32,
    alpha: f32,
}

fn start_angle_deg(impact: [f32; 3]) -> f32 {
    let seed = impact[0].to_bits() ^ impact[2].to_bits() ^ 0x9E37_79B9;
    ((seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223) >> 16) % 360) as f32
}

pub struct PressureEffect {
    params: PressureParams,
    impact: [f32; 3],
    process: i32,
    icon_pos: [f32; 3],
    icon_spin_deg: f32,
    icon_alpha: f32,
    icon_done: bool,
    icon_history: Vec<IconGhost>,
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
            icon_spin_deg: start_angle_deg(impact),
            icon_alpha: 0.0,
            icon_done: false,
            icon_history: Vec::with_capacity(4),
            shake_fired: false,
            time_accum: 0.0,
        }
    }

    fn ring_age(&self) -> Option<f32> {
        let age = self.process - self.params.ring_delay_frames;
        (age > 0).then_some(age as f32)
    }

    fn ring_done(&self) -> bool {
        self.ring_age().is_some_and(|age| age >= RING_LIFE_FRAMES)
    }

    fn tick(&mut self) {
        self.process += 1;

        if !self.icon_done {
            if self.process <= ICON_FADE_IN_UNTIL {
                self.icon_alpha = (self.icon_alpha + ICON_FADE_IN_PER_FRAME).min(1.0);
            }
            if self.process <= ICON_ACTIVE_FRAMES {
                if self.process <= self.params.fall_frames {
                    self.icon_spin_deg =
                        (self.icon_spin_deg + SPIN_PER_FRAME_DEG).rem_euclid(360.0);
                    // −Y is up: falling toward the impact means y increases.
                    self.icon_pos[1] += FALL_SPEED;
                }
            } else {
                self.icon_alpha -= ICON_FADE_OUT_PER_FRAME;
                if self.icon_alpha <= 0.0 {
                    self.icon_alpha = 0.0;
                    self.icon_done = true;
                }
            }
            self.icon_history.push(IconGhost {
                pos: self.icon_pos,
                spin_deg: self.icon_spin_deg,
                alpha: self.icon_alpha,
            });
            if self.icon_history.len() > 4 {
                self.icon_history.remove(0);
            }
        }
    }

    fn push_icon(&self, out: &mut EffectDrawList, pos: [f32; 3], spin_deg: f32, alpha: f32) {
        if alpha <= 0.0 {
            return;
        }
        let side = self.params.icon_distance * std::f32::consts::SQRT_2;
        let t = self.params.icon_tint;
        out.push(EffectPrimitiveDraw::Billboard {
            pos,
            size: [side, side],
            uv: UNIT_UV,
            rotation: spin_deg.to_radians(),
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
        if !self.icon_done {
            // The trail copies sit 100, then 25, then 25 alpha behind the icon.
            const TRAIL_ALPHA_DROP: [f32; 3] = [100.0 / 255.0, 125.0 / 255.0, 150.0 / 255.0];
            for (k, drop) in TRAIL_ALPHA_DROP.iter().enumerate() {
                if let Some(ghost) = self
                    .icon_history
                    .len()
                    .checked_sub(2 + k)
                    .and_then(|i| self.icon_history.get(i))
                {
                    self.push_icon(out, ghost.pos, ghost.spin_deg, ghost.alpha - drop);
                }
            }
            self.push_icon(out, self.icon_pos, self.icon_spin_deg, self.icon_alpha);
        }

        if let Some(age) = self.ring_age() {
            if age < RING_LIFE_FRAMES {
                let distance = RING_START_DISTANCE * RING_GROWTH_PER_FRAME.powf(age);
                let alpha = if age < RING_FADE_IN_FRAMES {
                    RING_PEAK_ALPHA * (age / RING_FADE_IN_FRAMES)
                } else {
                    RING_PEAK_ALPHA
                        * (1.0
                            - (age - RING_FADE_IN_FRAMES)
                                / (RING_LIFE_FRAMES - RING_FADE_IN_FRAMES))
                };
                let r = distance * RING_CORNER_FACTOR;
                let y = self.impact[1] + RING_Y_OFFSET;
                let mut corners = [[0.0_f32; 3]; 4];
                for (i, c) in corners.iter_mut().enumerate() {
                    let (s, cs) = (i as f32 * 90.0).to_radians().sin_cos();
                    *c = [self.impact[0] + cs * r, y, self.impact[2] + s * r];
                }
                let t = self.params.ring_tint;
                for _ in 0..self.params.ring_draws {
                    out.push(EffectPrimitiveDraw::WorldQuad {
                        corners,
                        uv: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
                        texture: self.params.ring_texture,
                        color: [t[0], t[1], t[2], alpha.max(0.0)],
                        blend: BlendKind::Additive,
                        no_depth: false,
                    });
                }
            }
        }
    }

    fn take_camera_shake(&mut self) -> Option<CameraShake> {
        if self.params.quake && !self.shake_fired && self.process > ICON_ACTIVE_FRAMES {
            self.shake_fired = true;
            Some(CameraShake {
                amplitude: 1.5,
                duration_ms: 350,
            })
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
            s = e.update(&EffectUpdateCtx {
                delta: FRAME_DT,
                camera_target: None,
                caster_yaw: None,
            });
        }
        s
    }

    fn ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn list(e: &PressureEffect) -> Vec<EffectPrimitiveDraw> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &ctx());
        l.primitives
    }

    /// `(y, rotation)` of the leading icon — the trail ghosts are pushed first.
    fn icon(e: &PressureEffect) -> Option<(f32, f32)> {
        list(e).iter().rev().find_map(|p| match p {
            EffectPrimitiveDraw::Billboard { pos, rotation, .. } => Some((pos[1], *rotation)),
            _ => None,
        })
    }

    /// `(corner distance from the impact, alpha)` of the shockwave quad.
    fn ring(e: &PressureEffect) -> Option<(f32, f32)> {
        list(e).iter().find_map(|p| match p {
            EffectPrimitiveDraw::WorldQuad { corners, color, .. } => {
                Some((corners[0][0].hypot(corners[0][2]), color[3]))
            }
            _ => None,
        })
    }

    #[test]
    fn potion_falls_toward_the_ground() {
        let mut e = PressureEffect::new([0.0, 0.0, 0.0], SLIM);
        step(&mut e, 1);
        let (y0, _) = icon(&e).expect("icon visible");
        step(&mut e, 8);
        let (y1, _) = icon(&e).expect("icon visible");
        assert!(y0 < 0.0, "icon starts above the impact (negative Y = up)");
        assert!(y1 > y0, "icon descends toward the impact over time");
    }

    #[test]
    fn ring_appears_after_delay_then_grows_and_fades() {
        let mut e = PressureEffect::new([0.0, 0.0, 0.0], SLIM);
        step(&mut e, SLIM.ring_delay_frames as u32);
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
            step(&mut e, params.ring_delay_frames as u32 + 3);
            let has_tinted_ring = list(&e).iter().any(|p| matches!(p,
                EffectPrimitiveDraw::WorldQuad { color, .. }
                    if color[0] == ring_tint[0] && color[1] == ring_tint[1] && color[2] == ring_tint[2]));
            assert!(has_tinted_ring, "ring carries the variant colour");
            assert!(
                e.take_camera_shake().is_none(),
                "Slim never shakes the screen"
            );
        }
    }

    #[test]
    fn pressure_drops_cross_and_shakes_screen() {
        let mut e = PressureEffect::new([0.0, 0.0, 0.0], PRESSURE);
        // Falling cross icon is the named texture.
        step(&mut e, 1);
        assert!(
            list(&e).iter().any(|p| matches!(p,
                EffectPrimitiveDraw::Billboard { texture, .. } if *texture == "cross_old.bmp")),
            "the falling icon is the cross texture"
        );
        step(&mut e, 44);
        assert!(
            list(&e).iter().any(|p| matches!(p,
                EffectPrimitiveDraw::WorldQuad { texture, .. } if *texture == "explosive_1_128.bmp")),
            "the explosion ring expands after the delay"
        );
        assert!(
            e.take_camera_shake().is_some(),
            "Pressure shakes the screen on landing"
        );
        assert!(e.take_camera_shake().is_none(), "the shake is a one-shot");
        // The cross runs out of fall before it reaches the ground and fades
        // there, well above the impact plane.
        let (resting, spin) = icon(&e).expect("icon still fading");
        assert!(
            (-20.0..-18.0).contains(&resting),
            "cross stops short of the ground: {resting}"
        );
        step(&mut e, 10);
        let (_, spin_later) = icon(&e).expect("icon still fading");
        assert_eq!(spin, spin_later, "the cross stops spinning once it lands");
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
