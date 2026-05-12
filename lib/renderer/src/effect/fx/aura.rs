//! Lv99 / persistent aura. Three layered billboards at increasing radii
//! around the entity, with a slow pulse. Currently uses the fallback white
//! texture tinted yellow/orange/red — when GRF aura textures are loaded the
//! `params.texture` override can drop them into each layer.

use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const LAYER_COUNT: usize = 3;
/// Layer radii in world units.
const LAYER_RADII: [f32; LAYER_COUNT] = [12.0, 18.0, 26.0];
/// Per-layer base color (RGBA, 0..1). Alphas kept low so the three
/// additive billboards don't pile up into solid white at the center.
const LAYER_COLORS: [[f32; 4]; LAYER_COUNT] = [
    [1.00, 0.85, 0.20, 0.30], // inner: yellow
    [1.00, 0.55, 0.10, 0.22], // mid: orange
    [0.95, 0.20, 0.05, 0.14], // outer: red, faint
];
/// Vertical offset of each layer (slight stacking so they don't z-fight).
const LAYER_Y_OFFSET: [f32; LAYER_COUNT] = [-1.0, -0.5, 0.0];

pub struct Aura {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl Aura {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            world_pos: params.world_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for Aura {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        // Auras are persistent; the holder despawns them via duration or
        // explicit despawn.
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        // Slow pulse: 6% size oscillation, ~0.7 Hz.
        let pulse = 1.0 + 0.06 * (self.age * 4.4).sin();
        let uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        for i in 0..LAYER_COUNT {
            let radius = LAYER_RADII[i] * pulse;
            let base = LAYER_COLORS[i];
            let color = [
                base[0] * self.tint[0],
                base[1] * self.tint[1],
                base[2] * self.tint[2],
                base[3] * self.tint[3],
            ];
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [
                    self.world_pos[0],
                    self.world_pos[1] + LAYER_Y_OFFSET[i],
                    self.world_pos[2],
                ],
                size: [radius * 2.0, radius * 2.0],
                uv,
                texture: self.texture,
                color,
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_three_layers_at_spawn_position() {
        let params = CustomParams {
            world_pos: [10.0, 5.0, -3.0],
            target_pos: None,
            texture: None,
            tint: None,
        };
        let aura = Aura::new(&params);

        let mut list = EffectDrawList::new();
        // Build a minimal RenderCtx by hand. Camera is unused by Aura's
        // collect_draws, so a default is fine.
        let camera = crate::camera::Camera::default();
        let ctx = EffectRenderCtx {
            camera: &camera,
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        };
        aura.collect_draws(&mut list, &ctx);

        assert_eq!(list.len(), LAYER_COUNT);
        for prim in &list.primitives {
            match prim {
                EffectPrimitiveDraw::Billboard { pos, blend, .. } => {
                    assert!((pos[0] - 10.0).abs() < 0.01);
                    assert!((pos[2] - (-3.0)).abs() < 0.01);
                    assert_eq!(*blend, BlendKind::Additive);
                }
                _ => panic!("Aura should emit only billboards"),
            }
        }
    }

    #[test]
    fn update_keeps_it_running() {
        let aura = &mut Aura::new(&CustomParams::default());
        let ctx = EffectUpdateCtx { dt: 0.1 };
        assert_eq!(aura.update(&ctx), EffectStatus::Running);
        // After many seconds, still running.
        for _ in 0..100 {
            aura.update(&ctx);
        }
        assert_eq!(aura.update(&ctx), EffectStatus::Running);
    }
}
