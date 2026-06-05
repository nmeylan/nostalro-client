//! SphereWind family — `EF_SPHEREWIND` (346), `EF_SPHEREWIND2` (394) and
//! `EF_BABY` (408). Orbiting-ribbon energy spheres that wreathe the caster.
//!
//! Each effect spawns five times, at launch
//! offsets 0, 9, 18, 27 and 36;
//! every launch makes one primitive,
//! and every primitive carries four
//! sub-emitters. So one effect is **20
//! ribbons**. Each ribbon is a meridian arc of a sphere whose poles sit on the
//! Z axis: sampling the angle along the arc traces
//! `distance·(cos(rise)·cosA, sin(rise)·cosA, sinA)` — a great-circle tilted by
//! `rise_angle` about Z. The 20 ribbons fan their `rise_angle` across `0..171°`,
//! filling a hollow glowing globe that spins (rotation `+= 3..6°`/frame).
//!
//! * **Spherewind** (346): blue (`blue_ivy.bmp`), persistent buff aura.
//! * **Spherewind2** (394): fire (`fire_ivy.bmp`), persistent — same orbit as
//!   346 with the magenta tint.
//! * **Baby** (408): fire, transient — a tighter sphere (`distance = 5`) that
//!   expands (`distance *= 1.01`/frame), fades in then out, and drifts its
//!   `rise_angle`.
//!
//! Each ribbon is a glowing curved band oriented along its own arm vector,
//! so we render it as a strip of additive
//! [`EffectPrimitiveDraw::WorldQuad`] segments following the arc
//! (half-width = `max_height`). Colour comes from the texture (blue / fire),
//! modulated by the per-variant tint.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

pub const TEXTURES: &[&str] = &["blue_ivy.bmp", "fire_ivy.bmp"];

/// Persistent buff auras run until the buff is removed; the export clamps the
/// recording to 5 s.
pub const PERSISTENT_DURATION_MS: u32 = u32::MAX;
/// Baby's last ribbon fades out at `delay(≤20) + hold-to-70 + 25 fade` ≈ 115
/// frames; 2000 ms (120 frames) lets every ribbon finish fading.
pub const BABY_TOTAL_DURATION_MS: u32 = 2000;

/// `distance = 14` → a sphere radius of ~6 world units (about sprite
/// height); every spatial literal is scaled by this uniformly.
const WORLD_SCALE: f32 = 0.4;
/// Points sampled along each ribbon arc.
const SEGMENTS: usize = 16;
/// The five launch-angle offsets (0, 9, 18, 27, 36).
const LAUNCH_OFFSETS: [i32; 5] = [0, 9, 18, 27, 36];
const EMITTERS_PER_LAUNCH: i32 = 4;

/// Blue and fire variant tints.
const BLUE_TINT: [f32; 3] = [1.0, 175.0 / 255.0, 175.0 / 255.0];
const FIRE_TINT: [f32; 3] = [1.0, 89.0 / 255.0, 182.0 / 255.0];

#[derive(Clone, Copy)]
pub struct SpherewindParams {
    texture: &'static str,
    tint: [f32; 3],
    /// Orbital radius.
    distance: f32,
    /// Ribbon band half-width base (`max_height`); a `random(11)*step` jitter is
    /// added per ribbon.
    max_height_base: f32,
    max_height_step: f32,
    /// Vertical offset of the sphere centre (native RO −Y is up, so
    /// a negative value raises it to body height).
    height0: f32,
    /// Constant alpha for the persistent variants (100); transient
    /// `Baby` starts at 0 and ramps.
    base_alpha: f32,
    /// Transient variant: fade-in/out, expand, rise-drift, delayed start.
    transient: bool,
}

impl SpherewindParams {
    pub const fn total_duration_ms(&self) -> u32 {
        if self.transient {
            BABY_TOTAL_DURATION_MS
        } else {
            PERSISTENT_DURATION_MS
        }
    }
}

pub const SPHEREWIND: SpherewindParams = SpherewindParams {
    texture: "blue_ivy.bmp",
    tint: BLUE_TINT,
    distance: 14.0,
    max_height_base: 4.0,
    max_height_step: 0.2,
    height0: -14.0,
    base_alpha: 100.0,
    transient: false,
};

pub const SPHEREWIND2: SpherewindParams = SpherewindParams {
    texture: "fire_ivy.bmp",
    tint: FIRE_TINT,
    distance: 14.0,
    max_height_base: 4.0,
    max_height_step: 0.2,
    height0: -14.0,
    base_alpha: 100.0,
    transient: false,
};

/// `EF_SPHEREWIND3` — a tighter persistent
/// blue sphere (`distance = 10`, `max_height = 3 + random(11)*0.3`).
pub const SPHEREWIND3: SpherewindParams = SpherewindParams {
    texture: "blue_ivy.bmp",
    tint: BLUE_TINT,
    distance: 10.0,
    max_height_base: 3.0,
    max_height_step: 0.3,
    height0: -10.0,
    base_alpha: 100.0,
    transient: false,
};

pub const BABY: SpherewindParams = SpherewindParams {
    texture: "fire_ivy.bmp",
    tint: FIRE_TINT,
    distance: 5.0,
    max_height_base: 2.0,
    max_height_step: 0.2,
    height0: -10.0,
    base_alpha: 0.0,
    transient: true,
};

/// Deterministic LCG (mirrors `soul_breaker.rs` / `tripleattack.rs`).
struct Rng(u32);
impl Rng {
    fn from_seed(seed: u32) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9).wrapping_add(1))
    }
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// `random(n)` — uniform integer in `0..n`.
    fn random(&mut self, n: u32) -> u32 {
        self.next_u32() % n.max(1)
    }
}

struct Ribbon {
    rise_angle_deg: f32,
    rot_start_deg: f32,
    spin_deg: f32,
    distance: f32,
    max_height: f32,
    full_display_deg: f32,
    /// Frame counter; negative on transient ribbons so they fade in staggered.
    process: i32,
    alpha: f32,
    alive: bool,
}

pub struct SpherewindEffect {
    anchor: [f32; 3],
    params: SpherewindParams,
    ribbons: Vec<Ribbon>,
    rng: Rng,
    frame_accum: f32,
}

impl SpherewindEffect {
    pub fn new(anchor: [f32; 3], params: SpherewindParams) -> Self {
        let seed = anchor[0].to_bits() ^ anchor[2].to_bits() ^ params.distance.to_bits() ^ 0x59_E2_C1_07;
        let mut rng = Rng::from_seed(seed);
        let mut ribbons = Vec::with_capacity(LAUNCH_OFFSETS.len() * EMITTERS_PER_LAUNCH as usize);

        for time in LAUNCH_OFFSETS {
            for ec in 0..EMITTERS_PER_LAUNCH {
                // max_height = base + random(11)*0.2, rotation = random(360),
                // spin = 3 + random(4), full display = 180 + random(91) deg.
                let max_height = params.max_height_base + rng.random(11) as f32 * params.max_height_step;
                let rot_start_deg = rng.random(360) as f32;
                let spin_deg = (3 + rng.random(4)) as f32;
                let full_display_deg = (180 + rng.random(91)) as f32;
                let process = if params.transient { -(rng.random(20) as i32) } else { 0 };
                ribbons.push(Ribbon {
                    rise_angle_deg: (ec * 45 + time) as f32,
                    rot_start_deg,
                    spin_deg,
                    distance: params.distance,
                    max_height,
                    full_display_deg,
                    process,
                    alpha: params.base_alpha,
                    alive: true,
                });
            }
        }

        Self { anchor, params, ribbons, rng, frame_accum: 0.0 }
    }

    fn step_frame(&mut self) {
        let transient = self.params.transient;
        for r in &mut self.ribbons {
            if !r.alive {
                continue;
            }
            if !transient {
                // Persistent variant: spin only.
                r.rot_start_deg = wrap360(r.rot_start_deg + r.spin_deg);
                continue;
            }
            // Transient variant.
            r.process += 1;
            if r.process <= 0 {
                continue;
            }
            if r.process <= 25 {
                r.alpha += 2.0;
            } else if r.process > 70 {
                r.alpha -= 2.0;
                if r.alpha <= 0.0 {
                    r.alpha = 0.0;
                    r.alive = false;
                }
            }
            r.distance *= 1.01;
            let mut rise = r.rise_angle_deg + 1.0;
            if self.rng.random(2) == 0 {
                rise += 1.0;
            }
            r.rise_angle_deg = wrap360(rise);
            r.rot_start_deg = wrap360(r.rot_start_deg + r.spin_deg);
        }
        self.ribbons.retain(|r| r.alive);
    }
}

fn wrap360(deg: f32) -> f32 {
    let d = deg % 360.0;
    if d < 0.0 { d + 360.0 } else { d }
}

impl Effect for SpherewindEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND;
        while self.frame_accum >= 1.0 {
            self.frame_accum -= 1.0;
            self.step_frame();
        }
        if self.params.transient && self.ribbons.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for r in &self.ribbons {
            if r.alpha <= 0.0 || r.process < 0 {
                continue;
            }
            let color = [
                self.params.tint[0],
                self.params.tint[1],
                self.params.tint[2],
                (r.alpha / 255.0).clamp(0.0, 1.0),
            ];
            let rise = r.rise_angle_deg.to_radians();
            let (sr, cr) = rise.sin_cos();

            // The band's width arm, fixed per ribbon (z = 0). The band
            // is oriented on the sphere surface (NOT camera-facing), so the 20
            // arcs read as a 3-D shell instead of a flat tangle.
            let ax = -(r.max_height) * sr * WORLD_SCALE;
            let ay = r.max_height * cr * WORLD_SCALE;

            // Centre-line point at arc step `k` (arc sample plus the
            // vertical offset).
            let center = |k: usize| -> [f32; 3] {
                let count = k as f32 / SEGMENTS as f32 * r.full_display_deg;
                let ang = (r.rot_start_deg + count).to_radians();
                let (sa, ca) = ang.sin_cos();
                [
                    self.anchor[0] + (r.distance * cr * ca) * WORLD_SCALE,
                    self.anchor[1] + (r.distance * sr * ca + self.params.height0) * WORLD_SCALE,
                    self.anchor[2] + (r.distance * sa) * WORLD_SCALE,
                ]
            };

            // One quad per arc segment: top edge = centre + arm, bottom =
            // centre − arm.
            let mut m0 = center(0);
            for k in 0..SEGMENTS {
                let m1 = center(k + 1);
                let v0 = k as f32 / SEGMENTS as f32;
                let v1 = (k + 1) as f32 / SEGMENTS as f32;
                let prev_top = [m0[0] + ax, m0[1] + ay, m0[2]];
                let cur_top = [m1[0] + ax, m1[1] + ay, m1[2]];
                let cur_bot = [m1[0] - ax, m1[1] - ay, m1[2]];
                let prev_bot = [m0[0] - ax, m0[1] - ay, m0[2]];
                out.push(EffectPrimitiveDraw::WorldQuad {
                    corners: [prev_top, cur_top, cur_bot, prev_bot],
                    uv: [[0.0, v0], [0.0, v1], [1.0, v1], [1.0, v0]],
                    texture: self.params.texture,
                    color,
                    blend: BlendKind::Additive,
                });
                m0 = m1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectUpdateCtx {
        EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None }
    }

    fn tick(e: &mut SpherewindEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&ctx());
        }
        st
    }

    /// All band quads' first corner + alpha.
    fn quads(e: &SpherewindEffect) -> Vec<([f32; 3], f32)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &EffectRenderCtx {
            camera: Default::default(),
            screen_w: 256.0,
            screen_h: 256.0,
            elapsed: 0.0,
        });
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::WorldQuad { corners, color, .. } => (corners[0], color[3]),
                other => panic!("expected WorldQuad, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn spawns_twenty_ribbon_bands() {
        let e = SpherewindEffect::new([0.0; 3], SPHEREWIND);
        // 5 launches × 4 emitters × SEGMENTS quads per ribbon.
        assert_eq!(quads(&e).len(), 20 * SEGMENTS);
    }

    #[test]
    fn bands_span_a_sphere_not_a_plane() {
        // A real sphere has extent on all three axes; a flat tangle would not.
        let e = SpherewindEffect::new([0.0; 3], SPHEREWIND);
        let corners = quads(&e);
        let spread = |axis: usize| {
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for (c, _) in &corners {
                lo = lo.min(c[axis]);
                hi = hi.max(c[axis]);
            }
            hi - lo
        };
        assert!(spread(0) > 4.0 && spread(1) > 4.0 && spread(2) > 4.0, "extent x/y/z: {} {} {}", spread(0), spread(1), spread(2));
    }

    #[test]
    fn persistent_sphere_spins_and_stays_running() {
        let mut e = SpherewindEffect::new([0.0; 3], SPHEREWIND);
        let before = quads(&e)[SEGMENTS / 2].0;
        assert_eq!(tick(&mut e, 30), EffectStatus::Running);
        let after = quads(&e)[SEGMENTS / 2].0;
        let moved = (before[0] - after[0]).abs() + (before[2] - after[2]).abs();
        assert!(moved > 0.01, "band spins: {before:?} -> {after:?}");
        assert_eq!(tick(&mut e, 600), EffectStatus::Running, "buff aura persists");
    }

    #[test]
    fn baby_fades_in_expands_then_self_terminates() {
        let mut e = SpherewindEffect::new([0.0; 3], BABY);
        let r0 = e.ribbons[0].distance;
        tick(&mut e, 40); // past the delay + fade-in
        let visible: f32 = quads(&e).iter().map(|(_, a)| *a).sum();
        assert!(visible > 0.0, "faded in: {visible}");
        assert!(e.ribbons.iter().all(|r| r.distance > r0), "orbit expanded");
        assert_eq!(tick(&mut e, 300), EffectStatus::Dead, "transient ends");
    }
}
