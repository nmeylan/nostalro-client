use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_TEXTURE: &str = "ring_blue.tga";
pub const TEXTURES: &[&str] = &[RING_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 55.0;
const DURATION_S: f32 = DURATION_FRAMES / FRAMES_PER_SECOND;

pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const OUTER_BOTTOM_RADIUS: f32 = 5.0;
const OUTER_TOP_RADIUS_INIT: f32 = 6.0;
const OUTER_TOP_RADIUS_SPEED: f32 = 0.08;
const OUTER_HEIGHT_INIT: f32 = 6.5;
const OUTER_HEIGHT_SPEED: f32 = -0.1;
const OUTER_SPIN_DEG_PER_FRAME: f32 = -10.0;
const OUTER_ALPHA_INIT: f32 = 5.0 / 255.0;
const OUTER_ALPHA_MAX: f32 = 245.0 / 255.0;
const OUTER_ALPHA_SPEED_PER_FRAME: f32 = 10.0 / 255.0;
const OUTER_FADE_OUT_AT: f32 = DURATION_FRAMES - DURATION_FRAMES / 5.0;

const INNER_RADIUS: f32 = 4.5;
const INNER_HEIGHT_SPEED: f32 = 2.5;
const INNER_HEIGHT_ACCEL: f32 = -(INNER_HEIGHT_SPEED / DURATION_FRAMES) * 2.0;
const INNER_SPIN_DEG_PER_FRAME: f32 = 10.0;
const INNER_ALPHA: f32 = 160.0 / 255.0;
const INNER_FADE_OUT_AT: f32 = DURATION_FRAMES - DURATION_FRAMES / 3.0;
const INNER_SIDES: u32 = 8;
const OUTER_SIDES: u32 = 24;

fn alpha_fade_out(frame: f32, peak: f32, fade_out_at: f32) -> f32 {
    if frame < fade_out_at {
        peak
    } else {
        let t = ((frame - fade_out_at) / (DURATION_FRAMES - fade_out_at)).clamp(0.0, 1.0);
        peak * (1.0 - t)
    }
}

fn outer_alpha(frame: f32) -> f32 {
    let fade_in_target =
        (OUTER_ALPHA_INIT + OUTER_ALPHA_SPEED_PER_FRAME * frame).min(OUTER_ALPHA_MAX);
    alpha_fade_out(frame, fade_in_target, OUTER_FADE_OUT_AT)
}

fn inner_height(frame: f32) -> f32 {
    INNER_HEIGHT_SPEED * frame + INNER_HEIGHT_ACCEL * frame * (frame + 1.0) / 2.0
}

pub struct EntryEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl EntryEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
        }
    }

    fn frame(&self) -> f32 {
        (self.age * FRAMES_PER_SECOND).clamp(0.0, DURATION_FRAMES)
    }
}

impl Effect for EntryEffect {
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

        let outer_height = (OUTER_HEIGHT_INIT + OUTER_HEIGHT_SPEED * frame).max(0.0);
        let outer_top = OUTER_TOP_RADIUS_INIT + OUTER_TOP_RADIUS_SPEED * frame;
        if outer_height > 0.0 {
            let outer_a = outer_alpha(frame);
            out.push(EffectPrimitiveDraw::Cylinder {
                base: self.world_pos,
                bottom_size: OUTER_BOTTOM_RADIUS,
                top_size: outer_top,
                height: outer_height,
                sides: OUTER_SIDES,
                rotation: (frame * OUTER_SPIN_DEG_PER_FRAME).to_radians(),
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                uv_scroll: [0.0, 0.0],
                texture: RING_TEXTURE,
                color: [1.0, 1.0, 1.0, outer_a],
                alpha_bottom: outer_a,
                blend: BlendKind::Additive,
            });
        }

        let inner_height_val = inner_height(frame).max(0.0);
        if inner_height_val > 0.0 {
            let inner_alpha = alpha_fade_out(frame, INNER_ALPHA, INNER_FADE_OUT_AT);
            out.push(EffectPrimitiveDraw::Cylinder {
                base: self.world_pos,
                bottom_size: INNER_RADIUS,
                top_size: INNER_RADIUS,
                height: inner_height_val,
                sides: INNER_SIDES,
                rotation: (frame * INNER_SPIN_DEG_PER_FRAME).to_radians(),
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                uv_scroll: [0.0, 0.0],
                texture: RING_TEXTURE,
                color: [1.0, 1.0, 1.0, inner_alpha],
                alpha_bottom: inner_alpha,
                blend: BlendKind::Additive,
            });
        }
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

    fn step(effect: &mut EntryEffect, dt: f32) {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn draws(effect: &EntryEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_two_cylinders_with_opposing_spin_and_8strip_inner() {
        let mut e = EntryEffect::new([0.0; 3]);
        step(&mut e, 10.0 / FRAMES_PER_SECOND);
        let prims = draws(&e);
        assert_eq!(prims.len(), 2);

        let (
            outer_bottom,
            outer_top,
            outer_sides,
            outer_rot,
            inner_bottom,
            inner_top,
            inner_sides,
            inner_rot,
        ) = match (&prims[0], &prims[1]) {
            (
                EffectPrimitiveDraw::Cylinder {
                    bottom_size: b0,
                    top_size: t0,
                    sides: s0,
                    rotation: r0,
                    blend: bl0,
                    ..
                },
                EffectPrimitiveDraw::Cylinder {
                    bottom_size: b1,
                    top_size: t1,
                    sides: s1,
                    rotation: r1,
                    blend: bl1,
                    ..
                },
            ) => {
                assert_eq!(*bl0, BlendKind::Additive);
                assert_eq!(*bl1, BlendKind::Additive);
                (*b0, *t0, *s0, *r0, *b1, *t1, *s1, *r1)
            }
            _ => panic!("expected two Cylinder prims"),
        };
        assert_eq!(outer_sides, OUTER_SIDES);
        assert_eq!(inner_sides, 8, "arcAngle=45 → 8 vertical strips");
        assert!(outer_rot < 0.0, "outer spins CCW (longSpeed=-10)");
        assert!(inner_rot > 0.0, "inner spins CW (longSpeed=+10)");
        assert!(
            outer_top > outer_bottom,
            "outer is a flared chalice (top wider than bottom): {outer_bottom} → {outer_top}",
        );
        assert!(
            (inner_top - inner_bottom).abs() < 1e-4,
            "inner is a true cylinder (top == bottom)"
        );
    }

    #[test]
    fn outer_top_widens_over_lifetime() {
        let mut e = EntryEffect::new([0.0; 3]);
        step(&mut e, 0.0);
        let (b0, t0) = match &draws(&e)[0] {
            EffectPrimitiveDraw::Cylinder {
                bottom_size,
                top_size,
                ..
            } => (*bottom_size, *top_size),
            _ => unreachable!(),
        };
        step(&mut e, DURATION_S * 0.8);
        let (b1, t1) = match draws(&e).first() {
            Some(EffectPrimitiveDraw::Cylinder {
                bottom_size,
                top_size,
                ..
            }) => (*bottom_size, *top_size),
            _ => panic!("outer cone disappeared too early"),
        };
        assert!((b1 - b0).abs() < 1e-4, "bottom stays at innerSize=5");
        assert!(t1 > t0, "top widens with outerSpeed");
    }

    #[test]
    fn inner_height_swells_then_returns_near_zero() {
        let mut e = EntryEffect::new([0.0; 3]);
        step(&mut e, 1.0 / FRAMES_PER_SECOND);
        let h_early = match draws(&e).get(1) {
            Some(EffectPrimitiveDraw::Cylinder { height, .. }) => *height,
            _ => panic!("inner cylinder expected at frame 1"),
        };

        step(&mut e, DURATION_S * 0.5);
        let h_mid = match &draws(&e)[1] {
            EffectPrimitiveDraw::Cylinder { height, .. } => *height,
            _ => unreachable!(),
        };
        assert!(h_mid > h_early + 1.0, "height grows by mid-life");

        step(&mut e, DURATION_S * 0.49);
        let h_late = match draws(&e).get(1) {
            Some(EffectPrimitiveDraw::Cylinder { height, .. }) => *height,
            // height may have hit zero — primitive is then skipped entirely
            None => 0.0,
            _ => unreachable!(),
        };
        assert!(h_late < h_mid, "height collapses by end of life");
    }

    #[test]
    fn outer_alpha_fades_in_then_out() {
        let mut e = EntryEffect::new([0.0; 3]);
        step(&mut e, 0.0);
        let a0 = match &draws(&e)[0] {
            EffectPrimitiveDraw::Cylinder { color, .. } => color[3],
            _ => unreachable!(),
        };
        step(&mut e, 30.0 / FRAMES_PER_SECOND);
        let a_peak = match &draws(&e)[0] {
            EffectPrimitiveDraw::Cylinder { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_peak > a0, "alpha rises during fade-in");
        step(&mut e, (DURATION_FRAMES - 30.0 - 2.0) / FRAMES_PER_SECOND);
        let a_late = match draws(&e).first() {
            Some(EffectPrimitiveDraw::Cylinder { color, .. }) => color[3],
            _ => 0.0,
        };
        assert!(a_late < a_peak, "alpha drops during fade-out");
    }

    #[test]
    fn dies_after_duration() {
        let mut e = EntryEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < DURATION_S * 2.0 {
            status = e.update(&EffectUpdateCtx {
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
