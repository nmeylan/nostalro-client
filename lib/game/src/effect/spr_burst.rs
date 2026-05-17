//! Per-`EffectId` SprBurst parameters.
//!
//! Returning `Some((sprite, params))` routes the id through
//! `EffectSpec::SprBurst` in `bucket_default`. The parent emitter's lifetime
//! comes from [`default_duration_ms`](super::table::default_duration_ms);
//! per-particle settings live in [`SprBurstParams`].

use models::enums::effect_id::EffectId;

use super::spec::SprBurstParams;

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
            },
        )),
        // Original game `FireFly()`: one particle at random spherical offset (radius
        // 2..15), brief glow that fades. The curve/twinkle paths aren't
        // simulated yet — the result is a stationary sparkle, which still
        // reads as a firefly puff in a viewer test.
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
            },
        )),
        _ => None,
    }
}
