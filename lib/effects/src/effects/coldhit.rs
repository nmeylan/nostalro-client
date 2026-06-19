//! `EF_COLDHIT` (#51) — the ice-shard splash of a cold-property hit.
//!
//! The reference plays in two phases: first a crisp white **needle starburst**
//! radiating from the impact point (frames 3-4 of the reference gif), then a
//! lingering lumpy white **puff cluster** (frames 5-9) that swells and fades.
//!
//! The original game launches nine ice shards (`effect\ice.tga`, every 40° +
//! jitter, 3 long/6 short) as screen-space tapered spindles, plus two smoke
//! puffs (`effect\smoke.tga`, at frames 0 and 7) that grow in place. Both are
//! flat alpha-blended screen quads anchored at the target's projected
//! position.
//!
//! `ice.tga` is a shapeless frost cloud — the original's needle silhouette
//! comes entirely from the spindle *geometry*, which a rectangular billboard
//! can't taper into. The reference gif (the authority) shows crisp double-ended
//! spikes, which `lens1.tga` (a vertically symmetric lens streak that fades to
//! its tips) reproduces as a screen-space [`BillboardFlash`] rotated to each
//! radial angle — the same mechanism as the Hit2 petals. `smoke.tga` is itself
//! a multi-lobed puff cluster, so two growing copies read as the gif's lumpy
//! frost cloud.
//!
//! [`BillboardFlash`]: EffectPrimitiveDraw::BillboardFlash

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use super::energy_drain::hash01;
use super::spike_burst::fade_in_out;

/// Needle texture: a vertically symmetric lens streak whose alpha tapers to
/// both tips, giving the crisp double-ended spikes the reference gif shows
/// (the original's `ice.tga` is a shapeless cloud whose needle form is pure
/// spindle geometry we can't reproduce on a rectangular quad).
pub const NEEDLE_TEXTURE: &str = "lens1.tga";
/// `smoke.tga` is a lumpy multi-puff cluster — two growing copies make the
/// lingering frost cloud.
pub const SMOKE_TEXTURE: &str = "smoke.tga";
pub const TEXTURES: &[&str] = &[NEEDLE_TEXTURE, SMOKE_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// Icy white tint for both the needles and the cloud.
const ICE_TINT: [f32; 3] = [0.88, 0.94, 1.0];
/// Quad lifted to the target's body (native RO — negative Y = up).
const BODY_LIFT: f32 = 3.0;
/// The original game's quad sizes are screen pixels; our
/// `BillboardFlash` is sized in world units (a character is ~5-8). Downscaled
/// uniformly so a big needle spans roughly two characters.
const WORLD_SCALE: f32 = 0.12;

pub const TOTAL_DURATION_MS: u32 = 550;

// --- phase 1: ice needles (frames 0-15) ---
const NUM_SHARDS: usize = 9;
const SHARD_LIFE: f32 = 15.0;
const SHARD_FADE_START: f32 = 5.0;
const SHARD_MAX_ALPHA: f32 = 200.0 / 255.0;
const SHARD_FADE_IN: f32 = 1.0;
/// Needles shoot out to full length over the first frames, then hold and fade.
const SHARD_EXTEND_FRAMES: f32 = 4.0;

// --- phase 2: smoke puffs (frames 0 & 7), the lingering feature ---
const SMOKE_BIRTHS: [f32; 2] = [0.0, 7.0];
const SMOKE_LIFE: f32 = 25.0;
const SMOKE_FADE_START: f32 = 16.7; // duration - duration/3
const SMOKE_FADE_IN: f32 = 3.0;
const SMOKE_MAX_ALPHA: f32 = 0.8;
/// Half-extent growth: `+6`/frame, slowing to `0.5`/frame at the change point.
const SMOKE_HALF_INIT: f32 = 1.0;
const SMOKE_GROWTH_FAST: f32 = 6.0;
const SMOKE_GROWTH_SLOW: f32 = 0.5;
const SMOKE_CHANGE_FRAME: f32 = 10.0;

const TOTAL_FRAMES: f32 = 33.0;

/// Per-shard (width, length, outward speed) in source screen pixels: every
/// third shard (indices 0/3/6 of nine) is a long, thick, fast spike, the rest
/// short, thin and slow.
fn shard_dims(a: usize, seed: u32) -> (f32, f32, f32) {
    // The source's 100-vs-45 length split reads as a lopsided 3-pointed star;
    // the reference gif shows ~9 fairly even needles, so the disparity is
    // narrowed here (the gif outranks the source).
    if a % 3 == 0 {
        (9.0, 90.0 + hash01(seed) * 10.0, 12.0 + hash01(seed ^ 0x11) * 4.5)
    } else {
        (5.0, 65.0 + hash01(seed) * 5.0, 9.0 + hash01(seed ^ 0x11) * 4.5)
    }
}

/// Outward travel (source pixels) of a shard that left at `speed`/frame and
/// decelerates at `accel = -(speed/life)/1.5` per frame, integrated to `frame`.
fn shard_radius_px(speed: f32, frame: f32) -> f32 {
    let accel = -(speed / SHARD_LIFE) / 1.5;
    (speed * frame + accel * frame * frame * 0.5).max(0.0)
}

pub struct ColdHitEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl ColdHitEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self { world_pos, age: 0.0 }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }

    fn collect_shards(&self, out: &mut EffectDrawList, frame: f32) {
        let alpha = fade_in_out(frame, SHARD_MAX_ALPHA, SHARD_FADE_IN, SHARD_FADE_START, SHARD_LIFE);
        if alpha <= 0.0 {
            return;
        }
        let cx = self.world_pos[0];
        let cy = self.world_pos[1] - BODY_LIFT;
        let cz = self.world_pos[2];
        let extend = (frame / SHARD_EXTEND_FRAMES).clamp(0.0, 1.0);
        for a in 0..NUM_SHARDS {
            let seed = a as u32 * 0x9E37;
            // roll = (i - 15) + random(60), i = a*40°.
            let roll_deg = (a as f32) * 40.0 - 15.0 + hash01(seed ^ 0x55) * 60.0;
            let roll_rad = roll_deg.to_radians();
            let (width, length, speed) = shard_dims(a, seed);
            let half_w = width * WORLD_SCALE;
            let full_len = length * WORLD_SCALE * extend;
            if full_len <= 0.0 {
                continue;
            }
            // The shard translates outward along its radial direction over its
            // life (outward speed with deceleration) — this is what
            // turns the centred star into long radiating spikes. The XY plane
            // is screen-aligned at the default camera (native RO: -Y up), same
            // as the Hit2 petals.
            let (sin_r, cos_r) = roll_rad.sin_cos();
            let radius = shard_radius_px(speed, frame) * WORLD_SCALE;
            let pos = [cx + radius * sin_r, cy - radius * cos_r, cz];
            out.push(EffectPrimitiveDraw::BillboardFlash {
                pos,
                // Tall thin quad: long axis (height) is the needle, taper-points
                // to both tips like a sparkle.
                size: [half_w, full_len],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: roll_rad,
                texture: NEEDLE_TEXTURE,
                color: [ICE_TINT[0], ICE_TINT[1], ICE_TINT[2], alpha],
                blend: BlendKind::Alpha,
            });
        }
    }

    fn collect_smoke(&self, out: &mut EffectDrawList, frame: f32) {
        for (i, &birth) in SMOKE_BIRTHS.iter().enumerate() {
            let local = frame - birth;
            if local < 0.0 || local > SMOKE_LIFE {
                continue;
            }
            let alpha = fade_in_out(local, SMOKE_MAX_ALPHA, SMOKE_FADE_IN, SMOKE_FADE_START, SMOKE_LIFE);
            if alpha <= 0.0 {
                continue;
            }
            // Half-extent grows fast, then slows past the change point.
            let half = if local <= SMOKE_CHANGE_FRAME {
                SMOKE_HALF_INIT + SMOKE_GROWTH_FAST * local
            } else {
                SMOKE_HALF_INIT + SMOKE_GROWTH_FAST * SMOKE_CHANGE_FRAME
                    + SMOKE_GROWTH_SLOW * (local - SMOKE_CHANGE_FRAME)
            };
            let full = 2.0 * half * WORLD_SCALE;
            // Each puff drifts a little so the two cluster lopsidedly.
            let s = i as u32 * 0x51ED;
            let ox = (hash01(s) - 0.5) * 2.0 * WORLD_SCALE;
            let oy = (hash01(s ^ 0x9) - 0.5) * 2.0 * WORLD_SCALE;
            out.push(EffectPrimitiveDraw::BillboardFlash {
                pos: [self.world_pos[0] + ox, self.world_pos[1] - BODY_LIFT + oy, self.world_pos[2]],
                size: [full, full],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: 0.0,
                texture: SMOKE_TEXTURE,
                color: [ICE_TINT[0], ICE_TINT[1], ICE_TINT[2], alpha],
                blend: BlendKind::Alpha,
            });
        }
    }
}

impl Effect for ColdHitEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.frame() > TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.frame();
        self.collect_smoke(out, frame);
        self.collect_shards(out, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn run_to(c: &mut ColdHitEffect, target_frame: f32) {
        let delta = (target_frame - c.frame()) / FRAMES_PER_SECOND;
        if delta > 0.0 {
            c.update(&EffectUpdateCtx { delta, ..Default::default() });
        }
    }

    fn draws(c: &ColdHitEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn count_tex(c: &ColdHitEffect, tex: &str) -> usize {
        draws(c)
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::BillboardFlash { texture, blend, .. }
                if *texture == tex && *blend == BlendKind::Alpha))
            .count()
    }

    #[test]
    fn emits_nine_needles_and_a_smoke_cluster() {
        let mut c = ColdHitEffect::new([0.0; 3]);
        run_to(&mut c, 3.0);
        // Phase 1: all nine ice needles; the first smoke puff (born frame 0).
        assert_eq!(count_tex(&c, NEEDLE_TEXTURE), NUM_SHARDS);
        assert!(count_tex(&c, SMOKE_TEXTURE) >= 1, "smoke born at frame 0");
        // Phase 2: needles fade out, both smoke puffs present.
        run_to(&mut c, 12.0);
        assert_eq!(count_tex(&c, SMOKE_TEXTURE), SMOKE_BIRTHS.len(), "both puffs alive");
        run_to(&mut c, SHARD_LIFE + 1.0);
        assert_eq!(count_tex(&c, NEEDLE_TEXTURE), 0, "needles gone after their life");
    }

    fn first_shard(c: &ColdHitEffect) -> (f32, f32) {
        draws(c).into_iter().find_map(|p| match p {
            EffectPrimitiveDraw::BillboardFlash { size, color, texture, .. } if texture == NEEDLE_TEXTURE => {
                Some((size[1], color[3]))
            }
            _ => None,
        }).expect("ice needle")
    }

    #[test]
    fn shards_extend_then_fade_out() {
        let mut c = ColdHitEffect::new([0.0; 3]);
        run_to(&mut c, 1.0);
        let (len_early, _) = first_shard(&c);
        run_to(&mut c, SHARD_EXTEND_FRAMES);
        let (len_full, a_hold) = first_shard(&c);
        assert!(len_full > len_early, "needle extends ({len_early} → {len_full})");
        run_to(&mut c, SHARD_FADE_START + 2.0);
        let (_, a_late) = first_shard(&c);
        assert!(a_late < a_hold, "needle fades after frame {SHARD_FADE_START}");
    }

    #[test]
    fn dies_after_duration() {
        let mut c = ColdHitEffect::new([0.0; 3]);
        run_to(&mut c, TOTAL_FRAMES + 2.0);
        assert_eq!(c.update(&EffectUpdateCtx::default()), EffectStatus::Dead);
    }
}
