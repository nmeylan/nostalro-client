use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{CameraView, Effect, EffectRenderCtx, EffectUpdateCtx};

pub const LENS1: &str = "lens1.tga";
pub const LENS2: &str = "lens2.tga";
pub const TEXTURES: &[&str] = &[LENS1, LENS2];

const FRAMES_PER_SECOND: f32 = 60.0;

const PETAL_COUNT: usize = 8;
const Y_OFFSET_BASE: f32 = -10.0;
const SIZE_SCALE: f32 = 1.0 / 3.0;
const WIDTH_MIN: f32 = 5.0 * SIZE_SCALE;
const WIDTH_MAX: f32 = 20.0 * SIZE_SCALE;
const HEIGHT_MIN: f32 = 20.0 * SIZE_SCALE;
const HEIGHT_MAX: f32 = 40.0 * SIZE_SCALE;

const HEIGHT_SPEED_INIT_ORIG: f32 = 1.5;
const HEIGHT_ACCEL_ORIG: f32 = 0.25;
const SPEED_MIN_PER_FRAME: f32 = 0.5;
const SPEED_MAX_PER_FRAME: f32 = 5.0;
const DURATION_MIN_FRAMES: f32 = 10.0;
const DURATION_MAX_FRAMES: f32 = 30.0;
const FADE_IN_FRAMES: f32 = 8.0;
const SPAWN_RADIUS_MAX: f32 = 5.0 * SIZE_SCALE;
const ANGLE_JITTER_DEG: f32 = 15.0;

pub const TOTAL_DURATION_MS: u32 = (DURATION_MAX_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn lcg_float(state: &mut u32) -> f32 {
    (lcg_next(state) >> 8) as f32 / ((1u32 << 24) as f32)
}

#[derive(Clone, Copy)]
struct Petal {
    slice_angle_rad: f32,
    roll_rad: f32,
    speed_world_per_s: f32,
    decel_world_per_s2: f32,
    radius: f32,
    width: f32,
    height: f32,
    width_speed_world_per_s: f32,
    height_speed_per_frame: f32,
    height_accel_per_frame: f32,
    age: f32,
    lifetime: f32,
    texture: &'static str,
}

impl Petal {
    fn alive(&self) -> bool {
        self.age < self.lifetime
    }

    fn alpha(&self) -> f32 {
        let frame = self.age * FRAMES_PER_SECOND;
        let fade_in = (frame / FADE_IN_FRAMES).clamp(0.0, 1.0);
        let fade_out = (1.0 - self.age / self.lifetime).clamp(0.0, 1.0);
        fade_in * fade_out
    }
}

pub struct Hit2Effect {
    world_pos: [f32; 3],
    petals: Vec<Petal>,
    age: f32,
    total_duration_s: f32,
    rng_state: u32,
    has_spawned: bool,
}

impl Hit2Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let rng_state =
            0x9E37_79B9 ^ world_pos[0].to_bits() ^ world_pos[2].to_bits().rotate_left(13);
        Self {
            world_pos,
            petals: Vec::with_capacity(PETAL_COUNT),
            age: 0.0,
            total_duration_s: TOTAL_DURATION_MS as f32 / 1000.0,
            rng_state,
            has_spawned: false,
        }
    }

    fn spawn_petals(&mut self) {
        let slice = std::f32::consts::TAU / PETAL_COUNT as f32;
        for k in 0..PETAL_COUNT {
            let slice_angle = k as f32 * slice;
            let jitter =
                (lcg_float(&mut self.rng_state) * 2.0 - 1.0) * ANGLE_JITTER_DEG.to_radians();
            let roll = slice_angle + jitter;

            let initial_radius = lcg_float(&mut self.rng_state) * SPAWN_RADIUS_MAX;

            let speed_per_frame = SPEED_MIN_PER_FRAME
                + lcg_float(&mut self.rng_state) * (SPEED_MAX_PER_FRAME - SPEED_MIN_PER_FRAME);
            let duration_frames = DURATION_MIN_FRAMES
                + lcg_float(&mut self.rng_state) * (DURATION_MAX_FRAMES - DURATION_MIN_FRAMES);
            let lifetime = duration_frames / FRAMES_PER_SECOND;

            // Per-frame accel = `-(speed / duration) / 2`.
            // Convert to per-second^2.
            let decel_per_frame = -(speed_per_frame / duration_frames) / 2.0;
            let speed_world_per_s = speed_per_frame * FRAMES_PER_SECOND;
            let decel_world_per_s2 = decel_per_frame * FRAMES_PER_SECOND * FRAMES_PER_SECOND;

            let width = WIDTH_MIN + lcg_float(&mut self.rng_state) * (WIDTH_MAX - WIDTH_MIN);
            let height = HEIGHT_MIN + lcg_float(&mut self.rng_state) * (HEIGHT_MAX - HEIGHT_MIN);
            let width_speed_world_per_s = -width / lifetime;
            let height_speed_per_frame = HEIGHT_SPEED_INIT_ORIG * SIZE_SCALE;
            let height_accel_per_frame = HEIGHT_ACCEL_ORIG * SIZE_SCALE;

            let texture = if k % 2 == 0 { LENS1 } else { LENS2 };

            self.petals.push(Petal {
                slice_angle_rad: slice_angle,
                roll_rad: roll,
                speed_world_per_s,
                decel_world_per_s2,
                radius: initial_radius,
                width,
                height,
                width_speed_world_per_s,
                height_speed_per_frame,
                height_accel_per_frame,
                age: 0.0,
                lifetime,
                texture,
            });
        }
    }

    fn step_petals(&mut self, dt: f32) {
        let dt_frames = dt * FRAMES_PER_SECOND;
        for p in &mut self.petals {
            p.speed_world_per_s = (p.speed_world_per_s + p.decel_world_per_s2 * dt).max(0.0);
            p.radius += p.speed_world_per_s * dt;
            p.width = (p.width + p.width_speed_world_per_s * dt).max(0.0);
            p.height_speed_per_frame += p.height_accel_per_frame * dt_frames;
            p.height += p.height_speed_per_frame * dt_frames;
            p.age += dt;
        }
    }
}

fn camera_right_up(cam: &CameraView) -> ([f32; 3], [f32; 3]) {
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let norm = |v: [f32; 3]| {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
        [v[0] / len, v[1] / len, v[2] / len]
    };
    let view = norm(sub(cam.target, cam.eye));
    let right = norm(cross(view, cam.up));
    let up = norm(cross(right, view));
    (right, up)
}

impl Effect for Hit2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        if !self.has_spawned {
            self.spawn_petals();
            self.has_spawned = true;
        }
        self.age += ctx.delta;
        self.step_petals(ctx.delta);
        self.petals.retain(|p| p.alive());
        if self.age >= self.total_duration_s && self.petals.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        let (right, up) = camera_right_up(&ctx.camera);
        let center = [
            self.world_pos[0],
            self.world_pos[1] + Y_OFFSET_BASE,
            self.world_pos[2],
        ];
        for p in &self.petals {
            let alpha = p.alpha();
            if alpha <= 0.0 || p.width <= 0.0 || p.height <= 0.0 {
                continue;
            }
            let (sin_a, cos_a) = p.slice_angle_rad.sin_cos();
            let pos = [
                center[0] + p.radius * (sin_a * right[0] + cos_a * up[0]),
                center[1] + p.radius * (sin_a * right[1] + cos_a * up[1]),
                center[2] + p.radius * (sin_a * right[2] + cos_a * up[2]),
            ];
            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: [p.width, p.height],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: p.roll_rad,
                texture: p.texture,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Alpha,
            });
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

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: CameraView {
                eye: [50.0, -80.0, -120.0],
                target: [0.0, 0.0, 0.0],
                up: [0.0, -1.0, 0.0],
            },
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn spawns_eight_petals_alternating_textures() {
        let mut e = Hit2Effect::new([0.0; 3]);
        e.update(&ctx(0.0));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        // Frame 0 alpha is 0 (fade-in start) — step one frame so the
        // petals become visible before checking the draw count.
        e.update(&ctx(1.0 / 60.0));
        list.primitives.clear();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), PETAL_COUNT);
        assert!(list.primitives.iter().all(|p| matches!(
            p,
            EffectPrimitiveDraw::Billboard {
                blend: BlendKind::Alpha,
                ..
            }
        )));
        let textures: Vec<&str> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Billboard { texture, .. } => Some(*texture),
                _ => None,
            })
            .collect();
        assert!(textures.iter().any(|t| *t == LENS1));
        assert!(textures.iter().any(|t| *t == LENS2));
        let n1 = textures.iter().filter(|t| **t == LENS1).count();
        let n2 = textures.iter().filter(|t| **t == LENS2).count();
        assert_eq!(n1, PETAL_COUNT / 2);
        assert_eq!(n2, PETAL_COUNT / 2);
    }

    #[test]
    fn petals_fan_out_in_the_camera_facing_plane() {
        let rctx = render_ctx();
        let cam = rctx.camera;
        let mut e = Hit2Effect::new([10.0, 20.0, 30.0]);
        e.update(&ctx(1.0 / 60.0));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &rctx);
        let centre = [10.0_f32, 20.0 + Y_OFFSET_BASE, 30.0];
        let view = {
            let v = [
                cam.target[0] - cam.eye[0],
                cam.target[1] - cam.eye[1],
                cam.target[2] - cam.eye[2],
            ];
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            [v[0] / l, v[1] / l, v[2] / l]
        };
        let mut count = 0;
        for prim in &list.primitives {
            let EffectPrimitiveDraw::Billboard { pos, .. } = prim else {
                continue;
            };
            let off = [pos[0] - centre[0], pos[1] - centre[1], pos[2] - centre[2]];
            let along_view = off[0] * view[0] + off[1] * view[1] + off[2] * view[2];
            assert!(
                along_view.abs() < 1e-3,
                "petal offset lies in the camera plane (⊥ view): along_view={along_view}"
            );
            count += 1;
        }
        assert_eq!(count, PETAL_COUNT);
    }

    #[test]
    fn petal_width_shrinks_height_grows() {
        let mut e = Hit2Effect::new([0.0; 3]);
        e.update(&ctx(0.0));
        let initial: Vec<(f32, f32)> = e.petals.iter().map(|p| (p.width, p.height)).collect();
        // Advance ~half a typical petal's lifetime.
        for _ in 0..10 {
            e.update(&ctx(1.0 / 60.0));
        }
        let shrunk_and_grew = e
            .petals
            .iter()
            .zip(&initial)
            .any(|(p, (w0, h0))| p.width < *w0 && p.height > *h0);
        assert!(
            shrunk_and_grew,
            "expected at least one petal with shrunk width + grown height"
        );
    }

    #[test]
    fn effect_dies_after_petals_finish() {
        let mut e = Hit2Effect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < 1.5 {
            status = e.update(&ctx(1.0 / 60.0));
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
