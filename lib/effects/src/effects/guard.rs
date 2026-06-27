use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{BodyTint, Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

const FADE_OUT_START: u32 = 26;
const DEATH_PROCESS: u32 = 43;
pub const TOTAL_DURATION_MS: u32 = 720;

const COLUMNS: u32 = 5;
const ROWS: u32 = 3;
const LAYER_RADIUS_ADD: [f32; 3] = [0.0, 0.2, 0.4];
const ROT_OFFSET_DEG: f32 = 270.0;
const GREEN_LAYERS: [[f32; 3]; 3] = [
    [205.0 / 255.0, 1.0, 205.0 / 255.0],
    [155.0 / 255.0, 1.0, 155.0 / 255.0],
    [225.0 / 255.0, 1.0, 225.0 / 255.0],
];
const GOLD: [f32; 3] = [1.0, 155.0 / 255.0, 0.0];
const GOLD_LAYERS: [[f32; 3]; 3] = [GOLD, GOLD, GOLD];

#[derive(Clone, Copy, Debug)]
pub struct GuardParams {
    pub tex0: &'static str,
    pub tex1: &'static str,
    pub max_height: f32,
    pub spin: bool,
    pub layer_rgb: [[f32; 3]; 3],
    pub body_flash: bool,
}

pub const GUARD: GuardParams = GuardParams {
    tex0: "guardK.tga",
    tex1: "guardK2.tga",
    max_height: 7.8,
    spin: false,
    layer_rgb: GREEN_LAYERS,
    body_flash: false,
};

pub const GUARD3: GuardParams = GuardParams {
    layer_rgb: GOLD_LAYERS,
    ..GUARD
};

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

    fn process(&self) -> u32 {
        self.frames + 1
    }

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
        let base = self.caster_yaw.map(f32::to_degrees).unwrap_or(0.0) + ROT_OFFSET_DEG;
        if self.params.spin {
            base + self.frames as f32 * 10.0
        } else {
            base
        }
    }
}

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
            let add2 = row as f32 * 45.0 - 45.0;
            let dist = 1.5 - (row as f32 - 1.0).abs() + 1.5;
            let (sn_lat, cs_lat) = add2.to_radians().sin_cos();
            let (sn_o2, cs_o2) = (90.0 + add2).to_radians().sin_cos();

            for col in 0..COLUMNS {
                let add = col as f32 * 45.0 - 90.0;
                let (sn_lon, cs_lon) = (rot_start + add).to_radians().sin_cos();
                let (sn_o1, cs_o1) = (rot_start + add - 90.0).to_radians().sin_cos();

                for layer in 0..3 {
                    let radius =
                        mh + LAYER_RADIUS_ADD[layer] + if layer == 2 { height0 } else { 0.0 };
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
                let d = ((c[0] - 10.0).powi(2) + c[1].powi(2) + (c[2] - 20.0).powi(2)).sqrt();
                assert!(d < 3.0 * GUARD.max_height, "corner within shell radius");
            }
        }
        assert_eq!(additive_tex0, 30, "two additive layers");
        assert_eq!(alpha_tex1, 15, "one alpha layer");
    }

    #[test]
    fn alpha_rises_holds_then_dies_and_tint_per_variant() {
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
