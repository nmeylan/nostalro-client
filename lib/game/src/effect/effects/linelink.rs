//! `EF_LINELINK` (232) / `EF_LINELINK2` (384) / `EF_LINELINK3` (395).
//!
//! The Soul Linker tether: a two-cylinder ribbon between the caster
//! and a second, independently-moving linked actor. Both endpoints are re-read
//! **every frame** — the renderer holder resolves an `Attach::Link` and
//! feeds the live positions through [`Effect::set_link_endpoints`]. With no live
//! feed (the effect viewer's static fake-entity path) the spawn-time anchor
//! endpoints are used unchanged.
//!
//!
//!
//! Geometry: a ring (`i·36°`) in the plane spanned
//! by the world-up axis and the horizontal perpendicular to the caster→target
//! direction, swept from the caster to the target as a prism. Two concentric
//! cylinders: inner radius `max_height`, outer `×2` (outer alpha doubled). The
//! target endpoint eases out from the caster over a 15-frame sinusoid fade-in.
//!
//! All three use `alpha_center.tga` (a uniform beam gradient) so the tube reads
//! as one solid blade; the inner cylinder is the bright core, the outer the glow.
//! Per id (`max_height`, colours, alpha law):
//!   * 232 0.4, near-white core → blue glow, pulses ±2 / 5 frames (a blue laser;
//!     a yellow core + tiled puff texture didn't read as a beam).
//!   * 384 0.2, teal core → red glow, ramps in (+6/f <20) then fades out (−3/f >40).
//!   * 395 0.7, magenta, ramps in (+1/f ≤80) and breathes
//!     `max_height += sin(process°)·0.005`.

use std::f32::consts::FRAC_PI_2;

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::EffectAnchor;

pub const TEXTURES: &[&str] = &["alpha_center.tga"];

const FRAMES_PER_SECOND: f32 = 60.0;

/// Circumferential staves of the tube. The original uses 10; we round the
/// silhouette up a little so a thin beam doesn't read as a decagonal prism when
/// the camera is close.
const SEGMENTS: usize = 16;

/// Effect units ≈ our world units (≈1:1, the same mapping
/// `attack_energy`'s dome uses), so `max_height` is the tube radius directly.
/// The link must read as a *thin* beam — a lightsaber blade, not a pillar:
/// the inner (`max_height`) and outer (`×2`) additive cylinders overlap along
/// the sightline so the core glows near-white inside the coloured rim.
const WORLD_SCALE: f32 = 1.0;

pub const LINELINK_DURATION_MS: u32 = 999990;
pub const LINELINK2_DURATION_MS: u32 = 2000;
pub const LINELINK3_DURATION_MS: u32 = u32::MAX;

/// Per-frame alpha / `max_height` behaviour, one law per id.
#[derive(Clone, Copy, PartialEq)]
enum LinelinkLaw {
    /// 232: alpha pulses ±2 every 5 frames around its start value.
    Pulse,
    /// 384: alpha ramps in (+6/f while `process < 20`), then fades out
    /// (−3/f while `process > 40`).
    FadeInOut,
    /// 395: alpha ramps in (+1/f while `process <= 80`); `max_height` breathes
    /// by `sin(process°)·0.005` each frame.
    AsuraBreathe,
}

pub struct LinelinkParams {
    texture: &'static str,
    /// Inner cylinder RGB (0..255).
    inner: [u8; 3],
    /// Outer cylinder RGB (0..255).
    outer: [u8; 3],
    max_height: f32,
    alpha_init: f32,
    law: LinelinkLaw,
}

// 232 reads as a blue lightsaber: a near-white core (inner cylinder) inside a
// saturated-blue glow (outer cylinder). A yellow inner + tiled puff texture
// reads as a string of beads, not a solid beam (the 2D puff tiles into separate
// blobs around the tube) and the yellow core looks wrong;
// `alpha_center.tga` is the uniform beam-gradient the other two already use.
pub const LINELINK: LinelinkParams = LinelinkParams {
    texture: "alpha_center.tga",
    inner: [210, 225, 255],
    outer: [30, 80, 255],
    max_height: 0.4,
    // The original's base alpha here is 45, but that reads as a faint thread
    // with the beam texture; 120 makes a solid laser whose additive core saturates to
    // white while the outer (×2 = 240/255) still pulses without clamping.
    alpha_init: 120.0,
    law: LinelinkLaw::Pulse,
};

pub const LINELINK2: LinelinkParams = LinelinkParams {
    texture: "alpha_center.tga",
    inner: [0, 100, 150],
    outer: [255, 50, 0],
    max_height: 0.2,
    alpha_init: 0.0,
    law: LinelinkLaw::FadeInOut,
};

pub const LINELINK3: LinelinkParams = LinelinkParams {
    texture: "alpha_center.tga",
    inner: [255, 89, 182],
    outer: [255, 89, 182],
    max_height: 0.7,
    alpha_init: 0.0,
    law: LinelinkLaw::AsuraBreathe,
};

pub struct LinelinkEffect {
    caster_pos: [f32; 3],
    target_pos: [f32; 3],
    process: f32,
    alpha_b: f32,
    max_height: f32,
    params: &'static LinelinkParams,
    age_frames: f32,
    last_frame: u32,
}

impl LinelinkEffect {
    pub fn new(anchor: EffectAnchor, params: &'static LinelinkParams) -> Self {
        let (from, to) = match anchor {
            EffectAnchor::Trail { from, to } => (from, to),
            EffectAnchor::Point(p) => (p, p),
        };
        Self {
            caster_pos: from,
            target_pos: to,
            process: 0.0,
            alpha_b: params.alpha_init,
            max_height: params.max_height,
            params,
            age_frames: 0.0,
            last_frame: 0,
        }
    }

    fn step(&mut self) {
        self.process += 1.0;
        match self.params.law {
            LinelinkLaw::Pulse => {
                if (self.process as i32) % 10 < 5 {
                    self.alpha_b += 2.0;
                } else {
                    self.alpha_b -= 2.0;
                }
                self.alpha_b = self.alpha_b.clamp(0.0, 255.0);
            }
            LinelinkLaw::FadeInOut => {
                if self.process > 40.0 {
                    self.alpha_b = (self.alpha_b - 3.0).max(0.0);
                } else if self.process < 20.0 {
                    self.alpha_b += 6.0;
                }
            }
            LinelinkLaw::AsuraBreathe => {
                if self.process <= 80.0 {
                    self.alpha_b += 1.0;
                }
                let deg = (self.process as i32 % 360) as f32;
                self.max_height += deg.to_radians().sin() * 0.005;
            }
        }
    }

    /// Emit the two-cylinder ribbon for one radius layer.
    fn collect_cylinder(
        &self,
        radius: f32,
        color: [f32; 4],
        target: [f32; 3],
        angle: f32,
        out: &mut EffectDrawList,
    ) {
        let (ca, sa) = (angle.cos(), angle.sin());
        let now = self.caster_pos;
        let mut prev_now = [0.0; 3];
        let mut prev_tgt = [0.0; 3];
        for i in 0..=SEGMENTS {
            let ring = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
            let horiz = radius * ring.cos();
            let vert = radius * ring.sin();
            let (ox, oz) = (horiz * ca, horiz * sa);
            let v_now = [now[0] + ox, now[1] + vert, now[2] + oz];
            let v_tgt = [target[0] + ox, target[1] + vert, target[2] + oz];
            if i > 0 {
                out.push(EffectPrimitiveDraw::WorldQuad {
                    corners: [prev_now, v_now, v_tgt, prev_tgt],
                    uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    texture: self.params.texture,
                    color,
                    blend: BlendKind::Additive,
                });
            }
            prev_now = v_now;
            prev_tgt = v_tgt;
        }
    }
}

fn rgb(c: [u8; 3], alpha: f32) -> [f32; 4] {
    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, alpha.min(1.0)]
}

impl Effect for LinelinkEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let target = self.age_frames as u32;
        while self.last_frame < target {
            self.step();
            self.last_frame += 1;
        }
        // The holder caps lifetime from the spec duration and drops the effect
        // when the linked actor disappears, so the tether just keeps running.
        EffectStatus::Running
    }

    fn set_link_endpoints(&mut self, caster: [f32; 3], target: [f32; 3]) {
        self.caster_pos = caster;
        self.target_pos = target;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.alpha_b <= 0.0 {
            return;
        }
        let now = self.caster_pos;
        let pre = self.target_pos;
        // Target endpoint eases out from the caster over the first 15 frames.
        let target = if self.process < 15.0 {
            let s = (self.process * 6.0).to_radians().sin();
            [now[0] + (pre[0] - now[0]) * s, now[1] + (pre[1] - now[1]) * s, now[2] + (pre[2] - now[2]) * s]
        } else {
            pre
        };
        // Horizontal axis perpendicular to the caster→target direction
        // (heading + 90°).
        let angle = (now[2] - pre[2]).atan2(now[0] - pre[0]) + FRAC_PI_2;
        let inner_r = self.max_height * WORLD_SCALE;
        self.collect_cylinder(inner_r, rgb(self.params.inner, self.alpha_b / 255.0), target, angle, out);
        self.collect_cylinder(inner_r * 2.0, rgb(self.params.outer, self.alpha_b * 2.0 / 255.0), target, angle, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn step(e: &mut LinelinkEffect, frames: u32) {
        for _ in 0..frames {
            e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None });
        }
    }

    fn world_quads(e: &LinelinkEffect) -> usize {
        let mut out = EffectDrawList::new();
        e.collect_draws(&mut out, &render_ctx());
        out.primitives.iter().filter(|d| matches!(d, EffectPrimitiveDraw::WorldQuad { .. })).count()
    }

    #[test]
    fn linelink_tracks_live_endpoints_and_renders_two_cylinders() {
        let anchor = EffectAnchor::Trail { from: [0.0, 0.0, 0.0], to: [10.0, 0.0, 0.0] };
        let mut e = LinelinkEffect::new(anchor, &LINELINK);

        // Live feed (in-game path) overrides the spawn endpoints each frame.
        e.set_link_endpoints([1.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
        step(&mut e, 20);
        assert_eq!(e.caster_pos, [1.0, 0.0, 0.0]);
        assert_eq!(e.target_pos, [2.0, 0.0, 0.0]);

        // Two concentric cylinders of `SEGMENTS` staves each.
        assert_eq!(world_quads(&e), 2 * SEGMENTS);

        // No live feed (viewer static path): endpoints keep the spawn anchor.
        let mut s = LinelinkEffect::new(anchor, &LINELINK);
        step(&mut s, 20);
        assert_eq!(s.caster_pos, [0.0, 0.0, 0.0]);
        assert_eq!(s.target_pos, [10.0, 0.0, 0.0]);
        assert_eq!(world_quads(&s), 2 * SEGMENTS);
    }
}
