use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::radial_emitter::{RADIAL_EMITTER_SLOTS, RadialEmitter, RadialEmitterSlot};

pub const TEXTURE: &str = "ring_yellow.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;

const SIDES: u32 = 24;

const DISTANCE_BASE: f32 = 1.0;
const DISTANCE_STEP: f32 = 0.6;
const DISTANCE_GROWTH_PER_FRAME: f32 = 0.1;

const MAX_HEIGHT: f32 = 7.0;

const RISE_INITIAL_DEG: f32 = 80.0;
const RISE_FINAL_DEG: f32 = 30.0;
const RISE_DECAY_PER_FRAME: f32 = 1.0;

const ALPHA_RISE_FRAMES: f32 = 10.0;
const ALPHA_B_PEAK: f32 = 200.0;
const ALPHA_B_RISE_PER_FRAME: f32 = 20.0;
const ALPHA_B_FALL_PER_FRAME: f32 = 2.0;
const ALPHA_DIVISOR: f32 = 255.0;

const TOTAL_FRAMES: f32 = ALPHA_RISE_FRAMES + ALPHA_B_PEAK / ALPHA_B_FALL_PER_FRAME;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const ROT_DEG_PER_FRAME: f32 = 3.0;

pub struct Gumgang2Effect {
    world_pos: [f32; 3],
    age_frames: f32,
    emitter: RadialEmitter,
}

impl Gumgang2Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let mut slots = [RadialEmitterSlot::dormant(); RADIAL_EMITTER_SLOTS];
        for (ec, slot) in slots.iter_mut().enumerate() {
            *slot = RadialEmitterSlot::spawn(
                DISTANCE_BASE + ec as f32 * DISTANCE_STEP,
                RISE_INITIAL_DEG,
                MAX_HEIGHT,
            );
        }
        Self {
            world_pos,
            age_frames: 0.0,
            emitter: RadialEmitter::from_slots(slots),
        }
    }
}

fn alpha_at(frame: f32) -> f32 {
    let alpha_b = if frame <= ALPHA_RISE_FRAMES {
        (ALPHA_B_RISE_PER_FRAME * frame).min(ALPHA_B_PEAK)
    } else {
        (ALPHA_B_PEAK - ALPHA_B_FALL_PER_FRAME * (frame - ALPHA_RISE_FRAMES)).max(0.0)
    };
    alpha_b / ALPHA_DIVISOR
}

fn rise_angle_deg(frame: f32) -> f32 {
    (RISE_INITIAL_DEG - RISE_DECAY_PER_FRAME * frame).max(RISE_FINAL_DEG)
}

impl Effect for Gumgang2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let prev_frames = self.age_frames;
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let delta_frames = self.age_frames - prev_frames;

        let new_rise = rise_angle_deg(self.age_frames);
        self.emitter.tick();
        for slot in self.emitter.slots.iter_mut().filter(|s| s.alive) {
            slot.distance += DISTANCE_GROWTH_PER_FRAME * delta_frames;
            slot.rise_angle_deg = new_rise;
            slot.rot_start_deg += ROT_DEG_PER_FRAME * delta_frames;
        }

        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = alpha_at(self.age_frames);
        if alpha <= 0.0 {
            return;
        }

        for (ec, slot) in self.emitter.active() {
            let (sin_rise, cos_rise) = slot.rise_angle_deg.to_radians().sin_cos();
            let max_outward = cos_rise * slot.max_height;
            let max_upward = sin_rise * slot.max_height;
            let rotation_rad = (ec as f32 * 90.0 + slot.rot_start_deg).to_radians();

            out.push(EffectPrimitiveDraw::Cylinder {
                base: self.world_pos,
                bottom_size: slot.distance,
                top_size: slot.distance + max_outward,
                height: max_upward,
                sides: SIDES,
                rotation: rotation_rad,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                uv_scroll: [0.0, 0.0],
                texture: TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
                alpha_bottom: alpha,
                blend: BlendKind::Alpha,
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

    fn step(e: &mut Gumgang2Effect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn draws(e: &Gumgang2Effect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn cone(prim: &EffectPrimitiveDraw) -> (f32, f32, f32, &'static str) {
        match prim {
            EffectPrimitiveDraw::Cylinder {
                bottom_size,
                top_size,
                height,
                texture,
                ..
            } => (*bottom_size, *top_size, *height, *texture),
            _ => panic!("expected Cylinder, got {:?}", prim),
        }
    }

    #[test]
    fn emits_concentric_flared_cones() {
        let mut e = Gumgang2Effect::new([5.0, 0.0, -3.0]);
        step(&mut e, 5.0);
        let prims = draws(&e);
        assert_eq!(prims.len(), RADIAL_EMITTER_SLOTS);
        for (i, prim) in prims.iter().enumerate() {
            let (bottom, top, height, tex) = cone(prim);
            let expected_base =
                DISTANCE_BASE + i as f32 * DISTANCE_STEP + DISTANCE_GROWTH_PER_FRAME * 5.0;
            assert!(
                (bottom - expected_base).abs() < 1e-3,
                "ring {i} bottom={bottom}, want {expected_base}"
            );
            assert!(
                top > bottom,
                "ring {i} should flare: top={top}, bottom={bottom}"
            );
            assert!(height > 0.0, "ring {i} should have height");
            assert_eq!(tex, TEXTURE);
        }
    }

    #[test]
    fn petals_open_over_time() {
        let mut e = Gumgang2Effect::new([0.0; 3]);
        step(&mut e, 4.0);
        let (b_early, t_early, h_early, _) = cone(&draws(&e)[0]);
        let flare_early = t_early - b_early;

        step(&mut e, 36.0);
        let (b_late, t_late, h_late, _) = cone(&draws(&e)[0]);
        let flare_late = t_late - b_late;

        assert!(
            flare_late > flare_early,
            "petals must open over time: flare {flare_early} -> {flare_late}"
        );
        assert!(
            h_late < h_early,
            "vertical reach shrinks as petals open: {h_early} -> {h_late}"
        );
        assert!(
            b_late > b_early,
            "ring expands outward: {b_early} -> {b_late}"
        );
    }

    #[test]
    fn dies_after_total_frames() {
        let mut e = Gumgang2Effect::new([0.0; 3]);
        let s = step(&mut e, TOTAL_FRAMES + 1.0);
        assert!(matches!(s, EffectStatus::Dead));
    }
}
