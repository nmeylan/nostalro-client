//! `Hitline` (330) / `Hitline2` (376) / `Hitline3` (428) / `Hitline4` (429) /
//! `Hitline5` (430) / `Hitline6` (431) / `Hitline7` (434) — two families of
//! rising slash-streaks drawn with `effect\white02.bmp`.
//!
//! Both families render the streaks the same way under the hood.
//! Each streak is a
//! short filament built from the same phase-ramp path as the lightning arcs
//! ([`super::electric`]): a per-segment ramp `height[j]` drives outward reach
//! (`speed = height[j]/90 · distance`) and a sine vertical bow
//! (`y = -distance · sin(height[j]) · width`). The streak distance is fixed at
//! spawn and the ramp scrolls **up every frame** (`height[0] += 6`, shift), so
//! the slash sweeps from the ground up to head height — the render adds a
//! fixed `-5` lift (native −Y up) so the visible line reaches the target's
//! head even though the anchor is at the feet. `alpha_b` holds, then drains
//! −5/frame after frame 35.
//!
//! Faithful render details:
//! * **per-segment alpha** `alpha_b - j·10` — brightest at the streak tip;
//! * **per-segment tint**: Hitline7 grades deep blue
//!   `(120-(5-j)·22, ·, 255)`; Hitline4/5/6 grade gray-white
//!   `(255-(5-j)·30)`, with a second ×4-radius (0,0,255) halo pass at half
//!   alpha;
//! * the tube is 3 additive faces of radius `max_height` (0.05-0.10) — we
//!   draw one ribbon and fold the overdraw into the alpha.
//!
//! Each launch seeds 4 streaks; the handlers launch one set per frame of
//! their launch window (Hitline4: frames 6-9, Hitline5: 0-2, Hitline6:
//! 0-4, Hitline7: 0-1), and each streak then sits out its own negative
//! `process` before animating.
//!
//! The older `Hitline`/`Hitline2` use the separate bounce law
//! ([`HitLineBounceEffect`]): the phase **wraps** at 180° instead of capping,
//! each wrap counts a "turn" that adds a full hop of reach (`distance·2`) and
//! shrinks the next arc to 70% — the streaks skip outward like thrown embers.
//! 20 segments, alpha falloff `j·5`, no head lift, fixed 1.5 bow amplitude,
//! warm `(255, 200, 150-(20-j)·7)` tint (`Hitline2`: red tail
//! `(255, 200-(20-j)·7, ·)` plus an orange body flash on frames 10-20).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{BodyCopy, BodyTint, Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TEXTURE: &str = "white02.bmp";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FADE_START_FRAME: i32 = 35;
const FADE_RATE: f32 = 5.0;
/// Lifts the streak to head height (native −Y up).
const HEAD_LIFT: f32 = -5.0;
/// Tube radius → ribbon half-width, 1:1 — the original's
/// 0.05-0.10 tube is a hairline and the captures confirm hairline arcs.
const WIDTH_SCALE: f32 = 1.0;
/// One ribbon stands in for the original's tube faces; its rgb·alpha is
/// baked into the rgb (alpha pinned at 1 — identical under additive blend).
const FACE_OVERDRAW: f32 = 1.0;

#[derive(Clone, Copy, PartialEq)]
enum Tint {
    /// Warm yellow-orange gradient slash.
    Warm,
    /// Deep-blue gradient slash.
    Blue,
    /// Gray-white gradient slash with a wider blue glow pass.
    WhiteGlow,
}

#[derive(Clone, Copy)]
pub struct HitLineParams {
    /// Frame the first launch fires (launch window start).
    launch_start: i32,
    /// Number of launches (window length); each seeds 4 streaks.
    launch_frames: usize,
    /// Segments per streak.
    segments: usize,
    /// Tube radius.
    radius: f32,
    /// `process = -random(stagger) - stagger_base` at seed time.
    stagger: u32,
    stagger_base: i32,
    /// `width_lo + random(7)·0.1` — sine bow amplitude.
    width_lo: f32,
    distance_lo: f32,
    distance_range: f32,
    /// `alpha_b = 125 + random(alpha_range)`.
    alpha_range: u32,
    /// Pre-step the ramp 10×6° at seed (Hitline7's instant-visibility
    /// loop) so the first active frame already shows a slash.
    pre_ramp: bool,
    tint: Tint,
    /// Aim the streaks away from the target (`Hitline5`) rather than at
    /// random compass angles.
    directional: bool,
}

/// `Hitline3`: warm 5-segment slashes. The original leaves
/// this variant's launch window uninitialised; mirror
/// the sibling bounce family's 4-frame burst.
pub const HITLINE3: HitLineParams = HitLineParams {
    launch_start: 0,
    launch_frames: 4,
    segments: 5,
    radius: 0.10,
    stagger: 26,
    stagger_base: 0,
    width_lo: 0.6,
    distance_lo: 6.0,
    distance_range: 6.0,
    alpha_range: 51,
    pre_ramp: false,
    tint: Tint::Warm,
    directional: false,
};

/// `Hitline4`: 4 launches over frames 6-9, random angles.
pub const HITLINE4: HitLineParams = HitLineParams {
    launch_start: 6,
    launch_frames: 4,
    segments: 4,
    radius: 0.05,
    stagger: 31,
    stagger_base: 0,
    width_lo: 0.8,
    distance_lo: 6.0,
    distance_range: 6.0,
    alpha_range: 121,
    pre_ramp: false,
    tint: Tint::WhiteGlow,
    directional: false,
};

/// `Hitline5`: aimed away from the target, 3 launch frames.
pub const HITLINE5: HitLineParams = HitLineParams {
    launch_start: 0,
    launch_frames: 3,
    segments: 3,
    radius: 0.06,
    stagger: 21,
    stagger_base: 25,
    width_lo: 0.8,
    distance_lo: 6.0,
    distance_range: 7.0,
    alpha_range: 121,
    pre_ramp: false,
    tint: Tint::WhiteGlow,
    directional: true,
};

/// `Hitline6`: 5 launch frames, random angles, long stagger.
pub const HITLINE6: HitLineParams = HitLineParams {
    launch_start: 0,
    launch_frames: 5,
    segments: 4,
    radius: 0.07,
    stagger: 41,
    stagger_base: 30,
    width_lo: 0.6,
    distance_lo: 6.0,
    distance_range: 6.0,
    alpha_range: 121,
    pre_ramp: false,
    tint: Tint::WhiteGlow,
    directional: false,
};

/// `Hitline7`: a quick blue 2-frame burst, pre-ramped so it
/// reads instantly (the basic-hit flourish).
pub const HITLINE7: HitLineParams = HitLineParams {
    launch_start: 0,
    launch_frames: 2,
    segments: 3,
    radius: 0.10,
    stagger: 11,
    stagger_base: 0,
    width_lo: 0.6,
    distance_lo: 5.0,
    distance_range: 5.0,
    alpha_range: 51,
    pre_ramp: true,
    tint: Tint::Blue,
    directional: false,
};

struct Streak {
    rise_angle_deg: f32,
    width: f32,
    distance: f32,
    alpha_b: f32,
    process: i32,
    height: [f32; 8],
}

impl Streak {
    fn step(&mut self) {
        self.process += 1;
        if self.process <= 0 {
            return;
        }
        if self.process > FADE_START_FRAME {
            self.alpha_b = (self.alpha_b - FADE_RATE).max(0.0);
        }
        self.ramp_once();
    }

    fn ramp_once(&mut self) {
        self.height[0] = (self.height[0] + 6.0).min(180.0);
        for i in (1..self.height.len()).rev() {
            self.height[i] = self.height[i - 1];
        }
    }

    fn point(&self, j: usize) -> [f32; 3] {
        let phase = self.height[j.min(self.height.len() - 1)];
        let speed = if phase > 0.0 {
            (phase / 90.0) * self.distance
        } else {
            0.0
        };
        let (sn, cs) = self.rise_angle_deg.to_radians().sin_cos();
        [
            speed * cs,
            -self.distance * phase.to_radians().sin() * self.width + HEAD_LIFT,
            speed * sn,
        ]
    }
}

pub struct HitLineEffect {
    anchor: [f32; 3],
    params: HitLineParams,
    streaks: Vec<Streak>,
    age: f32,
    frame: u32,
}

impl HitLineEffect {
    pub fn new(anchor: [f32; 3], params: HitLineParams, aim: [f32; 3]) -> Self {
        // Direction the streaks favour: away from the target for the
        // directional variant (anchor minus target), else
        // unused.
        let base_angle = (anchor[2] - aim[2]).atan2(anchor[0] - aim[0]).to_degrees();
        let mut streaks = Vec::with_capacity(params.launch_frames * 4);
        for launch in 0..params.launch_frames {
            for ec in 0..4u32 {
                let r = noise(launch as u32 * 31 + ec);
                let rise_angle_deg = if params.directional {
                    base_angle + (r % 121) as f32 - 60.0
                } else {
                    (r % 360) as f32
                };
                let launch_delay = params.launch_start + launch as i32;
                let mut s = Streak {
                    rise_angle_deg,
                    width: params.width_lo + (noise(r ^ 0x55) % 7) as f32 * 0.1,
                    distance: params.distance_lo
                        + (noise(r ^ 0xAA) % 100) as f32 / 100.0 * params.distance_range,
                    alpha_b: 125.0 + (noise(r ^ 0x33) % params.alpha_range) as f32,
                    process: -((noise(r) % params.stagger) as i32)
                        - params.stagger_base
                        - launch_delay,
                    height: [0.0; 8],
                };
                if params.pre_ramp {
                    for _ in 0..10 {
                        s.ramp_once();
                    }
                }
                streaks.push(s);
            }
        }
        Self {
            anchor,
            params,
            streaks,
            age: 0.0,
            frame: 0,
        }
    }

    fn any_alive(&self) -> bool {
        self.streaks
            .iter()
            .any(|s| s.process <= 0 || s.alpha_b > 0.0)
    }

    /// Per-segment tint for segment `j`.
    fn segment_rgb(&self, j: usize) -> [f32; 3] {
        let fade = 5_usize.saturating_sub(j) as f32;
        match self.params.tint {
            Tint::Warm => {
                let b = ((150.0 - fade * 30.0).max(0.0)) / 255.0;
                [1.0, 200.0 / 255.0, b]
            }
            Tint::Blue => {
                let c = ((120.0 - fade * 22.0).max(0.0)) / 255.0;
                [c, c, 1.0]
            }
            Tint::WhiteGlow => {
                let c = ((255.0 - fade * 30.0).max(0.0)) / 255.0;
                [c, c, c]
            }
        }
    }

    fn push_ribbon(&self, out: &mut EffectDrawList, s: &Streak, width_scale: f32, halo: bool) {
        let n_points = self.params.segments + 1;
        let mut points = Vec::with_capacity(n_points);
        let mut colors = Vec::with_capacity(n_points);
        for j in 0..n_points {
            let p = s.point(j);
            points.push([
                self.anchor[0] + p[0],
                self.anchor[1] + p[1],
                self.anchor[2] + p[2],
            ]);
            let seg = j.min(self.params.segments - 1);
            let a_raw = (s.alpha_b - seg as f32 * 10.0).max(0.0) / 255.0;
            // Additive contribution = rgb · alpha summed over the
            // 3 tube faces; bake the whole product into rgb at alpha 1 so the
            // overdraw boost survives even where alpha would clamp.
            let boost = a_raw * FACE_OVERDRAW;
            if halo {
                colors.push([0.0, 0.0, (boost * 0.5).clamp(0.0, 1.0), 1.0]);
            } else {
                let [r, g, b] = self.segment_rgb(seg);
                colors.push([
                    (r * boost).clamp(0.0, 1.0),
                    (g * boost).clamp(0.0, 1.0),
                    (b * boost).clamp(0.0, 1.0),
                    1.0,
                ]);
            }
        }
        if points.len() < 2 {
            return;
        }
        out.push(EffectPrimitiveDraw::LineStrip {
            points,
            uv_along: 1.0,
            u_along: false,
            half_width: self.params.radius * WIDTH_SCALE * width_scale,
            texture: TEXTURE,
            color: [1.0, 1.0, 1.0, 1.0],
            colors: Some(colors),
            blend: BlendKind::Additive,
        });
    }
}

fn noise(seed: u32) -> u32 {
    let mut x = seed.wrapping_mul(0x9E3779B1).wrapping_add(0x7F4A7C15);
    x ^= x >> 15;
    x = x.wrapping_mul(0x85EBCA6B);
    x ^= x >> 13;
    x
}

impl Effect for HitLineEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let before = (self.age * FRAMES_PER_SECOND).floor();
        self.age += ctx.delta;
        let after = (self.age * FRAMES_PER_SECOND).floor();
        for _ in 0..(after - before).max(0.0) as u32 {
            self.frame += 1;
            for s in &mut self.streaks {
                s.step();
            }
        }
        if self.frame > 0 && !self.any_alive() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for s in &self.streaks {
            if s.process <= 0 || s.alpha_b <= 0.0 {
                continue;
            }
            match self.params.tint {
                Tint::Warm | Tint::Blue => self.push_ribbon(out, s, 1.0, false),
                Tint::WhiteGlow => {
                    // Wide blue halo behind, gray-white core on top (two
                    // passes: 4× radius blue glow + the slash).
                    self.push_ribbon(out, s, 4.0, true);
                    self.push_ribbon(out, s, 1.0, false);
                }
            }
        }
    }
}

/// Tints for the 20-segment bounce law.
#[derive(Clone, Copy, PartialEq)]
pub enum BounceTint {
    /// Warm `(255, 200, 150-(20-j)·7)`.
    Warm,
    /// `Hitline2` — red tail `(255, 200-(20-j)·7, ·)`.
    RedTail,
}

const BOUNCE_SEGMENTS: usize = 20;
const BOUNCE_RADIUS: f32 = 0.10;
/// Each 180° wrap shrinks the next hop to 70%.
const HOP_DECAY: f32 = 0.7;

struct BounceStreak {
    rise_angle_deg: f32,
    distance: f32,
    alpha_b: f32,
    process: i32,
    height: [f32; BOUNCE_SEGMENTS + 1],
    turns: [u8; BOUNCE_SEGMENTS + 1],
}

impl BounceStreak {
    fn step(&mut self) {
        self.process += 1;
        if self.process <= 0 {
            return;
        }
        if self.process > FADE_START_FRAME {
            self.alpha_b = (self.alpha_b - FADE_RATE).max(0.0);
        }
        self.height[0] += 8.0;
        if self.height[0] > 180.0 {
            self.height[0] -= 180.0;
            self.turns[0] = self.turns[0].saturating_add(1);
        }
        for i in (1..self.height.len()).rev() {
            self.height[i] = self.height[i - 1];
            self.turns[i] = self.turns[i - 1];
        }
    }

    fn point(&self, j: usize) -> [f32; 3] {
        let phase = self.height[j];
        let mut speed = 0.0;
        let mut radius = 1.0_f32;
        for _ in 0..self.turns[j] {
            speed += self.distance * 2.0 * radius;
            radius *= HOP_DECAY;
        }
        if phase > 0.0 {
            speed += (phase / 90.0) * self.distance * radius;
        }
        let (sn, cs) = self.rise_angle_deg.to_radians().sin_cos();
        [
            speed * cs,
            -self.distance * phase.to_radians().sin() * 1.5 * radius,
            speed * sn,
        ]
    }
}

/// `Hitline` (330, warm) / `Hitline2` (376, red tail + orange body flash) —
/// the bounce-law slash family.
pub struct HitLineBounceEffect {
    anchor: [f32; 3],
    tint: BounceTint,
    /// `Hitline2` flashes the body orange `(255,120,50)` on frames
    /// 10-20, alongside a second body copy: one pulsing
    /// vertically-stretched ghost copy, emitted via [`Effect::body_copies`].
    body_flash: bool,
    streaks: Vec<BounceStreak>,
    age: f32,
    frame: u32,
}

impl HitLineBounceEffect {
    pub fn new(anchor: [f32; 3], tint: BounceTint, body_flash: bool) -> Self {
        let mut streaks = Vec::with_capacity(16);
        for launch in 0..4u32 {
            for ec in 0..4u32 {
                let r = noise(launch * 31 + ec);
                streaks.push(BounceStreak {
                    rise_angle_deg: (r % 360) as f32,
                    distance: 6.0 + (noise(r ^ 0xAA) % 8) as f32,
                    alpha_b: 125.0 + (noise(r ^ 0x33) % 51) as f32,
                    process: -((noise(r) % 26) as i32) - launch as i32,
                    height: [0.0; BOUNCE_SEGMENTS + 1],
                    turns: [0; BOUNCE_SEGMENTS + 1],
                });
            }
        }
        Self {
            anchor,
            tint,
            body_flash,
            streaks,
            age: 0.0,
            frame: 0,
        }
    }

    fn segment_rgb(&self, j: usize) -> [f32; 3] {
        let fade = (BOUNCE_SEGMENTS - j.min(BOUNCE_SEGMENTS)) as f32;
        match self.tint {
            BounceTint::Warm => {
                let b = ((150.0 - fade * 7.0).max(0.0)) / 255.0;
                [1.0, 200.0 / 255.0, b]
            }
            BounceTint::RedTail => {
                let gb = ((200.0 - fade * 7.0).max(0.0)) / 255.0;
                [1.0, gb, gb]
            }
        }
    }
}

impl Effect for HitLineBounceEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let before = (self.age * FRAMES_PER_SECOND).floor();
        self.age += ctx.delta;
        let after = (self.age * FRAMES_PER_SECOND).floor();
        for _ in 0..(after - before).max(0.0) as u32 {
            self.frame += 1;
            for s in &mut self.streaks {
                s.step();
            }
        }
        let any_alive = self
            .streaks
            .iter()
            .any(|s| s.process <= 0 || s.alpha_b > 0.0);
        if self.frame > 0 && !any_alive {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for s in &self.streaks {
            if s.process <= 0 || s.alpha_b <= 0.0 {
                continue;
            }
            let n_points = BOUNCE_SEGMENTS + 1;
            let mut points = Vec::with_capacity(n_points);
            let mut colors = Vec::with_capacity(n_points);
            for j in 0..n_points {
                let p = s.point(j);
                points.push([
                    self.anchor[0] + p[0],
                    self.anchor[1] + p[1],
                    self.anchor[2] + p[2],
                ]);
                let seg = j.min(BOUNCE_SEGMENTS - 1);
                let a_raw = (s.alpha_b - seg as f32 * 5.0).max(0.0) / 255.0;
                let boost = a_raw * FACE_OVERDRAW;
                let [r, g, b] = self.segment_rgb(seg);
                colors.push([
                    (r * boost).clamp(0.0, 1.0),
                    (g * boost).clamp(0.0, 1.0),
                    (b * boost).clamp(0.0, 1.0),
                    1.0,
                ]);
            }
            out.push(EffectPrimitiveDraw::LineStrip {
                points,
                uv_along: 1.0,
                u_along: false,
                half_width: BOUNCE_RADIUS * WIDTH_SCALE,
                texture: TEXTURE,
                color: [1.0, 1.0, 1.0, 1.0],
                colors: Some(colors),
                blend: BlendKind::Additive,
            });
        }
    }

    fn body_tint(&self) -> Option<BodyTint> {
        (self.body_flash && (10..=20).contains(&self.frame)).then_some(BodyTint {
            rgb: [255, 120, 50],
        })
    }

    fn body_copies(&self) -> Option<Vec<BodyCopy>> {
        // One alpha-blended ghost behind the body, stretched
        // vertically by a small pulse (`add = sin(phase)·1.5 + 5`), during
        // the same orange-flash window.
        if !(self.body_flash && (10..=20).contains(&self.frame)) {
            return None;
        }
        let pulse = (self.frame as f32 * 0.8).sin() * 1.5 + 5.0;
        Some(vec![BodyCopy {
            offset_px: [0.0, 0.0],
            margin_px: 0.0,
            // Stretch relative to a rough half-sprite-height (~45 px).
            scale: [1.0, 1.0 + pulse / 45.0],
            tint: [255, 120, 50],
            alpha: 0.5,
            additive: false,
            behind: true,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectUpdateCtx {
        EffectUpdateCtx {
            delta: 1.0 / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn tick(e: &mut HitLineEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&ctx());
        }
        st
    }

    fn ribbons(e: &HitLineEffect) -> Vec<(Vec<[f32; 3]>, Vec<[f32; 4]>)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::LineStrip { points, colors, .. } => {
                    (points.clone(), colors.clone().expect("per-segment colors"))
                }
                other => panic!("expected LineStrip, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn white_glow_burst_emits_blue_halo_and_white_core_reaching_head_height() {
        // Sociable: Hitline4 = a flurry of gray-white slashes, each paired
        // with a wider blue glow ribbon, anchored on the target and rising
        // toward the head (well above the feet anchor at y=0; native −Y up →
        // negative y).
        let mut e = HitLineEffect::new([0.0, 0.0, 0.0], HITLINE4, [0.0, 0.0, 0.0]);
        tick(&mut e, 45);
        let r = ribbons(&e);
        assert!(!r.is_empty(), "burst is rendering");
        let has_core = r
            .iter()
            .any(|(_, c)| c.iter().any(|c| c[0] > 0.3 && (c[0] - c[2]).abs() < 1e-6));
        let has_halo = r
            .iter()
            .any(|(_, c)| c.iter().any(|c| c[2] > 0.2 && c[0] < 1e-6 && c[1] < 1e-6));
        assert!(
            has_core && has_halo,
            "gray-white core + blue glow both present"
        );
        // At least one streak rises above head height (y well negative).
        let highest = r
            .iter()
            .flat_map(|(pts, _)| pts.iter().map(|p| p[1]))
            .fold(0.0_f32, f32::min);
        assert!(
            highest < HEAD_LIFT,
            "streak rises past the head lift, got {highest}"
        );
    }

    #[test]
    fn hitline7_pre_ramp_shows_blue_streaks_on_the_first_active_frame() {
        let mut e = HitLineEffect::new([5.0, 0.0, 5.0], HITLINE7, [5.0, 0.0, 5.0]);
        tick(&mut e, 1);
        let r = ribbons(&e);
        assert!(!r.is_empty(), "pre-ramped streaks visible immediately");
        for (pts, colors) in &r {
            // Every ribbon leans blue and already has spatial extent.
            assert!(colors.iter().all(|c| c[2] > c[0]));
            let len: f32 = pts
                .windows(2)
                .map(|w| {
                    let d = [w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]];
                    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
                })
                .sum();
            assert!(len > 0.5, "streak has visible length, got {len}");
        }
    }

    #[test]
    fn bounce_streaks_hop_outward_with_warm_tint_and_die() {
        // Sociable: Hitline = 16 ember streaks skipping outward. After the
        // phase has wrapped a few times the streak tip must sit farther out
        // than a single arc's reach (turns add `distance·2` hops), the tint
        // is warm (red ≥ green > blue), and the burst eventually fades dead.
        let mut e = HitLineBounceEffect::new([0.0, 0.0, 0.0], BounceTint::Warm, false);
        for _ in 0..60 {
            e.update(&ctx());
        }
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert!(!list.primitives.is_empty(), "streaks render mid-life");
        let mut max_reach = 0.0_f32;
        for p in &list.primitives {
            if let EffectPrimitiveDraw::LineStrip { points, colors, .. } = p {
                for pt in points {
                    max_reach = max_reach.max((pt[0] * pt[0] + pt[2] * pt[2]).sqrt());
                }
                for c in colors.as_ref().expect("per-segment colors") {
                    assert!(c[0] >= c[1] && c[1] >= c[2], "warm gradient, got {c:?}");
                }
                let head = &colors.as_ref().unwrap()[0];
                assert!(head[0] > head[2], "head is visibly warm, got {head:?}");
            }
        }
        // One arc reaches at most distance·2 (= 28 at max distance); hops
        // accumulate past it.
        assert!(max_reach > 28.0, "turns extend the reach, got {max_reach}");
        let mut st = EffectStatus::Running;
        for _ in 0..600 {
            st = e.update(&ctx());
            if st == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(st, EffectStatus::Dead);
    }

    #[test]
    fn hitline2_flashes_the_body_orange_in_its_window() {
        let mut e = HitLineBounceEffect::new([0.0, 0.0, 0.0], BounceTint::RedTail, true);
        tick_bounce(&mut e, 5);
        assert_eq!(e.body_tint(), None, "no flash before frame 10");
        tick_bounce(&mut e, 10);
        assert_eq!(
            e.body_tint(),
            Some(BodyTint {
                rgb: [255, 120, 50]
            })
        );
        tick_bounce(&mut e, 10);
        assert_eq!(e.body_tint(), None, "flash ends after frame 20");
    }

    fn tick_bounce(e: &mut HitLineBounceEffect, frames: u32) {
        for _ in 0..frames {
            e.update(&ctx());
        }
    }

    #[test]
    fn streaks_fade_out_and_the_burst_ends() {
        let mut e = HitLineEffect::new([0.0, 0.0, 0.0], HITLINE7, [0.0, 0.0, 0.0]);
        let mut st = EffectStatus::Running;
        for _ in 0..600 {
            st = e.update(&ctx());
            if st == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(
            st,
            EffectStatus::Dead,
            "all streaks fade and the effect dies"
        );
    }
}
