use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{BodyCopy, BodyTint, Effect, EffectRenderCtx, EffectUpdateCtx};

pub const DRAIN_SPRITE: &str = ragnarok_resources::sprite::effect::PARTICLE1;
pub const SPRITES: &[&str] = &[DRAIN_SPRITE];

const FPS: f32 = 60.0;
const FRAME_DT: f32 = 1.0 / FPS;

const DURATION_FRAMES: u32 = 60;
const ANIM_SPEED: u32 = 4;

const DELTA_POS2_Y: f32 = -10.0;

const NUM_SEGMENT: usize = 7;
const GRAV_SPEED_INIT: f32 = 2.0;
const LATI_SPEED_INIT: f32 = -2.0;
const ROLLS_DEG: [f32; 3] = [-90.0, 0.0, 90.0];

const SIZE_STEP: f32 = 0.1;

const SCATTER_SPAWN_THROUGH: u32 = 4;
const SCATTER_SPREAD: u32 = 7;
const SCATTER_CENTRE: f32 = 3.0;

const HALO_MARGIN_BASE: f32 = 5.0;
const HALO_MARGIN_SWING: f32 = 1.5;
const HALO_PERIOD_DEG: u32 = 181;

#[derive(Clone, Copy, PartialEq)]
pub enum DrainGeometry {
    /// One burst at the far end of the trail, travelling back to its start.
    Inward { radius_scale: f32 },
    /// Five bursts around the caster, spraying in random directions.
    Scatter,
}

impl DrainGeometry {
    const fn spawn_through(&self) -> u32 {
        match self {
            DrainGeometry::Inward { .. } => 0,
            DrainGeometry::Scatter => SCATTER_SPAWN_THROUGH,
        }
    }
}

#[derive(Clone, Copy)]
enum Chan {
    Fixed(f32),
    Rand { base: f32, span: u32 },
}

impl Chan {
    fn pick(&self, seed: u32) -> f32 {
        match *self {
            Chan::Fixed(v) => v / 255.0,
            Chan::Rand { base, span } => (base + rand_below(seed, span) as f32) / 255.0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct BodyRecolor {
    pub window: (u32, u32),
    pub rgb: Option<[f32; 3]>,
    pub additive: bool,
    pub halo: bool,
}

#[derive(Clone, Copy)]
pub struct DrainParams {
    rgb: [Chan; 3],
    size_base: f32,
    size_steps: u32,
    geometry: DrainGeometry,
    body_recolor: Option<BodyRecolor>,
}

impl DrainParams {
    pub const fn total_duration_ms(&self) -> u32 {
        let frames = self.geometry.spawn_through() + DURATION_FRAMES;
        (frames as f32 / FPS * 1000.0) as u32
    }
}

pub const BLOOD_DRAIN: DrainParams = DrainParams {
    rgb: [Chan::Fixed(250.0), Chan::Fixed(100.0), Chan::Fixed(100.0)],
    size_base: 1.7,
    size_steps: 0,
    geometry: DrainGeometry::Inward { radius_scale: 1.0 },
    body_recolor: None,
};

pub const ENERGY_DRAIN: DrainParams = DrainParams {
    rgb: [Chan::Fixed(100.0), Chan::Fixed(100.0), Chan::Fixed(250.0)],
    size_base: 1.7,
    size_steps: 0,
    geometry: DrainGeometry::Inward { radius_scale: 1.0 },
    body_recolor: None,
};

pub const ENERGY_DRAIN2: DrainParams = DrainParams {
    rgb: [
        Chan::Rand {
            base: 160.0,
            span: 81,
        },
        Chan::Rand {
            base: 160.0,
            span: 81,
        },
        Chan::Fixed(255.0),
    ],
    size_base: 1.5,
    size_steps: 6,
    geometry: DrainGeometry::Inward { radius_scale: 0.5 },
    body_recolor: Some(BodyRecolor {
        window: (55, 65),
        rgb: Some([100.0, 100.0, 255.0]),
        additive: true,
        halo: false,
    }),
};

pub const ENERGY_DRAIN3: DrainParams = DrainParams {
    rgb: [
        Chan::Rand {
            base: 160.0,
            span: 81,
        },
        Chan::Fixed(255.0),
        Chan::Rand {
            base: 160.0,
            span: 81,
        },
    ],
    size_base: 1.0,
    size_steps: 6,
    geometry: DrainGeometry::Scatter,
    body_recolor: Some(BodyRecolor {
        window: (50, 80),
        rgb: None,
        additive: false,
        halo: true,
    }),
};

pub fn hash01(seed: u32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E37_79B9).wrapping_add(0x6D2B_79F5);
    x ^= x >> 15;
    x = x.wrapping_mul(0x8589_45CD);
    x ^= x >> 13;
    (x & 0xFF_FFFF) as f32 / 0x100_0000 as f32
}

fn rand_below(seed: u32, n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    ((hash01(seed) * n as f32) as u32).min(n - 1)
}

struct DrainStrand {
    org_pos: [f32; 3],
    sin_lon: f32,
    cos_lon: f32,
    roll_rad: f32,
    speed: f32,
    grav_speed: f32,
    grav_accel: f32,
    latitude: f32,
    lati_speed: f32,
    lati_accel: f32,
    delta_pos: [f32; 3],
    segments: [[f32; 3]; NUM_SEGMENT],
    frame_count: u32,
    color: [f32; 3],
    size: f32,
}

impl DrainStrand {
    fn new(
        org_pos: [f32; 3],
        radius: f32,
        dx: f32,
        dz: f32,
        roll_deg: f32,
        color: [f32; 3],
        size: f32,
    ) -> Self {
        let longitude = -dx.atan2(-dz);
        let dur = DURATION_FRAMES as f32;
        Self {
            org_pos,
            sin_lon: longitude.sin(),
            cos_lon: longitude.cos(),
            roll_rad: roll_deg.to_radians(),
            speed: (radius / dur) * 2.0,
            grav_speed: GRAV_SPEED_INIT,
            grav_accel: -(GRAV_SPEED_INIT / dur) * 2.0,
            latitude: 0.0,
            lati_speed: LATI_SPEED_INIT,
            lati_accel: -(LATI_SPEED_INIT / dur) * 2.0,
            delta_pos: [0.0; 3],
            segments: [org_pos; NUM_SEGMENT],
            frame_count: 0,
            color,
            size,
        }
    }

    fn step(&mut self) {
        self.latitude += self.lati_speed;
        self.lati_speed += self.lati_accel;
        self.grav_speed += self.grav_accel;

        let fwd = self.speed - self.grav_speed;
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
            self.org_pos[0] + self.delta_pos[0] + delta_pos3[0],
            self.org_pos[1] + self.delta_pos[1] + delta_pos3[1],
            self.org_pos[2] + self.delta_pos[2] + delta_pos3[2],
        ];

        for i in (1..NUM_SEGMENT).rev() {
            self.segments[i] = self.segments[i - 1];
        }
        self.segments[0] = pos;
        self.frame_count += 1;
    }

    fn alive(&self) -> bool {
        self.frame_count <= DURATION_FRAMES
    }
}

pub struct DrainEffect {
    org_pos: [f32; 3],
    params: DrainParams,
    radius: f32,
    dir: [f32; 2],
    next_spawn_frame: u32,
    spawn_seed: u32,
    strands: Vec<DrainStrand>,

    effect_frame: u32,
    time_accum: f32,
}

impl DrainEffect {
    /// `from` is the trail's caster/attacker end, `to` the target end.
    pub fn new(from: [f32; 3], to: [f32; 3], params: DrainParams) -> Self {
        let (origin, radius, dir) = match params.geometry {
            DrainGeometry::Inward { radius_scale } => {
                let (dx, dz) = (from[0] - to[0], from[2] - to[2]);
                (to, (dx * dx + dz * dz).sqrt() * radius_scale, [dx, dz])
            }
            DrainGeometry::Scatter => (from, 0.0, [0.0, 0.0]),
        };

        Self {
            org_pos: [origin[0], origin[1] + DELTA_POS2_Y, origin[2]],
            params,
            radius,
            dir,
            next_spawn_frame: 0,
            spawn_seed: 0,
            strands: Vec::with_capacity(15),
            effect_frame: 0,
            time_accum: 0.0,
        }
    }

    fn spawn_burst(&mut self) {
        for &roll in &ROLLS_DEG {
            let seed = self.spawn_seed;
            self.spawn_seed += 1;

            let [dx, dz] = match self.params.geometry {
                DrainGeometry::Inward { .. } => self.dir,
                DrainGeometry::Scatter => [
                    rand_below(seed ^ 0x11, SCATTER_SPREAD) as f32 - SCATTER_CENTRE,
                    rand_below(seed ^ 0x22, SCATTER_SPREAD) as f32 - SCATTER_CENTRE,
                ],
            };

            let color = [
                self.params.rgb[0].pick(seed ^ 0x01),
                self.params.rgb[1].pick(seed ^ 0x02),
                self.params.rgb[2].pick(seed ^ 0x03),
            ];
            let size = self.params.size_base
                + rand_below(seed ^ 0x04, self.params.size_steps) as f32 * SIZE_STEP;

            self.strands.push(DrainStrand::new(
                self.org_pos,
                self.radius,
                dx,
                dz,
                roll,
                color,
                size,
            ));
        }
    }

    fn tick(&mut self) {
        for s in &mut self.strands {
            s.step();
        }
        self.strands.retain(|s| s.alive());

        while self.next_spawn_frame <= self.params.geometry.spawn_through()
            && self.effect_frame >= self.next_spawn_frame
        {
            self.spawn_burst();
            self.next_spawn_frame += 1;
        }
        self.effect_frame += 1;
    }

    fn recolor(&self) -> Option<BodyRecolor> {
        let r = self.params.body_recolor?;
        (r.window.0..=r.window.1)
            .contains(&self.effect_frame)
            .then_some(r)
    }
}

impl Effect for DrainEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.time_accum += ctx.delta;
        while self.time_accum >= FRAME_DT {
            self.time_accum -= FRAME_DT;
            self.tick();
        }

        let done_spawning = self.next_spawn_frame > self.params.geometry.spawn_through();
        if done_spawning && self.strands.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let fn_seg = NUM_SEGMENT as f32;
        for strand in &self.strands {
            let motion = (strand.frame_count / ANIM_SPEED) as usize;
            for i in 0..NUM_SEGMENT {
                let fi = i as f32;
                let alpha = 1.0 - fi / fn_seg;
                let size = strand.size * (1.0 - fi / (2.0 * fn_seg));
                out.push(EffectPrimitiveDraw::SpriteParticle {
                    sprite_path: DRAIN_SPRITE,
                    position: strand.segments[i],
                    action_index: 0,
                    motion_index: motion,
                    size_scale: size,
                    color: [strand.color[0], strand.color[1], strand.color[2], alpha],
                    blend: BlendKind::Additive,
                    aim_target: None,
                    no_depth: false,
                });
            }
        }
    }

    fn body_tint(&self) -> Option<BodyTint> {
        let r = self.recolor()?;
        let rgb = match r.rgb {
            Some(c) => [c[0] as u8, c[1] as u8, c[2] as u8],
            None => {
                let v = (250i32 - 2 * self.effect_frame as i32).clamp(0, 255) as u8;
                [v, v, 250]
            }
        };
        Some(BodyTint { rgb })
    }

    fn body_additive(&self) -> bool {
        self.recolor().is_some_and(|r| r.additive)
    }

    fn body_copies(&self) -> Option<Vec<BodyCopy>> {
        if !self.recolor()?.halo {
            return None;
        }
        let deg = (self.effect_frame % HALO_PERIOD_DEG) as f32;
        Some(vec![BodyCopy {
            offset_px: [0.0, 0.0],
            margin_px: deg.to_radians().sin() * HALO_MARGIN_SWING + HALO_MARGIN_BASE,
            scale: [1.0, 1.0],
            tint: self.body_tint().map_or([255, 255, 255], |t| t.rgb),
            alpha: 1.0,
            additive: false,
            behind: true,
            body_layers_only: false,
        }])
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

    fn run(e: &mut DrainEffect, frames: u32) -> EffectStatus {
        let mut status = EffectStatus::Running;
        for _ in 0..frames {
            status = e.update(&EffectUpdateCtx {
                delta: FRAME_DT,
                camera_target: None,
                caster_yaw: None,
            });
        }
        status
    }

    fn draws(e: &DrainEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn heads(e: &DrainEffect) -> Vec<[f32; 3]> {
        e.strands.iter().map(|s| s.segments[0]).collect()
    }

    fn dist2d(a: [f32; 3], b: [f32; 3]) -> f32 {
        ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    }

    #[test]
    fn every_drain_is_three_seven_segment_strands() {
        for params in [BLOOD_DRAIN, ENERGY_DRAIN, ENERGY_DRAIN2] {
            let mut e = DrainEffect::new([0.0, 0.0, 40.0], [0.0, 0.0, 0.0], params);
            run(&mut e, 1);
            assert_eq!(draws(&e).len(), 3 * NUM_SEGMENT);
        }
        let mut hp = DrainEffect::new([0.0; 3], [0.0; 3], ENERGY_DRAIN3);
        run(&mut hp, 6);
        assert_eq!(
            draws(&hp).len(),
            15 * NUM_SEGMENT,
            "3 strands x frames 0..=4"
        );
    }

    #[test]
    fn strands_leave_the_target_and_reach_the_attacker() {
        let attacker = [0.0, 0.0, 40.0];
        let victim = [0.0, 0.0, 0.0];
        let mut e = DrainEffect::new(attacker, victim, BLOOD_DRAIN);
        run(&mut e, 1);
        assert!(
            heads(&e).iter().all(|p| dist2d(*p, victim) < 5.0),
            "the burst starts on the victim"
        );
        run(&mut e, 29);
        let mid = heads(&e);
        assert!(
            mid.iter().all(|p| p[2] > 5.0),
            "it travels toward the attacker, not away"
        );
        run(&mut e, 30);
        assert!(
            heads(&e).iter().all(|p| p[2] > 60.0),
            "a full-radius drain overshoots the attacker"
        );
    }

    #[test]
    fn soul_drain_lands_on_the_caster_and_glows_blue() {
        let caster = [0.0, 0.0, 40.0];
        let target = [0.0, 0.0, 0.0];
        let mut e = DrainEffect::new(caster, target, ENERGY_DRAIN2);
        run(&mut e, 55);
        assert!(
            heads(&e)
                .iter()
                .all(|p| dist2d(*p, caster) < dist2d(*p, target)),
            "the half-radius drain ends on the caster"
        );
        assert_eq!(
            e.body_tint(),
            Some(BodyTint {
                rgb: [100, 100, 255]
            })
        );
        assert!(e.body_additive());
        assert!(e.body_copies().is_none(), "no doubled body on Soul Drain");
    }

    #[test]
    fn hp_conversion_scatters_and_wears_a_fading_halo() {
        let mut e = DrainEffect::new([0.0; 3], [0.0; 3], ENERGY_DRAIN3);
        run(&mut e, 30);
        let spread = heads(&e);
        let xs = spread.iter().map(|p| p[0]);
        assert!(
            xs.clone().fold(f32::MAX, f32::min) < -1.0 && xs.fold(f32::MIN, f32::max) > 1.0,
            "strands spray to both sides of the caster"
        );
        assert!(e.body_tint().is_none(), "tint starts at frame 50");

        run(&mut e, 25);
        let tint = e.body_tint().expect("inside the fade window");
        assert_eq!(tint.rgb[2], 250);
        assert!(tint.rgb[0] < 200 && tint.rgb[0] == tint.rgb[1]);
        assert!(!e.body_additive(), "HP Conversion doubles the body");
        let halo = e.body_copies().expect("doubled body")[0];
        assert!(halo.behind && (5.0..=6.5).contains(&halo.margin_px));
    }

    #[test]
    fn colours_and_sizes_match_the_original_ranges() {
        let mut blood = DrainEffect::new([0.0; 3], [0.0; 3], BLOOD_DRAIN);
        let mut energy = DrainEffect::new([0.0; 3], [0.0; 3], ENERGY_DRAIN);
        run(&mut blood, 1);
        run(&mut energy, 1);
        let b = blood.strands[0].color;
        let n = energy.strands[0].color;
        assert_eq!(b, [250.0 / 255.0, 100.0 / 255.0, 100.0 / 255.0]);
        assert_eq!(n, [100.0 / 255.0, 100.0 / 255.0, 250.0 / 255.0]);
        assert_eq!(blood.strands[0].size, 1.7);

        let mut hp = DrainEffect::new([0.0; 3], [0.0; 3], ENERGY_DRAIN3);
        run(&mut hp, 1);
        for s in &hp.strands {
            assert_eq!(s.color[1], 1.0, "green is pinned at 255");
            assert!((160.0 / 255.0..=240.0 / 255.0).contains(&s.color[0]));
            assert!((1.0..=1.5).contains(&s.size));
        }
    }

    #[test]
    fn effects_die_after_their_duration() {
        for params in [BLOOD_DRAIN, ENERGY_DRAIN2, ENERGY_DRAIN3] {
            let mut e = DrainEffect::new([0.0, 0.0, 20.0], [0.0; 3], params);
            let frames = params.geometry.spawn_through() + DURATION_FRAMES + 2;
            assert_eq!(run(&mut e, frames), EffectStatus::Dead);
        }
    }
}
