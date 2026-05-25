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
    /// Base of the per-slot `max_height = max_height_base + max_height_step*ec`.
    pub max_height_base: f32,
    pub max_height_step: f32,
    /// `distance = distance_base + random(0..1) * distance_jitter`.
    pub distance_base: f32,
    pub distance_jitter: f32,
    /// Frame window during which the master sprite gets tinted.
    pub body_light_frames: (i32, i32),
    /// Body-light RGB (alpha is fully opaque).
    pub body_light_rgb: [u8; 3],
    /// `true` for Portal4 — plays `effect\windwalk.wav` at frame 0.
    pub play_windwalk_wav: bool,
}

pub const PORTAL4: PortalWindConfig = PortalWindConfig {
    f1: 0,
    max_height_base: 5.0,
    max_height_step: 2.0,
    distance_base: 4.5,
    distance_jitter: 0.03,
    body_light_frames: (5, 25),
    body_light_rgb: [220, 250, 220],
    play_windwalk_wav: true,
};

pub const PORTAL5: PortalWindConfig = PortalWindConfig {
    f1: 1,
    max_height_base: 3.0,
    max_height_step: 2.0,
    distance_base: 2.5,
    distance_jitter: 0.01,
    body_light_frames: (5, 65),
    body_light_rgb: [250, 250, 200],
    play_windwalk_wav: false,
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
}

impl WindSlot {
    fn new(slot_idx: usize, cfg: &PortalWindConfig) -> Self {
        let ec = slot_idx as f32;
        Self {
            rot_start_deg: ROT_START_DEG[slot_idx],
            process: 0.0,
            alpha_b: 0.0,
            distance: cfg.distance_base
                + DISTANCE_JITTER_FRACTION[slot_idx] * cfg.distance_jitter,
            full_display_angle_deg: 30.0,
            max_height: cfg.max_height_base + cfg.max_height_step * ec,
            rise_angle_deg: RISE_ANGLE_DEG[slot_idx],
        }
    }

    /// Wind primitive per-frame, alpha-type == 1 branch only (Portal4/5 are both F1
    /// in {0,1} → "windwalk").
    fn step(&mut self) {
        self.process += 1.0;
        if self.process <= 0.0 {
            return;
        }
        self.rot_start_deg = (self.rot_start_deg + 5.0).rem_euclid(360.0);
        // terminal-alpha != 2 branch — cap at 120.
        self.full_display_angle_deg = (self.full_display_angle_deg + 3.0).min(120.0);
        if self.process > 20.0 {
            self.alpha_b = (self.alpha_b - 2.0).max(0.0);
            self.distance += 0.10;
        }
        if self.process > 1400.0 {
            // terminal-alpha != 3 here, so the terminal decay applies.
            self.alpha_b = (self.alpha_b - 3.0).max(0.0);
        }
        if self.process < 12.0 {
            self.alpha_b = (self.alpha_b + 10.0).min(250.0);
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
        if self.age_frames >= TOTAL_FRAMES {
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
            // Wind render: bottom y = -max_height; top y = -(Ry + max_height).
            // height = sin(rise)*1.0 (height[i]=1 constant for the wind primitive).
            // top_size = bottom_size + cos(rise)*1.0.
            let (sin_rise, cos_rise) = s.rise_angle_deg.to_radians().sin_cos();
            let bottom = s.distance;
            let top = s.distance + cos_rise * 1.0;
            let vert = sin_rise * 1.0;
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
                color: [1.0, 1.0, 1.0, alpha],
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
        EffectUpdateCtx { delta: dt, camera_target: None }
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
    fn portal4_windwalk_sfx_drained_once() {
        let mut e = PortalWindEffect::new([0.0, 0.0, 0.0], PORTAL4);
        assert_eq!(e.take_sfx_request(), Some("effect\\windwalk.wav"));
        assert_eq!(e.take_sfx_request(), None, "SFX consumed on first take");
    }
}
