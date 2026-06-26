//! `RG_COIN` / `RG_COIN2` / `INTIMIDATE` — the Rogue
//! coin/strip bursts: Steal Coin (274), Full Strip (495), Disarm (627) and
//! Intimidate (227). A swarm of tumbling,
//! camera-facing item billboards erupts from a point above the caster and
//! expands outward on a growing sphere while fading, each member ejecting on
//! its own staggered delay.
//!
//! Every coin sits on a sphere of
//! radius `distance` (start 12) around a centre lifted 12 units above the
//! caster, at a random latitude/longitude. Once a coin's `process` counter
//! passes 0 (it starts at `-random(30)-18`, so 18–47 frames of delay) the
//! sphere radius grows `growth`/frame, the coin spins, its alpha ramps
//! `+25/frame` to ~250 over the first 10 frames, holds to frame 25, then
//! fades `-7/frame`. Additive blend, tinted per variant
//! (additive: `SRC_ALPHA, ONE`).
//!
//! Two distinct quad styles are drawn:
//!   * coins — a **world-space vertical diamond spinning
//!     around the world Y axis** (+5°/frame). The in-plane corner angle
//!     is random but static; the Y rotation's cos-projection
//!     squashes the quad's width as it turns, reading as a coin flipping in
//!     flight. [`CoinStyle::FlipY`].
//!   * items — corners flattened to a horizontal diamond then
//!     turned toward the screen: a camera-facing billboard spinning in-plane
//!     at +3°/frame. [`CoinStyle::Billboard`].
//!
//! Each launch spawns a prim holding **four** sub-emitters
//! in a loop, and the dispatch fires several launches at frame
//! 0, so the coin count is `launches × 4`. Variants differ only by the icon
//! set, quad size, tint and spin rate:
//!   * 274 Steal Coin — 30 launches → 120 gold coins (`coin_a.bmp`),
//!     pale-yellow tint.
//!   * 495 Full Strip — 4×(shield + sword) launches → 16 shields + 16 swords,
//!     reddish tint, larger.
//!   * 627 Disarm — 4 launches → 16 stripped-weapon icons (4 each). The
//!     original game names four specific weapon item icons (`gold_lux`,
//!     `double_shotgun`, …) that are absent from the classic GRF, so we
//!     substitute four present weapon textures.
//!
//! In the original game the Steal Coin *skill* launches the coin swarm
//! **plus** the Stealcoin effect (268, `steal_coin.str` — the money bag
//! with coin glints trickling down its sides). Our skill→effect wiring
//! doesn't exist yet, so 274 embeds the bag via [`Effect::str_overlay`];
//! should a server ever send 268 alongside 274 the bag double-renders, which
//! is acceptable.
//!
//! Structurally this mirrors [`super::twilight`] (a caster-centred swarm
//! emitting one camera-facing billboard per member), with a per-coin ejection
//! delay and tumble in place of the hover/drift.
//!
//! [`Effect::str_overlay`]: crate::effect_trait::Effect::str_overlay

use std::f32::consts::{SQRT_2, TAU};

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// The radius and growth literals are large-world numbers;
/// downscale uniformly so the bright coins (radius ~27–50 at peak alpha) burst
/// around the caster rather than across the map.
const WORLD_SCALE: f32 = 0.2;

/// Initial sphere radius.
const INITIAL_DISTANCE: f32 = 12.0;

/// Stagger spread: the delay is `delay_base + random(DELAY_RANGE)`.
const DELAY_RANGE: u32 = 30;

/// Alpha law (0–255): rises `alpha_rise_per_frame_255` over the first
/// 10 frames to `alpha_max_255`, holds to frame 25, then `−7/frame`.
const ALPHA_RISE_FRAMES: f32 = 10.0;
const ALPHA_FADE_START: f32 = 25.0;
const ALPHA_FALL_255_PER_FRAME: f32 = 7.0;

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

/// One icon set within a burst: a texture spawned `count` times at quad
/// corner-radius `size`.
#[derive(Clone, Copy)]
pub struct CoinGroup {
    pub texture: &'static str,
    pub count: usize,
    pub size: f32,
}

/// Which quad style a variant uses (see module docs).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CoinStyle {
    /// Vertical world-space diamond spinning around the world Y axis — the
    /// spin squashes its width like a coin flipping in flight.
    FlipY,
    /// Camera-facing billboard spinning in the screen plane.
    Billboard,
}

#[derive(Clone, Copy)]
pub struct RgCoinParams {
    pub groups: &'static [CoinGroup],
    /// Tint, 0–1.
    pub color: [f32; 3],
    /// Sphere-radius growth per frame.
    pub growth: f32,
    /// Spin rate in degrees per frame — the Y flip for
    /// [`CoinStyle::FlipY`], the in-plane corner advance for
    /// [`CoinStyle::Billboard`].
    pub spin_deg_per_frame: f32,
    pub style: CoinStyle,
    /// Lifts the burst centre above the caster (`−Y` is up).
    /// Coins use 12; Intimidate 9.
    pub center_lift: f32,
    /// Stagger base: ejection delay is `delay_base + random(30)` frames.
    /// Coins use 18 (18–47); Intimidate 70 (70–100, a much longer trickle).
    pub delay_base: u32,
    /// Peak alpha (0–255). Coins peak at 250; the Intimidate dim
    /// branch rises slower (`+15`/frame) so it caps at 150 — dimmer.
    pub alpha_max_255: f32,
    /// STR played alongside at the anchor (the Steal Coin money bag).
    pub str_overlay: Option<&'static str>,
}

impl RgCoinParams {
    /// Per-frame alpha-rise rate: `alpha_max_255` reached over
    /// [`ALPHA_RISE_FRAMES`].
    const fn alpha_rise_per_frame_255(&self) -> f32 {
        self.alpha_max_255 / ALPHA_RISE_FRAMES
    }

    /// Wall-clock end: the latest-delayed coin plus hold + linear fade-out.
    pub fn total_duration_ms(&self) -> u32 {
        let life = ALPHA_FADE_START + self.alpha_max_255 / ALPHA_FALL_255_PER_FRAME;
        let frames = (self.delay_base + DELAY_RANGE) as f32 + life;
        (frames / FRAMES_PER_SECOND * 1000.0) as u32
    }
}

const COIN_A: &str = "coin_a.bmp";
const SHIELD: &str = "shield.bmp";
const SWORD: &str = "sword.bmp";
const BLACK_SWORD: &str = "black_sword.bmp";
const LEXAETERNA_SWORD: &str = "lexaeterna_sword.bmp";
const WHITE01: &str = "white01.bmp";

pub const TEXTURES: &[&str] = &[
    COIN_A,
    SHIELD,
    SWORD,
    BLACK_SWORD,
    LEXAETERNA_SWORD,
    WHITE01,
];

// 274 Steal Coin — 30 launches × 4 = 120 gold coins flipping around Y,
// pale-yellow tint, plus the money-bag STR at the anchor.
pub const RG_COIN: RgCoinParams = RgCoinParams {
    groups: &[CoinGroup {
        texture: COIN_A,
        count: 120,
        size: 3.5,
    }],
    color: [250.0 / 255.0, 250.0 / 255.0, 155.0 / 255.0],
    growth: 1.5,
    spin_deg_per_frame: 5.0,
    style: CoinStyle::FlipY,
    center_lift: 12.0,
    delay_base: 18,
    alpha_max_255: 250.0,
    str_overlay: Some("steal_coin"),
};

// 495 Full Strip — 4×(shield + sword) launches × 4 = 16 shields + 16 swords,
// reddish tint, larger icons.
pub const RG_COIN2: RgCoinParams = RgCoinParams {
    groups: &[
        CoinGroup {
            texture: SHIELD,
            count: 16,
            size: 5.0,
        },
        CoinGroup {
            texture: SWORD,
            count: 16,
            size: 9.0,
        },
    ],
    color: [250.0 / 255.0, 100.0 / 255.0, 100.0 / 255.0],
    growth: 1.0,
    spin_deg_per_frame: 3.0,
    style: CoinStyle::Billboard,
    center_lift: 12.0,
    delay_base: 18,
    alpha_max_255: 250.0,
    str_overlay: None,
};

// 627 Disarm — 4 launches × 4 = 16 icons. The original four weapon item
// icons are absent from the classic GRF, so substitute four present weapon
// textures (4 of each).
pub const RG_COIN3: RgCoinParams = RgCoinParams {
    groups: &[
        CoinGroup {
            texture: SWORD,
            count: 4,
            size: 5.0,
        },
        CoinGroup {
            texture: BLACK_SWORD,
            count: 4,
            size: 5.0,
        },
        CoinGroup {
            texture: LEXAETERNA_SWORD,
            count: 4,
            size: 5.0,
        },
        CoinGroup {
            texture: SHIELD,
            count: 4,
            size: 5.0,
        },
    ],
    color: [250.0 / 255.0, 100.0 / 255.0, 100.0 / 255.0],
    growth: 1.0,
    spin_deg_per_frame: 3.0,
    style: CoinStyle::Billboard,
    center_lift: 12.0,
    delay_base: 18,
    alpha_max_255: 250.0,
    str_overlay: None,
};

// 227 Intimidate — dispatched ×20 → 80 quads. The same
// primitive as the coins; the dim render branch
// uses the identical world-space flipping-diamond geometry, only re-tinted
// blue. Differences from the coins: a much longer ejection trickle
// (`-random(30)-70`), a lower centre lift (9 units), a dimmer alpha
// (the dim branch ramps `+15`/frame, capping at 150) and the
// `white01.bmp` spark texture.
pub const INTIMIDATE: RgCoinParams = RgCoinParams {
    groups: &[CoinGroup {
        texture: WHITE01,
        count: 80,
        size: 3.5,
    }],
    color: [100.0 / 255.0, 150.0 / 255.0, 255.0 / 255.0],
    growth: 1.5,
    spin_deg_per_frame: 5.0,
    style: CoinStyle::FlipY,
    center_lift: 9.0,
    delay_base: 70,
    alpha_max_255: 150.0,
    str_overlay: None,
};

/// Deterministic LCG — same constants as `twilight` / `stormgust`; avoids a
/// runtime `rand` dependency for the per-coin scatter and stagger.
fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn lcg_unit(state: &mut u32) -> f32 {
    (lcg_next(state) >> 8) as f32 / ((1u32 << 24) as f32)
}

struct Coin {
    texture: &'static str,
    /// Quad corner radius.
    size: f32,
    /// Frames before `process > 0`.
    delay: f32,
    /// Sphere latitude.
    elevation: f32,
    /// Sphere longitude.
    azimuth: f32,
    /// In-plane corner angle — static for `FlipY` coins, the
    /// spinning rotation for `Billboard` items.
    rot0: f32,
    /// Initial Y-flip angle, advanced by `spin_deg_per_frame`
    /// for `FlipY` coins.
    flip0: f32,
}

pub struct RgCoinEffect {
    anchor: [f32; 3],
    params: RgCoinParams,
    coins: Vec<Coin>,
    age: f32,
}

impl RgCoinEffect {
    pub fn new(anchor: [f32; 3], params: RgCoinParams) -> Self {
        let mut state: u32 = 0x5EED_C01D;
        let coins = params
            .groups
            .iter()
            .flat_map(|g| (0..g.count).map(move |_| (g.texture, g.size)))
            .map(|(texture, size)| {
                let delay = (params.delay_base + (lcg_next(&mut state) % DELAY_RANGE)) as f32;
                Coin {
                    texture,
                    size,
                    delay,
                    elevation: lcg_unit(&mut state) * TAU,
                    azimuth: lcg_unit(&mut state) * TAU,
                    rot0: lcg_unit(&mut state) * TAU,
                    flip0: lcg_unit(&mut state) * TAU,
                }
            })
            .collect();
        Self {
            anchor,
            params,
            coins,
            age: 0.0,
        }
    }
}

impl Effect for RgCoinEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age * 1000.0 >= self.params.total_duration_ms() as f32 {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let age_frames = self.age * FRAMES_PER_SECOND;
        let spin_per_frame = self.params.spin_deg_per_frame.to_radians();
        let max_alpha = self.params.alpha_max_255 / 255.0;
        let alpha_rise = self.params.alpha_rise_per_frame_255() / 255.0;
        for coin in &self.coins {
            let process = age_frames - coin.delay;
            if process <= 0.0 {
                continue;
            }
            let alpha = if process <= ALPHA_RISE_FRAMES {
                process * alpha_rise
            } else if process <= ALPHA_FADE_START {
                max_alpha
            } else {
                max_alpha - (process - ALPHA_FADE_START) * (ALPHA_FALL_255_PER_FRAME / 255.0)
            };
            if alpha <= 0.0 {
                continue;
            }

            let distance = INITIAL_DISTANCE + self.params.growth * process;
            let (sin_lat, cos_lat) = coin.elevation.sin_cos();
            let (sin_lon, cos_lon) = coin.azimuth.sin_cos();
            let horiz = cos_lat * distance;
            let pos = [
                self.anchor[0] + (cos_lon * horiz) * WORLD_SCALE,
                self.anchor[1] + (-self.params.center_lift + sin_lat * distance) * WORLD_SCALE,
                self.anchor[2] + (sin_lon * horiz) * WORLD_SCALE,
            ];

            let [r, g, b] = self.params.color;
            let color = [r, g, b, alpha];

            match self.params.style {
                CoinStyle::FlipY => {
                    // Vertical diamond with corner radius `size`, spun around
                    // the world Y axis so its width squashes as it turns.
                    let radius = coin.size * WORLD_SCALE;
                    let (sin_flip, cos_flip) = (coin.flip0 + spin_per_frame * process).sin_cos();
                    let mut corners = [[0.0f32; 3]; 4];
                    for (k, corner) in corners.iter_mut().enumerate() {
                        let (sin_c, cos_c) =
                            (coin.rot0 + k as f32 * std::f32::consts::FRAC_PI_2).sin_cos();
                        let u = cos_c * radius;
                        *corner = [
                            pos[0] + cos_flip * u,
                            pos[1] + sin_c * radius,
                            pos[2] + sin_flip * u,
                        ];
                    }
                    out.push(EffectPrimitiveDraw::WorldQuad {
                        corners,
                        uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                        texture: coin.texture,
                        color,
                        blend: BlendKind::Additive,
                        no_depth: false,
                    });
                }
                CoinStyle::Billboard => {
                    // Diamond corners at `size` → rendered side `size * √2`.
                    let side = coin.size * SQRT_2 * WORLD_SCALE;
                    out.push(EffectPrimitiveDraw::Billboard {
                        pos,
                        size: [side, side],
                        uv: UNIT_UV,
                        rotation: coin.rot0 + spin_per_frame * process,
                        texture: coin.texture,
                        color,
                        blend: BlendKind::Additive,
                    });
                }
            }
        }
    }

    fn str_overlay(&self) -> Option<&'static str> {
        self.params.str_overlay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step(e: &mut RgCoinEffect, frames: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..frames {
            s = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        s
    }

    /// Normalised view of either quad style: centre position, texture, alpha,
    /// and a spin signature (billboard rotation, or the world angle of a
    /// FlipY diamond's first corner around its centre).
    fn quads(e: &RgCoinEffect) -> Vec<([f32; 3], &'static str, f32, f32)> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &ctx());
        l.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard {
                    pos,
                    texture,
                    color,
                    rotation,
                    ..
                } => (*pos, *texture, color[3], *rotation),
                EffectPrimitiveDraw::WorldQuad {
                    corners,
                    texture,
                    color,
                    ..
                } => {
                    let center = [
                        corners.iter().map(|c| c[0]).sum::<f32>() / 4.0,
                        corners.iter().map(|c| c[1]).sum::<f32>() / 4.0,
                        corners.iter().map(|c| c[2]).sum::<f32>() / 4.0,
                    ];
                    let spin = (corners[0][2] - center[2]).atan2(corners[0][0] - center[0]);
                    (center, *texture, color[3], spin)
                }
                _ => panic!("rg_coin only emits Billboard/WorldQuad"),
            })
            .collect()
    }

    #[test]
    fn steal_coin_emits_flipping_world_quads_after_their_delay() {
        let e = RgCoinEffect::new([0.0; 3], RG_COIN);
        assert_eq!(
            e.str_overlay(),
            Some("steal_coin"),
            "money-bag STR plays alongside"
        );
        let mut e = e;
        // Before any delay elapses nothing is visible.
        step(&mut e, 1);
        assert!(quads(&e).is_empty(), "coins wait out their ejection delay");
        // Past the max delay every coin is live and uses the gold texture.
        step(&mut e, RG_COIN.delay_base + DELAY_RANGE + 5);
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &ctx());
        assert!(
            l.primitives
                .iter()
                .all(|p| matches!(p, EffectPrimitiveDraw::WorldQuad { .. })),
            "coins are world-space flipping diamonds, not billboards"
        );
        let draws = quads(&e);
        assert_eq!(draws.len(), 120);
        assert!(draws.iter().all(|d| d.1 == "coin_a.bmp"));
    }

    #[test]
    fn intimidate_is_a_dim_blue_long_delayed_swarm() {
        let mut e = RgCoinEffect::new([0.0; 3], INTIMIDATE);
        // Coins are already long gone by the time Intimidate's trickle starts.
        step(&mut e, RG_COIN.delay_base + DELAY_RANGE + 5);
        assert!(
            quads(&e).is_empty(),
            "Intimidate quads delayed past the coin window"
        );
        // Past the max delay (100) every quad is live but none have faded yet.
        step(
            &mut e,
            INTIMIDATE.delay_base + DELAY_RANGE + 5 - (RG_COIN.delay_base + DELAY_RANGE + 5),
        );
        let draws = quads(&e);
        assert_eq!(draws.len(), 80, "20 launches × 4 = 80 quads");
        assert!(draws.iter().all(|d| d.1 == "white01.bmp"));
        // The dim branch caps at 150/255 — dimmer than the coins' 250.
        let peak = draws.iter().map(|d| d.2).fold(0.0_f32, f32::max);
        assert!(peak <= 150.0 / 255.0 + 1e-3, "dimmer than coins: {peak}");
    }

    #[test]
    fn full_strip_splits_shields_and_swords() {
        let mut e = RgCoinEffect::new([0.0; 3], RG_COIN2);
        step(&mut e, RG_COIN2.delay_base + DELAY_RANGE + 5);
        let draws = quads(&e);
        assert_eq!(draws.len(), 32);
        let shields = draws.iter().filter(|d| d.1 == "shield.bmp").count();
        let swords = draws.iter().filter(|d| d.1 == "sword.bmp").count();
        assert_eq!((shields, swords), (16, 16));
    }

    #[test]
    fn coins_expand_outward_and_tumble() {
        let mut e = RgCoinEffect::new([0.0; 3], RG_COIN);
        step(&mut e, RG_COIN.delay_base + DELAY_RANGE + 2);
        let early = quads(&e);
        let early_radius: f32 = early
            .iter()
            .map(|d| (d.0[0] * d.0[0] + d.0[2] * d.0[2]).sqrt())
            .sum::<f32>()
            / early.len() as f32;
        let early_rot = early[0].3;

        step(&mut e, 10);
        let late = quads(&e);
        let late_radius: f32 = late
            .iter()
            .map(|d| (d.0[0] * d.0[0] + d.0[2] * d.0[2]).sqrt())
            .sum::<f32>()
            / late.len() as f32;
        let late_rot = late[0].3;

        assert!(
            late_radius > early_radius,
            "burst expands: {early_radius} -> {late_radius}"
        );
        assert!(
            (late_rot - early_rot).abs() > 1e-3,
            "coins tumble over time"
        );
    }

    #[test]
    fn alpha_fades_out_and_effect_terminates() {
        let mut e = RgCoinEffect::new([0.0; 3], RG_COIN);
        // Earliest coin (delay 18) is at peak shortly after frame ~28.
        step(&mut e, RG_COIN.delay_base + 9);
        let peak = quads(&e).iter().map(|d| d.2).fold(0.0_f32, f32::max);
        let total_frames = (RG_COIN.total_duration_ms() as f32 / 1000.0 * FRAMES_PER_SECOND) as u32;
        let status = step(&mut e, total_frames);
        let late = quads(&e).iter().map(|d| d.2).fold(0.0_f32, f32::max);
        assert!(
            peak > late,
            "coins fade after their hold window: {peak} -> {late}"
        );
        assert_eq!(status, EffectStatus::Dead);
    }
}
