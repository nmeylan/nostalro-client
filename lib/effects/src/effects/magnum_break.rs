//! EF_MAGNUMBREAK — yellow ground shockwave + spherical explosion.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{CameraShake, Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_TEXTURE: &str = "ring_yellow.tga";
pub const EXPLOSION_TEXTURE: &str = "대폭발.tga";
pub const TEXTURES: &[&str] = &[RING_TEXTURE, EXPLOSION_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 30.0;
const DURATION_S: f32 = DURATION_FRAMES / FRAMES_PER_SECOND;

pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const RING_INITIAL_RADIUS: f32 = 0.0;
const RING_RADIUS_SPEED_PER_FRAME: f32 = 1.75;
const RING_RADIUS_ACCEL_PER_FRAME2: f32 = -(RING_RADIUS_SPEED_PER_FRAME / DURATION_FRAMES) / 2.0;
const RING_THICKNESS: f32 = 12.0;
const RING_PEAK_ALPHA: f32 = 1.0;
const FADE_IN_FRAMES: f32 = 15.0;
const FADE_OUT_FRAMES: f32 = DURATION_FRAMES - 15.0;
const RING_UV_REPEAT: f32 = 4.0;
const RING_SEGMENTS: u32 = 32;

const EXPLOSION_INITIAL_RADIUS: f32 = 0.0;
const EXPLOSION_RADIUS_SPEED_PER_FRAME: f32 = 1.15;
const EXPLOSION_RADIUS_ACCEL_PER_FRAME2: f32 =
    -(EXPLOSION_RADIUS_SPEED_PER_FRAME / DURATION_FRAMES) / 2.0;
const EXPLOSION_PEAK_ALPHA: f32 = 180.0 / 255.0;
const EXPLOSION_ROT_DEG_PER_FRAME: f32 = 3.0;
const EXPLOSION_SIDES_LAT: u32 = 5;
const EXPLOSION_SIDES_LON: u32 = 10;

const SIZE_SCALE: f32 = 0.65;

fn alpha_curve(frame: f32, peak: f32) -> f32 {
    if frame <= FADE_IN_FRAMES {
        peak * (frame / FADE_IN_FRAMES).clamp(0.0, 1.0)
    } else {
        let fade =
            ((frame - FADE_OUT_FRAMES) / (DURATION_FRAMES - FADE_OUT_FRAMES)).clamp(0.0, 1.0);
        peak * (1.0 - fade)
    }
}

fn radius_at(initial: f32, speed: f32, accel: f32, frame: f32) -> f32 {
    initial + speed * frame + accel * frame * (frame + 1.0) / 2.0
}

pub struct MagnumBreakEffect {
    world_pos: [f32; 3],
    age: f32,
    shake_fired: bool,
}

impl MagnumBreakEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
            shake_fired: false,
        }
    }

    fn frame(&self) -> f32 {
        (self.age * FRAMES_PER_SECOND).clamp(0.0, DURATION_FRAMES)
    }

    fn ring_draw(&self, frame: f32) -> Option<EffectPrimitiveDraw> {
        let outer = radius_at(
            RING_INITIAL_RADIUS,
            RING_RADIUS_SPEED_PER_FRAME,
            RING_RADIUS_ACCEL_PER_FRAME2,
            frame,
        ) * SIZE_SCALE;
        if outer <= 0.0 {
            return None;
        }
        Some(EffectPrimitiveDraw::BillboardRing {
            pos: self.world_pos,
            radius: outer,
            thickness: outer.min(RING_THICKNESS * SIZE_SCALE),
            segments: RING_SEGMENTS,
            uv_repeat: RING_UV_REPEAT,
            texture: RING_TEXTURE,
            color: [1.0, 1.0, 1.0, alpha_curve(frame, RING_PEAK_ALPHA)],
            blend: BlendKind::Alpha,
        })
    }
}

impl Effect for MagnumBreakEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= DURATION_S {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.frame();

        if let Some(ring) = self.ring_draw(frame) {
            out.push(ring);
        }

        let explosion_radius = radius_at(
            EXPLOSION_INITIAL_RADIUS,
            EXPLOSION_RADIUS_SPEED_PER_FRAME,
            EXPLOSION_RADIUS_ACCEL_PER_FRAME2,
            frame,
        ) * SIZE_SCALE;
        if explosion_radius > 0.0 {
            let longitude_offset_rad = (frame * EXPLOSION_ROT_DEG_PER_FRAME).to_radians();
            out.push(EffectPrimitiveDraw::Sphere {
                center: self.world_pos,
                radius: explosion_radius,
                sides_lat: EXPLOSION_SIDES_LAT,
                sides_lon: EXPLOSION_SIDES_LON,
                longitude_offset: longitude_offset_rad,
                longitude_arc: std::f32::consts::TAU,
                uv_repeat: [1.0, 1.0],
                texture: EXPLOSION_TEXTURE,
                color: [1.0, 1.0, 1.0, alpha_curve(frame, EXPLOSION_PEAK_ALPHA)],
                blend: BlendKind::Alpha,
                no_depth: true,
            });
        }
    }

    fn take_camera_shake(&mut self) -> Option<CameraShake> {
        if self.shake_fired {
            return None;
        }
        self.shake_fired = true;
        Some(CameraShake {
            amplitude: 2.0,
            duration_ms: 400,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn draws(effect: &MagnumBreakEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut MagnumBreakEffect, dt: f32) {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    #[test]
    fn emits_one_camera_facing_ring_and_a_sphere() {
        let mut mb = MagnumBreakEffect::new([0.0; 3]);
        step(&mut mb, 1.0 / FRAMES_PER_SECOND);
        let prims = draws(&mb);
        assert_eq!(prims.len(), 2, "ring + sphere");
        assert!(matches!(
            prims[0],
            EffectPrimitiveDraw::BillboardRing { .. }
        ));
        assert!(matches!(prims[1], EffectPrimitiveDraw::Sphere { .. }));
    }

    #[test]
    fn ring_and_sphere_grow_together() {
        let mut mb = MagnumBreakEffect::new([0.0; 3]);
        step(&mut mb, 1.0 / FRAMES_PER_SECOND);
        let (r0, s0) = match (&draws(&mb)[0], &draws(&mb)[1]) {
            (
                EffectPrimitiveDraw::BillboardRing { radius, .. },
                EffectPrimitiveDraw::Sphere { radius: sr, .. },
            ) => (*radius, *sr),
            _ => unreachable!(),
        };
        step(&mut mb, DURATION_S * 0.5);
        let (r_mid, s_mid) = match (&draws(&mb)[0], &draws(&mb)[1]) {
            (
                EffectPrimitiveDraw::BillboardRing { radius, .. },
                EffectPrimitiveDraw::Sphere { radius: sr, .. },
            ) => (*radius, *sr),
            _ => unreachable!(),
        };
        assert!(r_mid > r0);
        assert!(s_mid > s0);
    }

    #[test]
    fn sphere_longitude_offset_advances_with_time() {
        let mut mb = MagnumBreakEffect::new([0.0; 3]);
        step(&mut mb, 1.0 / FRAMES_PER_SECOND);
        let off0 = match &draws(&mb)[1] {
            EffectPrimitiveDraw::Sphere {
                longitude_offset, ..
            } => *longitude_offset,
            _ => unreachable!(),
        };
        step(&mut mb, 10.0 / FRAMES_PER_SECOND);
        let off1 = match &draws(&mb)[1] {
            EffectPrimitiveDraw::Sphere {
                longitude_offset, ..
            } => *longitude_offset,
            _ => unreachable!(),
        };
        assert!(off1 > off0, "longitude_offset advances each frame");
    }

    #[test]
    fn alpha_fades_in_then_out() {
        let mut mb = MagnumBreakEffect::new([0.0; 3]);
        step(&mut mb, 1.0 / FRAMES_PER_SECOND);
        let a0 = match &draws(&mb)[0] {
            EffectPrimitiveDraw::BillboardRing { color, .. } => color[3],
            _ => unreachable!(),
        };
        step(&mut mb, FADE_IN_FRAMES / FRAMES_PER_SECOND);
        let a_peak = match &draws(&mb)[0] {
            EffectPrimitiveDraw::BillboardRing { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_peak > a0, "alpha grows during fade-in");
        step(&mut mb, DURATION_S * 0.4);
        let a_late = match &draws(&mb)[0] {
            EffectPrimitiveDraw::BillboardRing { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_late < a_peak, "alpha drops during fade-out");
    }

    #[test]
    fn dies_after_duration() {
        let mut mb = MagnumBreakEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < DURATION_S * 2.0 {
            status = mb.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert!(matches!(status, EffectStatus::Dead));
    }
}
