//! `EF_BLESSING` — Priest Blessing: cyan ground disc + angel sprites + rising twinkles.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const DISC_TEXTURE: &str = "alpha_down.tga";
pub const TEXTURES: &[&str] = &[DISC_TEXTURE];

pub const BLESSING_SPRITE: &str = "data/sprite/이팩트/축복";
pub const TWINKLE_SPRITE: &str = "data/sprite/이팩트/particle6";
pub const SPRITES: &[&str] = &[BLESSING_SPRITE, TWINKLE_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const PARENT_DURATION_FRAMES: f32 = 120.0;

const DISC_RADIUS: f32 = 10.0;
const DISC_TINT: [f32; 3] = [
    0x20 as f32 / 255.0,
    0xb0 as f32 / 255.0,
    0xe8 as f32 / 255.0,
];
const DISC_MAX_ALPHA: f32 = 100.0 / 255.0;
const DISC_FADE_IN_FRAMES: f32 = 30.0;
const DISC_FADE_OUT_AT: f32 = PARENT_DURATION_FRAMES - 30.0;

const BLESSING_SPAWN_PERIOD_FRAMES: u32 = 3;
const BLESSING_SPAWN_END_FRAME: u32 = 9;
const BLESSING_HEIGHT_OFFSET: f32 = -25.0;
const BLESSING_SIZE: f32 = 1.0;
const BLESSING_ANIM_TICKS: f32 = 1.75;
const BLESSING_FRAME_MS: f32 = 1000.0 / FRAMES_PER_SECOND * BLESSING_ANIM_TICKS;

const TWINKLE_SPAWN_PERIOD_FRAMES: u32 = 4;
const TWINKLE_DURATION_FRAMES: f32 = 80.0;
const TWINKLE_RADIUS_MIN: f32 = 2.0;
const TWINKLE_RADIUS_MAX: f32 = 9.0;
const TWINKLE_SIZE: f32 = 0.5;
const TWINKLE_RISE_SPEED_MIN: f32 = -0.4;
const TWINKLE_RISE_SPEED_MAX: f32 = -0.2;
const TWINKLE_FADEOUT_AT: f32 = TWINKLE_DURATION_FRAMES - TWINKLE_DURATION_FRAMES / 5.0;
const TWINKLE_ANIM_TICKS: f32 = 4.0;
const TWINKLE_FRAME_MS: f32 = 1000.0 / FRAMES_PER_SECOND * TWINKLE_ANIM_TICKS;

pub const TOTAL_DURATION_MS: u32 =
    ((PARENT_DURATION_FRAMES + TWINKLE_DURATION_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

fn disc_alpha(frame: f32) -> f32 {
    let rise = (frame / DISC_FADE_IN_FRAMES).clamp(0.0, 1.0);
    let fall = if frame < DISC_FADE_OUT_AT {
        1.0
    } else {
        let span = (PARENT_DURATION_FRAMES - DISC_FADE_OUT_AT).max(1e-3);
        (1.0 - (frame - DISC_FADE_OUT_AT) / span).clamp(0.0, 1.0)
    };
    DISC_MAX_ALPHA * rise * fall
}

#[derive(Clone, Copy, Debug)]
struct AngelParticle {
    spawn_frame: f32,
    alpha_at_spawn: f32,
}

#[derive(Clone, Copy, Debug)]
struct Twinkle {
    offset_at_spawn: [f32; 3],
    rise_speed_per_frame: f32,
    age_frames: f32,
}

impl Twinkle {
    fn alive(&self) -> bool {
        self.age_frames < TWINKLE_DURATION_FRAMES
    }

    fn step(&mut self, dt_frames: f32) {
        self.age_frames += dt_frames;
    }

    fn alpha(&self) -> f32 {
        let rise = (self.age_frames / 10.0).clamp(0.0, 1.0);
        let fall = if self.age_frames < TWINKLE_FADEOUT_AT {
            1.0
        } else {
            let span = (TWINKLE_DURATION_FRAMES - TWINKLE_FADEOUT_AT).max(1e-3);
            (1.0 - (self.age_frames - TWINKLE_FADEOUT_AT) / span).clamp(0.0, 1.0)
        };
        rise * fall
    }

    fn position(&self, anchor: [f32; 3]) -> [f32; 3] {
        [
            anchor[0] + self.offset_at_spawn[0],
            anchor[1] + self.offset_at_spawn[1] + self.rise_speed_per_frame * self.age_frames,
            anchor[2] + self.offset_at_spawn[2],
        ]
    }
}

pub struct BlessingEffect {
    world_pos: [f32; 3],
    angels: Vec<AngelParticle>,
    twinkles: Vec<Twinkle>,
    age_frames: f32,
    last_spawn_frame: i32,
    rng_state: u32,
}

impl BlessingEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let rng_state =
            0x9E37_79B9 ^ world_pos[0].to_bits() ^ world_pos[2].to_bits().rotate_left(13);
        Self {
            world_pos,
            angels: Vec::new(),
            twinkles: Vec::new(),
            age_frames: 0.0,
            last_spawn_frame: -1,
            rng_state,
        }
    }

    fn lcg(&mut self) -> u32 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.rng_state
    }

    fn lcg_float(&mut self) -> f32 {
        (self.lcg() >> 8) as f32 / ((1u32 << 24) as f32)
    }

    fn spawn_twinkle(&mut self) {
        let longitude_deg = self.lcg_float() * 360.0;
        let radius =
            TWINKLE_RADIUS_MIN + self.lcg_float() * (TWINKLE_RADIUS_MAX - TWINKLE_RADIUS_MIN);
        let rise = TWINKLE_RISE_SPEED_MIN
            + self.lcg_float() * (TWINKLE_RISE_SPEED_MAX - TWINKLE_RISE_SPEED_MIN);
        let (sn, cs) = longitude_deg.to_radians().sin_cos();
        self.twinkles.push(Twinkle {
            offset_at_spawn: [radius * sn, BLESSING_HEIGHT_OFFSET, radius * cs],
            rise_speed_per_frame: rise,
            age_frames: 0.0,
        });
    }
}

impl Effect for BlessingEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt_frames;

        let current_frame = self.age_frames.floor() as i32;
        let next_frame = self.last_spawn_frame + 1;
        for f in next_frame..=current_frame {
            if f < 0 {
                continue;
            }
            let fu = f as u32;
            if fu <= BLESSING_SPAWN_END_FRAME && fu % BLESSING_SPAWN_PERIOD_FRAMES == 0 {
                let alpha_at_spawn = ((200.0 - fu as f32 * 6.0) / 255.0).clamp(0.0, 1.0);
                self.angels.push(AngelParticle {
                    spawn_frame: f as f32,
                    alpha_at_spawn,
                });
            }
            if (f as f32) <= PARENT_DURATION_FRAMES && fu % TWINKLE_SPAWN_PERIOD_FRAMES == 0 {
                self.spawn_twinkle();
            }
        }
        self.last_spawn_frame = current_frame;

        for t in &mut self.twinkles {
            t.step(dt_frames);
        }
        self.twinkles.retain(|t| t.alive());

        if self.age_frames >= PARENT_DURATION_FRAMES + TWINKLE_DURATION_FRAMES
            && self.twinkles.is_empty()
        {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.age_frames <= PARENT_DURATION_FRAMES {
            let alpha = disc_alpha(self.age_frames);
            if alpha > 0.0 {
                out.push(EffectPrimitiveDraw::GroundDisc {
                    center: self.world_pos,
                    radius: DISC_RADIUS,
                    thickness: DISC_RADIUS,
                    rotation: 0.0,
                    arc_angle_deg: 360.0,
                    uv_repeat: 1.0,
                    texture: DISC_TEXTURE,
                    color: [DISC_TINT[0], DISC_TINT[1], DISC_TINT[2], alpha],
                    blend: BlendKind::Alpha,
                    no_depth: false,
                    tilt_rad: 0.0,
                    spin_rad: 0.0,
                });
            }
        }

        let angel_pos = [
            self.world_pos[0],
            self.world_pos[1] + BLESSING_HEIGHT_OFFSET,
            self.world_pos[2],
        ];
        for angel in &self.angels {
            let age = self.age_frames - angel.spawn_frame;
            if age < 0.0 || age >= PARENT_DURATION_FRAMES {
                continue;
            }
            let fade = if age < PARENT_DURATION_FRAMES - 20.0 {
                1.0
            } else {
                ((PARENT_DURATION_FRAMES - age) / 20.0).clamp(0.0, 1.0)
            };
            let alpha = angel.alpha_at_spawn * fade;
            if alpha <= 0.0 {
                continue;
            }
            let motion = (age * (1000.0 / FRAMES_PER_SECOND) / BLESSING_FRAME_MS) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: BLESSING_SPRITE,
                position: angel_pos,
                action_index: 0,
                motion_index: motion,
                size_scale: BLESSING_SIZE,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Alpha,
                aim_target: None,
                no_depth: false,
            });
        }

        for t in &self.twinkles {
            let alpha = t.alpha();
            if alpha <= 0.0 {
                continue;
            }
            let motion = (t.age_frames * (1000.0 / FRAMES_PER_SECOND) / TWINKLE_FRAME_MS) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: TWINKLE_SPRITE,
                position: t.position(self.world_pos),
                action_index: 0,
                motion_index: motion,
                size_scale: TWINKLE_SIZE,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
                aim_target: None,
                no_depth: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step_frames(e: &mut BlessingEffect, n: i32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    #[test]
    fn ground_disc_plus_angels_plus_twinkles() {
        let mut e = BlessingEffect::new([2.0, 0.0, 3.0]);
        step_frames(&mut e, 13);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let discs: usize = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { .. }))
            .count();
        assert_eq!(discs, 1);

        let angels: usize = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == BLESSING_SPRITE))
            .count();
        assert_eq!(angels, 4);

        let twinkles: usize = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == TWINKLE_SPRITE))
            .count();
        assert!(twinkles >= 3);
    }

    #[test]
    fn disc_alpha_fades_in_then_out() {
        assert_eq!(disc_alpha(0.0), 0.0);
        assert!((disc_alpha(DISC_FADE_IN_FRAMES) - DISC_MAX_ALPHA).abs() < 1e-3);
        assert!((disc_alpha(45.0) - DISC_MAX_ALPHA).abs() < 1e-3);
        assert!(disc_alpha(PARENT_DURATION_FRAMES - 1.0) < DISC_MAX_ALPHA);
    }

    #[test]
    fn dies_after_parent_plus_twinkle_lifetime() {
        let mut e = BlessingEffect::new([0.0; 3]);
        let total = PARENT_DURATION_FRAMES + TWINKLE_DURATION_FRAMES + 5.0;
        let mut status = EffectStatus::Running;
        for _ in 0..(total as i32) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
