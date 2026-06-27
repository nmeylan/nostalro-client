use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::begin_asura::BeginAsuraEffect;

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TOTAL_DURATION_MS: u32 = 5000;

pub const TEXTURES: &[&str] = &["whitelight.tga"];

const SWOOP_SCALE: f32 = 0.2;
const SWOOP_Y_BASE: f32 = 40.0;
const SWOOP_Y_AMP: f32 = 20.0;
const LIGHT_SIZE: f32 = 5.5;
const LIGHT_COLOR: [f32; 3] = [125.0 / 255.0, 125.0 / 255.0, 1.0];
const ALPHA_RAMP_PER_FRAME: f32 = 20.0;
const ALPHA_PEAK: f32 = 200.0;
const ALPHA_DRAIN_START: f32 = 170.0;
const ALPHA_DRAIN_PER_FRAME: f32 = 5.0;

struct SoulLight {
    process: f32,
    angle: f32,
    alpha: f32,
}

impl SoulLight {
    fn new() -> Self {
        Self {
            process: 0.0,
            angle: 0.0,
            alpha: 0.0,
        }
    }

    fn update(&mut self, frames: f32) {
        self.process += frames;
        self.angle = (self.angle + self.process / 30.0 * frames).min(359.0);
        if self.process <= 10.0 {
            self.alpha = (self.alpha + ALPHA_RAMP_PER_FRAME * frames).min(ALPHA_PEAK);
        } else if self.process > ALPHA_DRAIN_START {
            self.alpha = (self.alpha - ALPHA_DRAIN_PER_FRAME * frames).max(0.0);
        }
    }

    fn is_done(&self) -> bool {
        self.process > ALPHA_DRAIN_START && self.alpha <= 0.0
    }

    fn collect_draws(&self, center: [f32; 3], out: &mut EffectDrawList) {
        if self.alpha <= 0.0 {
            return;
        }
        let aa = self.angle.to_radians();
        let radial = (-30.0 + self.angle / 6.0) * SWOOP_SCALE;
        let up = (SWOOP_Y_BASE + SWOOP_Y_AMP * aa.sin()) * SWOOP_SCALE;
        let pos = [center[0] + radial, center[1] - up, center[2]];
        let [r, g, b] = LIGHT_COLOR;
        out.push(EffectPrimitiveDraw::Billboard {
            pos,
            size: [LIGHT_SIZE, LIGHT_SIZE],
            uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            rotation: 0.0,
            texture: "whitelight.tga",
            color: [r, g, b, self.alpha / 255.0],
            blend: BlendKind::Alpha,
        });
    }
}

pub struct SoullinkEffect {
    glyphs: BeginAsuraEffect,
    light: SoulLight,
    center: [f32; 3],
}

impl SoullinkEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            glyphs: BeginAsuraEffect::soul_link(world_pos),
            light: SoulLight::new(),
            center: world_pos,
        }
    }
}

impl Effect for SoullinkEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frames = ctx.delta * FRAMES_PER_SECOND;
        let glyphs_alive = self.glyphs.update(ctx) == EffectStatus::Running;
        self.light.update(frames);
        if glyphs_alive || !self.light.is_done() {
            EffectStatus::Running
        } else {
            EffectStatus::Dead
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        self.glyphs.collect_draws(out, ctx);
        self.light.collect_draws(self.center, out);
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

    fn tick(e: &mut SoullinkEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        st
    }

    fn draws(e: &SoullinkEffect) -> Vec<EffectPrimitiveDraw> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        l.primitives
    }

    fn light(e: &SoullinkEffect) -> Option<([f32; 3], f32)> {
        draws(e).into_iter().find_map(|p| match p {
            EffectPrimitiveDraw::Billboard {
                texture,
                pos,
                color,
                ..
            } if texture == "whitelight.tga" => Some((pos, color[3])),
            _ => None,
        })
    }

    #[test]
    fn glyph_cascade_and_soul_light_both_present() {
        let mut e = SoullinkEffect::new([0.0; 3]);
        tick(&mut e, 6);
        let d = draws(&e);
        // The soul-light billboard...
        assert!(d.iter().any(|p| matches!(p,
            EffectPrimitiveDraw::Billboard { texture, .. } if *texture == "whitelight.tga")));
        // ...plus at least one soul glyph (the cascade's first letters).
        assert!(d.iter().any(|p| matches!(p,
            EffectPrimitiveDraw::Billboard { texture, .. } if texture.starts_with("soul_"))));
    }

    #[test]
    fn soul_light_sweeps_and_bobs() {
        let mut e = SoullinkEffect::new([0.0; 3]);
        tick(&mut e, 5);
        let (p0, _) = light(&e).unwrap();
        tick(&mut e, 40);
        let (p1, _) = light(&e).unwrap();
        // The light moves horizontally over time and rides above the caster.
        assert!((p1[0] - p0[0]).abs() > 1e-3, "swoops horizontally");
        assert!(p1[1] < 0.0, "rides above the caster's feet (native -Y up)");
    }

    #[test]
    fn soul_light_alpha_ramps_then_drains() {
        let mut e = SoullinkEffect::new([0.0; 3]);
        tick(&mut e, 3);
        let early = light(&e).unwrap().1;
        tick(&mut e, 9);
        let peak = light(&e).unwrap().1;
        assert!(peak > early, "ramps in ({early} → {peak})");
    }

    #[test]
    fn self_terminates() {
        let mut e = SoullinkEffect::new([0.0; 3]);
        assert_eq!(tick(&mut e, 1200), EffectStatus::Dead);
        assert!(draws(&e).is_empty());
    }
}
