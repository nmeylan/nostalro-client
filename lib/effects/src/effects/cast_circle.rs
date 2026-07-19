use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::spec::Attach;

const FRAMES_PER_SECOND: f32 = 60.0;
const TOTAL_FRAMES: f32 = 56.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const WORLD_SCALE: f32 = 0.75;
const COLUMN_HEIGHT_SCALE: f32 = 0.5;
const NUM_PETALS: usize = 3;
const PETAL_ALPHA_MAX: f32 = 180.0 / 255.0;
const COLUMN_ALPHA_MAX: f32 = 70.0 / 255.0;
const RING_ALPHA_MAX: f32 = 120.0 / 255.0;

const FADE_FRAMES: f32 = 8.0;

const COLUMN_SIDES: u32 = 12;
const COLUMN_UV_REPEAT: f32 = 3.0;
const COLUMN_GROWTH_FRAMES: f32 = 12.0;
const COLUMN_RISE_ANGLE_DEG: f32 = 89.0;

const RING_UV_REPEAT: f32 = 1.0;
const RING_TEXTURE: &str = "alpha_down.tga";

const PETAL_SIDES: u32 = 20;
const PETAL_UV_REPEAT: f32 = 1.0;
const PETAL_ARC_DEG: f32 = 315.0;
const PETAL_RISE_ANGLES_DEG: [f32; NUM_PETALS] = [70.0, 57.0, 45.0];
const PETAL_ROT_SPEED_DEG_PER_FRAME: f32 = 4.0;

#[derive(Clone, Copy, Debug)]
pub struct CastCircleParams {
    pub texture: &'static str,
    pub color_rgb: [f32; 3],
    pub column_max_height: f32,
    pub column_radius: f32,
    pub ring_radius: f32,
    pub ring_thickness: f32,
    pub petal_distances: [f32; NUM_PETALS],
    pub petal_heights: [f32; NUM_PETALS],
}

const fn spell_cast(texture: &'static str, r: f32, g: f32, b: f32) -> CastCircleParams {
    CastCircleParams {
        texture,
        color_rgb: [r, g, b],
        column_max_height: 250.0,
        column_radius: 4.0,
        ring_radius: 4.0,
        ring_thickness: 2.0,
        petal_distances: [4.5, 5.0, 5.5],
        petal_heights: [25.0, 22.0, 19.0],
    }
}

pub const YELLOW: CastCircleParams = spell_cast("ring_yellow.tga", 1.00, 1.00, 1.00);
pub const WATER: CastCircleParams = spell_cast("ring_blue.tga", 1.00, 1.00, 1.00);
pub const FIRE: CastCircleParams = spell_cast("ring_red.tga", 1.00, 1.00, 1.00);
pub const WIND: CastCircleParams = spell_cast("ring_white.tga", 1.00, 1.00, 1.00);
pub const EARTH: CastCircleParams = spell_cast("ring_yellow.tga", 1.00, 1.00, 1.00);
pub const HOLY: CastCircleParams = spell_cast("ring_white.tga", 1.00, 1.00, 1.00);
pub const POISON: CastCircleParams = spell_cast("ring_purple.tga", 1.00, 1.00, 1.00);
pub const RED: CastCircleParams = spell_cast("ring_red.tga", 1.00, 1.00, 1.00);
pub const WHITE: CastCircleParams = spell_cast("ring_white.tga", 1.00, 1.00, 1.00);
pub const N_BLUE: CastCircleParams = spell_cast("ring_blue.tga", 1.00, 1.00, 1.00);

pub const DARK: CastCircleParams = spell_cast("ring_black.tga", 1.00, 1.00, 1.00);
pub const FLAME: CastCircleParams = spell_cast("ring_jadu.tga", 1.00, 1.00, 1.00);
pub const EARTH_BROWN: CastCircleParams = spell_cast("ring_brown.tga", 1.00, 1.00, 1.00);

pub const TEXTURES: &[&str] = &[
    "ring_yellow.tga",
    "ring_blue.tga",
    "ring_red.tga",
    "ring_white.tga",
    "ring_purple.tga",
    "ring_black.tga",
    "ring_jadu.tga",
    "ring_brown.tga",
    "alpha_down.tga",
];

pub struct CastCircleEffect {
    params: CastCircleParams,
    world_pos: [f32; 3],
    age: f32,
    life_frames: f32,
}

impl CastCircleEffect {
    pub fn new(world_pos: [f32; 3], params: CastCircleParams) -> Self {
        Self {
            params,
            world_pos,
            age: 0.0,
            life_frames: TOTAL_FRAMES,
        }
    }

    pub fn with_life_ms(mut self, ms: Option<u32>) -> Self {
        if let Some(ms) = ms {
            self.life_frames = (ms as f32 / 1000.0 * FRAMES_PER_SECOND).max(1.0);
        }
        self
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }
}

fn fade(local_age: f32, local_life: f32, alpha_max: f32) -> f32 {
    if local_age < 0.0 || local_age > local_life {
        return 0.0;
    }
    let fade_in = (local_age / FADE_FRAMES).clamp(0.0, 1.0);
    let fade_out = if local_age <= local_life - FADE_FRAMES {
        1.0
    } else {
        ((local_life - local_age) / FADE_FRAMES).clamp(0.0, 1.0)
    };
    alpha_max * fade_in * fade_out
}

impl Effect for CastCircleEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        EffectStatus::Running
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [r, g, b] = self.params.color_rgb;
        let frame = self.frame();

        let col_alpha = fade(frame, self.life_frames, COLUMN_ALPHA_MAX);
        if col_alpha > 0.0 {
            let growth = (frame / COLUMN_GROWTH_FRAMES).clamp(0.0, 1.0);
            let col_rise_rad = COLUMN_RISE_ANGLE_DEG.to_radians();
            let (col_sin, col_cos) = col_rise_rad.sin_cos();
            let max_h = self.params.column_max_height * growth * COLUMN_HEIGHT_SCALE;
            let col_radius = self.params.column_radius * WORLD_SCALE;
            let height = col_sin * max_h;
            if height > 0.0 {
                out.push(EffectPrimitiveDraw::Frustum {
                    base_alpha: 1.0,
                    base: self.world_pos,
                    bottom_size: col_radius,
                    top_size: col_radius + col_cos * max_h,
                    height,
                    sides: COLUMN_SIDES,
                    arc_angle_deg: 360.0,
                    rotation: 0.0,
                    uv_repeat: COLUMN_UV_REPEAT,
                    uv_scroll: [0.0, 0.0],
                    wave_amplitude: 0.0,
                    wave_frequency: 1.0,
                    wave_phase: 0.0,
                    wave_mode: FrustumWaveMode::Sine,
                    tilt_x_rad: 0.0,
                    rotation_y_rad: 0.0,
                    cull_back: false,
                    texture: self.params.texture,
                    color: [r, g, b, col_alpha],
                    blend: BlendKind::Alpha,
                });
            }
        }

        let ring_alpha = fade(frame, self.life_frames, RING_ALPHA_MAX);
        if ring_alpha > 0.0 {
            out.push(EffectPrimitiveDraw::GroundDisc {
                center: self.world_pos,
                radius: self.params.ring_radius * WORLD_SCALE,
                thickness: self.params.ring_thickness * WORLD_SCALE,
                rotation: 0.0,
                arc_angle_deg: 360.0,
                uv_repeat: RING_UV_REPEAT,
                texture: RING_TEXTURE,
                color: [r, g, b, ring_alpha],
                blend: BlendKind::Alpha,
                no_depth: false,
                tilt_rad: 0.0,
                spin_rad: 0.0,
            });
        }

        let spin_rad = (frame * PETAL_ROT_SPEED_DEG_PER_FRAME).to_radians();
        let alpha = fade(frame, self.life_frames, PETAL_ALPHA_MAX);
        if alpha > 0.0 {
            for i in 0..NUM_PETALS {
                let rise_rad = PETAL_RISE_ANGLES_DEG[i].to_radians();
                let (sin_rise, cos_rise) = rise_rad.sin_cos();
                let max_h = self.params.petal_heights[i] * WORLD_SCALE;
                let distance = self.params.petal_distances[i] * WORLD_SCALE;
                let offset_rad = (i as f32) * std::f32::consts::FRAC_PI_2;
                out.push(EffectPrimitiveDraw::Frustum {
                    base_alpha: 1.0,
                    base: self.world_pos,
                    bottom_size: distance,
                    top_size: distance + cos_rise * max_h,
                    height: sin_rise * max_h,
                    sides: PETAL_SIDES,
                    arc_angle_deg: PETAL_ARC_DEG,
                    rotation: spin_rad + offset_rad,
                    uv_repeat: PETAL_UV_REPEAT,
                    uv_scroll: [0.0, 0.0],
                    wave_amplitude: 0.0,
                    wave_frequency: 1.0,
                    wave_phase: 0.0,
                    wave_mode: FrustumWaveMode::Sine,
                    tilt_x_rad: 0.0,
                    rotation_y_rad: 0.0,
                    cull_back: false,
                    texture: self.params.texture,
                    color: [r, g, b, alpha],
                    blend: BlendKind::Alpha,
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

    fn run_to(c: &mut CastCircleEffect, target_frame: f32) {
        let current = c.frame();
        let delta = (target_frame - current) / FRAMES_PER_SECOND;
        if delta > 0.0 {
            c.update(&EffectUpdateCtx {
                delta,
                camera_target: None,
                caster_yaw: None,
            });
        }
    }

    fn collect(c: &CastCircleEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_all_three_elements_at_peak() {
        let mut c = CastCircleEffect::new([0.0; 3], YELLOW);
        run_to(&mut c, 30.0);
        let prims = collect(&c);
        let columns = prims.iter().filter(|p| is_column(p)).count();
        let petals = prims.iter().filter(|p| is_petal(p)).count();
        let discs = prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { .. }))
            .count();
        assert_eq!(columns, 1);
        assert_eq!(petals, NUM_PETALS);
        assert_eq!(discs, 1);
    }

    fn is_column(p: &EffectPrimitiveDraw) -> bool {
        matches!(p, EffectPrimitiveDraw::Frustum { sides, .. } if *sides == COLUMN_SIDES)
    }

    fn is_petal(p: &EffectPrimitiveDraw) -> bool {
        matches!(p, EffectPrimitiveDraw::Frustum { sides, .. } if *sides == PETAL_SIDES)
    }

    #[test]
    fn flame_ring_centered_on_column_with_rotating_texture() {
        let caster = [10.0, 5.0, 20.0];
        let mut c = CastCircleEffect::new(caster, YELLOW);
        run_to(&mut c, 30.0);
        let snapshot = |c: &CastCircleEffect| -> Option<([f32; 3], f32)> {
            collect(c).into_iter().find_map(|p| match p {
                EffectPrimitiveDraw::Frustum {
                    base,
                    rotation,
                    sides,
                    ..
                } if sides == PETAL_SIDES => Some((base, rotation)),
                _ => None,
            })
        };
        let (base_early, rot_early) = snapshot(&c).expect("flame ring should be emitted");
        assert!(
            (base_early[0] - caster[0]).abs() < 1e-3,
            "petal X must equal caster X"
        );
        assert!(
            (base_early[2] - caster[2]).abs() < 1e-3,
            "petal Z must equal caster Z"
        );
        run_to(&mut c, 40.0);
        let (_, rot_later) = snapshot(&c).unwrap();
        assert!(
            (rot_later - rot_early).abs() > 1e-3,
            "flame ring rotation should advance over time ({} → {})",
            rot_early,
            rot_later
        );
    }

    #[test]
    fn column_grows_over_growth_window() {
        let mut c = CastCircleEffect::new([0.0; 3], YELLOW);
        let height_of_column = |c: &CastCircleEffect| -> f32 {
            collect(c)
                .into_iter()
                .find_map(|p| {
                    if is_column(&p) {
                        if let EffectPrimitiveDraw::Frustum { height, .. } = p {
                            Some(height)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .unwrap_or(0.0)
        };
        run_to(&mut c, 2.0);
        let h_early = height_of_column(&c);
        run_to(&mut c, COLUMN_GROWTH_FRAMES);
        let h_full = height_of_column(&c);
        assert!(
            h_full > h_early,
            "column should grow ({} → {})",
            h_early,
            h_full
        );
        let expected = COLUMN_RISE_ANGLE_DEG.to_radians().sin()
            * YELLOW.column_max_height
            * COLUMN_HEIGHT_SCALE;
        assert!(
            (h_full - expected).abs() < 1e-3,
            "column should reach full height by frame {}, got {} (expected {})",
            COLUMN_GROWTH_FRAMES,
            h_full,
            expected
        );
    }

    #[test]
    fn with_life_ms_keeps_the_ring_visible_for_the_whole_cast() {
        let frame = TOTAL_FRAMES + 34.0;
        let mut default = CastCircleEffect::new([0.0; 3], YELLOW);
        run_to(&mut default, frame);
        assert!(
            collect(&default).is_empty(),
            "default ring is gone past its 56-frame life"
        );
        let mut long = CastCircleEffect::new([0.0; 3], YELLOW).with_life_ms(Some(2000));
        run_to(&mut long, frame);
        assert!(
            !collect(&long).is_empty(),
            "stretched ring is still visible at frame {frame}"
        );
    }

    #[test]
    fn every_variant_has_a_real_texture() {
        for params in [
            YELLOW,
            WATER,
            FIRE,
            WIND,
            EARTH,
            HOLY,
            POISON,
            RED,
            WHITE,
            N_BLUE,
            DARK,
            FLAME,
            EARTH_BROWN,
        ] {
            assert!(!params.texture.is_empty());
            assert!(TEXTURES.contains(&params.texture));
        }
        assert!(TEXTURES.contains(&RING_TEXTURE));
    }

    #[test]
    fn never_self_terminates() {
        let mut c = CastCircleEffect::new([0.0; 3], YELLOW);
        for _ in 0..200 {
            assert_eq!(
                c.update(&EffectUpdateCtx {
                    delta: 0.1,
                    camera_target: None,
                    caster_yaw: None
                }),
                EffectStatus::Running
            );
        }
    }
}
