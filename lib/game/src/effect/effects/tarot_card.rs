//! `EF_TAROTCARD1`..`EF_TAROTCARD14` + `EF_NPCSLOWCAST` — `EffectTextureSet`
//! camera-facing billboards.
//!
//! The tarot cards (`Tarot{NN}.tga`) and the slow-cast clock
//! (`blast_mine##clock.bmp`) are both EffectTexture quads. The clock's branch
//! forces the same fade behaviour, so both share one alpha curve, also used by
//! the result banner (`temp_result.rs`): alpha rises to
//! 220/255, holds, then fades out, while a `1 + 0.05·sin` factor breathes the
//! quad. The quad always billboards toward the screen,
//! so these are camera-facing, not ground quads.
//!
//! Tarot cards use the tall 64×128 aspect; the slow-cast
//! clock is square. Everything else is per-texture only.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

#[derive(Clone, Copy)]
pub struct TarotCardParams {
    pub texture: &'static str,
    /// World-space full width / height of the billboard quad.
    pub width: f32,
    pub height: f32,
    /// Offset above the caster's feet (native RO `-Y = up`, so negative floats
    /// the card up over the head; lifts the card 30 above the feet).
    pub y_offset: f32,
    pub fade_in_frames: f32,
    pub hold_frames: f32,
    pub fade_out_frames: f32,
    pub max_alpha: f32,
    /// Radius wobble amplitude as a fraction of size (base `0.05`).
    pub wobble_amp: f32,
    /// Degrees the wobble phase advances per frame (base `2`).
    pub wobble_speed_deg: f32,
}

/// Texture basenames in `EffectId::Tarotcard{1..14}` order, followed by the
/// slow-cast clock. The factory indexes 0..=13 for the cards and 14 for the
/// clock; the renderer's texture loader prepends `data/texture/effect/`.
pub const TEXTURES: &[&str] = &[
    "tarot01.tga",
    "tarot02.tga",
    "tarot03.tga",
    "tarot04.tga",
    "tarot05.tga",
    "tarot06.tga",
    "tarot07.tga",
    "tarot08.tga",
    "tarot09.tga",
    "tarot10.tga",
    "tarot11.tga",
    "tarot12.tga",
    "tarot13.tga",
    "tarot14.tga",
    "blast_mine##clock.bmp",
];

const CLOCK_INDEX: usize = 14;

// Alpha curve: climbs +10/frame to 220/255 (~22 frames), holds
// until frame 200, then eases out at -5/frame (~44 frames). The
// 64×128 card aspect (height = 2× width) is the tall-card flag; wobble
// values are the `0.05` amplitude / `2°`-per-frame speed.
const CARD_WIDTH: f32 = 11.0;

/// Build params for tarot card `index` (0-based; `EffectId::Tarotcard1` → 0).
pub fn tarot_params(index: usize) -> TarotCardParams {
    TarotCardParams {
        texture: TEXTURES[index],
        width: CARD_WIDTH,
        height: CARD_WIDTH * 2.0,
        y_offset: -14.0,
        fade_in_frames: 22.0,
        hold_frames: 178.0,
        fade_out_frames: 44.0,
        max_alpha: 220.0 / 255.0,
        wobble_amp: 0.05,
        wobble_speed_deg: 2.0,
    }
}

// Slow-cast clock: square, smaller base size, and a
// shorter hold to match its ~3 s display.
pub const NPC_SLOWCAST: TarotCardParams = TarotCardParams {
    texture: TEXTURES[CLOCK_INDEX],
    width: 12.0,
    height: 12.0,
    y_offset: -12.0,
    fade_in_frames: 22.0,
    hold_frames: 120.0,
    fade_out_frames: 44.0,
    max_alpha: 220.0 / 255.0,
    wobble_amp: 0.05,
    wobble_speed_deg: 2.0,
};

pub struct TarotCardEffect {
    params: TarotCardParams,
    center: [f32; 3],
    process: f32,
}

impl TarotCardEffect {
    pub fn new(anchor: [f32; 3], params: TarotCardParams) -> Self {
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
    /// shrinks back. Matches the original's `1 + sin(phase) * 0.05` breathe.
    fn scale(&self) -> f32 {
        1.0 + self.params.wobble_amp * (self.process * self.params.wobble_speed_deg).to_radians().sin()
    }
}

impl Effect for TarotCardEffect {
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
            pos: [self.center[0], self.center[1] + self.params.y_offset, self.center[2]],
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

    fn tick(e: &mut TarotCardEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None });
        }
        st
    }

    fn card(e: &TarotCardEffect) -> Option<([f32; 2], f32, &'static str)> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &EffectRenderCtx {
            camera: Default::default(),
            screen_w: 256.0,
            screen_h: 256.0,
            elapsed: 0.0,
        });
        l.primitives.first().map(|p| match p {
            EffectPrimitiveDraw::Billboard { size, color, texture, blend: BlendKind::Alpha, .. } => {
                (*size, color[3], *texture)
            }
            _ => panic!("expected an alpha-blended Billboard"),
        })
    }

    #[test]
    fn fades_in_holds_then_out_and_dies() {
        let mut e = TarotCardEffect::new([0.0; 3], tarot_params(0));
        tick(&mut e, 4);
        let (_, a_in, _) = card(&e).expect("visible during fade-in");
        tick(&mut e, 30);
        let (_, a_hold, _) = card(&e).expect("visible at hold");
        assert!(a_hold > a_in, "alpha rises into the hold: {a_in} -> {a_hold}");
        assert!(a_hold <= tarot_params(0).max_alpha + 1e-3, "alpha is capped at max");
        assert_eq!(tick(&mut e, 400), EffectStatus::Dead, "self-terminates after its lifetime");
    }

    #[test]
    fn breathes_grows_then_shrinks() {
        let mut e = TarotCardEffect::new([0.0; 3], tarot_params(0));
        tick(&mut e, 2);
        let (early, ..) = card(&e).expect("visible early");
        tick(&mut e, 43); // ~frame 45: sin phase at 90°, peak size
        let (peak, ..) = card(&e).expect("visible at peak");
        tick(&mut e, 50); // ~frame 95: phase past 180°, size dips back
        let (late, ..) = card(&e).expect("visible at tail");
        assert!(peak[0] > early[0], "card grows as it appears: {} -> {}", early[0], peak[0]);
        assert!(late[0] < peak[0], "card shrinks again: {} -> {}", peak[0], late[0]);
    }

    #[test]
    fn index_selects_texture_with_tall_aspect_clock_is_square() {
        let first = TarotCardEffect::new([0.0; 3], tarot_params(0));
        let last = TarotCardEffect::new([0.0; 3], tarot_params(13));
        let mut clock = TarotCardEffect::new([0.0; 3], NPC_SLOWCAST);
        let mut f = first;
        let mut l = last;
        tick(&mut f, 12);
        tick(&mut l, 12);
        tick(&mut clock, 12);
        let (fs, _, ft) = card(&f).expect("first card visible");
        let (_, _, lt) = card(&l).expect("last card visible");
        let (cs, _, ct) = card(&clock).expect("clock visible");
        assert_eq!(ft, "tarot01.tga");
        assert_eq!(lt, "tarot14.tga");
        assert_eq!(ct, "blast_mine##clock.bmp");
        assert!(fs[1] > fs[0], "tarot card is taller than wide");
        assert!((cs[0] - cs[1]).abs() < 1e-3, "clock is square");
    }
}
