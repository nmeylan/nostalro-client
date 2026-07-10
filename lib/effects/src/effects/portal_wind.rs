use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect_trait::{BodyTint, Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &["cloud11.tga"];

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TOTAL_DURATION_MS: u32 = 2000;
const TOTAL_FRAMES: f32 = (TOTAL_DURATION_MS as f32) * FRAMES_PER_SECOND / 1000.0;

const WIND_SIDES: u32 = 20;
const WIND_TEXTURE: &str = "cloud11.tga";

const ROT_START_DEG: [f32; 4] = [0.0, 90.0, 180.0, 270.0];
const RISE_ANGLE_DEG: [f32; 4] = [82.0, 88.0, 95.0, 100.0];
const DISTANCE_JITTER_FRACTION: [f32; 4] = [0.0, 0.33, 0.66, 1.0];

#[derive(Clone, Copy)]
pub struct PortalWindConfig {
    pub f1: u8,
    /// 1 = windwalk (quick fade, 120° arc), 2 = gust (slow fade, 180° arc),
    /// 3 = persistent gust (120° arc, ramps in and holds — no fade, no outward drift).
    pub alpha_t: u8,
    pub duration_frames: f32,
    pub max_height_base: f32,
    pub max_height_step: f32,
    pub distance_base: f32,
    pub distance_step: f32,
    pub distance_jitter: f32,
    /// `(0, -1)` disables the tint.
    pub body_light_frames: (i32, i32),
    pub body_light_rgb: [u8; 3],
    pub play_windwalk_wav: bool,
    pub height_scale: f32,
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

pub const MGDEF1: PortalWindConfig = mgdef(1.0, [255, 255, 255], [220, 250, 220]);
pub const MGDEF2: PortalWindConfig = mgdef(2.0, [89, 197, 10], [89, 197, 10]);
pub const MGDEF3: PortalWindConfig = mgdef(5.0, [255, 255, 17], [89, 197, 10]);
pub const MGDEF4: PortalWindConfig = mgdef(8.0, [255, 255, 17], [255, 255, 17]);

const STORMKICK_GUST_SCALE: f32 = 0.15;

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
            self.alpha_b = (self.alpha_b - 3.0).max(0.0);
        }
    }
}

pub struct PortalWindEffect {
    world_pos: [f32; 3],
    age_frames: f32,
    cfg: PortalWindConfig,
    slots: [WindSlot; 4],
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
            Some("effect\\윈드워크.wav")
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
                base_alpha: 1.0,
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
        EffectUpdateCtx {
            delta: dt,
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
        for (i, &(arc, rot, bot)) in f.iter().enumerate() {
            assert!((arc - 33.0).abs() < 0.01, "arc {arc} after 1 step");
            let expected_rot = ((ROT_START_DEG[i] + 5.0).to_radians()) as f32;
            assert!((rot - expected_rot).abs() < 1e-4, "slot {i} rotation");
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

        step_frames(&mut e, 13);
        let after = wind_frustums(&draws(&e));
        for (i, &(_arc, _rot, bot)) in after.iter().enumerate() {
            let init_lo =
                PORTAL4.distance_base + DISTANCE_JITTER_FRACTION[i] * PORTAL4.distance_jitter;
            assert!(
                bot > init_lo + 0.3,
                "slot {i} distance grew past {init_lo}+0.3 (got {bot})"
            );
        }
    }

    #[test]
    fn portal5_yellow_body_light_window_and_no_sfx() {
        let mut e = PortalWindEffect::new([0.0, 0.0, 0.0], PORTAL5);
        assert!(e.take_sfx_request().is_none());
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
        step_frames(&mut e, 12);
        let alpha_12 = draws(&e)
            .iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::Frustum { color, .. } => Some(color[3]),
                _ => None,
            })
            .unwrap_or(0.0);
        assert!(alpha_12 > 0.0, "alpha ramped in by frame 12");

        step_frames(&mut e, 60);
        let arcs = wind_frustums(&draws(&e));
        for (arc, _, _) in arcs {
            assert!(
                (arc - 180.0).abs() < 0.01,
                "gust arc opens to 180°, got {arc}"
            );
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
        assert_eq!(e.take_sfx_request(), Some("effect\\윈드워크.wav"));
        assert_eq!(e.take_sfx_request(), None, "SFX consumed on first take");
    }
}
