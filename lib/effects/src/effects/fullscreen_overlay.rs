use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{CameraView, Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

pub const TEXTURES: &[&str] = &["fullb.tga", "poison_f.bmp", "white02.bmp", "lens_r.bmp"];

pub const PERSISTENT_DURATION_MS: u32 = 99990;
pub const PULSE_DURATION_MS: u32 = 1500;

const PULSE_RAMP_FRAMES: f32 = 10.0;
const PULSE_FADE_START_FRAME: f32 = 65.0;

const SLASH_ANGLE_DEG: f32 = 15.0;
const SLASH_COUNT: usize = 3;
const SLASH_MAX_ALPHA: f32 = 1.0;
const SLASH_LENGTH_FRAC: f32 = 0.6;
const SLASH_WIDTH_FRAC: f32 = 0.06;
const SLASH_SPACING_FRAC: f32 = 0.13;
const SLASH_STAGGER_FRAMES: f32 = 4.0;
const SLASH_GROW_FRAMES: f32 = 6.0;
const SLASH_FADE_FRAMES: f32 = 15.0;

#[derive(Clone, Copy, PartialEq)]
pub enum OverlayShape {
    WorldVignette,
    CircleVignette,
    Wash,
}

#[derive(Clone, Copy)]
pub struct FullscreenOverlayParams {
    pub texture: &'static str,
    pub tint: [f32; 3],
    pub blend: BlendKind,
    pub shape: OverlayShape,
    pub ramp_per_frame: f32,
    pub max_alpha: f32,
    pub pulse: bool,
    pub slashes: bool,
    pub distance: f32,
    pub duration_ms: u32,
}

impl FullscreenOverlayParams {
    pub const fn total_duration_ms(&self) -> u32 {
        self.duration_ms
    }
}

pub const BLIND: FullscreenOverlayParams = FullscreenOverlayParams {
    texture: "white02.bmp",
    tint: [10.0 / 255.0, 10.0 / 255.0, 10.0 / 255.0],
    blend: BlendKind::Alpha,
    shape: OverlayShape::CircleVignette,
    ramp_per_frame: 1.0 / 255.0,
    max_alpha: 1.0,
    pulse: false,
    slashes: false,
    distance: 18.0,
    duration_ms: PERSISTENT_DURATION_MS,
};

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

    fn body_color(&self) -> [f32; 4] {
        [
            self.params.tint[0],
            self.params.tint[1],
            self.params.tint[2],
            self.alpha,
        ]
    }
}

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

fn camera_basis(cam: &CameraView) -> ([f32; 3], [f32; 3]) {
    let fwd = normalize(sub(cam.target, cam.eye));
    let mut right = normalize(cross(fwd, cam.up));
    if right == [0.0, 0.0, 0.0] {
        right = [1.0, 0.0, 0.0];
    }
    let up = normalize(cross(right, fwd));
    (right, up)
}

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

    // UVs inset off the 0/1 edges: the sampler wraps (Repeat), so exact-edge
    // sampling bilinear-blends the opposite opaque edge in — grey cross artifact.
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

    const FILL_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let f = fill_half.max(d * 1.5);
    let band = |out: &mut EffectDrawList, x0: f32, x1: f32, z0: f32, z1: f32| {
        let corners = [world(x0, z0), world(x1, z0), world(x1, z1), world(x0, z1)];
        quad(out, corners, FILL_UV, "white02.bmp");
    };
    band(out, -f, f, d, f);
    band(out, -f, f, -f, -d);
    band(out, d, f, -d, d);
    band(out, -f, -d, -d, d);
}

const CIRCLE_SEGMENTS: usize = 96;
// Clear/dark radii in world units, so the hole covers a fixed number of cells and
// shrinks on screen as the camera zooms out. CAMERA_HALF_FOV_Y mirrors the renderer's
// fixed 15° vertical FOV (zoom is done by moving the eye, not changing FOV).
const CIRCLE_CLEAR_WORLD: f32 = 4.7;
const CIRCLE_DARK_WORLD: f32 = 11.0;
const CAMERA_HALF_FOV_Y_DEG: f32 = 7.5;

fn push_circle_vignette(
    out: &mut EffectDrawList,
    screen_w: f32,
    screen_h: f32,
    ndc_per_world: f32,
    tint: [f32; 3],
    alpha: f32,
) {
    let aspect = (screen_w / screen_h).max(1e-3);
    let clear = [tint[0], tint[1], tint[2], 0.0];
    let dark = [tint[0], tint[1], tint[2], alpha];
    let ry_clear = CIRCLE_CLEAR_WORLD * ndc_per_world;
    let ry_dark = CIRCLE_DARK_WORLD * ndc_per_world;

    let circle = |ry: f32, ang: f32| [ry * ang.cos() / aspect, ry * ang.sin()];
    let screen_edge = |ang: f32| {
        let dx = ang.cos() / aspect;
        let dy = ang.sin();
        let t = 1.05 / dx.abs().max(dy.abs());
        [dx * t, dy * t]
    };

    let mut vertices: Vec<([f32; 2], [f32; 4])> = Vec::with_capacity(CIRCLE_SEGMENTS * 3);
    for i in 0..CIRCLE_SEGMENTS {
        let ang = i as f32 / CIRCLE_SEGMENTS as f32 * std::f32::consts::TAU;
        vertices.push((circle(ry_clear, ang), clear));
        vertices.push((circle(ry_dark, ang), dark));
        vertices.push((screen_edge(ang), dark));
    }

    let mut indices: Vec<u32> = Vec::with_capacity(CIRCLE_SEGMENTS * 12);
    for i in 0..CIRCLE_SEGMENTS {
        let a = (3 * i) as u32;
        let b = (3 * ((i + 1) % CIRCLE_SEGMENTS)) as u32;
        indices.extend_from_slice(&[a, a + 1, b + 1, a, b + 1, b]);
        indices.extend_from_slice(&[a + 1, a + 2, b + 2, a + 1, b + 2, b + 1]);
    }

    out.push(EffectPrimitiveDraw::ScreenMesh {
        texture: "white02.bmp",
        blend: BlendKind::Alpha,
        vertices,
        indices,
    });
}

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
                OverlayShape::CircleVignette => {
                    let eye_dist = {
                        let d = sub(ctx.camera.eye, ctx.camera.target);
                        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
                    };
                    let ndc_per_world =
                        1.0 / (eye_dist.max(1.0) * CAMERA_HALF_FOV_Y_DEG.to_radians().tan());
                    push_circle_vignette(
                        out,
                        ctx.screen_w,
                        ctx.screen_h,
                        ndc_per_world,
                        self.params.tint,
                        self.alpha,
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

    fn screen_mesh(
        e: &FullscreenOverlayEffect,
        ctx: &EffectRenderCtx,
    ) -> Vec<([f32; 2], [f32; 4])> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, ctx);
        list.primitives
            .iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::ScreenMesh { vertices, .. } => Some(vertices.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    fn clear_ring_radius(e: &FullscreenOverlayEffect, dist: f32) -> f32 {
        let ctx = render_ctx_at(dist);
        let verts = screen_mesh(e, &ctx);
        let aspect = ctx.screen_w / ctx.screen_h;
        let radii: Vec<f32> = verts
            .iter()
            .filter(|(_, c)| c[3] == 0.0)
            .map(|([x, y], _)| ((x * aspect).powi(2) + y.powi(2)).sqrt())
            .collect();
        assert!(!radii.is_empty(), "inner ring is fully transparent");
        let min = radii.iter().cloned().fold(f32::MAX, f32::min);
        let max = radii.iter().cloned().fold(0.0_f32, f32::max);
        assert!(
            (max - min).abs() < 1e-3,
            "clear ring is a circle: {min}..{max}"
        );
        max
    }

    #[test]
    fn blind_clear_hole_is_circular_and_shrinks_when_zooming_out() {
        let mut e = FullscreenOverlayEffect::new([0.0, 0.0, 0.0], BLIND);
        step_frames(&mut e, 200);

        let near = clear_ring_radius(&e, 100.0);
        let far = clear_ring_radius(&e, 400.0);
        assert!(near > 0.0);
        assert!(
            far < near * 0.6,
            "zooming out shrinks the clear circle: {near} -> {far}"
        );

        let verts = screen_mesh(&e, &render_ctx_at(100.0));
        assert!(
            verts.iter().any(|(_, c)| c[3] > 0.5),
            "outer rings darken the screen"
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
