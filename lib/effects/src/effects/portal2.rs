use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &[
    "magic_violet.tga",
    "ring_blue.tga",
    "ring_red.tga",
    "ring_purple.tga",
];

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TOTAL_DURATION_MS: u32 = 99990;
const TOTAL_FRAMES: f32 = (TOTAL_DURATION_MS as f32) * FRAMES_PER_SECOND / 1000.0;

const STATE1_START_FRAME: f32 = 1.0;
const HEAL_SIDES: u32 = 48;
const PORTAL_SIDES: u32 = 48;
const HEAL_INITIAL_ROT_DEG: [f32; 3] = [0.0, 137.0, 251.0];

#[derive(Clone, Copy)]
pub struct Portal2Config {
    pub heal_texture: &'static str,
    pub portal_texture: &'static str,
    pub color_rgb: [f32; 3],
    pub call_partner: bool,
}

pub const PORTAL2: Portal2Config = Portal2Config {
    heal_texture: "ring_blue.tga",
    portal_texture: "ring_blue.tga",
    color_rgb: [0.55, 0.7, 1.0],
    call_partner: false,
};

pub const PORTAL3: Portal2Config = Portal2Config {
    heal_texture: "ring_purple.tga",
    portal_texture: "ring_purple.tga",
    color_rgb: [1.0, 0.55, 0.85],
    call_partner: true,
};

const HEAL_ROT_SPEED_DEG: [f32; 3] = [8.0, 9.0, 10.0];
const HEAL_DISTANCE: [f32; 3] = [4.0, 3.0, 2.0];
const HEAL_MAX_HEIGHT: f32 = 50.0;
const HEAL_ALPHA_FADE_TRIGGER: f32 = 1400.0;
const HEAL_ALPHA_RAMP_FRAMES: f32 = 16.0;
const HEAL_ALPHA_RAMP_PER_FRAME: f32 = 5.0;
const HEAL_ALPHA_RAMP_CAP: f32 = 180.0;
const HEAL_ALPHA_DECAY_PER_FRAME: f32 = 2.0;
const HEAL_RAMP_FRAMES: f32 = 90.0;
const PORTAL_ROT_START_DEG: [f32; 3] = [0.0, 25.0, 50.0];
const PORTAL_RISE_ANGLE_DEG: [f32; 3] = [2.0, 3.0, 4.0];
const PORTAL_INITIAL_PROCESS: [f32; 3] = [0.0, -10.0, -20.0];
const PORTAL_INITIAL_PROCESS_CALLPARTNER: [f32; 3] = [0.0, -20.0, -40.0];
const PORTAL_MAX_HEIGHT: f32 = 6.001;
const PORTAL_ALPHA_CAP: f32 = 240.0;
const PORTAL_VY_OFFSET: f32 = -2.0;

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

    fn current_height(&self) -> f32 {
        let mut h = HEAL_MAX_HEIGHT;
        if self.process <= HEAL_RAMP_FRAMES {
            h *= (self.process.to_radians()).sin();
        }
        h
    }
}

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
        for s in &self.heal {
            let alpha = s.alpha_b / 255.0;
            if alpha <= 0.0 {
                continue;
            }
            let h = s.current_height();
            out.push(EffectPrimitiveDraw::Frustum {
                base_alpha: 1.0,
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

        for s in &self.portal {
            push_portal_slot_draw(
                out,
                self.world_pos,
                s,
                self.cfg.portal_texture,
                self.cfg.color_rgb,
            );
        }
    }
}

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
    let uv_repeat = (bottom.max(top) * 0.35).round().max(2.0);
    out.push(EffectPrimitiveDraw::Frustum {
        base_alpha: 1.0,
        base,
        bottom_size: bottom,
        top_size: top,
        height: vert,
        sides: PORTAL_SIDES,
        arc_angle_deg: 360.0,
        rotation: s.rot_start_deg.to_radians(),
        uv_repeat,
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

pub const READYPORTAL2_DURATION_MS: u32 = 2000;
const READYPORTAL2_TOTAL_FRAMES: f32 =
    (READYPORTAL2_DURATION_MS as f32) * FRAMES_PER_SECOND / 1000.0;
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
                    Some((*bottom_size, *top_size))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn portal2_heal_slots_ramp_then_hold() {
        let mut e = Portal2Effect::new([0.0, 0.0, 0.0], PORTAL2);
        step_frames(&mut e, 10);
        let h10 = heal_draws(&draws(&e), PORTAL2.heal_texture);
        assert_eq!(h10.len(), 3, "expected 3 PP_HEAL slots at frame 10");
        for &(bot, _top, h) in &h10 {
            assert!(h > 0.0 && h < HEAL_MAX_HEIGHT, "height {h} mid-ramp");
            assert!(matches!(bot as i32, 4 | 3 | 2));
        }
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
        step_frames(&mut e, 5);
        let p = portal_draws(&draws(&e), PORTAL2.portal_texture);
        assert_eq!(p.len(), 1, "only ec0 should be live at frame 5: {:?}", p);

        step_frames(&mut e, 10);
        let p = portal_draws(&draws(&e), PORTAL2.portal_texture);
        assert_eq!(p.len(), 2, "ec0 + ec1 should be live at frame 15");

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

        step_frames(&mut e, 20);
        let later = portal_draws(&draws(&e), PORTAL3.portal_texture);
        for &(bot, _) in &later {
            assert!(bot <= 14.0, "CALLPARTNER never exceeds 14 (got {bot})");
        }

        let heal_p = heal_draws(&prims, PORTAL3.heal_texture);
        assert_eq!(heal_p.len(), 3);
    }

    #[test]
    fn readyportal2_three_staggered_blue_rings_then_dies() {
        let mut e = ReadyPortal2Effect::new([0.0, 0.0, 0.0]);
        let one = |e: &ReadyPortal2Effect| {
            let mut l = EffectDrawList::new();
            e.collect_draws(&mut l, &render_ctx());
            portal_draws(&l.primitives, "ring_blue.tga").len()
        };
        step_frames(&mut e, 1);
        assert_eq!(one(&e), 1, "only ec0 live early (ec1/ec2 start at -10/-20)");

        step_frames(&mut e, 30);
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        let rings = portal_draws(&l.primitives, "ring_blue.tga");
        assert_eq!(rings.len(), 3, "all three PP_PORTAL rings live by frame 30");
        for &(bot, top) in &rings {
            assert!(top > bot, "ring extends outward (PP_PORTAL, not a pillar)");
        }

        let mut status = EffectStatus::Running;
        for _ in 0..130 {
            status = e.update(&ctx(1.0 / 60.0));
        }
        assert_eq!(
            status,
            EffectStatus::Dead,
            "dies at the 2 s parent duration"
        );
    }
}
