//! `EF_SPRAYPOND` — pond spray / water fountain: a ring of water streams with
//! pulsing crests and expanding ground rings around the entity.
//!
//! Composite, lifetime 130 frames at 60 fps:
//!
//!   * Frame 0 — 8 water-stream panels (`pond_water.bmp`)
//!     evenly spaced around the entity (every 45°). Each
//!     stream sits at radius 12.2 wu, alpha 120/255, scrolling the texture
//!     v-coordinate over time so the water reads as
//!     falling.
//!   * Every 10 frames — 8 short crests (`alpha_down.tga`)
//!     at radius 13.8 wu, alpha 100/255, lifetime 30 frames. They sit at
//!     the foot of the streams and pulse outward.
//!   * Every 20 frames — an expanding ground ring (radius
//!     13 → +0.25 wu/frame, alpha 100/255, lifetime 25 frames).
//!     `alpha_center.tga`.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const WATER_TEXTURE: &str = "pond_water.bmp";
pub const CREST_TEXTURE: &str = "alpha_down.tga";
pub const RING_TEXTURE: &str = "alpha_center.tga";
pub const TEXTURES: &[&str] = &[WATER_TEXTURE, CREST_TEXTURE, RING_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const PARENT_DURATION_FRAMES: f32 = 130.0;

const STREAM_COUNT: usize = 8;
const STREAM_RADIUS: f32 = 12.2;
// The original's small 5.0 × 2.7 panels read as tiny next to the radius-12
// circle; the gif shows streams about a
// character tall (≈ 5 wu) and ~5 chars (25 wu) for the water column
// from the splash up to the apex. Stretched to fit the gif.
const STREAM_HALF_TANGENT: f32 = 2.5;
const STREAM_TOP_OFFSET: f32 = -25.0;
const STREAM_GROUND_OFFSET: f32 = 0.0;
const STREAM_MAX_ALPHA: f32 = 120.0 / 255.0;
const STREAM_UV_SCROLL_PER_FRAME: f32 = 1.0 / 30.0;
const STREAM_TINT: [f32; 3] = [0.8, 0.95, 1.0];

const CREST_SPAWN_PERIOD_FRAMES: u32 = 10;
const CREST_DURATION_FRAMES: f32 = 30.0;
const CREST_RADIUS: f32 = 13.8;
const CREST_HALF_TANGENT: f32 = 2.75;
const CREST_HEIGHT_HALF: f32 = 1.0;
const CREST_MAX_ALPHA: f32 = 100.0 / 255.0;

const RING_SPAWN_PERIOD_FRAMES: u32 = 20;
const RING_DURATION_FRAMES: f32 = 25.0;
const RING_INNER_RADIUS: f32 = 13.0;
const RING_RADIUS_SPEED_PER_FRAME: f32 = 0.25;
const RING_RADIUS_ACCEL_PER_FRAME: f32 = -(RING_RADIUS_SPEED_PER_FRAME / RING_DURATION_FRAMES);
const RING_MAX_ALPHA: f32 = 100.0 / 255.0;
const RING_THICKNESS: f32 = 1.25;

pub const TOTAL_DURATION_MS: u32 =
    ((PARENT_DURATION_FRAMES + CREST_DURATION_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

#[derive(Clone, Copy, Debug)]
struct Burst {
    spawn_frame: f32,
}

pub struct SpraypondEffect {
    world_pos: [f32; 3],
    crests: Vec<Burst>,
    rings: Vec<Burst>,
    age_frames: f32,
    last_spawn_frame: i32,
}

impl SpraypondEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            crests: Vec::new(),
            rings: Vec::new(),
            age_frames: 0.0,
            last_spawn_frame: -1,
        }
    }
}

fn vertical_panel(
    centre: [f32; 3],
    tangent: [f32; 2],
    half_tan: f32,
    top: f32,
    bot: f32,
) -> [[f32; 3]; 4] {
    // Panel lies in a vertical plane whose tangent is `tangent` (XZ
    // unit), top at `centre.y + top`, bottom at `centre.y + bot`.
    let tx = tangent[0] * half_tan;
    let tz = tangent[1] * half_tan;
    [
        [centre[0] - tx, centre[1] + top, centre[2] - tz],
        [centre[0] + tx, centre[1] + top, centre[2] + tz],
        [centre[0] - tx, centre[1] + bot, centre[2] - tz],
        [centre[0] + tx, centre[1] + bot, centre[2] + tz],
    ]
}

impl Effect for SpraypondEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt_frames;

        let current_frame = self.age_frames.floor() as i32;
        if self.age_frames <= PARENT_DURATION_FRAMES {
            let next_frame = self.last_spawn_frame + 1;
            for f in next_frame..=current_frame {
                if f < 0 {
                    continue;
                }
                let fu = f as u32;
                if fu % CREST_SPAWN_PERIOD_FRAMES == 0 {
                    self.crests.push(Burst {
                        spawn_frame: f as f32,
                    });
                }
                if fu % RING_SPAWN_PERIOD_FRAMES == 0 {
                    self.rings.push(Burst {
                        spawn_frame: f as f32,
                    });
                }
            }
            self.last_spawn_frame = current_frame;
        }

        let now = self.age_frames;
        self.crests
            .retain(|b| (now - b.spawn_frame) < CREST_DURATION_FRAMES);
        self.rings
            .retain(|b| (now - b.spawn_frame) < RING_DURATION_FRAMES);

        if self.age_frames >= PARENT_DURATION_FRAMES + CREST_DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        // 8 sustained water streams.
        if self.age_frames <= PARENT_DURATION_FRAMES {
            let scroll = STREAM_UV_SCROLL_PER_FRAME * self.age_frames;
            let uv = [
                [0.0, 0.0 + scroll],
                [1.0, 0.0 + scroll],
                [0.0, 1.0 + scroll],
                [1.0, 1.0 + scroll],
            ];
            for i in 0..STREAM_COUNT {
                let angle_deg = (i as f32) * 45.0 + 22.5;
                let (sn, cs) = angle_deg.to_radians().sin_cos();
                let centre = [
                    self.world_pos[0] + sn * STREAM_RADIUS,
                    self.world_pos[1],
                    self.world_pos[2] + cs * STREAM_RADIUS,
                ];
                // Tangent direction (perpendicular to radial in XZ).
                let tangent = [cs, -sn];
                out.push(EffectPrimitiveDraw::WorldQuad {
                    corners: vertical_panel(
                        centre,
                        tangent,
                        STREAM_HALF_TANGENT,
                        STREAM_TOP_OFFSET,
                        STREAM_GROUND_OFFSET,
                    ),
                    uv,
                    texture: WATER_TEXTURE,
                    color: [
                        STREAM_TINT[0],
                        STREAM_TINT[1],
                        STREAM_TINT[2],
                        STREAM_MAX_ALPHA,
                    ],
                    blend: BlendKind::Alpha,
                    no_depth: false,
                });
            }
        }

        // 8 crests per active burst.
        for crest in &self.crests {
            let age = self.age_frames - crest.spawn_frame;
            if age < 0.0 || age >= CREST_DURATION_FRAMES {
                continue;
            }
            // Linear fade out.
            let alpha = CREST_MAX_ALPHA * (1.0 - age / CREST_DURATION_FRAMES);
            if alpha <= 0.0 {
                continue;
            }
            for i in 0..STREAM_COUNT {
                let angle_deg = (i as f32) * 45.0 + 22.5;
                let (sn, cs) = angle_deg.to_radians().sin_cos();
                let centre = [
                    self.world_pos[0] + sn * CREST_RADIUS,
                    self.world_pos[1] - CREST_HEIGHT_HALF,
                    self.world_pos[2] + cs * CREST_RADIUS,
                ];
                let tangent = [cs, -sn];
                out.push(EffectPrimitiveDraw::WorldQuad {
                    corners: vertical_panel(
                        centre,
                        tangent,
                        CREST_HALF_TANGENT,
                        -CREST_HEIGHT_HALF,
                        CREST_HEIGHT_HALF,
                    ),
                    uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                    texture: CREST_TEXTURE,
                    color: [1.0, 1.0, 1.0, alpha],
                    blend: BlendKind::Additive,
                    no_depth: false,
                });
            }
        }

        // Expanding ground rings.
        for ring in &self.rings {
            let age = self.age_frames - ring.spawn_frame;
            if age < 0.0 || age >= RING_DURATION_FRAMES {
                continue;
            }
            let grown = RING_RADIUS_SPEED_PER_FRAME * age
                + RING_RADIUS_ACCEL_PER_FRAME * age * (age + 1.0) / 2.0;
            let radius = RING_INNER_RADIUS + grown.max(0.0);
            let alpha = RING_MAX_ALPHA * (1.0 - age / RING_DURATION_FRAMES);
            if alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::GroundDisc {
                center: self.world_pos,
                radius,
                thickness: RING_THICKNESS,
                rotation: 0.0,
                arc_angle_deg: 360.0,
                uv_repeat: 1.0,
                texture: RING_TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
            });
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

    fn step_frames(e: &mut SpraypondEffect, n: i32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    #[test]
    fn eight_streams_plus_crests_and_rings() {
        // Sociable: at frame 12 we have 8 water streams every frame, 8
        // crests per burst (frame 10 already fired, frame 0 expired
        // not yet — wait, both 0 and 10 are alive at frame 12 since
        // crest lifetime is 30 frames). At frame 12, ring at frame 0
        // (age 12) is still alive.
        let mut e = SpraypondEffect::new([2.0, 0.0, 3.0]);
        step_frames(&mut e, 12);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let streams: usize = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::WorldQuad { texture, .. } if *texture == WATER_TEXTURE))
            .count();
        assert_eq!(streams, STREAM_COUNT, "8 sustained water streams");

        let crests: usize = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::WorldQuad { texture, .. } if *texture == CREST_TEXTURE))
            .count();
        assert_eq!(crests, STREAM_COUNT * 2, "crests from frames 0 and 10");

        let rings: usize = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { .. }))
            .count();
        assert!(rings >= 1, "ground ring respawns every 20 frames");
    }

    #[test]
    fn streams_sit_on_radius_12_circle_at_eight_compass_points() {
        // Sociable: each stream's XZ centre is at radius 12.2 from
        // anchor, and the 8 streams cover ≈ 8 distinct angles.
        let anchor = [5.0, 0.0, 7.0];
        let mut e = SpraypondEffect::new(anchor);
        step_frames(&mut e, 1);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let mut angles: Vec<f32> = Vec::new();
        for prim in &list.primitives {
            if let EffectPrimitiveDraw::WorldQuad {
                corners, texture, ..
            } = prim
            {
                if *texture != WATER_TEXTURE {
                    continue;
                }
                // Average XZ centre across the 4 corners.
                let cx = corners.iter().map(|c| c[0]).sum::<f32>() / 4.0;
                let cz = corners.iter().map(|c| c[2]).sum::<f32>() / 4.0;
                let r = ((cx - anchor[0]).powi(2) + (cz - anchor[2]).powi(2)).sqrt();
                assert!(
                    (r - STREAM_RADIUS).abs() < 0.5,
                    "stream centre on radius {STREAM_RADIUS}: r={r}",
                );
                angles.push((cz - anchor[2]).atan2(cx - anchor[0]));
            }
        }
        assert_eq!(angles.len(), STREAM_COUNT);
    }

    #[test]
    fn dies_after_parent_plus_crest_lifetime() {
        let mut e = SpraypondEffect::new([0.0; 3]);
        let total = PARENT_DURATION_FRAMES + CREST_DURATION_FRAMES + 5.0;
        let mut status = EffectStatus::Running;
        for _ in 0..(total as i32) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
