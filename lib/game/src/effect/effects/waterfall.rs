//! `WaterFall(tName, F1, F2, F3)` family — water-curtain sheet +
//! rising-mist particles (ids 349–356).
//!
//! The original game draws a flat, translucent vertical water curtain as **four
//! stacked emitters**, each a five-quad strip rising from the caster's feet to
//! ~200 units above. The strips scroll downward via a treadmill: the whole
//! pattern slides down by up to one 40-unit cell (`scroll`) while a three-frame
//! texture index (`waterfall11/12/13` or, for the T2 set, `waterfall31/32/33`)
//! cycles in lockstep, so the flow is seamless. The bottom strip retracts and
//! the top strip grows in at the cell boundary. The sheet
//! is alpha-blended white at `80/255` (the additive-blue cousin is
//! BlueFall, not built here).
//!
//! The four emitters differ only in width (`36+ec`, or `18+ec` for the "small"
//! variants), scroll speed (`80 − 13·ec` frames per cell) and a small per-strip
//! Z depth offset (`ec − 1`) that fakes volume. `F1` swaps each vertex's X and Z
//! to face the sheet 90° around; `F3` selects the brighter T2 texture set.
//!
//! Alongside the sheet, a mist field spawns faint additive
//! greenish puffs (`freeze_a_small.bmp`) that rise slowly from the base,
//! wander horizontally, fade in near the feet and out as they climb, then loop.
//! The original game launches one 16-particle batch per call (40× for the big
//! variants, 20× for the small ones); we keep that exact count. Particle spawns
//! use a per-index hash rather than an RNG so the field is reproducible.
//!
//! These are persistent map decorations (`table.rs` pins the duration sentinel);
//! `update` never reports `Dead`.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// Emitter count and strips-per-emitter for the sheet.
const EMITTERS: usize = 4;
const STRIPS: usize = 5;
/// Cell height up the sheet (strip `i` sits at `i*40`).
const CELL: f32 = 40.0;
/// Sheet alpha out of 255.
const SHEET_ALPHA: f32 = 80.0 / 255.0;
/// Mist particles per spawn call (4 emitters × 4 drift points).
const PARTICLES_PER_CALL: u32 = 16;
/// Mist rise per frame (`0.05`, native −Y up so y decreases).
const MIST_RISE: f32 = 0.05;
/// Mist additive tint `(70, 120, 100)`, the observed waterfall mist colour.
const MIST_RGB: [f32; 3] = [70.0 / 255.0, 120.0 / 255.0, 100.0 / 255.0];

const T1: [&str; 3] = ["waterfall11.tga", "waterfall12.tga", "waterfall13.tga"];
const T2: [&str; 3] = ["waterfall31.tga", "waterfall32.tga", "waterfall33.tga"];

pub const TEXTURES: &[&str] = &[
    "waterfall11.tga",
    "waterfall12.tga",
    "waterfall13.tga",
    "waterfall31.tga",
    "waterfall32.tga",
    "waterfall33.tga",
    "freeze_a_small.bmp",
];

#[derive(Clone, Copy, Debug)]
pub struct WaterfallParams {
    /// `F1` — swap each vertex's X and Z so the sheet faces 90° around.
    pub rotate90: bool,
    /// `F2` — narrower sheet (width `18+ec` vs `36+ec`).
    pub small: bool,
    /// `F3` — the three cycling sheet textures (T1 normal, T2 brighter).
    pub textures: [&'static str; 3],
    /// Number of mist spawn calls (40 big / 20 small; 0 = no mist).
    pub mist_calls: u32,
    /// Sheet RGBA. White at `80/255` for WaterFall; additive blue at `120/255`
    /// for BlueFall.
    pub sheet_color: [f32; 4],
    /// Sheet blend — `Alpha` (WaterFall) or `Additive` (BlueFall).
    pub sheet_blend: BlendKind,
    /// Reverse the treadmill scroll and cycle the texture index
    /// backwards (BlueFall).
    pub reverse: bool,
    /// Frames-per-cell at emitter 0 and its per-emitter decrement (`speed =
    /// speed_base − speed_step·ec`). `(80, 13)` normal, `(30, 6)` fast.
    pub speed_base: i32,
    pub speed_step: i32,
}

/// `EF_WATERFALL` — base sheet (F1=0, F2=0, F3=0) + 40 mist calls.
pub const WATERFALL: WaterfallParams = WaterfallParams {
    rotate90: false,
    small: false,
    textures: T1,
    mist_calls: 40,
    sheet_color: [1.0, 1.0, 1.0, SHEET_ALPHA],
    sheet_blend: BlendKind::Alpha,
    reverse: false,
    speed_base: 80,
    speed_step: 13,
};
/// `EF_WATERFALL_90` — base sheet rotated 90° (F1=1).
pub const WATERFALL_90: WaterfallParams = WaterfallParams { rotate90: true, ..WATERFALL };
/// `EF_WATERFALL_SMALL` — narrow sheet (F2=1) + 20 mist calls.
pub const WATERFALL_SMALL: WaterfallParams =
    WaterfallParams { small: true, mist_calls: 20, ..WATERFALL };
/// `EF_WATERFALL_SMALL_90` — narrow sheet rotated 90° (F1=1, F2=1).
pub const WATERFALL_SMALL_90: WaterfallParams =
    WaterfallParams { rotate90: true, ..WATERFALL_SMALL };
/// `EF_WATERFALL_T2` — brighter texture set (F3=1).
pub const WATERFALL_T2: WaterfallParams = WaterfallParams { textures: T2, ..WATERFALL };
/// `EF_WATERFALL_T2_90` — brighter set rotated 90° (F1=1, F3=1).
pub const WATERFALL_T2_90: WaterfallParams = WaterfallParams { rotate90: true, ..WATERFALL_T2 };
/// `EF_WATERFALL_SMALL_T2` — narrow brighter sheet (F2=1, F3=1).
pub const WATERFALL_SMALL_T2: WaterfallParams =
    WaterfallParams { textures: T2, ..WATERFALL_SMALL };
/// `EF_WATERFALL_SMALL_T2_90` — narrow brighter sheet rotated 90° (F1=1, F2=1, F3=1).
pub const WATERFALL_SMALL_T2_90: WaterfallParams =
    WaterfallParams { rotate90: true, ..WATERFALL_SMALL_T2 };

/// `EF_BLUEFALL` — the WaterFall sheet tinted additive blue
/// (alpha `120/255`) with reversed scroll and no mist.
pub const BLUEFALL: WaterfallParams = WaterfallParams {
    rotate90: false,
    small: false,
    textures: T1,
    mist_calls: 0,
    sheet_color: [55.0 / 255.0, 55.0 / 255.0, 1.0, 120.0 / 255.0],
    sheet_blend: BlendKind::Additive,
    reverse: true,
    speed_base: 80,
    speed_step: 13,
};
/// `EF_BLUEFALL_90` — blue sheet rotated 90° (F1=1).
pub const BLUEFALL_90: WaterfallParams = WaterfallParams { rotate90: true, ..BLUEFALL };
/// `EF_FASTBLUEFALL` — blue sheet at `speed = 30 − 6·ec` (faster scroll).
pub const FASTBLUEFALL: WaterfallParams =
    WaterfallParams { speed_base: 30, speed_step: 6, ..BLUEFALL };
/// `EF_FASTBLUEFALL_90` — fast blue sheet rotated 90° (F1=1).
pub const FASTBLUEFALL_90: WaterfallParams = WaterfallParams { rotate90: true, ..FASTBLUEFALL };

fn hash01(i: u32, salt: u32) -> f32 {
    let x = i
        .wrapping_mul(2_654_435_761)
        .wrapping_add(salt.wrapping_mul(40_503))
        .wrapping_add(0x9E37_79B9);
    let x = x ^ (x >> 15);
    (x % 100_000) as f32 / 100_000.0
}

/// One rising mist puff, in caster-relative native coords (−Y up).
#[derive(Clone, Copy)]
struct Mist {
    pos: [f32; 3],
    /// Horizontal wander phases (X, Z) and their per-frame rates.
    phase: [f32; 2],
    rate: [f32; 2],
    /// How many times this slot has looped — varies its respawn position.
    generation: u32,
}

pub struct WaterfallEffect {
    world_pos: [f32; 3],
    params: WaterfallParams,
    age_frames: f32,
    mist: Vec<Mist>,
}

impl WaterfallEffect {
    pub fn new(world_pos: [f32; 3], params: WaterfallParams) -> Self {
        let count = params.mist_calls * PARTICLES_PER_CALL;
        let mist = (0..count).map(|i| spawn_mist(i, 0, params)).collect();
        Self { world_pos, params, age_frames: 0.0, mist }
    }

    fn step_mist(&mut self, df: f32) {
        for (i, m) in self.mist.iter_mut().enumerate() {
            m.pos[0] += 0.01 * m.phase[0].sin() * df;
            m.pos[2] += 0.01 * m.phase[1].sin() * df;
            m.phase[0] += m.rate[0] * df;
            m.phase[1] += m.rate[1] * df;
            m.pos[1] -= MIST_RISE * df;
            if m.pos[1] < -8.0 {
                let next_gen = m.generation + 1;
                *m = spawn_mist(i as u32, next_gen, self.params);
            }
        }
    }
}

/// Mist spawn box: x∈[-22,22] (±11 small), y∈[0,30],
/// z∈[-5,5], then X/Z swapped for the 90° variant.
fn spawn_mist(i: u32, generation: u32, params: WaterfallParams) -> Mist {
    let s = generation.wrapping_mul(7);
    let x_span = if params.small { 22.0 } else { 44.0 };
    let mut x = hash01(i, s + 1) * x_span - x_span * 0.5;
    let y = hash01(i, s + 2) * 30.0;
    let mut z = hash01(i, s + 3) * 10.0 - 5.0;
    if params.rotate90 {
        std::mem::swap(&mut x, &mut z);
    }
    Mist {
        pos: [x, y, z],
        phase: [hash01(i, s + 4) * 360.0, hash01(i, s + 5) * 360.0],
        rate: [0.2 + hash01(i, s + 6) * 0.6, 0.2 + hash01(i, s + 7) * 0.6],
        generation,
    }
}

/// Faint mist alpha (0..1) for a puff at caster-relative `y` (native −Y up):
/// invisible below the feet, fades in near the base, holds ~50/255, fades out
/// as it climbs past `y = -4`.
fn mist_alpha(y: f32) -> f32 {
    let mut count = 50.0;
    if y < -4.0 {
        count = 50.0 + (y + 4.0) * 15.0;
    }
    if y > -0.5 {
        count = y * -100.0;
    }
    count.max(0.0) / 255.0
}

impl Effect for WaterfallEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let df = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += df;
        self.step_mist(df);
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [wx, wy, wz] = self.world_pos;
        let frames = self.age_frames as u32;

        // --- Sheet: four emitters × five scrolling strips. ---
        for ec in 0..EMITTERS {
            let width = if self.params.small { 18.0 } else { 36.0 } + ec as f32;
            let half_w = width * 0.5;
            let depth = ec as f32 - 1.0;
            let speed = (self.params.speed_base - self.params.speed_step * ec as i32) as f32;

            let mut scroll = (frames % speed as u32) as f32 * CELL / speed;
            let addtn = ((frames % (speed as u32 * 3)) / speed as u32) as usize;
            if self.params.reverse {
                // BlueFall: reverse the treadmill and cycle textures back.
                scroll = CELL - scroll;
            }

            for i in 0..STRIPS {
                let tn = if self.params.reverse {
                    ((i as i32 % 3) - addtn as i32).rem_euclid(3) as usize
                } else {
                    (i % 3 + addtn) % 3
                };
                let mut y_top = -((i + 1) as f32) * CELL + scroll;
                let mut y_bot = -(i as f32) * CELL + scroll;
                // V at the top / bottom edges of the strip.
                let (mut v_top, mut v_bot) = (1.0_f32, 0.0_f32);
                if i == 0 {
                    // Bottom strip retracts as the cell scrolls out.
                    v_bot = scroll * 0.025;
                    y_bot -= CELL * v_bot;
                } else if i == STRIPS - 1 {
                    // Top strip grows in at the cell boundary.
                    v_top = scroll * 0.025;
                    y_top += CELL * (1.0 - v_top);
                }

                // Corner order TL, TR, BR, BL (CCW from the front face).
                let mut corners = [
                    [-half_w, y_top, depth],
                    [half_w, y_top, depth],
                    [half_w, y_bot, depth],
                    [-half_w, y_bot, depth],
                ];
                for c in &mut corners {
                    if self.params.rotate90 {
                        c.swap(0, 2);
                    }
                    c[0] += wx;
                    c[1] += wy;
                    c[2] += wz;
                }

                out.push(EffectPrimitiveDraw::WorldQuad {
                    corners,
                    uv: [[0.0, v_top], [1.0, v_top], [1.0, v_bot], [0.0, v_bot]],
                    texture: self.params.textures[tn],
                    color: self.params.sheet_color,
                    blend: self.params.sheet_blend,
                });
            }
        }

        // --- Mist: rising additive greenish puffs at the base. ---
        let [r, g, b] = MIST_RGB;
        for m in &self.mist {
            let alpha = mist_alpha(m.pos[1]);
            if alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [wx + m.pos[0], wy + m.pos[1], wz + m.pos[2]],
                size: [4.5, 4.5],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: 0.0,
                texture: "freeze_a_small.bmp",
                color: [r, g, b, alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn step(e: &mut WaterfallEffect, frames: f32) {
        e.update(&EffectUpdateCtx { delta: frames / FRAMES_PER_SECOND, camera_target: None });
    }

    fn draws(e: &WaterfallEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn sheet_quads(prims: &[EffectPrimitiveDraw]) -> Vec<EffectPrimitiveDraw> {
        prims.iter().filter(|p| matches!(p, EffectPrimitiveDraw::WorldQuad { .. })).cloned().collect()
    }

    #[test]
    fn sheet_is_twenty_alpha_white_quads_narrower_when_small() {
        // Four emitters × five strips = 20 alpha-blended white WorldQuads on the
        // T1 textures; the "small" variant draws the same count but a narrower
        // sheet (distance 18+ec vs 36+ec).
        let mut big = WaterfallEffect::new([10.0, 0.0, 20.0], WATERFALL);
        step(&mut big, 1.0);
        let bq = sheet_quads(&draws(&big));
        assert_eq!(bq.len(), EMITTERS * STRIPS);
        for p in &bq {
            let EffectPrimitiveDraw::WorldQuad { color, blend, texture, .. } = p else { unreachable!() };
            assert_eq!(*blend, BlendKind::Alpha);
            assert_eq!(*color, [1.0, 1.0, 1.0, SHEET_ALPHA]);
            assert!(T1.contains(texture), "T1 textures, got {texture}");
        }
        let span = |p: &EffectPrimitiveDraw| {
            let EffectPrimitiveDraw::WorldQuad { corners, .. } = p else { unreachable!() };
            (corners[1][0] - corners[0][0]).abs()
        };
        let mut small = WaterfallEffect::new([10.0, 0.0, 20.0], WATERFALL_SMALL);
        step(&mut small, 1.0);
        assert!(span(&sheet_quads(&draws(&small))[0]) < span(&bq[0]), "small sheet is narrower");
    }

    #[test]
    fn rotate90_swaps_the_spanning_axis_and_t2_picks_the_bright_set() {
        // The plain sheet spans X (constant Z per strip); the 90° variant spans
        // Z (constant X). T2 selects the waterfall3x textures.
        let mut plain = WaterfallEffect::new([0.0, 0.0, 0.0], WATERFALL);
        step(&mut plain, 1.0);
        let EffectPrimitiveDraw::WorldQuad { corners, .. } = &sheet_quads(&draws(&plain))[0] else { unreachable!() };
        assert!((corners[0][0] - corners[1][0]).abs() > 1.0 && (corners[0][2] - corners[1][2]).abs() < 1e-3);

        let mut rot = WaterfallEffect::new([0.0, 0.0, 0.0], WATERFALL_90);
        step(&mut rot, 1.0);
        let EffectPrimitiveDraw::WorldQuad { corners, .. } = &sheet_quads(&draws(&rot))[0] else { unreachable!() };
        assert!((corners[0][2] - corners[1][2]).abs() > 1.0 && (corners[0][0] - corners[1][0]).abs() < 1e-3);

        let mut t2 = WaterfallEffect::new([0.0, 0.0, 0.0], WATERFALL_T2);
        step(&mut t2, 1.0);
        let EffectPrimitiveDraw::WorldQuad { texture, .. } = &sheet_quads(&draws(&t2))[0] else { unreachable!() };
        assert!(T2.contains(texture), "T2 textures, got {texture}");
    }

    #[test]
    fn sheet_scrolls_and_cycles_textures_over_time() {
        // Over a full speed*3 period the first emitter's texture index visits all
        // three textures, and the strip's vertical offset moves between frames.
        let mut e = WaterfallEffect::new([0.0, 0.0, 0.0], WATERFALL);
        let mut seen = std::collections::HashSet::new();
        let mut prev_y: Option<f32> = None;
        let mut moved = false;
        for _ in 0..240 {
            step(&mut e, 1.0);
            let q = sheet_quads(&draws(&e));
            let EffectPrimitiveDraw::WorldQuad { corners, texture, .. } = &q[5] else { unreachable!() };
            seen.insert(*texture);
            if let Some(p) = prev_y {
                if (corners[0][1] - p).abs() > 1e-4 {
                    moved = true;
                }
            }
            prev_y = Some(corners[0][1]);
        }
        assert_eq!(seen.len(), 3, "cycles through all three textures");
        assert!(moved, "sheet scrolls vertically");
    }

    #[test]
    fn bluefall_is_additive_blue_with_no_mist() {
        // BlueFall reuses the sheet but additive-blue, and launches no mist.
        let mut e = WaterfallEffect::new([0.0, 0.0, 0.0], BLUEFALL);
        step(&mut e, 1.0);
        let all = draws(&e);
        assert!(
            all.iter().all(|p| matches!(p, EffectPrimitiveDraw::WorldQuad { .. })),
            "no Billboard mist for BlueFall",
        );
        let q = sheet_quads(&all);
        assert_eq!(q.len(), EMITTERS * STRIPS);
        for p in &q {
            let EffectPrimitiveDraw::WorldQuad { color, blend, .. } = p else { unreachable!() };
            assert_eq!(*blend, BlendKind::Additive);
            assert!(color[2] > color[0] && color[2] > color[1], "blue tint: {color:?}");
        }
    }

    #[test]
    fn fastbluefall_cycles_textures_faster_than_bluefall() {
        // Emitter 0's speed is `speed_base` (80 normal, 30 fast); the faster
        // sheet changes its texture index more often over the same frames.
        fn tex_changes(params: WaterfallParams) -> usize {
            let mut e = WaterfallEffect::new([0.0, 0.0, 0.0], params);
            let mut changes = 0;
            let mut prev: Option<&'static str> = None;
            for _ in 0..120 {
                step(&mut e, 1.0);
                let q = sheet_quads(&draws(&e));
                let EffectPrimitiveDraw::WorldQuad { texture, .. } = &q[0] else { unreachable!() };
                if prev.is_some_and(|p| p != *texture) {
                    changes += 1;
                }
                prev = Some(*texture);
            }
            changes
        }
        assert!(
            tex_changes(FASTBLUEFALL) > tex_changes(BLUEFALL),
            "fast variant cycles faster",
        );
    }

    #[test]
    fn mist_is_additive_persists_and_loops() {
        // Mist puffs are additive greenish billboards; the effect never dies and
        // the puff count stays bounded as particles loop back to the base.
        let mut e = WaterfallEffect::new([0.0, 0.0, 0.0], WATERFALL);
        let mut status = EffectStatus::Running;
        for _ in 0..600 {
            status = e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None });
        }
        assert_eq!(status, EffectStatus::Running, "persistent map effect");
        let mist: Vec<_> = draws(&e)
            .into_iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Billboard { .. }))
            .collect();
        assert!(!mist.is_empty(), "some puffs are in the visible band");
        for p in &mist {
            let EffectPrimitiveDraw::Billboard { blend, color, texture, .. } = p else { unreachable!() };
            assert_eq!(*blend, BlendKind::Additive);
            assert_eq!(*texture, "freeze_a_small.bmp");
            assert!(color[1] >= color[0] && color[1] >= color[2], "greenish tint");
        }
    }
}
