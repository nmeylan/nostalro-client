//! `EF_THUNDERSTORM2` (#622) — Lightning Crash: a vertical lightning bolt
//! that strikes the caster's tile repeatedly over the effect's lifetime.
//!
//! The original plays a single particle sprite — size 2.5,
//! anim speed 2, lifted ~4.5 off the ground (native RO `-Y = up`,
//! so the sprite floats above the tile),
//! additive and drawn with no depth check. The sprite/act are absent from
//! the GRF, but its
//! constituent textures survive: four jagged bolt segments
//! (`썬더스톰1..4.tga`, 128×128) and one radial impact flash
//! (`썬더스톰파티클.tga`, 64×64). We replay the sprite by compositing those
//! directly.
//!
//! The bolt segments tile vertically — each carries the jagged streak across
//! both its top and bottom edge, so stacking them forms one continuous bolt.
//! The original sprite chains them: a fixed top segment (`1`), a middle segment
//! that animates through `2→3→4`, and the radial flash (`파티클`) at the base.
//!
//! The original-game gif (ground truth) shows the bolt flickering: each strike
//! snaps on at full brightness, then fades, dark between strikes, a new one
//! landing roughly every ~9 ticks. We reproduce that cadence directly — the
//! exact frame schedule is unreadable, so the strike timing is
//! measured from the gif.
//!
//! Layout (each bolt segment fills its full quad height, base at the bottom
//! edge): the flash sits at the caster tile, the middle bolt segment rises from
//! it, and the top segment caps the bolt — all stacked straight up
//! (native RO `-Y = up`).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// Original game parent duration: 200 frames @ 60 fps.
const DURATION_FRAMES: f32 = 200.0;
pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

/// Fixed top bolt segment.
pub const TOP_TEXTURE: &str = "썬더스톰1.tga";
/// Middle bolt segment, animated one frame per strike.
pub const MIDDLE_TEXTURES: &[&str] = &["썬더스톰2.tga", "썬더스톰3.tga", "썬더스톰4.tga"];
/// Radial impact flash at the bolt base.
pub const STAR_TEXTURE: &str = "썬더스톰파티클.tga";

pub const TEXTURES: &[&str] = &[
    "썬더스톰1.tga",
    "썬더스톰2.tga",
    "썬더스톰3.tga",
    "썬더스톰4.tga",
    "썬더스톰파티클.tga",
];

/// Square side of one stacked bolt segment (world units). Two segments chain
/// for a bolt ~3 character heights tall.
const BOLT_SIZE: f32 = 12.0;
/// Side of the impact-flash billboard.
const STAR_SIZE: f32 = 9.0;

/// Ticks between successive strikes; the bolt is bright for `BRIGHT_FRAMES` of
/// them, dark for the rest. Measured from the original-game gif (~9-tick
/// cadence, ~5-tick bright window).
const STRIKE_PERIOD_FRAMES: f32 = 9.0;
const BRIGHT_FRAMES: f32 = 5.0;
/// The strike snaps on at full brightness (lightning has no fade-in in the
/// original-game gif), then fades out over the tail of the bright window.
const FADE_OUT_FRAMES: f32 = 3.0;

pub struct Thunderstorm2Effect {
    /// Caster tile (ground), in world coords.
    world_pos: [f32; 3],
    age: f32,
}

impl Thunderstorm2Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
        }
    }

    fn age_frames(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }

    /// Middle-segment frame for the current strike (advances 2→3→4 per strike).
    fn middle_index(&self) -> usize {
        let strike = (self.age_frames() / STRIKE_PERIOD_FRAMES) as usize;
        strike % MIDDLE_TEXTURES.len()
    }

    /// Alpha of the current strike: a trapezoid over the bright window, 0 while
    /// dark between strikes.
    fn strike_alpha(&self) -> f32 {
        let local = self.age_frames() % STRIKE_PERIOD_FRAMES;
        if local >= BRIGHT_FRAMES {
            return 0.0;
        }
        ((BRIGHT_FRAMES - local) / FADE_OUT_FRAMES).clamp(0.0, 1.0)
    }
}

impl Effect for Thunderstorm2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age_frames() >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = self.strike_alpha();
        if alpha <= 0.0 {
            return;
        }
        let [x, y, z] = self.world_pos;
        let uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        // The strike draws with no depth check, so the bolt and
        // flash draw over the terrain instead of being depth-culled by the
        // ground its base sits on. `BillboardFlash` is the matching no-depth quad.
        let bolt = |out: &mut EffectDrawList, center_y: f32, texture: &'static str| {
            out.push(EffectPrimitiveDraw::BillboardFlash {
                pos: [x, center_y, z],
                size: [BOLT_SIZE, BOLT_SIZE],
                uv,
                rotation: 0.0,
                texture,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
            });
        };
        // Flash star sits at the impact (caster tile).
        out.push(EffectPrimitiveDraw::BillboardFlash {
            pos: [x, y, z],
            size: [STAR_SIZE, STAR_SIZE],
            uv,
            rotation: 0.0,
            texture: STAR_TEXTURE,
            color: [1.0, 1.0, 1.0, alpha],
            blend: BlendKind::Additive,
        });
        // Stacked bolt: middle segment rises from the impact, top segment caps
        // it. Native RO `-Y = up`, so each segment's center is half a side above
        // its base.
        bolt(
            out,
            y - BOLT_SIZE * 0.5,
            MIDDLE_TEXTURES[self.middle_index()],
        );
        bolt(out, y - BOLT_SIZE * 1.5, TOP_TEXTURE);
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

    fn draws(e: &Thunderstorm2Effect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(e: &mut Thunderstorm2Effect, dt: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        })
    }

    #[test]
    fn strike_stacks_flash_and_two_bolt_segments_as_additive_billboards() {
        // At the peak of a strike the flash plus two stacked bolt segments are
        // emitted, all additive, climbing in Y (native RO `-Y = up`).
        let e = Thunderstorm2Effect::new([5.0, 0.0, 7.0]);
        let prims = draws(&e); // strike snaps on at full brightness at age 0
        assert_eq!(prims.len(), 3, "flash + middle + top");
        let mut ys = Vec::new();
        for p in prims {
            let EffectPrimitiveDraw::BillboardFlash { pos, blend, .. } = p else {
                panic!("expected BillboardFlash");
            };
            assert_eq!(blend, BlendKind::Additive);
            ys.push(pos[1]);
        }
        // Flash at ground, middle above it, top highest (smallest Y).
        assert!(ys.iter().any(|y| y.abs() < 1e-4), "flash at ground");
        assert_eq!(
            ys.iter().cloned().fold(f32::INFINITY, f32::min),
            -BOLT_SIZE * 1.5,
            "top segment sits two half-sides up",
        );
    }

    #[test]
    fn goes_dark_between_strikes() {
        // Mid-period (after the bright window) emits nothing.
        let mut e = Thunderstorm2Effect::new([0.0; 3]);
        // Advance to a dark tick: local frame = 7 (> BRIGHT_FRAMES = 5).
        step(&mut e, 7.0 / FRAMES_PER_SECOND);
        assert!(draws(&e).is_empty(), "no draws while dark between strikes");
    }

    #[test]
    fn middle_segment_advances_each_strike() {
        // The middle bolt segment cycles 2→3→4 across consecutive strikes; the
        // top segment stays fixed.
        let mut e = Thunderstorm2Effect::new([0.0; 3]);
        // Sit ~1 tick into each strike's bright window so float drift in the
        // period stepping can't land us on a dark/boundary tick.
        step(&mut e, 1.0 / FRAMES_PER_SECOND);
        let mut seen = Vec::new();
        for _ in 0..MIDDLE_TEXTURES.len() {
            for p in draws(&e) {
                if let EffectPrimitiveDraw::BillboardFlash { texture, .. } = p {
                    if MIDDLE_TEXTURES.contains(&texture) {
                        seen.push(texture);
                    }
                }
            }
            // Jump to the bright window of the next strike.
            step(&mut e, STRIKE_PERIOD_FRAMES / FRAMES_PER_SECOND);
        }
        assert_eq!(seen, MIDDLE_TEXTURES);
    }

    #[test]
    fn dies_after_parent_duration() {
        let mut e = Thunderstorm2Effect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < DURATION_FRAMES / FRAMES_PER_SECOND + 0.1 {
            status = step(&mut e, 0.05);
            t += 0.05;
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
