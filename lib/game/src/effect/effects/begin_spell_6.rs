//! `EF_BEGINSPELL6` — wind/holy cast aura.
//!
//! Structurally different from the other `BeginSpell*` variants. The other
//! variants build their cast circle via `BeginCasting` (3 tilted petals at
//! `rise_angle=70/57/45°` rotating around a near-vertical `rise_angle=89°,
//! max_height=250` light pillar). `BeginSpell6` instead calls
//! `SAINTCASTING(45, "ring_white.tga"); SAINTCASTING(25, ...)` once at
//! frame 0.
//!
//! `SAINTCASTING` seeds 4 GI-1 emitters at
//! `distance = 4.1`, `rise_angle = 80°`, full display angle = 360
//! (closed cone, not 315° petals), `max_height ∈ {20, 19, 18, 17}` and
//! `rot_start_deg ∈ {180, 270, 0, 90}`. Then the GI-1 primitive
//! animates each emitter per frame:
//!
//!   * `distance += 0.07` — bottom ring slides outward over time;
//!   * `rise_angle -= 1` (clamped at 10°) — the cone collapses FROM steep
//!     near-vertical TO flat radial splay. `cos(rise)` swings from ~0.17 to
//!     ~0.91, so `top_size = distance + cos(rise) * max_height` blows up
//!     from ~7.5 to ~26 even though `distance` only grows by ~4;
//!   * `height[i] = max_h + max_h * sin(across_ring) * 0.3 * sin(time_phase)`
//!     — per-segment flame flicker; positive envelope peaks opposite the
//!     `rot_start_deg` seam, modulated in time.
//!
//! No vertical center column (no `GI[3]` with `max_height=250`), no rotation
//! of the cone around its axis — it just expands in breadth as it ages.
//! Two `SAINTCASTING` calls at `time=45`/`time=25` lay down 2 overlapping
//! bursts at different peak alphas (start alpha = time + {135,90,45,0} per
//! emitter index).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

pub const TEXTURE: &str = "ring_white.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// `SAINTCASTING` clamps the parent effect's duration to 56 if smaller;
/// the parent then runs for that many ticks.
const TOTAL_FRAMES: f32 = 56.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

/// Initial per-emitter geometry (set in `SAINTCASTING`, identical across the
/// 4 emitters except `max_height` / start rotation / start alpha).
const INIT_DISTANCE: f32 = 4.1;
const INIT_RISE_DEG: f32 = 80.0;
/// Per-frame deltas (set in the GI ring-1 emitter).
const DISTANCE_GROW_PER_FRAME: f32 = 0.07;
const RISE_SHRINK_PER_FRAME: f32 = 1.0;
const RISE_FLOOR_DEG: f32 = 10.0;
/// Reset point used by the original game when an emitter's base alpha drops to 0
/// before the effect ends — the cone collapses back to a closer/steeper
/// position and the alpha pulse restarts.
const RESET_DISTANCE: f32 = 3.37;
const RESET_RISE_DEG: f32 = 74.0;
/// start alpha drains by 3/frame whenever `distance >= 4.0`, climbs by 10/frame
/// otherwise (the GI ring-1 emitter, lines 13399-13421).
const ALPHA_DRAIN_PER_FRAME: f32 = 3.0;
const ALPHA_REFILL_PER_FRAME: f32 = 10.0;
const ALPHA_REFILL_DISTANCE_GATE: f32 = 4.0;
/// Block the reset branch in the last `duration - 30` frames so the effect
/// doesn't spawn another pulse it can't fade out (matches the original GI1 pulse logic).
const RESET_BLOCK_FRAMES_FROM_END: f32 = 30.0;

/// 4 emitters per `SAINTCASTING` call. Differ only in `max_height`,
/// `rot_start_deg`, and the base alpha.
const NUM_EMITTERS: usize = 4;
/// `GI[ec].max_height` for `ec ∈ 0..4` with `F1 == 0` (white variant).
const MAX_HEIGHTS: [f32; NUM_EMITTERS] = [17.0, 18.0, 19.0, 20.0];
/// `GI[ec].rot_start` literals.
const ROT_START_DEG: [f32; NUM_EMITTERS] = [180.0, 270.0, 0.0, 90.0];
/// start alpha = time + offset.
const ALPHA_OFFSET: [f32; NUM_EMITTERS] = [135.0, 90.0, 45.0, 0.0];
/// Two saint-casting calls fire at `state_cnt==0` with these `time`s
/// (observed in the original game).
const PASS_TIMES: [f32; 2] = [45.0, 25.0];

/// Closed-cone segment count. Matches the original game's `E_DIVISION-1 = 20`
/// so the per-segment flicker wave has the same angular resolution as the
/// other cast-circle emitters.
const CONE_SIDES: u32 = 20;
const CONE_UV_REPEAT: f32 = 1.0;
/// `height[i]` swings ±30% around `max_h` per frame.
const WAVE_REL_AMPLITUDE: f32 = 0.3;
/// Per-emitter `pr` advance rates (degrees/frame). The original game alternates
/// between 1 and 2 deg/frame across emitter indices, so each emitter's wave
/// drifts to a different phase over time — at any given frame the 8 cones
/// have different flame-tip heights and different peak-angle positions
/// instead of all undulating in unison.
const WAVE_PHASE_PER_FRAME_DEG: [f32; NUM_EMITTERS] = [1.0, 2.0, 1.0, 2.0];

#[derive(Clone, Copy)]
struct Emitter {
    distance: f32,
    rise_deg: f32,
    alpha: f32,
    /// Seam angle for the closed cone — the original game's rotation start.
    rot_start_deg: f32,
    max_height: f32,
    /// Independent flame-flicker phase in radians. Advances by
    /// `wave_phase_rate_rad` each frame so emitters drift apart visually.
    wave_phase_rad: f32,
    wave_phase_rate_rad: f32,
}

impl Emitter {
    fn step(&mut self) {
        self.distance += DISTANCE_GROW_PER_FRAME;
        self.rise_deg = (self.rise_deg - RISE_SHRINK_PER_FRAME).max(RISE_FLOOR_DEG);
        self.wave_phase_rad += self.wave_phase_rate_rad;

        if self.distance >= ALPHA_REFILL_DISTANCE_GATE {
            self.alpha -= ALPHA_DRAIN_PER_FRAME;
        } else {
            self.alpha += ALPHA_REFILL_PER_FRAME;
        }
    }

    fn try_reset(&mut self, frames_remaining: f32) {
        if self.alpha > 0.0 || frames_remaining <= RESET_BLOCK_FRAMES_FROM_END {
            return;
        }
        self.distance = RESET_DISTANCE;
        self.rise_deg = RESET_RISE_DEG;
        self.alpha = 0.0;
    }

    /// Per-emitter alpha in [0, 1]. Uses the raw per-cell base alpha / 255
    /// scale so the 4-emitter brightness staircase set by SAINTCASTING
    /// (start alpha ∈ {180, 135, 90, 45} for `time=45`, halved-ish for `time=25`)
    /// shows up as a real intensity gradient instead of every emitter
    /// rendering at peak 1.0 and stacking to oversaturation.
    fn alpha_unit(&self) -> f32 {
        (self.alpha / 255.0).clamp(0.0, 1.0)
    }
}

pub struct BeginSpell6Effect {
    world_pos: [f32; 3],
    age: f32,
    emitters: Vec<Emitter>,
}

impl BeginSpell6Effect {
    pub fn new(attach: Attach) -> Self {
        let world_pos = match attach {
            Attach::WorldPos(p) => p,
            Attach::Entity(_) | Attach::Projectile { .. } => [0.0; 3],
        };
        let mut emitters = Vec::with_capacity(PASS_TIMES.len() * NUM_EMITTERS);
        for (pass_idx, pass_time) in PASS_TIMES.iter().enumerate() {
            for ec in 0..NUM_EMITTERS {
                let alpha = pass_time + ALPHA_OFFSET[ec];
                // Spread initial phases across emitters so the
                // `sin(wave_phase)` amplitude envelope is alive on day 1
                // for every emitter — without the spread the four pass-1
                // emitters all start at `sin(0) = 0` and the cone tops
                // look perfectly flat until the phase rotates round.
                // Pass 2 is offset another 90° so its four emitters
                // don't trace the same envelope as pass 1.
                let pass_offset_deg = if pass_idx == 0 { 0.0 } else { 90.0 };
                let initial_phase_deg: f32 = pass_offset_deg + ec as f32 * 45.0;
                emitters.push(Emitter {
                    distance: INIT_DISTANCE,
                    rise_deg: INIT_RISE_DEG,
                    alpha,
                    rot_start_deg: ROT_START_DEG[ec],
                    max_height: MAX_HEIGHTS[ec],
                    wave_phase_rad: initial_phase_deg.to_radians(),
                    wave_phase_rate_rad: WAVE_PHASE_PER_FRAME_DEG[ec].to_radians(),
                });
            }
        }
        Self {
            world_pos,
            age: 0.0,
            emitters,
        }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }
}

impl Effect for BeginSpell6Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frame_before = self.frame();
        self.age += ctx.delta;
        let frame_after = self.frame();
        // Catch up integer frame steps to keep the per-frame deltas
        // accurate regardless of `ctx.delta` granularity.
        let steps = (frame_after.floor() - frame_before.floor()).max(0.0) as i32;
        for _ in 0..steps {
            let frames_remaining = TOTAL_FRAMES - self.frame();
            for em in &mut self.emitters {
                em.step();
                em.try_reset(frames_remaining);
            }
        }
        if frame_after >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.frame();
        if frame > TOTAL_FRAMES {
            return;
        }
        for (i, em) in self.emitters.iter().enumerate() {
            let alpha = em.alpha_unit();
            if alpha <= 0.0 {
                continue;
            }
            let (sin_rise, cos_rise) = em.rise_deg.to_radians().sin_cos();
            let height = sin_rise * em.max_height;
            let bottom = em.distance;
            let top = em.distance + cos_rise * em.max_height;
            // `height[i] = max_h * (1 + sin(across)*0.3*sin(time))` — the
            // height swing is ±30% of max_h, scaled by the current vertical
            // component so the wave shrinks as the cone flattens.
            let wave_amplitude = WAVE_REL_AMPLITUDE * height * em.wave_phase_rad.sin();
            // let color = if i == 1 {
            //     [1.0, 0.0, 0.0, alpha]
            // } else if i == 2 {
            //     [0.0, 1.0, 0.0, alpha]
            // } else if i == 3 {
            //     [0.0, 0.0, 1.0, alpha]
            // } else {
            //     [1.0, 1.0, 1.0, alpha]
            // };
            let color = [1.0, 1.0, 1.0, alpha];
            out.push(EffectPrimitiveDraw::Frustum {
                base: self.world_pos,
                bottom_size: bottom,
                top_size: top,
                height,
                sides: CONE_SIDES,
                rotation: em.rot_start_deg.to_radians(),
                uv_repeat: CONE_UV_REPEAT,
                uv_scroll: [0.0, 0.0],
                wave_amplitude,
                wave_frequency: 1.0,
                wave_phase: em.wave_phase_rad,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                cull_back: true,
                texture: TEXTURE,
                color,
                blend: BlendKind::Additive,
            });
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

    fn draws(e: &BeginSpell6Effect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step_frames(e: &mut BeginSpell6Effect, n: u32) -> EffectStatus {
        let mut status = EffectStatus::Running;
        for _ in 0..n {
            status = e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None });
        }
        status
    }

    fn widest_top(prims: &[EffectPrimitiveDraw]) -> f32 {
        prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { top_size, .. } => Some(*top_size),
                _ => None,
            })
            .fold(0.0_f32, f32::max)
    }

    fn tallest(prims: &[EffectPrimitiveDraw]) -> f32 {
        prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { height, .. } => Some(*height),
                _ => None,
            })
            .fold(0.0_f32, f32::max)
    }

    #[test]
    fn cone_expands_in_breadth_and_collapses_vertically() {
        // The whole point of this effect: `rise_angle` drops, so `top_size`
        // grows dramatically while `height` shrinks. Early frames are
        // narrow+tall; late frames are wide+short.
        let mut e = BeginSpell6Effect::new(Attach::WorldPos([0.0; 3]));
        step_frames(&mut e, 4);
        let early_top = widest_top(&draws(&e));
        let early_h = tallest(&draws(&e));
        step_frames(&mut e, 40);
        let late_top = widest_top(&draws(&e));
        let late_h = tallest(&draws(&e));
        assert!(
            late_top > early_top * 2.0,
            "top width must more than double over the effect ({early_top} → {late_top})"
        );
        assert!(
            late_h < early_h,
            "vertical height must collapse as rise angle drops ({early_h} → {late_h})"
        );
    }

    #[test]
    fn no_vertical_center_pillar() {
        // CastCircle's column uses `max_height = 250`, height ≈ 250. The
        // tallest emitter here is `sin(80°)*20 ≈ 19.7` at frame 0, smaller
        // every frame after — must never approach 250.
        let mut e = BeginSpell6Effect::new(Attach::WorldPos([0.0; 3]));
        for _ in 0..(TOTAL_FRAMES as u32) {
            assert!(
                tallest(&draws(&e)) < 25.0,
                "no emitter is tall enough to read as a center pillar"
            );
            step_frames(&mut e, 1);
        }
    }

    #[test]
    fn cone_does_not_rotate_around_its_axis() {
        // `rot_start_deg` is set once and never advanced by the GI-1 primitive — every
        // emitted Frustum's `rotation` over the effect's lifetime must
        // belong to the finite set of initial `ROT_START_DEG` values.
        let mut e = BeginSpell6Effect::new(Attach::WorldPos([0.0; 3]));
        let allowed: Vec<f32> = ROT_START_DEG.iter().map(|d| d.to_radians()).collect();
        for _ in 0..(TOTAL_FRAMES as u32) {
            for p in draws(&e) {
                if let EffectPrimitiveDraw::Frustum { rotation, .. } = p {
                    assert!(
                        allowed.iter().any(|a| (a - rotation).abs() < 1e-5),
                        "rotation {rotation} must match an initial RotStart, never advance"
                    );
                }
            }
            step_frames(&mut e, 1);
        }
    }

    #[test]
    fn per_segment_wave_amplitude_present_while_cone_has_height() {
        // The flame-flicker wave amplitude is set to a nonzero value at
        // some point during the cone's lifetime (it's a function of
        // `sin(wave_phase)` so it crosses zero, but the magnitude over the
        // effect must reach a meaningful fraction of `height`).
        let mut e = BeginSpell6Effect::new(Attach::WorldPos([0.0; 3]));
        let mut max_amp = 0.0_f32;
        let mut max_h = 0.0_f32;
        for _ in 0..(TOTAL_FRAMES as u32) {
            for p in draws(&e) {
                if let EffectPrimitiveDraw::Frustum {
                    wave_amplitude,
                    height,
                    ..
                } = p
                {
                    max_amp = max_amp.max(wave_amplitude.abs());
                    max_h = max_h.max(height);
                }
            }
            step_frames(&mut e, 1);
        }
        assert!(
            max_amp > 0.1 * max_h,
            "wave amplitude must reach >10% of height at some point ({max_amp} vs {max_h})"
        );
    }

    #[test]
    fn dies_after_total_duration() {
        let mut e = BeginSpell6Effect::new(Attach::WorldPos([0.0; 3]));
        for f in 0..(TOTAL_FRAMES as u32) {
            assert_eq!(
                step_frames(&mut e, 1),
                EffectStatus::Running,
                "still alive at frame {f}"
            );
        }
        assert_eq!(step_frames(&mut e, 1), EffectStatus::Dead);
    }

    #[test]
    fn spawns_eight_emitters() {
        // 2 SAINTCASTING passes × 4 emitters = 8 Frustums while all alpha
        // is nonzero (frame 0 — every emitter's alpha_b ≥ 25, alpha_unit > 0).
        let e = BeginSpell6Effect::new(Attach::WorldPos([0.0; 3]));
        let n = draws(&e)
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
            .count();
        assert_eq!(n, 8);
    }
}
