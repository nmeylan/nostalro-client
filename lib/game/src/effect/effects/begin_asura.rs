//! `EF_BEGINASURA` / `EF_BEGINASURA1`..`7` / `EF_BEGINASURA11` — the
//! Asura Strike cast displays glowing Chinese-character glyphs floating above
//! the caster, not a ground ring.
//!
//! Each glyph is a screen-facing billboard (the same camera-facing family as
//! the tarot card). It ramps 10 concentric texture layers up in alpha over
//! ~20 frames, holds the bright core, then fades — the layered scaling is
//! what gives each glyph its soft glow. Each layer is laid out as a
//! screen-facing quad whose half-diagonal is `distance + (10 - i)·3`. The
//! bright small core renders first, with larger dimmer halo layers blended
//! over it.
//!
//! - The base cast (`EF_BEGINASURA`) spells **阿修羅覇凰拳** with `asura1..6`
//!   spread left-to-right, and `EF_BEGINASURA11` (Champion) uses the larger
//!   `asura11..16` set. Both also launch two `SAINTCASTING` white starburst
//!   rings (`ring_white.tga`, F1=2 → `max_height` `25..22`), reused here via
//!   [`super::saint_casting`].
//! - The elemental variants (`EF_BEGINASURA1..7`) show a single element glyph
//!   (`hanmoon1..7`: 地 風 水 火 暗 聖 念).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::effects::saint_casting::{
    SaintCastingConfig, SaintCastingEffect, TOTAL_DURATION_MS as SAINT_TOTAL_DURATION_MS,
};

const FRAMES_PER_SECOND: f32 = 60.0;

const NUM_LAYERS: usize = 10;
/// Layer `i`'s quad half-diagonal is `distance + (10-i)·3`.
const LAYER_STEP: f32 = 3.0;
/// Per-layer alpha ceiling `50 + i·20` (0..255), brightest at the core (i=10).
const LAYER_ALPHA_BASE: f32 = 50.0;
const LAYER_ALPHA_STEP: f32 = 20.0;
/// Alpha rates: +13/frame ramp for ~20 frames, then -5/frame fade.
const RAMP_PER_FRAME: f32 = 13.0;
const FADE_PER_FRAME: f32 = 5.0;
const RAMP_FRAMES: f32 = 20.0;
/// The bright core layer (i=10) holds until `process > 140`.
const CORE_HOLD_FRAMES: f32 = 140.0;
/// `distance -= 0.1` per frame while `process < 50` — a slight inward settle.
const SETTLE_FRAMES: f32 = 50.0;
const SETTLE_PER_FRAME: f32 = 0.1;

/// World height above the caster's feet (native `-Y = up`; the quad floats
/// up about 30 units).
const Y_OFFSET: f32 = -22.0;
/// Quad full width = half-diagonal · √2, scaled into our world units. Tuned so
/// the bright core layer (`distance·√2·SIZE_SCALE`) is about one character wide
/// — i.e. the phrase's 12-unit character spacing — so the six base glyphs read
/// as a row instead of a white blur.
const SIZE_SCALE: f32 = 0.47;
const SQRT2: f32 = std::f32::consts::SQRT_2;

/// `hanmoon1..7` in element order — indexed by the elemental variant. The
/// non-sequential 5/6/7 → 7/5/6 mapping matches the original game's ordering.
const HANMOON: [&str; 7] = [
    "hanmoon1.tga", // 地 earth   (BEGINASURA1)
    "hanmoon2.tga", // 風 wind    (BEGINASURA2)
    "hanmoon3.tga", // 水 water   (BEGINASURA3)
    "hanmoon4.tga", // 火 fire    (BEGINASURA4)
    "hanmoon7.tga", // 念 ghost   (BEGINASURA5)
    "hanmoon5.tga", // 暗 shadow  (BEGINASURA6)
    "hanmoon6.tga", // 聖 holy    (BEGINASURA7)
];

const PHRASE: [&str; 6] = [
    "asura1.tga",
    "asura2.tga",
    "asura3.tga",
    "asura4.tga",
    "asura5.tga",
    "asura6.tga",
];
const PHRASE_CHAMPION: [&str; 6] = [
    "asura11.tga",
    "asura12.tga",
    "asura13.tga",
    "asura14.tga",
    "asura15.tga",
    "asura16.tga",
];

/// Every GRF texture this effect can reference, for renderer preload at boot.
pub const TEXTURES: &[&str] = &[
    "asura1.tga",
    "asura2.tga",
    "asura3.tga",
    "asura4.tga",
    "asura5.tga",
    "asura6.tga",
    "asura11.tga",
    "asura12.tga",
    "asura13.tga",
    "asura14.tga",
    "asura15.tga",
    "asura16.tga",
    "hanmoon1.tga",
    "hanmoon2.tga",
    "hanmoon3.tga",
    "hanmoon4.tga",
    "hanmoon5.tga",
    "hanmoon6.tga",
    "hanmoon7.tga",
    "ring_white.tga",
];

const PHRASE_DISTANCE: f32 = 18.0;
const PHRASE_X: [f32; 6] = [-30.0, -18.0, -6.0, 6.0, 18.0, 30.0];
const CHAMPION_DISTANCE: f32 = 24.0;
const CHAMPION_X: [f32; 6] = [-40.0, -24.0, -8.0, 8.0, 24.0, 40.0];
/// First three characters appear together; the last three follow a group later
/// (spawned at frame 1 then frame 21).
const GROUP2_DELAY: f32 = 20.0;
const ELEMENTAL_DISTANCE: f32 = 18.0;

/// Wall-clock lifetime of one glyph: ramp + core hold + core fade-out.
const GLYPH_LIFE_FRAMES: f32 =
    CORE_HOLD_FRAMES + (LAYER_ALPHA_BASE + NUM_LAYERS as f32 * LAYER_ALPHA_STEP) / FADE_PER_FRAME;
/// The base/champion phrase outlasts a lone glyph by the group-2 stagger; take
/// the longer of that and the saint-ring lifetime.
const PHRASE_LIFE_MS: u32 =
    ((GLYPH_LIFE_FRAMES + GROUP2_DELAY) / FRAMES_PER_SECOND * 1000.0) as u32;
pub const TOTAL_DURATION_MS: u32 = if PHRASE_LIFE_MS > SAINT_TOTAL_DURATION_MS {
    PHRASE_LIFE_MS
} else {
    SAINT_TOTAL_DURATION_MS
};

struct Glyph {
    texture: &'static str,
    x_offset: f32,
    /// Frames; starts negative when the glyph is delayed.
    process: f32,
    distance: f32,
    /// Per-layer alpha (0..255), index `1..=NUM_LAYERS`; `[0]` unused.
    layer_alpha: [f32; NUM_LAYERS + 1],
}

impl Glyph {
    fn new(texture: &'static str, x_offset: f32, distance: f32, start_delay: f32) -> Self {
        Self {
            texture,
            x_offset,
            process: -start_delay,
            distance,
            layer_alpha: [0.0; NUM_LAYERS + 1],
        }
    }

    fn layer_cap(i: usize) -> f32 {
        LAYER_ALPHA_BASE + i as f32 * LAYER_ALPHA_STEP
    }

    fn update(&mut self, frames: f32) {
        self.process += frames;
        if self.process <= 0.0 {
            return;
        }
        if self.process < RAMP_FRAMES {
            for i in 1..=NUM_LAYERS {
                self.layer_alpha[i] = (self.layer_alpha[i] + RAMP_PER_FRAME * frames).min(Self::layer_cap(i));
            }
        } else {
            for i in 1..NUM_LAYERS {
                self.layer_alpha[i] = (self.layer_alpha[i] - FADE_PER_FRAME * frames).max(0.0);
            }
            if self.process > CORE_HOLD_FRAMES {
                self.layer_alpha[NUM_LAYERS] =
                    (self.layer_alpha[NUM_LAYERS] - FADE_PER_FRAME * frames).max(0.0);
            }
        }
        if self.process < SETTLE_FRAMES {
            self.distance -= SETTLE_PER_FRAME * frames;
        }
    }

    fn is_done(&self) -> bool {
        self.process > RAMP_FRAMES && self.layer_alpha.iter().all(|a| *a <= 0.0)
    }

    fn collect_draws(&self, center: [f32; 3], out: &mut EffectDrawList) {
        let pos = [center[0] + self.x_offset, center[1] + Y_OFFSET, center[2]];
        // Draw order: the bright small core first, then the larger, dimmer
        // halo layers blended over it (walk `i = 10..1`).
        for i in (1..=NUM_LAYERS).rev() {
            let alpha = self.layer_alpha[i];
            if alpha <= 0.0 {
                continue;
            }
            let half_diag = self.distance + (NUM_LAYERS - i) as f32 * LAYER_STEP;
            let size = half_diag * SQRT2 * SIZE_SCALE;
            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: [size, size],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: 0.0,
                texture: self.texture,
                color: [1.0, 1.0, 1.0, alpha / 255.0],
                blend: BlendKind::Alpha,
            });
        }
    }
}

pub struct BeginAsuraEffect {
    center: [f32; 3],
    rings: Option<SaintCastingEffect>,
    glyphs: Vec<Glyph>,
}

/// `SAINTCASTING(.., "ring_white.tga", 2)` size table (F1=2 → asura).
const RING_CONFIG: SaintCastingConfig = SaintCastingConfig {
    texture: "ring_white.tga",
    max_heights: [25.0, 24.0, 23.0, 22.0],
};

impl BeginAsuraEffect {
    /// Single element glyph (`EF_BEGINASURA1..7`); `index` is 0-based.
    pub fn elemental(anchor: [f32; 3], index: usize) -> Self {
        Self {
            center: anchor,
            rings: None,
            glyphs: vec![Glyph::new(HANMOON[index], 0.0, ELEMENTAL_DISTANCE, 0.0)],
        }
    }

    /// Base cast `EF_BEGINASURA` — 阿修羅覇凰拳 plus the white saint rings.
    pub fn base(anchor: [f32; 3]) -> Self {
        Self::phrase(anchor, &PHRASE, PHRASE_DISTANCE, &PHRASE_X)
    }

    /// Champion cast `EF_BEGINASURA11` — larger glyphs (`asura11..16`).
    pub fn champion(anchor: [f32; 3]) -> Self {
        Self::phrase(anchor, &PHRASE_CHAMPION, CHAMPION_DISTANCE, &CHAMPION_X)
    }

    fn phrase(anchor: [f32; 3], textures: &[&'static str; 6], distance: f32, xs: &[f32; 6]) -> Self {
        let glyphs = (0..6)
            .map(|k| {
                // First half spawns immediately, the second half a group later.
                let delay = if k < 3 { 0.0 } else { GROUP2_DELAY };
                Glyph::new(textures[k], xs[k], distance, delay)
            })
            .collect();
        Self {
            center: anchor,
            rings: Some(SaintCastingEffect::new(anchor, RING_CONFIG)),
            glyphs,
        }
    }
}

impl Effect for BeginAsuraEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frames = ctx.delta * FRAMES_PER_SECOND;
        let mut alive = false;
        if let Some(rings) = &mut self.rings {
            if rings.update(ctx) == EffectStatus::Running {
                alive = true;
            }
        }
        for g in &mut self.glyphs {
            g.update(frames);
            if !g.is_done() {
                alive = true;
            }
        }
        if alive {
            EffectStatus::Running
        } else {
            EffectStatus::Dead
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        if let Some(rings) = &self.rings {
            rings.collect_draws(out, ctx);
        }
        for g in &self.glyphs {
            g.collect_draws(self.center, out);
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

    fn tick(e: &mut BeginAsuraEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
            });
        }
        st
    }

    fn billboards(e: &BeginAsuraEffect) -> Vec<(&'static str, f32)> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        l.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Billboard { texture, color, .. } => Some((*texture, color[3])),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn elemental_emits_mapped_glyph_texture() {
        let mut earth = BeginAsuraEffect::elemental([0.0; 3], 0);
        let mut ghost = BeginAsuraEffect::elemental([0.0; 3], 4);
        tick(&mut earth, 10);
        tick(&mut ghost, 10);
        assert!(billboards(&earth).iter().all(|(t, _)| *t == "hanmoon1.tga"));
        assert!(billboards(&ghost).iter().all(|(t, _)| *t == "hanmoon7.tga"));
    }

    #[test]
    fn glyph_ramps_holds_then_fades_and_dies() {
        let mut e = BeginAsuraEffect::elemental([0.0; 3], 0);
        tick(&mut e, 5);
        let early = billboards(&e).iter().map(|(_, a)| a).cloned().fold(0.0, f32::max);
        tick(&mut e, 12); // ~frame 17, near the top of the ramp
        let peak = billboards(&e).iter().map(|(_, a)| a).cloned().fold(0.0, f32::max);
        assert!(peak > early, "alpha rises through the ramp: {early} -> {peak}");
        assert_eq!(tick(&mut e, 400), EffectStatus::Dead, "self-terminates");
        assert!(billboards(&e).is_empty(), "nothing left to draw once dead");
    }

    #[test]
    fn base_emits_rings_and_six_phrase_glyphs() {
        let mut base = BeginAsuraEffect::base([0.0; 3]);
        let mut champ = BeginAsuraEffect::champion([0.0; 3]);

        // Early on, both saint-casting passes (4 emitters each) are alive.
        tick(&mut base, 2);
        let mut l = EffectDrawList::new();
        base.collect_draws(&mut l, &render_ctx());
        let frustums = l
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
            .count();
        assert_eq!(frustums, 8, "two saint-casting passes × 4 emitters");

        // Past the group-2 delay so all six characters are present.
        tick(&mut base, 28);
        tick(&mut champ, 30);

        let textures: std::collections::HashSet<_> =
            billboards(&base).into_iter().map(|(t, _)| t).collect();
        assert_eq!(textures.len(), 6, "all six phrase characters are present");
        assert!(textures.contains("asura1.tga") && textures.contains("asura6.tga"));

        let champ_textures: std::collections::HashSet<_> =
            billboards(&champ).into_iter().map(|(t, _)| t).collect();
        assert!(champ_textures.contains("asura11.tga"), "champion uses the asura1x set");
    }

    #[test]
    fn group_two_lags_behind_group_one() {
        let mut e = BeginAsuraEffect::base([0.0; 3]);
        tick(&mut e, 5);
        let visible: std::collections::HashSet<_> =
            billboards(&e).into_iter().map(|(t, _)| t).collect();
        // Group one (asura1..3) is already fading in; group two (asura4..6) is
        // still delayed and not yet drawn.
        assert!(visible.contains("asura1.tga"), "group one is visible early");
        assert!(!visible.contains("asura6.tga"), "group two has not started yet");
    }
}
