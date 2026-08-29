//! `EF_BLITZBEAT` (id 115) — Falcon Blitz Beat cross-textured needle volley.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const BLITZBEAT_TEXTURE: &str = "ac_center2.tga";
pub const MOTE_TEXTURE: &str = "magic_green.tga";
pub const TEXTURES: &[&str] = &[BLITZBEAT_TEXTURE, MOTE_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const NEEDLE_COUNT: u32 = 10;
const DURATION_FRAMES: f32 = 20.0;
const LIFETIME_FRAMES: f32 = MOTE_FRAME + MOTE_LIFE_FRAMES;
pub const TOTAL_DURATION_MS: u32 = (LIFETIME_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const SCATTER_RADIUS: f32 = 3.5;
const Y_OFFSET: f32 = -7.0;
const FORWARD_INIT: f32 = 15.0;
const SPEED_INIT: f32 = -1.2;
const SPEED_ACCEL: f32 = -0.1;
/// The streaks rush out, then brake almost to a stop and hold near the target.
const BRAKE_FRAME: f32 = 10.0;
const BRAKE_SPEED: f32 = -0.3;
const BRAKE_ACCEL: f32 = 0.001;

const HALF_HEIGHT: f32 = 0.2;
const HALF_WIDTH_MIN: f32 = 2.0;
const HALF_WIDTH_MAX: f32 = 5.9;
const WIDTH_SPEED_INIT: f32 = 0.1;
const WIDTH_ACCEL: f32 = -0.01;
const MAX_ALPHA: f32 = 250.0 / 255.0;

const FADE_IN_FRAMES: f32 = 2.0;
const FADE_OUT_START: f32 = DURATION_FRAMES - 2.0;

// A single green cone, one hit's worth: the effect is always launched with a
// count of 1, so the emitter that would repeat it every third frame fires once.
const MOTE_FRAME: f32 = 36.0;
const MOTE_LIFE_FRAMES: f32 = 12.0;
const MOTE_HOLD_FRAMES: f32 = MOTE_LIFE_FRAMES / 2.0;
const MOTE_BOTTOM_SIZE: f32 = 0.5;
const MOTE_TOP_SIZE: f32 = 1.5;
const MOTE_HEIGHT: f32 = 3.5;
const MOTE_SIDES: u32 = 10;
const MOTE_TILT: f32 = -std::f32::consts::FRAC_PI_2;
const MOTE_SPEED: f32 = 0.4;
const MOTE_ACCEL: f32 = -(MOTE_SPEED / MOTE_LIFE_FRAMES) / 2.0;
const MOTE_ALPHA: f32 = 254.0 / 255.0;

#[derive(Clone, Copy)]
struct Needle {
    scatter: [f32; 3],
    base_half_width: f32,
}

pub struct BlitzbeatEffect {
    caster_pos: [f32; 3],
    yaw: f32,
    /// Set once from the target's facing on the first update.
    yaw_locked: bool,
    age: f32,
    needles: [Needle; NEEDLE_COUNT as usize],
    /// Sideways and vertical offset of the green cone from the target.
    mote_offset: [f32; 2],
}

const FIXED_YAW: f32 = std::f32::consts::FRAC_PI_4;

impl BlitzbeatEffect {
    pub fn new(caster_pos: [f32; 3]) -> Self {
        Self::with_yaw(caster_pos, FIXED_YAW)
    }

    pub fn with_yaw(caster_pos: [f32; 3], yaw: f32) -> Self {
        let seed = position_hash(&caster_pos);
        let mut needles = [Needle {
            scatter: [0.0; 3],
            base_half_width: 0.0,
        }; NEEDLE_COUNT as usize];
        for i in 0..NEEDLE_COUNT as usize {
            let salt = (i as u64) * 4;
            let theta = rand_in_range(seed, salt + 1, 0.0, std::f32::consts::TAU);
            let half_width = rand_in_range(seed, salt + 2, HALF_WIDTH_MIN, HALF_WIDTH_MAX);
            let (sn, cs) = theta.sin_cos();
            needles[i] = Needle {
                scatter: [SCATTER_RADIUS * sn, Y_OFFSET, -SCATTER_RADIUS * cs],
                base_half_width: half_width,
            };
        }
        // `(random(10)+1)/5` and `-12 + (random(20)+1)/2`, both integer
        // divisions, so 0..2 sideways and 2..12 above the target.
        let side = (rand_in_range(seed, 41, 1.0, 11.0) as i32 / 5) as f32;
        let lift = -12.0 + (rand_in_range(seed, 42, 1.0, 21.0) as i32 / 2) as f32;
        Self {
            caster_pos,
            yaw,
            yaw_locked: false,
            age: 0.0,
            needles,
            mote_offset: [side, lift],
        }
    }

    fn forward(&self) -> [f32; 3] {
        let (s, c) = self.yaw.sin_cos();
        [c, 0.0, s]
    }

    fn forward_offset_at(&self, frame: f32) -> f32 {
        let rush = frame.min(BRAKE_FRAME);
        let mut offset = FORWARD_INIT + SPEED_INIT * rush + SPEED_ACCEL * rush * (rush + 1.0) * 0.5;
        if frame > BRAKE_FRAME {
            let braked = frame - BRAKE_FRAME;
            offset += BRAKE_SPEED * braked + BRAKE_ACCEL * braked * (braked + 1.0) * 0.5;
        }
        offset
    }

    /// Cone base position and how far it has crept along its own axis.
    fn mote_at(&self, frame: f32) -> Option<([f32; 3], f32)> {
        let age = frame - MOTE_FRAME;
        if !(0.0..MOTE_LIFE_FRAMES).contains(&age) {
            return None;
        }
        let travel = MOTE_SPEED * age + MOTE_ACCEL * age * (age + 1.0) * 0.5;
        let fwd = self.forward();
        // The cone is flung two units ahead and one unit down when it spawns.
        let base = [
            self.caster_pos[0] + self.mote_offset[0] + fwd[0] * (2.0 + travel),
            self.caster_pos[1] + self.mote_offset[1] + 1.0,
            self.caster_pos[2] + fwd[2] * (2.0 + travel),
        ];
        let alpha = if age <= MOTE_HOLD_FRAMES {
            MOTE_ALPHA
        } else {
            MOTE_ALPHA * (1.0 - (age - MOTE_HOLD_FRAMES) / MOTE_HOLD_FRAMES)
        };
        Some((base, alpha))
    }

    fn half_width_at(&self, base: f32, frame: f32) -> f32 {
        let w = base + WIDTH_SPEED_INIT * frame + WIDTH_ACCEL * frame * (frame + 1.0) * 0.5;
        w.max(0.05)
    }

    fn alpha_at(&self, frame: f32) -> f32 {
        MAX_ALPHA
            * if frame < FADE_IN_FRAMES {
                (frame / FADE_IN_FRAMES).clamp(0.0, 1.0)
            } else if frame < FADE_OUT_START {
                1.0
            } else {
                (1.0 - (frame - FADE_OUT_START) / (DURATION_FRAMES - FADE_OUT_START))
                    .clamp(0.0, 1.0)
            }
    }
}

impl Effect for BlitzbeatEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        if !self.yaw_locked
            && let Some(yaw) = ctx.caster_yaw
        {
            self.yaw = yaw;
            self.yaw_locked = true;
        }
        self.age += ctx.delta;
        if self.age * FRAMES_PER_SECOND >= LIFETIME_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FRAMES_PER_SECOND;
        if let Some((base, alpha)) = self.mote_at(frame) {
            out.push(EffectPrimitiveDraw::Cylinder {
                base,
                bottom_size: MOTE_BOTTOM_SIZE,
                top_size: MOTE_TOP_SIZE,
                height: MOTE_HEIGHT,
                sides: MOTE_SIDES,
                rotation: 0.0,
                tilt_x_rad: MOTE_TILT,
                // The tilt lays the cone on its side; this yaw then points its
                // apex along `forward`.
                rotation_y_rad: self.yaw + std::f32::consts::FRAC_PI_2,
                uv_scroll: [0.0, 0.0],
                texture: MOTE_TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
                alpha_bottom: alpha,
                blend: BlendKind::Additive,
            });
        }

        let alpha = self.alpha_at(frame);
        if alpha <= 0.0 || frame >= DURATION_FRAMES {
            return;
        }
        let forward = self.forward();
        let forward_offset = self.forward_offset_at(frame);
        let color = [1.0, 1.0, 1.0, alpha];
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        for needle in &self.needles {
            let half_width = self.half_width_at(needle.base_half_width, frame);
            let center = [
                self.caster_pos[0] + needle.scatter[0] + forward[0] * forward_offset,
                self.caster_pos[1] + needle.scatter[1] + forward[1] * forward_offset,
                self.caster_pos[2] + needle.scatter[2] + forward[2] * forward_offset,
            ];
            for plane in [
                QuadPlane::HorizontalYaw(self.yaw),
                QuadPlane::VerticalYaw(self.yaw),
            ] {
                out.push(EffectPrimitiveDraw::Texture3D {
                    center,
                    size: [half_width, HALF_HEIGHT],
                    plane,
                    uv,
                    texture: BLITZBEAT_TEXTURE,
                    color,
                    blend: BlendKind::Additive,
                });
            }
        }
    }
}

fn position_hash(pos: &[f32; 3]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    pos[0].to_bits().hash(&mut h);
    pos[1].to_bits().hash(&mut h);
    pos[2].to_bits().hash(&mut h);
    h.finish()
}

fn rand_in_range(seed: u64, salt: u64, lo: f32, hi: f32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(salt);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    let t = ((x >> 40) as f32) / ((1u64 << 24) as f32);
    lo + t * (hi - lo)
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

    fn step_n(e: &mut BlitzbeatEffect, n: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..n {
            s = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
            if s == EffectStatus::Dead {
                break;
            }
        }
        s
    }

    fn draws(e: &BlitzbeatEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn ten_parallel_cross_textured_needles_translate_along_forward() {
        let mut e = BlitzbeatEffect::with_yaw([0.0, 0.0, 0.0], 0.0);

        step_n(&mut e, 3);
        let prims = draws(&e);
        assert_eq!(prims.len(), 20);

        for p in &prims {
            let yaw = match p {
                EffectPrimitiveDraw::Texture3D {
                    plane: QuadPlane::HorizontalYaw(y) | QuadPlane::VerticalYaw(y),
                    ..
                } => *y,
                _ => panic!("expected Texture3D needle plane"),
            };
            assert_eq!(yaw, 0.0);
        }

        let avg_x = |prims: &[EffectPrimitiveDraw]| -> f32 {
            let xs: Vec<f32> = prims
                .iter()
                .map(|p| match p {
                    EffectPrimitiveDraw::Texture3D { center, .. } => center[0],
                    _ => panic!(),
                })
                .collect();
            xs.iter().sum::<f32>() / xs.len() as f32
        };
        let x_early = avg_x(&prims);
        step_n(&mut e, 8);
        let x_later = avg_x(&draws(&e));
        assert!(x_later < x_early, "{x_early} -> {x_later}");

        // Past frame 10 the needles brake, so they settle just short of the
        // target instead of sailing on through it.
        assert!((e.forward_offset_at(10.0) + 2.5).abs() < 0.01);
        assert!((e.forward_offset_at(20.0) + 5.45).abs() < 0.01);
    }

    #[test]
    fn green_cone_fires_once_after_the_needles_and_points_downrange() {
        let mut e = BlitzbeatEffect::new([0.0, 0.0, 0.0]);
        assert_eq!(step_n(&mut e, 25), EffectStatus::Running);
        assert!(draws(&e).is_empty(), "needles are spent by frame 25");

        step_n(&mut e, 13);
        let cones: Vec<EffectPrimitiveDraw> = draws(&e)
            .into_iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Cylinder { .. }))
            .collect();
        assert_eq!(cones.len(), 1, "one cone: the effect carries a single hit");

        let EffectPrimitiveDraw::Cylinder {
            height,
            tilt_x_rad,
            rotation_y_rad,
            ..
        } = &cones[0]
        else {
            unreachable!()
        };
        // Walk the renderer's local-to-world transform for the apex at
        // (0, -height, 0) and check the cone lies flat, aimed downrange.
        let (sin_tx, cos_tx) = tilt_x_rad.sin_cos();
        let (sin_ry, cos_ry) = rotation_y_rad.sin_cos();
        let y1 = -height * cos_tx;
        let z1 = height * sin_tx;
        let apex = [-z1 * sin_ry, y1, z1 * cos_ry];
        let fwd = e.forward();
        assert!(apex[1].abs() < 1e-4, "cone lies on its side: {apex:?}");
        assert!(
            (apex[0] / height - fwd[0]).abs() < 1e-4 && (apex[2] / height - fwd[2]).abs() < 1e-4,
            "apex points downrange: {apex:?} vs {fwd:?}"
        );

        assert_eq!(step_n(&mut e, 12), EffectStatus::Dead);
    }

    #[test]
    fn deterministic_scatter_per_position() {
        let a = BlitzbeatEffect::new([10.0, 0.0, 20.0]);
        let b = BlitzbeatEffect::new([10.0, 0.0, 20.0]);
        for i in 0..NEEDLE_COUNT as usize {
            assert_eq!(a.needles[i].scatter, b.needles[i].scatter);
            assert_eq!(a.needles[i].base_half_width, b.needles[i].base_half_width);
        }
    }
}
