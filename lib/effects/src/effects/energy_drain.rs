//! Energy Drain / Blood Drain family.
//!
//! Two distinct shapes share one parameter table:
//! * Blood/Energy Drain and Energy Drain 2 are **straight-line** dot trails:
//!   three `particle1` strands fan out from the caster (NW / N / NE) with the
//!   sprites flowing along each line. Energy Drain 2 is the reverse — three
//!   lines reaching in from the south, the sprites flowing back into the caster.
//! * Energy Drain 3 is the complex self-buff swirl: jittered `particle1`
//!   strands integrated as sprite particles along a spline path.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{BodyTint, Effect, EffectRenderCtx, EffectUpdateCtx};

pub const DRAIN_SPRITE: &str = "data/sprite/이팩트/particle1";
pub const SPRITES: &[&str] = &[DRAIN_SPRITE];

const FPS: f32 = 60.0;
const FRAME_DT: f32 = 1.0 / FPS;

/// Effect lifetime in frames.
const DURATION_FRAMES: u32 = 60;
const ANIM_SPEED: u32 = 4;

/// Maps the original game's sprite size onto our renderer's sprite scale, whose
/// per-unit footprint is larger.
const SIZE_RENDER_SCALE: f32 = 0.5;

// --- Straight-line (Blood / Energy / Energy2) ----------------------------
//
// A single migrating burst: each of the three lines emits `num_line_dots`
// sprites in turn (`emit_period_frames` apart). Each sprite slides along its
// line at `line_flow_speed` and fades out after `line_max_dist`, so the burst
// streams away from (LinesOut) or into (LinesIn) the caster and the caster end
// empties — it does not loop. These knobs are configured per effect on
// `DrainParams::lines`.

/// Lines start at the caster's head, not the feet (negative Y is up).
const HEAD_Y: f32 = -7.0;

// --- Spline (Energy Drain 3) ---------------------------------------------

const NUM_SEGMENT: usize = 7;
const GRAV_SPEED_INIT: f32 = 2.0;
const LATI_SPEED_INIT: f32 = -2.0;
const SPLINE_DELTA_POS2_Y: f32 = -10.0;
const ROLLS_DEG: [f32; 3] = [-90.0, 0.0, 90.0];
const SPLINE_SPAWN_THROUGH: u32 = 4;
/// Caster→target distance assumed when spawned at a single point (effect
/// viewer). Energy Drain 3 is a self-buff with near-zero travel, so the swirl
/// stays anchored on the caster.
const SPLINE_FALLBACK_DIST: f32 = 6.0;

#[derive(Clone, Copy, PartialEq)]
pub enum DrainShape {
    /// Lines fan outward from the caster; sprites flow away.
    LinesOut,
    /// Lines reach in from the south; sprites flow back into the caster.
    LinesIn,
    /// Jittered spline swirl (Energy Drain 3).
    Spline,
}

/// Per-effect tuning for the straight-line burst shapes (LinesOut / LinesIn).
#[derive(Clone, Copy)]
pub struct LineParams {
    /// Sprites emitted per line (three lines → `3 * num_line_dots` total).
    pub num_line_dots: u32,
    /// Frames between consecutive sprite emissions on a line.
    pub emit_period_frames: u32,
    /// How far a sprite slides along its line each frame.
    pub flow_speed: f32,
    /// Travel distance over which a sprite lives and fades.
    pub max_dist: f32,
    /// Half-angle of the NW/N/NE fan, in degrees.
    pub fan_deg: f32,
}

impl LineParams {
    /// Frames a single sprite is alive.
    const fn life_frames(&self) -> u32 {
        (self.max_dist / self.flow_speed) as u32
    }

    /// Whole-effect frame span: last sprite's emission plus its lifetime.
    const fn total_frames(&self) -> u32 {
        (self.num_line_dots - 1) * self.emit_period_frames + self.life_frames()
    }
}

const DEFAULT_LINES: LineParams = LineParams {
    num_line_dots: 7,
    emit_period_frames: 5,
    flow_speed: 1.65,
    max_dist: 80.0,
    fan_deg: 15.0,
};

const DEFAULT_SPLINES: LineParams = LineParams {
    num_line_dots: 7,
    emit_period_frames: 5,
    flow_speed: 0.5,
    max_dist: 40.0,
    fan_deg: 45.0,
};
/// Caster body recolor played alongside the drain — Soul Drain glows the
/// caster blue, HP Conversion fades it toward blue (the original's `SetArgb`
/// over a frame window). `rgb` is a static 8-bit multiply; `None` selects HP
/// Conversion's per-frame fade (`250−2f` on R/G, `250` on B).
#[derive(Clone, Copy)]
pub struct BodyRecolor {
    /// Inclusive effect-age frame window.
    pub window: (u32, u32),
    pub rgb: Option<[f32; 3]>,
    /// Render the body additively within the window (the original's
    /// `BL_LIGHT_BODY` glow — Soul Drain).
    pub additive: bool,
}

#[derive(Clone, Copy)]
pub struct DrainParams {
    pub color: [f32; 4],
    pub size: f32,
    pub color_jitter: f32,
    pub size_jitter: f32,
    pub shape: DrainShape,
    /// Straight-line burst tuning; ignored by the `Spline` shape.
    pub lines: LineParams,
    /// Optional caster body recolor; `None` for the drains that don't tint.
    pub body_recolor: Option<BodyRecolor>,
}

impl DrainParams {
    pub const fn total_duration_ms(&self) -> u32 {
        let frames = match self.shape {
            DrainShape::Spline => SPLINE_SPAWN_THROUGH + DURATION_FRAMES,
            _ => self.lines.total_frames(),
        };
        (frames as f32 / FPS * 1000.0) as u32
    }
}

pub const BLOOD_DRAIN: DrainParams = DrainParams {
    color: [1.0, 0.39, 0.39, 1.0],
    size: 1.7,
    color_jitter: 0.0,
    size_jitter: 0.0,
    shape: DrainShape::LinesOut,
    lines: DEFAULT_LINES,
    body_recolor: None,
};

pub const ENERGY_DRAIN: DrainParams = DrainParams {
    color: [0.39, 0.39, 0.98, 1.0],
    size: 1.7,
    color_jitter: 0.0,
    size_jitter: 0.0,
    shape: DrainShape::LinesOut,
    lines: DEFAULT_LINES,
    body_recolor: None,
};

// red = 160 + random(81), blue = 255, size = 1.5 + random(6)*0.1.
pub const ENERGY_DRAIN2: DrainParams = DrainParams {
    color: [0.7, 0.7, 1.0, 1.0],
    size: 1.5,
    color_jitter: 0.3,
    size_jitter: 0.6,
    shape: DrainShape::LinesIn,
    lines: DEFAULT_LINES,
    // Soul Drain glows the caster blue over frames 55..=65.
    body_recolor: Some(BodyRecolor {
        window: (55, 65),
        rgb: Some([100.0, 100.0, 255.0]),
        additive: true,
    }),
};

// green = 255, red/blue = 160 + random(81), spawns on frames 0..=4.
pub const ENERGY_DRAIN3: DrainParams = DrainParams {
    color: [0.7, 1.0, 0.7, 1.0],
    size: 1.0,
    color_jitter: 0.3,
    size_jitter: 0.6,
    shape: DrainShape::Spline,
    lines: DEFAULT_SPLINES,
    // HP Conversion fades the caster toward blue over frames 50..=80.
    body_recolor: Some(BodyRecolor {
        window: (50, 80),
        rgb: None,
        additive: false,
    }),
};

/// Deterministic per-strand pseudo-random in `[0, 1)` so colour/size/heading
/// jitter is stable across runs and testable without an RNG dependency.
pub fn hash01(seed: u32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E37_79B9).wrapping_add(0x6D2B_79F5);
    x ^= x >> 15;
    x = x.wrapping_mul(0x8589_45CD);
    x ^= x >> 13;
    (x & 0xFF_FFFF) as f32 / 0x100_0000 as f32
}

// ---------------------------------------------------------------------------
// Spline strand (Energy Drain 3) — faithful spline-particle integration.
// ---------------------------------------------------------------------------

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
    color: [f32; 4],
    size: f32,
}

impl DrainStrand {
    fn new(
        org_pos: [f32; 3],
        radius: f32,
        heading_rad: f32,
        roll_deg: f32,
        color: [f32; 4],
        size: f32,
    ) -> Self {
        let longitude = -heading_rad;
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

// ---------------------------------------------------------------------------
// Effect
// ---------------------------------------------------------------------------

pub struct DrainEffect {
    org_pos: [f32; 3],
    params: DrainParams,
    /// Unit directions of the three fan lines (straight-line shapes only).
    line_dirs: [[f32; 3]; 3],
    /// Spline-only state.
    radius: f32,
    heading_rad: f32,
    next_spawn_frame: u32,
    spawn_seed: u32,
    strands: Vec<DrainStrand>,

    effect_frame: u32,
    time_accum: f32,
    age: f32,
}

impl DrainEffect {
    pub fn new(from: [f32; 3], to: [f32; 3], params: DrainParams) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let has_dir = dx * dx + dz * dz > 0.001;
        // Base heading: +Z ("north", away from the viewer camera) when no axis
        // is supplied. `atan2(dx, dz)` keeps +Z as angle 0.
        let base = if has_dir { dx.atan2(dz) } else { 0.0 };

        let head = [from[0], from[1] + HEAD_Y, from[2]];

        // For the reverse shape the lines reach in from the opposite side, so
        // the fan is built around the back heading.
        let fan_base = match params.shape {
            DrainShape::LinesIn => base + std::f32::consts::PI,
            _ => base,
        };
        let fan = params.lines.fan_deg.to_radians();
        let line_dirs = [-fan, 0.0, fan].map(|off| {
            let a = fan_base + off;
            [a.sin(), 0.0, a.cos()]
        });

        let radius = if has_dir {
            (dx * dx + dz * dz).sqrt()
        } else {
            SPLINE_FALLBACK_DIST
        };
        let spline_org = [from[0], from[1] + SPLINE_DELTA_POS2_Y, from[2]];

        Self {
            org_pos: match params.shape {
                DrainShape::Spline => spline_org,
                _ => head,
            },
            params,
            line_dirs,
            radius,
            heading_rad: base,
            next_spawn_frame: 0,
            spawn_seed: 0,
            strands: Vec::with_capacity(15),
            effect_frame: 0,
            time_accum: 0.0,
            age: 0.0,
        }
    }

    fn spawn_spline_burst(&mut self) {
        for &roll in &ROLLS_DEG {
            let seed = self.spawn_seed;
            self.spawn_seed += 1;

            // Energy Drain 3 jitters each strand's heading (original game offsets
            // the aim point by `random(7) - 3` units at the cast distance).
            let jitter = (hash01(seed) - 0.5) * 6.0;
            let heading = self.heading_rad + (jitter / self.radius.max(1.0));

            let mut color = self.params.color;
            let j = self.params.color_jitter;
            color[0] = (color[0] + (hash01(seed ^ 0x01) - 0.5) * j).clamp(0.0, 1.0);
            color[1] = (color[1] + (hash01(seed ^ 0x02) - 0.5) * j).clamp(0.0, 1.0);
            color[2] = (color[2] + (hash01(seed ^ 0x03) - 0.5) * j).clamp(0.0, 1.0);
            let size = self.params.size + hash01(seed ^ 0x04) * self.params.size_jitter;

            self.strands.push(DrainStrand::new(
                self.org_pos,
                self.radius,
                heading,
                roll,
                color,
                size,
            ));
        }
    }

    fn tick(&mut self) {
        self.effect_frame += 1;

        if self.params.shape == DrainShape::Spline {
            for s in &mut self.strands {
                if s.alive() {
                    s.step();
                }
            }
            self.strands.retain(|s| s.alive());

            while self.next_spawn_frame <= SPLINE_SPAWN_THROUGH
                && self.effect_frame >= self.next_spawn_frame
            {
                self.spawn_spline_burst();
                self.next_spawn_frame += 1;
            }
        }
    }

    fn collect_lines(&self, out: &mut EffectDrawList) {
        let outward = self.params.shape == DrainShape::LinesOut;
        let lines = &self.params.lines;
        let motion = (self.effect_frame / ANIM_SPEED) as usize;
        let size = self.params.size * SIZE_RENDER_SCALE;

        for dir in &self.line_dirs {
            for k in 0..lines.num_line_dots {
                let birth = (k * lines.emit_period_frames) as f32;
                let traveled = (self.effect_frame as f32 - birth) * lines.flow_speed;
                if traveled < 0.0 || traveled > lines.max_dist {
                    continue;
                }
                // LinesOut: sprites leave the caster (distance grows from 0).
                // LinesIn: sprites arrive from the far end and converge on it.
                let dist = if outward {
                    traveled
                } else {
                    lines.max_dist - traveled
                };
                // Fade in at the source end, out at the destination end.
                let alpha =
                    self.params.color[3] * (std::f32::consts::PI * traveled / lines.max_dist).sin();
                out.push(EffectPrimitiveDraw::SpriteParticle {
                    sprite_path: DRAIN_SPRITE,
                    position: [
                        self.org_pos[0] + dir[0] * dist,
                        self.org_pos[1],
                        self.org_pos[2] + dir[2] * dist,
                    ],
                    action_index: 0,
                    motion_index: motion,
                    size_scale: size,
                    color: [
                        self.params.color[0],
                        self.params.color[1],
                        self.params.color[2],
                        alpha,
                    ],
                    blend: BlendKind::Additive,
                    aim_target: None,
                    no_depth: false,
                });
            }
        }
    }

    fn collect_spline(&self, out: &mut EffectDrawList) {
        let fn_seg = NUM_SEGMENT as f32;
        for strand in &self.strands {
            let motion = (strand.frame_count / ANIM_SPEED) as usize;
            for i in 0..NUM_SEGMENT {
                let fi = i as f32;
                let alpha = strand.color[3] * (1.0 - fi / fn_seg);
                let size = strand.size * SIZE_RENDER_SCALE * (1.0 - fi / (2.0 * fn_seg));
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
}

impl Effect for DrainEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        self.time_accum += ctx.delta;
        while self.time_accum >= FRAME_DT {
            self.time_accum -= FRAME_DT;
            self.tick();
        }

        match self.params.shape {
            DrainShape::Spline => {
                let done_spawning = self.next_spawn_frame > SPLINE_SPAWN_THROUGH;
                if done_spawning && self.strands.is_empty() {
                    EffectStatus::Dead
                } else {
                    EffectStatus::Running
                }
            }
            _ => {
                if self.effect_frame >= self.params.lines.total_frames() {
                    EffectStatus::Dead
                } else {
                    EffectStatus::Running
                }
            }
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        match self.params.shape {
            DrainShape::Spline => self.collect_spline(out),
            _ => self.collect_lines(out),
        }
    }

    fn body_tint(&self) -> Option<BodyTint> {
        let r = self.params.body_recolor?;
        if !(r.window.0..=r.window.1).contains(&self.effect_frame) {
            return None;
        }
        let rgb = match r.rgb {
            Some(c) => [c[0] as u8, c[1] as u8, c[2] as u8],
            // HP Conversion: 250 − 2·frame on R/G (clamped), 250 on B.
            None => {
                let v = (250i32 - 2 * self.effect_frame as i32).clamp(0, 255) as u8;
                [v, v, 250]
            }
        };
        Some(BodyTint { rgb })
    }

    fn body_additive(&self) -> bool {
        self.params
            .body_recolor
            .is_some_and(|r| r.additive && (r.window.0..=r.window.1).contains(&self.effect_frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut DrainEffect, dt: f32) -> EffectStatus {
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

    fn draws(e: &DrainEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn spr(p: &EffectPrimitiveDraw) -> ([f32; 3], [f32; 4]) {
        match p {
            EffectPrimitiveDraw::SpriteParticle {
                position, color, ..
            } => (*position, *color),
            _ => panic!("expected SpriteParticle"),
        }
    }

    fn run(e: &mut DrainEffect, frames: u32) {
        for _ in 0..frames {
            step(e, FRAME_DT);
        }
    }

    /// Mean distance of every drawn sprite from the caster, in the ground plane.
    fn mean_dist(e: &DrainEffect) -> f32 {
        let d = draws(e);
        let org = e.org_pos;
        let sum: f32 = d
            .iter()
            .map(|p| {
                let pos = spr(p).0;
                ((pos[0] - org[0]).powi(2) + (pos[2] - org[2]).powi(2)).sqrt()
            })
            .sum();
        sum / d.len() as f32
    }

    #[test]
    fn straight_lines_emit_three_lines_of_seven_sprites() {
        // By the time the last sprite has been emitted and the first has not yet
        // expired, all three lines are fully populated.
        for params in [BLOOD_DRAIN, ENERGY_DRAIN, ENERGY_DRAIN2] {
            let lines = params.lines;
            let mut e = DrainEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], params);
            run(
                &mut e,
                (lines.num_line_dots - 1) * lines.emit_period_frames + 1,
            );
            assert_eq!(draws(&e).len(), 3 * lines.num_line_dots as usize);
        }
    }

    #[test]
    fn lines_out_fan_north_lines_in_fan_south() {
        // Default base heading is +Z (north). LinesOut points north (+Z),
        // LinesIn reaches in from the south (-Z).
        let out = DrainEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], ENERGY_DRAIN);
        let inn = DrainEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], ENERGY_DRAIN2);
        assert!(
            out.line_dirs.iter().all(|d| d[2] > 0.0),
            "out lines head north"
        );
        assert!(
            inn.line_dirs.iter().all(|d| d[2] < 0.0),
            "in lines come from south"
        );
        // Outer lines splay east/west, centre line is straight ahead.
        assert!(out.line_dirs[0][0] < -0.1 && out.line_dirs[2][0] > 0.1);
        assert!(out.line_dirs[1][0].abs() < 0.01);
    }

    #[test]
    fn out_burst_migrates_away_in_burst_migrates_toward_caster() {
        let mut out = DrainEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], ENERGY_DRAIN);
        let mut inn = DrainEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], ENERGY_DRAIN2);
        run(&mut out, 10);
        run(&mut inn, 10);
        let (o_early, i_early) = (mean_dist(&out), mean_dist(&inn));
        run(&mut out, 30);
        run(&mut inn, 30);
        assert!(
            mean_dist(&out) > o_early,
            "out: the burst streams away from the caster"
        );
        assert!(
            mean_dist(&inn) < i_early,
            "in: the burst converges onto the caster"
        );
    }

    #[test]
    fn spline_spawns_fifteen_strands_over_spawn_window() {
        let mut e = DrainEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 20.0], ENERGY_DRAIN3);
        for _ in 0..6 {
            step(&mut e, FRAME_DT);
        }
        assert_eq!(e.strands.len(), 15, "3 strands × frames 0..=4");
        assert_eq!(draws(&e).len(), 15 * NUM_SEGMENT);
    }

    #[test]
    fn blood_and_energy_tints_differ() {
        let mut blood = DrainEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], BLOOD_DRAIN);
        let mut energy = DrainEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], ENERGY_DRAIN);
        step(&mut blood, FRAME_DT);
        step(&mut energy, FRAME_DT);
        let bc = spr(&draws(&blood)[0]).1;
        let ec = spr(&draws(&energy)[0]).1;
        assert!(bc[0] > bc[2], "blood drain is red-dominant");
        assert!(ec[2] > ec[0], "energy drain is blue-dominant");
    }

    #[test]
    fn soul_drain_glows_blue_and_hp_conversion_fades_in_window() {
        // Soul Drain: blue glow (additive) over frames 55..=65, nothing before.
        let mut sd = DrainEffect::new([0.0; 3], [0.0; 3], ENERGY_DRAIN2);
        run(&mut sd, 50);
        assert_eq!(sd.body_tint(), None, "no tint before the window");
        run(&mut sd, 10); // frame ~60
        assert_eq!(
            sd.body_tint(),
            Some(BodyTint {
                rgb: [100, 100, 255]
            })
        );
        assert!(sd.body_additive(), "Soul Drain glows (BL_LIGHT_BODY)");

        // HP Conversion: fade toward blue (250−2f) over 50..=80, not additive.
        let mut hp = DrainEffect::new([0.0; 3], [0.0, 0.0, 20.0], ENERGY_DRAIN3);
        run(&mut hp, 60); // frame ~60 → 250 − 120 = 130
        let tint = hp.body_tint().expect("inside the fade window");
        assert_eq!(tint.rgb[2], 250, "blue stays at 250");
        assert!(
            tint.rgb[0] < 200 && tint.rgb[0] == tint.rgb[1],
            "R/G fade together"
        );
        assert!(
            !hp.body_additive(),
            "HP Conversion is a multiply, not a glow"
        );
    }

    #[test]
    fn effects_die_after_their_duration() {
        for params in [ENERGY_DRAIN, ENERGY_DRAIN2, ENERGY_DRAIN3] {
            let mut e = DrainEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 20.0], params);
            let mut status = EffectStatus::Running;
            for _ in 0..200 {
                status = step(&mut e, FRAME_DT);
                if status == EffectStatus::Dead {
                    break;
                }
            }
            assert_eq!(status, EffectStatus::Dead);
        }
    }
}
