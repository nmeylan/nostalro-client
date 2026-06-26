//! `EF_SAKURA` (id 163) — a gentle rain of drifting cherry-blossom petals.
//!
//! The original game spawns one petal every 2 frames
//! (up to frame 300, then it loops), each a `sakura01` sprite with a random
//! action (0–2) on a 36-frame loop. A petal spawns in a `±150` XZ box at
//! `y = pos.y - 200` (high up), falls by `[0.2, 0.4]` per frame
//! and sways `±0.24/0.30·sin` in X/Z; when it reaches the
//! ground it respawns at the top. Alpha 200. Persistent.
//!
//! Petals are [`SpriteParticle`]s on the resolved `sakura01` sprite. No
//! reference gif — validated against observed original-game behaviour.
//!
//! [`SpriteParticle`]: EffectPrimitiveDraw::SpriteParticle

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
/// Persistent (effectively unbounded); clamps to 5 s in the exporter.
pub const TOTAL_DURATION_MS: u32 = 99990;

/// `sakura01.spr/.act` lives under the effect sprite folder with this name.
const SPRITE: &str = "data/sprite/이팩트/sakura01";
pub const SPRITES: &[&str] = &[SPRITE];

/// Large literals (`±150` spread, `200` fall height) → ~0.15×.
const WORLD_SCALE: f32 = 0.15;
const SPREAD: f32 = 150.0 * WORLD_SCALE;
const TOP_HEIGHT: f32 = 200.0 * WORLD_SCALE;
const FALL_MIN: f32 = 0.2 * WORLD_SCALE;
const FALL_MAX: f32 = 0.4 * WORLD_SCALE;
const DRIFT_X: f32 = 0.24 * WORLD_SCALE;
const DRIFT_Z: f32 = 0.30 * WORLD_SCALE;

const MAX_PETALS: usize = 150;
const SPAWN_INTERVAL_FRAMES: f32 = 2.0;
const SIZE_MIN: f32 = 0.22;
const SIZE_MAX: f32 = 0.28;
const ALPHA: f32 = 200.0 / 255.0;
/// The effect prims ignore
/// the `.act` delay and advance one motion frame every 36 ticks
/// at 60 fps, so the petal tumbles slowly.
const ANIM_SPEED_TICKS: f32 = 36.0;

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
    rng: Rng,
    petals: Vec<Petal>,
    spawn_accumulator: f32,
    frame: f32,
}

impl SakuraEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let seed = (world_pos[0] * 91.0 + world_pos[2] * 57.0) as i64 as u32 ^ 0x1357_9BDF;
        Self {
            world_pos,
            rng: Rng(seed | 1),
            petals: Vec::new(),
            spawn_accumulator: 0.0,
            frame: 0.0,
        }
    }

    /// Spawn high (or, for the initial fill, at a random fall progress so the
    /// rain is populated from the first frame rather than after the warm-up).
    fn spawn(&mut self, initial_fill: bool) {
        let [cx, cy, cz] = self.world_pos;
        let top = cy - TOP_HEIGHT;
        let y = if initial_fill {
            self.rng.range(top, cy)
        } else {
            top
        };
        self.petals.push(Petal {
            pos: [
                cx + self.rng.range(-SPREAD, SPREAD),
                y,
                cz + self.rng.range(-SPREAD, SPREAD),
            ],
            fall_speed: self.rng.range(FALL_MIN, FALL_MAX),
            size: self.rng.range(SIZE_MIN, SIZE_MAX),
            action_index: (self.rng.next_u32() % 3) as usize,
            sway_x_phase: self.rng.range(0.0, 360.0),
            sway_z_phase: self.rng.range(0.0, 360.0),
            age_frames: 0.0,
        });
    }
}

impl Effect for SakuraEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frames = ctx.delta * FRAMES_PER_SECOND;
        self.frame += frames;

        // Spawn one petal per 2 frames until the cap; recycling keeps the count.
        self.spawn_accumulator += frames;
        while self.spawn_accumulator >= SPAWN_INTERVAL_FRAMES && self.petals.len() < MAX_PETALS {
            self.spawn_accumulator -= SPAWN_INTERVAL_FRAMES;
            let initial_fill = self.frame < 1.5;
            self.spawn(initial_fill);
        }

        let cy = self.world_pos[1];
        let ground = cy;
        for p in &mut self.petals {
            p.age_frames += frames;
            // Native -Y = up: falling means y increases toward the ground.
            p.pos[1] += p.fall_speed * frames;
            p.sway_x_phase = (p.sway_x_phase + 3.0 * frames) % 360.0;
            p.sway_z_phase = (p.sway_z_phase + 3.0 * frames) % 360.0;
            p.pos[0] += DRIFT_X * p.sway_x_phase.to_radians().sin() * frames;
            p.pos[2] += DRIFT_Z * p.sway_z_phase.to_radians().sin() * frames;
            if p.pos[1] > ground {
                // Respawn at the top with a fresh column.
                p.pos[0] = self.world_pos[0] + self.rng.range(-SPREAD, SPREAD);
                p.pos[1] = cy - TOP_HEIGHT;
                p.pos[2] = self.world_pos[2] + self.rng.range(-SPREAD, SPREAD);
                p.fall_speed = self.rng.range(FALL_MIN, FALL_MAX);
                p.age_frames = 0.0;
            }
        }
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for p in &self.petals {
            // One motion frame per 36 ticks (renderer wraps by motion count).
            let motion = (p.age_frames / ANIM_SPEED_TICKS) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: SPRITE,
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
        let mut e = SakuraEffect::new([0.0; 3]);
        tick(&mut e, 10);
        let early = petals(&e).len();
        tick(&mut e, 400);
        let full = petals(&e).len();
        assert!(full > early, "petals accumulate ({early} → {full})");
        assert_eq!(full, MAX_PETALS, "capped at {MAX_PETALS}");
    }

    #[test]
    fn petals_fall_toward_the_ground() {
        let mut e = SakuraEffect::new([0.0; 3]);
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
        // Native -Y up: falling means the mean y increases.
        assert!(y1 > y0, "petals drift downward ({y0} → {y1})");
    }

    #[test]
    fn action_index_varies_across_petals() {
        let mut e = SakuraEffect::new([3.0, 0.0, 7.0]);
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
    fn uses_resolved_sakura_sprite() {
        let mut e = SakuraEffect::new([0.0; 3]);
        tick(&mut e, 5);
        assert!(petals(&e).iter().all(|p| matches!(p,
            EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == SPRITE)));
        assert!(SPRITES.contains(&SPRITE));
    }
}
