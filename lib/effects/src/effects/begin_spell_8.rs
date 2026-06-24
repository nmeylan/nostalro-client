//! `EF_BEGINSPELL8` (id 1014) — green casting cylinder.
//!
//! The original game draws this as a single green casting aura
//! (the plan's extra glyph/flame parts do not exist in the original
//! effect). The cast aura
//! launches four sub-emitters: three flared cone rings
//! (`distance` 4.5/5.0/5.5, `rise` 70/57/45°, `max_height` 25/22/19) and a
//! fourth, near-vertical (`rise = 89°`) tall central column at half the alpha
//! (`max_height = 250`, alpha 70) — the casting light shaft.
//!
//! The cast aura uses the same primitive the level-99 ring uses, so the three
//! flared rings reuse [`super::casting_ring`] (with a green tint, since
//! `ring_green.tga` is absent from the classic GRF — `ring_white.tga` tinted
//! green is the documented substitution). The central shaft is one extra
//! narrow vertical [`Frustum`] this module adds on top.
//!
//! Finite effect (the cast aura clamps its lifetime to ≥70 frames). No
//! reference gif exists; validated against the original game's behaviour.
//!
//! [`Frustum`]: EffectPrimitiveDraw::Frustum

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::casting_ring::{CastingRingEffect, BEGINSPELL8 as RING_PARAMS};

const FRAMES_PER_SECOND: f32 = 60.0;
/// The cast aura runs for at least 70 frames; the visible cylinder runs that
/// long then despawns.
const TOTAL_FRAMES: f32 = 70.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const FADE_IN_FRAMES: f32 = 20.0;
const FADE_OUT_FRAMES: f32 = 20.0;

/// The fourth cast-aura emitter — a near-vertical (`rise = 89°`) shaft.
/// Its `max_height = 250` is one of the original's large literals, so it is
/// downscaled to a sprite-relative beam rather than ported 1:1.
const COLUMN_BOTTOM: f32 = 0.8;
const COLUMN_TOP: f32 = 1.6;
const COLUMN_HEIGHT: f32 = 26.0;
const COLUMN_SIDES: u32 = 16;
const COLUMN_ALPHA: f32 = 0.16;
const COLUMN_SPIN_DEG_PER_FRAME: f32 = 4.0;

pub const TEXTURES: &[&str] = super::casting_ring::TEXTURES;

pub struct BeginSpell8Effect {
    rings: CastingRingEffect,
    world_pos: [f32; 3],
    age: f32,
}

impl BeginSpell8Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            rings: CastingRingEffect::new(world_pos, RING_PARAMS),
            world_pos,
            age: 0.0,
        }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }

    /// Shared fade envelope — ramp in, hold, ramp out near the end.
    fn envelope(&self) -> f32 {
        let frame = self.frame();
        let fade_in = (frame / FADE_IN_FRAMES).clamp(0.0, 1.0);
        let fade_out = ((TOTAL_FRAMES - frame) / FADE_OUT_FRAMES).clamp(0.0, 1.0);
        fade_in.min(fade_out)
    }
}

impl Effect for BeginSpell8Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        self.rings.update(ctx);
        if self.frame() >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
        self.rings.set_position(pos);
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        self.rings.collect_draws(out, ctx);

        let alpha = COLUMN_ALPHA * self.envelope();
        if alpha <= 0.0 {
            return;
        }
        let [r, g, b] = RING_PARAMS.color_rgb;
        let spin = -(self.frame() * COLUMN_SPIN_DEG_PER_FRAME).to_radians();
        out.push(EffectPrimitiveDraw::Frustum {
                base_alpha: 1.0,
            base: self.world_pos,
            bottom_size: COLUMN_BOTTOM,
            top_size: COLUMN_TOP,
            height: COLUMN_HEIGHT,
            sides: COLUMN_SIDES,
            arc_angle_deg: 360.0,
            rotation: spin,
            uv_repeat: 1.0,
            uv_scroll: [0.0, 0.0],
            wave_amplitude: 0.0,
            wave_frequency: 1.0,
            wave_phase: 0.0,
            wave_mode: FrustumWaveMode::Sine,
            tilt_x_rad: 0.0,
            rotation_y_rad: 0.0,
            cull_back: false,
            texture: RING_PARAMS.texture,
            color: [r, g, b, alpha],
            blend: BlendKind::Additive,
        });
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

    fn run_to(c: &mut BeginSpell8Effect, target_frame: f32) {
        let delta = (target_frame - c.frame()) / FRAMES_PER_SECOND;
        if delta > 0.0 {
            c.update(&EffectUpdateCtx { delta, camera_target: None, caster_yaw: None });
        }
    }

    fn prims(c: &BeginSpell8Effect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_three_rings_plus_central_column() {
        let mut c = BeginSpell8Effect::new([5.0, 0.0, 5.0]);
        run_to(&mut c, FADE_IN_FRAMES);
        let p = prims(&c);
        // 3 flared rings (reused) + 1 tall narrow column.
        assert_eq!(p.len(), 4);
        let tallest = p.iter().filter_map(|d| match d {
            EffectPrimitiveDraw::Frustum { height, top_size, bottom_size, .. } => {
                Some((*height, *top_size - *bottom_size))
            }
            _ => None,
        }).max_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).unwrap();
        // The column is the tallest and the narrowest (least flare).
        assert!(tallest.0 >= COLUMN_HEIGHT - 1e-3);
        assert!(tallest.1 < 1.0, "column barely flares");
    }

    #[test]
    fn all_primitives_are_green_tinted() {
        let mut c = BeginSpell8Effect::new([0.0; 3]);
        run_to(&mut c, FADE_IN_FRAMES);
        for d in prims(&c) {
            if let EffectPrimitiveDraw::Frustum { color, .. } = d {
                assert!(color[1] > color[0] && color[1] > color[2], "green dominant");
            }
        }
    }

    #[test]
    fn alpha_fades_in_then_out() {
        let mut c = BeginSpell8Effect::new([0.0; 3]);
        run_to(&mut c, 3.0);
        let early = column_alpha(&c);
        run_to(&mut c, TOTAL_FRAMES * 0.5);
        let mid = column_alpha(&c);
        run_to(&mut c, TOTAL_FRAMES - 3.0);
        let late = column_alpha(&c);
        assert!(mid > early, "ramps in ({early} → {mid})");
        assert!(mid > late, "ramps out ({mid} → {late})");
    }

    #[test]
    fn finishes_after_duration() {
        let mut c = BeginSpell8Effect::new([0.0; 3]);
        run_to(&mut c, TOTAL_FRAMES - 1.0);
        assert_eq!(
            c.update(&EffectUpdateCtx { delta: 0.1, camera_target: None, caster_yaw: None }),
            EffectStatus::Dead
        );
    }

    fn column_alpha(c: &BeginSpell8Effect) -> f32 {
        // The column is the last primitive pushed.
        match prims(c).last().unwrap() {
            EffectPrimitiveDraw::Frustum { color, .. } => color[3],
            _ => panic!(),
        }
    }
}
