//! STIN wind-swirl family — `EF_STIN` (547), `EF_STIN2` (553), `EF_STIN4`
//! (621), `EF_STIN5` (624).
//!
//! Wind swirls fired toward the target.
//! Each launcher spawns one or
//! more *swirls*: a flat textured square that flies along the caster→target
//! heading while its corners spin (`rise_angle += 30°/frame`), leaving a
//! four-slot motion trail (snapshots of the head taken every 3rd
//! frame, each `alpha × 0.8`). The blue wind blob smears into the swirling
//! comma the gif shows.
//!
//! * **STIN** (547): one straight blue swirl flying out, growing + fading.
//! * **STIN2** (553): two swirls (curl + / curl −) that home around the target.
//! * **STIN4** (621): `count` swirls fanning out with random spread, homing.
//! * **STIN5** (624): five green swirls spawned over frames 0-4 at cardinal XZ
//!   offsets, each a travelling projectile flying toward the pointed direction
//!   (rotation = bearing to target) with a fading trail, lower alpha.
//!
//! The original draws each swirl as a spinning world-space square (flat XZ,
//! tilted for STIN5); every STIN gif shows it **face-on**
//! as a swirling comma, and a flat ground quad is nearly invisible at the
//! grazing camera angle — so (gif outranks that) we render a camera-facing
//! spinning billboard. The wind texture is itself a full feathery ring, so a
//! single billboard *is* the swirl; in-place casts draw the head alone (the
//! trail only smears a flying swirl). Alpha blend
//! shows the ring's whole shape; STIN tints blue, STIN5 green.
//!
//! The English `wind.tga` is absent from the classic GRF; the real
//! resource is stored under its Korean name `270바람.tga` (바람 = "wind") — a
//! feathery ring of radiating fibers, exactly the swirl the gif shows. We name
//! the English alias first with the Korean texture as the `|`-fallback.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// The swirls travel along the heading at a per-frame step, so the reach scales
/// with distance. Uses STIN's base step (0.8 units/frame) as the family
/// representative; the step grows slightly over the flight, so this is a small
/// overestimate.
pub const PROJECTILE_FLIGHT: crate::effect_queue::ProjectileFlight =
    crate::effect_queue::ProjectileFlight::ConstantSpeed {
        delay_frames: 0.0,
        units_per_frame: 0.8,
    };

/// The English-named `wind.tga` is absent from the classic GRF; the real
/// resource is the Korean-named `270바람.tga` (바람 = "wind"), a feathery
/// fibre ring matching the gif's swirl.
const WIND_TEXTURE: &str = "wind.tga|270바람.tga";

pub const TEXTURES: &[&str] = &[WIND_TEXTURE];

// The original tints `(80,80,255)`/`(80,255,80)`, but under alpha blend that
// dim blue sits below a light background and only the densest fibres read;
// lifted toward the gif's vivid glow so the whole ring shows on any scene.
const BLUE: [f32; 3] = [0.45, 0.5, 1.0];
const GREEN: [f32; 3] = [0.45, 1.0, 0.5];

/// UV in the renderer's billboard corner order — `TL, TR, BL, BR` (not CCW
/// winding). Getting this wrong bowtie-twists the texture; only matters for
/// non-symmetric textures like the wind ring.
const CARD_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

#[derive(Clone, Copy)]
pub struct StinParams {
    tint: [f32; 3],
    /// STIN2/STIN4 steer the heading toward the target each frame.
    homing: bool,
    /// Per-frame travel step along the heading.
    step: f32,
    /// STIN/STIN5 grow the travel step (`*= 1.02` rising, `*= 1.04` dying).
    grow: bool,
    /// Swirl corner radius (tuned to the gif, not the
    /// original's ~6×-too-large literal).
    distance: f32,
    distance_rand: f32,
    /// Alpha (0..255 scale) rise step / cap, and fall step.
    fade_in: f32,
    alpha_max: f32,
    fade_out: f32,
    /// Frame after which a non-homing swirl starts fading.
    fade_after: i32,
    /// Number of swirls and how their initial curl is chosen.
    spawn: SpawnKind,
}

#[derive(Clone, Copy)]
enum SpawnKind {
    /// One swirl flying straight along the heading (STIN).
    Single,
    /// Two swirls, curl `+90°` and `−90°` (STIN2).
    CurlPair,
    /// `count` swirls, alternating curl side, random spread (STIN4).
    Fan { count: usize },
    /// Five swirls spawned over frames 0-4 at small XZ offsets (STIN5).
    Staggered,
}

impl StinParams {
    pub const fn total_duration_ms(&self) -> u32 {
        // Longest swirl life: rise to alpha_max then fall to 0, plus the trail
        // tail (3 frames × 4 slots) and the staggered spawn window.
        let rise = self.alpha_max / self.fade_in;
        let fall = self.alpha_max / self.fade_out;
        let frames = self.fade_after as f32 + rise + fall + 20.0;
        ((frames / FRAMES_PER_SECOND) * 1000.0) as u32
    }
}

pub const STIN: StinParams = StinParams {
    tint: BLUE,
    homing: false,
    // Per-frame travel step.
    step: 0.8,
    grow: true,
    distance: 14.0,
    distance_rand: 0.0,
    fade_in: 25.0,
    alpha_max: 250.0,
    fade_out: 5.0,
    fade_after: 10,
    spawn: SpawnKind::Single,
};

pub const STIN2: StinParams = StinParams {
    homing: true,
    step: 2.0,
    grow: false,
    distance: 7.0,
    fade_in: 15.0,
    alpha_max: 250.0,
    fade_out: 5.0,
    fade_after: 140,
    spawn: SpawnKind::CurlPair,
    ..STIN
};

pub const STIN4: StinParams = StinParams {
    homing: true,
    step: 1.5,
    grow: false,
    distance: 4.0,
    distance_rand: 2.0,
    fade_in: 8.0,
    alpha_max: 120.0,
    fade_out: 5.0,
    fade_after: 20,
    spawn: SpawnKind::Fan { count: 8 },
    ..STIN
};

pub const STIN5: StinParams = StinParams {
    tint: GREEN,
    homing: false,
    step: 1.5,
    grow: true,
    distance: 14.0,
    distance_rand: 0.0,
    fade_in: 6.0,
    alpha_max: 60.0,
    fade_out: 3.0,
    fade_after: 10,
    spawn: SpawnKind::Staggered,
};

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
    fn random(&mut self, n: u32) -> f32 {
        (self.next_u32() % n.max(1)) as f32
    }
}

/// One trail snapshot — the head is slot 0, slots 1..3 are older copies.
#[derive(Clone, Copy)]
struct Slot {
    pos: [f32; 3],
    alpha: f32,
    spin: f32,
}

struct Swirl {
    /// Frame counter; swirls spawn with a negative value to stagger their start.
    process: i32,
    /// Heading along which the swirl travels (radians; `cos→x`, `sin→z`).
    heading: f32,
    /// Corner spin (radians), advances `+30°/frame`.
    spin: f32,
    distance: f32,
    step: f32,
    /// Homing swirl has arrived (passed the target) and is
    /// now flying straight through while it fades out.
    arrived: bool,
    slots: [Slot; 4],
    alive: bool,
}

pub struct StinEffect {
    params: StinParams,
    from: [f32; 3],
    target: [f32; 3],
    /// Whether there's a real caster→target direction. When `from == to` the
    /// effect is in-place: the head doesn't drift, so the four spinning trail
    /// copies stay concentric and their `90°`-stepped textures **combine into
    /// one ring** (the gif), instead of scattering along a flight path.
    has_dir: bool,
    swirls: Vec<Swirl>,
    frame: u32,
    frame_accum: f32,
    rng: Rng,
}

impl StinEffect {
    pub fn new(from: [f32; 3], to: [f32; 3], params: StinParams) -> Self {
        let seed = from[0].to_bits() ^ to[2].to_bits() ^ 0x5719_0A3C;
        let has_dir = (from[0] - to[0]).abs() > 1e-3 || (from[2] - to[2]).abs() > 1e-3;
        let mut e = Self {
            params,
            from,
            target: to,
            has_dir,
            swirls: Vec::new(),
            frame: 0,
            frame_accum: 0.0,
            rng: Rng::from_seed(seed),
        };
        // Cards launched at frame 0 (all but STIN5's staggered batch).
        match params.spawn {
            SpawnKind::Single => e.spawn_swirl(0.0, 0.0),
            SpawnKind::CurlPair => {
                e.spawn_swirl(0.0, 1.0); // curl +90°
                e.spawn_swirl(0.0, -1.0); // curl −90°
            }
            SpawnKind::Fan { count } => {
                for i in 0..count {
                    let curl = if i % 2 == 0 { 1.0 } else { -1.0 };
                    e.spawn_swirl(0.0, curl);
                }
            }
            SpawnKind::Staggered => {} // spawned over frames 0-4 in step_frame
        }
        e
    }

    fn base_heading(&self) -> f32 {
        let dx = self.target[0] - self.from[0];
        let dz = self.target[2] - self.from[2];
        if dx == 0.0 && dz == 0.0 {
            0.0
        } else {
            dz.atan2(dx) // cos(h) = dx, sin(h) = dz
        }
    }

    /// `xz_offset` is STIN5's per-frame seed offset; `curl` selects the
    /// initial 90°/spread offset and steer direction.
    fn spawn_swirl(&mut self, xz_offset_idx: f32, curl: f32) {
        // STIN seeds the origin Y at `pos.y - 10 + random(5)`.
        let y = self.from[1] - 10.0 + self.rng.random(5);
        let mut origin = [self.from[0], y, self.from[2]];
        if let SpawnKind::Staggered = self.params.spawn {
            // STIN5 nudges later spawns to the four cardinal offsets.
            let (ox, oz) = match xz_offset_idx as i32 {
                1 => (5.0, 0.0),
                2 => (-5.0, 0.0),
                3 => (0.0, 5.0),
                4 => (0.0, -5.0),
                _ => (0.0, 0.0),
            };
            origin[0] += ox;
            origin[2] += oz;
        }

        let mut heading = self.base_heading();
        match self.params.spawn {
            SpawnKind::CurlPair => heading += curl * std::f32::consts::FRAC_PI_2,
            SpawnKind::Fan { .. } => {
                let spread = (25.0 + self.rng.random(21)).to_radians();
                heading += curl * spread;
            }
            _ => {}
        }

        let distance = self.params.distance + self.rng.random(11) * 0.1 * self.params.distance_rand;
        let spin = self.rng.random(360).to_radians();
        let slot = Slot { pos: origin, alpha: 0.0, spin };
        self.swirls.push(Swirl {
            process: 0,
            heading,
            spin,
            distance,
            step: self.params.step,
            arrived: false,
            slots: [slot; 4],
            alive: true,
        });
    }

    fn step_frame(&mut self) {
        if let SpawnKind::Staggered = self.params.spawn {
            if self.frame < 5 {
                self.spawn_swirl(self.frame as f32, 0.0);
            }
        }

        let p = self.params;
        let target = self.target;
        let has_dir = self.has_dir;
        for swirl in &mut self.swirls {
            if !swirl.alive {
                continue;
            }
            swirl.process += 1;
            if swirl.process <= 0 {
                continue;
            }
            swirl.spin += 30.0_f32.to_radians();

            if p.homing && has_dir {
                Self::step_homing(&p, swirl, target);
            } else {
                Self::step_straight(&p, swirl);
            }

            // Translate the head along its heading — only when the effect has a
            // real direction; in-place casts keep the trail concentric.
            let (hc, hs) = (swirl.heading.cos(), swirl.heading.sin());
            let spin = swirl.spin;
            let head = &mut swirl.slots[0];
            if has_dir {
                head.pos[0] += swirl.step * hc;
                head.pos[2] += swirl.step * hs;
            }
            head.spin = spin;

            // Shift the motion trail down every third frame.
            if swirl.process % 3 == 0 {
                for i in (1..4).rev() {
                    swirl.slots[i].pos = swirl.slots[i - 1].pos;
                    swirl.slots[i].alpha = swirl.slots[i - 1].alpha * 0.8;
                    swirl.slots[i].spin = swirl.slots[i - 1].spin;
                }
            }

            if swirl.slots.iter().all(|s| s.alpha <= 0.0) && swirl.process > p.fade_after {
                swirl.alive = false;
            }
        }
        self.swirls.retain(|c| c.alive);
        self.frame += 1;
    }

    fn step_straight(p: &StinParams, swirl: &mut Swirl) {
        if swirl.process <= p.fade_after {
            swirl.slots[0].alpha = (swirl.slots[0].alpha + p.fade_in).min(p.alpha_max);
            if p.grow {
                swirl.step *= 1.02;
            }
        } else {
            swirl.slots[0].alpha -= p.fade_out;
            if p.grow {
                swirl.step *= 1.04;
            }
            if swirl.slots[0].alpha < 0.0 {
                swirl.slots[0].alpha = 0.0;
            }
        }
    }

    fn step_homing(p: &StinParams, swirl: &mut Swirl, target: [f32; 3]) {
        if swirl.process <= 20 {
            swirl.slots[0].alpha = (swirl.slots[0].alpha + p.fade_in).min(p.alpha_max);
        }
        if swirl.process <= 5 {
            return;
        }

        if !swirl.arrived {
            // Seek the target's ACTUAL position: the `curl` sign already chose
            // the launch side (±90° perpendicular), and from here the heading
            // turns toward the *live* bearing-to-target by the shortest path, so
            // the swirl curves in and crosses the target instead of orbiting a
            // fixed circle. Arrival is by distance — once reached, the head keeps
            // flying straight through (the overshoot) while it fades.
            let dx = target[0] - swirl.slots[0].pos[0];
            let dz = target[2] - swirl.slots[0].pos[2];
            if (dx * dx + dz * dz).sqrt() <= swirl.step * 2.0 {
                swirl.arrived = true;
            } else {
                swirl.heading = turn_toward(swirl.heading, dz.atan2(dx), HOMING_TURN);
            }
        } else {
            for s in swirl.slots.iter_mut() {
                s.alpha -= p.fade_out;
                if s.alpha < 0.0 {
                    s.alpha = 0.0;
                }
            }
        }
    }

}

/// Per-frame max steer (radians) for the homing swirls. A touch above a
/// gentle 5°/frame so the seek converges on the target near-field instead
/// of orbiting a fixed circle.
const HOMING_TURN: f32 = 8.0 * std::f32::consts::PI / 180.0;

/// Rotate `from` toward `to` by at most `max_step`, along the shortest arc.
fn turn_toward(from: f32, to: f32, max_step: f32) -> f32 {
    let mut d = (to - from) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    } else if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    from + d.clamp(-max_step, max_step)
}

impl Effect for StinEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND;
        while self.frame_accum >= 1.0 {
            self.frame_accum -= 1.0;
            self.step_frame();
        }
        let spawning = matches!(self.params.spawn, SpawnKind::Staggered) && self.frame < 5;
        if self.swirls.is_empty() && !spawning {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [tr, tg, tb] = self.params.tint;
        for swirl in &self.swirls {
            // The wind texture is itself a complete feathery ring — one swirl
            // already *is* the swirl. The trail's only role is the motion smear
            // of a flying swirl; in-place (`!has_dir`), stacking 4 spun copies of
            // a ring just makes moiré, so draw the head alone.
            let slots: &[Slot] = if self.has_dir { &swirl.slots } else { &swirl.slots[..1] };
            for slot in slots {
                if slot.alpha <= 0.0 {
                    continue;
                }
                // Camera-facing: a flat ground quad is invisible at
                // the grazing camera angle and every STIN gif shows the ring
                // face-on. Alpha blend renders the
                // ring's alpha-channel shape evenly tinted — additive would only
                // reveal the brightest arc and read as a rotating fragment.
                let s = swirl.distance * 2.0;
                out.push(EffectPrimitiveDraw::Billboard {
                    pos: slot.pos,
                    size: [s, s],
                    uv: CARD_UV,
                    rotation: slot.spin,
                    texture: WIND_TEXTURE,
                    color: [tr, tg, tb, (slot.alpha / 255.0).clamp(0.0, 1.0)],
                    blend: BlendKind::Alpha,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectUpdateCtx {
        EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None }
    }

    fn tick(e: &mut StinEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&ctx());
        }
        st
    }

    fn quads(e: &StinEffect) -> Vec<([f32; 4], BlendKind)> {
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
                EffectPrimitiveDraw::Billboard { color, blend, .. } => (*color, *blend),
                other => panic!("expected Billboard, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn stin_emits_blue_alpha_quads_then_dies() {
        let mut e = StinEffect::new([0.0; 3], [0.0, 0.0, 22.0], STIN);
        tick(&mut e, 6); // past fade-in
        let q = quads(&e);
        assert!(!q.is_empty(), "swirl quads emitted");
        let (color, blend) = q[0];
        assert_eq!(blend, BlendKind::Alpha);
        assert!(color[2] > color[0] && color[2] > color[1], "blue tint: {color:?}");
        // Eventually fades out and the effect reports Dead.
        assert_eq!(tick(&mut e, 300), EffectStatus::Dead);
    }

    #[test]
    fn motion_trail_grows_to_cap_then_fades() {
        let mut e = StinEffect::new([0.0; 3], [0.0, 0.0, 22.0], STIN);
        tick(&mut e, 12); // head + trail slots populated
        let peak = quads(&e).len();
        assert!(peak >= 2, "head plus at least one trail copy: {peak}");
        let peak_alpha: f32 = quads(&e).iter().map(|(c, _)| c[3]).sum();
        tick(&mut e, 30);
        let late_alpha: f32 = quads(&e).iter().map(|(c, _)| c[3]).sum();
        assert!(late_alpha < peak_alpha, "alpha fades: {peak_alpha} -> {late_alpha}");
    }

    #[test]
    fn stin5_is_green_and_travels_toward_target() {
        let mut e = StinEffect::new([0.0; 3], [0.0, 0.0, 22.0], STIN5);
        tick(&mut e, 8);
        let q = quads(&e);
        assert!(!q.is_empty());
        let (color, _) = q[0];
        assert!(color[1] > color[0] && color[1] > color[2], "green tint: {color:?}");
        // Five staggered swirls → more visible quads than a single swirl.
        assert!(e.swirls.len() >= 4, "staggered batch alive: {}", e.swirls.len());
        // §9c: a real caster→target direction makes each swirl a projectile
        // flying toward the target (+Z here), not an in-place swirl.
        assert!(e.has_dir, "directional anchor → travels");
        let head_z = e.swirls[0].slots[0].pos[2];
        tick(&mut e, 6);
        assert!(
            e.swirls[0].slots[0].pos[2] > head_z,
            "head advances toward +Z target: {head_z} -> {}",
            e.swirls[0].slots[0].pos[2]
        );
    }

    #[test]
    fn stin2_pair_launches_perpendicular_then_seeks_target() {
        let target = [0.0, 0.0, 22.0];
        let mut e = StinEffect::new([0.0; 3], target, STIN2);
        // Two swirls seeded at frame 0, launched ±90° to opposite sides of the
        // caster→target heading (so they cross the target from each side).
        assert_eq!(e.swirls.len(), 2);
        let base = e.base_heading();
        let (h0, h1) = (e.swirls[0].heading, e.swirls[1].heading);
        assert!(((h0 + h1) * 0.5 - base).abs() < 1e-4, "symmetric about heading");
        assert!((h0 - h1).abs() > 1e-3, "launched to opposite sides");
        // The seeker pulls each swirl toward the target's actual position.
        let before: f32 = e.swirls.iter().map(|s| dist(s.slots[0].pos, target)).fold(f32::MAX, f32::min);
        tick(&mut e, 40);
        let after: f32 = e.swirls.iter().map(|s| dist(s.slots[0].pos, target)).fold(f32::MAX, f32::min);
        assert!(after < before, "swirls close on the target: {before:.1} -> {after:.1}");
    }

    fn dist(p: [f32; 3], q: [f32; 3]) -> f32 {
        ((p[0] - q[0]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
    }
}
