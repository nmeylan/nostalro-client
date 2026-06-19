//! `EF_PORTAL4` / `EF_PORTAL5` — 4-slot wind aura.
//!
//! The aura
//! spawns 4 narrow wind cones at 90° spacing around the master. Each slot
//! widens (arc `+= 3°` per frame, capped at 120°) and rotates
//! (`+5°` per frame). After process>20 the slots drift outward by 0.10/frame
//! and the alpha fades. The arc is
//! walked in 21 steps,
//! emitting one quad per step.
//!
//! Variants:
//!   * Portal4 (`F1=0`) — size 5, `max_height = 5+2*ec` per slot, distance
//!     ≈ 4.5. Adds windwalk SFX at frame 0 and a green body tint on the
//!     master sprite during frames 5..=25.
//!   * Portal5 (`F1=1`) — size 8, `max_height = 3+2*ec`, distance ≈ 2.5.
//!     Yellow body tint on the master during frames 5..=65.
//!
//! Body-tint and SFX side effects are exposed via [`Effect::body_tint`] and
//! [`Effect::take_sfx_request`] (default trait methods returning `None` for
//! every other effect). The renderer-side wiring that consumes those is a
//! separate piece of infrastructure noted in the holder.

use crate::effect::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode,
};
use crate::effect::effect_trait::{BodyTint, Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &["cloud11.tga"];

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TOTAL_DURATION_MS: u32 = 2000;
const TOTAL_FRAMES: f32 = (TOTAL_DURATION_MS as f32) * FRAMES_PER_SECOND / 1000.0;

/// The arc is walked in 21 steps; the closing edge produces 20
/// bridging quads, so `sides` = 20 matches the rendered geometry.
const WIND_SIDES: u32 = 20;
const WIND_TEXTURE: &str = "cloud11.tga";

/// Rotation starts at 0/90/180/270 for the 4 cardinal slots.
const ROT_START_DEG: [f32; 4] = [0.0, 90.0, 180.0, 270.0];
/// `rise_angle = 80 + random(0..21)` — we substitute fixed offsets centred
/// in the original range so tests are deterministic. Visually
/// indistinguishable since each slot's rise is independent and the cones
/// are very narrow.
const RISE_ANGLE_DEG: [f32; 4] = [82.0, 88.0, 95.0, 100.0];
/// A small random distance jitter (`random(101)*0.01`) — substituted with
/// per-slot constants in `[0, jitter]` range.
const DISTANCE_JITTER_FRACTION: [f32; 4] = [0.0, 0.33, 0.66, 1.0];

#[derive(Clone, Copy)]
pub struct PortalWindConfig {
    /// `F1` parameter — preserved so callers can read the variant if needed.
    pub f1: u8,
    /// `alpha_t` mode: 1 = windwalk (quick fade, 120° arc), 2 =
    /// gust (slow fade, 180° arc), 3 = persistent gust (120° arc, ramps in and
    /// holds — no fade, no outward drift). Selects the per-frame `step` branch.
    pub alpha_t: u8,
    /// Frames the emitter lives before reporting `Dead`.
    pub duration_frames: f32,
    /// Base of the per-slot `max_height = max_height_base + max_height_step*ec`.
    pub max_height_base: f32,
    pub max_height_step: f32,
    /// `distance = distance_base + distance_step*ec + random(0..1)*distance_jitter`.
    pub distance_base: f32,
    pub distance_step: f32,
    pub distance_jitter: f32,
    /// Frame window during which the master sprite gets tinted.
    /// An empty window (e.g. `(0, -1)`) disables the tint.
    pub body_light_frames: (i32, i32),
    /// Body-light RGB (alpha is fully opaque).
    pub body_light_rgb: [u8; 3],
    /// `true` for Portal4 — plays `effect\windwalk.wav` at frame 0.
    pub play_windwalk_wav: bool,
    /// Per-segment ribbon size. The plain wind variants keep this at 1.0;
    /// the Mgdef variants set it to `nLevel`, so the funnel grows taller and
    /// wider with the buff strength.
    pub height_scale: f32,
    /// Additive tint on the wind quads.
    /// White for the untinted wind variants; coloured for Mgdef.
    pub wind_color_rgb: [u8; 3],
}

pub const PORTAL4: PortalWindConfig = PortalWindConfig {
    f1: 0,
    alpha_t: 1,
    duration_frames: TOTAL_FRAMES,
    max_height_base: 5.0,
    max_height_step: 2.0,
    distance_base: 4.5,
    distance_step: 0.0,
    distance_jitter: 0.03,
    body_light_frames: (5, 25),
    body_light_rgb: [220, 250, 220],
    play_windwalk_wav: true,
    height_scale: 1.0,
    wind_color_rgb: [255, 255, 255],
};

pub const PORTAL5: PortalWindConfig = PortalWindConfig {
    f1: 1,
    alpha_t: 1,
    duration_frames: TOTAL_FRAMES,
    max_height_base: 3.0,
    max_height_step: 2.0,
    distance_base: 2.5,
    distance_step: 0.0,
    distance_jitter: 0.01,
    body_light_frames: (5, 65),
    body_light_rgb: [250, 250, 200],
    play_windwalk_wav: false,
    height_scale: 1.0,
    wind_color_rgb: [255, 255, 255],
};

/// Gust ring used by the StormKick batch. `alpha_t=2`:
/// wider 180° arc, slow fade after `process>50`. The per-slot literals
/// (`max_height = 48 - 5*ec`, `distance = 14 - 2*ec`) are in the same inflated
/// scale as the StormKick funnel — at face value the ribbon floats ~48 units
/// up. Scaled by `STORMKICK_GUST_SCALE` so the gust hugs the funnel base, per
/// the gif. No body tint, no SFX.
pub const PORTAL_WIND2: PortalWindConfig = PortalWindConfig {
    f1: 2,
    alpha_t: 2,
    duration_frames: TOTAL_FRAMES,
    max_height_base: 48.0 * STORMKICK_GUST_SCALE,
    max_height_step: -5.0 * STORMKICK_GUST_SCALE,
    distance_base: 14.0 * STORMKICK_GUST_SCALE,
    distance_step: -2.0 * STORMKICK_GUST_SCALE,
    distance_jitter: 0.0,
    body_light_frames: (0, -1),
    body_light_rgb: [255, 255, 255],
    play_windwalk_wav: false,
    height_scale: 1.0,
    wind_color_rgb: [255, 255, 255],
};

/// Tighter gust ring. `max_height = 28 - 5*ec`,
/// `distance = 6 - 1*ec`, scaled to match.
pub const PORTAL_WIND3: PortalWindConfig = PortalWindConfig {
    f1: 3,
    alpha_t: 2,
    duration_frames: TOTAL_FRAMES,
    max_height_base: 28.0 * STORMKICK_GUST_SCALE,
    max_height_step: -5.0 * STORMKICK_GUST_SCALE,
    distance_base: 6.0 * STORMKICK_GUST_SCALE,
    distance_step: -1.0 * STORMKICK_GUST_SCALE,
    distance_jitter: 0.0,
    body_light_frames: (0, -1),
    body_light_rgb: [255, 255, 255],
    play_windwalk_wav: false,
    height_scale: 1.0,
    wind_color_rgb: [255, 255, 255],
};

/// The Mgdef (magic-defense buff) wind, 2022–
/// 2025. Identical to `PORTAL4` (same `max_height`, `distance`, `alpha_t=1`,
/// windwalk SFX, frame 5–25 body tint) except the per-segment height is
/// `nLevel` rather than 1, so the four cones rise taller with the buff strength,
/// and the wind is tinted by `nColor`. The body-light RGB tracks the original
/// game per variant (note Mgdef3's body stays green while its wind is
/// yellow — faithful to the original).
const fn mgdef(n_level: f32, wind_rgb: [u8; 3], body_rgb: [u8; 3]) -> PortalWindConfig {
    PortalWindConfig {
        f1: 0,
        alpha_t: 1,
        duration_frames: TOTAL_FRAMES,
        max_height_base: 5.0,
        max_height_step: 2.0,
        distance_base: 4.5,
        distance_step: 0.0,
        distance_jitter: 0.03,
        body_light_frames: (5, 25),
        body_light_rgb: body_rgb,
        play_windwalk_wav: true,
        height_scale: n_level,
        wind_color_rgb: wind_rgb,
    }
}

// 2022 Mgdef1 — nLevel 1, untinted white wind, pale-green body light.
pub const MGDEF1: PortalWindConfig = mgdef(1.0, [255, 255, 255], [220, 250, 220]);
// 2023 Mgdef2 — nLevel 2, green wind + green body (0x59C50A).
pub const MGDEF2: PortalWindConfig = mgdef(2.0, [89, 197, 10], [89, 197, 10]);
// 2024 Mgdef3 — nLevel 5, yellow wind (0xFFFF11), green body.
pub const MGDEF3: PortalWindConfig = mgdef(5.0, [255, 255, 17], [89, 197, 10]);
// 2025 Mgdef4 — nLevel 8, yellow wind + yellow body (0xFFFF11).
pub const MGDEF4: PortalWindConfig = mgdef(8.0, [255, 255, 17], [255, 255, 17]);

/// StormKick's funnel downscales the original literals to ~sprite height (see
/// `storm_kick.rs`'s `WORLD_SCALE`); its gust rings share that factor so they
/// stay coherent with the funnel instead of floating tens of units up.
const STORMKICK_GUST_SCALE: f32 = 0.15;

/// Portal3's wind ring at frame 2 — the wide flat halo ring around the big
/// warp portal. `alpha_t = 3`: ramps in and holds, no outward drift. Four
/// cardinal slots at distance 13/11/9/8 (outside the violet ring column) rising
/// to 2.5/5/7.5/10. The distance step isn't perfectly linear; −1.67
/// reproduces 13/11.33/9.67/8 ≈ 13/11/9/8.
pub const BIGPORTAL_WIND: PortalWindConfig = PortalWindConfig {
    f1: 0,
    alpha_t: 3,
    duration_frames: 1200.0,
    max_height_base: 2.5,
    max_height_step: 2.5,
    distance_base: 13.0,
    distance_step: -1.67,
    distance_jitter: 0.0,
    body_light_frames: (0, -1),
    body_light_rgb: [255, 255, 255],
    play_windwalk_wav: false,
    height_scale: 1.0,
    wind_color_rgb: [255, 255, 255],
};

/// Persistent variant for BigPortal2 (recall portal) — same ring, long life.
pub const BIGPORTAL_WIND2: PortalWindConfig = PortalWindConfig {
    f1: 1,
    alpha_t: 3,
    duration_frames: 5999.0,
    max_height_base: 2.5,
    max_height_step: 2.5,
    distance_base: 13.0,
    distance_step: -1.67,
    distance_jitter: 0.0,
    body_light_frames: (0, -1),
    body_light_rgb: [255, 255, 255],
    play_windwalk_wav: false,
    height_scale: 1.0,
    wind_color_rgb: [255, 255, 255],
};

#[derive(Clone, Copy)]
struct WindSlot {
    rot_start_deg: f32,
    process: f32,
    alpha_b: f32,
    distance: f32,
    full_display_angle_deg: f32,
    max_height: f32,
    rise_angle_deg: f32,
    alpha_t: u8,
}

impl WindSlot {
    fn new(slot_idx: usize, cfg: &PortalWindConfig) -> Self {
        let ec = slot_idx as f32;
        Self {
            rot_start_deg: ROT_START_DEG[slot_idx],
            process: 0.0,
            alpha_b: 0.0,
            distance: cfg.distance_base
                + cfg.distance_step * ec
                + DISTANCE_JITTER_FRACTION[slot_idx] * cfg.distance_jitter,
            full_display_angle_deg: 30.0,
            max_height: cfg.max_height_base + cfg.max_height_step * ec,
            rise_angle_deg: RISE_ANGLE_DEG[slot_idx],
            alpha_t: cfg.alpha_t,
        }
    }

    /// Per-frame slot update. `alpha_t==1` ("windwalk"): 120° arc, +10/f fade-in
    /// for 12 frames, −2/f fade after process>20, grows after process>20.
    /// `alpha_t==2` ("gust"): 180° arc, +1/f fade-in for 12 frames, −1/f fade
    /// after process>50, grows after process>12.
    fn step(&mut self) {
        self.process += 1.0;
        if self.process <= 0.0 {
            return;
        }
        self.rot_start_deg = (self.rot_start_deg + 5.0).rem_euclid(360.0);
        if self.alpha_t == 2 {
            self.full_display_angle_deg = (self.full_display_angle_deg + 3.0).min(180.0);
            if self.process > 50.0 {
                self.alpha_b = (self.alpha_b - 1.0).max(0.0);
            }
            if self.process > 12.0 {
                self.distance += 0.10;
            }
            if self.process < 12.0 {
                self.alpha_b += 1.0;
            }
        } else if self.alpha_t == 3 {
            // Persistent gust: arc opens to 120°, alpha ramps to 250 over the
            // first 12 frames and holds — no fade-out, no outward drift.
            self.full_display_angle_deg = (self.full_display_angle_deg + 3.0).min(120.0);
            if self.process < 12.0 {
                self.alpha_b = (self.alpha_b + 10.0).min(250.0);
            }
        } else {
            self.full_display_angle_deg = (self.full_display_angle_deg + 3.0).min(120.0);
            if self.process > 20.0 {
                self.alpha_b = (self.alpha_b - 2.0).max(0.0);
                self.distance += 0.10;
            }
            if self.process < 12.0 {
                self.alpha_b = (self.alpha_b + 10.0).min(250.0);
            }
        }
        if self.process > 1400.0 && self.alpha_t != 3 {
            // The terminal decay applies to every mode except the persistent
            // gust (alpha_t == 3).
            self.alpha_b = (self.alpha_b - 3.0).max(0.0);
        }
    }
}

pub struct PortalWindEffect {
    world_pos: [f32; 3],
    age_frames: f32,
    cfg: PortalWindConfig,
    slots: [WindSlot; 4],
    /// `Some(path)` until consumed by [`Effect::take_sfx_request`].
    pending_sfx: Option<&'static str>,
}

impl PortalWindEffect {
    pub fn new(world_pos: [f32; 3], cfg: PortalWindConfig) -> Self {
        let slots = [
            WindSlot::new(0, &cfg),
            WindSlot::new(1, &cfg),
            WindSlot::new(2, &cfg),
            WindSlot::new(3, &cfg),
        ];
        let pending_sfx = if cfg.play_windwalk_wav {
            Some("effect\\windwalk.wav")
        } else {
            None
        };
        Self {
            world_pos,
            age_frames: 0.0,
            cfg,
            slots,
            pending_sfx,
        }
    }

    fn step_one_frame(&mut self) {
        for s in &mut self.slots {
            s.step();
        }
    }

    fn current_frame_int(&self) -> i32 {
        self.age_frames.floor() as i32
    }
}

impl Effect for PortalWindEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let before = self.age_frames;
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let steps = (self.age_frames.floor() - before.floor()).max(0.0) as i32;
        for _ in 0..steps {
            self.step_one_frame();
        }
        if self.age_frames >= self.cfg.duration_frames {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for s in &self.slots {
            let alpha = s.alpha_b / 255.0;
            if alpha <= 0.0 {
                continue;
            }
            // Wind geometry: bottom y = -max_height; top y = -(Ry + max_height).
            // vert = sin(rise)*h; top_size = bottom_size +
            // cos(rise)*h. h is 1.0 for the plain wind, `nLevel`
            // (`height_scale`) for the Mgdef variants.
            let (sin_rise, cos_rise) = s.rise_angle_deg.to_radians().sin_cos();
            let h = self.cfg.height_scale;
            let bottom = s.distance;
            let top = s.distance + cos_rise * h;
            let vert = sin_rise * h;
            let base = [
                self.world_pos[0],
                self.world_pos[1] - s.max_height,
                self.world_pos[2],
            ];
            out.push(EffectPrimitiveDraw::Frustum {
                base,
                bottom_size: bottom,
                top_size: top,
                height: vert,
                sides: WIND_SIDES,
                arc_angle_deg: s.full_display_angle_deg,
                rotation: s.rot_start_deg.to_radians(),
                uv_repeat: 1.0,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: 0.0,
                wave_frequency: 1.0,
                wave_phase: 0.0,
                wave_mode: FrustumWaveMode::Sine,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                cull_back: false,
                texture: WIND_TEXTURE,
                color: [
                    self.cfg.wind_color_rgb[0] as f32 / 255.0,
                    self.cfg.wind_color_rgb[1] as f32 / 255.0,
                    self.cfg.wind_color_rgb[2] as f32 / 255.0,
                    alpha,
                ],
                blend: BlendKind::Additive,
            });
        }
    }

    fn body_tint(&self) -> Option<BodyTint> {
        let frame = self.current_frame_int();
        let (lo, hi) = self.cfg.body_light_frames;
        if frame >= lo && frame <= hi {
            Some(BodyTint {
                rgb: self.cfg.body_light_rgb,
            })
        } else {
            None
        }
    }

    fn take_sfx_request(&mut self) -> Option<&'static str> {
        self.pending_sfx.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step_frames(e: &mut PortalWindEffect, n: u32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    fn draws(e: &PortalWindEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn wind_frustums(prims: &[EffectPrimitiveDraw]) -> Vec<(f32, f32, f32)> {
        prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum {
                    arc_angle_deg,
                    rotation,
                    bottom_size,
                    ..
                } => Some((*arc_angle_deg, *rotation, *bottom_size)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn portal4_four_narrow_cones_at_90_offsets() {
        let mut e = PortalWindEffect::new([0.0, 0.0, 0.0], PORTAL4);
        step_frames(&mut e, 1);
        let f = wind_frustums(&draws(&e));
        assert_eq!(f.len(), 4, "4 PP_WIND slots");
        // After 1 frame: arc span = 30+3 = 33, rotation = (start+5).
        for (i, &(arc, rot, bot)) in f.iter().enumerate() {
            assert!((arc - 33.0).abs() < 0.01, "arc {arc} after 1 step");
            let expected_rot = ((ROT_START_DEG[i] + 5.0).to_radians()) as f32;
            assert!((rot - expected_rot).abs() < 1e-4, "slot {i} rotation");
            // distance starts in [4.5, 4.53]; no growth until process>20.
            assert!(
                bot >= 4.5 && bot <= 4.53,
                "slot {i} distance {bot} within init range"
            );
        }
    }

    #[test]
    fn portal4_windstorm_opens_then_fades() {
        let mut e = PortalWindEffect::new([0.0, 0.0, 0.0], PORTAL4);
        step_frames(&mut e, 12);
        // alpha_b at frame 12: ramp +10/frame for first 11 frames → 110, then
        // capped at 250. At process=12 the ramp stops adding (`process<12`),
        // so value is 110. arc span = 30 + 12*3 = 66.
        let prims = draws(&e);
        let f = wind_frustums(&prims);
        for &(arc, _rot, _bot) in &f {
            assert!((arc - 66.0).abs() < 0.01, "arc {arc} at frame 12");
        }
        let any_alpha = prims.iter().any(|p| match p {
            EffectPrimitiveDraw::Frustum { color, .. } => color[3] > 0.0,
            _ => false,
        });
        assert!(any_alpha, "alpha must be > 0 at frame 12");

        // Frame 25: distance has been growing since frame 21 (process>20), so
        // it's now base + ~0.4 (0.10/frame × 4 frames). alpha decaying.
        step_frames(&mut e, 13);
        let after = wind_frustums(&draws(&e));
        for (i, &(_arc, _rot, bot)) in after.iter().enumerate() {
            let init_lo = PORTAL4.distance_base
                + DISTANCE_JITTER_FRACTION[i] * PORTAL4.distance_jitter;
            assert!(
                bot > init_lo + 0.3,
                "slot {i} distance grew past {init_lo}+0.3 (got {bot})"
            );
        }
    }

    #[test]
    fn portal5_yellow_body_light_window_and_no_sfx() {
        let mut e = PortalWindEffect::new([0.0, 0.0, 0.0], PORTAL5);
        // Portal5 does not play the windwalk SFX.
        assert!(e.take_sfx_request().is_none());

        // Frame 0: body tint not yet active (window starts at frame 5).
        assert!(e.body_tint().is_none(), "tint inactive before frame 5");

        step_frames(&mut e, 5);
        let tint = e.body_tint().expect("tint active at frame 5");
        assert_eq!(tint.rgb, [250, 250, 200]);

        step_frames(&mut e, 60);
        let tint = e.body_tint().expect("tint active at frame 65");
        assert_eq!(tint.rgb, [250, 250, 200]);

        step_frames(&mut e, 1);
        assert!(e.body_tint().is_none(), "tint inactive after frame 65");
    }

    #[test]
    fn portal_wind2_opens_to_180_and_fades_on_gust_schedule() {
        let mut e = PortalWindEffect::new([0.0, 0.0, 0.0], PORTAL_WIND2);
        // alpha_t==2: +1/frame fade-in for 12 frames, arc grows +3/frame.
        step_frames(&mut e, 12);
        let alpha_12 = draws(&e)
            .iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::Frustum { color, .. } => Some(color[3]),
                _ => None,
            })
            .unwrap_or(0.0);
        assert!(alpha_12 > 0.0, "alpha ramped in by frame 12");

        // Arc caps at 180 (not 120 like alpha_t==1). 30 + 3*N reaches 180 at N=50.
        step_frames(&mut e, 60);
        let arcs = wind_frustums(&draws(&e));
        for (arc, _, _) in arcs {
            assert!((arc - 180.0).abs() < 0.01, "gust arc opens to 180°, got {arc}");
        }
    }

    fn wind_geometry(prims: &[EffectPrimitiveDraw]) -> Vec<(f32, [f32; 4])> {
        prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { height, color, .. } => Some((*height, *color)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn mgdef_funnel_scales_with_nlevel_and_tints_the_wind() {
        // Four cones, like Portal4. nLevel scales the per-segment height, so
        // Mgdef4 (nLevel 8) rises far taller than Mgdef1 (nLevel 1).
        let mut weak = PortalWindEffect::new([0.0, 0.0, 0.0], MGDEF1);
        let mut strong = PortalWindEffect::new([0.0, 0.0, 0.0], MGDEF4);
        step_frames(&mut weak, 10);
        step_frames(&mut strong, 10);
        let weak_h = wind_geometry(&draws(&weak));
        let strong_h = wind_geometry(&draws(&strong));
        assert_eq!(weak_h.len(), 4);
        assert_eq!(strong_h.len(), 4);
        assert!(
            strong_h[0].0 > weak_h[0].0 * 4.0,
            "nLevel 8 funnel much taller than nLevel 1: {} vs {}",
            strong_h[0].0,
            weak_h[0].0
        );
        // Mgdef1 wind is white; Mgdef2 wind is green (R and B suppressed).
        let mut green = PortalWindEffect::new([0.0, 0.0, 0.0], MGDEF2);
        step_frames(&mut green, 10);
        let g = wind_geometry(&draws(&green))[0].1;
        assert!(g[1] > g[0] && g[1] > g[2], "green wind dominant G: {g:?}");
        assert!((weak_h[0].1[0] - 1.0).abs() < 1e-6, "Mgdef1 wind is white");
    }

    #[test]
    fn mgdef_body_tint_window_is_frames_5_to_25() {
        let mut e = PortalWindEffect::new([0.0, 0.0, 0.0], MGDEF2);
        assert!(e.body_tint().is_none(), "no tint before frame 5");
        step_frames(&mut e, 5);
        assert_eq!(e.body_tint().expect("tint at frame 5").rgb, [89, 197, 10]);
        step_frames(&mut e, 21);
        assert!(e.body_tint().is_none(), "tint ends after frame 25");
    }

    #[test]
    fn portal4_windwalk_sfx_drained_once() {
        let mut e = PortalWindEffect::new([0.0, 0.0, 0.0], PORTAL4);
        assert_eq!(e.take_sfx_request(), Some("effect\\windwalk.wav"));
        assert_eq!(e.take_sfx_request(), None, "SFX consumed on first take");
    }
}
