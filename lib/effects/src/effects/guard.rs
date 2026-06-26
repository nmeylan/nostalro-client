//! `Guard(texture, F1)` / `Guard2(texture)` — aura shell.
//!
//! All three ids (Guard 336, Guard3 675, Guard2 496) build the same shell in
//! the original game; they differ only in textures,
//! shell radius, spin, tint and a body flash. It is a
//! forward-facing hemispherical shell of panels: a grid of 5 longitude
//! columns (`add = count*45 - 90`) × 3 latitude rows (`add2 = count2*45 - 45`)
//! = 15 panels, each drawn as 3 stacked layers at increasing radius. The
//! first two layers are additive on the primary texture; the third is
//! alpha-blended on the secondary texture and drifts upward during fade-out.
//!
//! A single fade curve runs off a `process` counter: alpha ramps
//! `0 → 100` over the first frames, holds, then ramps back to 0 while the
//! outer layer rises. Guard2 (`flag1 == 1`) also spins the shell `+10°`/frame
//! and pulses the caster sprite white; the static variants lock the shell to
//! the caster's facing. Tint is green-white for Guard/Guard2 and gold for
//! Guard3 (`flag1 == 2`).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{BodyTint, Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// Fade-out begins once `process` passes this (alpha holds at 100 until here).
const FADE_OUT_START: u32 = 26;
/// `process` at which the `-6`/frame fade-out has driven alpha to 0; the
/// effect dies here. `100 / 6 ≈ 17` frames after [`FADE_OUT_START`] + 1.
const DEATH_PROCESS: u32 = 43;
/// Wall-clock lifetime — the visible animation ends when alpha hits 0, well
/// before the original game's longer parent-emitter lifetime.
pub const TOTAL_DURATION_MS: u32 = 720;

/// Shell grid: 5 longitude columns, 3 latitude rows.
const COLUMNS: u32 = 5;
const ROWS: u32 = 3;

/// Per-layer outward radius added to `max_height` (the third also rides the
/// fade-out `height0` drift).
const LAYER_RADIUS_ADD: [f32; 3] = [0.0, 0.2, 0.4];

/// Shell facing offset (caster facing + 270°). The live caster
/// yaw is added on top each frame; with no caster facing the shell falls back
/// to a fixed front (yaw 0), tuned so the open dome faces the export/viewer
/// camera.
const ROT_OFFSET_DEG: f32 = 270.0;

/// Green-white tint per layer (`205/155/225`,255,…), 0..1.
const GREEN_LAYERS: [[f32; 3]; 3] = [
    [205.0 / 255.0, 1.0, 205.0 / 255.0],
    [155.0 / 255.0, 1.0, 155.0 / 255.0],
    [225.0 / 255.0, 1.0, 225.0 / 255.0],
];
/// Gold tint `(255,155,0)` used for all three layers of Guard3.
const GOLD: [f32; 3] = [1.0, 155.0 / 255.0, 0.0];
const GOLD_LAYERS: [[f32; 3]; 3] = [GOLD, GOLD, GOLD];

#[derive(Clone, Copy, Debug)]
pub struct GuardParams {
    /// Additive panel texture (layers 0 and 1).
    pub tex0: &'static str,
    /// Alpha-blended outer panel texture (layer 2).
    pub tex1: &'static str,
    /// Shell radius.
    pub max_height: f32,
    /// `flag1 == 1`: spin the shell `+10°`/frame and pulse the caster body.
    pub spin: bool,
    /// Per-layer RGB tint (0..1). Gold variant repeats one colour.
    pub layer_rgb: [[f32; 3]; 3],
    /// Guard2 pulses the caster sprite white during the early frames.
    pub body_flash: bool,
}

/// `EF_GUARD` → `Guard("effect\\guardK.tga")`.
pub const GUARD: GuardParams = GuardParams {
    tex0: "guardK.tga",
    tex1: "guardK2.tga",
    max_height: 7.8,
    spin: false,
    layer_rgb: GREEN_LAYERS,
    body_flash: false,
};

/// `EF_GUARD3` → `Guard("effect\\guardK.tga", 2)`: gold tint.
pub const GUARD3: GuardParams = GuardParams {
    layer_rgb: GOLD_LAYERS,
    ..GUARD
};

/// `EF_GUARD2` → `Guard2("effect\\a01.bmp")`: spinning sparkle shell + body
/// flash, both panel textures are `a01.bmp`, larger radius.
pub const GUARD2: GuardParams = GuardParams {
    tex0: "a01.bmp",
    tex1: "a01.bmp",
    max_height: 10.0,
    spin: true,
    layer_rgb: GREEN_LAYERS,
    body_flash: true,
};

pub const TEXTURES: &[&str] = &["guardK.tga", "guardK2.tga", "a01.bmp"];

pub struct GuardEffect {
    world_pos: [f32; 3],
    params: GuardParams,
    caster_yaw: Option<f32>,
    age: f32,
    frames: u32,
}

impl GuardEffect {
    pub fn new(world_pos: [f32; 3], params: GuardParams) -> Self {
        Self {
            world_pos,
            params,
            caster_yaw: None,
            age: 0.0,
            frames: 0,
        }
    }

    /// `process` counter (1-based on the first drawn frame).
    fn process(&self) -> u32 {
        self.frames + 1
    }

    /// `(alpha out of 255-scale max 100, outer-layer drift)` for the current
    /// `process`. Alpha ramps `+20`/frame to 100, holds, then `-6`/frame to 0;
    /// the outer layer rises `+0.1`/frame once the fade-out starts.
    fn alpha_and_drift(&self) -> (f32, f32) {
        let p = self.process();
        if p <= FADE_OUT_START {
            ((p as f32 * 20.0).min(100.0), 0.0)
        } else {
            let out = (p - FADE_OUT_START) as f32;
            ((100.0 - out * 6.0).max(0.0), out * 0.1)
        }
    }

    fn rot_start_deg(&self) -> f32 {
        // Live caster facing + the 270° offset.
        let base = self.caster_yaw.map(f32::to_degrees).unwrap_or(0.0) + ROT_OFFSET_DEG;
        if self.params.spin {
            base + self.frames as f32 * 10.0
        } else {
            base
        }
    }
}

/// Place a panel corner offset
/// `(rx, ry)` in the panel's local frame, oriented by the latitude pair
/// `(sn2, cs2)` and longitude pair `(sn1, cs1)`, then translate to `center`.
fn guard_point(
    rx: f32,
    ry: f32,
    sn2: f32,
    cs2: f32,
    sn1: f32,
    cs1: f32,
    center: [f32; 3],
) -> [f32; 3] {
    let y = ry * sn2;
    let z0 = ry * cs2;
    let x0 = rx;
    let x = x0 * cs1 - z0 * sn1;
    let z = x0 * sn1 + z0 * cs1;
    [center[0] + x, center[1] + y, center[2] + z]
}

impl Effect for GuardEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.caster_yaw = ctx.caster_yaw;
        self.age += ctx.delta;
        self.frames = (self.age * FRAMES_PER_SECOND) as u32;
        if self.process() >= DEATH_PROCESS {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let (alpha_b, height0) = self.alpha_and_drift();
        if alpha_b <= 0.0 {
            return;
        }
        let alpha = alpha_b / 255.0;
        let rot_start = self.rot_start_deg();
        let mh = self.params.max_height;

        for row in 0..ROWS {
            // Latitude band and per-row panel half-size (middle row is larger).
            let add2 = row as f32 * 45.0 - 45.0;
            let dist = 1.5 - (row as f32 - 1.0).abs() + 1.5;
            let (sn_lat, cs_lat) = add2.to_radians().sin_cos();
            // Panel orientation latitude pair uses `90 + add2`.
            let (sn_o2, cs_o2) = (90.0 + add2).to_radians().sin_cos();

            for col in 0..COLUMNS {
                let add = col as f32 * 45.0 - 90.0;
                let (sn_lon, cs_lon) = (rot_start + add).to_radians().sin_cos();
                // Panel orientation longitude pair uses `rot_start + add - 90`.
                let (sn_o1, cs_o1) = (rot_start + add - 90.0).to_radians().sin_cos();

                for layer in 0..3 {
                    let radius =
                        mh + LAYER_RADIUS_ADD[layer] + if layer == 2 { height0 } else { 0.0 };
                    // Shell-relative panel centre. Layer 0 sits slightly lower.
                    let y_off = if layer == 0 { -(mh + 2.0) } else { -mh };
                    let center = [
                        radius * cs_lat * cs_lon,
                        radius * sn_lat + y_off,
                        radius * cs_lat * sn_lon,
                    ];
                    let corner = |rx: f32, ry: f32| {
                        let p = guard_point(rx, ry, sn_o2, cs_o2, sn_o1, cs_o1, center);
                        [
                            p[0] + self.world_pos[0],
                            p[1] + self.world_pos[1],
                            p[2] + self.world_pos[2],
                        ]
                    };
                    let corners = [
                        corner(dist, dist),
                        corner(-dist, dist),
                        corner(-dist, -dist),
                        corner(dist, -dist),
                    ];
                    let [r, g, b] = self.params.layer_rgb[layer];
                    let (texture, blend) = if layer == 2 {
                        (self.params.tex1, BlendKind::Alpha)
                    } else {
                        (self.params.tex0, BlendKind::Additive)
                    };
                    out.push(EffectPrimitiveDraw::WorldQuad {
                        corners,
                        uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                        texture,
                        color: [r, g, b, alpha],
                        blend,
                        no_depth: false,
                    });
                }
            }
        }
    }

    fn body_tint(&self) -> Option<BodyTint> {
        if self.params.body_flash && (10..=30).contains(&self.frames) && self.frames % 2 == 0 {
            Some(BodyTint {
                rgb: [250, 250, 250],
            })
        } else {
            None
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

    fn draws_after(params: GuardParams, secs: f32) -> Vec<EffectPrimitiveDraw> {
        let mut e = GuardEffect::new([10.0, 0.0, 20.0], params);
        e.update(&EffectUpdateCtx {
            delta: secs,
            camera_target: None,
            caster_yaw: None,
        });
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn shell_shape_layers_and_textures() {
        // Mid-hold: 15 panels × 3 layers = 45 quads. Layers 0/1 additive on
        // tex0, layer 2 alpha on tex1; every corner sits within the shell
        // radius of the caster.
        let prims = draws_after(GUARD, 15.0 / 60.0);
        assert_eq!(
            prims.len(),
            (COLUMNS * ROWS * 3) as usize,
            "5×3 panels, 3 layers"
        );
        let (mut additive_tex0, mut alpha_tex1) = (0, 0);
        for p in &prims {
            let EffectPrimitiveDraw::WorldQuad {
                corners,
                texture,
                blend,
                ..
            } = p
            else {
                panic!("expected WorldQuad, got {p:?}");
            };
            match blend {
                BlendKind::Additive => {
                    assert_eq!(*texture, "guardK.tga");
                    additive_tex0 += 1;
                }
                BlendKind::Alpha => {
                    assert_eq!(*texture, "guardK2.tga");
                    alpha_tex1 += 1;
                }
                other => panic!("unexpected blend {other:?}"),
            }
            for c in corners {
                // Shell is lifted ~(mh+2) above the caster origin to wrap the
                // body, so corners reach ~2·mh from it — a loose bound just
                // confirms the shell is bounded, not flying off.
                let d = ((c[0] - 10.0).powi(2) + c[1].powi(2) + (c[2] - 20.0).powi(2)).sqrt();
                assert!(d < 3.0 * GUARD.max_height, "corner within shell radius");
            }
        }
        assert_eq!(additive_tex0, 30, "two additive layers");
        assert_eq!(alpha_tex1, 15, "one alpha layer");
    }

    #[test]
    fn alpha_rises_holds_then_dies_and_tint_per_variant() {
        // Fade-in then hold: alpha is higher mid-hold than on the first frame.
        let early = draws_after(GUARD, 0.0);
        let hold = draws_after(GUARD, 15.0 / 60.0);
        let alpha_of = |p: &EffectPrimitiveDraw| {
            let EffectPrimitiveDraw::WorldQuad { color, .. } = p else {
                panic!()
            };
            color[3]
        };
        assert!(alpha_of(&early[0]) < alpha_of(&hold[0]), "ramps in");
        assert!(
            (alpha_of(&hold[0]) - 100.0 / 255.0).abs() < 1e-3,
            "holds at 100/255"
        );

        // Gold variant is red-dominant; green variant is green-dominant.
        let EffectPrimitiveDraw::WorldQuad { color: gold, .. } =
            &draws_after(GUARD3, 15.0 / 60.0)[0]
        else {
            panic!()
        };
        assert!(gold[0] > gold[1], "gold: r > g");
        let EffectPrimitiveDraw::WorldQuad { color: green, .. } = &hold[0] else {
            panic!()
        };
        assert!(green[1] >= green[0], "green: g >= r");

        // Self-terminates once the fade-out completes.
        let mut e = GuardEffect::new([0.0; 3], GUARD);
        let mut status = EffectStatus::Running;
        for _ in 0..DEATH_PROCESS + 5 {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn guard2_spins_and_flashes_body() {
        // The spinning variant advances panel azimuth between frames, so a
        // given corner moves; the static variant does not. Body flash is on
        // for an even frame in the window and off for Guard/Guard3.
        let f1 = draws_after(GUARD2, 5.0 / 60.0);
        let f2 = draws_after(GUARD2, 6.0 / 60.0);
        let EffectPrimitiveDraw::WorldQuad { corners: c1, .. } = &f1[0] else {
            panic!()
        };
        let EffectPrimitiveDraw::WorldQuad { corners: c2, .. } = &f2[0] else {
            panic!()
        };
        assert!(c1[0] != c2[0], "spinning shell rotates between frames");

        let mut spin = GuardEffect::new([0.0; 3], GUARD2);
        spin.update(&EffectUpdateCtx {
            delta: 12.0 / 60.0,
            camera_target: None,
            caster_yaw: None,
        });
        assert!(
            spin.body_tint().is_some(),
            "body flashes on even frame in window"
        );
        let mut still = GuardEffect::new([0.0; 3], GUARD);
        still.update(&EffectUpdateCtx {
            delta: 12.0 / 60.0,
            camera_target: None,
            caster_yaw: None,
        });
        assert!(still.body_tint().is_none(), "static guard never flashes");
    }
}
