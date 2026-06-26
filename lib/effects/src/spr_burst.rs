//! Per-`EffectId` SprBurst parameters.
//!
//! Returning `Some((sprite, params))` routes the id through
//! `EffectSpec::SprBurst` in `bucket_default`. The parent emitter's lifetime
//! comes from [`default_duration_ms`](super::table::default_duration_ms);
//! per-particle settings live in [`SprBurstParams`].

use models::enums::effect_id::EffectId;

use super::spec::{AlphaKeyframe, CurveParams, SprBurstParams};

/// Firefly's alpha-keyframe schedule from the original game:
/// frame 40 → (alpha 120/255, max 200/255); frame 100 → (0, 80/255).
/// The implicit frame-0 entry (alpha 0, max 80/255) primes the
/// sawtooth at spawn so the dim opening pulse matches.
const FIREFLY_ALPHA_KEYFRAMES: &[AlphaKeyframe] = &[
    AlphaKeyframe {
        at_frame: 0,
        alpha_init: 0.0,
        alpha_max: 80.0 / 255.0,
    },
    AlphaKeyframe {
        at_frame: 40,
        alpha_init: 120.0 / 255.0,
        alpha_max: 200.0 / 255.0,
    },
    AlphaKeyframe {
        at_frame: 100,
        alpha_init: 0.0,
        alpha_max: 80.0 / 255.0,
    },
];

pub fn spr_burst_params(id: EffectId) -> Option<(&'static str, SprBurstParams)> {
    match id {
        // Smoke: 1..=4 particles at frame 0; each drifts upward
        // (y offset -9) at speed (rand(3)+3)/10 with size 1.5; fade-out
        // starts at duration*2/3. One-shot.
        EffectId::Smoke => Some((
            "data/sprite/이팩트/굴뚝연기",
            SprBurstParams {
                particle_lifetime_ms: 833.0,
                size: 1.5,
                alpha_max: 1.0,
                burst_count_range: (1, 4),
                speed_range: (0.3, 0.6),
                anim_speed: 4.0,
                pos_y_start: -9.0,
                spawn_radius_xz: 0.0,
                period_frames: None,
                follow_camera: false,
                gravity_world_per_sec2: 0.0,
                cone_latitude_deg: None,
                size_shrink: false,
                twinkle: false,
                curve: None,
                alpha_keyframes: &[],
            },
        )),
        // Dragonsmoke is implemented as a Custom trail effect
        // (`effects/dragonsmoke.rs`) — it needs the caster→target trail
        // to lean the smoke column along the wind direction, which a
        // SprBurst entry can't express.
        // EnchantPoison: every 5 frames a single
        // particle on a flat disc around the master (radius rand(6)+2,
        // size 0.4..0.9, particle3.spr). The original recipe is shared by
        // three ids: EF_PATTACK (1000 ms parent), EF_ENCHANTPOISON
        // (2500 ms), EF_ENCHANTPOISON_FLOW (250 ms — re-anchors each
        // frame, which our SprBurst pipeline already
        // does because particles snapshot positions at spawn). Parent
        // duration comes from `default_duration_ms`, not this struct.
        EffectId::Pattack | EffectId::Enchantpoison | EffectId::EnchantpoisonFlow => Some((
            "data/sprite/이팩트/particle3",
            SprBurstParams {
                particle_lifetime_ms: 666.0,
                size: 0.65,
                alpha_max: 1.0,
                burst_count_range: (1, 1),
                speed_range: (0.3, 0.6),
                anim_speed: 4.0,
                pos_y_start: 0.0,
                spawn_radius_xz: 8.0,
                period_frames: Some(5),
                follow_camera: false,
                gravity_world_per_sec2: 0.0,
                cone_latitude_deg: None,
                size_shrink: false,
                twinkle: false,
                curve: None,
                alpha_keyframes: &[],
            },
        )),
        // Detoxication: every 5 frames a single particle on a flat
        // disc around the master (radius rand(6)+2), small upward drift,
        // size ~0.6. Total parent lifetime is 1000 ms.
        EffectId::Detoxication => Some((
            "data/sprite/이팩트/particle2",
            SprBurstParams {
                particle_lifetime_ms: 666.0,
                size: 0.6,
                alpha_max: 1.0,
                burst_count_range: (1, 1),
                speed_range: (0.3, 0.6),
                anim_speed: 4.0,
                pos_y_start: 0.0,
                spawn_radius_xz: 8.0,
                period_frames: Some(5),
                follow_camera: false,
                gravity_world_per_sec2: 0.0,
                cone_latitude_deg: None,
                size_shrink: false,
                twinkle: false,
                curve: None,
                alpha_keyframes: &[],
            },
        )),
        // Snow: 2 particles per frame spawned around the player
        // at radius up to 300, height -100, drifting down
        // at speed -0.5 over 320 frames. Negative `speed_range` flips the
        // sign in `spawn_burst`, giving downward Y motion. `follow_camera`
        // makes each spawn re-anchor to the active view, blanketing whatever
        // the player is looking at instead of just the origin.
        EffectId::Snow => Some((
            "data/sprite/이팩트/ef_snow",
            SprBurstParams {
                particle_lifetime_ms: 5333.0,
                size: 0.7,
                alpha_max: 0.6,
                burst_count_range: (2, 2),
                speed_range: (-0.5, -0.5),
                anim_speed: 4.0,
                pos_y_start: -100.0,
                spawn_radius_xz: 300.0,
                period_frames: Some(1),
                follow_camera: true,
                gravity_world_per_sec2: 0.0,
                cone_latitude_deg: None,
                size_shrink: false,
                twinkle: false,
                curve: None,
                alpha_keyframes: &[],
            },
        )),
        // FireFly: one particle at random 3D offset,
        // drifting in a random direction with a curved path (periodic
        // re-randomized heading) and twinkling (pulsing alpha). We
        // approximate by: cone direction over the full sphere for the
        // 3D drift, twinkle flag for the pulse, and the random XZ
        // spawn-disc for the initial offset (close enough — the original
        // game's "spawn anywhere on a sphere of radius 2..15" reads
        // similar in a viewer).
        EffectId::Firefly => Some((
            "data/sprite/이팩트/particle1",
            SprBurstParams {
                particle_lifetime_ms: 2333.0,
                size: 0.65,
                alpha_max: 0.8,
                burst_count_range: (1, 1),
                speed_range: (0.15, 0.5),
                anim_speed: 2.0,
                pos_y_start: -10.0,
                spawn_radius_xz: 8.0,
                period_frames: None,
                follow_camera: false,
                gravity_world_per_sec2: 0.0,
                // Firefly's random longitude/latitude in the original
                // game picks an isotropic 3D
                // direction. `cone_latitude_deg` here is in the spawn
                // formula's lat space where `vy = cos(lat°)`, so
                // (0, 180) spans the full vertical hemisphere
                // (cos(0)=+1 down to cos(180)=-1 up); paired with
                // random longitude that's a full sphere.
                cone_latitude_deg: Some((0.0, 180.0)),
                size_shrink: false,
                twinkle: true,
                curve: Some(CurveParams {
                    initial_period_frames: (5, 30),
                    subsequent_period_frames: (5, 15),
                    angle_jitter_deg: 40.0,
                    speed_resample: true,
                }),
                alpha_keyframes: FIREFLY_ALPHA_KEYFRAMES,
            },
        )),
        // Steal: 10 gravity-driven particles at frame 0;
        // each fires upward-ish at random longitude with latitude in the
        // upper hemisphere, slows under gravity (initial rise speed
        // -0.6..-1.5, accel positive), shrinks to 0 over duration (500ms
        // default). Visible effect: gold coins flying out of the
        // monster's chest and falling back.
        EffectId::Steal => Some((
            "data/sprite/이팩트/particle7",
            SprBurstParams {
                particle_lifetime_ms: 500.0,
                size: 1.5,
                alpha_max: 1.0,
                burst_count_range: (10, 10),
                speed_range: (0.5, 1.0),
                anim_speed: 4.0,
                pos_y_start: -10.0,
                spawn_radius_xz: 0.0,
                period_frames: None,
                follow_camera: false,
                // The original game's gravity accel plus initial rise
                // speed integrate to particles slowing as they rise then falling.
                // Net effect in native RO coords (positive Y = down): a
                // ~30 unit/s² downward pull is a visible arc over the
                // 500 ms lifetime.
                gravity_world_per_sec2: 36.0,
                // -90+40..-90+140 in original game latitudes = -50..50 from the
                // horizontal plane — most particles go up, some sideways.
                cone_latitude_deg: Some((40.0, 140.0)),
                size_shrink: true,
                twinkle: false,
                curve: None,
                alpha_keyframes: &[],
            },
        )),
        // SlowPoison: every 5 frames a single
        // particle on a flat disc around the master (radius
        // 2..8), spawn 20 above ground, negative speed so the
        // particle drifts downward. particle3.spr like the rest of
        // the poison family. Master duration ≈ 80 frames in the original game.
        EffectId::Slowpoison => Some((
            "data/sprite/이팩트/particle3",
            SprBurstParams {
                particle_lifetime_ms: 666.0,
                size: 0.65,
                alpha_max: 1.0,
                burst_count_range: (1, 1),
                speed_range: (-0.6, -0.3),
                anim_speed: 4.0,
                pos_y_start: -20.0,
                spawn_radius_xz: 8.0,
                period_frames: Some(5),
                follow_camera: false,
                gravity_world_per_sec2: 0.0,
                cone_latitude_deg: None,
                size_shrink: false,
                twinkle: false,
                curve: None,
                alpha_keyframes: &[],
            },
        )),
        // Note: EF_COIN (id 10) is hybrid — the STR `Maemor` carries the
        // visible ground rings in the reference gif (`0-50/10.gif`) and
        // the gravity-driven coins are secondary. Routing it via
        // SprBurst here would lose the STR, so it stays on the default
        // STR path until a Custom hybrid impl is written.
        // EnchantPoison2 (= EF_EDP): every 3 frames
        // a single particle on the standard particle3.spr disc.
        // Smaller particles (0.2..0.4) and faster cadence than the
        // base EnchantPoison entry, so this gets its own params.
        EffectId::Edp => Some((
            "data/sprite/이팩트/particle3",
            SprBurstParams {
                particle_lifetime_ms: 666.0,
                size: 0.3,
                alpha_max: 1.0,
                burst_count_range: (1, 1),
                speed_range: (0.3, 0.8),
                anim_speed: 4.0,
                pos_y_start: 0.0,
                spawn_radius_xz: 8.0,
                period_frames: Some(3),
                follow_camera: false,
                gravity_world_per_sec2: 0.0,
                cone_latitude_deg: None,
                size_shrink: false,
                twinkle: false,
                curve: None,
                alpha_keyframes: &[],
            },
        )),
        _ => None,
    }
}
