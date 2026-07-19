use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const DISC_TEXTURE: &str = "ring_blue.tga";
pub const TEXTURES: &[&str] = &[DISC_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const PARENT_DURATION_FRAMES: f32 = 100.0;
const DISC_SPAWN_PERIOD_FRAMES: u32 = 14;
const DISC_DURATION_FRAMES: f32 = 30.0;
const DISC_THICKNESS: f32 = 8.0;
const DISC_RADIUS_SPEED_PER_FRAME: f32 = 1.0;
const DISC_RADIUS_ACCEL_PER_FRAME2: f32 =
    -(DISC_RADIUS_SPEED_PER_FRAME / DISC_DURATION_FRAMES) / 2.0;
const DISC_FADE_OUT_AT_FRAME: f32 = DISC_DURATION_FRAMES / 2.0;
const DISC_MAX_ALPHA: f32 = 200.0 / 255.0;
const DISC_UV_REPEAT: f32 = 4.0;
/// Native-RO `-Y = up`: lift the ring 1 wu above the ground plane to avoid z-fighting.
const GROUND_OFFSET_Y: f32 = -1.0;

pub const TOTAL_DURATION_MS: u32 =
    ((PARENT_DURATION_FRAMES + DISC_DURATION_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

#[derive(Clone, Copy, Debug)]
struct Disc {
    age_frames: f32,
}

impl Disc {
    fn alive(&self) -> bool {
        self.age_frames < DISC_DURATION_FRAMES
    }

    fn outer_radius(&self) -> f32 {
        let n = self.age_frames.clamp(0.0, DISC_DURATION_FRAMES);
        n * DISC_RADIUS_SPEED_PER_FRAME + DISC_RADIUS_ACCEL_PER_FRAME2 * n * (n + 1.0) / 2.0
    }

    fn alpha(&self) -> f32 {
        if self.age_frames <= DISC_FADE_OUT_AT_FRAME {
            DISC_MAX_ALPHA
        } else {
            let span = (DISC_DURATION_FRAMES - DISC_FADE_OUT_AT_FRAME).max(1e-3);
            let fade = ((self.age_frames - DISC_FADE_OUT_AT_FRAME) / span).clamp(0.0, 1.0);
            DISC_MAX_ALPHA * (1.0 - fade)
        }
    }
}

pub struct ReadyPortalDiscEmitter {
    world_pos: [f32; 3],
    discs: Vec<Disc>,
    age_frames: f32,
    last_spawn_frame: i32,
}

impl ReadyPortalDiscEmitter {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            discs: Vec::new(),
            age_frames: 0.0,
            last_spawn_frame: -1,
        }
    }

    pub fn step(&mut self, dt_frames: f32, spawn_window_frames: f32) {
        self.age_frames += dt_frames;

        let current_frame = self.age_frames.floor() as i32;
        let spawning = self.age_frames <= spawn_window_frames;
        if spawning {
            let next_frame = self.last_spawn_frame + 1;
            for f in next_frame..=current_frame {
                if f >= 0
                    && (f as f32) <= spawn_window_frames
                    && (f as u32) % DISC_SPAWN_PERIOD_FRAMES == 0
                {
                    let initial_age = (self.age_frames - f as f32).max(0.0);
                    self.discs.push(Disc {
                        age_frames: initial_age,
                    });
                }
            }
            self.last_spawn_frame = current_frame;
        }

        for d in &mut self.discs {
            d.age_frames += dt_frames;
        }
        self.discs.retain(|d| d.alive());
    }

    pub fn is_empty(&self) -> bool {
        self.discs.is_empty()
    }

    pub fn collect_draws(&self, out: &mut EffectDrawList) {
        let center = [
            self.world_pos[0],
            self.world_pos[1] + GROUND_OFFSET_Y,
            self.world_pos[2],
        ];
        for d in &self.discs {
            let outer = d.outer_radius();
            if outer <= 0.0 {
                continue;
            }
            let alpha = d.alpha();
            if alpha <= 0.0 {
                continue;
            }
            let thickness = outer.min(DISC_THICKNESS);
            out.push(EffectPrimitiveDraw::GroundDisc {
                center,
                radius: outer,
                thickness,
                rotation: 0.0,
                arc_angle_deg: 360.0,
                uv_repeat: DISC_UV_REPEAT,
                texture: DISC_TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
                no_depth: false,
                tilt_rad: 0.0,
                spin_rad: 0.0,
            });
        }
    }
}

pub struct ReadyPortalEffect {
    emitter: ReadyPortalDiscEmitter,
}

impl ReadyPortalEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            emitter: ReadyPortalDiscEmitter::new(world_pos),
        }
    }
}

impl Effect for ReadyPortalEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.emitter.step(dt_frames, PARENT_DURATION_FRAMES);
        if self.emitter.age_frames >= PARENT_DURATION_FRAMES + DISC_DURATION_FRAMES
            && self.emitter.is_empty()
        {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        self.emitter.collect_draws(out);
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

    fn draws(e: &ReadyPortalEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(e: &mut ReadyPortalEffect, dt: f32) -> EffectStatus {
        e.update(&ctx(dt))
    }

    #[test]
    fn emits_a_ground_disc_with_blue_ring_texture() {
        let mut e = ReadyPortalEffect::new([3.0, -2.0, 5.0]);
        for _ in 0..3 {
            step(&mut e, 1.0 / FRAMES_PER_SECOND);
        }
        let prims = draws(&e);
        assert_eq!(prims.len(), 1);
        match &prims[0] {
            EffectPrimitiveDraw::GroundDisc {
                center,
                arc_angle_deg,
                uv_repeat,
                texture,
                blend,
                ..
            } => {
                assert_eq!(*texture, DISC_TEXTURE);
                assert!((arc_angle_deg - 360.0).abs() < f32::EPSILON);
                assert!((uv_repeat - DISC_UV_REPEAT).abs() < f32::EPSILON);
                assert_eq!(*blend, BlendKind::Additive);
                // 1 wu above ground in native-RO -Y space.
                assert!((center[1] - (-2.0 + GROUND_OFFSET_Y)).abs() < 1e-4);
            }
            other => panic!("expected GroundDisc, got {other:?}"),
        }
    }

    #[test]
    fn disc_grows_then_fades_over_its_life() {
        let mut e = ReadyPortalEffect::new([0.0, 0.0, 0.0]);
        for _ in 0..2 {
            step(&mut e, 1.0 / FRAMES_PER_SECOND);
        }
        let r0 = match &draws(&e)[0] {
            EffectPrimitiveDraw::GroundDisc { radius, .. } => *radius,
            _ => unreachable!(),
        };

        step(&mut e, 10.0 / FRAMES_PER_SECOND);
        let (r_mid, a_mid) = match &draws(&e)[0] {
            EffectPrimitiveDraw::GroundDisc { radius, color, .. } => (*radius, color[3]),
            _ => unreachable!(),
        };
        assert!(r_mid > r0, "outer radius grows: {r0} -> {r_mid}");
        assert!((a_mid - DISC_MAX_ALPHA).abs() < 1e-4);

        step(&mut e, 12.0 / FRAMES_PER_SECOND);
        let a_late = match &draws(&e)[0] {
            EffectPrimitiveDraw::GroundDisc { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_late < DISC_MAX_ALPHA);
    }

    fn alive_disc_count(e: &ReadyPortalEffect) -> usize {
        e.emitter.discs.len()
    }

    #[test]
    fn discs_respawn_every_fourteen_frames_until_parent_dies() {
        let mut e = ReadyPortalEffect::new([0.0, 0.0, 0.0]);
        step(&mut e, 0.0);
        assert_eq!(alive_disc_count(&e), 1, "1st disc at frame 0");

        for _ in 0..(DISC_SPAWN_PERIOD_FRAMES as i32) {
            step(&mut e, 1.0 / FRAMES_PER_SECOND);
        }
        assert_eq!(alive_disc_count(&e), 2);

        let mut max_seen = 2;
        for _ in 0..130 {
            step(&mut e, 1.0 / FRAMES_PER_SECOND);
            max_seen = max_seen.max(alive_disc_count(&e));
        }
        assert!(max_seen >= 3, "parent emits multiple discs before dying");
    }

    #[test]
    fn effect_dies_after_parent_plus_last_disc() {
        let mut e = ReadyPortalEffect::new([0.0; 3]);
        let total = PARENT_DURATION_FRAMES + DISC_DURATION_FRAMES + 5.0;
        let mut status = EffectStatus::Running;
        for _ in 0..(total as i32) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
