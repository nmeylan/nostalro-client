//! `EF_MAP_MAGICZONE` (#650) / `EF_MAP_MAGICZONE2` (#651) / `EF_GLOW4` (#695) —
//! map-scale magic-zone ground effects.
//!
//! In the original game these compose a `Map_MagicZone()` ground ring (one or
//! more spinning flat magic-circle textures), plus either a `Map_Particle()`
//! sparkle-mote field (651, 695) or a `Map_Pika()` sparkle floor + `Map_Aura()`
//! flared ring (650).
//!
//! * `Map_MagicZone(tex, F1)`: a single flat quad whose four
//!   corners orbit at radius `distance*0.9` around the caster, spinning `±1°`
//!   per frame. The magic-circle / radial-glow texture carries all the detail —
//!   geometry is just a spinning square — so we draw it as one ground
//!   [`WorldQuad`] with spun corners. White rings alpha-blend; the
//!   blue/green tinted ones are additive.
//! * `Map_Particle(tex, F1, F2)`: camera-facing sparkle motes
//!   orbiting the zone, fading with height, white or green (`F2==2`). Modeled as
//!   a deterministic per-index hash-scattered [`Billboard`] field with a slow
//!   orbit, vertical bob and twinkle (no RNG, matching [`super::light_sphere`]).
//! * `Map_Pika()` and `Map_Aura()` are the same
//!   primitives the level-99 aura uses, so 650 reuses [`super::floor_aura`] and
//!   [`super::casting_ring`] with map-zone params.
//!
//! All persistent: the holder reaps them at their `u32::MAX` duration.
//!
//! [`WorldQuad`]: EffectPrimitiveDraw::WorldQuad
//! [`Billboard`]: EffectPrimitiveDraw::Billboard

use std::f32::consts::{FRAC_PI_2, TAU};

use super::casting_ring::{CastingRingEffect, MAP_AURA};
use super::floor_aura::{FloorAuraEffect, MAP_PIKA};
use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

/// Shared alpha ramp-in so nothing pops in (frames).
const FADE_IN_FRAMES: f32 = 20.0;

/// The corners sit at `distance*0.9`; `RING_SCALE` lets us
/// pull the map-scale `distance` values down to the gif silhouette (the source
/// numbers are map-scale and read larger than the capture).
const RING_SCALE: f32 = 0.7;
/// Rotation advances 1° per frame.
const RING_SPIN_DEG_PER_FRAME: f32 = 1.0;

const PARTICLE_COUNT: u32 = 60;
const PARTICLE_SIZE: f32 = 1.1;
const PARTICLE_PEAK_ALPHA: f32 = 0.8;
/// Motes scatter across the zone disc rather than the original's tall thin
/// orbit column — the gif shows them spread around/over the magic circle.
const ORBIT_R_MIN: f32 = 5.0;
const ORBIT_R_MAX: f32 = 28.0;
/// Mote height range above the ground (native RO: negative y = up). Kept low so
/// they hover over the circle instead of forming a column.
const HEIGHT_MIN: f32 = 1.0;
const HEIGHT_MAX: f32 = 16.0;
const BOB_AMP: f32 = 3.0;
const BOB_SPEED_DEG_PER_FRAME: f32 = 2.0;
const TWINKLE_SPEED_DEG_PER_FRAME: f32 = 4.0;
const FIELD_SPIN_DEG_PER_FRAME: f32 = 0.4;

const WHITE: [f32; 3] = [1.0, 1.0, 1.0];

#[derive(Clone, Copy)]
pub struct GroundRing {
    pub texture: &'static str,
    /// Ring `distance`; corner radius is `distance*0.9`.
    pub distance: f32,
    pub color_rgb: [f32; 3],
    pub alpha: f32,
    /// Ground height offset (-0.5 / -0.4).
    pub y_lift: f32,
    /// `F1 == 14` spins counter-clockwise; everything else clockwise.
    pub spin_ccw: bool,
    /// White rings alpha-blend; tinted rings are additive.
    pub additive: bool,
}

#[derive(Clone, Copy)]
pub struct ParticleField {
    pub texture: &'static str,
    pub color_rgb: [f32; 3],
}

#[derive(Clone, Copy)]
pub struct MapZoneParams {
    pub rings: &'static [GroundRing],
    pub particles: Option<ParticleField>,
    pub pika: bool,
    pub aura: bool,
}

/// `EF_MAP_MAGICZONE` (650): two white magic-circle rings + sparkle floor + aura.
pub const MAP_MAGICZONE: MapZoneParams = MapZoneParams {
    rings: &[
        GroundRing { texture: "mjin2.tga", distance: 30.0, color_rgb: WHITE, alpha: 200.0 / 255.0, y_lift: -0.5, spin_ccw: false, additive: false },
        GroundRing { texture: "mjin.tga", distance: 30.0, color_rgb: WHITE, alpha: 50.0 / 255.0, y_lift: -0.4, spin_ccw: false, additive: false },
    ],
    particles: None,
    pika: true,
    aura: true,
};

/// `EF_MAP_MAGICZONE2` (651): two white rings + white sparkle motes.
pub const MAP_MAGICZONE2: MapZoneParams = MapZoneParams {
    rings: &[
        GroundRing { texture: "mjin2.tga", distance: 40.0, color_rgb: WHITE, alpha: 200.0 / 255.0, y_lift: -0.5, spin_ccw: false, additive: false },
        GroundRing { texture: "mjin.tga", distance: 40.0, color_rgb: WHITE, alpha: 50.0 / 255.0, y_lift: -0.4, spin_ccw: false, additive: false },
    ],
    particles: Some(ParticleField { texture: "mpa3.tga", color_rgb: WHITE }),
    pika: false,
    aura: false,
};

/// `EF_GLOW4` (695): a green radial glow ring + green sparkle motes.
pub const GLOW4: MapZoneParams = MapZoneParams {
    rings: &[GroundRing {
        texture: "glow04.bmp",
        distance: 30.0,
        color_rgb: [105.0 / 255.0, 225.0 / 255.0, 105.0 / 255.0],
        alpha: 250.0 / 255.0,
        y_lift: -0.5,
        spin_ccw: false,
        additive: true,
    }],
    // `Map_Particle(..., F2 = 2)` tints the motes green (155,255,155).
    particles: Some(ParticleField { texture: "mpa3.tga", color_rgb: [155.0 / 255.0, 1.0, 155.0 / 255.0] }),
    pika: false,
    aura: false,
};

pub const TEXTURES: &[&str] = &["mjin2.tga", "mjin.tga", "glow04.bmp", "mpa3.tga"];

/// Persistent map effects — duration table ships `u32::MAX`.
pub const TOTAL_DURATION_MS: u32 = u32::MAX;

fn hash01(i: u32, salt: u32) -> f32 {
    let x = i
        .wrapping_mul(2_654_435_761)
        .wrapping_add(salt.wrapping_mul(40_503))
        .wrapping_add(0x9E37_79B9);
    let x = x ^ (x >> 15);
    (x % 100_000) as f32 / 100_000.0
}

pub struct MapZoneEffect {
    params: MapZoneParams,
    center: [f32; 3],
    age_frames: f32,
    pika: Option<FloorAuraEffect>,
    aura: Option<CastingRingEffect>,
}

impl MapZoneEffect {
    pub fn new(world_pos: [f32; 3], params: MapZoneParams) -> Self {
        Self {
            params,
            center: world_pos,
            age_frames: 0.0,
            pika: params.pika.then(|| FloorAuraEffect::new(world_pos, MAP_PIKA)),
            aura: params.aura.then(|| CastingRingEffect::new(world_pos, MAP_AURA)),
        }
    }

    fn ramp_in(&self) -> f32 {
        (self.age_frames / FADE_IN_FRAMES).clamp(0.0, 1.0)
    }

    fn push_rings(&self, out: &mut EffectDrawList) {
        let ramp = self.ramp_in();
        for ring in self.params.rings {
            let alpha = ring.alpha * ramp;
            if alpha <= 0.0 {
                continue;
            }
            let radius = ring.distance * 0.9 * RING_SCALE;
            let dir = if ring.spin_ccw { -1.0 } else { 1.0 };
            let rot = (self.age_frames * RING_SPIN_DEG_PER_FRAME * dir).to_radians();
            let y = self.center[1] + ring.y_lift;
            let mut corners = [[0.0f32; 3]; 4];
            for (k, corner) in corners.iter_mut().enumerate() {
                let a = rot + k as f32 * FRAC_PI_2;
                *corner = [self.center[0] + a.cos() * radius, y, self.center[2] + a.sin() * radius];
            }
            let [r, g, b] = ring.color_rgb;
            out.push(EffectPrimitiveDraw::WorldQuad {
                corners,
                uv: UNIT_UV,
                texture: ring.texture,
                color: [r, g, b, alpha],
                blend: if ring.additive { BlendKind::Additive } else { BlendKind::Alpha },
                no_depth: false,
            });
        }
    }

    fn push_particles(&self, out: &mut EffectDrawList, field: &ParticleField) {
        let ramp = self.ramp_in();
        let spin = (self.age_frames * FIELD_SPIN_DEG_PER_FRAME).to_radians();
        let [r, g, b] = field.color_rgb;
        for i in 0..PARTICLE_COUNT {
            let theta = hash01(i, 1) * TAU + spin;
            let radius = ORBIT_R_MIN + hash01(i, 2) * (ORBIT_R_MAX - ORBIT_R_MIN);
            let base_h = HEIGHT_MIN + hash01(i, 3) * (HEIGHT_MAX - HEIGHT_MIN);
            let bob = (self.age_frames * BOB_SPEED_DEG_PER_FRAME).to_radians() + hash01(i, 4) * TAU;
            let height = -(base_h + bob.sin() * BOB_AMP);
            let twinkle_phase = (self.age_frames * TWINKLE_SPEED_DEG_PER_FRAME).to_radians() + hash01(i, 5) * TAU;
            let twinkle = 0.55 + 0.45 * twinkle_phase.sin();
            let alpha = PARTICLE_PEAK_ALPHA * twinkle * ramp;
            if alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [self.center[0] + theta.cos() * radius, self.center[1] + height, self.center[2] + theta.sin() * radius],
                size: [PARTICLE_SIZE, PARTICLE_SIZE],
                uv: UNIT_UV,
                rotation: 0.0,
                texture: field.texture,
                color: [r, g, b, alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

impl Effect for MapZoneEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if let Some(p) = self.pika.as_mut() {
            p.update(ctx);
        }
        if let Some(a) = self.aura.as_mut() {
            a.update(ctx);
        }
        // Persistent — the holder despawns it via the duration table.
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        self.push_rings(out);
        if let Some(field) = &self.params.particles {
            self.push_particles(out, field);
        }
        if let Some(p) = &self.pika {
            p.collect_draws(out, ctx);
        }
        if let Some(a) = &self.aura {
            a.collect_draws(out, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 256.0, screen_h: 256.0, elapsed: 0.0 }
    }

    fn tick(e: &mut MapZoneEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx { delta: frames / FPS, camera_target: None, caster_yaw: None })
    }

    fn draws(e: &MapZoneEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn count<F: Fn(&EffectPrimitiveDraw) -> bool>(ds: &[EffectPrimitiveDraw], f: F) -> usize {
        ds.iter().filter(|d| f(d)).count()
    }
    fn is_quad(d: &EffectPrimitiveDraw) -> bool { matches!(d, EffectPrimitiveDraw::WorldQuad { .. }) }
    fn is_billboard(d: &EffectPrimitiveDraw) -> bool { matches!(d, EffectPrimitiveDraw::Billboard { .. }) }
    fn is_frustum(d: &EffectPrimitiveDraw) -> bool { matches!(d, EffectPrimitiveDraw::Frustum { .. }) }

    #[test]
    fn magiczone2_emits_two_spinning_ground_rings_and_a_mote_field() {
        let mut e = MapZoneEffect::new([4.0, 1.0, 6.0], MAP_MAGICZONE2);
        tick(&mut e, FADE_IN_FRAMES);
        let ds = draws(&e);
        assert_eq!(count(&ds, is_quad), 2, "two magic-circle ground rings");
        assert_eq!(count(&ds, is_billboard), PARTICLE_COUNT as usize, "full mote field");

        // The ground ring is flat and spins: a corner moves between two times.
        let corner0 = match ds.iter().find(|d| is_quad(d)).unwrap() {
            EffectPrimitiveDraw::WorldQuad { corners, .. } => {
                assert!(corners.iter().all(|c| (c[1] - corners[0][1]).abs() < 1e-4), "ring is flat");
                corners[0]
            }
            _ => unreachable!(),
        };
        tick(&mut e, 30.0);
        let corner1 = match draws(&e).iter().find(|d| is_quad(d)).unwrap() {
            EffectPrimitiveDraw::WorldQuad { corners, .. } => corners[0],
            _ => unreachable!(),
        };
        assert!(corner0 != corner1, "ring spins over time");
    }

    #[test]
    fn glow4_ring_and_motes_are_green_and_additive() {
        let mut e = MapZoneEffect::new([0.0; 3], GLOW4);
        tick(&mut e, FADE_IN_FRAMES);
        let ds = draws(&e);
        let ring = ds.iter().find(|d| is_quad(d)).expect("green glow ring");
        match ring {
            EffectPrimitiveDraw::WorldQuad { color, blend, .. } => {
                assert_eq!(*blend, BlendKind::Additive);
                assert!(color[1] > color[0] && color[1] > color[2], "green-dominant ring");
            }
            _ => unreachable!(),
        }
        let mote = ds.iter().find(|d| is_billboard(d)).expect("green motes");
        match mote {
            EffectPrimitiveDraw::Billboard { color, blend, .. } => {
                assert_eq!(*blend, BlendKind::Additive);
                assert!(color[1] > color[0] && color[1] > color[2], "green-dominant mote");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn magiczone_650_composes_pika_floor_and_flared_aura() {
        let mut e = MapZoneEffect::new([0.0; 3], MAP_MAGICZONE);
        tick(&mut e, FADE_IN_FRAMES);
        let ds = draws(&e);
        // Two mjin rings + two pikapika2 floor quads (FloorAura) = 4 WorldQuads,
        // three ring_blue Frustums (CastingRing), and no mote field.
        assert_eq!(count(&ds, is_quad), 4, "mjin rings + pika sparkle floor");
        assert_eq!(count(&ds, is_frustum), 3, "three flared aura rings");
        assert_eq!(count(&ds, is_billboard), 0, "650 has no mote field");
    }

    #[test]
    fn persistent_never_self_terminates() {
        let mut e = MapZoneEffect::new([0.0; 3], MAP_MAGICZONE2);
        for _ in 0..200 {
            assert_eq!(tick(&mut e, 10.0), EffectStatus::Running);
        }
    }
}
