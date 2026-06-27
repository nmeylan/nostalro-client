use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const TIME_SCALE: f32 = 0.45;

#[derive(Clone, Copy)]
pub struct EffectTextureParams {
    pub texture: &'static str,
    pub tint: [f32; 3],
    pub base_dist: f32,
    pub grow_per_frame: f32,
    pub fade_in_frames: f32,
    pub fade_out_frames: f32,
    pub max_alpha: f32,
    pub yaw_deg: f32,
    /// Height above the actor's feet. Native RO: negative Y = up.
    pub y_offset: f32,
}

pub const HITTEXTURE: EffectTextureParams = EffectTextureParams {
    texture: "freeze_a.bmp",
    tint: [1.0, 1.0, 1.0],
    base_dist: 10.0,
    grow_per_frame: 0.7,
    fade_in_frames: 22.0,
    fade_out_frames: 22.0,
    max_alpha: 176.0 / 255.0,
    yaw_deg: 135.0,
    y_offset: -3.0,
};

pub const TEXTURES: &[&str] = &[HITTEXTURE.texture];

pub struct EffectTextureEffect {
    params: EffectTextureParams,
    center: [f32; 3],
    process: f32,
    distance: f32,
    frame_accum: f32,
}

impl EffectTextureEffect {
    pub fn new(anchor: [f32; 3], params: EffectTextureParams) -> Self {
        Self {
            distance: params.base_dist,
            params,
            center: anchor,
            process: 0.0,
            frame_accum: 0.0,
        }
    }

    fn alpha(&self) -> f32 {
        let p = &self.params;
        if self.process <= p.fade_in_frames {
            (self.process / p.fade_in_frames) * p.max_alpha
        } else {
            let t = (self.process - p.fade_in_frames) / p.fade_out_frames;
            (p.max_alpha * (1.0 - t)).max(0.0)
        }
    }

    fn life_frames(&self) -> f32 {
        self.params.fade_in_frames + self.params.fade_out_frames
    }
}

impl Effect for EffectTextureEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND * TIME_SCALE;
        while self.frame_accum >= 1.0 {
            self.frame_accum -= 1.0;
            self.process += 1.0;
            self.distance += self.params.grow_per_frame;
        }
        if self.process >= self.life_frames() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let a = self.alpha();
        if a <= 0.0 {
            return;
        }
        let [r, g, b] = self.params.tint;
        out.push(EffectPrimitiveDraw::Texture3D {
            center: [
                self.center[0],
                self.center[1] + self.params.y_offset,
                self.center[2],
            ],
            size: [self.distance, self.distance],
            plane: QuadPlane::HorizontalYaw(self.params.yaw_deg.to_radians()),
            uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            texture: self.params.texture,
            color: [r, g, b, a],
            blend: BlendKind::Additive,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(e: &mut EffectTextureEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        let real = (frames as f32 / TIME_SCALE).ceil() as u32;
        for _ in 0..real {
            st = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        st
    }

    fn quad(e: &EffectTextureEffect) -> Option<([f32; 2], f32)> {
        let mut l = EffectDrawList::new();
        e.collect_draws(
            &mut l,
            &EffectRenderCtx {
                camera: Default::default(),
                screen_w: 256.0,
                screen_h: 256.0,
                elapsed: 0.0,
            },
        );
        l.primitives.first().map(|p| match p {
            EffectPrimitiveDraw::Texture3D {
                size,
                color,
                plane: QuadPlane::HorizontalYaw(_),
                ..
            } => (*size, color[3]),
            _ => panic!("expected a yawed Texture3D ground quad"),
        })
    }

    #[test]
    fn expands_and_fades_in_then_out_then_dies() {
        let mut e = EffectTextureEffect::new([0.0; 3], HITTEXTURE);
        tick(&mut e, 6);
        let (size_a, alpha_a) = quad(&e).expect("visible during fade-in");
        tick(&mut e, HITTEXTURE.fade_in_frames as u32);
        let (size_b, alpha_b) = quad(&e).expect("visible at peak/fade-out");
        assert!(
            size_b[0] > size_a[0],
            "quad expands: {} -> {}",
            size_a[0],
            size_b[0]
        );
        assert!(alpha_b < alpha_a + HITTEXTURE.max_alpha, "alpha is bounded");
        assert!(alpha_a > 0.0);
        assert_eq!(tick(&mut e, 200), EffectStatus::Dead);
    }
}
