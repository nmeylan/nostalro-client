//! `EF_BOTTOM_BASILICA` — Priest's Basilica holy-ground pillar walls.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const TEXTURE: &str = "alpha_down.tga";
const GROW_FRAMES: u32 = 90;

pub const TEXTURES: &[&str] = &[TEXTURE];

#[derive(Clone, Copy, Debug)]
struct InnerCell {
    half_extent: f32,
    max_height: f32,
    alpha_b: f32,
}

const INNER_CELLS: [InnerCell; 4] = [
    // Variant A
    InnerCell {
        half_extent: 12.5,
        max_height: 30.0,
        alpha_b: 65.0 / 255.0,
    },
    InnerCell {
        half_extent: 12.9,
        max_height: 31.0,
        alpha_b: 65.0 / 255.0,
    },
    // Variant B
    InnerCell {
        half_extent: 14.0,
        max_height: 20.0,
        alpha_b: 32.0 / 255.0,
    },
    InnerCell {
        half_extent: 14.4,
        max_height: 21.0,
        alpha_b: 32.0 / 255.0,
    },
];

const BASE_Y_OFFSET: f32 = -1.0;

const UV_QUAD: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

pub struct BasilicaEffect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl BasilicaEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age_frames: 0.0,
        }
    }

    fn wall_height(max_height: f32, age_frames: f32) -> f32 {
        let process = age_frames.max(0.0);
        if process >= GROW_FRAMES as f32 {
            max_height
        } else {
            let angle_rad = process.to_radians();
            max_height * angle_rad.sin()
        }
    }

    fn push_cube_walls(&self, out: &mut EffectDrawList, cell: InnerCell, alpha: f32, wall_h: f32) {
        if wall_h <= 0.0 {
            return;
        }
        let h = cell.half_extent;
        let cx = self.world_pos[0];
        let cy = self.world_pos[1];
        let cz = self.world_pos[2];
        let y_bot = cy + BASE_Y_OFFSET;
        let y_top = y_bot - wall_h;

        let a = [cx + h, y_bot, cz + h];
        let b = [cx + h, y_bot, cz - h];
        let c = [cx - h, y_bot, cz - h];
        let d = [cx - h, y_bot, cz + h];
        let a_t = [a[0], y_top, a[2]];
        let b_t = [b[0], y_top, b[2]];
        let c_t = [c[0], y_top, c[2]];
        let d_t = [d[0], y_top, d[2]];

        let color = [1.0, 1.0, 1.0, alpha];
        for corners in [
            [a, b, b_t, a_t],
            [b, c, c_t, b_t],
            [c, d, d_t, c_t],
            [d, a, a_t, d_t],
        ] {
            out.push(EffectPrimitiveDraw::WorldQuad {
                corners,
                uv: UV_QUAD,
                texture: TEXTURE,
                color,
                blend: BlendKind::Alpha,
                no_depth: false,
            });
        }
    }
}

impl Effect for BasilicaEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        EffectStatus::Running
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for cell in INNER_CELLS.iter() {
            let wall_h = Self::wall_height(cell.max_height, self.age_frames);
            self.push_cube_walls(out, *cell, cell.alpha_b, wall_h);
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

    fn step(e: &mut BasilicaEffect, frames: f32) {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn draws(e: &BasilicaEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn corners_of(prim: &EffectPrimitiveDraw) -> [[f32; 3]; 4] {
        match prim {
            EffectPrimitiveDraw::WorldQuad { corners, .. } => *corners,
            other => panic!("expected WorldQuad, got {other:?}"),
        }
    }

    #[test]
    fn basilica_emits_axis_aligned_cube_walls() {
        let mut e = BasilicaEffect::new([0.0, 0.0, 0.0]);
        step(&mut e, GROW_FRAMES as f32);
        let prims = draws(&e);
        assert_eq!(prims.len(), 16);

        for prim in &prims {
            let c = corners_of(prim);
            let dx = (c[0][0] - c[1][0]).abs();
            let dz = (c[0][2] - c[1][2]).abs();
            let axis_aligned = dx < 1e-4 || dz < 1e-4;
            assert!(
                axis_aligned,
                "wall edge {:?} -> {:?} not axis-aligned (dx={dx}, dz={dz})",
                c[0], c[1],
            );
        }
    }

    #[test]
    fn cube_walls_grow_up_over_first_90_frames() {
        let mut e = BasilicaEffect::new([0.0, 0.0, 0.0]);

        step(&mut e, 0.0);
        let prims0 = draws(&e);
        assert_eq!(prims0.len(), 0, "flat walls at frame 0 are skipped");

        step(&mut e, 45.0);
        let prims_mid = draws(&e);
        assert_eq!(prims_mid.len(), 16);
        for prim in &prims_mid {
            let c = corners_of(prim);
            let y_bot = c[0][1].max(c[1][1]);
            let y_top = c[2][1].min(c[3][1]);
            assert!(y_top < y_bot, "y_top={y_top}, y_bot={y_bot}");
        }

        step(&mut e, 100.0);
        let prims_full = draws(&e);
        let c = corners_of(&prims_full[0]);
        let y_top = c[2][1].min(c[3][1]);
        assert!((y_top - (-31.0)).abs() < 1e-3, "y_top={y_top}");
    }

    #[test]
    fn outer_walls_extend_outwards_to_half_extent_in_x_and_z() {
        let mut e = BasilicaEffect::new([100.0, 0.0, -50.0]);
        step(&mut e, GROW_FRAMES as f32);
        let prims = draws(&e);

        // First cell has h=12.5; the +X wall is the first of its 4
        // walls in our emission order.
        let c = corners_of(&prims[0]);
        let cx = 100.0;
        let cz = -50.0;
        assert!((c[0][0] - (cx + 12.5)).abs() < 1e-4);
        assert!((c[0][2] - (cz + 12.5)).abs() < 1e-4);
        assert!((c[1][0] - (cx + 12.5)).abs() < 1e-4);
        assert!((c[1][2] - (cz - 12.5)).abs() < 1e-4);
    }
}
