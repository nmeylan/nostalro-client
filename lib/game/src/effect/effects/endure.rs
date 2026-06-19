//! `EF_ENDURE` — Endure skill activation visual.
//!
//!
//! A pure screen-space recipe in the original:
//!
//!   * Frame 0 — central `endure.tga` icon, starting at 150 px square and
//!     shrinking at -5 px/frame with a decelerating +0.385 px/frame². Alpha
//!     ramps in over
//!     20 frames to 255, holds, then fades out the final third of the
//!     48-frame parent lifetime.
//!   * Every frame for the first 50 frames — one radial spike
//!     (`alpha_down.tga`): a tall thin rectangle (width ∈ [0.6, 1.6]
//!     px, height ∈ [30, 50] px) positioned at a distance ∈ [100, 140]
//!     px from centre along a random angle, rolled so
//!     its long axis points radially outward. 40-frame lifetime, same
//!     alpha envelope as the icon.
//!
//! Our system has no 2D screen primitive, so we approximate with
//! camera-facing world-space `Billboard`s anchored at the entity. The
//! screen-space pixel values are translated to world units by the
//! observed mapping `1 character ≈ 5 wu ≈ 50 px` (`px / 10` → wu).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const ICON_TEXTURE: &str = "endure.tga";
pub const SPIKE_TEXTURE: &str = "alpha_down.tga";
pub const TEXTURES: &[&str] = &[ICON_TEXTURE, SPIKE_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// Duration 80 frames — the icon lives the full 80 frames so
/// it stays on-screen while the inward-sliding spikes finish.
const PARENT_DURATION_FRAMES: f32 = 80.0;
const SPIKE_SPAWN_WINDOW_FRAMES: f32 = 55.0;

// Icon — starts at 150 px square in screen-space; the natural
// reading of the gif is that the icon takes up about a full character's
// height when shrunk, so a start size of ~16 wu and the floor of ~9 wu
// matches the silhouette.
const ICON_INITIAL_SIZE: f32 = 16.0;
const ICON_FINAL_SIZE: f32 = 9.0;
const ICON_FADE_IN_FRAMES: f32 = 20.0;
const ICON_FADE_OUT_AT: f32 = PARENT_DURATION_FRAMES - PARENT_DURATION_FRAMES / 3.0;
const ICON_HEIGHT_OFFSET: f32 = -5.0;
const ICON_MAX_ALPHA: f32 = 1.0;

// Radial spikes — width 0.6..1.6 px × height
// 30..50 px → ~0.06..0.16 wu × 3..5 wu via the 10 px/wu mapping.
// Thickness is nudged up slightly so each spike is visible at our
// default camera distance without becoming a fat bar.
const SPIKE_DURATION_FRAMES: f32 = 40.0;
const SPIKE_FADE_IN_FRAMES: f32 = 20.0;
const SPIKE_FADE_OUT_AT: f32 = SPIKE_DURATION_FRAMES - SPIKE_DURATION_FRAMES / 3.0;
const SPIKE_THICKNESS: f32 = 0.25;
const SPIKE_LENGTH_MIN: f32 = 3.0;
const SPIKE_LENGTH_MAX: f32 = 5.0;
// Initial radius from entity where the spike spawns. Spawn distance
// random(40) + 100 px → 10..14 wu via the 10 px/wu mapping.
// The spike then slides inward over its life (see SPIKE_INWARD_*).
const SPIKE_RADIUS_MIN: f32 = 10.0;
const SPIKE_RADIUS_MAX: f32 = 14.0;
// Inward slide: speed -4 px/frame,
// accel = -speed / duration = +0.1 px/frame² (decelerating inward
// motion that stops near the centre). 10 px/wu → 0.4 / 0.01.
const SPIKE_INWARD_SPEED_PER_FRAME: f32 = 0.4;
const SPIKE_INWARD_ACCEL_PER_FRAME2: f32 =
    SPIKE_INWARD_SPEED_PER_FRAME / SPIKE_DURATION_FRAMES;
// Spikes orbit the icon's anchor — lifted 20 px in screen space so
// they sit at the icon's centre, not on the ground.
const SPIKE_VERTICAL_OFFSET: f32 = -5.0;
const SPIKE_MAX_ALPHA: f32 = 1.0;

pub const TOTAL_DURATION_MS: u32 =
    ((PARENT_DURATION_FRAMES + SPIKE_DURATION_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

fn fade_in_out(frame: f32, peak: f32, fade_in_frames: f32, fade_out_at: f32, total: f32) -> f32 {
    let rise = (frame / fade_in_frames).clamp(0.0, 1.0);
    let fall = if frame < fade_out_at {
        1.0
    } else {
        let span = (total - fade_out_at).max(1e-3);
        (1.0 - (frame - fade_out_at) / span).clamp(0.0, 1.0)
    };
    peak * rise * fall
}

fn icon_size(frame: f32) -> f32 {
    // Linear interp from ICON_INITIAL_SIZE → ICON_FINAL_SIZE across the
    // shrink window (`duration - 35` ≈ 13
    // frames; we stretch it across the same window).
    let shrink_window = (PARENT_DURATION_FRAMES - 35.0).max(1.0);
    let t = (frame / shrink_window).clamp(0.0, 1.0);
    ICON_INITIAL_SIZE + (ICON_FINAL_SIZE - ICON_INITIAL_SIZE) * t
}

#[derive(Clone, Copy, Debug)]
struct Spike {
    anchor: [f32; 3],
    /// XZ direction (unit length) the spike points outward along.
    direction: [f32; 2],
    /// Roll angle in radians, so the rotation aligns the spike's long
    /// axis radially in screen
    /// space.
    longitude_rad: f32,
    /// Initial distance from the entity where the spike spawned; the
    /// effective radius shrinks each frame via the inward slide formula.
    initial_radius: f32,
    length: f32,
    age_frames: f32,
}

impl Spike {
    fn alive(&self) -> bool {
        self.age_frames < SPIKE_DURATION_FRAMES
    }

    fn step(&mut self, dt_frames: f32) {
        self.age_frames += dt_frames;
    }

    fn alpha(&self) -> f32 {
        fade_in_out(
            self.age_frames,
            SPIKE_MAX_ALPHA,
            SPIKE_FADE_IN_FRAMES,
            SPIKE_FADE_OUT_AT,
            SPIKE_DURATION_FRAMES,
        )
    }

    /// Current distance from the entity. Integrating
    /// `speed += accel; pos += speed` per tick with `speed = -4`
    /// and `accel = +0.1` gives the closed form
    /// `r(n) = r0 - speed*n + accel * n(n+1)/2` (speed and accel are
    /// magnitudes of the inward motion). Clamped at 0 so the spike
    /// settles on the centre once the slide overshoots.
    fn radius(&self) -> f32 {
        let n = self.age_frames.clamp(0.0, SPIKE_DURATION_FRAMES);
        let displacement = SPIKE_INWARD_SPEED_PER_FRAME * n
            - SPIKE_INWARD_ACCEL_PER_FRAME2 * n * (n + 1.0) / 2.0;
        (self.initial_radius - displacement).max(0.0)
    }

    fn position(&self) -> [f32; 3] {
        let r = self.radius();
        [
            self.anchor[0] + self.direction[0] * r,
            self.anchor[1] + SPIKE_VERTICAL_OFFSET,
            self.anchor[2] + self.direction[1] * r,
        ]
    }

    /// Screen-space rotation that aligns the spike's long axis radially
    /// **with the bright tip pointing inward toward the entity**. The
    /// texture's V=0 end (bright tip) sits at the top of the unrotated
    /// quad, so a rotation that flips the quad about its width axis
    /// points that tip back along the radial direction — matching the
    /// original game's look, where spikes converge on the centre rather
    /// than radiating outward.
    fn rotation(&self) -> f32 {
        self.longitude_rad + std::f32::consts::PI
    }
}

pub struct EndureEffect {
    world_pos: [f32; 3],
    spikes: Vec<Spike>,
    age_frames: f32,
    last_spawn_frame: i32,
    rng_state: u32,
}

impl EndureEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let rng_state = 0x9E37_79B9
            ^ world_pos[0].to_bits()
            ^ world_pos[2].to_bits().rotate_left(13);
        Self {
            world_pos,
            spikes: Vec::new(),
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

    fn spawn_spike(&mut self) {
        let longitude_deg = self.lcg_float() * 360.0;
        let initial_radius =
            SPIKE_RADIUS_MIN + self.lcg_float() * (SPIKE_RADIUS_MAX - SPIKE_RADIUS_MIN);
        let length = SPIKE_LENGTH_MIN + self.lcg_float() * (SPIKE_LENGTH_MAX - SPIKE_LENGTH_MIN);
        let longitude_rad = longitude_deg.to_radians();
        let (sn, cs) = longitude_rad.sin_cos();
        self.spikes.push(Spike {
            anchor: self.world_pos,
            direction: [sn, cs],
            longitude_rad,
            initial_radius,
            length,
            age_frames: 0.0,
        });
    }
}

impl Effect for EndureEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt_frames;

        let current_frame = self.age_frames.floor() as i32;
        let next_frame = self.last_spawn_frame + 1;
        for f in next_frame..=current_frame {
            if f >= 0 && (f as f32) < SPIKE_SPAWN_WINDOW_FRAMES {
                self.spawn_spike();
            }
        }
        self.last_spawn_frame = current_frame;

        for s in &mut self.spikes {
            s.step(dt_frames);
        }
        self.spikes.retain(|s| s.alive());

        if self.age_frames >= PARENT_DURATION_FRAMES + SPIKE_DURATION_FRAMES
            && self.spikes.is_empty()
        {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.age_frames <= PARENT_DURATION_FRAMES {
            let alpha = fade_in_out(
                self.age_frames,
                ICON_MAX_ALPHA,
                ICON_FADE_IN_FRAMES,
                ICON_FADE_OUT_AT,
                PARENT_DURATION_FRAMES,
            );
            if alpha > 0.0 {
                let size = icon_size(self.age_frames);
                out.push(EffectPrimitiveDraw::Billboard {
                    pos: [
                        self.world_pos[0],
                        self.world_pos[1] + ICON_HEIGHT_OFFSET,
                        self.world_pos[2],
                    ],
                    size: [size, size],
                    uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                    rotation: 0.0,
                    texture: ICON_TEXTURE,
                    color: [1.0, 1.0, 1.0, alpha],
                    blend: BlendKind::Alpha,
                });
            }
        }

        for s in &self.spikes {
            let alpha = s.alpha();
            if alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos: s.position(),
                size: [SPIKE_THICKNESS, s.length],
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                rotation: s.rotation(),
                texture: SPIKE_TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
                // The Endure spike is alpha-blended, not additive — additive
                // vanishes
                // against a bright lightmap.
                blend: BlendKind::Alpha,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step_frames(e: &mut EndureEffect, n: i32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    #[test]
    fn icon_plus_radial_spikes_emitted_on_correct_schedule() {
        // Sociable: central icon billboard + radial spike billboards
        // every frame for 50 frames; each spike is at most one initial
        // outer-band away from the entity (it can be closer due to the
        // inward slide).
        let mut e = EndureEffect::new([5.0, 0.0, 7.0]);
        step_frames(&mut e, 25);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let icon: usize = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Billboard { texture, .. } if *texture == ICON_TEXTURE))
            .count();
        let spikes: Vec<&EffectPrimitiveDraw> = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Billboard { texture, .. } if *texture == SPIKE_TEXTURE))
            .collect();
        assert_eq!(icon, 1, "one central icon");
        assert!(spikes.len() >= 20, "≈ one spike per frame, after 25 frames");
        assert!(
            spikes.iter().all(|p| matches!(p, EffectPrimitiveDraw::Billboard { blend: BlendKind::Alpha, .. })),
            "Endure spikes are alpha-blended (RF_EFFECT_OM)"
        );

        for prim in &spikes {
            if let EffectPrimitiveDraw::Billboard { pos, .. } = prim {
                let dx = pos[0] - 5.0;
                let dz = pos[2] - 7.0;
                let r = (dx * dx + dz * dz).sqrt();
                assert!(
                    r <= SPIKE_RADIUS_MAX + 0.5,
                    "spike radius {r} should not exceed initial-band max {SPIKE_RADIUS_MAX}",
                );
            }
        }
    }

    #[test]
    fn spikes_slide_inward_over_their_lifetime() {
        // Sociable: a spike's `radius()` strictly decreases from its
        // initial value as `age_frames` advances — the
        // `speed = -4 + accel*n` integration always points inward
        // for n < duration.
        let spike = Spike {
            anchor: [0.0, 0.0, 0.0],
            direction: [1.0, 0.0],
            longitude_rad: 0.0,
            initial_radius: SPIKE_RADIUS_MAX,
            length: SPIKE_LENGTH_MIN,
            age_frames: 0.0,
        };
        let r0 = spike.radius();
        let mut s_mid = spike;
        s_mid.age_frames = SPIKE_DURATION_FRAMES * 0.5;
        let mut s_end = spike;
        s_end.age_frames = SPIKE_DURATION_FRAMES;
        assert!(s_mid.radius() < r0, "slides inward: {r0} -> {}", s_mid.radius());
        assert!(s_end.radius() < s_mid.radius(), "still inward at end");
    }

    #[test]
    fn icon_shrinks_and_fades() {
        // Sociable: at frame 0 the icon is at INITIAL size and zero
        // alpha, mid-life it's at FINAL size and full alpha, late-life
        // it's faded down again.
        let s0 = icon_size(0.0);
        let s_mid = icon_size(PARENT_DURATION_FRAMES / 2.0);
        assert!(s0 > s_mid, "shrinks from initial → final");
        assert!((s0 - ICON_INITIAL_SIZE).abs() < 1e-3);
        let a0 = fade_in_out(0.0, ICON_MAX_ALPHA, ICON_FADE_IN_FRAMES, ICON_FADE_OUT_AT, PARENT_DURATION_FRAMES);
        let a_peak = fade_in_out(ICON_FADE_IN_FRAMES, ICON_MAX_ALPHA, ICON_FADE_IN_FRAMES, ICON_FADE_OUT_AT, PARENT_DURATION_FRAMES);
        let a_end = fade_in_out(PARENT_DURATION_FRAMES, ICON_MAX_ALPHA, ICON_FADE_IN_FRAMES, ICON_FADE_OUT_AT, PARENT_DURATION_FRAMES);
        assert!(a0 < a_peak);
        assert!(a_end < a_peak);
    }

    #[test]
    fn dies_after_parent_plus_spike_lifetime() {
        let mut e = EndureEffect::new([0.0; 3]);
        let total = PARENT_DURATION_FRAMES + SPIKE_DURATION_FRAMES + 5.0;
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
