use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TOTAL_DURATION_MS: u32 = 99990;

const SAKURA_SPRITE: &str = "data/sprite/이팩트/sakura01";
const MAPLE_SPRITE: &str = "data/sprite/이팩트/단풍";
pub const SPRITES: &[&str] = &[SAKURA_SPRITE, MAPLE_SPRITE];

const SPREAD: f32 = 150.0;
const TOP_HEIGHT: f32 = 200.0;

const SPAWN_INTERVAL_FRAMES: f32 = 2.0;
const ALPHA: f32 = 200.0 / 255.0;
const ANIM_SPEED_TICKS: f32 = 36.0;

#[derive(Clone, Copy)]
pub struct SakuraParams {
    pub sprite: &'static str,
    pub max_particles: usize,
    pub fall: (f32, f32),
    pub drift: (f32, f32),
    pub size: (f32, f32),
    pub random_action: bool,
}

pub const SAKURA: SakuraParams = SakuraParams {
    sprite: SAKURA_SPRITE,
    max_particles: 150,
    fall: (0.2, 0.4),
    drift: (0.24, 0.30),
    size: (0.22, 0.28),
    random_action: true,
};

pub const MAPLE: SakuraParams = SakuraParams {
    sprite: MAPLE_SPRITE,
    max_particles: 100,
    fall: (0.06, 0.15),
    drift: (0.12, 0.15),
    size: (0.35, 0.40),
    random_action: false,
};

struct Petal {
    pos: [f32; 3],
    fall_speed: f32,
    size: f32,
    action_index: usize,
    sway_x_phase: f32,
    sway_z_phase: f32,
    age_frames: f32,
}

struct Rng(u32);
impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next_u32() as f32 / u32::MAX as f32)
    }
}

pub struct SakuraEffect {
    world_pos: [f32; 3],
    params: SakuraParams,
    rng: Rng,
    petals: Vec<Petal>,
    spawn_accumulator: f32,
    frame: f32,
}

impl SakuraEffect {
    pub fn new(world_pos: [f32; 3], params: SakuraParams) -> Self {
        let seed = (world_pos[0] * 91.0 + world_pos[2] * 57.0) as i64 as u32 ^ 0x1357_9BDF;
        Self {
            world_pos,
            params,
            rng: Rng(seed | 1),
            petals: Vec::new(),
            spawn_accumulator: 0.0,
            frame: 0.0,
        }
    }

    fn spawn(&mut self, initial_fill: bool) {
        let [cx, cy, cz] = self.world_pos;
        let top = cy - TOP_HEIGHT;
        let y = if initial_fill {
            self.rng.range(top, cy)
        } else {
            top
        };
        let action_index = if self.params.random_action {
            (self.rng.next_u32() % 3) as usize
        } else {
            0
        };
        self.petals.push(Petal {
            pos: [
                cx + self.rng.range(-SPREAD, SPREAD),
                y,
                cz + self.rng.range(-SPREAD, SPREAD),
            ],
            fall_speed: self.rng.range(self.params.fall.0, self.params.fall.1),
            size: self.rng.range(self.params.size.0, self.params.size.1),
            action_index,
            sway_x_phase: self.rng.range(0.0, 360.0),
            sway_z_phase: self.rng.range(0.0, 360.0),
            age_frames: 0.0,
        });
    }
}

impl Effect for SakuraEffect {
    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frames = ctx.delta * FRAMES_PER_SECOND;
        self.frame += frames;

        self.spawn_accumulator += frames;
        while self.spawn_accumulator >= SPAWN_INTERVAL_FRAMES
            && self.petals.len() < self.params.max_particles
        {
            self.spawn_accumulator -= SPAWN_INTERVAL_FRAMES;
            let initial_fill = self.frame < 1.5;
            self.spawn(initial_fill);
        }

        let cy = self.world_pos[1];
        let ground = cy;
        let (drift_x, drift_z) = self.params.drift;
        for p in &mut self.petals {
            p.age_frames += frames;
            p.pos[1] += p.fall_speed * frames;
            p.sway_x_phase = (p.sway_x_phase + 3.0 * frames) % 360.0;
            p.sway_z_phase = (p.sway_z_phase + 3.0 * frames) % 360.0;
            p.pos[0] += drift_x * p.sway_x_phase.to_radians().sin() * frames;
            p.pos[2] += drift_z * p.sway_z_phase.to_radians().sin() * frames;
            if p.pos[1] > ground {
                p.pos[0] = self.world_pos[0] + self.rng.range(-SPREAD, SPREAD);
                p.pos[1] = cy - TOP_HEIGHT;
                p.pos[2] = self.world_pos[2] + self.rng.range(-SPREAD, SPREAD);
                p.fall_speed = self.rng.range(self.params.fall.0, self.params.fall.1);
                p.age_frames = 0.0;
            }
        }
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for p in &self.petals {
            let motion = (p.age_frames / ANIM_SPEED_TICKS) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: self.params.sprite,
                position: p.pos,
                action_index: p.action_index,
                motion_index: motion,
                size_scale: p.size,
                color: [1.0, 1.0, 1.0, ALPHA],
                blend: BlendKind::Alpha,
                aim_target: None,
                no_depth: false,
            });
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

    fn tick(e: &mut SakuraEffect, frames: u32) {
        for _ in 0..frames {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
    }

    fn petals(e: &SakuraEffect) -> Vec<EffectPrimitiveDraw> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        l.primitives
    }

    #[test]
    fn petals_accumulate_to_the_cap() {
        let mut e = SakuraEffect::new([0.0; 3], SAKURA);
        tick(&mut e, 10);
        let early = petals(&e).len();
        tick(&mut e, 400);
        let full = petals(&e).len();
        assert!(full > early, "petals accumulate ({early} → {full})");
        assert_eq!(full, SAKURA.max_particles, "capped at {}", SAKURA.max_particles);
    }

    #[test]
    fn petals_fall_toward_the_ground() {
        let mut e = SakuraEffect::new([0.0; 3], SAKURA);
        tick(&mut e, 30);
        let y0: f32 = petals(&e)
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { position, .. } => position[1],
                _ => 0.0,
            })
            .sum::<f32>()
            / petals(&e).len() as f32;
        tick(&mut e, 60);
        let y1: f32 = petals(&e)
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { position, .. } => position[1],
                _ => 0.0,
            })
            .sum::<f32>()
            / petals(&e).len() as f32;
        assert!(y1 > y0, "petals drift downward ({y0} → {y1})");
    }

    #[test]
    fn action_index_varies_across_petals() {
        let mut e = SakuraEffect::new([3.0, 0.0, 7.0], SAKURA);
        tick(&mut e, 400);
        let actions: std::collections::BTreeSet<usize> = petals(&e)
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { action_index, .. } => Some(*action_index),
                _ => None,
            })
            .collect();
        assert!(actions.len() > 1, "random action 0–2 across petals");
        assert!(actions.iter().all(|a| *a < 3));
    }

    #[test]
    fn maple_uses_its_sprite_and_action_zero() {
        let mut e = SakuraEffect::new([0.0; 3], MAPLE);
        tick(&mut e, 300);
        assert_eq!(petals(&e).len(), MAPLE.max_particles, "maple leaf cap");
        assert!(petals(&e).iter().all(|p| matches!(p,
            EffectPrimitiveDraw::SpriteParticle { sprite_path, action_index, .. }
                if *sprite_path == MAPLE_SPRITE && *action_index == 0)));
    }

    #[test]
    fn recycled_petals_track_the_live_anchor() {
        let mut e = SakuraEffect::new([0.0; 3], SAKURA);
        tick(&mut e, 300);
        e.set_position([500.0, 0.0, 500.0]);
        tick(&mut e, 1500);
        let near_new_anchor = petals(&e).iter().any(|p| matches!(p,
            EffectPrimitiveDraw::SpriteParticle { position, .. } if position[0] > 300.0));
        assert!(
            near_new_anchor,
            "petals that reach the ground re-enter around the moved anchor"
        );
    }
}
