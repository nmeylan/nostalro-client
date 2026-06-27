use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const FADE_DURATION_FRAMES: u32 = 180;
const ALPHA_RAMP_FRAMES: u32 = 40;
const ALPHA_FADE_START: u32 = 140;
const PEAK_ALPHA: f32 = ALPHA_RAMP_FRAMES as f32;
const SEGMENTS: usize = 5;
const FOREST_RGB: [f32; 3] = [230.0 / 255.0, 1.0, 230.0 / 255.0];

#[derive(Clone, Copy, Debug)]
pub struct ForestLightParams {
    pub texture: &'static str,
    pub color_rgb: [f32; 3],
    pub radii: [f32; 4],
    pub alpha_base: f32,
    /// When `true`: ramp alpha `0 → alpha_base → 0` and self-terminate at `FADE_DURATION_FRAMES`; column 3 is suppressed.
    pub fade: bool,
    pub bottom_offset: [f32; 3],
}

const LONG_OFFSET: [f32; 3] = [-70.0, -300.0, -70.0];

pub const FORESTLIGHT: ForestLightParams = ForestLightParams {
    texture: "cloud11.tga",
    color_rgb: FOREST_RGB,
    radii: [2.0, 3.0, 4.0, 2.0],
    alpha_base: 40.0,
    fade: false,
    bottom_offset: LONG_OFFSET,
};

pub const FORESTLIGHT2: ForestLightParams = ForestLightParams {
    radii: [4.0, 6.0, 8.0, 4.0],
    ..FORESTLIGHT
};

pub const FORESTLIGHT3: ForestLightParams = ForestLightParams {
    radii: [1.0, 1.5, 2.0, 1.0],
    alpha_base: 30.0,
    ..FORESTLIGHT
};

pub const FORESTLIGHT4: ForestLightParams = ForestLightParams {
    alpha_base: 25.0,
    bottom_offset: [-15.0, -60.0, -15.0],
    ..FORESTLIGHT
};

pub const ITEM_LIGHT: ForestLightParams = ForestLightParams {
    radii: [4.0, 6.0, 8.0, 4.0],
    fade: true,
    ..FORESTLIGHT
};

pub const TEXTURES: &[&str] = &["cloud11.tga"];

struct Column {
    rot_start_deg: f32,
    /// Column 3 starts at `FADE_DURATION_FRAMES` so the fade variant suppresses it.
    process_start: u32,
    breathes: bool,
}

const COLUMNS: [Column; 4] = [
    Column {
        rot_start_deg: 0.0,
        process_start: 0,
        breathes: false,
    },
    Column {
        rot_start_deg: 25.0,
        process_start: 0,
        breathes: true,
    },
    Column {
        rot_start_deg: 50.0,
        process_start: 0,
        breathes: false,
    },
    Column {
        rot_start_deg: 75.0,
        process_start: FADE_DURATION_FRAMES,
        breathes: true,
    },
];

pub struct ForestLightEffect {
    world_pos: [f32; 3],
    params: ForestLightParams,
    age: f32,
    frames: u32,
}

impl ForestLightEffect {
    pub fn new(world_pos: [f32; 3], params: ForestLightParams) -> Self {
        Self {
            world_pos,
            params,
            age: 0.0,
            frames: 0,
        }
    }

    fn column_alpha(&self, process: u32) -> f32 {
        if !self.params.fade {
            return self.params.alpha_base;
        }
        if process <= ALPHA_RAMP_FRAMES {
            process as f32
        } else if process > ALPHA_FADE_START {
            (PEAK_ALPHA - (process - ALPHA_FADE_START) as f32).max(0.0)
        } else {
            PEAK_ALPHA
        }
    }
}

fn column_radius(params: &ForestLightParams, ec: usize, process: u32) -> f32 {
    let base = params.radii[ec];
    if COLUMNS[ec].breathes {
        let sinp = (process % 720) as f32 * 0.5;
        base + sinp.to_radians().sin() * 0.5
    } else {
        base
    }
}

impl Effect for ForestLightEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        self.frames = (self.age * FRAMES_PER_SECOND) as u32;
        if self.params.fade && self.frames >= FADE_DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [tr, tg, tb] = self.params.color_rgb;
        let top_center = self.world_pos;
        let bottom_center = [
            self.world_pos[0] + self.params.bottom_offset[0],
            self.world_pos[1] + self.params.bottom_offset[1],
            self.world_pos[2] + self.params.bottom_offset[2],
        ];

        for (ec, col) in COLUMNS.iter().enumerate() {
            let process = self.frames + col.process_start;
            let alpha = self.column_alpha(process);
            if alpha <= 0.0 {
                continue;
            }
            let radius = column_radius(&self.params, ec, process);

            let ring_point = |center: [f32; 3], i: usize| {
                let angle_deg = (i as f32 * 72.0 + col.rot_start_deg) % 360.0;
                let (s, c) = angle_deg.to_radians().sin_cos();
                [center[0] + radius * c, center[1], center[2] + radius * s]
            };

            for i in 1..=SEGMENTS {
                let prev_top = ring_point(top_center, i - 1);
                let cur_top = ring_point(top_center, i);
                let cur_bottom = ring_point(bottom_center, i);
                let prev_bottom = ring_point(bottom_center, i - 1);
                out.push(EffectPrimitiveDraw::WorldQuad {
                    corners: [prev_top, cur_top, cur_bottom, prev_bottom],
                    uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    texture: self.params.texture,
                    color: [tr, tg, tb, alpha / 255.0],
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

    fn draws_after(params: ForestLightParams, secs: f32) -> Vec<EffectPrimitiveDraw> {
        let mut e = ForestLightEffect::new([10.0, 0.0, 20.0], params);
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
    fn item_light_fades_suppresses_one_column_and_dies() {
        let prims = draws_after(ITEM_LIGHT, 1.0);
        assert_eq!(prims.len(), 3 * SEGMENTS, "3 visible columns × 5 quads");
        for p in &prims {
            let EffectPrimitiveDraw::WorldQuad {
                corners,
                color,
                blend,
                texture,
                ..
            } = p
            else {
                panic!("expected WorldQuad, got {p:?}");
            };
            assert_eq!(*blend, BlendKind::Additive);
            assert_eq!(*texture, "cloud11.tga");
            assert!(color[1] >= color[0] && color[1] >= color[2], "greenish");
            assert!((color[3] - PEAK_ALPHA / 255.0).abs() < 1e-3, "hold alpha");
            assert!(
                corners[0][1] - corners[3][1] > 100.0,
                "tube spans the offset"
            );
        }
        let early = draws_after(ITEM_LIGHT, 10.0 / 60.0);
        let EffectPrimitiveDraw::WorldQuad { color, .. } = &early[0] else {
            panic!()
        };
        assert!(color[3] < PEAK_ALPHA / 255.0, "still fading in");
        let mut e = ForestLightEffect::new([0.0; 3], ITEM_LIGHT);
        let mut status = EffectStatus::Running;
        for _ in 0..FADE_DURATION_FRAMES + 5 {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn persistent_forest_shows_all_four_columns_at_constant_alpha() {
        let mut e = ForestLightEffect::new([0.0, 0.0, 0.0], FORESTLIGHT3);
        let mut status = EffectStatus::Running;
        for _ in 0..FADE_DURATION_FRAMES + 60 {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
        }
        assert_eq!(status, EffectStatus::Running, "persistent");
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), 4 * SEGMENTS, "all 4 columns visible");
        let EffectPrimitiveDraw::WorldQuad { color, .. } = &list.primitives[0] else {
            panic!()
        };
        assert!(
            (color[3] - 30.0 / 255.0).abs() < 1e-3,
            "flat alpha_base 30/255"
        );
    }
}
