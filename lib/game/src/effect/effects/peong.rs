//! `EF_PEONG` (id 411) — the flower-pop burst: a soft cream flower-swirl that
//! blooms, opens into a ring and fades, ringed by twinkling sparkles, used by
//! the original game's wedding / bloom effect.
//!
//! Two populations bloom together (the original starts them at
//! frame 35):
//!
//! * Rising sparkles ×N — the rising-mote variant:
//!   small twinkling motes grouped tight at the
//!   centre that rise out of the bloom while wandering left/right and growing.
//!   Alpha ramps `+5`/frame for 30 frames then `−2`/frame. Their side-to-side
//!   drift is the level-99 aura motion:
//!   horizontal position accumulates `0.15·sin(phase)` per axis each frame. We
//!   render them as the same animated `particle1.spr` sparkle the portal pad
//!   uses (gif outranks a plain `whitelight.tga` quad).
//! * Swirl motes — `peong{1,2,3}.tga` ×16 (4 sectors × the 4-texture call
//!   group): 4 emitters per
//!   call, each a billboard sitting 4 units up at radius offset 4, drifting
//!   outward along its sector angle while arcing up-then-down in y
//!   (`sin(process·2°)·1.5`). Alpha ramps `+2`/frame to ~50 then `−1` after
//!   frame 70. 4 sectors × 4 calls × 4 emitters = 64 motes — the cream flower
//!   that stays filled, then opens into a ring as the motes drift apart.
//!
//! The burst motes render as camera-facing quads, sized by
//! the `distance` field, white-tinted. Sibling `peong_up.rs` implements the
//! StormKick fountain variant.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
/// The original spawns both populations at frame 35, but the reference
/// capture begins at the bloom — so we spawn at frame 0 and let the per-mote
/// `process` stagger (−random(26)/−random(11)) ease them in over ~0.4 s.
const START_FRAME: f32 = 0.0;

/// Original literals (radius 2.5–4, billboard size 2.5–4.7) sized so the
/// overlapping motes bloom into a roughly character-sized soft flower; ratios preserved.
const WORLD_SCALE: f32 = 1.6;

const RISING_COUNT: usize = 20;
const BURST_SECTORS: usize = 4;
const BURST_PER_SECTOR: usize = 16;

const PEONG_TEXTURES: [&str; 4] = ["peong1.tga", "peong2.tga", "peong3.tga", "peong2.tga"];

pub const TEXTURES: &[&str] = &["peong1.tga", "peong2.tga", "peong3.tga"];

/// The rising motes render as the portal pad's animated sparkle sprite.
pub const SPARKLE_SPRITE: &str = "data/sprite/이팩트/particle1";
pub const SPRITES: &[&str] = &[SPARKLE_SPRITE];

/// Frames per sparkle-sprite cell — fast enough that each mote visibly twinkles
/// over its life.
const SPARKLE_ANIM_TICKS: f32 = 3.0;

/// Burst motes ramp to peak alpha by frame ~25, hold, then fade out after frame
/// 70 (−1/frame from 50). With the slower outward drift the ring keeps
/// opening to ~frame 150 ≈ 2.5 s before the last motes fade; pad slightly.
pub const TOTAL_DURATION_MS: u32 = 2600;

struct Rng(u32);
impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next_u32() as f32 / u32::MAX as f32)
    }
    fn int(&mut self, n: u32) -> i32 {
        (self.next_u32() % n) as i32
    }
}

/// A single mote, shared by both populations; the [`Kind`] decides whether it
/// draws as a sparkle sprite or a swirl billboard.
struct Mote {
    pos: [f32; 3],
    size: f32,
    alpha: f32,
    process: i32,
    texture: &'static str,
    kind: Kind,
}

enum Kind {
    /// Rising sparkle: a twinkling sparkle that rises out of the centre while
    /// wandering left/right and growing — the same motion the
    /// level-99 aura sparkles use: each frame the horizontal
    /// position accumulates `0.15·sin(phase)` per axis (the phase walking on its
    /// own), so the sparkle drifts side-to-side rather than in a straight line.
    Sparkle {
        rise: f32,
        /// Per-frame horizontal accumulation amplitude (0.15).
        amp: f32,
        /// Walking wobble phases (radians) and their per-frame step, per axis.
        wx_phase: f32,
        wx_speed: f32,
        wz_phase: f32,
        wz_speed: f32,
        /// World units added to the sparkle's half-size each frame.
        grow: f32,
    },
    /// Swirl mote: outward sector drift + y arc — the cream flower.
    Burst { base_y: f32, drift: [f32; 2] },
}

impl Mote {
    fn step(&mut self) {
        self.process += 1;
        if self.process <= 0 {
            return;
        }
        match &mut self.kind {
            Kind::Sparkle { rise, amp, wx_phase, wx_speed, wz_phase, wz_speed, grow } => {
                *wx_phase += *wx_speed;
                *wz_phase += *wz_speed;
                // Accumulating sine drift → a random left/right wander.
                self.pos[0] += *amp * wx_phase.sin();
                self.pos[2] += *amp * wz_phase.sin();
                self.pos[1] -= *rise; // native −Y = up
                self.size += *grow;
                // Rising mote: alpha += 5 while process < 30, else -= 2.
                if self.process < 30 {
                    self.alpha = (self.alpha + 5.0 / 255.0).min(1.0);
                } else {
                    self.alpha -= 2.0 / 255.0;
                }
            }
            Kind::Burst { base_y, drift } => {
                if self.process <= 90 {
                    self.pos[0] += drift[0];
                    self.pos[2] += drift[1];
                    let arc = (self.process as f32 * 2.0).to_radians().sin();
                    // Vertical arc lift of ~1.5 units at its peak.
                    self.pos[1] = *base_y - arc * 1.5 * WORLD_SCALE;
                }
                // Swirl mote: alpha += 2 while process <= 25, -= 1 after 70.
                if self.process <= 25 {
                    self.alpha = (self.alpha + 2.0 / 255.0).min(1.0);
                }
                if self.process > 70 {
                    self.alpha -= 1.0 / 255.0;
                }
            }
        }
    }

    fn dead(&self) -> bool {
        self.process > 25 && self.alpha <= 0.0
    }
}

pub struct PeongEffect {
    anchor: [f32; 3],
    motes: Vec<Mote>,
    age_frames: f32,
    /// Fractional-frame accumulator so motes step at a fixed 60 Hz regardless
    /// of the render frame rate.
    step_accumulator: f32,
    spawned: bool,
}

impl PeongEffect {
    pub fn new(anchor: [f32; 3]) -> Self {
        Self { anchor, motes: Vec::new(), age_frames: 0.0, step_accumulator: 0.0, spawned: false }
    }

    fn spawn(&mut self) {
        let [ax, ay, az] = self.anchor;
        let seed = ax.to_bits() ^ az.to_bits() ^ 0x9E37_79B9;
        let mut rng = Rng(seed | 1);

        // Rising sparkles ×N grouped tight at the centre, each
        // rising while wandering left/right and growing (the level-99 aura
        // motion, radius 0.8 ≈ hugging the axis).
        for _ in 0..RISING_COUNT {
            let angle = rng.range(0.0, std::f32::consts::TAU);
            let radius = rng.range(0.0, 0.8) * WORLD_SCALE;
            let up = rng.range(0.0, 3.0) * WORLD_SCALE; // initial random rise
            // Start small and grow as they rise — point-sized twinkles.
            let size = (0.2 + rng.range(0.0, 0.2)) * WORLD_SCALE;
            let grow = 0.012 * WORLD_SCALE;
            // Sparkles climb well above the bloom over their life.
            let rise = (0.10 + rng.range(0.0, 0.08)) * WORLD_SCALE;
            // The original accumulates `0.15·sin(phase)` per axis per frame; we use
            // a smaller amplitude so the sparkles wander only gently left/right
            // rather than swinging wide. Phases walk at small per-sparkle rates
            // so the drift wanders rather than oscillating in lockstep.
            let amp = 0.08 * WORLD_SCALE;
            self.motes.push(Mote {
                pos: [ax + angle.cos() * radius, ay - up, az + angle.sin() * radius],
                size,
                alpha: 0.0,
                process: -rng.int(26),
                texture: SPARKLE_SPRITE,
                kind: Kind::Sparkle {
                    rise,
                    amp,
                    wx_phase: rng.range(0.0, std::f32::consts::TAU),
                    wx_speed: rng.range(0.03, 0.09),
                    wz_phase: rng.range(0.0, std::f32::consts::TAU),
                    wz_speed: rng.range(0.03, 0.09),
                    grow,
                },
            });
        }

        // Swirl motes ×16 → 64 starburst motes across 4 sectors.
        for sector in 0..BURST_SECTORS {
            let base = sector as f32 * 90.0;
            for k in 0..BURST_PER_SECTOR {
                let angle = (base + rng.range(0.0, 90.0)).to_radians();
                // Start clustered near centre (heavy overlap → a bright filled
                // bloom) and let the outward drift open it into a ring over the
                // life — matching the reference's filled-then-ring evolution.
                let offset = (2.0 + rng.range(0.0, 0.8)) * WORLD_SCALE;
                let size = (2.5 + rng.range(0.0, 2.0)) * WORLD_SCALE; // size 2.5–4.5
                let base_y = ay - 4.0 * WORLD_SCALE; // 4 units up
                // Drift outward slowly so the cluster stays a *filled* flower
                // for most of its life and only opens into a ring near the end
                // — the reference keeps the bloom solid far longer than a fast
                // expansion would (user: "too fast").
                let drift = [angle.cos() * 0.05 * WORLD_SCALE, angle.sin() * 0.05 * WORLD_SCALE];
                self.motes.push(Mote {
                    pos: [ax + angle.cos() * offset, base_y, az + angle.sin() * offset],
                    size,
                    alpha: 0.0,
                    process: -rng.int(11),
                    texture: PEONG_TEXTURES[k % 4],
                    kind: Kind::Burst { base_y, drift },
                });
            }
        }
        self.spawned = true;
    }
}

impl Effect for PeongEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if !self.spawned {
            if self.age_frames < START_FRAME {
                return EffectStatus::Running;
            }
            self.spawn();
        }
        // The original game ticks effect logic on a fixed-step 60 Hz catch-up
        // loop, so the mote `process` counter must advance per 1/60 s — not
        // once per render frame (which would run fast on high-refresh displays).
        self.step_accumulator += ctx.delta * FRAMES_PER_SECOND;
        while self.step_accumulator >= 1.0 {
            for m in &mut self.motes {
                m.step();
            }
            self.step_accumulator -= 1.0;
        }
        self.motes.retain(|m| !m.dead());
        if self.spawned && self.motes.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for m in &self.motes {
            if m.alpha <= 0.0 {
                continue;
            }
            match m.kind {
                Kind::Sparkle { .. } => {
                    let cell = (m.process.max(0) as f32 / SPARKLE_ANIM_TICKS) as usize;
                    out.push(EffectPrimitiveDraw::SpriteParticle {
                        sprite_path: SPARKLE_SPRITE,
                        position: m.pos,
                        action_index: 0,
                        motion_index: cell,
                        size_scale: m.size,
                        // Untinted, additive: the sprite's own warm/blue cells
                        // give the multicoloured twinkle the reference shows.
                        color: [1.0, 1.0, 1.0, m.alpha],
                        blend: BlendKind::Additive,
                        aim_target: None,
                        no_depth: false,
                    });
                }
                Kind::Burst { .. } => {
                    out.push(EffectPrimitiveDraw::Billboard {
                        pos: m.pos,
                        size: [m.size, m.size],
                        uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                        rotation: 0.0,
                        texture: m.texture,
                        color: [1.0, 1.0, 1.0, m.alpha],
                        // Alpha blend. The cream `peong*` swirl
                        // textures read as soft smoke/dust this way; additive
                        // would wash them into bright round "bubbles".
                        blend: BlendKind::Alpha,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(e: &mut PeongEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        st
    }

    fn draws(e: &PeongEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &EffectRenderCtx {
            camera: Default::default(),
            screen_w: 256.0,
            screen_h: 256.0,
            elapsed: 0.0,
        });
        list.primitives
    }

    /// Rising y (sum) of the sparkle sprites — native RO up is −Y.
    fn sparkle_y_sum(prims: &[EffectPrimitiveDraw]) -> f32 {
        prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { position, .. } => Some(position[1]),
                _ => None,
            })
            .sum()
    }

    #[test]
    fn blooms_in_promptly_after_spawn() {
        let mut e = PeongEffect::new([0.0; 3]);
        // Per-mote stagger (−random(26)) eases motes in over the first ~0.4 s.
        tick(&mut e, 30);
        assert!(!draws(&e).is_empty(), "cluster has bloomed in");
    }

    #[test]
    fn sparkles_surround_the_swirl_flower_at_peak() {
        let mut e = PeongEffect::new([0.0; 3]);
        tick(&mut e, 60);
        let prims = draws(&e);
        // PeongUp population draws as the twinkling sparkle sprite...
        assert!(
            prims.iter().any(|p| matches!(
                p,
                EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == SPARKLE_SPRITE
            )),
            "twinkling sparkle sprites present"
        );
        // ...and the Peong population draws as the cream swirl billboards.
        assert!(
            prims.iter().any(|p| matches!(
                p,
                EffectPrimitiveDraw::Billboard { texture, .. } if texture.starts_with("peong")
            )),
            "swirl-flower billboards present"
        );
    }

    #[test]
    fn sparkles_drift_upward() {
        let mut e = PeongEffect::new([0.0; 3]);
        tick(&mut e, 40);
        let y_early = sparkle_y_sum(&draws(&e));
        tick(&mut e, 8);
        let y_late = sparkle_y_sum(&draws(&e));
        assert!(y_late < y_early, "sparkles drift up (native −Y): {y_early} -> {y_late}");
    }

    #[test]
    fn effect_dies_after_the_bloom() {
        let mut e = PeongEffect::new([0.0; 3]);
        assert_eq!(tick(&mut e, 300), EffectStatus::Dead);
    }

    #[test]
    fn mote_advance_is_frame_rate_independent() {
        // The original game ticks effect logic at a fixed 60 Hz; a coarse
        // (low-fps) delta must reach the same state as fine 60-fps steps over
        // the same wall-clock time, or the bloom runs fast on high-refresh
        // displays.
        let half_second = |dt: f32, steps: u32| {
            let mut e = PeongEffect::new([0.0; 3]);
            for _ in 0..steps {
                e.update(&EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None });
            }
            e.motes.iter().map(|m| m.process).sum::<i32>()
        };
        // 30 frames @ 60 fps vs 15 ticks of double-length dt — both 0.5 s.
        let fine = half_second(1.0 / 60.0, 30);
        let coarse = half_second(2.0 / 60.0, 15);
        assert_eq!(fine, coarse, "process advances by wall-clock, not per call");
    }
}
