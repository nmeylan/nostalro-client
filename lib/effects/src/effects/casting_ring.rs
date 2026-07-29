use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

const NUM_RINGS: usize = 3;
const RING_SIDES: u32 = 20;
const RING_ARC_DEG: f32 = 360.0;
const RING_UV_REPEAT: f32 = 1.0;
const RING_SPIN_BASE_DEG_PER_FRAME: f32 = 3.0;
const FADE_IN_FRAMES: f32 = 20.0;

#[derive(Clone, Copy, Debug)]
pub struct CastingRingParams {
    pub texture: &'static str,
    pub color_rgb: [f32; 3],
    pub bottom_size: f32,
    pub top_size: f32,
    pub height: f32,
    pub alpha_max: f32,
    pub base_alpha: f32,
}

pub const LV99: CastingRingParams = CastingRingParams {
    texture: "ring_blue.tga",
    color_rgb: [0.55, 0.55, 1.00],
    bottom_size: 3.9,
    top_size: 12.5,
    height: 12.3,
    alpha_max: 0.47,
    base_alpha: 0.25,
};

pub const GREEN995: CastingRingParams = CastingRingParams {
    texture: "ring_white.tga",
    color_rgb: [0.14, 1.00, 0.14],
    bottom_size: 2.5,
    top_size: 8.0,
    height: 14.0,
    alpha_max: 0.30,
    base_alpha: 1.0,
};

pub const MAP_AURA: CastingRingParams = CastingRingParams {
    texture: "ring_blue.tga",
    color_rgb: [0.55, 0.55, 1.00],
    bottom_size: 12.9,
    top_size: 18.0,
    height: 12.0,
    alpha_max: 50.0 / 255.0,
    base_alpha: 1.0,
};

pub const BEGINSPELL8: CastingRingParams = CastingRingParams {
    texture: "ring_white.tga",
    color_rgb: [0.45, 1.00, 0.55],
    bottom_size: 2.5,
    top_size: 7.5,
    height: 13.0,
    alpha_max: 0.30,
    base_alpha: 1.0,
};

pub const TEXTURES: &[&str] = &["ring_blue.tga", "ring_white.tga"];

pub struct CastingRingEffect {
    params: CastingRingParams,
    world_pos: [f32; 3],
    age: f32,
}

impl CastingRingEffect {
    pub fn new(world_pos: [f32; 3], params: CastingRingParams) -> Self {
        Self {
            params,
            world_pos,
            age: 0.0,
        }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }
}

impl Effect for CastingRingEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        EffectStatus::Running
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [r, g, b] = self.params.color_rgb;
        let frame = self.frame();
        let alpha = self.params.alpha_max * (frame / FADE_IN_FRAMES).clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }

        for i in 0..NUM_RINGS {
            let fi = i as f32;
            let height = self.params.height - fi * 0.5;
            let bottom_size = self.params.bottom_size;
            let top_size = self.params.top_size + fi * 0.3;
            let rot_start = fi * std::f32::consts::FRAC_PI_2;
            let spin = -(frame * (RING_SPIN_BASE_DEG_PER_FRAME + fi)).to_radians();

            out.push(EffectPrimitiveDraw::Frustum {
                base_alpha: self.params.base_alpha,
                base: self.world_pos,
                bottom_size,
                top_size,
                height,
                sides: RING_SIDES,
                arc_angle_deg: RING_ARC_DEG,
                rotation: rot_start + spin,
                uv_repeat: RING_UV_REPEAT,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: 0.0,
                wave_frequency: 1.0,
                wave_phase: 0.0,
                wave_mode: FrustumWaveMode::Sine,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                cull_back: false,
                texture: self.params.texture,
                color: [r, g, b, alpha],
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

    fn run_to(c: &mut CastingRingEffect, target_frame: f32) {
        let delta = (target_frame - c.frame()) / FRAMES_PER_SECOND;
        if delta > 0.0 {
            c.update(&EffectUpdateCtx {
                delta,
                camera_target: None,
                caster_yaw: None,
            });
        }
    }

    fn rings(c: &CastingRingEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_three_flared_rings_centered_on_caster() {
        let caster = [10.0, 5.0, 20.0];
        let mut c = CastingRingEffect::new(caster, LV99);
        run_to(&mut c, FADE_IN_FRAMES);
        let prims = rings(&c);
        assert_eq!(prims.len(), NUM_RINGS);
        for p in &prims {
            let EffectPrimitiveDraw::Frustum {
                base,
                top_size,
                bottom_size,
                arc_angle_deg,
                blend,
                ..
            } = p
            else {
                panic!("expected Frustum");
            };
            assert!((base[0] - caster[0]).abs() < 1e-4 && (base[2] - caster[2]).abs() < 1e-4);
            assert!(
                top_size > bottom_size,
                "ring should flare outward as it rises"
            );
            assert_eq!(*arc_angle_deg, RING_ARC_DEG);
            assert_eq!(*blend, BlendKind::Additive);
        }
    }

    #[test]
    fn rings_spin_at_distinct_rates() {
        let mut c = CastingRingEffect::new([0.0; 3], LV99);
        run_to(&mut c, 30.0);
        let rotations: Vec<f32> = rings(&c)
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Frustum { rotation, .. } => *rotation,
                _ => panic!(),
            })
            .collect();
        assert!((rotations[0] - rotations[1]).abs() > 1e-3);
        assert!((rotations[1] - rotations[2]).abs() > 1e-3);
    }

    #[test]
    fn alpha_ramps_in_then_holds() {
        let mut c = CastingRingEffect::new([0.0; 3], LV99);
        run_to(&mut c, 5.0);
        let early = ring_alpha(&c);
        run_to(&mut c, FADE_IN_FRAMES);
        let peak = ring_alpha(&c);
        run_to(&mut c, FADE_IN_FRAMES * 4.0);
        let held = ring_alpha(&c);
        assert!(peak > early, "alpha ramps in ({early} → {peak})");
        assert!((held - peak).abs() < 1e-4, "alpha holds after ramp-in");
    }

    fn ring_alpha(c: &CastingRingEffect) -> f32 {
        match &rings(c)[0] {
            EffectPrimitiveDraw::Frustum { color, .. } => color[3],
            _ => panic!(),
        }
    }

    #[test]
    fn variants_use_real_distinct_textures() {
        assert_ne!(LV99.texture, GREEN995.texture);
        for p in [LV99, GREEN995, MAP_AURA, BEGINSPELL8] {
            assert!(TEXTURES.contains(&p.texture));
        }
    }

    #[test]
    fn green995_resolves_custom_and_renders_a_green_flared_ring() {
        use crate::factory::make_effect;
        use crate::spec::{EffectAnchor, EffectSpec};
        use crate::table::effect_spec;
        use models::enums::effect_id::EffectId;

        // Alias deleted + custom bucket ⇒ both green level-99 ids dispatch via
        // the factory, not a (missing) STR.
        for id in [
            EffectId::Green993,
            EffectId::Green995,
            EffectId::Green996,
        ] {
            assert!(
                matches!(effect_spec(id), Some(EffectSpec::Custom)),
                "{id:?} should resolve to Custom"
            );
        }
        let green = crate::effects::sparkle_column::GREEN99.color_rgb;
        assert!(green[1] > green[0] && green[1] > green[2], "green column");

        assert_eq!(GREEN995.texture, "ring_white.tga");
        assert!(
            GREEN995.color_rgb[1] > GREEN995.color_rgb[0]
                && GREEN995.color_rgb[1] > GREEN995.color_rgb[2],
            "green-dominant tint"
        );

        let mut eff = make_effect(
            EffectId::Green995,
            EffectAnchor::Point([0.0; 3]),
            None,
            None,
            None,
        )
        .expect("Green995 dispatches via factory");
        eff.update(&EffectUpdateCtx {
            delta: FADE_IN_FRAMES / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        });
        let mut list = EffectDrawList::new();
        eff.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), NUM_RINGS);
        assert!(
            list.primitives
                .iter()
                .all(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
        );
    }

    #[test]
    fn never_self_terminates() {
        let mut c = CastingRingEffect::new([0.0; 3], LV99);
        for _ in 0..200 {
            assert_eq!(
                c.update(&EffectUpdateCtx {
                    delta: 0.1,
                    camera_target: None,
                    caster_yaw: None
                }),
                EffectStatus::Running
            );
        }
    }
}
