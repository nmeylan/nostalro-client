//! Chemical streak family — `EF_CHEMICALPROTECTION` (300), `EF_CHEMICAL2`
//! (303), `EF_CHEMICAL3` (439), `EF_CHEMICAL4` (623), `EF_CHEMICAL2DASH`
//! (512), `EF_MGATTACK2` (2015) and `EF_SMATK1..4` (2009-2012).
//!
//! Three streak shapes. Each launcher fires
//! **once** at its spawn frame and seeds `4` sub-emitters;
//! a dual-call dispatch multiplies that into
//! `4 × num_calls` streaks spawned in a single burst that then fade out — it
//! is **not** continuous emission.
//!
//! * **Protection**: 8 spokes billboarded into
//!   the screen plane radiating from the entity
//!   (lifted 9 units) at angles `ec*90 + {30,60}`, length `distance = 100`.
//!   White core (±3°) + light-blue edge (±5°), additive.
//! * **Chemical2**: a static flat band spanning
//!   `±distance` along the caster→target heading, offset laterally by
//!   `∓(15 + ec*2.5)`, white, alpha. Launches at frame 40.
//! * **Chemical3**: a short flat ribbon along the
//!   heading whose `[near, far] = [scroll-100, scroll+length-100]` slides as
//!   `scroll` wraps 0→200 — the segment travels from behind the
//!   caster to ahead (bottom→top on screen). The lateral offset `±(15+rand)` puts the
//!   two calls on opposite sides (left/right groups).
//!   The colour group flag → `line3.tga` blue / `line3y.tga` yellow|green.
//!
//! Both CHEMICAL2 and CHEMICAL3 build the same flat quad ([`band_corners`]);
//! they differ only in `near`/`far`, lift, fade rates and scrolling.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{
    BodyTint, CameraShake, CameraView, Effect, EffectRenderCtx, EffectUpdateCtx,
};

const FRAMES_PER_SECOND: f32 = 60.0;

/// 100 — the spoke length (Protection) and the band
/// half-span (CHEMICAL2/3). Engine units ≈ world units; the viewer frames
/// ~125 units of height, so a streak fills most of it.
const DISTANCE: f32 = 100.0;

/// Centre lifted 9 units (CHEMICALPROTECTION / CHEMICAL2). Native RO `-Y` is up.
const CENTER_RISE: f32 = 9.0;

/// CHEMICAL3 lifts the ribbon 6 units above the
/// caster's feet (`-Y` up).
const STREAK_LIFT: f32 = 6.0;

const QUAKE_AMPLITUDE: f32 = 1.6;
const QUAKE_DURATION_MS: u32 = 600;

/// Visibility multiplier for the Protection spokes — the `shockwave_c` texture
/// is dark, so the faithful alpha reads too faint under additive blend.
const PROTECTION_BOOST: f32 = 2.5;

/// Fixed UV for the four corners with default texture span:
/// `(0,1) (1,1) (1,0) (0,0)`.
/// Both `band_corners` and `spoke_corners` return verts in this corner order.
const TEI_UV: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChemKind {
    Protection,
    Chemical2,
    Chemical3,
}

#[derive(Clone, Copy)]
pub struct ChemicalParams {
    kind: ChemKind,
    /// Base group for CHEMICAL3 (0 / 10 / 20). Selects texture,
    /// palette and the size/speed bands. Unused for the other kinds.
    group: u8,
    /// Number of `(call, call+1)` dispatch pairs. Streaks = `pairs * 8`
    /// (two calls × four emitters). Protection/CHEMICAL2 always use one pair.
    pairs: u32,
    /// Parent frame the primitive launches on (0, except CHEMICAL2 at 40).
    spawn_frame: f32,
    /// `(rgb, (start, end))` body-tint window on the parent frame counter.
    body_tint: Option<([u8; 3], (f32, f32))>,
    /// Camera-shake frame, if any.
    quake_at: Option<f32>,
    /// `(wave, frame)` one-shot SFX, if any.
    sfx: Option<(&'static str, f32)>,
}

/// Per-frame alpha rule (255-scale): rise `+in` while `process <= until`,
/// fall `-out` while `process > after`.
struct FadeRule {
    fade_in: f32,
    until: f32,
    fade_out: f32,
    after: f32,
}

const fn fade_rule(kind: ChemKind, group: u8) -> FadeRule {
    match kind {
        ChemKind::Protection => FadeRule { fade_in: 18.0, until: 10.0, fade_out: 15.0, after: 20.0 },
        ChemKind::Chemical2 => FadeRule { fade_in: 4.0, until: 20.0, fade_out: 2.0, after: 50.0 },
        ChemKind::Chemical3 => {
            if group < 10 {
                FadeRule { fade_in: 12.0, until: 10.0, fade_out: 4.0, after: 20.0 }
            } else {
                FadeRule { fade_in: 20.0, until: 10.0, fade_out: 2.0, after: 20.0 }
            }
        }
    }
}

impl ChemicalParams {
    /// Wall-clock end = spawn frame + rise/hold + linear fade-out tail.
    pub const fn total_duration_ms(&self) -> u32 {
        let f = fade_rule(self.kind, self.group);
        let peak = f.fade_in * f.until;
        let frames = self.spawn_frame + f.after + peak / f.fade_out;
        ((frames / FRAMES_PER_SECOND) * 1000.0) as u32
    }

    const fn texture(&self) -> &'static str {
        match self.kind {
            ChemKind::Protection => "shockwave_c.bmp",
            ChemKind::Chemical2 => "slash01.tga",
            ChemKind::Chemical3 => {
                if self.group < 10 {
                    "line3.tga"
                } else {
                    "line3y.tga"
                }
            }
        }
    }
}

const YELLOW_GLOW: [u8; 3] = [255, 255, 150];

pub const CHEMICALPROTECTION: ChemicalParams = ChemicalParams {
    kind: ChemKind::Protection,
    group: 0,
    pairs: 1,
    spawn_frame: 0.0,
    body_tint: None,
    quake_at: None,
    sfx: None,
};
pub const MGATTACK2: ChemicalParams =
    ChemicalParams { body_tint: Some((YELLOW_GLOW, (0.0, 60.0))), ..CHEMICALPROTECTION };
pub const CHEMICAL2: ChemicalParams = ChemicalParams {
    kind: ChemKind::Chemical2,
    group: 0,
    pairs: 1,
    spawn_frame: 40.0,
    body_tint: Some((YELLOW_GLOW, (20.0, 120.0))),
    quake_at: Some(44.0),
    sfx: Some(("effect\\chemical2.wav", 44.0)),
};
pub const CHEMICAL3: ChemicalParams = ChemicalParams {
    kind: ChemKind::Chemical3,
    group: 0,
    pairs: 4,
    spawn_frame: 0.0,
    body_tint: None,
    quake_at: None,
    sfx: None,
};
pub const CHEMICAL4: ChemicalParams = ChemicalParams {
    group: 20,
    sfx: Some(("effect\\chemical4.wav", 28.0)),
    ..CHEMICAL3
};
pub const CHEMICAL2DASH: ChemicalParams = ChemicalParams {
    group: 10,
    body_tint: Some((YELLOW_GLOW, (20.0, 100.0))),
    quake_at: Some(44.0),
    ..CHEMICAL3
};
pub const SMATK1: ChemicalParams = ChemicalParams {
    group: 10,
    pairs: 1,
    body_tint: Some((YELLOW_GLOW, (0.0, 60.0))),
    ..CHEMICAL3
};
pub const SMATK2: ChemicalParams = ChemicalParams { pairs: 4, ..SMATK1 };
pub const SMATK3: ChemicalParams = ChemicalParams { pairs: 8, ..SMATK1 };
pub const SMATK4: ChemicalParams = ChemicalParams { pairs: 8, ..SMATK1 };

/// Textures referenced by this family (viewer pre-load / coverage).
pub const TEXTURES: &[&str] = &["line3.tga", "line3y.tga", "slash01.tga", "shockwave_c.bmp"];

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
        (self.next_u32() % n) as f32
    }
}

struct Streak {
    process: f32,
    /// Alpha, 0..255 scale (no cap; group≥10 peaks at 200).
    alpha: f32,
    /// Lateral offset (± = group side). CHEMICAL2/3.
    lateral: f32,
    /// Half-width across the heading.
    width: f32,
    /// Scroll position 0..200. CHEMICAL3 only.
    scroll: f32,
    /// Ribbon length along the heading. CHEMICAL3 only.
    length: f32,
    /// Scroll speed. CHEMICAL3 only.
    speed: f32,
    /// Resolved quad tint (0..1).
    color: [f32; 3],
    /// Protection spoke screen angle (radians).
    screen_angle: f32,
}

pub struct ChemicalEffect {
    params: ChemicalParams,
    fade: FadeRule,
    /// Groups ≥ 10 accelerate (scroll speed += 0.05/frame).
    accelerate: bool,
    center: [f32; 3],
    /// Caster→target heading basis on the ground plane.
    forward: [f32; 3],
    lateral_dir: [f32; 3],
    streaks: Vec<Streak>,
    spawned: bool,
    age: f32,
    frame_accum: f32,
    rng: Rng,
    shake_fired: bool,
    sfx_fired: bool,
    pending_sfx: Option<&'static str>,
}

impl ChemicalEffect {
    /// Point-anchored entry (radial Protection has no direction).
    pub fn new(anchor: [f32; 3], params: ChemicalParams) -> Self {
        Self::new_dir(anchor, anchor, params)
    }

    /// Trail entry — `from` is the caster, `to` the target the streaks aim at.
    pub fn new_dir(from: [f32; 3], to: [f32; 3], params: ChemicalParams) -> Self {
        let yaw = (to[0] - from[0]).atan2(to[2] - from[2]);
        let (s, c) = yaw.sin_cos();
        let forward = [s, 0.0, c];
        // Perpendicular to the heading: (cos(yaw), 0, -sin(yaw)).
        let lateral_dir = [c, 0.0, -s];
        let rise = match params.kind {
            ChemKind::Protection | ChemKind::Chemical2 => CENTER_RISE,
            ChemKind::Chemical3 => 0.0,
        };
        let center = [from[0], from[1] - rise, from[2]];
        let seed = from[0].to_bits() ^ to[2].to_bits() ^ 0x0C4E_3A1C;
        Self {
            fade: fade_rule(params.kind, params.group),
            accelerate: params.group >= 10,
            params,
            center,
            forward,
            lateral_dir,
            streaks: Vec::new(),
            spawned: false,
            age: 0.0,
            frame_accum: 0.0,
            rng: Rng::from_seed(seed),
            shake_fired: false,
            sfx_fired: false,
            pending_sfx: None,
        }
    }

    /// Seed the one-shot burst of `pairs * 8` sub-emitters: four emitters
    /// × two dispatch calls × `pairs`.
    fn spawn(&mut self) {
        for _ in 0..self.params.pairs {
            for call in 0..2 {
                for ec in 0..4 {
                    let st = match self.params.kind {
                        ChemKind::Protection => {
                            let time = if call == 0 { 30.0 } else { 60.0 };
                            self.spawn_protection(ec, time)
                        }
                        ChemKind::Chemical2 => self.spawn_chemical2(ec, call),
                        ChemKind::Chemical3 => self.spawn_chemical3(call),
                    };
                    self.streaks.push(st);
                }
            }
        }
    }

    fn spawn_protection(&self, ec: u32, time: f32) -> Streak {
        Streak {
            process: 0.0,
            alpha: 0.0,
            lateral: 0.0,
            width: 0.0,
            scroll: 0.0,
            length: DISTANCE,
            speed: 0.0,
            color: [1.0, 1.0, 1.0],
            screen_angle: (ec as f32 * 90.0 + time).to_radians(),
        }
    }

    fn spawn_chemical2(&self, ec: u32, call: u32) -> Streak {
        // Lateral offset `∓(15 + ec*2.5)` — opposite sides per call.
        let lateral = if call == 0 {
            -15.0 - ec as f32 * 2.5
        } else {
            15.0 + ec as f32 * 2.5
        };
        Streak {
            process: 0.0,
            alpha: 0.0,
            lateral,
            width: 1.0, // max_height
            scroll: 0.0,
            length: 0.0,
            speed: 0.0,
            color: [1.0, 1.0, 1.0],
            screen_angle: 0.0,
        }
    }

    fn spawn_chemical3(&mut self, call: u32) -> Streak {
        let g = self.params.group;
        // Lateral offset `±(15 + random(11))` — dir 0 left, dir 1 right.
        let mag = 15.0 + self.rng.random(11);
        let lateral = if call == 0 { -mag } else { mag };
        let (max_height, length, speed) = if g < 10 {
            (0.3 + self.rng.random(4) * 0.1, 20.0 + self.rng.random(31), 1.5 + self.rng.random(21) * 0.1)
        } else if g < 20 {
            (0.5 + self.rng.random(6) * 0.1, 40.0 + self.rng.random(31), 1.0 + self.rng.random(11) * 0.1)
        } else {
            (0.4 + self.rng.random(4) * 0.1, 30.0 + self.rng.random(31), 1.5 + self.rng.random(21) * 0.1)
        };
        let flag = self.rng.random(3) as u8 + g; // 0..2 / 10..12 / 20..22
        Streak {
            process: 0.0,
            alpha: 0.0,
            lateral,
            width: max_height,
            scroll: self.rng.random(200),
            length,
            speed,
            color: chemical3_color(flag),
            screen_angle: 0.0,
        }
    }

    fn step_frame(&mut self) {
        self.age += 1.0;
        if !self.spawned && self.age >= self.params.spawn_frame {
            self.spawn();
            self.spawned = true;
        }

        let scroll = self.params.kind == ChemKind::Chemical3;
        let accel = self.accelerate;
        let f = &self.fade;
        for s in &mut self.streaks {
            s.process += 1.0;
            if scroll {
                if accel {
                    s.speed += 0.05;
                }
                s.scroll += s.speed;
                if s.scroll >= 200.0 {
                    s.scroll -= 200.0;
                }
            }
            if s.process <= f.until {
                s.alpha += f.fade_in;
            } else if s.process > f.after {
                s.alpha = (s.alpha - f.fade_out).max(0.0);
            }
        }

        if let Some((wave, at)) = self.params.sfx {
            if !self.sfx_fired && self.age >= at {
                self.sfx_fired = true;
                self.pending_sfx = Some(wave);
            }
        }
    }

    /// Flat quad shared by CHEMICAL2/CHEMICAL3: a strip
    /// from `near` to `far` along the heading, spanning `lateral ± width`
    /// across it, at height `y`. Corner order matches the fixed
    /// four-corner UV above.
    fn band_corners(&self, near: f32, far: f32, lateral: f32, width: f32, y: f32) -> [[f32; 3]; 4] {
        let f = self.forward;
        let l = self.lateral_dir;
        let a = lateral - width;
        let b = lateral + width;
        let pt = |along: f32, across: f32| {
            [self.center[0] + f[0] * along + l[0] * across, y, self.center[2] + f[2] * along + l[2] * across]
        };
        [pt(far, a), pt(near, a), pt(near, b), pt(far, b)]
    }

    /// Camera-facing spoke for one CHEMICALPROTECTION wedge:
    /// a thin triangle from the entity out to `length`
    /// at `screen_angle ± half_deg`, in the camera's right/up basis.
    fn spoke_corners(&self, cam: &CameraView, angle: f32, half_deg: f32, length: f32) -> [[f32; 3]; 4] {
        let (right, up) = camera_right_up(cam);
        let far = |a: f32| {
            let (s, c) = a.sin_cos();
            [
                self.center[0] + (right[0] * c + up[0] * s) * length,
                self.center[1] + (right[1] * c + up[1] * s) * length,
                self.center[2] + (right[2] * c + up[2] * s) * length,
            ]
        };
        let h = half_deg.to_radians();
        [self.center, far(angle - h), far(angle + h), self.center]
    }
}

fn chemical3_color(flag: u8) -> [f32; 3] {
    let rgb = match flag {
        0 => [225.0, 225.0, 255.0],
        1 => [205.0, 205.0, 255.0],
        2 => [185.0, 185.0, 255.0],
        10 => [255.0, 255.0, 225.0],
        11 => [255.0, 255.0, 205.0],
        12 => [255.0, 255.0, 185.0],
        20 => [225.0, 255.0, 225.0],
        21 => [205.0, 255.0, 205.0],
        _ => [185.0, 255.0, 185.0],
    };
    [rgb[0] / 255.0, rgb[1] / 255.0, rgb[2] / 255.0]
}

/// Camera right/up unit vectors, for the screen-facing billboard geometry
/// the Protection spokes are drawn with.
fn camera_right_up(cam: &CameraView) -> ([f32; 3], [f32; 3]) {
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
    };
    let norm = |v: [f32; 3]| {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
        [v[0] / len, v[1] / len, v[2] / len]
    };
    let view = norm(sub(cam.target, cam.eye));
    let right = norm(cross(view, cam.up));
    let up = norm(cross(right, view));
    (right, up)
}

impl Effect for ChemicalEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND;
        while self.frame_accum >= 1.0 {
            self.frame_accum -= 1.0;
            self.step_frame();
        }
        // Alive until the burst has spawned and every streak has faded out.
        if !self.spawned {
            return EffectStatus::Running;
        }
        let alive = self.streaks.iter().any(|s| s.process <= self.fade.after || s.alpha > 0.0);
        if alive { EffectStatus::Running } else { EffectStatus::Dead }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        match self.params.kind {
            ChemKind::Protection => {
                for s in &self.streaks {
                    if s.alpha <= 0.0 {
                        continue;
                    }
                    // `shockwave_c` is a dark texture under additive blend, so
                    // the faithful alpha (~0.7 peak) reads too faint — boost
                    // it for visibility, clamped at 1.0.
                    let a = (s.alpha / 255.0 * PROTECTION_BOOST).min(1.0);
                    // White core (±3°, half alpha) + light-blue edge (±5°),
                    // both additive. The UV maps the apex
                    // (entity end) to texture U=0 — `shockwave_c`'s dim
                    // edge — so the spoke is faint at the entity and brightens
                    // outward, rather than glowing where the 8 spokes meet.
                    out.push(EffectPrimitiveDraw::WorldQuad {
                        corners: self.spoke_corners(&ctx.camera, s.screen_angle, 3.0, s.length),
                        uv: TEI_UV,
                        texture: "shockwave_c.bmp",
                        color: [1.0, 1.0, 1.0, (a * 0.5).min(1.0)],
                        blend: BlendKind::Additive,
                        // The spokes are drawn as a near-plane
                        // 2D overlay — they radiate from the entity through the
                        // ground, so ignore depth or the floor swallows them.
                        no_depth: true,
                    });
                    out.push(EffectPrimitiveDraw::WorldQuad {
                        corners: self.spoke_corners(&ctx.camera, s.screen_angle, 5.0, s.length),
                        uv: TEI_UV,
                        texture: "shockwave_c.bmp",
                        color: [130.0 / 255.0, 130.0 / 255.0, 1.0, a],
                        blend: BlendKind::Additive,
                        no_depth: true,
                    });
                }
            }
            // CHEMICAL2: static band ±DISTANCE; CHEMICAL3: scrolling segment.
            // Both alpha-blended.
            ChemKind::Chemical2 | ChemKind::Chemical3 => {
                let lift = if self.params.kind == ChemKind::Chemical3 { STREAK_LIFT } else { 0.0 };
                let y = self.center[1] - lift;
                for s in &self.streaks {
                    if s.alpha <= 0.0 {
                        continue;
                    }
                    let (near, far) = if self.params.kind == ChemKind::Chemical3 {
                        (s.scroll - DISTANCE, s.scroll + s.length - DISTANCE)
                    } else {
                        (-DISTANCE, DISTANCE)
                    };
                    let [r, g, b] = s.color;
                    out.push(EffectPrimitiveDraw::WorldQuad {
                        corners: self.band_corners(near, far, s.lateral, s.width, y),
                        uv: TEI_UV,
                        texture: self.params.texture(),
                        color: [r, g, b, s.alpha / 255.0],
                        blend: BlendKind::Alpha,
                        no_depth: false,
                    });
                }
            }
        }
    }

    fn body_tint(&self) -> Option<BodyTint> {
        let (rgb, (lo, hi)) = self.params.body_tint?;
        (self.age >= lo && self.age <= hi).then_some(BodyTint { rgb })
    }

    fn take_camera_shake(&mut self) -> Option<CameraShake> {
        match self.params.quake_at {
            Some(at) if !self.shake_fired && self.age >= at => {
                self.shake_fired = true;
                Some(CameraShake { amplitude: QUAKE_AMPLITUDE, duration_ms: QUAKE_DURATION_MS })
            }
            _ => None,
        }
    }

    fn take_sfx_request(&mut self) -> Option<&'static str> {
        self.pending_sfx.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(frames: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: frames / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 256.0, screen_h: 256.0, elapsed: 0.0 }
    }

    fn step(e: &mut ChemicalEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&ctx(1.0));
        }
        st
    }

    fn draws(e: &ChemicalEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn chemical3_spawns_one_burst_of_four_per_call_in_two_groups() {
        // 4 pairs × 2 calls × 4 emitters = 32 streaks, half on each side.
        let mut e = ChemicalEffect::new_dir([0.0; 3], [0.0, 0.0, 10.0], CHEMICAL3);
        step(&mut e, 6);
        assert_eq!(e.streaks.len(), 32);
        let left = e.streaks.iter().filter(|s| s.lateral < 0.0).count();
        let right = e.streaks.iter().filter(|s| s.lateral > 0.0).count();
        assert_eq!((left, right), (16, 16), "two left/right groups");
        // Blue palette, alpha-blended.
        for p in &draws(&e) {
            match p {
                EffectPrimitiveDraw::WorldQuad { color, blend: BlendKind::Alpha, texture, .. } => {
                    assert_eq!(*texture, "line3.tga");
                    assert!(color[2] >= color[0], "blue dominant {color:?}");
                }
                _ => panic!("expected alpha WorldQuad"),
            }
        }
    }

    #[test]
    fn chemical3_segment_travels_along_aim_toward_target() {
        let mut e = ChemicalEffect::new_dir([0.0; 3], [0.0, 0.0, 10.0], CHEMICAL4);
        step(&mut e, 6);
        let z0 = e.streaks[0].scroll;
        step(&mut e, 10);
        assert!(e.streaks[0].scroll != z0, "scroll advances (segment travels +aim)");
        // Green-ish palette for the 20-group.
        assert!(draws(&e).iter().any(|p| matches!(p,
            EffectPrimitiveDraw::WorldQuad { color, .. } if color[1] >= color[0] && color[1] >= color[2])));
    }

    #[test]
    fn protection_is_eight_camera_facing_spoke_pairs_above_feet() {
        let mut e = ChemicalEffect::new([0.0; 3], CHEMICALPROTECTION);
        step(&mut e, 6);
        assert_eq!(e.streaks.len(), 8);
        assert_eq!(draws(&e).len(), 16, "core + edge per spoke");
        assert!(e.center[1] < 0.0, "centre raised above feet (m_pos.y-=9)");
    }

    #[test]
    fn chemical2_launches_at_frame_40_then_static_band() {
        let mut e = ChemicalEffect::new_dir([0.0; 3], [0.0, 0.0, 10.0], CHEMICAL2);
        step(&mut e, 30);
        assert!(e.streaks.is_empty(), "nothing before frame 40");
        assert_eq!(e.body_tint(), Some(BodyTint { rgb: YELLOW_GLOW }), "tint window 20-120");
        step(&mut e, 20); // past 40 (spawn) and 44 (quake)
        assert_eq!(e.streaks.len(), 8);
        assert!(e.take_camera_shake().is_some(), "quake fires once");
        assert!(e.take_camera_shake().is_none());
    }

    #[test]
    fn density_scales_with_pairs_and_burst_dies_out() {
        let mut small = ChemicalEffect::new_dir([0.0; 3], [0.0, 0.0, 10.0], SMATK1); // 1 pair → 8
        let mut big = ChemicalEffect::new_dir([0.0; 3], [0.0, 0.0, 10.0], SMATK3); // 8 pairs → 64
        step(&mut small, 6);
        step(&mut big, 6);
        assert_eq!((small.streaks.len(), big.streaks.len()), (8, 64));
        // The one-shot burst fades out and the effect ends.
        assert_eq!(step(&mut big, 400), EffectStatus::Dead);
        assert!(draws(&big).is_empty());
    }
}
