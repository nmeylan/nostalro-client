use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{BodyTint, Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

const SLASH_TEXTURE: &str = "purpleslash.tga";
pub const TEXTURES: &[&str] = &[SLASH_TEXTURE];

const CARD_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

const BASE_DISTANCE: f32 = 20.0 * 0.35;
const BASE_STEP: f32 = 0.8 * 0.85;

const SPAWN_DELAY: i32 = 28;
const STREAKS_PER_EMITTER: usize = 4;
const FADE_AFTER: i32 = 10;
const FADE_IN: f32 = 10.0;
const FADE_OUT: f32 = 5.0;

pub const TOTAL_DURATION_MS: u32 = 1200;

pub const PROJECTILE_FLIGHT: crate::effect_queue::ProjectileFlight =
    crate::effect_queue::ProjectileFlight::AtTarget;

struct Rng(u32);
impl Rng {
    fn from_seed(seed: u32) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9).wrapping_add(1))
    }
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn random(&mut self, n: u32) -> u32 {
        self.next_u32() % n.max(1)
    }
}

struct Streak {
    process: i32,
    heading: f32,
    pos: [f32; 3],
    speed: f32,
    alpha: f32,
    alive: bool,
}

pub struct SoulBreakerEffect {
    streaks: Vec<Streak>,
    frame_accum: f32,
    age_frames: u32,
}

impl SoulBreakerEffect {
    pub fn new_directed(from: [f32; 3], to: [f32; 3]) -> Self {
        let seed = from[0].to_bits() ^ to[2].to_bits() ^ 0x50_1B_8E_43;
        let mut rng = Rng::from_seed(seed);
        let base = heading_of(from, to);
        let mut streaks = Vec::with_capacity(STREAKS_PER_EMITTER);
        for ec in 0..STREAKS_PER_EMITTER {
            let jitter = (rng.random(31) as f32 - 15.0).to_radians();
            streaks.push(new_streak(
                from,
                base + jitter,
                SPAWN_DELAY - ec as i32,
                &mut rng,
            ));
        }
        Self {
            streaks,
            frame_accum: 0.0,
            age_frames: 0,
        }
    }

    pub fn new_radial(center: [f32; 3]) -> Self {
        let seed = center[0].to_bits() ^ center[2].to_bits() ^ 0x9A_55_C0_17;
        let mut rng = Rng::from_seed(seed);
        let mut streaks = Vec::with_capacity(8 * STREAKS_PER_EMITTER);
        let mut dir_deg = 0;
        while dir_deg < 360 {
            let heading = (dir_deg as f32).to_radians();
            let shoot = rng.random(6) as i32;
            for ec in 0..STREAKS_PER_EMITTER {
                streaks.push(new_streak(
                    center,
                    heading,
                    SPAWN_DELAY - ec as i32 + shoot,
                    &mut rng,
                ));
            }
            dir_deg += 45;
        }
        Self {
            streaks,
            frame_accum: 0.0,
            age_frames: 0,
        }
    }

    fn step_frame(&mut self) {
        for s in &mut self.streaks {
            if !s.alive {
                continue;
            }
            s.process += 1;
            if s.process <= 0 {
                continue;
            }
            if s.process > 1 {
                if s.process <= FADE_AFTER {
                    s.alpha += FADE_IN;
                    s.speed *= 1.02;
                } else {
                    s.alpha -= FADE_OUT;
                    s.speed *= 1.04;
                    if s.alpha <= 0.0 {
                        s.alpha = 0.0;
                        s.alive = false;
                    }
                }
            }
            s.pos[0] += s.speed * s.heading.cos();
            s.pos[2] += s.speed * s.heading.sin();
        }
        self.streaks.retain(|s| s.alive);
    }
}

fn heading_of(from: [f32; 3], to: [f32; 3]) -> f32 {
    let dx = to[0] - from[0];
    let dz = to[2] - from[2];
    if dx == 0.0 && dz == 0.0 {
        0.0
    } else {
        dz.atan2(dx) // cos(h) = dx, sin(h) = dz
    }
}

fn new_streak(from: [f32; 3], heading: f32, delay: i32, rng: &mut Rng) -> Streak {
    let y = from[1] - 10.0 + rng.random(5) as f32;
    Streak {
        process: -delay,
        heading,
        pos: [from[0], y, from[2]],
        speed: BASE_STEP,
        alpha: 0.0,
        alive: true,
    }
}

impl Effect for SoulBreakerEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND;
        while self.frame_accum >= 1.0 {
            self.frame_accum -= 1.0;
            self.age_frames += 1;
            self.step_frame();
        }
        if self.streaks.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn body_tint(&self) -> Option<BodyTint> {
        (self.age_frames <= 10).then_some(BodyTint { rgb: [255, 0, 255] })
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for s in &self.streaks {
            if s.alpha <= 0.0 {
                continue;
            }
            let side = BASE_DISTANCE * 2.0;
            out.push(EffectPrimitiveDraw::Billboard {
                pos: s.pos,
                size: [side, side],
                uv: CARD_UV,
                rotation: std::f32::consts::PI - s.heading,
                texture: SLASH_TEXTURE,
                color: [1.0, 1.0, 1.0, (s.alpha / 255.0).clamp(0.0, 1.0)],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectUpdateCtx {
        EffectUpdateCtx {
            delta: 1.0 / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        }
    }

    fn tick(e: &mut SoulBreakerEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&ctx());
        }
        st
    }

    fn billboards(e: &SoulBreakerEffect) -> Vec<([f32; 3], f32, f32)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(
            &mut list,
            &EffectRenderCtx {
                camera: Default::default(),
                screen_w: 256.0,
                screen_h: 256.0,
                elapsed: 0.0,
            },
        );
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard {
                    pos,
                    rotation,
                    color,
                    ..
                } => (*pos, *rotation, color[3]),
                other => panic!("expected Billboard, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn caster_flashes_magenta_for_the_first_frames() {
        let mut e = SoulBreakerEffect::new_directed([0.0; 3], [10.0, 0.0, 0.0]);
        assert_eq!(
            e.body_tint(),
            Some(BodyTint { rgb: [255, 0, 255] }),
            "magenta at spawn"
        );
        tick(&mut e, 5);
        assert_eq!(
            e.body_tint(),
            Some(BodyTint { rgb: [255, 0, 255] }),
            "still flashing"
        );
        tick(&mut e, 10); // past frame 10
        assert_eq!(e.body_tint(), None, "flash clears after ~10 frames");
    }

    #[test]
    fn directed_slashes_fly_toward_the_target() {
        let from = [0.0, 0.0, 0.0];
        let to = [0.0, 0.0, 22.0]; // heading +Z
        let mut e = SoulBreakerEffect::new_directed(from, to);
        tick(&mut e, SPAWN_DELAY as u32 + 4); // past the spawn delay + fade-in
        let early = billboards(&e);
        assert!(!early.is_empty(), "slashes visible after the spawn delay");
        let z_early: f32 = early.iter().map(|b| b.0[2]).sum::<f32>() / early.len() as f32;
        tick(&mut e, 6);
        let late = billboards(&e);
        let z_late: f32 = late.iter().map(|b| b.0[2]).sum::<f32>() / late.len() as f32;
        assert!(
            z_late > z_early,
            "slashes advance toward +Z target: {z_early} -> {z_late}"
        );
    }

    #[test]
    fn radial_fires_in_many_directions() {
        let mut e = SoulBreakerEffect::new_radial([0.0; 3]);
        // Distinct headings among the 8 directions (mod TAU, rounded).
        let dirs: std::collections::BTreeSet<i32> = e
            .streaks
            .iter()
            .map(|s| (s.heading.to_degrees().rem_euclid(360.0)).round() as i32)
            .collect();
        assert!(
            dirs.len() >= 4,
            "radial spans multiple directions: {dirs:?}"
        );
        tick(&mut e, SPAWN_DELAY as u32 + 4);
        assert!(!billboards(&e).is_empty(), "radial slashes render");
    }

    #[test]
    fn alpha_fades_in_then_out_and_self_terminates() {
        let mut e = SoulBreakerEffect::new_directed([0.0; 3], [0.0, 0.0, 22.0]);
        tick(&mut e, SPAWN_DELAY as u32 + 3);
        let a_in: f32 = billboards(&e).iter().map(|b| b.2).sum();
        tick(&mut e, 20);
        let a_out: f32 = billboards(&e).iter().map(|b| b.2).sum();
        assert!(a_in > 0.0, "faded in: {a_in}");
        assert!(a_out < a_in, "faded out: {a_in} -> {a_out}");
        assert_eq!(tick(&mut e, 120), EffectStatus::Dead);
    }
}
