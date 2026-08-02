//! EF_WARPZONE / EF_WARPZONE2 — two distinct portal effects sharing a base pad.
//!
//! `EF_WARPZONE` (`PARAMS_BURST`, the 250-tick warp flash) is a flat pad: its
//! frame counter wraps at 78, and each cycle emits:
//!   * frame 0 — one base disc (`alpha_down.tga`, radius 15, duration 158,
//!     fade-in over 11 frames, fade-out last 10);
//!   * frames 0, 28, 56 — a flat inner ring (`ring_blue.tga`, radius 14, slight
//!     shrink, duration 80 — the original lays it flat with `latitude=90`);
//!   * every 10 frames — an orbiting `particle1.spr` sparkle that circles the
//!     pad and drifts upward under decelerating gravity (~70-frame life).
//!
//! `EF_WARPZONE2` (`PARAMS_SUSTAINED`, the persistent warp-NPC portal) keeps the
//! same flat base disc + sparkles, but replaces the flat inner ring with the
//! original's `PP_3DCASTING_4` *flared funnel*: concentric `ring_blue.tga` bands
//! that rise from the ground at an angle and slowly collapse inward
//! (`distance -= 0.05`/frame, wrapping back to 10), tilting from flat at radius
//! 10 (`rise_angle = 90 − distance·9`) toward vertical as they shrink. That angle
//! + slow inward drift is what reads as a 3-D swirling portal rather than a flat
//! disc.
//!
//! `EffectId::Warpzone` runs for the spec's `duration_ms` (2.5 s in the
//! generated table → 2 cycles); `EffectId::Warpzone2` runs effectively forever.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const BASE_TEXTURE: &str = "alpha_down.tga";
pub const INNER_TEXTURE: &str = "ring_blue.tga";
pub const TEXTURES: &[&str] = &[BASE_TEXTURE, INNER_TEXTURE];

pub const SPARKLE_SPRITE: &str = ragnarok_resources::sprite::effect::PARTICLE1;
pub const SPRITES: &[&str] = &[SPARKLE_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// The original game wraps its frame counter to 0 once it hits 78.
const CYCLE_FRAMES: f32 = 78.0;
const CYCLE_S: f32 = CYCLE_FRAMES / FRAMES_PER_SECOND;
/// Inner-ring cadence within a cycle (every 28 frames up to a 117-frame cap,
/// the upper bound never triggers because of the wrap at 78).
const INNER_INTERVAL_FRAMES: f32 = 28.0;
const INNER_INTERVAL_S: f32 = INNER_INTERVAL_FRAMES / FRAMES_PER_SECOND;
const BASE_DURATION_FRAMES: f32 = 158.0;
const BASE_DURATION_S: f32 = BASE_DURATION_FRAMES / FRAMES_PER_SECOND;
const INNER_DURATION_FRAMES: f32 = 80.0;
const INNER_DURATION_S: f32 = INNER_DURATION_FRAMES / FRAMES_PER_SECOND;

const BASE_RADIUS: f32 = 15.0;
const BASE_MAX_ALPHA: f32 = 80.0 / 255.0;
const BASE_FADE_IN_FRAMES: f32 = 11.0;
const BASE_FADE_OUT_AT_FRAME: f32 = BASE_DURATION_FRAMES - 10.0;

const INNER_RADIUS: f32 = 14.0;
const INNER_RADIUS_SHRINK_PER_FRAME: f32 = 0.15;
const INNER_THICKNESS: f32 = 4.0;
const INNER_MAX_ALPHA: f32 = 200.0 / 255.0;
const INNER_FADE_IN_FRAMES: f32 = 20.0;
/// Fade-out starts at `duration - duration/5 = 64`.
const INNER_FADE_OUT_AT_FRAME: f32 = INNER_DURATION_FRAMES * 0.8;
const INNER_UV_REPEAT: f32 = 4.0;

/// Sparkle cadence: the original game launches one orbiting sparkle every
/// 10 frames.
const SPARKLE_INTERVAL_FRAMES: f32 = 10.0;
const SPARKLE_INTERVAL_S: f32 = SPARKLE_INTERVAL_FRAMES / FRAMES_PER_SECOND;
const SPARKLE_DURATION_FRAMES: f32 = 70.0;
const SPARKLE_DURATION_S: f32 = SPARKLE_DURATION_FRAMES / FRAMES_PER_SECOND;
/// Fade-out starts at `duration - duration/3`.
const SPARKLE_FADE_OUT_AT_FRAME: f32 = SPARKLE_DURATION_FRAMES - SPARKLE_DURATION_FRAMES / 3.0;
/// Orbit radius `4.5 + rnd*0.75` — these run in the same world units the disc
/// already uses (base radius 15), so no extra scaling.
const SPARKLE_BASE_RADIUS: f32 = 4.5;
const SPARKLE_RADIUS_STEP: f32 = 0.75;
/// Longitude spin `±2.5` deg/frame.
const SPARKLE_LONG_SPEED_DEG: f32 = 2.5;
/// Vertical speed `-0.3` (negative Y = up), decelerating via the accel below.
const SPARKLE_Y_SPEED_PER_FRAME: f32 = -0.3;
const SPARKLE_Y_ACCEL_PER_FRAME: f32 = -(SPARKLE_Y_SPEED_PER_FRAME / SPARKLE_DURATION_FRAMES) / 1.5;
const SPARKLE_SIZE: f32 = 0.7;
const SPARKLE_ANIM_TICKS: f32 = 4.0;

/// Returned by the factory to differentiate `Warpzone` (timed) from
/// `Warpzone2` (sustained).
#[derive(Clone, Copy)]
pub struct WarpZoneParams {
    pub total_duration_s: f32,
    /// RGB multiplier on the disc + ring colour. The original game applies no
    /// tint — the blue comes entirely from `ring_blue.tga` — so this stays white;
    /// it exists only as a hook for reskins.
    pub tint: [f32; 3],
    /// World units to raise the disc/ring centre off the floor (native RO up
    /// is −Y) so the pad isn't swallowed by the ground at grazing angles.
    pub ground_lift: f32,
    /// Enable the orbiting `particle1.spr` sparkles.
    pub sparkles: bool,
    /// Render the inner ring as the original `PP_3DCASTING_4` flared funnel
    /// (`EF_WARPZONE2`) instead of a flat shrinking disc (`EF_WARPZONE`).
    pub funnel: bool,
}

pub const PARAMS_BURST: WarpZoneParams = WarpZoneParams {
    total_duration_s: 2.5,
    tint: [1.0, 1.0, 1.0],
    ground_lift: 1.0,
    sparkles: false,
    funnel: false,
};
pub const PARAMS_SUSTAINED: WarpZoneParams = WarpZoneParams {
    // Effectively infinite — the holder respects the spec's u32::MAX duration.
    total_duration_s: f32::INFINITY,
    // No tint: the original game renders `ring_blue.tga` at white vertex colour,
    // so the blue is the texture's own. A periwinkle multiplier here only
    // over-saturates the already-blue ring and forces the warm sparkles blue.
    tint: [0.8, 0.8, 1.0],
    ground_lift: 1.0,
    sparkles: true,
    funnel: true,
};

// --- EF_WARPZONE2 flared funnel (`PP_3DCASTING_4` / `Prim3DCasting4`) ---

/// Band start radii + texture-phase offsets, transcribed from the original's two
/// `PP_3DCASTING_4` launches. The four radii are doubled into near-coincident
/// pairs (0.2 apart); since every band drifts inward at the same rate and wraps
/// at the same boundary, that spacing is preserved, so the eight bands read as
/// **four** rings — each a tight pair, not eight separate rings.
const FUNNEL_BANDS: [(f32, f32); 8] = [
    (2.5, 270.0),
    (5.0, 0.0),
    (7.5, 90.0),
    (10.0, 180.0),
    (2.7, 271.0),
    (5.2, 1.0),
    (7.7, 91.0),
    (10.2, 181.0),
];
/// Segments around each band — matches the casting-ring division so `ring_blue`'s
/// ray stripes land on the same cadence.
const FUNNEL_SIDES: u32 = 20;
/// Largest band radius; a band resets here once it collapses to the centre.
const FUNNEL_DIST_MAX: f32 = 10.0;
/// Inward drift: the original shrinks `distance` by 0.05 per frame — the slow
/// movement that distinguishes the portal from the flat warp flash.
const FUNNEL_SHRINK_PER_FRAME: f32 = 0.05;
/// Peak band alpha — the original ramps `alphaB` to 70/255.
const FUNNEL_MAX_ALPHA: f32 = 70.0 / 255.0;
/// `alphaB` ramps +1/frame from a band's reset, so it reaches peak after this
/// many frames of inward travel.
const FUNNEL_FADE_IN_FRAMES: f32 = FUNNEL_MAX_ALPHA * 255.0;

#[derive(Clone, Copy)]
struct BaseDisc {
    age: f32,
}

impl BaseDisc {
    fn frame(&self) -> f32 {
        (self.age * FRAMES_PER_SECOND).clamp(0.0, BASE_DURATION_FRAMES)
    }
    fn alive(&self) -> bool {
        self.age < BASE_DURATION_S
    }
    fn alpha(&self) -> f32 {
        let n = self.frame();
        if n <= BASE_FADE_IN_FRAMES {
            BASE_MAX_ALPHA * (n / BASE_FADE_IN_FRAMES).clamp(0.0, 1.0)
        } else if n >= BASE_FADE_OUT_AT_FRAME {
            let fade = ((n - BASE_FADE_OUT_AT_FRAME)
                / (BASE_DURATION_FRAMES - BASE_FADE_OUT_AT_FRAME))
                .clamp(0.0, 1.0);
            BASE_MAX_ALPHA * (1.0 - fade)
        } else {
            BASE_MAX_ALPHA
        }
    }
}

#[derive(Clone, Copy)]
struct InnerRing {
    age: f32,
}

impl InnerRing {
    fn frame(&self) -> f32 {
        (self.age * FRAMES_PER_SECOND).clamp(0.0, INNER_DURATION_FRAMES)
    }
    fn alive(&self) -> bool {
        self.age < INNER_DURATION_S
    }
    fn radius(&self) -> f32 {
        (INNER_RADIUS - INNER_RADIUS_SHRINK_PER_FRAME * self.frame()).max(0.0)
    }
    fn alpha(&self) -> f32 {
        let n = self.frame();
        if n <= INNER_FADE_IN_FRAMES {
            INNER_MAX_ALPHA * (n / INNER_FADE_IN_FRAMES).clamp(0.0, 1.0)
        } else if n >= INNER_FADE_OUT_AT_FRAME {
            let fade = ((n - INNER_FADE_OUT_AT_FRAME)
                / (INNER_DURATION_FRAMES - INNER_FADE_OUT_AT_FRAME))
                .clamp(0.0, 1.0);
            INNER_MAX_ALPHA * (1.0 - fade)
        } else {
            INNER_MAX_ALPHA
        }
    }
}

/// One orbiting `particle1.spr` sparkle. The original game randomises
/// longitude / radius / spin direction per spawn; we derive them
/// deterministically from a monotonic spawn counter so tests stay
/// reproducible — the three rates still drift the sparkles apart within a
/// fraction of a second.
#[derive(Clone, Copy)]
struct Sparkle {
    age: f32,
    initial_longitude_deg: f32,
    radius: f32,
    long_speed_deg: f32,
    y_offset: f32,
    y_speed_per_frame: f32,
}

impl Sparkle {
    fn spawn(index: u32) -> Self {
        let k = (index % 7) as f32;
        Self {
            age: 0.0,
            initial_longitude_deg: (index as f32 * 47.0) % 360.0,
            radius: SPARKLE_BASE_RADIUS + k * SPARKLE_RADIUS_STEP / 2.0,
            long_speed_deg: if index % 2 == 0 {
                SPARKLE_LONG_SPEED_DEG
            } else {
                -SPARKLE_LONG_SPEED_DEG
            },
            y_offset: 0.0,
            y_speed_per_frame: SPARKLE_Y_SPEED_PER_FRAME,
        }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }
    fn alive(&self) -> bool {
        self.age < SPARKLE_DURATION_S
    }
    fn step(&mut self, dt: f32) {
        let dt_frames = dt * FRAMES_PER_SECOND;
        self.y_speed_per_frame += SPARKLE_Y_ACCEL_PER_FRAME * dt_frames;
        self.y_offset += self.y_speed_per_frame * dt_frames;
        self.age += dt;
    }
    fn longitude_deg(&self) -> f32 {
        self.initial_longitude_deg + self.long_speed_deg * self.frame()
    }
    fn alpha(&self) -> f32 {
        let n = self.frame();
        if n >= SPARKLE_FADE_OUT_AT_FRAME {
            let span = (SPARKLE_DURATION_FRAMES - SPARKLE_FADE_OUT_AT_FRAME).max(1e-3);
            (1.0 - (n - SPARKLE_FADE_OUT_AT_FRAME) / span).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }
    fn position(&self, anchor: [f32; 3]) -> [f32; 3] {
        let (sn, cs) = self.longitude_deg().to_radians().sin_cos();
        [
            anchor[0] + self.radius * sn,
            anchor[1] + self.y_offset,
            anchor[2] + self.radius * cs,
        ]
    }
    fn motion_index(&self) -> usize {
        (self.frame() / SPARKLE_ANIM_TICKS) as usize
    }
}

pub struct WarpZoneEffect {
    world_pos: [f32; 3],
    params: WarpZoneParams,
    age: f32,
    next_base_at: f32,
    next_inner_at: f32,
    next_sparkle_at: f32,
    sparkle_count: u32,
    base_discs: Vec<BaseDisc>,
    inner_rings: Vec<InnerRing>,
    sparkles: Vec<Sparkle>,
}

impl WarpZoneEffect {
    pub fn new(world_pos: [f32; 3], params: WarpZoneParams) -> Self {
        Self {
            world_pos,
            params,
            age: 0.0,
            next_base_at: 0.0,
            next_inner_at: 0.0,
            next_sparkle_at: 0.0,
            sparkle_count: 0,
            base_discs: Vec::new(),
            inner_rings: Vec::new(),
            sparkles: Vec::new(),
        }
    }

    /// Disc/ring centre, lifted off the floor (native RO up is −Y).
    fn center(&self) -> [f32; 3] {
        [
            self.world_pos[0],
            self.world_pos[1] - self.params.ground_lift,
            self.world_pos[2],
        ]
    }

    /// The `EF_WARPZONE2` flared funnel: each band is a flared cone rising from a
    /// ground ring at `distance` toward `rise_angle`, the whole set drifting
    /// inward over time (`Prim3DCasting4`). Computed analytically from `age` so
    /// the persistent effect needs no per-band state.
    fn collect_funnel_rings(&self, out: &mut EffectDrawList, [r, g, b]: [f32; 3]) {
        let frame = self.age * FRAMES_PER_SECOND;
        let startup = (frame / FUNNEL_FADE_IN_FRAMES).clamp(0.0, 1.0);
        for (start, rot) in FUNNEL_BANDS {
            // Inward drift, wrapping back to the outermost radius on collapse.
            let mut d = (start - FUNNEL_SHRINK_PER_FRAME * frame).rem_euclid(FUNNEL_DIST_MAX);
            if d <= 0.0 {
                d = FUNNEL_DIST_MAX;
            }
            // alphaB ramps +1/frame from the band's reset at radius 10.
            let frames_since_reset = (FUNNEL_DIST_MAX - d) / FUNNEL_SHRINK_PER_FRAME;
            let alpha = FUNNEL_MAX_ALPHA
                * (frames_since_reset / FUNNEL_FADE_IN_FRAMES).clamp(0.0, 1.0)
                * startup;
            if alpha <= 0.0 {
                continue;
            }
            let rise = (90.0 - d * 9.0).clamp(0.0, 90.0).to_radians();
            let height = d;
            out.push(EffectPrimitiveDraw::Frustum {
                base_alpha: 1.0,
                base: self.world_pos,
                bottom_size: d,
                top_size: d + rise.cos() * height,
                height: rise.sin() * height,
                sides: FUNNEL_SIDES,
                arc_angle_deg: 360.0,
                rotation: rot.to_radians(),
                uv_repeat: 1.0,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: 0.0,
                wave_frequency: 1.0,
                wave_phase: 0.0,
                wave_mode: FrustumWaveMode::Sine,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                cull_back: false,
                texture: INNER_TEXTURE,
                color: [r, g, b, alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

impl Effect for WarpZoneEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        for d in &mut self.base_discs {
            d.age += ctx.delta;
        }
        for r in &mut self.inner_rings {
            r.age += ctx.delta;
        }
        for s in &mut self.sparkles {
            s.step(ctx.delta);
        }

        let still_spawning = self.age < self.params.total_duration_s;

        // Catch up scheduled spawns landed during this tick.
        while still_spawning && self.next_base_at <= self.age {
            let initial_age = (self.age - self.next_base_at).max(0.0);
            self.base_discs.push(BaseDisc { age: initial_age });
            self.next_base_at += CYCLE_S;
        }
        // The funnel variant renders its rings analytically from `age`, so it
        // doesn't spawn flat inner-ring instances.
        while !self.params.funnel && still_spawning && self.next_inner_at <= self.age {
            let initial_age = (self.age - self.next_inner_at).max(0.0);
            self.inner_rings.push(InnerRing { age: initial_age });
            self.next_inner_at += INNER_INTERVAL_S;
        }
        while self.params.sparkles && still_spawning && self.next_sparkle_at <= self.age {
            let mut sparkle = Sparkle::spawn(self.sparkle_count);
            sparkle.age = (self.age - self.next_sparkle_at).max(0.0);
            self.sparkles.push(sparkle);
            self.sparkle_count += 1;
            self.next_sparkle_at += SPARKLE_INTERVAL_S;
        }

        self.base_discs.retain(|d| d.alive());
        self.inner_rings.retain(|r| r.alive());
        self.sparkles.retain(|s| s.alive());

        if !still_spawning
            && self.base_discs.is_empty()
            && self.inner_rings.is_empty()
            && self.sparkles.is_empty()
        {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let center = self.center();
        let [tr, tg, tb] = self.params.tint;
        for d in &self.base_discs {
            out.push(EffectPrimitiveDraw::GroundDisc {
                center,
                radius: BASE_RADIUS,
                thickness: BASE_RADIUS, // filled solid disc
                rotation: 0.0,
                arc_angle_deg: 360.0,
                uv_repeat: 1.0,
                texture: BASE_TEXTURE,
                color: [tr, tg, tb, d.alpha()],
                blend: BlendKind::Alpha,
                no_depth: false,
                tilt_rad: 0.0,
                spin_rad: 0.0,
            });
        }
        if self.params.funnel {
            self.collect_funnel_rings(out, [tr, tg, tb]);
        } else {
            for r in &self.inner_rings {
                let outer = r.radius();
                if outer <= 0.0 {
                    continue;
                }
                let thickness = outer.min(INNER_THICKNESS);
                out.push(EffectPrimitiveDraw::GroundDisc {
                    center,
                    radius: outer,
                    thickness,
                    rotation: 0.0,
                    arc_angle_deg: 360.0,
                    uv_repeat: INNER_UV_REPEAT,
                    texture: INNER_TEXTURE,
                    color: [1.0, 1.0, 1.0, r.alpha()],
                    // The WarpZone inner ring + orbit sparkles render
                    // source/inverse-source alpha, not additive —
                    // additive vanishes against a bright lightmap.
                    blend: BlendKind::Alpha,
                    no_depth: false,
                    tilt_rad: 0.0,
                    spin_rad: 0.0,
                });
            }
        }
        for s in &self.sparkles {
            let a = s.alpha();
            if a <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: SPARKLE_SPRITE,
                position: s.position(center),
                action_index: 0,
                motion_index: s.motion_index(),
                size_scale: SPARKLE_SIZE,
                // Untinted like the rings — the original game leaves the
                // `particle1.spr` sparkle its warm/yellow colour.
                color: [tr, tg, tb, a],
                blend: BlendKind::Alpha,
                aim_target: None,
                no_depth: false,
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

    fn draws(effect: &WarpZoneEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut WarpZoneEffect, dt: f32) -> EffectStatus {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn count_base(prims: &[EffectPrimitiveDraw]) -> usize {
        prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { texture, .. } if *texture == BASE_TEXTURE))
            .count()
    }
    fn count_inner(prims: &[EffectPrimitiveDraw]) -> usize {
        prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { texture, .. } if *texture == INNER_TEXTURE))
            .count()
    }
    fn count_sparkles(prims: &[EffectPrimitiveDraw]) -> usize {
        prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == SPARKLE_SPRITE))
            .count()
    }
    fn inner_rgb(prims: &[EffectPrimitiveDraw]) -> Option<[f32; 3]> {
        prims.iter().find_map(|p| match p {
            EffectPrimitiveDraw::GroundDisc { texture, color, .. } if *texture == INNER_TEXTURE => {
                Some([color[0], color[1], color[2]])
            }
            _ => None,
        })
    }
    fn disc_center_y(prims: &[EffectPrimitiveDraw]) -> Option<f32> {
        prims.iter().find_map(|p| match p {
            EffectPrimitiveDraw::GroundDisc { center, .. } => Some(center[1]),
            _ => None,
        })
    }

    #[test]
    fn spawns_a_base_disc_and_inner_at_frame_zero() {
        // The flat inner disc belongs to the burst variant; the sustained portal
        // uses the analytic funnel instead.
        let mut wz = WarpZoneEffect::new([0.0; 3], PARAMS_BURST);
        step(&mut wz, 0.0);
        let prims = draws(&wz);
        assert_eq!(count_base(&prims), 1);
        assert_eq!(count_inner(&prims), 1);
    }

    #[test]
    fn inner_ring_spawn_cadence_is_28_frames() {
        let mut wz = WarpZoneEffect::new([0.0; 3], PARAMS_BURST);
        step(&mut wz, 0.0);
        assert_eq!(count_inner(&draws(&wz)), 1);
        // Walk forward 28 frames → 2nd inner spawn.
        step(&mut wz, INNER_INTERVAL_S + 0.01);
        assert_eq!(count_inner(&draws(&wz)), 2);
        step(&mut wz, INNER_INTERVAL_S);
        assert_eq!(count_inner(&draws(&wz)), 3);
    }

    #[test]
    fn base_disc_respawns_each_cycle() {
        let mut wz = WarpZoneEffect::new([0.0; 3], PARAMS_SUSTAINED);
        step(&mut wz, 0.0);
        // After one cycle (78 frames) the previous base still lives (158-frame
        // duration) and a new one is spawned, so we see two simultaneously.
        step(&mut wz, CYCLE_S + 0.01);
        assert_eq!(count_base(&draws(&wz)), 2);
    }

    #[test]
    fn sustained_emits_orbiting_sparkles_burst_does_not() {
        // Sociable: walk past the first sparkle cadence and confirm the
        // sustained portal (321) emits orbiting `particle1.spr` sparkles while
        // the burst variant (gated off) emits none.
        let mut sustained = WarpZoneEffect::new([0.0; 3], PARAMS_SUSTAINED);
        step(&mut sustained, SPARKLE_INTERVAL_S * 2.5);
        let prims = draws(&sustained);
        assert!(count_sparkles(&prims) >= 2);
        // The flat base pad + sparkles render alpha-blended; the funnel rings
        // are additive glow (the casting-ring primitive).
        assert!(
            prims.iter().all(|p| matches!(
                p,
                EffectPrimitiveDraw::GroundDisc {
                    blend: BlendKind::Alpha,
                    ..
                } | EffectPrimitiveDraw::SpriteParticle {
                    blend: BlendKind::Alpha,
                    ..
                } | EffectPrimitiveDraw::Frustum {
                    blend: BlendKind::Additive,
                    ..
                }
            )),
            "base/sparkles alpha, funnel rings additive"
        );

        let mut burst = WarpZoneEffect::new([0.0; 3], PARAMS_BURST);
        step(&mut burst, SPARKLE_INTERVAL_S * 2.5);
        assert_eq!(count_sparkles(&draws(&burst)), 0);
    }

    fn funnel_rings(prims: &[EffectPrimitiveDraw]) -> Vec<&EffectPrimitiveDraw> {
        prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
            .collect()
    }

    #[test]
    fn burst_inner_ring_is_untinted_and_lifts_off_floor() {
        let mut burst = WarpZoneEffect::new([0.0; 3], PARAMS_BURST);
        step(&mut burst, 0.0);
        let burst_prims = draws(&burst);
        assert_eq!(inner_rgb(&burst_prims).unwrap(), [1.0, 1.0, 1.0]);
        assert_eq!(disc_center_y(&burst_prims).unwrap(), -1.0);
    }

    #[test]
    fn burst_variant_stops_spawning_after_duration() {
        let mut wz = WarpZoneEffect::new([0.0; 3], PARAMS_BURST);
        let mut t = 0.0;
        while t < PARAMS_BURST.total_duration_s + 0.5 {
            step(&mut wz, 1.0 / 60.0);
            t += 1.0 / 60.0;
        }
        let bases_after_burst = count_base(&draws(&wz));
        // Past spec duration we may still see dying primitives — but well past
        // the longest sub-life (158 frames ≈ 2.63 s), nothing remains.
        let mut t2 = 0.0;
        while t2 < BASE_DURATION_S + 0.2 {
            step(&mut wz, 1.0 / 60.0);
            t2 += 1.0 / 60.0;
        }
        assert!(
            count_base(&draws(&wz)) < bases_after_burst.max(1),
            "burst run must wind down to zero"
        );
    }

    #[test]
    fn burst_variant_dies_after_subprimitives_finish() {
        let mut wz = WarpZoneEffect::new([0.0; 3], PARAMS_BURST);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        // Walk well past spec duration + longest sub-life.
        while t < PARAMS_BURST.total_duration_s + BASE_DURATION_S + 1.0 {
            status = step(&mut wz, 1.0 / 60.0);
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert!(matches!(status, EffectStatus::Dead));
    }
}
