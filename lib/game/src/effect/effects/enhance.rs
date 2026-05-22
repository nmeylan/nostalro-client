//! `EF_ENHANCE` — weapon enhancement / buff visual.
//!
//! Composite spawned at the master's feet, a buff ring with rising
//! streaks:
//!
//!   * Frame 0 — a `Cylinder` (`alpha_down.tga`, both radii = 6,
//!     height speed 2/frame, decel `-(speed/duration)/1.5`, height capped at
//!     50). Alpha holds at `128/255` then fades out.
//!   * Every 5 frames while alive — one vertical
//!     streak at a random ring position (`radius ∈ [2, 7]`), 7 wu tall and
//!     0.25 wu thick, rising at 2 wu/frame for a 50-frame
//!     lifetime. The streak is two perpendicular textured quads;
//!     a single `Billboard` reproduces the silhouette in the gif.
//!
//! The original game lays a yellow ground ring (alpha 128) too, but the
//! reference gif shows no visible ring on the ground — additive 128/255
//! against black reads as nothing, and at the in-game camera distance the
//! segmented ring disappears behind the cylinder bottom. We skip emitting
//! it.
//!
//! Parent lifetime is the bucket-0-50 default of 800 ms (48 frames at 60
//! fps). Effective lifetime includes the last streak's 50-frame fade so
//! `TOTAL_DURATION_MS` outlasts the parent by the particle envelope.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const ALPHA_TEXTURE: &str = "alpha_down.tga";
pub const TEXTURES: &[&str] = &[ALPHA_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const PARENT_DURATION_FRAMES: f32 = 48.0;

// Cylinder. bottom radius = top radius = 6, height speed 2/frame,
// height accel = -(height_speed/duration)/1.5, max height 50, alpha 128.
const CYLINDER_RADIUS: f32 = 6.0;
const CYLINDER_HEIGHT_SPEED: f32 = 2.0;
const CYLINDER_HEIGHT_ACCEL: f32 =
    -(CYLINDER_HEIGHT_SPEED / PARENT_DURATION_FRAMES) / 1.5;
const CYLINDER_MAX_HEIGHT: f32 = 50.0;
const CYLINDER_MAX_ALPHA: f32 = 128.0 / 255.0;
const CYLINDER_FADE_OUT_AT: f32 = PARENT_DURATION_FRAMES - PARENT_DURATION_FRAMES / 3.0;
const CYLINDER_SIDES: u32 = 24;

// Crossed-quad streaks, one spawned every 5 frames.
const SPAWN_PERIOD_FRAMES: u32 = 5;
const STREAK_DURATION_FRAMES: f32 = 50.0;
const STREAK_LENGTH: f32 = 7.0;
const STREAK_THICKNESS: f32 = 0.25;
// 2 wu/frame upward; the streak is laid flat then tilted 90° so its
// velocity points along native RO -Y (upward).
const STREAK_SPEED_PER_FRAME: f32 = -2.0;
const STREAK_RADIUS_MIN: f32 = 2.0;
const STREAK_RADIUS_MAX: f32 = 7.0;
const STREAK_FADE_OUT_AT: f32 = STREAK_DURATION_FRAMES - STREAK_DURATION_FRAMES / 3.0;

pub const TOTAL_DURATION_MS: u32 =
    ((PARENT_DURATION_FRAMES + STREAK_DURATION_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

fn fade_out(frame: f32, peak: f32, fade_out_at: f32, total: f32) -> f32 {
    if frame < fade_out_at {
        peak
    } else {
        let span = (total - fade_out_at).max(1e-3);
        (peak * (1.0 - (frame - fade_out_at) / span)).max(0.0)
    }
}

fn cylinder_height(frame: f32) -> f32 {
    let raw = CYLINDER_HEIGHT_SPEED * frame
        + CYLINDER_HEIGHT_ACCEL * frame * (frame + 1.0) / 2.0;
    raw.clamp(0.0, CYLINDER_MAX_HEIGHT)
}

#[derive(Clone, Copy, Debug)]
struct Streak {
    anchor: [f32; 3],
    offset: [f32; 3],
    age_frames: f32,
}

impl Streak {
    fn alive(&self) -> bool {
        self.age_frames < STREAK_DURATION_FRAMES
    }

    fn step(&mut self, dt_frames: f32) {
        self.offset[1] += STREAK_SPEED_PER_FRAME * dt_frames;
        self.age_frames += dt_frames;
    }

    fn alpha(&self) -> f32 {
        fade_out(
            self.age_frames,
            1.0,
            STREAK_FADE_OUT_AT,
            STREAK_DURATION_FRAMES,
        )
    }

    fn position(&self) -> [f32; 3] {
        // Billboard pos sits at the centre of the streak quad; the streak
        // length extends `STREAK_LENGTH / 2` above and below this point.
        // The streak spawns at ground (y=0); the centre is therefore
        // half-length above ground at age 0.
        [
            self.anchor[0] + self.offset[0],
            self.anchor[1] + self.offset[1] - STREAK_LENGTH * 0.5,
            self.anchor[2] + self.offset[2],
        ]
    }
}

pub struct EnhanceEffect {
    world_pos: [f32; 3],
    streaks: Vec<Streak>,
    age_frames: f32,
    last_spawn_frame: i32,
    rng_state: u32,
}

impl EnhanceEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let rng_state = 0x9E37_79B9
            ^ world_pos[0].to_bits()
            ^ world_pos[2].to_bits().rotate_left(13);
        Self {
            world_pos,
            streaks: Vec::new(),
            age_frames: 0.0,
            last_spawn_frame: -1,
            rng_state,
        }
    }

    fn lcg(&mut self) -> u32 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.rng_state
    }

    fn lcg_float(&mut self) -> f32 {
        (self.lcg() >> 8) as f32 / ((1u32 << 24) as f32)
    }

    fn spawn_streak(&mut self) {
        let longitude_deg = self.lcg_float() * 360.0;
        let radius = STREAK_RADIUS_MIN
            + self.lcg_float() * (STREAK_RADIUS_MAX - STREAK_RADIUS_MIN);
        let (sn, cs) = longitude_deg.to_radians().sin_cos();
        self.streaks.push(Streak {
            anchor: self.world_pos,
            offset: [radius * sn, 0.0, radius * cs],
            age_frames: 0.0,
        });
    }
}

impl Effect for EnhanceEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt_frames;

        let current_frame = self.age_frames.floor() as i32;
        if self.age_frames <= PARENT_DURATION_FRAMES {
            let next_frame = self.last_spawn_frame + 1;
            for f in next_frame..=current_frame {
                if f >= 0
                    && (f as f32) <= PARENT_DURATION_FRAMES
                    && (f as u32) % SPAWN_PERIOD_FRAMES == 0
                {
                    self.spawn_streak();
                }
            }
            self.last_spawn_frame = current_frame;
        }

        for s in &mut self.streaks {
            s.step(dt_frames);
        }
        self.streaks.retain(|s| s.alive());

        if self.age_frames >= PARENT_DURATION_FRAMES + STREAK_DURATION_FRAMES
            && self.streaks.is_empty()
        {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let parent_frame = self.age_frames.min(PARENT_DURATION_FRAMES);

        // Cylinder.
        if self.age_frames <= PARENT_DURATION_FRAMES {
            let height = cylinder_height(parent_frame);
            let cyl_alpha = fade_out(
                parent_frame,
                CYLINDER_MAX_ALPHA,
                CYLINDER_FADE_OUT_AT,
                PARENT_DURATION_FRAMES,
            );
            if height > 0.0 && cyl_alpha > 0.0 {
                out.push(EffectPrimitiveDraw::Frustum {
                    base: self.world_pos,
                    bottom_size: CYLINDER_RADIUS,
                    top_size: CYLINDER_RADIUS,
                    height,
                    sides: CYLINDER_SIDES,
                    rotation: 0.0,
                    uv_repeat: 1.0,
                    uv_scroll: [0.0, 0.0],
                    wave_amplitude: 0.0,
                    wave_frequency: 0.0,
                    wave_phase: 0.0,
                    tilt_x_rad: 0.0,
                    rotation_y_rad: 0.0,
                    cull_back: false,
                    texture: ALPHA_TEXTURE,
                    color: [1.0, 1.0, 1.0, cyl_alpha],
                    blend: BlendKind::Alpha,
                });
            }
        }

        for s in &self.streaks {
            let alpha = s.alpha();
            if alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos: s.position(),
                size: [STREAK_THICKNESS, STREAK_LENGTH],
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                rotation: 0.0,
                texture: ALPHA_TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
            });
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

    fn step_frames(e: &mut EnhanceEffect, n: i32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    #[test]
    fn cylinder_and_streaks_emit_on_schedule() {
        // Sociable: cylinder is drawn while parent alive, streaks spawn
        // every 5 frames. Reference gif shows no ground ring even though
        // the original game lays one down — we skip it.
        let mut e = EnhanceEffect::new([5.0, 0.0, 7.0]);
        // Step to frame 11 — frames 0, 5, 10 have spawned streaks.
        step_frames(&mut e, 11);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let cylinders = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
            .count();
        let streaks = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Billboard { .. }))
            .count();
        let rings = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { .. }))
            .count();

        assert_eq!(rings, 0, "no ground disc — invisible in reference gif");
        assert_eq!(cylinders, 1);
        assert!(streaks >= 3, "frames 0, 5, 10 each spawn a streak");

        // Each streak sits on the radius 2..7 ring around the anchor.
        for prim in &list.primitives {
            if let EffectPrimitiveDraw::Billboard { pos, .. } = prim {
                let dx = pos[0] - 5.0;
                let dz = pos[2] - 7.0;
                let r = (dx * dx + dz * dz).sqrt();
                assert!(
                    (STREAK_RADIUS_MIN - 0.5..=STREAK_RADIUS_MAX + 0.5).contains(&r),
                    "streak on radius {STREAK_RADIUS_MIN}..{STREAK_RADIUS_MAX} ring: r={r}",
                );
            }
        }
    }

    #[test]
    fn cylinder_height_grows_then_caps() {
        // Sociable: height speed=2, decel<0 → grows quickly early, caps at
        // CYLINDER_MAX_HEIGHT. Pin both the early growth and the cap
        // without binding the exact closed-form value.
        let h_early = cylinder_height(2.0);
        let h_mid = cylinder_height(20.0);
        let h_late = cylinder_height(45.0);
        assert!(h_early < h_mid, "height grows over time");
        assert!(h_late <= CYLINDER_MAX_HEIGHT);
        assert!(h_mid > 5.0, "by mid-life the cylinder is visible");
    }

    #[test]
    fn streaks_rise_over_their_lifetime() {
        // The streak's y position decreases (native RO: -Y is up) as it
        // ages. Spawn one streak, step a few frames, observe.
        let mut e = EnhanceEffect::new([0.0; 3]);
        step_frames(&mut e, 1);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let y0 = list
            .primitives
            .iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::Billboard { pos, .. } => Some(pos[1]),
                _ => None,
            })
            .expect("streak emitted at frame 0");

        step_frames(&mut e, 4);
        let mut list2 = EffectDrawList::new();
        e.collect_draws(&mut list2, &render_ctx());
        let y1 = list2
            .primitives
            .iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::Billboard { pos, .. } => Some(pos[1]),
                _ => None,
            })
            .expect("streak still alive");
        assert!(y1 < y0, "native RO -Y is up, so y decreases over time");
    }

    #[test]
    fn dies_after_parent_plus_streak_lifetime() {
        let mut e = EnhanceEffect::new([0.0; 3]);
        let total = PARENT_DURATION_FRAMES + STREAK_DURATION_FRAMES + 5.0;
        let mut status = EffectStatus::Running;
        for _ in 0..(total as i32) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
