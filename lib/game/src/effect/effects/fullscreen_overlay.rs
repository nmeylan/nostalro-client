//! Status-overlay family (`EF_BLIND` / `EF_POISON` / `EF_DEVIL*` /
//! `EF_BLEEDING` / `EF_CRYSTALBLUE`) — the original game shows these as
//! camera-locked screen overlays.
//!
//! Geometry per effect (all camera-independent, depth-disabled
//! [`EffectPrimitiveDraw::ScreenQuad`]s built in NDC clip space):
//!
//!   * Blind / Devil — a centred vignette. The original (blind render) draws
//!     four mirrored quads of `fullb.tga` whose transparent texture corner
//!     meets at the master's screen centre, so the middle stays clear and the
//!     four screen corners darken. We reproduce the four quads directly.
//!   * Poison / CrystalBlue — a full-viewport tint wash (the poison render draws
//!     a grid of the texture across the screen; one stretched quad reads the
//!     same).
//!   * Bleeding — the same faint red wash plus three big `lens_r.bmp` claw
//!     slashes across the screen centre, running top-right to bottom-left
//!     (the original plays these as a screen-space slash sweep).
//!
//! Per-frame opacity curves:
//!   * Blind / Devil — alpha `+1`/frame, clamp 255 (slow fade-in).
//!   * DevilRed — alpha `+3`/frame (fast fade-in).
//!   * Poison / CrystalBlue — alpha `+1`/frame to 255, then hold.
//!   * Bleeding — single pulse: ramp in over the first 10 frames, hold, then
//!     fade out after frame 65.
//!
//! Blend: Blind's darkening quads are Alpha;
//! the Poison colour washes are Additive (the reference captures are on black,
//! so the additive tint reads as the coloured wash). The bleeding claws glow
//! additively over the wash.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// Textures preloaded at boot (see `effect_texture_paths`).
pub const TEXTURES: &[&str] = &["fullb.tga", "poison_f.bmp", "white02.bmp", "lens_r.bmp"];

/// Persistent washes (Blind/Poison/Devil/CrystalBlue) are status-driven and
/// have no fixed lifetime; the status system removes
/// them. We mark them persistent (the viewer clamps this to 5 s).
pub const PERSISTENT_DURATION_MS: u32 = 99990;
/// Bleeding is a one-shot slash pulse, not a persistent wash.
pub const PULSE_DURATION_MS: u32 = 1500;

/// Bleeding pulse boundaries, in frames.
const PULSE_RAMP_FRAMES: f32 = 10.0;
const PULSE_FADE_START_FRAME: f32 = 65.0;

/// Slash tilt off vertical, leaning so the top edge sits to the right — the
/// claw then runs top-right to bottom-left.
const SLASH_ANGLE_DEG: f32 = 15.0;
const SLASH_COUNT: usize = 3;
const SLASH_MAX_ALPHA: f32 = 1.0;
/// Slash dimensions / spacing as a fraction of screen height.
const SLASH_LENGTH_FRAC: f32 = 0.6;
const SLASH_WIDTH_FRAC: f32 = 0.06;
const SLASH_SPACING_FRAC: f32 = 0.13;
const SLASH_STAGGER_FRAMES: f32 = 4.0;
const SLASH_GROW_FRAMES: f32 = 6.0;
const SLASH_FADE_FRAMES: f32 = 15.0;

#[derive(Clone, Copy, PartialEq)]
pub enum OverlayShape {
    /// Four mirrored quads with the transparent texture corner at the centre.
    Vignette,
    /// One quad stretched across the whole viewport.
    Wash,
}

#[derive(Clone, Copy)]
pub struct FullscreenOverlayParams {
    /// GRF texture (bare name → `data/texture/effect/`).
    pub texture: &'static str,
    /// RGB tint multiplied with the texture; `[r, g, b]` in 0..1.
    pub tint: [f32; 3],
    pub blend: BlendKind,
    pub shape: OverlayShape,
    /// Opacity gained per frame during fade-in (0..1 units).
    pub ramp_per_frame: f32,
    /// Opacity clamp.
    pub max_alpha: f32,
    /// Bleeding: ramp in, hold, then fade out — a single pulse.
    pub pulse: bool,
    /// Bleeding: draw the three claw slashes on top of the wash.
    pub slashes: bool,
    pub duration_ms: u32,
}

impl FullscreenOverlayParams {
    pub const fn total_duration_ms(&self) -> u32 {
        self.duration_ms
    }
}

/// `BLIND(0)` — near-black `fullb.tga` vignette centred on screen.
pub const BLIND: FullscreenOverlayParams = FullscreenOverlayParams {
    texture: "fullb.tga",
    tint: [10.0 / 255.0, 10.0 / 255.0, 10.0 / 255.0],
    blend: BlendKind::Alpha,
    shape: OverlayShape::Vignette,
    ramp_per_frame: 1.0 / 255.0,
    max_alpha: 1.0,
    pulse: false,
    slashes: false,
    duration_ms: PERSISTENT_DURATION_MS,
};

/// `BLIND(1..10)` — Devil1-10. Same vignette, slightly lighter grey tint.
pub const DEVIL: FullscreenOverlayParams = FullscreenOverlayParams {
    texture: "fullb.tga",
    tint: [30.0 / 255.0, 30.0 / 255.0, 30.0 / 255.0],
    blend: BlendKind::Alpha,
    shape: OverlayShape::Vignette,
    ramp_per_frame: 1.0 / 255.0,
    max_alpha: 1.0,
    pulse: false,
    slashes: false,
    duration_ms: PERSISTENT_DURATION_MS,
};

/// `BLIND(0, nColor=2)` — DevilRed. Red tint, faster ramp.
pub const DEVIL_RED: FullscreenOverlayParams = FullscreenOverlayParams {
    texture: "fullb.tga",
    tint: [1.0, 0.0, 0.0],
    blend: BlendKind::Alpha,
    shape: OverlayShape::Vignette,
    ramp_per_frame: 3.0 / 255.0,
    max_alpha: 1.0,
    pulse: false,
    slashes: false,
    duration_ms: PERSISTENT_DURATION_MS,
};

/// Poison — green-ish `poison_f.bmp` wash (additive on black).
pub const POISON: FullscreenOverlayParams = FullscreenOverlayParams {
    texture: "poison_f.bmp",
    tint: [1.0, 1.0, 1.0],
    blend: BlendKind::Additive,
    shape: OverlayShape::Wash,
    ramp_per_frame: 1.0 / 255.0,
    max_alpha: 1.0,
    pulse: false,
    slashes: false,
    duration_ms: PERSISTENT_DURATION_MS,
};

/// Bleeding. Faint pulsing red wash + three claw slashes.
pub const BLEEDING: FullscreenOverlayParams = FullscreenOverlayParams {
    texture: "white02.bmp",
    tint: [1.0, 0.0, 0.0],
    blend: BlendKind::Additive,
    shape: OverlayShape::Wash,
    ramp_per_frame: 10.0 / 255.0,
    max_alpha: 45.0 / 255.0,
    pulse: true,
    slashes: true,
    duration_ms: PULSE_DURATION_MS,
};

/// CrystalBlue. Constant blue wash.
pub const CRYSTAL_BLUE: FullscreenOverlayParams = FullscreenOverlayParams {
    texture: "white02.bmp",
    tint: [0.0, 0.0, 1.0],
    blend: BlendKind::Additive,
    shape: OverlayShape::Wash,
    ramp_per_frame: 1.0 / 255.0,
    max_alpha: 1.0,
    pulse: false,
    slashes: false,
    duration_ms: PERSISTENT_DURATION_MS,
};

/// Bleeding claw tint (`lens_r.bmp` is already red-orange; nudge toward blood).
const SLASH_TINT: [f32; 3] = [1.0, 0.15, 0.15];

pub struct FullscreenOverlayEffect {
    params: FullscreenOverlayParams,
    age_frames: f32,
    process: f32,
    alpha: f32,
    total_frames: f32,
}

impl FullscreenOverlayEffect {
    pub fn new(_world_pos: [f32; 3], params: FullscreenOverlayParams) -> Self {
        Self {
            params,
            age_frames: 0.0,
            process: 0.0,
            alpha: 0.0,
            total_frames: params.duration_ms as f32 * FRAMES_PER_SECOND / 1000.0,
        }
    }

    fn step_one_frame(&mut self) {
        self.process += 1.0;
        if self.params.pulse {
            if self.process < PULSE_RAMP_FRAMES {
                self.alpha = (self.alpha + self.params.ramp_per_frame).min(self.params.max_alpha);
            } else if self.process > PULSE_FADE_START_FRAME {
                self.alpha = (self.alpha - self.params.ramp_per_frame).max(0.0);
            }
        } else {
            self.alpha = (self.alpha + self.params.ramp_per_frame).min(self.params.max_alpha);
        }
    }

    /// Tint+opacity colour for the wash / vignette.
    fn body_color(&self) -> [f32; 4] {
        [self.params.tint[0], self.params.tint[1], self.params.tint[2], self.alpha]
    }
}

/// Four mirrored quads forming a centred vignette: the transparent texture
/// corner (`uv (0,1)`) lands at the screen centre, the opaque corner
/// (`uv (1,0)`) at each screen corner.
fn vignette_quads() -> [([[f32; 2]; 4], [[f32; 2]; 4]); 4] {
    const UVS: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    const QUADRANTS: [[f32; 2]; 4] = [[1.0, 1.0], [-1.0, 1.0], [1.0, -1.0], [-1.0, -1.0]];
    QUADRANTS.map(|[sx, sy]| {
        let corners = [[0.0, 0.0], [sx, 0.0], [sx, sy], [0.0, sy]];
        (corners, UVS)
    })
}

/// Per-slash claw geometry + opacity for frame `process`. Returns `None`
/// before the slash's staggered start. `screen_w`/`screen_h` give the aspect
/// so the claw keeps its shape on non-square viewports.
fn slash_quad(
    i: usize,
    process: f32,
    screen_w: f32,
    screen_h: f32,
) -> Option<([[f32; 2]; 4], [[f32; 2]; 4], f32)> {
    let start = i as f32 * SLASH_STAGGER_FRAMES;
    let local = process - start;
    if local < 0.0 {
        return None;
    }
    let ramp = (local / SLASH_GROW_FRAMES).min(1.0);
    let fade = if process > PULSE_FADE_START_FRAME {
        (1.0 - (process - PULSE_FADE_START_FRAME) / SLASH_FADE_FRAMES).max(0.0)
    } else {
        1.0
    };
    let alpha = ramp * fade * SLASH_MAX_ALPHA;
    if alpha <= 0.0 {
        return None;
    }

    let theta = SLASH_ANGLE_DEG.to_radians();
    // Along the claw length (top end), and across its width.
    let up = [theta.sin(), theta.cos()];
    let across = [theta.cos(), -theta.sin()];

    let half_len = 0.5 * SLASH_LENGTH_FRAC * screen_h * (0.4 + 0.6 * ramp);
    let half_wid = 0.5 * SLASH_WIDTH_FRAC * screen_h;
    let spacing = SLASH_SPACING_FRAC * screen_h;
    let center_off = (i as f32 - (SLASH_COUNT as f32 - 1.0) / 2.0) * spacing;
    let cx = center_off * across[0];
    let cy = center_off * across[1];

    let to_ndc = |px: f32, py: f32| [px / (screen_w * 0.5), py / (screen_h * 0.5)];
    let corner = |len_sign: f32, wid_sign: f32| {
        to_ndc(
            cx + len_sign * half_len * up[0] + wid_sign * half_wid * across[0],
            cy + len_sign * half_len * up[1] + wid_sign * half_wid * across[1],
        )
    };
    // top-left, top-right, bottom-right, bottom-left → lens_r upright.
    let corners = [
        corner(1.0, -1.0),
        corner(1.0, 1.0),
        corner(-1.0, 1.0),
        corner(-1.0, -1.0),
    ];
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    Some((corners, uvs, alpha))
}

impl Effect for FullscreenOverlayEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let before = self.age_frames;
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let steps = (self.age_frames.floor() - before.floor()).max(0.0) as i32;
        for _ in 0..steps {
            self.step_one_frame();
        }
        if self.age_frames >= self.total_frames {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        if self.alpha > 0.0 {
            match self.params.shape {
                OverlayShape::Vignette => {
                    for (corners, uvs) in vignette_quads() {
                        out.push(EffectPrimitiveDraw::ScreenQuad {
                            texture: self.params.texture,
                            color: self.body_color(),
                            blend: self.params.blend,
                            corners,
                            uvs,
                        });
                    }
                }
                OverlayShape::Wash => {
                    out.push(EffectPrimitiveDraw::ScreenQuad {
                        texture: self.params.texture,
                        color: self.body_color(),
                        blend: self.params.blend,
                        corners: [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]],
                        uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
                    });
                }
            }
        }

        if self.params.slashes {
            for i in 0..SLASH_COUNT {
                if let Some((corners, uvs, alpha)) =
                    slash_quad(i, self.process, ctx.screen_w, ctx.screen_h)
                {
                    out.push(EffectPrimitiveDraw::ScreenQuad {
                        texture: "lens_r.bmp",
                        color: [SLASH_TINT[0], SLASH_TINT[1], SLASH_TINT[2], alpha],
                        blend: BlendKind::Additive,
                        corners,
                        uvs,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step_frames(e: &mut FullscreenOverlayEffect, n: u32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    fn quads(e: &FullscreenOverlayEffect) -> Vec<([[f32; 2]; 4], [[f32; 2]; 4], &'static str, [f32; 4])> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::ScreenQuad { corners, uvs, texture, color, .. } => {
                    Some((*corners, *uvs, *texture, *color))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn blind_emits_centered_vignette() {
        let mut e = FullscreenOverlayEffect::new([0.0, 0.0, 0.0], BLIND);
        step_frames(&mut e, 5);
        let qs = quads(&e);
        assert_eq!(qs.len(), 4, "vignette is four mirrored quads");
        for (corners, uvs, texture, _) in &qs {
            assert_eq!(*texture, "fullb.tga");
            // Each quad's first vertex is the screen centre with the
            // transparent texture corner, so the middle stays clear.
            assert_eq!(corners[0], [0.0, 0.0]);
            assert_eq!(uvs[0], [0.0, 1.0]);
        }
    }

    #[test]
    fn persistent_wash_alpha_grows_monotonically() {
        let mut e = FullscreenOverlayEffect::new([0.0, 0.0, 0.0], CRYSTAL_BLUE);
        step_frames(&mut e, 10);
        let a10 = quads(&e)[0].3[3];
        step_frames(&mut e, 20);
        let a30 = quads(&e)[0].3[3];
        assert!(a30 > a10, "alpha should ramp up: {a10} -> {a30}");
    }

    #[test]
    fn bleeding_emits_wash_and_three_slashes() {
        let mut e = FullscreenOverlayEffect::new([0.0, 0.0, 0.0], BLEEDING);
        // Past every slash's staggered start, still in the hold.
        step_frames(&mut e, 30);
        let qs = quads(&e);
        let slashes: Vec<_> = qs.iter().filter(|(_, _, tex, _)| *tex == "lens_r.bmp").collect();
        assert_eq!(slashes.len(), SLASH_COUNT, "three claw slashes");
        assert!(qs.iter().any(|(_, _, tex, _)| *tex == "white02.bmp"), "red wash present");
        // Claw runs top-right (first vertex) to bottom-left (third vertex).
        let (corners, _, _, _) = slashes[0];
        assert!(corners[0][0] > corners[2][0] && corners[0][1] > corners[2][1],
            "top end is up-right of the bottom end: {corners:?}");
    }

    #[test]
    fn bleeding_pulses_up_then_down() {
        let mut e = FullscreenOverlayEffect::new([0.0, 0.0, 0.0], BLEEDING);
        step_frames(&mut e, 20);
        let peak = quads(&e).iter().find(|(_, _, t, _)| *t == "white02.bmp").unwrap().3[3];
        step_frames(&mut e, 60);
        let tail = quads(&e)
            .iter()
            .find(|(_, _, t, _)| *t == "white02.bmp")
            .map(|q| q.3[3])
            .unwrap_or(0.0);
        assert!(peak > 0.0);
        assert!(tail < peak, "bleeding wash should fade after the pulse: {peak} -> {tail}");
    }
}
