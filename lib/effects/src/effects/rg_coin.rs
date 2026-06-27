use std::f32::consts::{SQRT_2, TAU};

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

const WORLD_SCALE: f32 = 0.2;

const INITIAL_DISTANCE: f32 = 12.0;

const DELAY_RANGE: u32 = 30;

const ALPHA_RISE_FRAMES: f32 = 10.0;
const ALPHA_FADE_START: f32 = 25.0;
const ALPHA_FALL_255_PER_FRAME: f32 = 7.0;

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

#[derive(Clone, Copy)]
pub struct CoinGroup {
    pub texture: &'static str,
    pub count: usize,
    pub size: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CoinStyle {
    FlipY,
    Billboard,
}

#[derive(Clone, Copy)]
pub struct RgCoinParams {
    pub groups: &'static [CoinGroup],
    pub color: [f32; 3],
    pub growth: f32,
    pub spin_deg_per_frame: f32,
    pub style: CoinStyle,
    /// Lifts the burst centre above the caster (`−Y` is up).
    pub center_lift: f32,
    pub delay_base: u32,
    pub alpha_max_255: f32,
    pub str_overlay: Option<&'static str>,
}

impl RgCoinParams {
    const fn alpha_rise_per_frame_255(&self) -> f32 {
        self.alpha_max_255 / ALPHA_RISE_FRAMES
    }

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

fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn lcg_unit(state: &mut u32) -> f32 {
    (lcg_next(state) >> 8) as f32 / ((1u32 << 24) as f32)
}

struct Coin {
    texture: &'static str,
    size: f32,
    delay: f32,
    elevation: f32,
    azimuth: f32,
    rot0: f32,
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
        step(&mut e, 1);
        assert!(quads(&e).is_empty(), "coins wait out their ejection delay");
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
        step(&mut e, RG_COIN.delay_base + DELAY_RANGE + 5);
        assert!(
            quads(&e).is_empty(),
            "Intimidate quads delayed past the coin window"
        );
        step(
            &mut e,
            INTIMIDATE.delay_base + DELAY_RANGE + 5 - (RG_COIN.delay_base + DELAY_RANGE + 5),
        );
        let draws = quads(&e);
        assert_eq!(draws.len(), 80, "20 launches × 4 = 80 quads");
        assert!(draws.iter().all(|d| d.1 == "white01.bmp"));
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
