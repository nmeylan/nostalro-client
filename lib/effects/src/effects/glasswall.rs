use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const WALL_TEXTURE: &str = "ring_blue.tga";
pub const TEXTURES: &[&str] = &[WALL_TEXTURE];

pub const STR_OVERLAY: &str = "safetywall";

const FRAMES_PER_SECOND: f32 = 60.0;

const WALL_OFFSET_Z: f32 = 2.6;
const WALL_OFFSET_X: f32 = 3.0;
const WALL_FRONT_BACK_HALF_WIDTH: f32 = WALL_OFFSET_X;
const WALL_LEFT_RIGHT_HALF_WIDTH: f32 = WALL_OFFSET_Z;
const WALL_HEIGHT: f32 = 20.0;

const WALL_MAX_ALPHA: f32 = 180.0 / 255.0;
const WALL_FADE_IN_FRAMES: f32 = 6.0;
const WALL_UV_SCROLL_PER_FRAME: f32 = 1.0 / 60.0;

pub const TOTAL_DURATION_MS: u32 = 99990;

fn wall_alpha(frame: f32) -> f32 {
    (frame / WALL_FADE_IN_FRAMES).clamp(0.0, 1.0) * WALL_MAX_ALPHA
}

pub struct GlasswallEffect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl GlasswallEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age_frames: 0.0,
        }
    }
}

fn wall_quad(centre: [f32; 3], half_along_x: f32, half_along_z: f32, height: f32) -> [[f32; 3]; 4] {
    let bx0 = centre[0] - half_along_x;
    let bz0 = centre[2] - half_along_z;
    let bx1 = centre[0] + half_along_x;
    let bz1 = centre[2] + half_along_z;
    let top_y = centre[1] - height;
    let bot_y = centre[1];
    [
        [bx0, top_y, bz0],
        [bx1, top_y, bz1],
        [bx1, bot_y, bz1],
        [bx0, bot_y, bz0],
    ]
}

impl Effect for GlasswallEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = wall_alpha(self.age_frames);
        if alpha <= 0.0 {
            return;
        }
        let scroll = WALL_UV_SCROLL_PER_FRAME * self.age_frames;
        let uv = [
            [0.0 + scroll, 0.0],
            [1.0 + scroll, 0.0],
            [1.0 + scroll, 1.0],
            [0.0 + scroll, 1.0],
        ];
        let colour = [0.5, 0.7, 1.0, alpha];

        for side in [1.0, -1.0] {
            let centre = [
                self.world_pos[0],
                self.world_pos[1],
                self.world_pos[2] + WALL_OFFSET_Z * side,
            ];
            out.push(EffectPrimitiveDraw::WorldQuad {
                corners: wall_quad(centre, WALL_FRONT_BACK_HALF_WIDTH, 0.0, WALL_HEIGHT),
                uv,
                texture: WALL_TEXTURE,
                color: colour,
                blend: BlendKind::Alpha,
                no_depth: false,
            });
        }

        for side in [1.0, -1.0] {
            let centre = [
                self.world_pos[0] + WALL_OFFSET_X * side,
                self.world_pos[1],
                self.world_pos[2],
            ];
            out.push(EffectPrimitiveDraw::WorldQuad {
                corners: wall_quad(centre, 0.0, WALL_LEFT_RIGHT_HALF_WIDTH, WALL_HEIGHT),
                uv,
                texture: WALL_TEXTURE,
                color: colour,
                blend: BlendKind::Alpha,
                no_depth: false,
            });
        }
    }

    fn str_overlay(&self) -> Option<&'static str> {
        Some(STR_OVERLAY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn emits_four_walls_forming_a_box() {
        let mut e = GlasswallEffect::new([10.0, 0.0, 20.0]);
        e.update(&ctx(10.0 / FRAMES_PER_SECOND));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let walls: Vec<_> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::WorldQuad {
                    corners, texture, ..
                } => Some((*corners, *texture)),
                _ => None,
            })
            .collect();
        assert_eq!(walls.len(), 4, "exactly 4 walls forming the box");
        for (_, tex) in &walls {
            assert_eq!(*tex, WALL_TEXTURE);
        }

        let mut centres: Vec<[f32; 3]> = walls
            .iter()
            .map(|(c, _)| {
                let mut sum = [0.0; 3];
                for v in c {
                    sum[0] += v[0];
                    sum[1] += v[1];
                    sum[2] += v[2];
                }
                [sum[0] / 4.0, sum[1] / 4.0, sum[2] / 4.0]
            })
            .collect();
        centres.sort_by(|a, b| a[2].partial_cmp(&b[2]).unwrap_or(std::cmp::Ordering::Equal));
        let z_min = centres[0][2];
        let z_max = centres[3][2];
        assert!((z_min - (20.0 - WALL_OFFSET_Z)).abs() < 1e-3);
        assert!((z_max - (20.0 + WALL_OFFSET_Z)).abs() < 1e-3);
    }

    #[test]
    fn declares_safetywall_str_overlay() {
        let e = GlasswallEffect::new([0.0; 3]);
        assert_eq!(e.str_overlay(), Some(STR_OVERLAY));
    }

    #[test]
    fn alpha_fades_in_from_zero() {
        let mut e = GlasswallEffect::new([0.0; 3]);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert!(list.primitives.is_empty(), "no walls at frame 0");

        e.update(&ctx(WALL_FADE_IN_FRAMES / FRAMES_PER_SECOND));
        let mut list2 = EffectDrawList::new();
        e.collect_draws(&mut list2, &render_ctx());
        let peak_alpha = list2
            .primitives
            .iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::WorldQuad { color, .. } => Some(color[3]),
                _ => None,
            })
            .unwrap();
        assert!((peak_alpha - WALL_MAX_ALPHA).abs() < 1e-3);
    }
}
