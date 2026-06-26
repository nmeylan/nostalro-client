//! Status-overlay family (`EF_BLIND` / `EF_POISON` / `EF_DEVIL*` /
//! `EF_BLEEDING` / `EF_CRYSTALBLUE`) — the original game shows these as
//! camera-locked screen overlays.
//!
//! Geometry per effect:
//!
//!   * Blind / Devil — a centred vignette built in **world space** around the
//!     master and billboarded toward the camera (the quads are built in the
//!     master-centred XZ plane, then stood up facing the view). The clear hole
//!     is a square of half-width
//!     `distance` *world units*, drawn as four mirrored `fullb.tga` quads whose
//!     transparent texture corner meets at the centre; a solid near-black
//!     `white02.bmp` frame fills everything outside it. Because the hole is a
//!     fixed world size it shrinks on screen as the camera zooms out — only a
//!     few cells around the master stay visible, instead of a fixed screen
//!     fraction that lets you zoom out to see the map.
//!   * Poison / CrystalBlue — a full-viewport tint wash (a tiled grid of the
//!     texture across the screen, which one stretched NDC quad reproduces
//!     directly).
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

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{CameraView, Effect, EffectRenderCtx, EffectUpdateCtx};

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
    /// Blind family (Blind / Devil / DevilRed): a world-space,
    /// camera-facing vignette centred on the master. A clear square hole of
    /// half-width `distance` *world units* (four mirrored `fullb.tga` gradient
    /// quads) surrounded by a solid near-black frame. Scales with camera zoom.
    WorldVignette,
    /// One quad stretched across the whole viewport (NDC).
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
    /// `WorldVignette` only: half-width of the clear central hole in **world
    /// units**. Blind = 50,
    /// DevilRed = 150, Devil = `140 - level*10`. Larger = more visible around
    /// the master. Unused by `Wash`.
    pub distance: f32,
    pub duration_ms: u32,
}

impl FullscreenOverlayParams {
    pub const fn total_duration_ms(&self) -> u32 {
        self.duration_ms
    }
}

/// Blind — near-black blindness centred on the master: a clear square
/// hole of half-width 50 world units (`fullb.tga` gradient) surrounded by a
/// solid near-black frame, billboarded toward the camera so it tracks the
/// player and narrows as the view zooms out.
pub const BLIND: FullscreenOverlayParams = FullscreenOverlayParams {
    texture: "fullb.tga",
    tint: [10.0 / 255.0, 10.0 / 255.0, 10.0 / 255.0],
    blend: BlendKind::Alpha,
    shape: OverlayShape::WorldVignette,
    ramp_per_frame: 1.0 / 255.0,
    max_alpha: 1.0,
    pulse: false,
    slashes: false,
    distance: 50.0,
    duration_ms: PERSISTENT_DURATION_MS,
};

/// Devil1-10. Same vignette, slightly lighter grey tint and a
/// wider clear hole (`140 - level*10`; level 5 ≈ 90).
pub const DEVIL: FullscreenOverlayParams = FullscreenOverlayParams {
    texture: "fullb.tga",
    tint: [30.0 / 255.0, 30.0 / 255.0, 30.0 / 255.0],
    blend: BlendKind::Alpha,
    shape: OverlayShape::WorldVignette,
    ramp_per_frame: 1.0 / 255.0,
    max_alpha: 1.0,
    pulse: false,
    slashes: false,
    distance: 90.0,
    duration_ms: PERSISTENT_DURATION_MS,
};

/// DevilRed. Red tint, faster ramp, wide hole.
pub const DEVIL_RED: FullscreenOverlayParams = FullscreenOverlayParams {
    texture: "fullb.tga",
    tint: [1.0, 0.0, 0.0],
    blend: BlendKind::Alpha,
    shape: OverlayShape::WorldVignette,
    ramp_per_frame: 3.0 / 255.0,
    max_alpha: 1.0,
    pulse: false,
    slashes: false,
    distance: 150.0,
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
    distance: 0.0,
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
    distance: 0.0,
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
    distance: 0.0,
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
        [
            self.params.tint[0],
            self.params.tint[1],
            self.params.tint[2],
            self.alpha,
        ]
    }
}

/// How far past the clear hole the solid dark frame extends, as a multiple of
/// the eye→target distance. The frame lives in a screen-parallel plane at the
/// master's depth, so a fixed *world* extent covers a shrinking screen
/// fraction as the camera pulls back; tying it to the eye distance keeps the
/// whole viewport blanketed at every zoom (a fixed extent of 100 only
/// sufficed for the original's limited zoom range).
const FILL_REACH_FACTOR: f32 = 4.0;

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-6 {
        [0.0, 0.0, 0.0]
    } else {
        [v[0] / len, v[1] / len, v[2] / len]
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Camera right / up basis vectors for a screen-parallel billboard.
fn camera_basis(cam: &CameraView) -> ([f32; 3], [f32; 3]) {
    let fwd = normalize(sub(cam.target, cam.eye));
    let mut right = normalize(cross(fwd, cam.up));
    if right == [0.0, 0.0, 0.0] {
        right = [1.0, 0.0, 0.0];
    }
    let up = normalize(cross(right, fwd));
    (right, up)
}

/// Build the world-space, camera-facing Blind vignette: a clear square hole of
/// half-width `distance` (four mirrored `fullb.tga` gradient quads, transparent
/// texture corner at the centre) wrapped in a solid `white02.bmp` frame out to
/// `fill_half`. `center` is the master position, `right`/`up` the camera basis;
/// every quad ignores depth so nothing occludes the overlay.
fn push_world_vignette(
    out: &mut EffectDrawList,
    center: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    distance: f32,
    fill_half: f32,
    color: [f32; 4],
    blend: BlendKind,
) {
    let world = |lx: f32, lz: f32| {
        [
            center[0] + right[0] * lx + up[0] * lz,
            center[1] + right[1] * lx + up[1] * lz,
            center[2] + right[2] * lx + up[2] * lz,
        ]
    };
    let quad = |out: &mut EffectDrawList, c: [[f32; 3]; 4], uv: [[f32; 2]; 4], tex| {
        out.push(EffectPrimitiveDraw::WorldQuad {
            corners: c,
            uv,
            texture: tex,
            color,
            blend,
            no_depth: true,
        });
    };

    // Four gradient quadrants — transparent texture corner (~uv 0,1) at the
    // centre, opaque corner toward each diagonal at `distance`. The UVs are
    // inset a few
    // texels off the `0`/`1` edges: the effect sampler wraps (`Repeat`), so
    // sampling exactly at an edge bilinear-blends the opposite (opaque) edge
    // back in — that bleed showed as a grey cross along the screen axes (the
    // `u=0` / `v=1` edges that touch the transparent corner).
    const E: f32 = 0.01;
    const GRAD_UV: [[f32; 2]; 4] = [[E, 1.0 - E], [1.0 - E, 1.0 - E], [1.0 - E, E], [E, E]];
    const QUADRANTS: [[f32; 2]; 4] = [[1.0, 1.0], [-1.0, 1.0], [-1.0, -1.0], [1.0, -1.0]];
    let d = distance;
    for [sx, sz] in QUADRANTS {
        let corners = [
            world(0.0, 0.0),
            world(0.0, sz * d),
            world(sx * d, sz * d),
            world(sx * d, 0.0),
        ];
        quad(out, corners, GRAD_UV, "fullb.tga");
    }

    // Solid dark frame around the hole: four non-overlapping bands tiling
    // [-F, F]² minus the central [-d, d]². `white02.bmp` is opaque so the
    // vertex colour/alpha drives the darkness.
    const FILL_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let f = fill_half.max(d * 1.5);
    let band = |out: &mut EffectDrawList, x0: f32, x1: f32, z0: f32, z1: f32| {
        let corners = [world(x0, z0), world(x1, z0), world(x1, z1), world(x0, z1)];
        quad(out, corners, FILL_UV, "white02.bmp");
    };
    band(out, -f, f, d, f); // top
    band(out, -f, f, -f, -d); // bottom
    band(out, d, f, -d, d); // right
    band(out, -f, -d, -d, d); // left
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
                OverlayShape::WorldVignette => {
                    // Centred on the master (the camera target tracks the
                    // afflicted player), billboarded toward the view. Built in
                    // world units so the clear hole narrows as the camera zooms
                    // out — only a few cells around the master stay visible.
                    let (right, up) = camera_basis(&ctx.camera);
                    let eye_dist = {
                        let d = sub(ctx.camera.target, ctx.camera.eye);
                        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
                    };
                    push_world_vignette(
                        out,
                        ctx.camera.target,
                        right,
                        up,
                        self.params.distance,
                        eye_dist * FILL_REACH_FACTOR,
                        self.body_color(),
                        self.params.blend,
                    );
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
        EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        }
    }

    /// Camera looking down-ish at the origin from `eye_dist` away — gives a
    /// non-degenerate basis so the world vignette builds real corners.
    fn render_ctx_at(eye_dist: f32) -> EffectRenderCtx {
        EffectRenderCtx {
            camera: CameraView {
                eye: [0.0, -eye_dist, eye_dist],
                target: [0.0, 0.0, 0.0],
                up: [0.0, -1.0, 0.0],
            },
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn render_ctx() -> EffectRenderCtx {
        render_ctx_at(100.0)
    }

    fn world_quads(
        e: &FullscreenOverlayEffect,
        ctx: &EffectRenderCtx,
    ) -> Vec<([[f32; 3]; 4], &'static str)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, ctx);
        list.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::WorldQuad {
                    corners,
                    texture,
                    no_depth,
                    ..
                } => {
                    assert!(*no_depth, "overlay quads ignore depth");
                    Some((*corners, *texture))
                }
                _ => None,
            })
            .collect()
    }

    fn step_frames(e: &mut FullscreenOverlayEffect, n: u32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    fn quads(
        e: &FullscreenOverlayEffect,
    ) -> Vec<([[f32; 2]; 4], [[f32; 2]; 4], &'static str, [f32; 4])> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::ScreenQuad {
                    corners,
                    uvs,
                    texture,
                    color,
                    ..
                } => Some((*corners, *uvs, *texture, *color)),
                _ => None,
            })
            .collect()
    }

    /// Planar distance of a world point from the vignette centre (the camera
    /// target = origin in the test camera).
    fn radius(p: [f32; 3]) -> f32 {
        (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt()
    }

    #[test]
    fn blind_is_a_world_vignette_clear_hole_plus_dark_frame() {
        let mut e = FullscreenOverlayEffect::new([0.0, 0.0, 0.0], BLIND);
        step_frames(&mut e, 30);
        let qs = world_quads(&e, &render_ctx());

        let grad: Vec<_> = qs.iter().filter(|(_, t)| *t == "fullb.tga").collect();
        let fill: Vec<_> = qs.iter().filter(|(_, t)| *t == "white02.bmp").collect();
        assert_eq!(grad.len(), 4, "four mirrored gradient quadrants");
        assert_eq!(fill.len(), 4, "four solid frame bands");

        // Each gradient quadrant's transparent corner sits at the centre.
        for (corners, _) in &grad {
            assert!(radius(corners[0]) < 1e-3, "gradient quad starts at centre");
        }
        // Clear hole spans `distance` world units; the dark frame reaches well
        // beyond it.
        let hole = grad
            .iter()
            .flat_map(|(c, _)| *c)
            .map(radius)
            .fold(0.0_f32, f32::max);
        let frame = fill
            .iter()
            .flat_map(|(c, _)| *c)
            .map(radius)
            .fold(0.0_f32, f32::max);
        assert!(
            (hole - BLIND.distance * std::f32::consts::SQRT_2).abs() < 1.0,
            "hole = distance: {hole}"
        );
        assert!(
            frame > hole * 3.0,
            "frame blankets far beyond the hole: {frame} vs {hole}"
        );
    }

    #[test]
    fn blind_hole_is_fixed_world_size_while_frame_tracks_zoom() {
        // Zooming out (larger eye distance) must NOT widen the clear hole — it
        // stays a fixed few cells — but the dark frame must grow to keep the
        // whole viewport covered.
        let mut e = FullscreenOverlayEffect::new([0.0, 0.0, 0.0], BLIND);
        step_frames(&mut e, 30);

        let measure = |dist: f32| {
            let qs = world_quads(&e, &render_ctx_at(dist));
            let hole = qs
                .iter()
                .filter(|(_, t)| *t == "fullb.tga")
                .flat_map(|(c, _)| *c)
                .map(radius)
                .fold(0.0_f32, f32::max);
            let frame = qs
                .iter()
                .filter(|(_, t)| *t == "white02.bmp")
                .flat_map(|(c, _)| *c)
                .map(radius)
                .fold(0.0_f32, f32::max);
            (hole, frame)
        };
        let (near_hole, near_frame) = measure(100.0);
        let (far_hole, far_frame) = measure(400.0);

        assert!(
            (near_hole - far_hole).abs() < 1e-3,
            "clear hole is zoom-independent"
        );
        assert!(
            far_frame > near_frame * 3.5,
            "dark frame grows with zoom-out: {near_frame} -> {far_frame}"
        );
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
        let slashes: Vec<_> = qs
            .iter()
            .filter(|(_, _, tex, _)| *tex == "lens_r.bmp")
            .collect();
        assert_eq!(slashes.len(), SLASH_COUNT, "three claw slashes");
        assert!(
            qs.iter().any(|(_, _, tex, _)| *tex == "white02.bmp"),
            "red wash present"
        );
        // Claw runs top-right (first vertex) to bottom-left (third vertex).
        let (corners, _, _, _) = slashes[0];
        assert!(
            corners[0][0] > corners[2][0] && corners[0][1] > corners[2][1],
            "top end is up-right of the bottom end: {corners:?}"
        );
    }

    #[test]
    fn bleeding_pulses_up_then_down() {
        let mut e = FullscreenOverlayEffect::new([0.0, 0.0, 0.0], BLEEDING);
        step_frames(&mut e, 20);
        let peak = quads(&e)
            .iter()
            .find(|(_, _, t, _)| *t == "white02.bmp")
            .unwrap()
            .3[3];
        step_frames(&mut e, 60);
        let tail = quads(&e)
            .iter()
            .find(|(_, _, t, _)| *t == "white02.bmp")
            .map(|q| q.3[3])
            .unwrap_or(0.0);
        assert!(peak > 0.0);
        assert!(
            tail < peak,
            "bleeding wash should fade after the pulse: {peak} -> {tail}"
        );
    }
}
