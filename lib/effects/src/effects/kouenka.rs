use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const SPRITE: &str = "data/sprite/이팩트/sakura01";
pub const TEXTURES: &[&str] = &[];

pub const STR_OVERLAY: &str = "firehit";

const FRAMES_PER_SECOND: f32 = 60.0;
const TOTAL_FRAMES: f32 = 180.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const SPAWN_UNTIL_FRAME: f32 = 20.0;
const RING_RADIUS: f32 = 8.0;
const SIZE_MIN: f32 = 22.0 / 70.0;
const SIZE_MAX: f32 = 28.0 / 70.0;
const RISE_MIN: f32 = 2.0 * 0.15;
const RISE_MAX: f32 = 5.0 * 0.15;
const PEAK_ALPHA: f32 = 250.0 / 255.0;
const FADE_OUT_FRAMES: f32 = 40.0;

#[derive(Clone, Copy)]
struct Rng(u32);

impl Rng {
    fn from_seed(seed: u32) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9).wrapping_add(1))
    }
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next_u32() as f32 / u32::MAX as f32)
    }
}

#[derive(Clone, Copy)]
struct Petal {
    spawn_frame: f32,
    origin: [f32; 3],
    rise_total: f32,
    size: f32,
    action: usize,
}

pub struct KouenkaEffect {
    age_frames: f32,
    next_spawn_frame: f32,
    petals: Vec<Petal>,
    rng: Rng,
    world_pos: [f32; 3],
}

impl KouenkaEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            age_frames: 0.0,
            next_spawn_frame: 0.0,
            petals: Vec::new(),
            rng: Rng::from_seed(world_pos[0].to_bits() ^ world_pos[2].to_bits() ^ 0x5A_4B_55_31),
            world_pos,
        }
    }

    fn spawn_petal(&mut self, frame: f32) {
        let angle = self.rng.range_f32(0.0, std::f32::consts::TAU);
        let (s, c) = angle.sin_cos();
        self.petals.push(Petal {
            spawn_frame: frame,
            origin: [
                self.world_pos[0] + RING_RADIUS * c,
                self.world_pos[1],
                self.world_pos[2] + RING_RADIUS * s,
            ],
            rise_total: self.rng.range_f32(RISE_MIN, RISE_MAX),
            size: self.rng.range_f32(SIZE_MIN, SIZE_MAX),
            action: (self.rng.next_u32() % 3) as usize,
        });
    }
}

impl Effect for KouenkaEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        while self.next_spawn_frame < self.age_frames && self.next_spawn_frame < SPAWN_UNTIL_FRAME {
            let f = self.next_spawn_frame;
            self.spawn_petal(f);
            self.next_spawn_frame += 1.0;
        }
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for p in &self.petals {
            let local = self.age_frames - p.spawn_frame;
            if local < 0.0 {
                continue;
            }
            let life_left = TOTAL_FRAMES - p.spawn_frame;
            let t = (local / life_left).clamp(0.0, 1.0);
            let y = p.origin[1] - p.rise_total * t;
            let alpha = if local > life_left - FADE_OUT_FRAMES {
                PEAK_ALPHA * ((life_left - local) / FADE_OUT_FRAMES).clamp(0.0, 1.0)
            } else {
                PEAK_ALPHA
            };
            if alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: SPRITE,
                position: [p.origin[0], y, p.origin[2]],
                action_index: p.action,
                motion_index: local as usize,
                size_scale: p.size,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Alpha,
                aim_target: None,
                no_depth: false,
            });
        }
    }

    fn str_overlay(&self) -> Option<&'static str> {
        Some(STR_OVERLAY)
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

    fn step(e: &mut KouenkaEffect, frames: f32) {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn draws(e: &KouenkaEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn scatters_sakura_particles_on_a_ring_with_firehit_overlay() {
        let mut e = KouenkaEffect::new([10.0, 0.0, 20.0]);
        assert_eq!(e.str_overlay(), Some(STR_OVERLAY));
        step(&mut e, 20.0);
        let prims = draws(&e);
        assert_eq!(prims.len(), 20, "20 petals spawned during the spawn window");
        for p in &prims {
            let EffectPrimitiveDraw::SpriteParticle {
                sprite_path,
                position,
                action_index,
                ..
            } = p
            else {
                panic!("expected SpriteParticle");
            };
            assert_eq!(*sprite_path, SPRITE);
            assert!(*action_index < 3, "one of three blossom actions");
            let dx = position[0] - 10.0;
            let dz = position[2] - 20.0;
            assert!(
                ((dx * dx + dz * dz).sqrt() - RING_RADIUS).abs() < 1e-2,
                "on the ring"
            );
        }
    }

    #[test]
    fn stops_spawning_after_window_and_dies_at_duration() {
        let mut e = KouenkaEffect::new([0.0; 3]);
        step(&mut e, 60.0);
        assert_eq!(draws(&e).len(), 20, "no new petals after frame 20");
        let s = e.update(&EffectUpdateCtx {
            delta: TOTAL_DURATION_MS as f32 / 1000.0 + 0.1,
            camera_target: None,
            caster_yaw: None,
        });
        assert!(matches!(s, EffectStatus::Dead));
    }
}
