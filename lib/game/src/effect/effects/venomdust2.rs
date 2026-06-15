//! `EF_VENOMDUST2` (#171) — the lingering poison dust cloud.
//!
//! In the original game the poison dust is a persistent emitter: every 5
//! frames it spawns one `particle3.spr` mote at size `0.8`,
//! flung in a random ground direction at `(rand(20)+20)/80`/frame and
//! decelerating, living 10 frames with the alpha ramping up to ~70/255 then
//! fading from frame 8. The cloud persists for the skill's whole lifetime
//! (the holder despawns it via the duration table).
//!
//! Particles are deterministic in their birth index, so the cloud replays
//! identically (no RNG dependency, testable).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use super::energy_drain::hash01;

pub const DUST_SPRITE: &str = "data/sprite/이팩트/particle3";
pub const SPRITES: &[&str] = &[DUST_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// Small spatial literals → light down-scale (~0.7× for small distances).
const WORLD_SCALE: f32 = 0.7;
/// Maps the 0.8 mote size onto our larger sprite footprint. A single mote is a
/// faint speck at this distance; the reference shows a dense glowing cloud, so
/// each mote is rendered larger (the gif outranks the lone particle).
const SIZE_RENDER_SCALE: f32 = 2.2;
/// Motes emitted per spawn tick. The original game spawns one per 5 frames; the
/// reference cloud is far denser than two live motes, so we emit a small
/// fan each tick (gif over original).
const MOTES_PER_TICK: u32 = 3;

const SPAWN_EVERY: u32 = 5;
const PARTICLE_LIFE: f32 = 10.0;
const SIZE_BASE: f32 = 0.8;
/// Alpha starts at 70 and climbs by peak/10 each frame to
/// the full 255, then fades from frame 8 — so the mote brightens before it
/// dies, it does not hold at the dim spawn alpha.
const ALPHA_START: f32 = 70.0 / 255.0;
const ALPHA_PEAK: f32 = 1.0;
const ALPHA_RAMP_FRAMES: f32 = 7.0;
const FADE_START: f32 = 8.0;
/// Slight lift so the dust sits at ankle/shin height (native RO — neg Y = up).
const DUST_LIFT: f32 = 2.0;

pub struct VenomDust2Effect {
    world_pos: [f32; 3],
    age: f32,
}

impl VenomDust2Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self { world_pos, age: 0.0 }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }
}

impl Effect for VenomDust2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        // Persistent: the holder despawns it via the duration table.
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.frame();
        // Walk every still-alive birth tick (spawned every SPAWN_EVERY frames).
        let oldest = (frame - PARTICLE_LIFE).max(0.0);
        let mut birth_idx = (oldest as u32) / SPAWN_EVERY;
        loop {
            let birth = (birth_idx * SPAWN_EVERY) as f32;
            if birth > frame {
                break;
            }
            let local = frame - birth;
            if local >= 0.0 && local <= PARTICLE_LIFE {
                for m in 0..MOTES_PER_TICK {
                    self.push_particle(out, birth_idx.wrapping_mul(7).wrapping_add(m), local);
                }
            }
            birth_idx += 1;
        }
    }
}

impl VenomDust2Effect {
    fn push_particle(&self, out: &mut EffectDrawList, idx: u32, local: f32) {
        let alpha = if local < ALPHA_RAMP_FRAMES {
            ALPHA_START + (ALPHA_PEAK - ALPHA_START) * (local / ALPHA_RAMP_FRAMES)
        } else if local < FADE_START {
            ALPHA_PEAK
        } else {
            ALPHA_PEAK * (1.0 - (local - FADE_START) / (PARTICLE_LIFE - FADE_START))
        };
        if alpha <= 0.0 {
            return;
        }
        let yaw = hash01(idx) * std::f32::consts::TAU;
        let speed = (hash01(idx ^ 0x55) * 20.0 + 20.0) / 80.0;
        let accel = -(speed / PARTICLE_LIFE) / 2.0;
        let dist = (speed * local + accel * local * (local + 1.0) / 2.0).max(0.0) * WORLD_SCALE;
        let (s, c) = yaw.sin_cos();
        out.push(EffectPrimitiveDraw::SpriteParticle {
            sprite_path: DUST_SPRITE,
            position: [
                self.world_pos[0] + s * dist,
                self.world_pos[1] - DUST_LIFT,
                self.world_pos[2] + c * dist,
            ],
            action_index: 0,
            motion_index: (local / 2.0) as usize,
            size_scale: SIZE_BASE * SIZE_RENDER_SCALE,
            color: [1.0, 1.0, 1.0, alpha],
            blend: BlendKind::Additive,
            aim_target: None,
            // Motes sit at the ground anchor; without this the terrain
            // depth-swallows them (the original game keeps them visible).
            no_depth: true,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn run_to(c: &mut VenomDust2Effect, target_frame: f32) {
        let delta = (target_frame - c.frame()) / FRAMES_PER_SECOND;
        if delta > 0.0 {
            c.update(&EffectUpdateCtx { delta, ..Default::default() });
        }
    }

    fn particle_count(c: &VenomDust2Effect) -> usize {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == DUST_SPRITE))
            .count()
    }

    #[test]
    fn emits_dust_on_a_five_frame_cadence() {
        let mut c = VenomDust2Effect::new([0.0; 3]);
        run_to(&mut c, 1.0);
        assert_eq!(particle_count(&c), MOTES_PER_TICK as usize, "frame-0 fan ramping in");
        // At frame 12: tick-5 (mid-life) and tick-10 (ramping) fans are visible;
        // tick-0 has expired (local 12 > 10).
        run_to(&mut c, 12.0);
        assert_eq!(particle_count(&c), 2 * MOTES_PER_TICK as usize);
    }

    #[test]
    fn live_set_stays_bounded_as_emitter_persists() {
        let mut c = VenomDust2Effect::new([0.0; 3]);
        // The emitter runs forever but each dust lives only 10 frames, so the
        // visible set never accumulates — at most a couple of fans at once.
        let max = 3 * MOTES_PER_TICK as usize;
        for f in [20.0, 37.0, 60.0, 95.0] {
            run_to(&mut c, f);
            let n = particle_count(&c);
            assert!(n >= 1 && n <= max, "bounded live set at frame {f}, got {n}");
        }
    }

    #[test]
    fn never_self_terminates() {
        let mut c = VenomDust2Effect::new([0.0; 3]);
        for _ in 0..200 {
            assert_eq!(c.update(&EffectUpdateCtx { delta: 0.1, ..Default::default() }), EffectStatus::Running);
        }
    }
}
