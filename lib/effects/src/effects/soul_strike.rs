use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const SOUL_STRIKE_SPRITE: &str = "data/sprite/이팩트/particle1";
pub const SOUL_STRIKE2_SPRITE: &str = "data/sprite/이팩트/particle5";
pub const SPRITES: &[&str] = &[SOUL_STRIKE_SPRITE, SOUL_STRIKE2_SPRITE];

const FPS: f32 = 60.0;
const FRAME_DT: f32 = 1.0 / FPS;

const BOLT_DURATION_FRAMES: u32 = 40;
const EFFECT_DURATION_FRAMES: u32 = 100;
pub const TOTAL_DURATION_MS: u32 = (EFFECT_DURATION_FRAMES as f32 / FPS * 1000.0) as u32;

const NUM_SEGMENT: usize = 12;
const SIZE: f32 = 2.75;
const GRAV_SPEED_INIT: f32 = 3.5;
const LATI_SPEED_INIT: f32 = -2.0;
const ANIM_SPEED: u32 = 4;
const Y_OFFSET: f32 = -10.0;

const SPAWN_DIST: u32 = 11;
const SPAWN_DELAY: u32 = 5;

pub const PROJECTILE_FLIGHT: crate::effect_queue::ProjectileFlight =
    crate::effect_queue::ProjectileFlight::FixedFrames(
        (SPAWN_DIST - SPAWN_DELAY + BOLT_DURATION_FRAMES) as f32,
    );

const STATIC_DURATION_S: f32 = 60.0 / FPS;
const STATIC_KILL_DIST: f32 = 3.0;

fn angle_config(hit_count: u8) -> (i32, i32) {
    match hit_count {
        1 => (0, 1),
        2 => (-90, 180),
        3 => (-90, 90),
        4 => (-90, 60),
        5 => (-90, 45),
        _ => (-90, 45),
    }
}

pub fn roll_angle_deg(hit_count: u8, bolt_index: u8) -> f32 {
    let (start, step) = angle_config(hit_count);
    (start + (bolt_index as i32 + 1) * step) as f32
}

fn bolt_spawn_frame(bolt_index: u8) -> u32 {
    (bolt_index as u32 + 1) * SPAWN_DIST - SPAWN_DELAY
}

struct SoulStrikeBolt {
    sin_lon: f32,
    cos_lon: f32,
    roll_rad: f32,
    forward_speed: f32,

    grav_speed: f32,
    grav_accel: f32,
    latitude: f32,
    lati_speed: f32,
    lati_accel: f32,
    delta_pos: [f32; 3],

    segments: [[f32; 3]; NUM_SEGMENT],
    frame_count: u32,
}

impl SoulStrikeBolt {
    fn new(org_pos: [f32; 3], radius: f32, dx: f32, dz: f32, roll_deg: f32) -> Self {
        let roty = dx.atan2(-dz);
        let longitude = -roty;
        let sin_lon = longitude.sin();
        let cos_lon = longitude.cos();
        let roll_rad = roll_deg.to_radians();

        let forward_speed = radius / BOLT_DURATION_FRAMES as f32;
        let grav_accel = -(GRAV_SPEED_INIT / BOLT_DURATION_FRAMES as f32) * 2.0;
        let lati_accel = -(LATI_SPEED_INIT / BOLT_DURATION_FRAMES as f32) * 2.0;

        Self {
            sin_lon,
            cos_lon,
            roll_rad,
            forward_speed,
            grav_speed: GRAV_SPEED_INIT,
            grav_accel,
            latitude: 0.0,
            lati_speed: LATI_SPEED_INIT,
            lati_accel,
            delta_pos: [0.0; 3],
            segments: [org_pos; NUM_SEGMENT],
            frame_count: 0,
        }
    }

    fn step(&mut self, org_pos: [f32; 3]) {
        self.latitude += self.lati_speed;
        self.lati_speed += self.lati_accel;
        self.grav_speed += self.grav_accel;

        let fwd = self.forward_speed - self.grav_speed;
        let speed3d = [fwd * self.sin_lon, 0.0, fwd * self.cos_lon];

        let sin_roll = self.roll_rad.sin();
        let cos_roll = self.roll_rad.cos();
        let delta_pos3 = [
            -self.cos_lon * self.latitude * sin_roll,
            self.latitude * cos_roll,
            self.sin_lon * self.latitude * sin_roll,
        ];

        self.delta_pos[0] -= speed3d[0];
        self.delta_pos[1] -= speed3d[1];
        self.delta_pos[2] -= speed3d[2];

        let pos = [
            org_pos[0] + self.delta_pos[0] + delta_pos3[0],
            org_pos[1] + self.delta_pos[1] + delta_pos3[1],
            org_pos[2] + self.delta_pos[2] + delta_pos3[2],
        ];

        for i in (1..NUM_SEGMENT).rev() {
            self.segments[i] = self.segments[i - 1];
        }
        self.segments[0] = pos;

        self.frame_count += 1;
    }

    fn alive(&self) -> bool {
        self.frame_count <= BOLT_DURATION_FRAMES
    }
}

pub struct SoulStrikeEffect {
    from: [f32; 3],
    org_pos: [f32; 3],
    radius: f32,
    dx: f32,
    dz: f32,
    hit_count: u8,
    sprite: &'static str,
    is_trail: bool,

    effect_frame: u32,
    time_accum: f32,
    age: f32,

    bolts: Vec<SoulStrikeBolt>,
    next_bolt_index: u8,
}

impl SoulStrikeEffect {
    pub fn new(from: [f32; 3], to: [f32; 3], hit_count: u8) -> Self {
        Self::with_sprite(from, to, hit_count, SOUL_STRIKE_SPRITE)
    }

    pub fn with_sprite(from: [f32; 3], to: [f32; 3], hit_count: u8, sprite: &'static str) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let radius = (dx * dx + dz * dz).sqrt();
        let is_trail = radius > STATIC_KILL_DIST;
        let org_pos = [from[0], from[1] + Y_OFFSET, from[2]];

        Self {
            from,
            org_pos,
            radius,
            dx,
            dz,
            hit_count: hit_count.clamp(1, 5),
            sprite,
            is_trail,
            effect_frame: 0,
            time_accum: 0.0,
            age: 0.0,
            bolts: Vec::with_capacity(5),
            next_bolt_index: 0,
        }
    }

    fn tick(&mut self) {
        self.effect_frame += 1;

        for bolt in &mut self.bolts {
            if bolt.alive() {
                bolt.step(self.org_pos);
            }
        }
        self.bolts.retain(|b| b.alive());

        while self.next_bolt_index < self.hit_count {
            let spawn_frame = bolt_spawn_frame(self.next_bolt_index);
            if self.effect_frame >= spawn_frame {
                let roll = roll_angle_deg(self.hit_count, self.next_bolt_index);
                self.bolts.push(SoulStrikeBolt::new(
                    self.org_pos,
                    self.radius,
                    self.dx,
                    self.dz,
                    roll,
                ));
                self.next_bolt_index += 1;
            } else {
                break;
            }
        }
    }
}

impl Effect for SoulStrikeEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;

        if !self.is_trail {
            return if self.age >= STATIC_DURATION_S {
                EffectStatus::Dead
            } else {
                EffectStatus::Running
            };
        }

        self.time_accum += ctx.delta;
        while self.time_accum >= FRAME_DT {
            self.time_accum -= FRAME_DT;
            self.tick();
        }

        let all_spawned = self.next_bolt_index >= self.hit_count;
        if all_spawned && self.bolts.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.is_trail {
            let fn_seg = NUM_SEGMENT as f32;
            for bolt in &self.bolts {
                let motion = (bolt.frame_count / ANIM_SPEED) as usize;
                for i in 0..NUM_SEGMENT {
                    let fi = i as f32;
                    let alpha = 1.0 - fi / fn_seg;
                    let size = SIZE * (1.0 - fi / (2.0 * fn_seg));

                    out.push(EffectPrimitiveDraw::SpriteParticle {
                        sprite_path: self.sprite,
                        position: bolt.segments[i],
                        action_index: 0,
                        motion_index: motion,
                        size_scale: size,
                        color: [1.0, 1.0, 1.0, alpha],
                        blend: BlendKind::Additive,
                        aim_target: None,
                        no_depth: false,
                    });
                }
            }
        } else {
            let t = (self.age / STATIC_DURATION_S).clamp(0.0, 1.0);
            let alpha = if t < 0.7 {
                1.0
            } else {
                (1.0 - (t - 0.7) / 0.3).clamp(0.0, 1.0)
            };
            let scale = SIZE * (1.0 + t * 0.4);
            let motion = (self.age * FPS / ANIM_SPEED as f32) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: self.sprite,
                position: self.from,
                action_index: 0,
                motion_index: motion,
                size_scale: scale,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
                aim_target: None,
                no_depth: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut SoulStrikeEffect, dt: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn draws(e: &SoulStrikeEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn bolt_spawn_timing_matches_formula() {
        let mut e = SoulStrikeEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 80.0], 3);
        let dt = FRAME_DT;

        for _ in 0..5 {
            step(&mut e, dt);
        }
        assert_eq!(e.bolts.len(), 0, "no bolts before frame 6");

        step(&mut e, dt);
        assert_eq!(e.bolts.len(), 1, "first bolt spawns at frame 6");

        for _ in 0..11 {
            step(&mut e, dt);
        }
        assert_eq!(e.bolts.len(), 2, "second bolt by frame 17");

        for _ in 0..11 {
            step(&mut e, dt);
        }
        assert_eq!(e.next_bolt_index, 3, "all 3 bolts spawned by frame 28");
    }

    #[test]
    fn bolt_count_matches_hit_count() {
        for count in 1..=5u8 {
            let mut e = SoulStrikeEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 80.0], count);
            let dt = FRAME_DT;
            for _ in 0..60 {
                step(&mut e, dt);
            }
            assert_eq!(
                e.next_bolt_index, count,
                "hit_count={count} should spawn {count} bolts"
            );
        }
    }

    #[test]
    fn soulstrike2_uses_red_particle5_sprite() {
        // Soulstrike2: same bolt motion, red particle5 sprite.
        let mut e = SoulStrikeEffect::with_sprite(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 80.0],
            1,
            SOUL_STRIKE2_SPRITE,
        );
        for _ in 0..20 {
            step(&mut e, FRAME_DT);
        }
        for p in draws(&e) {
            let EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } = p else {
                panic!("expected SpriteParticle");
            };
            assert_eq!(sprite_path, SOUL_STRIKE2_SPRITE);
        }
    }

    #[test]
    fn trail_emits_12_segments_per_bolt() {
        let mut e = SoulStrikeEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 80.0], 1);
        let dt = FRAME_DT;
        for _ in 0..20 {
            step(&mut e, dt);
        }
        let d = draws(&e);
        assert_eq!(d.len(), NUM_SEGMENT, "12 segments per bolt");

        let sizes: Vec<f32> = d
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { size_scale, .. } => *size_scale,
                _ => panic!("expected SpriteParticle"),
            })
            .collect();
        assert!(
            sizes[0] > sizes[sizes.len() - 1],
            "lead segment bigger than tail"
        );
    }

    #[test]
    fn bolt_reaches_target_by_end_of_duration() {
        let target_z = 60.0;
        let mut e = SoulStrikeEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, target_z], 1);
        let dt = FRAME_DT;
        for _ in 0..30 {
            step(&mut e, dt);
        }
        let z_mid = match &draws(&e)[0] {
            EffectPrimitiveDraw::SpriteParticle { position, .. } => position[2],
            _ => panic!("expected SpriteParticle"),
        };
        for _ in 0..15 {
            step(&mut e, dt);
        }
        let z_late = match &draws(&e)[0] {
            EffectPrimitiveDraw::SpriteParticle { position, .. } => position[2],
            _ => panic!("expected SpriteParticle"),
        };
        assert!(
            z_late > z_mid,
            "bolt should advance toward target in second half: mid={z_mid} late={z_late}"
        );
        assert!(
            z_late > target_z * 0.3,
            "bolt should be well past 30% of the way by frame 45: {z_late}"
        );
    }

    #[test]
    fn angle_spread_for_5_hits() {
        let mut angles: Vec<f32> = (0..5).map(|i| roll_angle_deg(5, i)).collect();
        angles.sort_by(|a, b| a.partial_cmp(b).unwrap());
        angles.dedup();
        assert_eq!(angles.len(), 5, "5-hit should produce 5 distinct angles");
        assert_eq!(angles, vec![-45.0, 0.0, 45.0, 90.0, 135.0]);
    }

    #[test]
    fn effect_dies_after_all_bolts_complete() {
        let mut e = SoulStrikeEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 60.0], 3);
        let dt = FRAME_DT;
        let mut status = EffectStatus::Running;
        for _ in 0..200 {
            status = step(&mut e, dt);
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn static_fallback_renders_single_sprite_then_dies() {
        let mut e = SoulStrikeEffect::new([5.0, 0.0, 7.0], [5.0, 0.0, 7.0], 1);
        step(&mut e, 0.0);
        assert_eq!(draws(&e).len(), 1);
        let mut status = EffectStatus::Running;
        for _ in 0..120 {
            status = step(&mut e, FRAME_DT);
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn multiple_bolts_have_lateral_spread() {
        let mut e = SoulStrikeEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 80.0], 5);
        let dt = FRAME_DT;
        for _ in 0..55 {
            step(&mut e, dt);
        }
        let lead_positions: Vec<[f32; 3]> = e.bolts.iter().map(|b| b.segments[0]).collect();
        assert!(
            lead_positions.len() >= 2,
            "need at least 2 active bolts for spread check"
        );
        let y_values: Vec<f32> = lead_positions.iter().map(|p| p[1]).collect();
        let y_range = y_values.iter().cloned().fold(f32::MAX, f32::min)
            - y_values.iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            y_range.abs() > 0.1,
            "bolts with different roll angles should spread in Y: {y_values:?}"
        );
    }
}
