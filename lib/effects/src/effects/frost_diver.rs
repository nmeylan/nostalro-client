use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::spike_util::{FRAMES_PER_SECOND, apex_velocity, fade_tail_alpha, rise_step};
use crate::projectile::ProjectileCursor;

pub const ICE_TEXTURE: &str = "ice.tga";
pub const STONE_TEXTURE: &str = "stone.bmp";
pub const TEXTURES: &[&str] = &[ICE_TEXTURE, STONE_TEXTURE];

#[derive(Clone, Copy)]
pub struct FrostDiverParams {
    pub texture: &'static str,
    pub blend: BlendKind,
    pub spike_count_range: (u32, u32),
    /// `0` = all spawn at frame 0; `> 0` staggers across the window. Trail mode ignores this.
    pub burst_over_frames: f32,
    pub trail_cadence_frames: f32,
    pub trail_initial_offset: f32,
    pub spike_duration_frames: f32,
    pub base_half_width_range: (f32, f32),
    pub height_range: (f32, f32),
    pub spawn_radius_range: (f32, f32),
}

pub const FROSTDIVER2: FrostDiverParams = FrostDiverParams {
    texture: ICE_TEXTURE,
    blend: BlendKind::Additive,
    spike_count_range: (8, 8),
    burst_over_frames: 0.0,
    trail_cadence_frames: 1.0,
    trail_initial_offset: TRAIL_INITIAL_OFFSET,
    spike_duration_frames: 40.0,
    base_half_width_range: (0.6, 1.4),
    height_range: (4.0, 6.5),
    spawn_radius_range: (1.5, 5.0),
};

pub const FROSTDIVER: FrostDiverParams = FrostDiverParams {
    texture: ICE_TEXTURE,
    blend: BlendKind::Additive,
    spike_count_range: (3, 5),
    burst_over_frames: 14.0,
    trail_cadence_frames: 1.0,
    trail_initial_offset: TRAIL_INITIAL_OFFSET,
    spike_duration_frames: 40.0,
    base_half_width_range: (0.3, 0.6),
    height_range: (7.0, 10.0),
    spawn_radius_range: (3.0, 8.0),
};

pub const GRIMTOOTH: FrostDiverParams = FrostDiverParams {
    texture: STONE_TEXTURE,
    blend: BlendKind::Alpha,
    spike_count_range: (3, 6),
    burst_over_frames: 18.0,
    trail_cadence_frames: 3.0,
    trail_initial_offset: 2.0,
    spike_duration_frames: 40.0,
    base_half_width_range: (0.18, 0.3),
    height_range: (2.5, 4.0),
    spawn_radius_range: (2.0, 5.0),
};

const SPIKE_TILT_MIN_DEG: f32 = 80.0;
const SPIKE_TILT_MAX_DEG: f32 = 100.0;
const SPIKE_SPEED_PER_FRAME: f32 = 0.18;
const SPIKE_SPEED_PER_S: f32 = SPIKE_SPEED_PER_FRAME * FRAMES_PER_SECOND;
const SPEED_LIMIT_FRAMES: f32 = 20.0;
const SPEED_LIMIT_S: f32 = SPEED_LIMIT_FRAMES / FRAMES_PER_SECOND;

pub const PROJECTILE_FLIGHT: crate::effect_queue::ProjectileFlight =
    crate::effect_queue::ProjectileFlight::ConstantSpeed {
        delay_frames: 0.0,
        units_per_frame: TRAIL_STEP_PER_FRAME,
    };
const PEAK_ALPHA: f32 = 200.0 / 255.0;
const FADE_OUT_FRAMES: f32 = 10.0;
const TRAIL_STEP_PER_FRAME: f32 = 2.0;
const TRAIL_INITIAL_OFFSET: f32 = 5.0;

fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn lcg_float(state: &mut u32) -> f32 {
    (lcg_next(state) >> 8) as f32 / ((1u32 << 24) as f32)
}

#[derive(Clone, Copy)]
struct IceSpike {
    age: f32,
    duration: f32,
    base_pos: [f32; 3],
    velocity: [f32; 3],
    tilt_x_deg: f32,
    rotation_y_deg: f32,
    size: f32,
    height: f32,
}

impl IceSpike {
    fn step(&mut self, dt: f32) {
        rise_step(
            &mut self.base_pos,
            self.velocity,
            self.age,
            dt,
            SPEED_LIMIT_S,
        );
        self.age += dt;
    }

    fn alive(&self) -> bool {
        self.age < self.duration
    }

    fn alpha(&self) -> f32 {
        fade_tail_alpha(self.age, self.duration, PEAK_ALPHA, FADE_OUT_FRAMES)
    }
}

pub struct FrostDiverEffect {
    origin: [f32; 3],
    trail_anchors: Vec<[f32; 3]>,
    params: FrostDiverParams,
    spike_count: u32,
    age: f32,
    spikes: Vec<IceSpike>,
    rng_state: u32,
    spike_index: u32,
}

impl FrostDiverEffect {
    pub fn new(from: [f32; 3], to: [f32; 3], params: FrostDiverParams) -> Self {
        let (origin, trail_anchors) = derive_anchors(from, to, params.trail_initial_offset);

        let mut rng_state = 0x9E37_79B9 ^ origin[0].to_bits() ^ origin[2].to_bits().rotate_left(11);

        let spike_count = if !trail_anchors.is_empty() {
            trail_anchors.len() as u32
        } else {
            let (count_min, count_max) = params.spike_count_range;
            if count_max <= count_min {
                count_min
            } else {
                let span = count_max - count_min + 1;
                count_min + (lcg_next(&mut rng_state) % span)
            }
        };

        let mut e = Self {
            origin,
            trail_anchors,
            params,
            spike_count,
            age: 0.0,
            spikes: Vec::with_capacity(spike_count as usize),
            rng_state,
            spike_index: 0,
        };
        if e.params.burst_over_frames <= 0.0 {
            for _ in 0..e.spike_count {
                e.spawn_one();
            }
        }
        e
    }

    fn spawn_one(&mut self) {
        let (size_min, size_max) = self.params.base_half_width_range;
        let (height_min, height_max) = self.params.height_range;

        let spawn_pos =
            if let Some(anchor) = self.trail_anchors.get(self.spike_index as usize).copied() {
                anchor
            } else {
                let (radius_min, radius_max) = self.params.spawn_radius_range;
                let placement_angle = lcg_float(&mut self.rng_state) * std::f32::consts::TAU;
                let placement_radius =
                    radius_min + lcg_float(&mut self.rng_state) * (radius_max - radius_min);
                [
                    self.origin[0] + placement_radius * placement_angle.cos(),
                    self.origin[1],
                    self.origin[2] + placement_radius * placement_angle.sin(),
                ]
            };

        let heading_deg = lcg_float(&mut self.rng_state) * 360.0;
        let tilt_deg = SPIKE_TILT_MIN_DEG
            + lcg_float(&mut self.rng_state) * (SPIKE_TILT_MAX_DEG - SPIKE_TILT_MIN_DEG);
        let size = size_min + lcg_float(&mut self.rng_state) * (size_max - size_min);
        let height = height_min + lcg_float(&mut self.rng_state) * (height_max - height_min);

        let velocity = apex_velocity(tilt_deg, heading_deg, SPIKE_SPEED_PER_S);

        self.spike_index = self.spike_index.wrapping_add(1);
        self.spikes.push(IceSpike {
            age: 0.0,
            duration: self.params.spike_duration_frames / FRAMES_PER_SECOND,
            base_pos: spawn_pos,
            velocity,
            tilt_x_deg: tilt_deg,
            rotation_y_deg: heading_deg,
            size,
            height,
        });
    }

    fn spawn_window_frames(&self) -> f32 {
        if self.trail_anchors.is_empty() {
            self.params.burst_over_frames
        } else {
            self.params.trail_cadence_frames * self.spike_count as f32
        }
    }

    fn total_duration_s(&self) -> f32 {
        (self.spawn_window_frames() + self.params.spike_duration_frames) / FRAMES_PER_SECOND
    }
}

fn derive_anchors(from: [f32; 3], to: [f32; 3], initial_offset: f32) -> ([f32; 3], Vec<[f32; 3]>) {
    let mut cursor = ProjectileCursor::new(from, to, TRAIL_STEP_PER_FRAME);
    if cursor.dist() <= initial_offset {
        return (from, Vec::new());
    }
    let ground_y = from[1];
    let mut anchors = Vec::new();
    loop {
        let arrived = cursor.advance();
        if cursor.traveled() >= initial_offset {
            let p = cursor.pos();
            anchors.push([p[0], ground_y, p[2]]);
        }
        if arrived {
            break;
        }
    }
    (from, anchors)
}

impl Effect for FrostDiverEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt = ctx.delta;
        self.age += dt;
        for spike in &mut self.spikes {
            spike.step(dt);
        }

        let window_frames = self.spawn_window_frames();
        if window_frames > 0.0 {
            let burst_s = window_frames / FRAMES_PER_SECOND;
            let target_spawned = ((self.age / burst_s) * self.spike_count as f32) as u32;
            let target = target_spawned.min(self.spike_count);
            while self.spike_index < target {
                self.spawn_one();
            }
        }

        self.spikes.retain(|s| s.alive());

        if self.age >= self.total_duration_s() && self.spikes.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for spike in &self.spikes {
            out.push(EffectPrimitiveDraw::QuadHorn {
                base: spike.base_pos,
                size: spike.size,
                height: spike.height,
                tilt_x_deg: spike.tilt_x_deg,
                rotation_y_deg: spike.rotation_y_deg,
                texture: self.params.texture,
                color: [1.0, 1.0, 1.0, spike.alpha()],
                blend: self.params.blend,
            });
        }
    }
}

pub const fn total_duration_ms(params: &FrostDiverParams) -> u32 {
    let frames = params.burst_over_frames + params.spike_duration_frames;
    (frames / FRAMES_PER_SECOND * 1000.0) as u32
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

    fn step(effect: &mut FrostDiverEffect, dt: f32) {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn draws(effect: &FrostDiverEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn fd_trail_lays_spikes_along_caster_to_target_line() {
        let from = [0.0, 0.0, 0.0];
        let to = [0.0, 0.0, 25.0];
        let mut e = FrostDiverEffect::new(from, to, FROSTDIVER);
        step(
            &mut e,
            FROSTDIVER.burst_over_frames / FRAMES_PER_SECOND + 0.01,
        );
        let spawned = e.spike_index;
        assert_eq!(spawned, 11, "11 spikes expected for a 25-unit trail");

        let mut max_z = f32::NEG_INFINITY;
        for prim in draws(&e) {
            let EffectPrimitiveDraw::QuadHorn { base, .. } = prim else {
                panic!("expected QuadHorn, got {prim:?}");
            };
            assert!(
                base[0].abs() < 1e-3,
                "spike X stays on the +Z line: {base:?}"
            );
            assert!(
                base[2] >= TRAIL_INITIAL_OFFSET - 1e-3 && base[2] <= to[2] + 1e-3,
                "spike Z {} must lie within [{}, {}]",
                base[2],
                TRAIL_INITIAL_OFFSET,
                to[2],
            );
            max_z = max_z.max(base[2]);
        }
        assert!(
            (max_z - to[2]).abs() < 1e-3,
            "the last spike must land on the target (z={}), got {max_z}",
            to[2],
        );
    }

    #[test]
    fn fd_trail_distance_scales_spike_count() {
        let from = [0.0, 0.0, 0.0];
        let short = FrostDiverEffect::new(from, [0.0, 0.0, 15.0], FROSTDIVER);
        let long = FrostDiverEffect::new(from, [0.0, 0.0, 35.0], FROSTDIVER);
        assert!(long.spike_count > short.spike_count + 5);

        let too_close = FrostDiverEffect::new(from, [0.0, 0.0, 3.0], FROSTDIVER);
        assert!(
            (FROSTDIVER.spike_count_range.0..=FROSTDIVER.spike_count_range.1)
                .contains(&too_close.spike_count),
            "too-close trail falls back to cluster range, got {}",
            too_close.spike_count,
        );
    }

    #[test]
    fn fd2_emits_eight_spikes_at_frame_zero() {
        let mut e = FrostDiverEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], FROSTDIVER2);
        step(&mut e, 0.0);
        let prims = draws(&e);
        assert_eq!(prims.len(), 8);
        for p in &prims {
            let EffectPrimitiveDraw::QuadHorn {
                tilt_x_deg,
                texture,
                blend,
                ..
            } = p
            else {
                panic!("expected QuadHorn, got {p:?}");
            };
            assert!(
                (SPIKE_TILT_MIN_DEG..=SPIKE_TILT_MAX_DEG).contains(tilt_x_deg),
                "tilt {tilt_x_deg} out of range"
            );
            assert_eq!(*texture, ICE_TEXTURE);
            assert_eq!(*blend, BlendKind::Additive);
        }
    }

    #[test]
    fn fd_and_fd2_size_ranges_differ_per_orig() {
        assert!(
            FROSTDIVER.base_half_width_range.1 <= FROSTDIVER2.base_half_width_range.0,
            "FD spikes must be narrower than FD2"
        );
        assert!(
            FROSTDIVER.height_range.0 > FROSTDIVER2.height_range.1,
            "FD spikes must be taller than FD2"
        );
        assert!(
            FROSTDIVER.spawn_radius_range.1 > FROSTDIVER2.spawn_radius_range.1,
            "FD must reach further out than FD2"
        );
    }

    #[test]
    fn fd_staggers_spawn_across_burst_window() {
        let mut e = FrostDiverEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], FROSTDIVER);
        step(&mut e, 0.0);
        let n0 = draws(&e).len();

        step(
            &mut e,
            FROSTDIVER.burst_over_frames / FRAMES_PER_SECOND / 2.0,
        );
        let n_mid = draws(&e).len();
        assert!(n_mid >= n0);

        step(&mut e, FROSTDIVER.burst_over_frames / FRAMES_PER_SECOND);
        let n_full = draws(&e).len();
        assert!(n_full >= n_mid);
        assert!(n_full <= FROSTDIVER.spike_count_range.1 as usize);
    }

    #[test]
    fn fd_spike_count_stays_inside_orig_range() {
        for (origin, label) in [
            ([0.0, 0.0, 0.0], "origin zero"),
            ([10.0, 0.0, -5.0], "origin offset"),
            ([-3.5, 0.0, 22.7], "origin offset 2"),
        ] {
            let mut e = FrostDiverEffect::new(origin, origin, FROSTDIVER);
            // Step past burst window so all scheduled spikes have spawned.
            step(
                &mut e,
                FROSTDIVER.burst_over_frames / FRAMES_PER_SECOND + 0.01,
            );
            let drawn = draws(&e).len() as u32;
            let spawned = e.spike_index;
            let (lo, hi) = FROSTDIVER.spike_count_range;
            assert!(
                (lo..=hi).contains(&spawned),
                "{label}: spawned {spawned} not in [{lo}, {hi}]"
            );
            assert!(
                spawned < FROSTDIVER2.spike_count_range.0,
                "{label}: FD spawned {spawned} >= FD2's fixed 8"
            );
            assert!(drawn <= spawned, "{label}: drawn ≤ spawned");
        }
    }

    #[test]
    fn spike_motion_stops_after_speed_limit() {
        let mut e = FrostDiverEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], FROSTDIVER2);
        step(&mut e, 0.0);
        let early_y = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { base, .. } => base[1],
            _ => unreachable!(),
        };

        // Step through the speed-limit window.
        step(&mut e, SPEED_LIMIT_S);
        let limit_y = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { base, .. } => base[1],
            _ => unreachable!(),
        };
        assert!(limit_y < early_y, "spike base rose (Y went more negative)");

        step(&mut e, 5.0 / FRAMES_PER_SECOND);
        let after_y = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { base, .. } => base[1],
            _ => unreachable!(),
        };
        assert!((after_y - limit_y).abs() < 1e-4, "frozen after speed limit");
    }

    #[test]
    fn alpha_fades_in_final_window() {
        let mut e = FrostDiverEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], FROSTDIVER2);
        step(&mut e, 0.0);
        let a0 = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!((a0 - PEAK_ALPHA).abs() < 1e-4);

        let fade_start_s =
            (FROSTDIVER2.spike_duration_frames - FADE_OUT_FRAMES) / FRAMES_PER_SECOND;
        step(&mut e, fade_start_s - 0.001);
        let a_pre = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!((a_pre - PEAK_ALPHA).abs() < 1e-3);

        step(&mut e, FADE_OUT_FRAMES / FRAMES_PER_SECOND * 0.5);
        let a_fade = match draws(&e).first() {
            Some(EffectPrimitiveDraw::QuadHorn { color, .. }) => color[3],
            _ => 0.0,
        };
        assert!(a_fade < PEAK_ALPHA);
    }

    #[test]
    fn dies_when_all_spikes_expire() {
        let mut e = FrostDiverEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], FROSTDIVER2);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        let end_s = total_duration_ms(&FROSTDIVER2) as f32 / 1000.0;
        while t < end_s * 2.0 {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert!(matches!(status, EffectStatus::Dead));
    }
}
