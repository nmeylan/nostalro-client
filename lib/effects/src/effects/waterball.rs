//! `EF_WATERBALL` — the water column Water Ball raises at the caster while it
//! is being cast (id 116). Standing in water, the surface climbs the caster.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const TEXTURE_CYCLE: &[&str] = &["water_out_a.bmp", "water_out_b.bmp", "water_out_c.bmp"];
pub const TEXTURES: &[&str] = TEXTURE_CYCLE;

const FPS: f32 = 60.0;
const DURATION_FRAMES: f32 = 250.0;
const FRAMES_PER_TEX: f32 = 5.0;
pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FPS * 1000.0) as u32;

/// `m_widthSize` / `m_heightSize` 3.5 are half extents; `Billboard::size` is
/// full.
const SIZE: [f32; 2] = [7.0, 7.0];
/// Spawn height above the caster's feet (native RO −Y is up).
const Y_OFFSET: f32 = -5.0;
/// The column climbs 25 units across its life.
const RISE: f32 = 25.0;

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

pub struct WaterballEffect {
    pos: [f32; 3],
    frame: f32,
}

impl WaterballEffect {
    pub fn new(caster: [f32; 3]) -> Self {
        Self {
            pos: [caster[0], caster[1] + Y_OFFSET, caster[2]],
            frame: 0.0,
        }
    }

    fn current_pos(&self) -> [f32; 3] {
        let risen = RISE * (self.frame / DURATION_FRAMES).clamp(0.0, 1.0);
        [self.pos[0], self.pos[1] - risen, self.pos[2]]
    }

    fn texture(&self) -> &'static str {
        let step = (self.frame / FRAMES_PER_TEX) as usize;
        TEXTURE_CYCLE[step % TEXTURE_CYCLE.len()]
    }
}

impl Effect for WaterballEffect {
    fn set_position(&mut self, pos: [f32; 3]) {
        self.pos = [pos[0], pos[1] + Y_OFFSET, pos[2]];
    }

    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.frame += ctx.delta * FPS;
        if self.frame >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        out.push(EffectPrimitiveDraw::Billboard {
            pos: self.current_pos(),
            size: SIZE,
            uv: UNIT_UV,
            rotation: 0.0,
            texture: self.texture(),
            color: [1.0, 1.0, 1.0, 1.0],
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

    fn draw(e: &WaterballEffect) -> ([f32; 3], &'static str) {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        match &list.primitives[..] {
            [
                EffectPrimitiveDraw::Billboard {
                    pos,
                    texture,
                    blend: BlendKind::Additive,
                    size,
                    ..
                },
            ] => {
                assert_eq!(*size, SIZE);
                (*pos, *texture)
            }
            other => panic!("expected one additive billboard, got {other:?}"),
        }
    }

    fn step(e: &mut WaterballEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        })
    }

    #[test]
    fn column_holds_over_the_caster_climbing_and_cycling_its_frames() {
        let mut e = WaterballEffect::new([10.0, 0.0, 20.0]);
        let (start, first) = draw(&e);
        assert_eq!([start[0], start[2]], [10.0, 20.0], "sits on the caster");
        assert_eq!(start[1], Y_OFFSET);

        step(&mut e, 6.0);
        let (_, second) = draw(&e);
        assert_ne!(first, second, "cycles a texture every 5 frames");

        step(&mut e, 244.0);
        let (top, _) = draw(&e);
        assert!(
            (start[1] - top[1] - RISE).abs() < 0.5,
            "climbs {RISE} over its life: {} -> {}",
            start[1],
            top[1]
        );
        assert_eq!(step(&mut e, 1.0), EffectStatus::Dead);
    }
}
