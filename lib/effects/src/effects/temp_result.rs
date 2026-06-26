//! `EF_TEMP_OK` / `EF_TEMP_FAIL` — item-refine result banners.
//!
//! The item-refine result banner: a single camera-facing quad anchored above
//! the caster (`success.bmp` / `failed.bmp`), alpha-blended, that fades in,
//! holds, then fades out across its 100-frame (~1.67 s at 60 fps) lifetime. No
//! UV scroll, no second layer — the starburst seen alongside the banner in the
//! original game is the caster's own action sprite, not part of this effect.
//!
//!
//!
//! The banner also breathes: the quad radius scales by
//! `1 + 0.05 * sin(phase°)` with the phase advancing 2° per frame, so it grows
//! ~5 % as it appears (sin climbing 0°→90° over the first ~45 frames) and
//! shrinks back as it leaves.
//!
//! The magenta field in `success.bmp` / `failed.bmp` is the colour key the
//! texture loader maps to zero alpha, so only the "Success!" / "Failed!"
//! lettering is drawn.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

#[derive(Clone, Copy)]
pub struct TempResultParams {
    pub texture: &'static str,
    /// World-space full width / height of the banner quad. Width is derived
    /// from the texture's native pixel aspect so the lettering isn't stretched.
    pub width: f32,
    pub height: f32,
    /// Offset above the caster's feet (native RO `-Y = up`, so negative floats
    /// the banner up over the head).
    pub y_offset: f32,
    pub fade_in_frames: f32,
    pub hold_frames: f32,
    pub fade_out_frames: f32,
    pub max_alpha: f32,
    /// Radius wobble amplitude as a fraction of size (`0.05`).
    pub wobble_amp: f32,
    /// Degrees the wobble phase advances per frame (2° per frame).
    pub wobble_speed_deg: f32,
}

// Banner sizes keep each texture's pixel aspect (success.bmp 76x28,
// failed.bmp 68x45) so the words stay legible above the caster. Widths are
// tuned so the banner stands ~6 world units tall — under a character sprite's
// height — and `y_offset` floats it clear of the head (native RO `-Y = up`).
// The original game anchors it above the head too but renders it far
// larger than the actor.
// 100-frame lifetime matches the original game's banner duration;
// alpha climbs to 220/255, holds, then eases out at the tail. Wobble values
// are the `0.05` amplitude / `2°`-per-frame speed.
pub const TEMP_OK: TempResultParams = TempResultParams {
    texture: "success.bmp",
    width: 9.0,
    height: 9.0 * 28.0 / 76.0,
    y_offset: -18.0,
    fade_in_frames: 22.0,
    hold_frames: 63.0,
    fade_out_frames: 15.0,
    max_alpha: 220.0 / 255.0,
    wobble_amp: 0.05,
    wobble_speed_deg: 2.0,
};

pub const TEMP_FAIL: TempResultParams = TempResultParams {
    texture: "failed.bmp",
    width: 9.0,
    height: 9.0 * 45.0 / 68.0,
    y_offset: -18.0,
    fade_in_frames: 22.0,
    hold_frames: 63.0,
    fade_out_frames: 15.0,
    max_alpha: 220.0 / 255.0,
    wobble_amp: 0.05,
    wobble_speed_deg: 2.0,
};

pub const TEXTURES: &[&str] = &[TEMP_OK.texture, TEMP_FAIL.texture];

pub struct TempResultEffect {
    params: TempResultParams,
    center: [f32; 3],
    process: f32,
}

impl TempResultEffect {
    pub fn new(anchor: [f32; 3], params: TempResultParams) -> Self {
        Self {
            params,
            center: anchor,
            process: 0.0,
        }
    }

    fn alpha(&self) -> f32 {
        let p = &self.params;
        if self.process < p.fade_in_frames {
            (self.process / p.fade_in_frames) * p.max_alpha
        } else if self.process < p.fade_in_frames + p.hold_frames {
            p.max_alpha
        } else {
            let t = (self.process - p.fade_in_frames - p.hold_frames) / p.fade_out_frames;
            (p.max_alpha * (1.0 - t)).max(0.0)
        }
    }

    fn life_frames(&self) -> f32 {
        self.params.fade_in_frames + self.params.hold_frames + self.params.fade_out_frames
    }

    /// Breathing scale: grows ~`wobble_amp` as the phase climbs 0°→90°, then
    /// shrinks back. Matches `1 + sin(phase) * 0.05`.
    fn scale(&self) -> f32 {
        1.0 + self.params.wobble_amp
            * (self.process * self.params.wobble_speed_deg)
                .to_radians()
                .sin()
    }
}

impl Effect for TempResultEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.process += ctx.delta * FRAMES_PER_SECOND;
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
        let s = self.scale();
        out.push(EffectPrimitiveDraw::Billboard {
            pos: [
                self.center[0],
                self.center[1] + self.params.y_offset,
                self.center[2],
            ],
            size: [self.params.width * s, self.params.height * s],
            uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            rotation: 0.0,
            texture: self.params.texture,
            color: [1.0, 1.0, 1.0, a],
            blend: BlendKind::Alpha,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(e: &mut TempResultEffect, frames: u32) -> EffectStatus {
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

    fn banner(e: &TempResultEffect) -> Option<([f32; 2], f32, &'static str)> {
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
            EffectPrimitiveDraw::Billboard {
                size,
                color,
                texture,
                blend: BlendKind::Alpha,
                ..
            } => (*size, color[3], *texture),
            _ => panic!("expected an alpha-blended Billboard"),
        })
    }

    #[test]
    fn fades_in_holds_then_out_and_dies() {
        let mut e = TempResultEffect::new([0.0; 3], TEMP_OK);
        tick(&mut e, 4);
        let (_, a_in, _) = banner(&e).expect("visible during fade-in");
        tick(
            &mut e,
            (TEMP_OK.fade_in_frames + TEMP_OK.hold_frames / 2.0) as u32,
        );
        let (_, a_hold, _) = banner(&e).expect("visible at hold");
        assert!(
            a_hold > a_in,
            "alpha rises into the hold: {a_in} -> {a_hold}"
        );
        assert!(a_hold <= TEMP_OK.max_alpha + 1e-3, "alpha is capped at max");
        assert_eq!(
            tick(&mut e, 200),
            EffectStatus::Dead,
            "self-terminates after its lifetime"
        );
    }

    #[test]
    fn breathes_grows_into_view_then_shrinks_out() {
        let mut e = TempResultEffect::new([0.0; 3], TEMP_OK);
        tick(&mut e, 2);
        let (early, ..) = banner(&e).expect("visible early");
        tick(&mut e, 43); // ~frame 45: sine phase at 90°, peak size
        let (peak, ..) = banner(&e).expect("visible at peak");
        tick(&mut e, 50); // ~frame 95: phase past 180°, size dips back below
        let (late, ..) = banner(&e).expect("visible at tail");
        assert!(
            peak[0] > early[0],
            "banner grows as it appears: {} -> {}",
            early[0],
            peak[0]
        );
        assert!(
            late[0] < peak[0],
            "banner shrinks as it leaves: {} -> {}",
            peak[0],
            late[0]
        );
    }

    #[test]
    fn ok_and_fail_differ_in_texture_and_aspect() {
        let mut ok = TempResultEffect::new([0.0; 3], TEMP_OK);
        let mut fail = TempResultEffect::new([0.0; 3], TEMP_FAIL);
        tick(&mut ok, 12);
        tick(&mut fail, 12);
        let (ok_size, _, ok_tex) = banner(&ok).expect("ok banner visible past fade-in");
        let (fail_size, _, fail_tex) = banner(&fail).expect("fail banner visible past fade-in");
        assert_eq!(ok_tex, "success.bmp");
        assert_eq!(fail_tex, "failed.bmp");
        let ok_aspect = ok_size[0] / ok_size[1];
        let fail_aspect = fail_size[0] / fail_size[1];
        assert!(
            ok_aspect > fail_aspect,
            "success banner is wider per height than failed"
        );
    }
}
