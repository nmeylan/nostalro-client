//! `EF_BEGINASURA` family — Asura Strike cast glyphs + saint-casting rings.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{CameraView, Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::saint_casting::{
    SaintCastingConfig, SaintCastingEffect, TOTAL_DURATION_MS as SAINT_TOTAL_DURATION_MS,
};

const FRAMES_PER_SECOND: f32 = 60.0;

const NUM_LAYERS: usize = 10;
const LAYER_STEP: f32 = 3.0;
const LAYER_ALPHA_BASE: f32 = 50.0;
const LAYER_ALPHA_STEP: f32 = 20.0;
const RAMP_PER_FRAME: f32 = 13.0;
const FADE_PER_FRAME: f32 = 5.0;
const RAMP_FRAMES: f32 = 20.0;
const CORE_HOLD_FRAMES: f32 = 140.0;
const SETTLE_FRAMES: f32 = 50.0;
const SETTLE_PER_FRAME: f32 = 0.1;

const Y_OFFSET: f32 = -22.0;
const SIZE_SCALE: f32 = 0.47;
const SQRT2: f32 = std::f32::consts::SQRT_2;

/// `hanmoon1..7` non-sequential mapping: 5/6/7 → 7/5/6 (matches original game ordering).
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
    "soul_s.tga",
    "soul_o.tga",
    "soul_u.tga",
    "soul_l.tga",
    "soul_i.tga",
    "soul_n.tga",
    "soul_k.tga",
];

const SOUL_LINK_DISTANCE: f32 = 5.0;
const SOUL_LINK_GLYPHS: [(&str, f32, f32); 8] = [
    ("soul_s.tga", -22.0, 1.0),
    ("soul_o.tga", -16.0, 21.0),
    ("soul_u.tga", -10.0, 41.0),
    ("soul_l.tga", -4.0, 61.0),
    ("soul_l.tga", 4.0, 81.0),
    ("soul_i.tga", 10.0, 101.0),
    ("soul_n.tga", 16.0, 121.0),
    ("soul_k.tga", 22.0, 141.0),
];

const PHRASE_DISTANCE: f32 = 18.0;
const PHRASE_X: [f32; 6] = [-30.0, -18.0, -6.0, 6.0, 18.0, 30.0];
const CHAMPION_DISTANCE: f32 = 24.0;
const CHAMPION_X: [f32; 6] = [-40.0, -24.0, -8.0, 8.0, 24.0, 40.0];
const PHRASE_TINT_BLACK: [f32; 3] = [0.0, 0.0, 0.0];
const PHRASE_TINT_DARK: [f32; 3] = [10.0 / 255.0, 10.0 / 255.0, 10.0 / 255.0];
const GROUP2_DELAY: f32 = 20.0;
const ELEMENTAL_DISTANCE: f32 = 18.0;

const GLYPH_LIFE_FRAMES: f32 =
    CORE_HOLD_FRAMES + (LAYER_ALPHA_BASE + NUM_LAYERS as f32 * LAYER_ALPHA_STEP) / FADE_PER_FRAME;
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
    process: f32,
    distance: f32,
    layer_alpha: [f32; NUM_LAYERS + 1],
    tint: [f32; 3],
}

impl Glyph {
    fn new(texture: &'static str, x_offset: f32, distance: f32, start_delay: f32) -> Self {
        Self {
            texture,
            x_offset,
            process: -start_delay,
            distance,
            layer_alpha: [0.0; NUM_LAYERS + 1],
            tint: [1.0, 1.0, 1.0],
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
                self.layer_alpha[i] =
                    (self.layer_alpha[i] + RAMP_PER_FRAME * frames).min(Self::layer_cap(i));
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

    fn collect_draws(&self, center: [f32; 3], right: [f32; 3], out: &mut EffectDrawList) {
        let pos = [
            center[0] + right[0] * self.x_offset,
            center[1] + Y_OFFSET + right[1] * self.x_offset,
            center[2] + right[2] * self.x_offset,
        ];
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
                color: [self.tint[0], self.tint[1], self.tint[2], alpha / 255.0],
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

const RING_CONFIG: SaintCastingConfig = SaintCastingConfig {
    texture: "ring_white.tga",
    pass_textures: None,
    max_heights: [25.0, 23.75, 22.5, 21.25],
    color_rgb: [1.0, 1.0, 1.0],
    blend: BlendKind::Additive,
    refill_per_frame: 10.0,
    reset_rise_deg: 74.0,
};

impl BeginAsuraEffect {
    pub fn elemental(anchor: [f32; 3], index: usize) -> Self {
        Self {
            center: anchor,
            rings: None,
            glyphs: vec![Glyph::new(HANMOON[index], 0.0, ELEMENTAL_DISTANCE, 0.0)],
        }
    }

    pub fn base(anchor: [f32; 3]) -> Self {
        Self::phrase(
            anchor,
            &PHRASE,
            PHRASE_DISTANCE,
            &PHRASE_X,
            PHRASE_TINT_BLACK,
        )
    }

    pub fn champion(anchor: [f32; 3]) -> Self {
        Self::phrase(
            anchor,
            &PHRASE_CHAMPION,
            CHAMPION_DISTANCE,
            &CHAMPION_X,
            PHRASE_TINT_DARK,
        )
    }

    pub fn soul_link(anchor: [f32; 3]) -> Self {
        let glyphs = SOUL_LINK_GLYPHS
            .iter()
            .map(|(tex, x, delay)| Glyph::new(tex, *x, SOUL_LINK_DISTANCE, *delay))
            .collect();
        Self {
            center: anchor,
            rings: None,
            glyphs,
        }
    }

    fn phrase(
        anchor: [f32; 3],
        textures: &[&'static str; 6],
        distance: f32,
        xs: &[f32; 6],
        tint: [f32; 3],
    ) -> Self {
        let glyphs = (0..6)
            .map(|k| {
                let delay = if k < 3 { 0.0 } else { GROUP2_DELAY };
                let mut g = Glyph::new(textures[k], xs[k], distance, delay);
                g.tint = tint;
                g
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
        let right = screen_right(&ctx.camera);
        for g in &self.glyphs {
            g.collect_draws(self.center, right, out);
        }
    }
}

fn screen_right(camera: &CameraView) -> [f32; 3] {
    let f = [
        camera.target[0] - camera.eye[0],
        camera.target[1] - camera.eye[1],
        camera.target[2] - camera.eye[2],
    ];
    let u = camera.up;
    let r = [
        f[1] * u[2] - f[2] * u[1],
        f[2] * u[0] - f[0] * u[2],
        f[0] * u[1] - f[1] * u[0],
    ];
    let len = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
    if len < 1e-4 {
        [1.0, 0.0, 0.0]
    } else {
        [r[0] / len, r[1] / len, r[2] / len]
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
                caster_yaw: None,
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
        let early = billboards(&e)
            .iter()
            .map(|(_, a)| a)
            .cloned()
            .fold(0.0, f32::max);
        tick(&mut e, 12); // ~frame 17, near the top of the ramp
        let peak = billboards(&e)
            .iter()
            .map(|(_, a)| a)
            .cloned()
            .fold(0.0, f32::max);
        assert!(
            peak > early,
            "alpha rises through the ramp: {early} -> {peak}"
        );
        assert_eq!(tick(&mut e, 400), EffectStatus::Dead, "self-terminates");
        assert!(billboards(&e).is_empty(), "nothing left to draw once dead");
    }

    #[test]
    fn base_emits_rings_and_six_phrase_glyphs() {
        let mut base = BeginAsuraEffect::base([0.0; 3]);
        let mut champ = BeginAsuraEffect::champion([0.0; 3]);

        tick(&mut base, 18);
        let mut l = EffectDrawList::new();
        base.collect_draws(&mut l, &render_ctx());
        let frustums = l
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
            .count();
        assert_eq!(frustums, 8, "two saint-casting passes × 4 emitters");

        tick(&mut base, 28);
        tick(&mut champ, 30);

        let textures: std::collections::HashSet<_> =
            billboards(&base).into_iter().map(|(t, _)| t).collect();
        assert_eq!(textures.len(), 6, "all six phrase characters are present");
        assert!(textures.contains("asura1.tga") && textures.contains("asura6.tga"));

        let champ_textures: std::collections::HashSet<_> =
            billboards(&champ).into_iter().map(|(t, _)| t).collect();
        assert!(
            champ_textures.contains("asura11.tga"),
            "champion uses the asura1x set"
        );
    }

    #[test]
    fn group_two_lags_behind_group_one() {
        let mut e = BeginAsuraEffect::base([0.0; 3]);
        tick(&mut e, 5);
        let visible: std::collections::HashSet<_> =
            billboards(&e).into_iter().map(|(t, _)| t).collect();
        assert!(visible.contains("asura1.tga"));
        assert!(!visible.contains("asura6.tga"));
    }
}
