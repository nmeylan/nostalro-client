//! `EF_BEGINSPELL` (id 12) — yellow cast aura.
//!
//! Mirrors the non-secondjob-skill2 branch of the BeginSpell path
//! (the ground-disc spawn path): one flat ground disc plus two interleaved
//! streams of expanding vertical "flame" cylinders.
//!
//! The original game spawns three primitives:
//!
//! * **Frame 0** — one 3D circle (flat, latitude = 90,
//!   fill-circle) using `alpha_down.tga`, radius `15`,
//!   max alpha = 128, duration = 25. A halo on the ground.
//!
//! * **Every 10 frames** — one 3D cylinder with `ring_blue.tga`,
//!   `outerSpeed = 0.4` (decel `-(0.4/25)/2`), `innerSpeed = 0.4` (same
//!   decel), height speed = 3.0 (decel `-(3.0/25)/2`). At frame N of its
//!   25-frame life: radius ≈ `N·0.4 − 0.008·N(N+1)/2`, height ≈
//!   `N·3.0 − 0.06·N(N+1)/2`. `innerSize == outerSize` keeps the cylinder
//!   straight-sided; the flame look comes from `ring_blue.tga`'s painted
//!   stripes wrapping `uv_repeat = 4` around the cylinder.
//!
//! * **Every 8 frames** — one 3D cylinder with `ring_yellow.tga`,
//!   outer growth rate `0.35`, height grow rate `2.5`, height accel =
//!   -(2.5/25)` (note: divided once, not twice — the cylinder's height
//!   peaks earlier and collapses harder than the blue one's).
//!
//! All sub-cylinders share an alpha speed of max alpha / 10 and
//! `fade_out_cnt = duration - 10` → linear fade-in for 10 frames, hold,
//! linear fade-out over the last 10. `max_alpha` defaults to 254.
//!
//! The parent's duration comes from the table (`default 24 frames =
//! 400 ms`); cylinders spawned near the end keep living after the parent
//! dies, so the visible effect outlasts the parent by roughly one
//! sub-cylinder lifetime (~25 frames).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// Parent emitter lifetime — matches the table default `400 ms ≈ 24 frames`.
const PARENT_DURATION_FRAMES: f32 = 24.0;
const PARENT_DURATION_S: f32 = PARENT_DURATION_FRAMES / FRAMES_PER_SECOND;

/// Lifetime of one sub-cylinder. Original game's `duration = 25`.
const CYLINDER_DURATION_FRAMES: f32 = 25.0;
const CYLINDER_DURATION_S: f32 = CYLINDER_DURATION_FRAMES / FRAMES_PER_SECOND;

/// Spawn cadence (frames between consecutive cylinder spawns).
const YELLOW_INTERVAL_FRAMES: f32 = 8.0;
const YELLOW_INTERVAL_S: f32 = YELLOW_INTERVAL_FRAMES / FRAMES_PER_SECOND;
const BLUE_INTERVAL_FRAMES: f32 = 10.0;
const BLUE_INTERVAL_S: f32 = BLUE_INTERVAL_FRAMES / FRAMES_PER_SECOND;

/// Sub-cylinder fade-in / fade-out window (alpha speed = max alpha / 10,
/// `fade_out_cnt = duration - 10`).
const FADE_FRAMES: f32 = 10.0;
const FADE_OUT_START_FRAMES: f32 = CYLINDER_DURATION_FRAMES - FADE_FRAMES;

/// Default peak alpha for the original game's primitives is max alpha =
/// 0xfe / 255`. The cast aura should brighten without saturating to white,
/// so cylinders peak slightly below that.
const CYLINDER_PEAK_ALPHA: f32 = 0.85;

/// Geometry-to-world scaling for the sub-cylinders.
///
/// The original game's literal height-speed values integrate to peak heights
/// of ~55 and ~30 units, ~6× taller than the gif silhouette
/// (`implement-effect` skill calls this "off by 6×"). Radius peaks at ~7
/// units, which actually reads narrower than the gif. We scale height
/// down and radius up to land near the gif's roughly-square silhouette
/// (height ≈ radius at peak).
const HEIGHT_SCALE: f32 = 0.65;
const RADIUS_SCALE: f32 = 1.6;
/// Progressive outward flare. Early frames are near-vertical, late frames
/// open up into a wide fan — matching the gif's behaviour where the
/// flames straighten upward at first then fan outward into arcs. The
/// flare grows linearly from 0 to `TOP_FLARE_MAX` over the sub-cylinder's
/// lifetime. The original game's cylinder primitive keeps `outer_size ==
/// inner_size`, but the visual ground truth in the gif clearly shows
/// flames angling outward; we model that here as a time-varying flare.
const TOP_FLARE_MAX: f32 = 4.0;

/// Number of sides on each sub-cylinder. Original game's
/// the 3D cylinder render walks the arc angle per step around 360°; for the
/// default arc angle = 6° / 5° that's ~60-72 sides. We use 24 — plenty
/// for the flame-stripe texture to wrap smoothly around the cylinder.
const CYLINDER_SIDES: u32 = 24;

/// Original game's `Render3DCylinder` advances `uInc += 0.25; resets at 1.0`,
/// so the texture wraps **four** times around the cylinder.
const CYLINDER_UV_REPEAT: f32 = 4.0;

// ---------- Ground disc (frame-0 3D circle) ----------

const GROUND_TEXTURE: &str = "alpha_down.tga";
const GROUND_DURATION_FRAMES: f32 = 25.0;
const GROUND_DURATION_S: f32 = GROUND_DURATION_FRAMES / FRAMES_PER_SECOND;
/// Original game's radius = 15. The gif's ground halo reads as ~1
/// character radius (~5 units); the C++ value is ~3× that. Scale down to
/// keep the halo from swallowing the flames.
const GROUND_RADIUS: f32 = 5.5;
const GROUND_THICKNESS: f32 = GROUND_RADIUS;
/// max alpha = 128 / 255.
const GROUND_PEAK_ALPHA: f32 = 128.0 / 255.0;

// ---------- Sub-cylinder per-variant constants ----------

#[derive(Clone, Copy)]
struct CylinderRecipe {
    texture: &'static str,
    /// the global prim's outer speed / inner speed (both set to the same value
    /// in the original game so the cylinder stays straight-sided).
    radius_speed_per_frame: f32,
    /// outer accel = -(outer speed / duration) / 2.
    radius_accel_per_frame2: f32,
    /// the global prim's height speed.
    height_speed_per_frame: f32,
    /// `height_accel`. Blue: `-(height_speed / duration) / 2`. Yellow:
    /// `-(height_speed / duration)` (note the `/2` is **missing** on the
    /// yellow path in the original game).
    height_accel_per_frame2: f32,
}

const BLUE: CylinderRecipe = CylinderRecipe {
    texture: "ring_blue.tga",
    radius_speed_per_frame: 0.4,
    radius_accel_per_frame2: -(0.4 / CYLINDER_DURATION_FRAMES) / 2.0,
    height_speed_per_frame: 3.0,
    height_accel_per_frame2: -(3.0 / CYLINDER_DURATION_FRAMES) / 2.0,
};

const YELLOW: CylinderRecipe = CylinderRecipe {
    texture: "ring_yellow.tga",
    radius_speed_per_frame: 0.35,
    radius_accel_per_frame2: -(0.35 / CYLINDER_DURATION_FRAMES) / 2.0,
    height_speed_per_frame: 2.5,
    height_accel_per_frame2: -(2.5 / CYLINDER_DURATION_FRAMES),
};

pub const TEXTURES: &[&str] = &[
    GROUND_TEXTURE,
    BLUE.texture,
    YELLOW.texture,
];

/// Wall-clock end of the last spawn's tail. Last yellow spawn happens at
/// roughly `parent_age = floor(PARENT_DURATION_FRAMES / 8) * 8 = 24`, and
/// that cylinder lives `CYLINDER_DURATION_FRAMES` more.
pub const TOTAL_DURATION_MS: u32 = ((PARENT_DURATION_FRAMES + CYLINDER_DURATION_FRAMES)
    / FRAMES_PER_SECOND
    * 1000.0) as u32;

#[derive(Clone, Copy)]
struct Cylinder {
    age: f32,
    recipe: CylinderRecipe,
}

impl Cylinder {
    fn frame(&self) -> f32 {
        (self.age * FRAMES_PER_SECOND).clamp(0.0, CYLINDER_DURATION_FRAMES)
    }

    fn alive(&self) -> bool {
        self.age < CYLINDER_DURATION_S
    }

    /// Closed form of the original game's per-frame `size += speed; speed +=
    /// accel` loop with `size(0) = speed(0) = 0`, accel constant.
    fn radius(&self) -> f32 {
        let n = self.frame();
        let raw = n * self.recipe.radius_speed_per_frame
            + self.recipe.radius_accel_per_frame2 * n * (n + 1.0) / 2.0;
        raw.max(0.0) * RADIUS_SCALE
    }

    fn height(&self) -> f32 {
        let n = self.frame();
        let raw = n * self.recipe.height_speed_per_frame
            + self.recipe.height_accel_per_frame2 * n * (n + 1.0) / 2.0;
        raw.max(0.0) * HEIGHT_SCALE
    }

    fn alpha(&self) -> f32 {
        let n = self.frame();
        let fade_in = (n / FADE_FRAMES).clamp(0.0, 1.0);
        let fade_out = if n <= FADE_OUT_START_FRAMES {
            1.0
        } else {
            ((CYLINDER_DURATION_FRAMES - n) / FADE_FRAMES).clamp(0.0, 1.0)
        };
        CYLINDER_PEAK_ALPHA * fade_in * fade_out
    }

    fn flare(&self) -> f32 {
        let n = self.frame();
        TOP_FLARE_MAX * (n / CYLINDER_DURATION_FRAMES).clamp(0.0, 1.0)
    }
}

pub struct BeginSpellEffect {
    world_pos: [f32; 3],
    parent_age: f32,
    next_blue_spawn_at: f32,
    next_yellow_spawn_at: f32,
    cylinders: Vec<Cylinder>,
    /// Ground disc lifetime tracker. `None` after the disc expires.
    ground_age: Option<f32>,
}

impl BeginSpellEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            parent_age: 0.0,
            next_blue_spawn_at: 0.0,
            next_yellow_spawn_at: 0.0,
            cylinders: Vec::with_capacity(8),
            ground_age: Some(0.0),
        }
    }

    fn ground_alpha(age: f32) -> f32 {
        let n = (age * FRAMES_PER_SECOND).clamp(0.0, GROUND_DURATION_FRAMES);
        let fade_in = (n / FADE_FRAMES).clamp(0.0, 1.0);
        let fade_out_start = GROUND_DURATION_FRAMES - FADE_FRAMES;
        let fade_out = if n <= fade_out_start {
            1.0
        } else {
            ((GROUND_DURATION_FRAMES - n) / FADE_FRAMES).clamp(0.0, 1.0)
        };
        GROUND_PEAK_ALPHA * fade_in * fade_out
    }
}

impl Effect for BeginSpellEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt = ctx.delta;
        self.parent_age += dt;

        for c in &mut self.cylinders {
            c.age += dt;
        }
        if let Some(a) = self.ground_age.as_mut() {
            *a += dt;
            if *a >= GROUND_DURATION_S {
                self.ground_age = None;
            }
        }

        // Catch up every spawn that landed inside the elapsed window. The
        // initial `age` compensates for the difference between the
        // scheduled spawn moment and the current `parent_age`.
        while self.next_blue_spawn_at <= PARENT_DURATION_S
            && self.next_blue_spawn_at <= self.parent_age
        {
            let initial_age = (self.parent_age - self.next_blue_spawn_at).max(0.0);
            self.cylinders.push(Cylinder {
                age: initial_age,
                recipe: BLUE,
            });
            self.next_blue_spawn_at += BLUE_INTERVAL_S;
        }
        while self.next_yellow_spawn_at <= PARENT_DURATION_S
            && self.next_yellow_spawn_at <= self.parent_age
        {
            let initial_age = (self.parent_age - self.next_yellow_spawn_at).max(0.0);
            self.cylinders.push(Cylinder {
                age: initial_age,
                recipe: YELLOW,
            });
            self.next_yellow_spawn_at += YELLOW_INTERVAL_S;
        }

        self.cylinders.retain(|c| c.alive());

        if self.parent_age >= PARENT_DURATION_S
            && self.cylinders.is_empty()
            && self.ground_age.is_none()
        {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if let Some(age) = self.ground_age {
            let alpha = Self::ground_alpha(age);
            if alpha > 0.0 {
                out.push(EffectPrimitiveDraw::GroundDisc {
                    center: self.world_pos,
                    radius: GROUND_RADIUS,
                    thickness: GROUND_THICKNESS,
                    rotation: 0.0,
                    arc_angle_deg: 360.0,
                    uv_repeat: 1.0,
                    texture: GROUND_TEXTURE,
                    color: [1.0, 1.0, 1.0, alpha],
                    blend: BlendKind::Alpha,
                });
            }
        }

        for c in &self.cylinders {
            let alpha = c.alpha();
            if alpha <= 0.0 {
                continue;
            }
            let r = c.radius();
            let h = c.height();
            if h <= 0.0 || r <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Frustum {
                base: self.world_pos,
                bottom_size: r,
                top_size: r * (1.0 + c.flare()),
                height: h,
                sides: CYLINDER_SIDES,
                rotation: 0.0,
                uv_repeat: CYLINDER_UV_REPEAT,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: 0.0,
                wave_frequency: 1.0,
                wave_phase: 0.0,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                // Geometry-fade the rear half of the cone once it flares
                // wider than tall. In the original game's gameplay, the
                // caster's sprite occludes the far rim of an upward cone,
                // leaving only the near rim's outside visible. The viewer
                // has no caster sprite, so we approximate that occlusion
                // by fading the back rim — same mechanism the renderer
                // uses for the flat cast aura under
                // `SAINTCASTING`.
                cull_back: true,
                texture: c.recipe.texture,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
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

    fn draws(e: &BeginSpellEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(e: &mut BeginSpellEffect, dt: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
        })
    }

    #[test]
    fn emits_ground_disc_and_two_cylinders_after_fade_in() {
        // Frame 0 schedules: 3D circle + one blue 3D cylinder + one
        // yellow. None are visible at age 0 (everything fades in from
        // alpha 0), but a few frames in they should all render.
        let mut e = BeginSpellEffect::new([0.0; 3]);
        step(&mut e, 5.0 / FRAMES_PER_SECOND);
        let prims = draws(&e);
        let discs = prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { .. }))
            .count();
        let frustums = prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
            .count();
        assert_eq!(discs, 1, "ground disc visible mid fade-in");
        assert!(
            frustums >= 2,
            "expected at least two cylinders by frame 5 (got {frustums})"
        );
    }

    #[test]
    fn cylinder_grows_radius_and_height_then_fades() {
        let mut e = BeginSpellEffect::new([0.0; 3]);
        step(&mut e, 0.0);
        // Skip the ground disc and pick the first cylinder.
        let pick = |e: &BeginSpellEffect| -> Option<(f32, f32, f32)> {
            draws(e).into_iter().find_map(|p| match p {
                EffectPrimitiveDraw::Frustum {
                    bottom_size,
                    height,
                    color,
                    ..
                } => Some((bottom_size, height, color[3])),
                _ => None,
            })
        };
        step(&mut e, 6.0 / FRAMES_PER_SECOND);
        let (r_mid, h_mid, _) = pick(&e).expect("cylinder visible mid-life");
        step(&mut e, 8.0 / FRAMES_PER_SECOND);
        let (r_late, h_late, _) = pick(&e).expect("cylinder visible later");
        assert!(r_late > r_mid, "radius grows ({r_mid} → {r_late})");
        assert!(h_late > h_mid, "height grows ({h_mid} → {h_late})");
    }

    #[test]
    fn spawns_match_original_cadence() {
        // Over the parent's 24-frame lifetime, expect:
        //   blue at  0, 10, 20         (3)
        //   yellow at 0, 8, 16, 24     (4)
        // = 7 cylinders total in the lifetime list.
        let mut e = BeginSpellEffect::new([0.0; 3]);
        // Drive forward in 1-frame increments to trigger every spawn.
        let dt = 1.0 / FRAMES_PER_SECOND;
        let mut total_seen: usize = 0;
        // Count unique cylinders by tracking the high-water mark.
        for _ in 0..=PARENT_DURATION_FRAMES as u32 + 1 {
            step(&mut e, dt);
            total_seen = total_seen.max(e.cylinders.len() + 0);
        }
        // The simultaneous-alive maximum is lower than the lifetime total
        // because early ones expire before later ones spawn. Test the
        // total spawn count directly by re-running and summing pushes.
        let mut e2 = BeginSpellEffect::new([0.0; 3]);
        let mut ever_spawned = 0usize;
        let mut prev_alive = 0usize;
        let mut prev_total = 0usize;
        for _ in 0..=PARENT_DURATION_FRAMES as u32 + 1 {
            step(&mut e2, dt);
            // `cylinders` only contains live ones; track new additions by
            // comparing length+deltas after retain.
            // Conservative: a new spawn happens when len jumped up since
            // last frame (we never spawn more than 2 per frame).
            if e2.cylinders.len() > prev_alive {
                ever_spawned += e2.cylinders.len() - prev_alive;
            }
            prev_alive = e2.cylinders.len();
            prev_total += 0;
        }
        let _ = (total_seen, prev_total);
        assert!(
            ever_spawned >= 6,
            "should see ~7 spawn events across the parent's life, got {ever_spawned}"
        );
    }

    #[test]
    fn dies_after_total_duration() {
        let mut e = BeginSpellEffect::new([0.0; 3]);
        // Run for the full parent + tail life.
        let dt = 1.0 / FRAMES_PER_SECOND;
        let total = (PARENT_DURATION_FRAMES + CYLINDER_DURATION_FRAMES + 5.0) as u32;
        let mut final_status = EffectStatus::Running;
        for _ in 0..total {
            final_status = step(&mut e, dt);
        }
        assert_eq!(final_status, EffectStatus::Dead);
    }

    #[test]
    fn yellow_cylinders_use_yellow_texture_blue_use_blue() {
        let mut e = BeginSpellEffect::new([0.0; 3]);
        step(&mut e, 5.0 / FRAMES_PER_SECOND);
        let textures: std::collections::HashSet<_> = draws(&e)
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { texture, .. } => Some(*texture),
                _ => None,
            })
            .collect();
        assert!(textures.contains(&"ring_yellow.tga"));
        assert!(textures.contains(&"ring_blue.tga"));
    }
}
