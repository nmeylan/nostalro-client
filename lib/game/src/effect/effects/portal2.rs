//! `EF_PORTAL2` / `EF_PORTAL3` — two-phase GI portal.
//!
//! The portal runs in two phases.
//! Phase 0 launches the heal phase with 3 vertical
//! ring slots at concentric radii. Phase 1 (when the parent
//! advances to state 1) overlays the portal phase with 3 nearly-flat ground rings that
//! sweep outward (default) or inward (CALLPARTNER, `F1==3`).
//!
//! The heal phase keeps `height = max_height` per slot
//! with a sin-ramp over 90 frames. The portal phase grows `distance` at 0.5/frame
//! (default) or shrinks at 0.25/frame (CALLPARTNER); alpha fades after 16/20
//! frames. Both render as casting rings: 20 segments per slot, one quad
//! bridging adjacent angular steps.
//!
//! Variants:
//!   * Portal2 (`F1=0`) — violet heal ring + blue portal ring, size 4
//!   * Portal3 (`F1=3`) — red heal ring + red portal ring, size 10, CALLPARTNER

use crate::effect::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode,
};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

/// GRF textures referenced across both Portal2 and Portal3.
pub const TEXTURES: &[&str] = &[
    "magic_violet.tga",
    "ring_blue.tga",
    "ring_red.tga",
    "ring_purple.tga",
];

const FRAMES_PER_SECOND: f32 = 60.0;
/// Matches the table.rs default — the parent keeps the table-derived
/// duration (Portal2/3 = 99990 ms).
pub const TOTAL_DURATION_MS: u32 = 99990;
const TOTAL_FRAMES: f32 = (TOTAL_DURATION_MS as f32) * FRAMES_PER_SECOND / 1000.0;

/// State 1 is the first frame after the parent advances state.
/// The parent advances its state counter once per frame, so state 1
/// begins at frame 1 (state 0 covers frame 0).
const STATE1_START_FRAME: f32 = 1.0;

/// The heal casting ring walks in 20 steps.
const HEAL_SIDES: u32 = 20;
/// The portal ring uses the same casting-ring path.
const PORTAL_SIDES: u32 = 20;

/// Random start angle in the original — we substitute deterministic per-slot offsets so
/// tests are reproducible. Visually, the three rings rotate at different
/// rates (8/9/10°/frame), so the starting phase drifts apart within a
/// second regardless of seed.
const HEAL_INITIAL_ROT_DEG: [f32; 3] = [0.0, 137.0, 251.0];

#[derive(Clone, Copy)]
pub struct Portal2Config {
    /// Heal-phase ring texture.
    pub heal_texture: &'static str,
    /// Portal-phase ring texture.
    pub portal_texture: &'static str,
    /// RGB tint multiplier applied to both phases (alpha is from the
    /// per-frame envelope). Used to nudge the rendered hue closer to the
    /// reference gif when the source texture reads too saturated.
    pub color_rgb: [f32; 3],
    /// F1==3 enables the CALLPARTNER branch — distance shrinks,
    /// slower alpha ramp.
    pub call_partner: bool,
}

// The original uses `magic_violet.tga` (heal) + `ring_blue.tga` (portal) for F1=0.
// Rendered through our additive pipeline that reads too violet against
// in-engine map ambient; we lean the heal phase onto `ring_blue.tga` and
// add a slight blue-favouring tint so the rising columns read as blue
// rather than purple, matching `imgs/300-350/315.gif`.
pub const PORTAL2: Portal2Config = Portal2Config {
    heal_texture: "ring_blue.tga",
    portal_texture: "ring_blue.tga",
    color_rgb: [0.55, 0.7, 1.0],
    call_partner: false,
};

// The original uses `ring_red.tga` for both phases when F1=3. The reference gif
// (`imgs/300-350/339.gif`) reads more magenta/pink than pure red; we swap
// to `ring_purple.tga` and bias the tint toward pink so the contracting
// CALLPARTNER ring is recognisably pink rather than blood-red.
pub const PORTAL3: Portal2Config = Portal2Config {
    heal_texture: "ring_purple.tga",
    portal_texture: "ring_purple.tga",
    color_rgb: [1.0, 0.55, 0.85],
    call_partner: true,
};

/// Heal-phase per-slot rotation speeds: `+ (ec + 8)°` per frame.
const HEAL_ROT_SPEED_DEG: [f32; 3] = [8.0, 9.0, 10.0];
/// Ring radius for the three heal slots.
const HEAL_DISTANCE: [f32; 3] = [4.0, 3.0, 2.0];
/// All heal slots share these. `rise_angle=90°` — but with rise=90°
/// cos=0/sin=1, so the cones reduce to vertical pillars: `top_size==distance`
/// and `height==max_height`. We bake this into `collect_draws` rather than
/// keeping a dead constant for parity.
const HEAL_MAX_HEIGHT: f32 = 50.0;
const HEAL_ALPHA_FADE_TRIGGER: f32 = 1400.0;
const HEAL_ALPHA_RAMP_FRAMES: f32 = 16.0;
const HEAL_ALPHA_RAMP_PER_FRAME: f32 = 5.0;
const HEAL_ALPHA_RAMP_CAP: f32 = 180.0;
const HEAL_ALPHA_DECAY_PER_FRAME: f32 = 2.0;
/// Below `process==91` the height sin-ramps in over 90 frames.
const HEAL_RAMP_FRAMES: f32 = 90.0;

/// Portal-phase per-slot rotation start (degrees) — 0/25/50.
const PORTAL_ROT_START_DEG: [f32; 3] = [0.0, 25.0, 50.0];
/// Portal-phase per-slot rise angle (degrees) — 2/3/4.
const PORTAL_RISE_ANGLE_DEG: [f32; 3] = [2.0, 3.0, 4.0];
/// Portal-phase initial process (frames) for non-CALLPARTNER slots — staggered
/// negative start so ec1/ec2 spawn 10/20 frames after ec0.
const PORTAL_INITIAL_PROCESS: [f32; 3] = [0.0, -10.0, -20.0];
/// Portal-phase initial process for CALLPARTNER (F1==3) — twice the stagger.
const PORTAL_INITIAL_PROCESS_CALLPARTNER: [f32; 3] = [0.0, -20.0, -40.0];
const PORTAL_MAX_HEIGHT: f32 = 6.001;
const PORTAL_ALPHA_CAP: f32 = 240.0;
/// Y-offset on the bottom ring — `-2.0` for `max_height==6.001`.
const PORTAL_VY_OFFSET: f32 = -2.0;

/// Per-slot state for the heal phase.
#[derive(Clone, Copy)]
struct HealSlot {
    rot_start_deg: f32,
    process: f32,
    alpha_b: f32,
    distance: f32,
    rot_speed_deg: f32,
}

impl HealSlot {
    fn new(slot_idx: usize) -> Self {
        Self {
            rot_start_deg: HEAL_INITIAL_ROT_DEG[slot_idx],
            process: 0.0,
            alpha_b: 0.0,
            distance: HEAL_DISTANCE[slot_idx],
            rot_speed_deg: HEAL_ROT_SPEED_DEG[slot_idx],
        }
    }

    fn step(&mut self) {
        self.process += 1.0;
        self.rot_start_deg = (self.rot_start_deg + self.rot_speed_deg).rem_euclid(360.0);
        if self.process < HEAL_ALPHA_RAMP_FRAMES {
            self.alpha_b = (self.alpha_b + HEAL_ALPHA_RAMP_PER_FRAME).min(HEAL_ALPHA_RAMP_CAP);
        }
        if self.process >= HEAL_ALPHA_FADE_TRIGGER {
            self.alpha_b = (self.alpha_b - HEAL_ALPHA_DECAY_PER_FRAME).max(0.0);
        }
    }

    /// Current height: `max_height`,
    /// optionally multiplied by `sin(process°)` while `process<=90`.
    fn current_height(&self) -> f32 {
        let mut h = HEAL_MAX_HEIGHT;
        if self.process <= HEAL_RAMP_FRAMES {
            h *= (self.process.to_radians()).sin();
        }
        h
    }
}

/// Per-slot state for the portal phase.
#[derive(Clone, Copy)]
struct PortalSlot {
    rot_start_deg: f32,
    process: f32,
    alpha_b: f32,
    distance: f32,
    rise_angle_deg: f32,
    live: bool,
}

impl PortalSlot {
    fn new(slot_idx: usize, call_partner: bool) -> Self {
        let initial_process = if call_partner {
            PORTAL_INITIAL_PROCESS_CALLPARTNER[slot_idx]
        } else {
            PORTAL_INITIAL_PROCESS[slot_idx]
        };
        Self {
            rot_start_deg: PORTAL_ROT_START_DEG[slot_idx],
            process: initial_process,
            alpha_b: 0.0,
            distance: 0.0,
            rise_angle_deg: PORTAL_RISE_ANGLE_DEG[slot_idx],
            live: true,
        }
    }

    /// Portal-slot per-frame update. `ctrl_process` is the parent control
    /// slot's process counter, used by the
    /// `process>1400` and `process>120` terminal checks.
    fn step(&mut self, call_partner: bool, ctrl_process: f32) {
        if !self.live {
            return;
        }
        self.process += 1.0;
        if self.process <= 0.0 {
            return;
        }
        if call_partner {
            self.distance -= 0.25;
            if self.distance < 7.0 {
                if self.distance < 0.0 {
                    self.distance = 0.0;
                }
                self.alpha_b -= 7.0;
                if self.alpha_b < 0.0 {
                    self.alpha_b = 0.0;
                    if ctrl_process > 1400.0 {
                        self.live = false;
                    } else {
                        self.process = 0.0;
                        self.distance = 14.0;
                    }
                }
            }
            if self.process < 20.0 {
                self.alpha_b = (self.alpha_b + 12.0).min(PORTAL_ALPHA_CAP);
            }
        } else {
            self.distance += 0.5;
            if self.distance > 7.0 {
                self.alpha_b -= 15.0;
                if self.alpha_b < 0.0 {
                    self.alpha_b = 0.0;
                    // For Portal2 the control slot never triggers the
                    // early terminal checks, so only `process>1400` applies
                    // late game. Reset to start a new pulse.
                    if ctrl_process > 1400.0 {
                        self.live = false;
                    } else {
                        self.process = 0.0;
                        self.distance = 0.0;
                    }
                }
            }
            if self.process < 10.0 {
                self.alpha_b = (self.alpha_b + 24.0).min(PORTAL_ALPHA_CAP);
            }
        }
    }

    /// `height[i] = max_height`, sin-ramped over 10 frames at 9°/step.
    fn current_height(&self) -> f32 {
        let mut h = PORTAL_MAX_HEIGHT;
        if self.process <= 10.0 && self.process > 0.0 {
            h *= ((self.process * 9.0).to_radians()).sin();
        }
        h
    }
}

pub struct Portal2Effect {
    world_pos: [f32; 3],
    age_frames: f32,
    cfg: Portal2Config,
    heal: [HealSlot; 3],
    portal: [PortalSlot; 3],
    /// Control-slot counter — incremented once per frame on the portal
    /// phase (the counter the portal slots use for terminal
    /// checks). Drives `live=false` once it exceeds 1400.
    portal_ctrl_process: f32,
    portal_phase_started: bool,
}

impl Portal2Effect {
    pub fn new(world_pos: [f32; 3], cfg: Portal2Config) -> Self {
        Self {
            world_pos,
            age_frames: 0.0,
            cfg,
            heal: [HealSlot::new(0), HealSlot::new(1), HealSlot::new(2)],
            portal: [
                PortalSlot::new(0, cfg.call_partner),
                PortalSlot::new(1, cfg.call_partner),
                PortalSlot::new(2, cfg.call_partner),
            ],
            portal_ctrl_process: 0.0,
            portal_phase_started: false,
        }
    }

    fn step_one_frame(&mut self) {
        for s in &mut self.heal {
            s.step();
        }
        if self.age_frames >= STATE1_START_FRAME {
            if !self.portal_phase_started {
                self.portal_phase_started = true;
            }
            self.portal_ctrl_process += 1.0;
            let ctrl = self.portal_ctrl_process;
            for s in &mut self.portal {
                s.step(self.cfg.call_partner, ctrl);
            }
        }
    }
}

impl Effect for Portal2Effect {
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
        // Heal slots — rise_angle = 90° so cos(rise)=0: top stays at
        // bottom_size radius (vertical pillar) and height = max_height_now.
        for s in &self.heal {
            let alpha = s.alpha_b / 255.0;
            if alpha <= 0.0 {
                continue;
            }
            let h = s.current_height();
            // rise=90°: sin=1, cos=0 → top_size==bottom_size, height==h.
            out.push(EffectPrimitiveDraw::Frustum {
                base: self.world_pos,
                bottom_size: s.distance,
                top_size: s.distance,
                height: h,
                sides: HEAL_SIDES,
                arc_angle_deg: 360.0,
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
                texture: self.cfg.heal_texture,
                color: [
                    self.cfg.color_rgb[0],
                    self.cfg.color_rgb[1],
                    self.cfg.color_rgb[2],
                    alpha,
                ],
                blend: BlendKind::Additive,
            });
        }

        // Portal slots — rise_angle ≈ 2/3/4°: cos(rise)≈1 so the ring
        // extends nearly horizontally outward (top_size = distance +
        // cos*height); height = sin*h is small (a few units of vertical
        // lean). Y offset -2 for max_height==6.001.
        for s in &self.portal {
            push_portal_slot_draw(out, self.world_pos, s, self.cfg.portal_texture, self.cfg.color_rgb);
        }
    }
}

/// One portal-phase ground ring, shared by `Portal2`/`Portal3` and the standalone
/// `ReadyPortal2`. Skips dead/transparent slots.
fn push_portal_slot_draw(
    out: &mut EffectDrawList,
    world_pos: [f32; 3],
    s: &PortalSlot,
    texture: &'static str,
    color_rgb: [f32; 3],
) {
    if !s.live {
        return;
    }
    let alpha = s.alpha_b / 255.0;
    if alpha <= 0.0 {
        return;
    }
    let h_now = s.current_height();
    let (sin_rise, cos_rise) = s.rise_angle_deg.to_radians().sin_cos();
    let bottom = s.distance;
    let top = s.distance + cos_rise * h_now;
    let vert = sin_rise * h_now;
    let base = [world_pos[0], world_pos[1] + PORTAL_VY_OFFSET, world_pos[2]];
    out.push(EffectPrimitiveDraw::Frustum {
        base,
        bottom_size: bottom,
        top_size: top,
        height: vert,
        sides: PORTAL_SIDES,
        arc_angle_deg: 360.0,
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
        texture,
        color: [color_rgb[0], color_rgb[1], color_rgb[2], alpha],
        blend: BlendKind::Additive,
    });
}

/// `EF_READYPORTAL2` (id 316) — the portal-ready pad: three concentric
/// portal-phase ground rings (`ring_blue.tga`, additive), staggered to spawn
/// 10 frames apart, each growing outward and pulsing. It
/// launches the same portal-phase slots as
/// `Portal2` (non-CALLPARTNER) but from frame 0 with no heal columns.
pub const READYPORTAL2_DURATION_MS: u32 = 2000;
const READYPORTAL2_TOTAL_FRAMES: f32 =
    (READYPORTAL2_DURATION_MS as f32) * FRAMES_PER_SECOND / 1000.0;
/// `ring_blue.tga` reads slightly violet through the additive pipeline; bias
/// toward blue to match the in-game pad (same correction as `PORTAL2`).
const READYPORTAL2_TINT: [f32; 3] = [0.55, 0.7, 1.0];

pub struct ReadyPortal2Effect {
    world_pos: [f32; 3],
    age_frames: f32,
    portal: [PortalSlot; 3],
    ctrl_process: f32,
}

impl ReadyPortal2Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age_frames: 0.0,
            portal: [
                PortalSlot::new(0, false),
                PortalSlot::new(1, false),
                PortalSlot::new(2, false),
            ],
            ctrl_process: 0.0,
        }
    }
}

impl Effect for ReadyPortal2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let before = self.age_frames;
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let steps = (self.age_frames.floor() - before.floor()).max(0.0) as i32;
        for _ in 0..steps {
            self.ctrl_process += 1.0;
            let ctrl = self.ctrl_process;
            for s in &mut self.portal {
                s.step(false, ctrl);
            }
        }
        if self.age_frames >= READYPORTAL2_TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for s in &self.portal {
            push_portal_slot_draw(out, self.world_pos, s, "ring_blue.tga", READYPORTAL2_TINT);
        }
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

    fn step_frames<E: Effect>(e: &mut E, n: u32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    fn draws(e: &Portal2Effect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn heal_draws(prims: &[EffectPrimitiveDraw], texture: &str) -> Vec<(f32, f32, f32)> {
        prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum {
                    bottom_size,
                    top_size,
                    height,
                    texture: t,
                    ..
                } if *t == texture && (*bottom_size - *top_size).abs() < 1e-3 => {
                    Some((*bottom_size, *top_size, *height))
                }
                _ => None,
            })
            .collect()
    }

    fn portal_draws(prims: &[EffectPrimitiveDraw], texture: &str) -> Vec<(f32, f32)> {
        prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum {
                    bottom_size,
                    top_size,
                    texture: t,
                    ..
                } if *t == texture && (*top_size - *bottom_size) > 1e-3 => {
                    // top != bottom → portal slot (heal has top==bottom)
                    Some((*bottom_size, *top_size))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn portal2_heal_slots_ramp_then_hold() {
        let mut e = Portal2Effect::new([0.0, 0.0, 0.0], PORTAL2);
        // Frame 10: 3 heal Frustums, height < max_height (still ramping).
        step_frames(&mut e, 10);
        let h10 = heal_draws(&draws(&e), PORTAL2.heal_texture);
        assert_eq!(h10.len(), 3, "expected 3 PP_HEAL slots at frame 10");
        for &(bot, _top, h) in &h10 {
            assert!(h > 0.0 && h < HEAL_MAX_HEIGHT, "height {h} mid-ramp");
            assert!(matches!(bot as i32, 4 | 3 | 2));
        }
        // Frame 91: ramp complete, height == max_height.
        step_frames(&mut e, 81);
        let h91 = heal_draws(&draws(&e), PORTAL2.heal_texture);
        for &(_b, _t, h) in &h91 {
            assert!(
                (h - HEAL_MAX_HEIGHT).abs() < 1e-3,
                "height {h} should equal {HEAL_MAX_HEIGHT} after ramp"
            );
        }
    }

    #[test]
    fn portal2_state1_spawns_with_staggered_starts() {
        let mut e = Portal2Effect::new([0.0, 0.0, 0.0], PORTAL2);
        // Step past state-0-only frame, into state 1.
        step_frames(&mut e, 5);
        // ec0 process = 4 (started at 0, advanced 4 times since frame 1).
        // ec1 process = 4-10 = -6 (not yet emitting).
        // ec2 process = 4-20 = -16 (not yet emitting).
        let p = portal_draws(&draws(&e), PORTAL2.portal_texture);
        assert_eq!(p.len(), 1, "only ec0 should be live at frame 5: {:?}", p);

        // Step to frame 15: ec0 process=14, ec1 process=4, ec2 process=-6.
        step_frames(&mut e, 10);
        let p = portal_draws(&draws(&e), PORTAL2.portal_texture);
        assert_eq!(p.len(), 2, "ec0 + ec1 should be live at frame 15");

        // Step to frame 30: all three live. Distances must be growing.
        step_frames(&mut e, 15);
        let p = portal_draws(&draws(&e), PORTAL2.portal_texture);
        assert_eq!(p.len(), 3, "all three slots live at frame 30");
        for &(bot, top) in &p {
            assert!(bot > 0.0, "default (non-CALLPARTNER) distance grows");
            assert!(top > bot, "ring extends outward via cos(rise)*height");
        }
    }

    #[test]
    fn portal3_callpartner_resets_outward_then_contracts() {
        let mut e = Portal2Effect::new([0.0, 0.0, 0.0], PORTAL3);
        // Portal CALLPARTNER: at
        // process=1 the initial `distance=0` decrements to negative and
        // clamps to 0; that drains alpha to <0 which triggers the reset to
        // `process=0, distance=14`. The visible ring then contracts from
        // radius 14 inward at 0.25/frame, matching `imgs/300-350/339.gif`
        // (large red ring sweeping in).
        step_frames(&mut e, 6);
        let prims = draws(&e);
        let portal_p = portal_draws(&prims, PORTAL3.portal_texture);
        assert!(!portal_p.is_empty(), "ec0 PP_PORTAL slot should be live");
        for &(bot, _) in &portal_p {
            assert!(
                (7.0..=14.0).contains(&bot),
                "post-reset CALLPARTNER distance must be in (7, 14] (got {bot})"
            );
        }

        // Step 20 more frames — distance should have decreased by 5 (or
        // possibly looped again if it crossed the threshold and reset).
        // Either way the ring is contracting, never growing past 14.
        step_frames(&mut e, 20);
        let later = portal_draws(&draws(&e), PORTAL3.portal_texture);
        for &(bot, _) in &later {
            assert!(bot <= 14.0, "CALLPARTNER never exceeds 14 (got {bot})");
        }

        // Sanity: heal texture is ring_red, not magic_violet (F1==3).
        let heal_p = heal_draws(&prims, PORTAL3.heal_texture);
        assert_eq!(heal_p.len(), 3);
    }

    #[test]
    fn readyportal2_three_staggered_blue_rings_then_dies() {
        let mut e = ReadyPortal2Effect::new([0.0, 0.0, 0.0]);
        // Frame 0 emission: only ec0 is live (ec1/ec2 start at -10/-20).
        let one = |e: &ReadyPortal2Effect| {
            let mut l = EffectDrawList::new();
            e.collect_draws(&mut l, &render_ctx());
            portal_draws(&l.primitives, "ring_blue.tga").len()
        };
        step_frames(&mut e, 1);
        assert_eq!(one(&e), 1, "only ec0 live early (ec1/ec2 start at -10/-20)");

        // By frame 30 all three rings are up, blue ring texture, growing out.
        step_frames(&mut e, 30);
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        let rings = portal_draws(&l.primitives, "ring_blue.tga");
        assert_eq!(rings.len(), 3, "all three PP_PORTAL rings live by frame 30");
        for &(bot, top) in &rings {
            assert!(top > bot, "ring extends outward (PP_PORTAL, not a pillar)");
        }

        // Outlives a couple of seconds → dead by the 2 s parent duration.
        let mut status = EffectStatus::Running;
        for _ in 0..130 {
            status = e.update(&ctx(1.0 / 60.0));
        }
        assert_eq!(status, EffectStatus::Dead, "dies at the 2 s parent duration");
    }
}
