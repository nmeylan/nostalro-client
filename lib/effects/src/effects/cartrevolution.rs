use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const STR_FILE: &str = "CartRevolution";
pub const RING_TEXTURE: &str = "ring_yellow.tga";
pub const SPHERE_TEXTURE: &str = "대폭발.tga";
pub const TEXTURES: &[&str] = &[RING_TEXTURE, SPHERE_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;

const PARENT_DURATION_FRAMES: f32 = 300.0;
pub const TOTAL_DURATION_MS: u32 = (PARENT_DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const BURST_FRAMES: [f32; 2] = [7.0, 20.0];
const SUB_DURATION_FRAMES: f32 = 20.0;

const RING_INITIAL_RADIUS: f32 = 0.0;
const RING_RADIUS_SPEED_PER_FRAME: f32 = 1.75;
const RING_RADIUS_ACCEL_PER_FRAME2: f32 =
    -(RING_RADIUS_SPEED_PER_FRAME / SUB_DURATION_FRAMES) / 2.0;
const RING_INNER_SIZE: f32 = 5.0;
const RING_PEAK_ALPHA: f32 = 180.0 / 255.0;
const RING_FADE_IN_FRAMES: f32 = 15.0;
const RING_FADE_OUT_START_FRAME: f32 = 5.0;
const RING_UV_REPEAT: f32 = 4.0;

// Burst sphere.
const SPHERE_INITIAL_RADIUS: f32 = 0.0;
const SPHERE_RADIUS_SPEED_PER_FRAME: f32 = 1.35;
const SPHERE_RADIUS_ACCEL_PER_FRAME2: f32 =
    -(SPHERE_RADIUS_SPEED_PER_FRAME / SUB_DURATION_FRAMES) / 2.0;
const SPHERE_PEAK_ALPHA: f32 = 240.0 / 255.0;
const SPHERE_FADE_IN_FRAMES: f32 = 7.0;
const SPHERE_FADE_OUT_START_FRAME: f32 = 5.0;
const SPHERE_ROT_DEG_PER_FRAME: f32 = 3.0;
const SPHERE_SIDES_LAT: u32 = 5;
const SPHERE_SIDES_LON: u32 = 10;

fn radius_at(initial: f32, speed: f32, accel: f32, frame: f32) -> f32 {
    initial + speed * frame + accel * frame * (frame + 1.0) / 2.0
}

fn fade_alpha(peak: f32, frame: f32, fade_in: f32, fade_out_start: f32, total: f32) -> f32 {
    if frame < 0.0 || frame >= total {
        return 0.0;
    }
    let in_curve = (frame / fade_in).clamp(0.0, 1.0);
    let out_curve = if frame <= fade_out_start {
        1.0
    } else {
        ((total - frame) / (total - fade_out_start)).max(0.0)
    };
    peak * in_curve * out_curve
}

pub struct CartRevolutionEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl CartRevolutionEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
        }
    }

    fn emit_burst(&self, out: &mut EffectDrawList, local_frame: f32) {
        if local_frame < 0.0 || local_frame >= SUB_DURATION_FRAMES {
            return;
        }

        let ring_radius = radius_at(
            RING_INITIAL_RADIUS,
            RING_RADIUS_SPEED_PER_FRAME,
            RING_RADIUS_ACCEL_PER_FRAME2,
            local_frame,
        );
        let ring_alpha = fade_alpha(
            RING_PEAK_ALPHA,
            local_frame,
            RING_FADE_IN_FRAMES,
            RING_FADE_OUT_START_FRAME,
            SUB_DURATION_FRAMES,
        );
        if ring_radius > 0.0 && ring_alpha > 0.0 {
            let thickness = ring_radius.min(RING_INNER_SIZE);
            out.push(EffectPrimitiveDraw::GroundDisc {
                center: self.world_pos,
                radius: ring_radius,
                thickness,
                rotation: 0.0,
                arc_angle_deg: 360.0,
                uv_repeat: RING_UV_REPEAT,
                texture: RING_TEXTURE,
                color: [1.0, 1.0, 1.0, ring_alpha],
                blend: BlendKind::Alpha,
                no_depth: true,
                tilt_rad: 0.0,
                spin_rad: 0.0,
            });
        }

        let sphere_radius = radius_at(
            SPHERE_INITIAL_RADIUS,
            SPHERE_RADIUS_SPEED_PER_FRAME,
            SPHERE_RADIUS_ACCEL_PER_FRAME2,
            local_frame,
        );
        let sphere_alpha = fade_alpha(
            SPHERE_PEAK_ALPHA,
            local_frame,
            SPHERE_FADE_IN_FRAMES,
            SPHERE_FADE_OUT_START_FRAME,
            SUB_DURATION_FRAMES,
        );
        if sphere_radius > 0.0 && sphere_alpha > 0.0 {
            let long_offset = (local_frame * SPHERE_ROT_DEG_PER_FRAME).to_radians();
            out.push(EffectPrimitiveDraw::Sphere {
                center: self.world_pos,
                radius: sphere_radius,
                sides_lat: SPHERE_SIDES_LAT,
                sides_lon: SPHERE_SIDES_LON,
                longitude_offset: long_offset,
                longitude_arc: std::f32::consts::TAU,
                uv_repeat: [1.0, 1.0],
                texture: SPHERE_TEXTURE,
                color: [1.0, 1.0, 1.0, sphere_alpha],
                blend: BlendKind::Alpha,
                no_depth: true,
            });
        }
    }
}

impl Effect for CartRevolutionEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age * FRAMES_PER_SECOND >= PARENT_DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FRAMES_PER_SECOND;
        for spawn_frame in BURST_FRAMES {
            self.emit_burst(out, frame - spawn_frame);
        }
    }

    fn str_overlay(&self) -> Option<&'static str> {
        Some(STR_FILE)
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

    fn step_and_draw(e: &mut CartRevolutionEffect, dt: f32) -> Vec<EffectPrimitiveDraw> {
        e.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_both_bursts_and_carries_str_overlay() {
        let mut e = CartRevolutionEffect::new([10.0, 0.0, 20.0]);
        assert_eq!(e.str_overlay(), Some(STR_FILE));

        let dt = 8.0 / FRAMES_PER_SECOND;
        let p8 = step_and_draw(&mut e, dt);
        let ring_count = p8
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { .. }))
            .count();
        let sphere_count = p8
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Sphere { .. }))
            .count();
        assert_eq!(ring_count, 1, "burst #1 emits one ring");
        assert_eq!(sphere_count, 1, "burst #1 emits one sphere");

        let sphere_radius = p8.iter().find_map(|p| match p {
            EffectPrimitiveDraw::Sphere { radius, .. } => Some(*radius),
            _ => None,
        });
        assert!(
            sphere_radius.is_some_and(|r| r < 2.0),
            "dome grows from zero (dhxj parity), not a seeded radius"
        );

        let p25 = step_and_draw(&mut e, 17.0 / FRAMES_PER_SECOND);
        let rings = p25
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { .. }))
            .count();
        assert!(rings >= 1, "second burst alive at frame 25");

        let p60 = step_and_draw(&mut e, 35.0 / FRAMES_PER_SECOND);
        let prim_count = p60.len();
        assert_eq!(prim_count, 0, "no primitives after both bursts expire");
    }

    #[test]
    fn dies_after_parent_emitter_finishes() {
        let mut e = CartRevolutionEffect::new([0.0; 3]);
        let s = e.update(&EffectUpdateCtx {
            delta: TOTAL_DURATION_MS as f32 / 1000.0 + 0.1,
            camera_target: None,
            caster_yaw: None,
        });
        assert!(matches!(s, EffectStatus::Dead));
    }
}
