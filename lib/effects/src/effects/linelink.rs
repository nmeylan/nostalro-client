//! `EF_LINELINK` (232) / `EF_LINELINK2` (384) / `EF_LINELINK3` (395).

use std::f32::consts::FRAC_PI_2;

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::spec::EffectAnchor;

pub const TEXTURES: &[&str] = &["alpha_center.tga"];

const FRAMES_PER_SECOND: f32 = 60.0;

const SEGMENTS: usize = 16;
const WORLD_SCALE: f32 = 1.0;

pub const LINELINK_DURATION_MS: u32 = 999990;
pub const LINELINK2_DURATION_MS: u32 = 2000;
pub const LINELINK3_DURATION_MS: u32 = u32::MAX;

#[derive(Clone, Copy, PartialEq)]
enum LinelinkLaw {
    Pulse,
    FadeInOut,
    AsuraBreathe,
}

pub struct LinelinkParams {
    texture: &'static str,
    inner: [u8; 3],
    outer: [u8; 3],
    max_height: f32,
    alpha_init: f32,
    law: LinelinkLaw,
}

pub const LINELINK: LinelinkParams = LinelinkParams {
    texture: "alpha_center.tga",
    inner: [210, 225, 255],
    outer: [30, 80, 255],
    max_height: 0.4,
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
                    no_depth: false,
                });
            }
            prev_now = v_now;
            prev_tgt = v_tgt;
        }
    }
}

fn rgb(c: [u8; 3], alpha: f32) -> [f32; 4] {
    [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
        alpha.min(1.0),
    ]
}

impl Effect for LinelinkEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let target = self.age_frames as u32;
        while self.last_frame < target {
            self.step();
            self.last_frame += 1;
        }
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
        let target = if self.process < 15.0 {
            let s = (self.process * 6.0).to_radians().sin();
            [
                now[0] + (pre[0] - now[0]) * s,
                now[1] + (pre[1] - now[1]) * s,
                now[2] + (pre[2] - now[2]) * s,
            ]
        } else {
            pre
        };
        let angle = (now[2] - pre[2]).atan2(now[0] - pre[0]) + FRAC_PI_2;
        let inner_r = self.max_height * WORLD_SCALE;
        self.collect_cylinder(
            inner_r,
            rgb(self.params.inner, self.alpha_b / 255.0),
            target,
            angle,
            out,
        );
        self.collect_cylinder(
            inner_r * 2.0,
            rgb(self.params.outer, self.alpha_b * 2.0 / 255.0),
            target,
            angle,
            out,
        );
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

    fn step(e: &mut LinelinkEffect, frames: u32) {
        for _ in 0..frames {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
    }

    fn world_quads(e: &LinelinkEffect) -> usize {
        let mut out = EffectDrawList::new();
        e.collect_draws(&mut out, &render_ctx());
        out.primitives
            .iter()
            .filter(|d| matches!(d, EffectPrimitiveDraw::WorldQuad { .. }))
            .count()
    }

    #[test]
    fn linelink_tracks_live_endpoints_and_renders_two_cylinders() {
        let anchor = EffectAnchor::Trail {
            from: [0.0, 0.0, 0.0],
            to: [10.0, 0.0, 0.0],
        };
        let mut e = LinelinkEffect::new(anchor, &LINELINK);

        e.set_link_endpoints([1.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
        step(&mut e, 20);
        assert_eq!(e.caster_pos, [1.0, 0.0, 0.0]);
        assert_eq!(e.target_pos, [2.0, 0.0, 0.0]);

        assert_eq!(world_quads(&e), 2 * SEGMENTS);

        let mut s = LinelinkEffect::new(anchor, &LINELINK);
        step(&mut s, 20);
        assert_eq!(s.caster_pos, [0.0, 0.0, 0.0]);
        assert_eq!(s.target_pos, [10.0, 0.0, 0.0]);
        assert_eq!(world_quads(&s), 2 * SEGMENTS);
    }
}
