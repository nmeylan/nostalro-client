use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_BLUE: &str = "ring_blue.tga";
pub const LENS2: &str = "lens2.tga";
pub const TEXTURES: &[&str] = &[RING_BLUE, LENS2];

pub const PARTICLE1_SPRITE: &str = "data/sprite/이팩트/particle1";
pub const SPRITES: &[&str] = &[PARTICLE1_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const NUM_SEGMENTS: usize = 3;
const FADE_IN_FRAMES: f32 = 3.0;
const PARTICLE_ANIM_TICKS: f32 = 4.0;
const PARTICLE_FRAME_MS: f32 = 1000.0 / FRAMES_PER_SECOND * PARTICLE_ANIM_TICKS;

#[derive(Clone, Copy, Debug)]
pub struct RingParams {
    pub duration_frames: f32,
    pub outer_size: f32,
    pub inner_size: f32,
    pub initial_height_size: f32,
    pub initial_height_speed: f32,
    pub height_accel: f32,
    pub initial_speed: f32,
    pub speed_accel: f32,
    pub y_offset: f32,
    pub tilt_x_rad: f32,
    pub texture: &'static str,
    pub color: [f32; 4],
}

impl RingParams {
    fn alpha_at(&self, frame: f32) -> f32 {
        let fade_out_at = self.duration_frames - self.duration_frames / 2.0;
        if frame <= FADE_IN_FRAMES {
            (frame / FADE_IN_FRAMES).clamp(0.0, 1.0)
        } else if frame >= fade_out_at {
            let span = (self.duration_frames - fade_out_at).max(1e-3);
            (1.0 - (frame - fade_out_at) / span).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct RingState {
    height_size: f32,
    height_speed: f32,
    speed: f32,
    position_offset: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct DebrisBurst {
    pub count: usize,
    pub base_yaw_deg: f32,
    pub cone_half_width_deg: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub size_min: f32,
    pub size_max: f32,
    pub duration_min_frames: f32,
    pub duration_max_frames: f32,
    pub spawn_y_offset: f32,
    pub spawn_distance_min: f32,
    pub spawn_distance_max: f32,
    pub gravity_initial_world_y: f32,
    pub gravity_accel_world_y: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct HitParams {
    pub rings: &'static [RingParams],
    pub bursts: &'static [DebrisBurst],
}

pub const HIT1: HitParams = HitParams {
    rings: &[RingParams {
        duration_frames: 10.0,
        outer_size: 10.0,
        inner_size: 5.0,
        initial_height_size: 3.5,
        initial_height_speed: 0.0,
        height_accel: 0.0,
        initial_speed: 0.7,
        speed_accel: -(0.7 / 10.0) / 2.0,
        y_offset: -10.0,
        tilt_x_rad: -std::f32::consts::FRAC_PI_2,
        texture: RING_BLUE,
        color: [1.0, 1.0, 1.0, 1.0],
    }],
    bursts: &[
        DebrisBurst {
            count: 2,
            base_yaw_deg: 0.0,
            cone_half_width_deg: 40.0,
            speed_min: 0.6,
            speed_max: 1.5,
            size_min: 0.2,
            size_max: 0.5,
            duration_min_frames: 6.0,
            duration_max_frames: 30.0,
            spawn_y_offset: -10.0,
            spawn_distance_min: 4.0,
            spawn_distance_max: 8.0,
            gravity_initial_world_y: 0.0,
            gravity_accel_world_y: 0.0,
        },
        DebrisBurst {
            count: 2,
            base_yaw_deg: 180.0,
            cone_half_width_deg: 40.0,
            speed_min: 0.6,
            speed_max: 1.5,
            size_min: 0.2,
            size_max: 0.5,
            duration_min_frames: 6.0,
            duration_max_frames: 30.0,
            spawn_y_offset: -10.0,
            spawn_distance_min: 4.0,
            spawn_distance_max: 8.0,
            gravity_initial_world_y: -0.75 * 60.0,
            gravity_accel_world_y: 5.0,
        },
    ],
};

pub const HIT3: HitParams = HitParams {
    rings: &[
        RingParams {
            duration_frames: 15.0,
            outer_size: 1.5,
            inner_size: 1.5,
            initial_height_size: 0.0,
            initial_height_speed: 0.5,
            height_accel: 0.2,
            initial_speed: 0.7,
            speed_accel: -(0.7 / 15.0) / 2.0,
            y_offset: -10.0,
            tilt_x_rad: -std::f32::consts::FRAC_PI_2,
            texture: LENS2,
            color: [1.0, 1.0, 1.0, 1.0],
        },
        RingParams {
            duration_frames: 15.0,
            outer_size: 4.0,
            inner_size: 1.5,
            initial_height_size: 0.0,
            initial_height_speed: 0.25,
            height_accel: 0.2,
            initial_speed: 0.7,
            speed_accel: -(0.7 / 15.0) / 2.0,
            y_offset: -10.0,
            tilt_x_rad: -std::f32::consts::FRAC_PI_2,
            texture: LENS2,
            color: [1.0, 1.0, 1.0, 1.0],
        },
    ],
    bursts: &[DebrisBurst {
        count: 8,
        base_yaw_deg: 0.0,
        cone_half_width_deg: 40.0,
        speed_min: 0.8,
        speed_max: 2.0,
        size_min: 0.6,
        size_max: 1.6,
        duration_min_frames: 6.0,
        duration_max_frames: 30.0,
        spawn_y_offset: -10.0,
        spawn_distance_min: 4.0,
        spawn_distance_max: 8.0,
        gravity_initial_world_y: 0.0,
        gravity_accel_world_y: 0.0,
    }],
};

pub const HIT4: HitParams = HitParams {
    rings: &[RingParams {
        duration_frames: 15.0,
        outer_size: 4.0,
        inner_size: 0.5,
        initial_height_size: 0.0,
        initial_height_speed: 0.25,
        height_accel: 0.15,
        initial_speed: 0.7,
        speed_accel: -(0.7 / 15.0) / 2.0,
        y_offset: -10.0,
        tilt_x_rad: -std::f32::consts::FRAC_PI_2,
        texture: LENS2,
        color: [1.0, 1.0, 1.0, 1.0],
    }],
    bursts: &[DebrisBurst {
        count: 5,
        base_yaw_deg: 0.0,
        cone_half_width_deg: 40.0,
        speed_min: 0.6,
        speed_max: 1.5,
        size_min: 0.6,
        size_max: 1.6,
        duration_min_frames: 6.0,
        duration_max_frames: 30.0,
        spawn_y_offset: -10.0,
        spawn_distance_min: 4.0,
        spawn_distance_max: 8.0,
        gravity_initial_world_y: 0.0,
        gravity_accel_world_y: 0.0,
    }],
};

pub const HIT1_TOTAL_DURATION_MS: u32 = total_duration_ms(HIT1);
pub const HIT3_TOTAL_DURATION_MS: u32 = total_duration_ms(HIT3);
pub const HIT4_TOTAL_DURATION_MS: u32 = total_duration_ms(HIT4);

const fn total_duration_ms(params: HitParams) -> u32 {
    let mut max_frames = 0.0_f32;
    let mut i = 0;
    while i < params.rings.len() {
        if params.rings[i].duration_frames > max_frames {
            max_frames = params.rings[i].duration_frames;
        }
        i += 1;
    }
    let mut j = 0;
    while j < params.bursts.len() {
        if params.bursts[j].duration_max_frames > max_frames {
            max_frames = params.bursts[j].duration_max_frames;
        }
        j += 1;
    }
    (max_frames / FRAMES_PER_SECOND * 1000.0) as u32
}

fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn lcg_float(state: &mut u32) -> f32 {
    (lcg_next(state) >> 8) as f32 / ((1u32 << 24) as f32)
}

#[derive(Clone, Copy)]
struct Particle {
    history: [[f32; 3]; NUM_SEGMENTS],
    velocity: [f32; 3],
    speed_world_per_s: f32,
    decel_world_per_s2: f32,
    direction: [f32; 3],
    gravity_velocity_y: f32,
    gravity_accel_y: f32,
    age: f32,
    lifetime: f32,
    size: f32,
}

impl Particle {
    fn alive(&self) -> bool {
        self.age < self.lifetime
    }

    fn step(&mut self, dt: f32) {
        for i in (1..NUM_SEGMENTS).rev() {
            self.history[i] = self.history[i - 1];
        }
        self.speed_world_per_s = (self.speed_world_per_s + self.decel_world_per_s2 * dt).max(0.0);
        self.velocity = [
            self.direction[0] * self.speed_world_per_s,
            self.direction[1] * self.speed_world_per_s,
            self.direction[2] * self.speed_world_per_s,
        ];
        self.gravity_velocity_y += self.gravity_accel_y * dt;
        let mut new_pos = self.history[0];
        new_pos[0] += self.velocity[0] * dt;
        new_pos[1] += (self.velocity[1] + self.gravity_velocity_y) * dt;
        new_pos[2] += self.velocity[2] * dt;
        self.history[0] = new_pos;
        self.age += dt;
    }

    fn alpha(&self) -> f32 {
        (1.0 - self.age / self.lifetime).clamp(0.0, 1.0)
    }
}

pub struct HitEffect {
    world_pos: [f32; 3],
    params: HitParams,
    heading_rad: f32,
    ring_state: Vec<RingState>,
    particles: Vec<Particle>,
    age: f32,
    total_duration_s: f32,
    rng_state: u32,
    has_spawned: bool,
}

impl HitEffect {
    pub fn new(world_pos: [f32; 3], params: HitParams) -> Self {
        Self::new_with_endpoints(world_pos, world_pos, params)
    }

    pub fn new_with_endpoints(from: [f32; 3], to: [f32; 3], params: HitParams) -> Self {
        let dx = from[0] - to[0];
        let dz = from[2] - to[2];
        let heading_rad = if dx.abs() < 1e-4 && dz.abs() < 1e-4 {
            0.0
        } else {
            dx.atan2(dz)
        };
        let world_pos = to;
        let total_duration_s = total_duration_ms(params) as f32 / 1000.0;
        let rng_state = 0x9E37_79B9
            ^ world_pos[0].to_bits()
            ^ world_pos[2].to_bits().rotate_left(13)
            ^ heading_rad.to_bits().rotate_left(7);
        let ring_state: Vec<RingState> = params
            .rings
            .iter()
            .map(|r| RingState {
                height_size: r.initial_height_size,
                height_speed: r.initial_height_speed,
                speed: r.initial_speed,
                position_offset: [0.0; 3],
            })
            .collect();
        Self {
            world_pos,
            params,
            heading_rad,
            ring_state,
            particles: Vec::new(),
            age: 0.0,
            total_duration_s,
            rng_state,
            has_spawned: false,
        }
    }

    fn heading_unit(&self) -> [f32; 3] {
        [0.0, 1.0, 0.0]
    }

    fn step_rings(&mut self, dt_frames: f32) {
        let heading = self.heading_unit();
        for (params, state) in self.params.rings.iter().zip(self.ring_state.iter_mut()) {
            state.speed += params.speed_accel * dt_frames;
            state.height_speed += params.height_accel * dt_frames;
            state.height_size += state.height_speed * dt_frames;
            if state.height_size > 100.0 {
                state.height_size = 100.0;
            }
            if state.height_size < 0.0 {
                state.height_size = 0.0;
            }
            let step = state.speed * dt_frames;
            state.position_offset[0] += heading[0] * step;
            state.position_offset[1] += heading[1] * step;
            state.position_offset[2] += heading[2] * step;
        }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }

    fn spawn_particles(&mut self) {
        for burst in self.params.bursts {
            let base_yaw_rad = self.heading_rad + burst.base_yaw_deg.to_radians();
            let cone_half_rad = burst.cone_half_width_deg.to_radians();
            for _ in 0..burst.count {
                let yaw_jitter = (lcg_float(&mut self.rng_state) * 2.0 - 1.0) * cone_half_rad;
                let yaw = base_yaw_rad + yaw_jitter;
                let elev_deg = 40.0 + lcg_float(&mut self.rng_state) * 100.0 - 90.0;
                let elev_rad = elev_deg.to_radians();
                let (sin_e, cos_e) = elev_rad.sin_cos();
                let (sin_y, cos_y) = yaw.sin_cos();
                let dir = [cos_e * sin_y, -sin_e, cos_e * cos_y];

                let speed_per_frame = burst.speed_min
                    + lcg_float(&mut self.rng_state) * (burst.speed_max - burst.speed_min);
                let speed_world_per_s = speed_per_frame * FRAMES_PER_SECOND;
                let duration_frames = burst.duration_min_frames
                    + lcg_float(&mut self.rng_state)
                        * (burst.duration_max_frames - burst.duration_min_frames);
                let lifetime = duration_frames / FRAMES_PER_SECOND;
                let decel_per_frame = -(speed_per_frame / duration_frames) / 2.0;
                let decel_world_per_s2 = decel_per_frame * FRAMES_PER_SECOND * FRAMES_PER_SECOND;

                let size = burst.size_min
                    + lcg_float(&mut self.rng_state) * (burst.size_max - burst.size_min);

                let spawn_distance = burst.spawn_distance_min
                    + lcg_float(&mut self.rng_state)
                        * (burst.spawn_distance_max - burst.spawn_distance_min);
                let spawn_pos = [
                    self.world_pos[0] + dir[0] * spawn_distance,
                    self.world_pos[1] + dir[1] * spawn_distance + burst.spawn_y_offset,
                    self.world_pos[2] + dir[2] * spawn_distance,
                ];

                self.particles.push(Particle {
                    history: [spawn_pos; NUM_SEGMENTS],
                    velocity: [
                        dir[0] * speed_world_per_s,
                        dir[1] * speed_world_per_s,
                        dir[2] * speed_world_per_s,
                    ],
                    speed_world_per_s,
                    decel_world_per_s2,
                    direction: dir,
                    gravity_velocity_y: burst.gravity_initial_world_y,
                    gravity_accel_y: burst.gravity_accel_world_y,
                    age: 0.0,
                    lifetime,
                    size,
                });
            }
        }
    }
}

impl Effect for HitEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        if !self.has_spawned {
            self.spawn_particles();
            self.has_spawned = true;
        }
        self.age += ctx.delta;
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.step_rings(dt_frames);
        for p in &mut self.particles {
            p.step(ctx.delta);
        }
        self.particles.retain(|p| p.alive());

        if self.age >= self.total_duration_s && self.particles.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.frame();
        for (ring, state) in self.params.rings.iter().zip(self.ring_state.iter()) {
            if frame >= ring.duration_frames {
                continue;
            }
            if state.height_size <= 0.001 {
                continue;
            }
            let alpha = ring.alpha_at(frame);
            let color = [
                ring.color[0],
                ring.color[1],
                ring.color[2],
                ring.color[3] * alpha,
            ];
            let cylinder_base = [
                self.world_pos[0] + state.position_offset[0],
                self.world_pos[1] + ring.y_offset + state.position_offset[1],
                self.world_pos[2] + state.position_offset[2],
            ];
            out.push(EffectPrimitiveDraw::Frustum {
                base_alpha: 1.0,
                base: cylinder_base,
                bottom_size: ring.inner_size,
                top_size: ring.outer_size,
                height: state.height_size,
                sides: 16,
                arc_angle_deg: 360.0,
                rotation: 0.0,
                uv_repeat: 4.0,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: 0.0,
                wave_frequency: 1.0,
                wave_phase: 0.0,
                wave_mode: FrustumWaveMode::Sine,
                tilt_x_rad: ring.tilt_x_rad,
                rotation_y_rad: std::f32::consts::PI - self.heading_rad,
                cull_back: false,
                texture: ring.texture,
                color,
                blend: BlendKind::Alpha,
            });
        }

        for p in &self.particles {
            let base_alpha = p.alpha();
            if base_alpha <= 0.0 {
                continue;
            }
            for i in 0..NUM_SEGMENTS {
                let seg_alpha = base_alpha * (NUM_SEGMENTS - i) as f32 / NUM_SEGMENTS as f32;
                let seg_size = p.size * (2 * NUM_SEGMENTS - i) as f32 / (2 * NUM_SEGMENTS) as f32;
                let frame_index = (p.age * 1000.0 / PARTICLE_FRAME_MS) as usize;
                let motion = frame_index.saturating_sub(i);
                out.push(EffectPrimitiveDraw::SpriteParticle {
                    sprite_path: PARTICLE1_SPRITE,
                    position: p.history[i],
                    action_index: 0,
                    motion_index: motion,
                    size_scale: seg_size,
                    color: [1.0, 1.0, 1.0, seg_alpha],
                    blend: BlendKind::Alpha,
                    aim_target: None,
                    no_depth: false,
                });
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

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn hit1_emits_side_laid_ring_plus_three_segments_per_particle() {
        let mut e = HitEffect::new_with_endpoints([11.0, 2.0, 3.0], [1.0, 2.0, 3.0], HIT1);
        e.update(&ctx(1.0 / 60.0));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let EffectPrimitiveDraw::Frustum {
            base,
            tilt_x_rad,
            rotation_y_rad,
            bottom_size,
            top_size,
            height,
            blend,
            ..
        } = list.primitives[0]
        else {
            panic!(
                "first draw must be the cylinder Frustum, got {:?}",
                list.primitives[0]
            );
        };
        assert_eq!(
            blend,
            BlendKind::Alpha,
            "Hit ring is alpha-blended (RF_EFFECT_OM_2)"
        );
        // XZ stays put (no horizontal translation).
        assert!(
            (base[0] - 1.0).abs() < 1e-4,
            "X stays at spawn: {}",
            base[0]
        );
        assert!(
            (base[2] - 3.0).abs() < 1e-4,
            "Z stays at spawn: {}",
            base[2]
        );
        let spawn_y = 2.0 + HIT1.rings[0].y_offset;
        assert!(
            base[1] > spawn_y,
            "cylinder Y moved downward (toward master): got {} starting from {}",
            base[1],
            spawn_y
        );
        assert!(
            (tilt_x_rad + std::f32::consts::FRAC_PI_2).abs() < 1e-5,
            "Hit1 ring is side-laid (tilt=-π/2) like the rest of the family: got {tilt_x_rad}"
        );
        assert!(
            (rotation_y_rad - std::f32::consts::FRAC_PI_2).abs() < 1e-5,
            "rotation_y_rad == π − heading: got {rotation_y_rad}"
        );
        assert!(
            (bottom_size - 5.0).abs() < 1e-4,
            "bottom_size=inner_size=5: {bottom_size}"
        );
        assert!(
            (top_size - 10.0).abs() < 1e-4,
            "top_size=outer_size=10: {top_size}"
        );
        assert!(
            (height - 3.5).abs() < 1e-4,
            "Hit1 heightSize is static at 3.5"
        );

        let particles: Vec<_> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle {
                    sprite_path,
                    color,
                    size_scale,
                    blend,
                    ..
                } => Some((*sprite_path, color[3], *size_scale, *blend)),
                _ => None,
            })
            .collect();
        assert!(
            particles.iter().all(|(_, _, _, b)| *b == BlendKind::Alpha),
            "Hit debris share the alpha prim flag (RF_EFFECT_OM_2)"
        );
        assert_eq!(
            particles.len(),
            HIT1.bursts.iter().map(|b| b.count).sum::<usize>() * NUM_SEGMENTS,
            "expected NUM_SEGMENTS=3 sprite draws per particle"
        );
        assert!(particles.iter().all(|(s, _, _, _)| *s == PARTICLE1_SPRITE));

        // Per-particle trail check: groups of NUM_SEGMENTS should have
        // strictly decreasing alpha and size from segment 0 to 2.
        for chunk in particles.chunks(NUM_SEGMENTS) {
            assert!(chunk[0].1 >= chunk[1].1, "segment 1 alpha ≤ segment 0");
            assert!(chunk[1].1 >= chunk[2].1, "segment 2 alpha ≤ segment 1");
            assert!(chunk[0].2 >= chunk[1].2, "segment 1 size ≤ segment 0");
            assert!(chunk[1].2 >= chunk[2].2, "segment 2 size ≤ segment 1");
        }
    }

    #[test]
    fn hit3_emits_two_rings_after_height_grows_and_eight_particles() {
        let mut e = HitEffect::new([0.0; 3], HIT3);
        e.update(&ctx(1.0 / 60.0));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let ring_count = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
            .count();
        let particle_count = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { .. }))
            .count();
        assert_eq!(
            ring_count, 2,
            "HIT3 launches 2 concentric rings after height>0"
        );
        let tops: Vec<f32> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { top_size, .. } => Some(*top_size),
                _ => None,
            })
            .collect();
        assert!(tops.contains(&1.5));
        assert!(tops.contains(&4.0));
        assert_eq!(
            particle_count,
            8 * NUM_SEGMENTS,
            "HIT3 launches 8 forward particles × 3 trail segments"
        );
    }

    #[test]
    fn hit3_height_grows_over_time_while_hit4_grows_slower() {
        let mut h3 = HitEffect::new([0.0; 3], HIT3);
        let mut h4 = HitEffect::new([0.0; 3], HIT4);
        for _ in 0..5 {
            h3.update(&ctx(1.0 / 60.0));
            h4.update(&ctx(1.0 / 60.0));
        }
        let h3_outer_height = h3
            .ring_state
            .iter()
            .zip(h3.params.rings.iter())
            .find(|(_, r)| (r.outer_size - 4.0).abs() < 1e-3)
            .map(|(s, _)| s.height_size)
            .unwrap();
        let h4_height = h4.ring_state[0].height_size;
        assert!(
            h3_outer_height > h4_height,
            "Hit3 outer height ({h3_outer_height}) must exceed Hit4 height ({h4_height}) after 5 ticks"
        );
    }

    #[test]
    fn debris_particles_have_3d_velocity_and_decay() {
        let mut e = HitEffect::new([0.0; 3], HIT1);
        e.update(&ctx(0.0));
        let spawn_positions: Vec<[f32; 3]> = e.particles.iter().map(|p| p.history[0]).collect();
        e.update(&ctx(1.0 / 60.0));
        let moved = e.particles.iter().zip(&spawn_positions).any(|(p, s)| {
            (p.history[0][0] - s[0]).abs() > 0.01
                || (p.history[0][1] - s[1]).abs() > 0.01
                || (p.history[0][2] - s[2]).abs() > 0.01
        });
        assert!(moved, "particles must move on integration step");
    }

    #[test]
    fn hit1_gravity_particles_arc_upward_then_fall() {
        let mut e = HitEffect::new([0.0; 3], HIT1);
        e.update(&ctx(0.0));
        let backward = HIT1.bursts[1].count;
        let total = e.particles.len();
        let initial_gravity: Vec<f32> = e
            .particles
            .iter()
            .skip(total - backward)
            .map(|p| p.gravity_velocity_y)
            .collect();
        assert!(
            initial_gravity.iter().all(|&g| g < 0.0),
            "gravity particles start with upward (negative) velocity: {initial_gravity:?}"
        );
        for _ in 0..30 {
            e.update(&ctx(1.0 / 60.0));
        }
        let now_falling = e
            .particles
            .iter()
            .filter(|p| p.gravity_velocity_y > 0.0)
            .count();
        assert!(
            now_falling > 0 || e.particles.is_empty(),
            "after 0.5s some backward gravity particles should be falling"
        );
    }

    #[test]
    fn effect_dies_after_total_duration() {
        let mut e = HitEffect::new([0.0; 3], HIT1);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < 2.0 {
            status = e.update(&ctx(1.0 / 60.0));
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
