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
                gravity_world_per_sec2: 0.0,
                cone_latitude_deg: None,
                size_shrink: false,
                twinkle: false,
            },
        )),
        // EnchantPoison: every 5 frames a single
        // particle on a flat disc around the master (radius rand(6)+2,
        // size 0.4..0.9, particle3.spr). The original recipe is shared by
        // three ids: EF_PATTACK (1000 ms parent), EF_ENCHANTPOISON
        // (2500 ms), EF_ENCHANTPOISON_FLOW (250 ms — re-anchors each
        // frame, which our SprBurst pipeline already
        // does because particles snapshot positions at spawn). Parent
        // duration comes from `default_duration_ms`, not this struct.
        EffectId::Pattack
        | EffectId::Enchantpoison
        | EffectId::EnchantpoisonFlow => Some((
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
                cone_latitude_deg: Some((-90.0, 90.0)),
                size_shrink: false,
                twinkle: true,
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
                // -90+40..-90+140 in latitudes = -50..50 from the
                // horizontal plane — most particles go up, some sideways.
                cone_latitude_deg: Some((40.0, 140.0)),
                size_shrink: true,
                twinkle: false,
            },
        )),
        _ => None,
    }
}
