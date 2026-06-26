//! `EF_BOTTOM_BASILICA` — Priest's Basilica holy ground.
//!
//! The effect launches twice — once per size table. Each
//! launch raises an upward-growing rectangular shell built from four
//! cells:
//!
//! * **Inner cells (0, 1)** render a full axis-aligned 4-wall
//!   pillar centred on the master. Corners sit at `(±half_extent_x, -1,
//!   ±half_extent_z)`; top edge sits at `y = -1 - wall_height` (native RO
//!   `-Y = up`).
//! * **Outer cells (2, 3)** are a **single sliding rectangle**
//!   that travels around the same square's perimeter (distance grows
//!   0.5 per frame, wrapping at `half_extent * 8`). Not yet
//!   implemented — deferred to a follow-up; the dominant Basilica
//!   silhouette is the two inner pillars.
//!
//! Per-frame integrator for the wall height (frame counter `f`):
//!   f += 1
//!   if f < 90:
//!       wall_height = max_height * sin(f°)         // grow up
//!   elif f < hold_until:
//!       wall_height = max_height                   // hold
//!   else:                                          // fade out
//!       angle = (variant ? 2475 : 2505) - f
//!       angle clamped to [0, 90]
//!       wall_height = max_height * sin(angle°)
//!
//! Inner cell wall geometry:
//!   bottom face corners (y = -1):
//!     A = (+h, -1, +h)
//!     B = (+h, -1, -h)
//!     C = (-h, -1, -h)
//!     D = (-h, -1, +h)
//!   top corners (y = -1 - wall_height): A', B', C', D'
//!   walls drawn (CCW from outside):
//!     +X face : A, B, B', A'
//!     -Z face : B, C, C', B'
//!     -X face : C, D, D', C'
//!     +Z face : D, A, A', D'
//!
//! Per-cell seed values:
//!
//! ```text
//! Variant A:
//!   cell 0  h=12.5  max_height=30  alpha=65
//!   cell 1  h=12.9  max_height=31  alpha=65
//!   cell 2  h=13.3  max_height=32  alpha=15  (sliding, deferred)
//!   cell 3  h=13.3  max_height=33  alpha=15  (sliding, deferred)
//!
//! Variant B:
//!   cell 0  h=14.0  max_height=20  alpha=32
//!   cell 1  h=14.4  max_height=21  alpha=32
//!   cell 2  h=14.8  max_height=22  alpha=15  (sliding, deferred)
//!   cell 3  h=14.8  max_height=23  alpha=15  (sliding, deferred)
//! ```

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const TEXTURE: &str = "alpha_down.tga";
const GROW_FRAMES: u32 = 90;

pub const TEXTURES: &[&str] = &[TEXTURE];

#[derive(Clone, Copy, Debug)]
struct InnerCell {
    /// Half-extent of the cube footprint in X. The Z half-extent is set
    /// equal so the footprint is square.
    half_extent: f32,
    /// Final wall height after the 90-frame grow-up.
    max_height: f32,
    /// Steady-state alpha (raw value / 255). The hold window is set far past
    /// the holder duration so the slow fade-out branch never triggers
    /// in-window.
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

/// Master Y-offset for the wall base. Native RO `-Y = up`,
/// so this is 1 unit ABOVE the caster's feet.
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

    /// Wall height for the grow phase: `max_height *
    /// sin(frame°)` for the first 90 frames, then held at `max_height`.
    /// Returns the current wall height in world units.
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

        // Bottom corners (y = -1 relative to master).
        let a = [cx + h, y_bot, cz + h];
        let b = [cx + h, y_bot, cz - h];
        let c = [cx - h, y_bot, cz - h];
        let d = [cx - h, y_bot, cz + h];
        // Top corners (y = -1 - wall_height).
        let a_t = [a[0], y_top, a[2]];
        let b_t = [b[0], y_top, b[2]];
        let c_t = [c[0], y_top, c[2]];
        let d_t = [d[0], y_top, d[2]];

        let color = [1.0, 1.0, 1.0, alpha];
        // CCW corner order viewed from outside the cube — wall vertex
        // order is `(top0, top1, bottom1, bottom0)`.
        for corners in [
            [a, b, b_t, a_t], // +X face
            [b, c, c_t, b_t], // -Z face
            [c, d, d_t, c_t], // -X face
            [d, a, a_t, d_t], // +Z face
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
        // Holder-enforced duration (table.rs = 299990 ms). Never self-die.
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
        // Sociable: 4 inner cells × 4 walls each = 16 axis-aligned wall
        // quads. Each wall's bottom edge must lie along a world-axis-
        // parallel line (X or Z constant), not on a diagonal — the
        // earlier `Cylinder { sides=4, rotation=0 }` implementation
        // placed corners on the diagonals and looked rotated 45°.
        let mut e = BasilicaEffect::new([0.0, 0.0, 0.0]);
        step(&mut e, GROW_FRAMES as f32);
        let prims = draws(&e);
        assert_eq!(prims.len(), 16);

        for prim in &prims {
            let c = corners_of(prim);
            // Bottom edge: c[0] -> c[1]. Wall is axis-aligned iff that
            // edge runs along constant X (Z-aligned) or constant Z
            // (X-aligned). Diagonal walls would have both X and Z
            // changing.
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
        // Sociable: the `wall_height = max_height * sin(frame°)`
        // grow animation must be live. At spawn (frame=0), all walls
        // are degenerate (top y == bottom y). After ~45 frames the
        // walls are mid-grow (top y is strictly above the bottom). At
        // 90+ frames, top y reaches the seeded max_height.
        let mut e = BasilicaEffect::new([0.0, 0.0, 0.0]);

        // Just before any tick — all walls flat (or empty).
        step(&mut e, 0.0);
        let prims0 = draws(&e);
        // Renderer code drops walls with wall_h <= 0; at frame 0 sin(0°)=0
        // so the helper returns 0 and we push nothing.
        assert_eq!(prims0.len(), 0, "flat walls at frame 0 are skipped");

        // Mid-grow.
        step(&mut e, 45.0);
        let prims_mid = draws(&e);
        assert_eq!(prims_mid.len(), 16);
        // y_bot = -1, y_top = -1 - wall_h. Top is strictly more negative.
        for prim in &prims_mid {
            let c = corners_of(prim);
            let y_bot = c[0][1].max(c[1][1]);
            let y_top = c[2][1].min(c[3][1]);
            assert!(
                y_top < y_bot,
                "mid-grow wall must have top above bottom (native -Y up): y_top={y_top}, y_bot={y_bot}",
            );
        }

        // Past grow window.
        step(&mut e, 100.0);
        let prims_full = draws(&e);
        // Walls now at their seeded max_height. Use the first inner cell
        // (max_height = 30, half_extent = 12.5) as the witness.
        let c = corners_of(&prims_full[0]);
        let y_top = c[2][1].min(c[3][1]);
        // y_bot = -1 → y_top = -1 - 30 = -31. Approximately.
        assert!(
            (y_top - (-31.0)).abs() < 1e-3,
            "first inner cell's wall top must reach max_height=30 above ground (y_top={y_top})",
        );
    }

    #[test]
    fn outer_walls_extend_outwards_to_half_extent_in_x_and_z() {
        // Sociable: a wall on the +X face has its corners' X equal to
        // +half_extent (12.5 for the smallest inner cell), and spans
        // [-half_extent, +half_extent] in Z. Confirms we're placing
        // the corners at axis-aligned positions and not at e.g. diagonals
        // of a 45°-rotated square.
        let mut e = BasilicaEffect::new([100.0, 0.0, -50.0]);
        step(&mut e, GROW_FRAMES as f32);
        let prims = draws(&e);

        // First cell has h=12.5; the +X wall is the first of its 4
        // walls in our emission order.
        let c = corners_of(&prims[0]);
        let cx = 100.0;
        let cz = -50.0;
        // Bottom-right corner A = (cx + 12.5, _, cz + 12.5).
        assert!((c[0][0] - (cx + 12.5)).abs() < 1e-4);
        assert!((c[0][2] - (cz + 12.5)).abs() < 1e-4);
        // Bottom-left corner B = (cx + 12.5, _, cz - 12.5).
        assert!((c[1][0] - (cx + 12.5)).abs() < 1e-4);
        assert!((c[1][2] - (cz - 12.5)).abs() < 1e-4);
    }
}
