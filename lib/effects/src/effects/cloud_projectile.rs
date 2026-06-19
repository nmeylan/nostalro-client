//! Cloud projectile family — a camera-facing square quad that spins,
//! breathes (±5% size pulse), and drags a short motion-blur ghost trail as it
//! flies. The original game renders several skills with the same look,
//! differing only
//! in trajectory, texture, tint, size, and a couple of extras — so one
//! [`CloudProjectileEffect`] holding a `Vec<Projectile>` + a [`CloudParams`]
//! table covers all of them.
//!
//! Trajectories ([`FlightMode`]):
//!   * `Overshoot` — Tanji (265): fly to the target, then blast past at ×3
//!     speed with a heading jitter while fading. No sparks.
//!   * `HitStop` — Tanji2 (412) / Alattack1-4 (2016-2019): straight flight that
//!     despawns on contact, spraying an `emp shock.tga` impact spark every 4
//!     frames. Alattack tints the orb yellow and recolours the spark.
//!   * `Homing` — Shieldboomerang (249) / Shieldboomerang2 (494): a thrown
//!     shield that flies out to the target, curves its heading back toward the
//!     caster, and fades on the return leg.
//!   * `StraightFade` — Shieldboomerang3 (520): five shields fired a few frames
//!     apart, each arriving from a random direction far out and flying inward
//!     through the impact point and on past, ramping alpha up then down.
//!
//! Textures: `blue_ivy.bmp` (spirit sphere), `emp shock.tga` (impact spark),
//! `shield_boomerang.bmp` (shield). Shieldboomerang2's original "toma" texture
//! is `axe.bmp` in the classic GRF (a thrown axe).

use std::f32::consts::{PI, TAU};

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &["blue_ivy.bmp", "emp shock.tga", "shield_boomerang.bmp", "토마.bmp"];

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

const FPS: f32 = 60.0;
const FRAME_DT: f32 = 1.0 / FPS;

/// Tanji's `process = -15`: the orb is invisible for 15 frames after spawn.
/// The shield modes launch immediately (`process = 0`).
const TANJI_SPAWN_DELAY_FRAMES: i32 = 15;
/// Spin `-15°` per frame.
const SPIN_PER_FRAME_DEG: f32 = -15.0;
/// Launches 8 units below the entity origin — chest height (−Y is up).
const HAND_Y: f32 = -8.0;
/// Alpha `+25` (of 255) per frame while ramping in (Tanji fade-in).
const FADE_IN_PER_FRAME: f32 = 25.0 / 255.0;
/// Alpha `-12` per frame on the overshoot / homing-return fade.
const FADE_OUT_PER_FRAME: f32 = 12.0 / 255.0;
/// StraightFade (520) alpha ramp: `+45/frame` while `process<=5`, `-10/frame`
/// once `process>=15`.
const RAMP_UP_PER_FRAME: f32 = 45.0 / 255.0;
const RAMP_DOWN_PER_FRAME: f32 = 10.0 / 255.0;
const RAMP_UP_UNTIL: i32 = 5;
const RAMP_DOWN_FROM: i32 = 15;
/// Radius pulse: `distance + sin(phase) * distance * 0.05`.
const SIZE_PULSE: f32 = 0.05;
const PULSE_PER_FRAME: f32 = 0.3;
/// Despawn within this xz-distance of the target (HitStop) or caster (Homing).
const HIT_RADIUS: f32 = 3.0;
/// Homing steers the heading toward the caster at 5°/frame.
const HOMING_TURN_RAD: f32 = 5.0 * PI / 180.0;
/// `process % 4 == 0` impact-spark cadence (HitStop only).
const SPARK_INTERVAL_FRAMES: u32 = 4;
const SPARK_LIFE_FRAMES: u32 = 18;
const SPARK_FADE_FRAMES: f32 = 4.0;
const SPARK_SIZE: f32 = 3.0;
/// Tanji overshoot: ×3 speed + a fixed heading jitter (the original randoms
/// ±45°; a fixed offset keeps the effect deterministic for tests).
const OVERSHOOT_JITTER_RAD: f32 = 0.35;
const OVERSHOOT_SPEED_MULT: f32 = 3.0;

/// 520 fires 5 shields, 3 frames apart, each from a random direction.
const SPRAY_COUNT: usize = 5;
const SPRAY_STAGGER_FRAMES: u32 = 3;
/// Each shield starts `speed * 15` units out along its random heading,
/// then flies inward through the centre.
const SPRAY_START_RADIUS_FRAMES: f32 = 15.0;

/// Backstop lifetime for the holder; every effect here self-terminates.
const MAX_TOTAL_FRAMES: f32 = 180.0;
pub const TOTAL_DURATION_MS: u32 = (MAX_TOTAL_FRAMES / FPS * 1000.0) as u32;

const BLUE: [f32; 3] = [150.0 / 255.0, 150.0 / 255.0, 250.0 / 255.0];
const YELLOW: [f32; 3] = [1.0, 1.0, 17.0 / 255.0];
const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
const SPARK_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const SPARK_BLUE: [f32; 4] = [10.0 / 255.0, 58.0 / 255.0, 203.0 / 255.0, 1.0]; // 0x0A3ACB
const SPARK_GREEN: [f32; 4] = [89.0 / 255.0, 197.0 / 255.0, 10.0 / 255.0, 1.0]; // 0x59C50A
const SPARK_YELLOW: [f32; 4] = [1.0, 1.0, 17.0 / 255.0, 1.0]; // 0xFFFF11

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FlightMode {
    Overshoot,
    HitStop,
    Homing,
    StraightFade,
}

#[derive(Clone, Copy)]
pub struct CloudParams {
    pub texture: &'static str,
    /// Orb tint (0..1), per variant.
    pub tint: [f32; 3],
    /// Quad corner radius (rendered side = `distance * √2`). `hit_count * 0.4`
    /// is added at construction.
    pub base_distance: f32,
    /// Travel units per frame.
    pub speed: f32,
    pub mode: FlightMode,
    /// Impact-spark colour (HitStop only), or `None`.
    pub spark: Option<[f32; 4]>,
    /// Quad blend. Glowy energy orbs (`blue_ivy.bmp`) are `Additive`; solid
    /// colour-keyed objects (the shields / axe) are `Alpha` — additive washes
    /// a solid sprite out to near-invisible over a lit map.
    pub blend: BlendKind,
}

// 265 Tanji — blue boomerang sphere, no sparks.
pub const TANJI: CloudParams =
    CloudParams { texture: "blue_ivy.bmp", tint: BLUE, base_distance: 2.0, speed: 2.0, mode: FlightMode::Overshoot, spark: None, blend: BlendKind::Additive };
// 412 Tanji2 — blue straight-flight sphere, white impact sparks.
pub const TANJI2: CloudParams =
    CloudParams { texture: "blue_ivy.bmp", tint: BLUE, base_distance: 2.0, speed: 2.0, mode: FlightMode::HitStop, spark: Some(SPARK_WHITE), blend: BlendKind::Additive };
// 2016-2019 Alattack1-4 — yellow sphere; size and spark colour per variant.
pub const ALATTACK1: CloudParams =
    CloudParams { texture: "blue_ivy.bmp", tint: YELLOW, base_distance: 2.0, speed: 2.0, mode: FlightMode::HitStop, spark: Some(SPARK_WHITE), blend: BlendKind::Additive };
pub const ALATTACK2: CloudParams =
    CloudParams { texture: "blue_ivy.bmp", tint: YELLOW, base_distance: 3.0, speed: 2.0, mode: FlightMode::HitStop, spark: Some(SPARK_BLUE), blend: BlendKind::Additive };
pub const ALATTACK3: CloudParams =
    CloudParams { texture: "blue_ivy.bmp", tint: YELLOW, base_distance: 4.0, speed: 2.0, mode: FlightMode::HitStop, spark: Some(SPARK_GREEN), blend: BlendKind::Additive };
pub const ALATTACK4: CloudParams =
    CloudParams { texture: "blue_ivy.bmp", tint: YELLOW, base_distance: 4.0, speed: 2.0, mode: FlightMode::HitStop, spark: Some(SPARK_YELLOW), blend: BlendKind::Additive };

// 249 Shieldboomerang — white shield, homing return. Source size 7; halved
// to the gif's ~1-character shield.
pub const SHIELDBOOMERANG: CloudParams =
    CloudParams { texture: "shield_boomerang.bmp", tint: WHITE, base_distance: 3.5, speed: 2.0, mode: FlightMode::Homing, spark: None, blend: BlendKind::Alpha };
// 494 Shieldboomerang2 — a thrown axe; the original's "toma" texture is
// `axe.bmp` in the classic GRF.
pub const SHIELDBOOMERANG2: CloudParams =
    CloudParams { texture: "토마.bmp", tint: WHITE, base_distance: 3.5, speed: 2.0, mode: FlightMode::Homing, spark: None, blend: BlendKind::Alpha };
// 520 Shieldboomerang3 — 5-shield fan; source size 5, speed 2.5.
pub const SHIELDBOOMERANG3: CloudParams =
    CloudParams { texture: "shield_boomerang.bmp", tint: WHITE, base_distance: 2.5, speed: 2.5, mode: FlightMode::StraightFade, spark: None, blend: BlendKind::Alpha };

#[derive(Clone, Copy)]
struct TrailSample {
    pos: [f32; 3],
    angle_deg: f32,
    pulse: f32,
}

struct Spark {
    pos: [f32; 3],
    spin_deg: f32,
    age: u32,
}

impl Spark {
    fn alpha(&self) -> f32 {
        let age = self.age as f32;
        let fade_out_start = SPARK_LIFE_FRAMES as f32 - SPARK_FADE_FRAMES;
        if age < SPARK_FADE_FRAMES {
            age / SPARK_FADE_FRAMES
        } else if age > fade_out_start {
            ((SPARK_LIFE_FRAMES as f32 - age) / SPARK_FADE_FRAMES).max(0.0)
        } else {
            1.0
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Outbound,
    Overshoot,
    Homing,
    Return,
}

/// One in-flight quad. State is integrated per fixed frame.
struct Projectile {
    params: CloudParams,
    caster: [f32; 3],
    target: [f32; 3],
    heading_rad: f32,
    step: f32,
    max_height: f32,
    y_speed: f32,
    distance: f32,
    launch_delay_frames: u32,

    pos: [f32; 3],
    base_y: f32,
    angle_deg: f32,
    pulse: f32,
    alpha: f32,
    process: i32,
    flight_frame: u32,
    phase: Phase,
    flying: bool,

    trail: Vec<TrailSample>,
    sparks: Vec<Spark>,
}

impl Projectile {
    fn new(from: [f32; 3], to: [f32; 3], heading_rad: f32, distance: f32, launch_delay_frames: u32, params: CloudParams) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let max_height = (dx * dx + dz * dz).sqrt().max(1e-3);
        let base_y = from[1] + HAND_Y;
        let alpha = match params.mode {
            FlightMode::Homing => 1.0,
            FlightMode::StraightFade => 25.0 / 255.0,
            FlightMode::Overshoot | FlightMode::HitStop => 0.0,
        };
        let spawn_delay = match params.mode {
            FlightMode::Overshoot | FlightMode::HitStop => TANJI_SPAWN_DELAY_FRAMES,
            FlightMode::Homing | FlightMode::StraightFade => 0,
        };
        Self {
            params,
            caster: [from[0], base_y, from[2]],
            target: [to[0], base_y, to[2]],
            heading_rad,
            step: params.speed,
            max_height,
            y_speed: (to[1] - from[1]) * (params.speed / max_height),
            distance,
            launch_delay_frames,
            pos: [from[0], base_y, from[2]],
            base_y,
            angle_deg: 0.0,
            pulse: 0.0,
            alpha,
            process: -spawn_delay,
            flight_frame: 0,
            phase: Phase::Outbound,
            flying: true,
            trail: Vec::with_capacity(4),
            sparks: Vec::new(),
        }
    }

    fn within(&self, p: [f32; 3]) -> bool {
        let dx = self.pos[0] - p[0];
        let dz = self.pos[2] - p[2];
        (dx * dx + dz * dz).sqrt() < HIT_RADIUS
    }

    fn advance(&mut self) {
        let (s, c) = self.heading_rad.sin_cos();
        self.pos[0] += self.step * s;
        self.pos[2] += self.step * c;
    }

    fn done(&self) -> bool {
        !self.flying && self.sparks.is_empty()
    }

    fn tick(&mut self) {
        for s in &mut self.sparks {
            s.age += 1;
        }
        self.sparks.retain(|s| s.age < SPARK_LIFE_FRAMES);

        if !self.flying {
            return;
        }
        self.process += 1;
        if self.process <= 0 {
            return;
        }

        self.angle_deg = (self.angle_deg + SPIN_PER_FRAME_DEG).rem_euclid(360.0);
        self.pulse += PULSE_PER_FRAME;

        match self.params.mode {
            FlightMode::Overshoot | FlightMode::HitStop => self.tick_tanji(),
            FlightMode::Homing => self.tick_homing(),
            FlightMode::StraightFade => self.tick_straight_fade(),
        }

        self.trail.push(TrailSample { pos: self.pos, angle_deg: self.angle_deg, pulse: self.pulse });
        if self.trail.len() > 4 {
            self.trail.remove(0);
        }
        self.flight_frame += 1;
    }

    fn tick_tanji(&mut self) {
        if self.phase == Phase::Overshoot {
            self.alpha -= FADE_OUT_PER_FRAME;
            self.base_y -= self.y_speed;
            self.pos[1] = self.base_y;
            self.advance();
            if self.alpha <= 0.0 {
                self.alpha = 0.0;
                self.flying = false;
            }
            return;
        }

        self.alpha = (self.alpha + FADE_IN_PER_FRAME).min(1.0);
        self.base_y += self.y_speed;
        self.pos[1] = self.base_y;
        self.advance();

        if self.params.mode == FlightMode::Overshoot {
            if self.flight_frame as f32 * self.step >= self.max_height {
                self.phase = Phase::Overshoot;
                self.step *= OVERSHOOT_SPEED_MULT;
                self.heading_rad += OVERSHOOT_JITTER_RAD;
            }
        } else {
            if self.params.spark.is_some() && self.flight_frame % SPARK_INTERVAL_FRAMES == 0 {
                self.sparks.push(Spark {
                    pos: self.pos,
                    spin_deg: (self.flight_frame.wrapping_mul(53) % 360) as f32,
                    age: 0,
                });
            }
            if self.within(self.target) {
                self.flying = false;
            }
        }
    }

    fn tick_homing(&mut self) {
        match self.phase {
            Phase::Outbound => {
                self.base_y += self.y_speed;
                self.pos[1] = self.base_y;
                self.advance();
                if self.flight_frame as f32 * self.step >= self.max_height {
                    self.phase = Phase::Homing;
                }
            }
            Phase::Homing => {
                let to_caster = (self.caster[0] - self.pos[0]).atan2(self.caster[2] - self.pos[2]);
                let (h, aligned) = rotate_toward(self.heading_rad, to_caster, HOMING_TURN_RAD);
                self.heading_rad = h;
                self.base_y -= self.y_speed;
                self.pos[1] = self.base_y;
                self.advance();
                if aligned {
                    self.phase = Phase::Return;
                }
            }
            Phase::Return => {
                self.base_y -= self.y_speed;
                self.pos[1] = self.base_y;
                self.advance();
                self.alpha -= FADE_OUT_PER_FRAME;
                if self.alpha <= 0.0 || self.within(self.caster) {
                    self.alpha = self.alpha.max(0.0);
                    self.flying = false;
                }
            }
            Phase::Overshoot => {}
        }
    }

    fn tick_straight_fade(&mut self) {
        if self.process <= RAMP_UP_UNTIL {
            self.alpha = (self.alpha + RAMP_UP_PER_FRAME).min(1.0);
        } else if self.process >= RAMP_DOWN_FROM {
            self.alpha -= RAMP_DOWN_PER_FRAME;
        }
        self.advance();
        if self.process >= RAMP_DOWN_FROM && self.alpha <= 0.0 {
            self.alpha = 0.0;
            self.flying = false;
        }
    }

    fn push_quad(&self, out: &mut EffectDrawList, sample: &TrailSample, alpha: f32) {
        if alpha <= 0.0 {
            return;
        }
        let radius = self.distance * (1.0 + SIZE_PULSE * sample.pulse.sin());
        let side = radius * std::f32::consts::SQRT_2;
        let t = self.params.tint;
        out.push(EffectPrimitiveDraw::Billboard {
            pos: sample.pos,
            size: [side, side],
            uv: UNIT_UV,
            rotation: sample.angle_deg.to_radians(),
            texture: self.params.texture,
            color: [t[0], t[1], t[2], alpha],
            blend: self.params.blend,
        });
    }

    fn collect_draws(&self, out: &mut EffectDrawList) {
        // Motion-blur ghost trail: lead at full alpha plus up to three lagging
        // copies (first ghost −150/255, then −25/255 each).
        const TRAIL_ALPHA_LAG: [f32; 3] = [150.0 / 255.0, 175.0 / 255.0, 200.0 / 255.0];

        if self.flying && self.process > 0 {
            for (k, lag) in TRAIL_ALPHA_LAG.iter().enumerate() {
                if let Some(sample) = self.trail.len().checked_sub(2 + k).and_then(|i| self.trail.get(i)) {
                    self.push_quad(out, sample, self.alpha - lag);
                }
            }
            if let Some(sample) = self.trail.last() {
                self.push_quad(out, sample, self.alpha);
            }
        }

        if let Some(color) = self.params.spark {
            for s in &self.sparks {
                let a = s.alpha();
                if a <= 0.0 {
                    continue;
                }
                out.push(EffectPrimitiveDraw::Billboard {
                    pos: s.pos,
                    size: [SPARK_SIZE, SPARK_SIZE],
                    uv: UNIT_UV,
                    rotation: s.spin_deg.to_radians(),
                    texture: "emp shock.tga",
                    color: [color[0], color[1], color[2], color[3] * a],
                    blend: BlendKind::Additive,
                });
            }
        }
    }
}

/// Deterministic pseudo-random heading in `[0, TAU)` for spray shield `k`, so the
/// burst looks random (the original picks a random rotation per shield) while
/// staying reproducible for tests.
fn spray_heading(k: usize) -> f32 {
    let mut s = (k as u32).wrapping_mul(2_654_435_761).wrapping_add(0x9E37_79B9);
    s ^= s >> 15;
    s = s.wrapping_mul(2_246_822_519);
    s ^= s >> 13;
    (s as f32 / u32::MAX as f32) * TAU
}

/// Rotate `current` toward `target` by at most `max_step`, returning the new
/// angle and whether it reached the target (shortest signed direction).
fn rotate_toward(current: f32, target: f32, max_step: f32) -> (f32, bool) {
    let mut diff = (target - current).rem_euclid(TAU);
    if diff > PI {
        diff -= TAU;
    }
    if diff.abs() <= max_step {
        (target, true)
    } else {
        (current + max_step * diff.signum(), false)
    }
}

pub struct CloudProjectileEffect {
    projectiles: Vec<Projectile>,
    global_frame: u32,
    time_accum: f32,
}

impl CloudProjectileEffect {
    /// Single projectile flying `from` → `to`.
    pub fn new(from: [f32; 3], to: [f32; 3], hit_count: u8, params: CloudParams) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let heading = if dx * dx + dz * dz > 1e-6 { dx.atan2(dz) } else { 0.0 };
        let distance = params.base_distance + hit_count as f32 * 0.4;
        Self { projectiles: vec![Projectile::new(from, to, heading, distance, 0, params)], global_frame: 0, time_accum: 0.0 }
    }

    /// 520: [`SPRAY_COUNT`] shields converging on `center` (the impact/target
    /// point), fired [`SPRAY_STAGGER_FRAMES`] apart. Each picks a random heading,
    /// starts far out in that direction, and flies
    /// inward through the centre and on past (no homing).
    pub fn new_spray(center: [f32; 3], params: CloudParams) -> Self {
        let offset = params.speed * SPRAY_START_RADIUS_FRAMES;
        let projectiles = (0..SPRAY_COUNT)
            .map(|k| {
                let heading = spray_heading(k);
                let (s, c) = heading.sin_cos();
                // Start `offset` units away, opposite the travel direction, so
                // advancing `+heading` carries the shield through the centre.
                let from = [center[0] - offset * s, center[1], center[2] - offset * c];
                Projectile::new(from, center, heading, params.base_distance, k as u32 * SPRAY_STAGGER_FRAMES, params)
            })
            .collect();
        Self { projectiles, global_frame: 0, time_accum: 0.0 }
    }

    fn tick(&mut self) {
        for p in &mut self.projectiles {
            if self.global_frame >= p.launch_delay_frames {
                p.tick();
            }
        }
        self.global_frame += 1;
    }
}

impl Effect for CloudProjectileEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.time_accum += ctx.delta;
        while self.time_accum >= FRAME_DT {
            self.time_accum -= FRAME_DT;
            self.tick();
        }
        if self.projectiles.iter().all(Projectile::done) {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for p in &self.projectiles {
            p.collect_draws(out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut CloudProjectileEffect, frames: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..frames {
            s = e.update(&EffectUpdateCtx { delta: FRAME_DT, camera_target: None, caster_yaw: None });
        }
        s
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn draws(e: &CloudProjectileEffect) -> Vec<([f32; 3], &'static str, [f32; 4])> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard { pos, texture, color, .. } => (*pos, *texture, *color),
                _ => panic!("cloud projectile only emits Billboard"),
            })
            .collect()
    }

    fn orbs(e: &CloudProjectileEffect, tex: &str) -> Vec<([f32; 3], [f32; 4])> {
        draws(e).into_iter().filter(|(_, t, _)| *t == tex).map(|(p, _, c)| (p, c)).collect()
    }

    fn lead(e: &CloudProjectileEffect) -> ([f32; 3], [f32; 4]) {
        orbs(e, "blue_ivy.bmp").into_iter().max_by(|a, b| a.1[3].total_cmp(&b.1[3])).unwrap()
    }

    #[test]
    fn invisible_during_spawn_delay_then_emits_orb() {
        let mut e = CloudProjectileEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 30.0], 0, TANJI);
        step(&mut e, TANJI_SPAWN_DELAY_FRAMES as u32);
        assert!(orbs(&e, "blue_ivy.bmp").is_empty(), "no orb until the 15-frame delay elapses");
        step(&mut e, 1);
        assert!(lead(&e).0[2] > 0.0, "orb has started flying toward +Z target");
    }

    #[test]
    fn flies_toward_target_with_fading_trail() {
        let mut e = CloudProjectileEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 60.0], 0, TANJI2);
        step(&mut e, TANJI_SPAWN_DELAY_FRAMES as u32 + 12);
        let o = orbs(&e, "blue_ivy.bmp");
        assert!(o.len() >= 2, "lead + at least one ghost");
        let lead_a = o.iter().map(|(_, c)| c[3]).fold(0.0_f32, f32::max);
        let min_a = o.iter().map(|(_, c)| c[3]).fold(f32::INFINITY, f32::min);
        assert!(lead_a > min_a, "ghost trail is dimmer than the lead");
        assert!(lead(&e).0[2] > 0.0, "advancing toward +Z target");
    }

    #[test]
    fn tanji2_emits_sparks_but_tanji_does_not() {
        let mut t2 = CloudProjectileEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 40.0], 0, TANJI2);
        step(&mut t2, TANJI_SPAWN_DELAY_FRAMES as u32 + 10);
        assert!(draws(&t2).iter().any(|(_, t, _)| *t == "emp shock.tga"), "Tanji2 sprays impact sparks");

        let mut t1 = CloudProjectileEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 40.0], 0, TANJI);
        step(&mut t1, TANJI_SPAWN_DELAY_FRAMES as u32 + 10);
        assert!(!draws(&t1).iter().any(|(_, t, _)| *t == "emp shock.tga"), "Tanji (boomerang) has no sparks");
    }

    #[test]
    fn alattack_orb_is_yellow_vs_tanji_blue() {
        let mut yellow = CloudProjectileEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 40.0], 0, ALATTACK1);
        let mut blue = CloudProjectileEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 40.0], 0, TANJI);
        step(&mut yellow, TANJI_SPAWN_DELAY_FRAMES as u32 + 3);
        step(&mut blue, TANJI_SPAWN_DELAY_FRAMES as u32 + 3);
        let yc = lead(&yellow).1;
        let bc = lead(&blue).1;
        assert!(yc[2] < 0.2 && yc[0] > 0.9, "Alattack orb reads yellow");
        assert!(bc[2] > bc[0], "Tanji orb reads blue");
    }

    #[test]
    fn shieldboomerang_flies_out_then_homes_back_toward_caster() {
        // White shield, no spawn delay. Track xz-distance to the caster (origin):
        // it must grow on the outbound leg, then shrink once homing kicks in.
        let mut e = CloudProjectileEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 30.0], 0, SHIELDBOOMERANG);
        let mut max_d = 0.0_f32;
        let mut returned = false;
        for _ in 0..120 {
            step(&mut e, 1);
            let p = &e.projectiles[0];
            let d = (p.pos[0] * p.pos[0] + p.pos[2] * p.pos[2]).sqrt();
            max_d = max_d.max(d);
            if max_d > 10.0 && d < max_d - 2.0 {
                returned = true;
            }
            if !p.flying {
                break;
            }
        }
        assert!(max_d > 10.0, "shield flies outward");
        assert!(returned, "shield curves back toward the caster");
        assert_eq!(draws(&e).iter().all(|(_, t, _)| *t == "shield_boomerang.bmp" || t.is_empty()), true);
    }

    #[test]
    fn shieldboomerang3_converges_five_staggered_shields() {
        let center = [0.0, 0.0, 0.0];
        let dist = |p: &Projectile| (p.pos[0] * p.pos[0] + p.pos[2] * p.pos[2]).sqrt();
        let mut e = CloudProjectileEffect::new_spray(center, SHIELDBOOMERANG3);
        assert_eq!(e.projectiles.len(), 5);
        // Each shield starts far out (≈ speed*15) in its own direction.
        assert!(e.projectiles.iter().all(|p| dist(p) > 30.0), "shields start far out");
        let h0 = e.projectiles[0].heading_rad;
        assert!(e.projectiles.iter().any(|p| (p.heading_rad - h0).abs() > 0.5), "random distinct headings");
        let before: Vec<f32> = e.projectiles.iter().map(dist).collect();
        step(&mut e, 2);
        let early = e.projectiles.iter().filter(|p| p.flight_frame > 0).count();
        assert!(early < 5, "later shields are still staggered");
        step(&mut e, 14);
        assert_eq!(e.projectiles.iter().filter(|p| p.flight_frame > 0).count(), 5, "all launched");
        // The lead shield has moved inward toward the centre.
        assert!(dist(&e.projectiles[0]) < before[0], "shield flies inward through the centre");
    }

    #[test]
    fn terminates_after_flight() {
        for params in [TANJI2, SHIELDBOOMERANG, SHIELDBOOMERANG3] {
            let mut e = match params.mode {
                FlightMode::StraightFade => CloudProjectileEffect::new_spray([0.0, 0.0, 0.0], params),
                _ => CloudProjectileEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 20.0], 0, params),
            };
            let mut status = EffectStatus::Running;
            for _ in 0..(MAX_TOTAL_FRAMES as u32) {
                status = step(&mut e, 1);
                if status == EffectStatus::Dead {
                    break;
                }
            }
            assert_eq!(status, EffectStatus::Dead, "must self-terminate");
        }
    }
}
