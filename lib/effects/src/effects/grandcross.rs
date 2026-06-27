use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const TOTAL_FRAMES: f32 = 110.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const WORLD_SCALE: f32 = 1.0;
const WALL_DISTANCE: f32 = 19.9 * WORLD_SCALE;
const WALL_MAX_HEIGHT: f32 = 120.0 * WORLD_SCALE;
const CORNER: f32 = 23.0 * WORLD_SCALE;
const WALL_SIDES: u32 = 9;

const BEAM_HALF_X: f32 = 24.0 * WORLD_SCALE;
const BEAM_HALF_Z: f32 = 3.2 * WORLD_SCALE;
const BEAM_MAX_HEIGHT: f32 = 60.0 * WORLD_SCALE;
const BEAM_BASE_Y: f32 = 1.0 * WORLD_SCALE;

const RAMP_FRAMES: f32 = 10.0;
const WALL_RAMP_PER_FRAME: f32 = 10.0;
const BEAM_RAMP_PER_FRAME: f32 = 9.0;
const DRAIN_PER_FRAME: f32 = 1.0;

/// The four corner walls: `(rotation degrees, corner-x sign, corner-z sign)`.
const WALLS: [(f32, f32, f32); 4] = [
    (180.0, 1.0, 1.0),
    (90.0, 1.0, -1.0),
    (0.0, -1.0, -1.0),
    (270.0, -1.0, 1.0),
];

#[derive(Clone, Copy)]
pub struct GrandcrossParams {
    pub wall_texture: &'static str,
    pub beam_texture: &'static str,
    pub wall_blend: BlendKind,
    pub wall_tint: [f32; 3],
}

pub const GRANDCROSS: GrandcrossParams = GrandcrossParams {
    wall_texture: "ring_white.tga",
    beam_texture: "ring_yellow.tga",
    wall_blend: BlendKind::Additive,
    wall_tint: [1.0, 175.0 / 255.0, 175.0 / 255.0],
};

pub const GRANDCROSS2: GrandcrossParams = GrandcrossParams {
    wall_texture: "ring_black.tga",
    beam_texture: "ring_black.tga",
    wall_blend: BlendKind::Alpha,
    wall_tint: [50.0 / 255.0, 50.0 / 255.0, 50.0 / 255.0],
};

pub const TEXTURES: &[&str] = &["ring_white.tga", "ring_yellow.tga", "ring_black.tga"];

pub struct GrandcrossEffect {
    params: GrandcrossParams,
    world_pos: [f32; 3],
    process: f32,
}

impl GrandcrossEffect {
    pub fn new(world_pos: [f32; 3], params: GrandcrossParams) -> Self {
        Self {
            params,
            world_pos,
            process: 0.0,
        }
    }

    fn alpha(&self, ramp_per_frame: f32) -> f32 {
        let p = self.process;
        let raw = if p < RAMP_FRAMES {
            ramp_per_frame * p
        } else {
            (ramp_per_frame * RAMP_FRAMES - DRAIN_PER_FRAME * (p - RAMP_FRAMES)).max(0.0)
        };
        raw / 255.0
    }

    fn swell(&self) -> f32 {
        self.process.to_radians().sin().max(0.0)
    }
}

fn rot(x: f32, z: f32, c: f32, s: f32) -> (f32, f32) {
    (c * x - s * z, s * x + c * z)
}

impl Effect for GrandcrossEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.process += ctx.delta * FRAMES_PER_SECOND;
        if self.process >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [cx, cy, cz] = self.world_pos;
        let swell = self.swell();

        let wall_alpha = self.alpha(WALL_RAMP_PER_FRAME);
        let [wt_r, wt_g, wt_b] = self.params.wall_tint;
        let wall_color = [wt_r, wt_g, wt_b, wall_alpha];
        if wall_alpha > 0.0 {
            let height = WALL_MAX_HEIGHT * swell;
            for (rot_start, sx, sz) in WALLS {
                out.push(EffectPrimitiveDraw::Frustum {
                    base_alpha: 1.0,
                    base: [cx + sx * CORNER, cy, cz + sz * CORNER],
                    bottom_size: WALL_DISTANCE,
                    top_size: WALL_DISTANCE,
                    height,
                    sides: WALL_SIDES,
                    arc_angle_deg: 90.0,
                    rotation: rot_start.to_radians(),
                    uv_repeat: 1.0,
                    uv_scroll: [0.0, 0.0],
                    wave_amplitude: 0.0,
                    wave_frequency: 1.0,
                    wave_phase: 0.0,
                    wave_mode: FrustumWaveMode::Sine,
                    tilt_x_rad: 0.0,
                    rotation_y_rad: 0.0,
                    cull_back: false,
                    texture: self.params.wall_texture,
                    color: wall_color,
                    blend: self.params.wall_blend,
                });
            }
        }

        let beam_alpha = self.alpha(BEAM_RAMP_PER_FRAME);
        if beam_alpha > 0.0 {
            let beam_color = [1.0, 1.0, 1.0, beam_alpha];
            let beam_color_top = [1.0, 1.0, 1.0, beam_alpha * 0.5];
            let h = BEAM_MAX_HEIGHT * swell;
            for k in 0..2 {
                let (c, s) = {
                    let a = (k as f32 * 90.0).to_radians();
                    (a.cos(), a.sin())
                };
                let base_corners = [
                    rot(BEAM_HALF_X, BEAM_HALF_Z, c, s),
                    rot(BEAM_HALF_X, -BEAM_HALF_Z, c, s),
                    rot(-BEAM_HALF_X, -BEAM_HALF_Z, c, s),
                    rot(-BEAM_HALF_X, BEAM_HALF_Z, c, s),
                ];
                let base_y = cy - BEAM_BASE_Y;
                let top_y = base_y - h;
                let base: Vec<[f32; 3]> = base_corners
                    .iter()
                    .map(|(x, z)| [cx + x, base_y, cz + z])
                    .collect();
                let top: Vec<[f32; 3]> = base_corners
                    .iter()
                    .map(|(x, z)| [cx + x, top_y, cz + z])
                    .collect();
                let uv = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
                for i in 0..4 {
                    let j = (i + 1) % 4;
                    out.push(EffectPrimitiveDraw::WorldQuad {
                        corners: [base[i], base[j], top[j], top[i]],
                        uv,
                        texture: self.params.beam_texture,
                        color: beam_color,
                        blend: BlendKind::Alpha,
                        no_depth: false,
                    });
                }
                out.push(EffectPrimitiveDraw::WorldQuad {
                    corners: [top[0], top[1], top[2], top[3]],
                    uv,
                    texture: self.params.beam_texture,
                    color: beam_color_top,
                    blend: BlendKind::Additive,
                    no_depth: false,
                });
            }
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

    fn run_to(e: &mut GrandcrossEffect, frame: f32) {
        let d = (frame - e.process) / FRAMES_PER_SECOND;
        if d > 0.0 {
            e.update(&EffectUpdateCtx {
                delta: d,
                camera_target: None,
                caster_yaw: None,
            });
        }
    }

    fn draws(e: &GrandcrossEffect) -> Vec<EffectPrimitiveDraw> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        l.primitives
    }

    #[test]
    fn emits_four_arc_walls_and_two_beam_boxes() {
        let mut e = GrandcrossEffect::new([0.0; 3], GRANDCROSS);
        run_to(&mut e, RAMP_FRAMES);
        let d = draws(&e);
        let walls = d
            .iter()
            .filter(|p| {
                matches!(p,
            EffectPrimitiveDraw::Frustum { arc_angle_deg, .. } if *arc_angle_deg == 90.0)
            })
            .count();
        let quads = d
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::WorldQuad { .. }))
            .count();
        assert_eq!(walls, 4, "four corner arc-walls");
        assert_eq!(quads, 10, "two beam boxes");
    }

    #[test]
    fn walls_swell_then_alpha_drains() {
        let mut e = GrandcrossEffect::new([0.0; 3], GRANDCROSS);
        run_to(&mut e, RAMP_FRAMES);
        let peak_h = wall_height(&e);
        let peak_a = wall_alpha(&e);
        run_to(&mut e, RAMP_FRAMES + 40.0);
        let late_h = wall_height(&e);
        let late_a = wall_alpha(&e);
        assert!(late_h > peak_h, "wall keeps swelling ({peak_h} → {late_h})");
        assert!(
            late_a < peak_a,
            "alpha drains after the ramp ({peak_a} → {late_a})"
        );
    }

    #[test]
    fn black_variant_uses_black_textures() {
        let mut e = GrandcrossEffect::new([0.0; 3], GRANDCROSS2);
        run_to(&mut e, RAMP_FRAMES);
        for p in draws(&e) {
            match p {
                EffectPrimitiveDraw::Frustum { texture, .. }
                | EffectPrimitiveDraw::WorldQuad { texture, .. } => {
                    assert_eq!(texture, "ring_black.tga");
                }
                _ => {}
            }
        }
    }

    #[test]
    fn self_terminates() {
        let mut e = GrandcrossEffect::new([0.0; 3], GRANDCROSS);
        run_to(&mut e, TOTAL_FRAMES - 1.0);
        assert_eq!(
            e.update(&EffectUpdateCtx {
                delta: 0.1,
                camera_target: None,
                caster_yaw: None
            }),
            EffectStatus::Dead
        );
    }

    fn wall_height(e: &GrandcrossEffect) -> f32 {
        draws(e)
            .into_iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::Frustum { height, .. } => Some(height),
                _ => None,
            })
            .unwrap()
    }

    fn wall_alpha(e: &GrandcrossEffect) -> f32 {
        draws(e)
            .into_iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::Frustum { color, .. } => Some(color[3]),
                _ => None,
            })
            .unwrap()
    }
}
