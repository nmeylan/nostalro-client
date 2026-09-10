use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::radial_emitter::RADIAL_EMITTER_DIVISION;

const FRAMES_PER_SECOND: f32 = 60.0;
const MIN_FRAMES: f32 = 70.0;
pub const TOTAL_DURATION_MS: u32 = (MIN_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const DIVISION: usize = RADIAL_EMITTER_DIVISION;
const SEGMENTS: u32 = (RADIAL_EMITTER_DIVISION - 1) as u32;
const NUM_GI: usize = 4;
const PILLAR: usize = 3;

const LOBE_STEP_DEG: f32 = 9.0;
const GROW_FRAMES: i32 = 90;
const FADE_IN_FRAMES: i32 = 20;
const FADE_OUT_LEAD: f32 = 40.0;
const SKIRT_ALPHA_STEP: f32 = 10.0;
const SKIRT_ALPHA_CAP: f32 = 180.0;
const SKIRT_FADE_OUT: f32 = 5.0;
const PILLAR_ALPHA_STEP: f32 = 8.0;
const PILLAR_ALPHA_CAP: f32 = 60.0;
const PILLAR_ALPHA_CAP_BREATHING: f32 = 100.0;
const PILLAR_FADE_OUT: f32 = 2.0;
const PILLAR_FADE_OUT_BREATHING: f32 = 3.0;
const BREATH_BASE: f32 = 0.55;
const BREATH_SWING: f32 = 0.45;
const SIZE3_ALPHA: f32 = 80.0;
/// The skirt renders at the reference's own units; the pillar is the one
/// element whose literal does not survive the trip to our world scale.
const PILLAR_SCALE: f32 = 0.75;

/// Vertex `i` stays down until `process > i * REVEAL_FRAMES_PER_VERTEX`, so the
/// blade unrolls from `rot_start` instead of rising everywhere at once. The
/// reference capture widens from a sliver to full width over ~25 ticks across
/// the 21 vertices.
const REVEAL_FRAMES_PER_VERTEX: i32 = 2;

struct GiSeed {
    arc_deg: f32,
    max_height: f32,
    distance: f32,
    rise_deg: f32,
    rot_start_deg: f32,
    alpha_b: f32,
}

const GI_SEEDS: [GiSeed; NUM_GI] = [
    GiSeed { arc_deg: 315.0, max_height: 25.0, distance: 4.5, rise_deg: 70.0, rot_start_deg: 0.0, alpha_b: 180.0 },
    GiSeed { arc_deg: 315.0, max_height: 22.0, distance: 5.0, rise_deg: 57.0, rot_start_deg: 90.0, alpha_b: 180.0 },
    GiSeed { arc_deg: 315.0, max_height: 19.0, distance: 5.5, rise_deg: 45.0, rot_start_deg: 180.0, alpha_b: 180.0 },
    GiSeed { arc_deg: 360.0, max_height: 250.0, distance: 4.0, rise_deg: 89.0, rot_start_deg: 0.0, alpha_b: 70.0 },
];

#[derive(Clone, Copy, Debug)]
pub struct CastCircleParams {
    pub texture: &'static str,
    /// `BeginCasting`'s `option`. 33 selects the breathing-pillar variant; 3
    /// draws the pillar alone.
    pub option: u8,
}

const fn casting(texture: &'static str, option: u8) -> CastCircleParams {
    CastCircleParams { texture, option }
}

pub const BEGINSPELL2: CastCircleParams = casting("ring_blue.tga", 2);
pub const BEGINSPELL3: CastCircleParams = casting("ring_yellow.tga", 1);
pub const BEGINSPELL4: CastCircleParams = casting("Magic_Green.tga", 33);
pub const BEGINSPELL5: CastCircleParams = casting("ring_yellow.tga", 0);
pub const BEGINSPELL7: CastCircleParams = casting("Magic_Violet.tga", 33);
pub const BEGINSPELL8: CastCircleParams = casting("ring_green.tga", 1);

pub const CHANGE_FIRE: CastCircleParams = casting("ring_red.tga", 3);
pub const CHANGE_COLD: CastCircleParams = casting("ring_blue.tga", 3);
pub const CHANGE_DARK: CastCircleParams = casting("ring_black.tga", 3);
pub const CHANGE_WIND: CastCircleParams = casting("ring_yellow.tga", 3);
pub const CHANGE_FLAME: CastCircleParams = casting("ring_jadu.tga", 3);
pub const CHANGE_EARTH: CastCircleParams = casting("ring_brown.tga", 3);
pub const CHANGE_HOLY: CastCircleParams = casting("ring_white.tga", 3);
pub const CHANGE_POISON: CastCircleParams = casting("ring_purple.tga", 3);

pub const TEXTURES: &[&str] = &[
    "ring_yellow.tga",
    "ring_blue.tga",
    "ring_red.tga",
    "ring_white.tga",
    "ring_purple.tga",
    "ring_black.tga",
    "ring_jadu.tga",
    "ring_brown.tga",
    "ring_green.tga",
    "Magic_Green.tga",
    "Magic_Violet.tga",
];

fn sin_deg(deg: f32) -> f32 {
    deg.to_radians().sin()
}

struct Appearance {
    rgb: [f32; 3],
    fixed_alpha: Option<f32>,
    blend: BlendKind,
}

fn appearance(size: u8) -> Appearance {
    match size {
        1 => Appearance {
            rgb: [1.0, 175.0 / 255.0, 175.0 / 255.0],
            fixed_alpha: None,
            blend: BlendKind::Additive,
        },
        2 => Appearance {
            rgb: [195.0 / 255.0, 195.0 / 255.0, 1.0],
            fixed_alpha: None,
            blend: BlendKind::Alpha,
        },
        3 => Appearance {
            rgb: [1.0, 1.0, 1.0],
            fixed_alpha: Some(SIZE3_ALPHA),
            blend: BlendKind::Alpha,
        },
        _ => Appearance {
            rgb: [1.0, 1.0, 1.0],
            fixed_alpha: None,
            blend: BlendKind::Alpha,
        },
    }
}

#[derive(Clone, Copy)]
struct Gi {
    arc_deg: f32,
    max_height: f32,
    distance: f32,
    rise_deg: f32,
    rot_start_deg: f32,
    alpha_b: f32,
    process: i32,
    heights: [f32; DIVISION],
    frozen: [bool; DIVISION],
}

impl Gi {
    fn from_seed(seed: &GiSeed) -> Self {
        Self {
            arc_deg: seed.arc_deg,
            max_height: seed.max_height,
            distance: seed.distance,
            rise_deg: seed.rise_deg,
            rot_start_deg: seed.rot_start_deg,
            alpha_b: seed.alpha_b,
            process: 0,
            heights: [0.0; DIVISION],
            frozen: [false; DIVISION],
        }
    }

    fn ceiling(&self, i: usize) -> f32 {
        self.max_height * sin_deg(i as f32 * LOBE_STEP_DEG)
    }

    fn step(&mut self, ec: usize, duration_frames: f32, breathing: bool) {
        self.process += 1;
        if ec < PILLAR {
            self.rot_start_deg = (self.rot_start_deg + ec as f32 + 3.0).rem_euclid(360.0);
        }

        let p = self.process as f32;
        if p >= duration_frames - FADE_OUT_LEAD {
            let step = if ec == PILLAR {
                if breathing {
                    PILLAR_FADE_OUT_BREATHING
                } else {
                    PILLAR_FADE_OUT
                }
            } else {
                SKIRT_FADE_OUT
            };
            self.alpha_b = (self.alpha_b - step).max(0.0);
        } else if self.process < FADE_IN_FRAMES {
            if ec == PILLAR {
                let cap = if breathing {
                    PILLAR_ALPHA_CAP_BREATHING
                } else {
                    PILLAR_ALPHA_CAP
                };
                self.alpha_b = (self.alpha_b + PILLAR_ALPHA_STEP).min(cap);
            } else {
                self.alpha_b = (self.alpha_b + SKIRT_ALPHA_STEP).min(SKIRT_ALPHA_CAP);
            }
        }

        if breathing && ec == PILLAR {
            self.step_breathing_heights();
        } else {
            self.step_revealed_heights();
        }
    }

    fn step_revealed_heights(&mut self) {
        for i in 0..DIVISION {
            if self.process <= i as i32 * REVEAL_FRAMES_PER_VERTEX {
                continue;
            }
            let ceiling = self.ceiling(i);
            if self.process <= GROW_FRAMES {
                self.heights[i] = ceiling * sin_deg(self.process as f32);
            }
            if self.heights[i] > ceiling {
                self.heights[i] = ceiling;
            }
            if self.heights[i] < 0.0 {
                self.heights[i] = 0.0;
            }
        }
    }

    fn step_breathing_heights(&mut self) {
        for i in 0..DIVISION {
            let ceiling = self.ceiling(i);
            if self.frozen[i] {
                let phase = (self.process % 360) as f32;
                self.heights[i] = ceiling * BREATH_BASE + ceiling * BREATH_SWING * sin_deg(phase);
                continue;
            }
            if self.process <= GROW_FRAMES {
                self.heights[i] = ceiling * sin_deg(self.process as f32);
            } else {
                self.frozen[i] = true;
            }
            if self.heights[i] > ceiling {
                self.frozen[i] = true;
            }
            if self.heights[i] < 0.0 {
                self.heights[i] = 0.0;
            }
        }
    }
}

pub struct CastCircleEffect {
    params: CastCircleParams,
    world_pos: [f32; 3],
    gis: Vec<Gi>,
    age: f32,
    life_frames: f32,
}

impl CastCircleEffect {
    pub fn new(world_pos: [f32; 3], params: CastCircleParams) -> Self {
        let first = if params.option == 3 { PILLAR } else { 0 };
        Self {
            params,
            world_pos,
            gis: GI_SEEDS[first..].iter().map(Gi::from_seed).collect(),
            age: 0.0,
            life_frames: MIN_FRAMES,
        }
    }

    pub fn with_life_ms(mut self, ms: Option<u32>) -> Self {
        if let Some(ms) = ms {
            self.life_frames = (ms as f32 / 1000.0 * FRAMES_PER_SECOND).max(MIN_FRAMES);
        }
        self
    }

    fn breathing(&self) -> bool {
        self.params.option == 33
    }

    fn first_ec(&self) -> usize {
        if self.params.option == 3 { PILLAR } else { 0 }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }
}

impl Effect for CastCircleEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let before = self.frame();
        self.age += ctx.delta;
        let after = self.frame();
        let steps = (after.floor() - before.floor()).max(0.0) as i32;
        let life = self.life_frames;
        let breathing = self.breathing();
        let first = self.first_ec();
        for _ in 0..steps {
            for (n, gi) in self.gis.iter_mut().enumerate() {
                gi.step(first + n, life, breathing);
            }
        }
        if after >= self.life_frames {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.frame() > self.life_frames {
            return;
        }
        let look = appearance(if self.params.option == 33 {
            0
        } else {
            self.params.option
        });
        for gi in &self.gis {
            let alpha = look.fixed_alpha.unwrap_or(gi.alpha_b) / 255.0;
            if alpha <= 0.0 {
                continue;
            }
            let scale = if gi.arc_deg >= 360.0 {
                PILLAR_SCALE
            } else {
                1.0
            };
            out.push(EffectPrimitiveDraw::RadialRing {
                center: self.world_pos,
                distance: gi.distance * scale,
                rise_angle_rad: gi.rise_deg.to_radians(),
                rot_start_rad: gi.rot_start_deg.to_radians(),
                full_arc_rad: gi.arc_deg.to_radians(),
                segments: SEGMENTS,
                height_scale: scale,
                heights: gi.heights,
                texture: self.params.texture,
                color: [look.rgb[0], look.rgb[1], look.rgb[2], alpha],
                blend: look.blend,
            });
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

    fn step_frames(c: &mut CastCircleEffect, n: u32) -> EffectStatus {
        let mut status = EffectStatus::Running;
        for _ in 0..n {
            status = c.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
        }
        status
    }

    fn rings(c: &CastCircleEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn pillar(c: &CastCircleEffect) -> ([f32; DIVISION], f32, f32) {
        rings(c)
            .iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::RadialRing {
                    heights,
                    full_arc_rad,
                    color,
                    ..
                } if (*full_arc_rad - std::f32::consts::TAU).abs() < 1e-4 => {
                    Some((*heights, *full_arc_rad, color[3]))
                }
                _ => None,
            })
            .expect("pillar ring")
    }

    #[test]
    fn pillar_unrolls_one_vertex_at_a_time_from_the_seam() {
        let mut c = CastCircleEffect::new([0.0; 3], BEGINSPELL3);
        step_frames(&mut c, 9);
        let (heights, _, _) = pillar(&c);
        // Vertex i wakes at frame i*2, so at frame 9 vertices 1..4 are up
        // (0 and 20 sit on sin(0)/sin(180) and are always flat).
        assert!(heights[1] > 0.0 && heights[4] > 0.0, "leading edge is up");
        assert_eq!(heights[5], 0.0, "vertex 5 has not woken yet");
        assert_eq!(heights[10], 0.0, "far side is still down");

        step_frames(&mut c, 8);
        let (later, _, _) = pillar(&c);
        assert!(later[8] > 0.0, "the edge advanced by frame 17");
        assert_eq!(later[10], 0.0, "and has not reached the far side yet");
    }

    #[test]
    fn four_rings_with_the_reference_geometry() {
        let mut c = CastCircleEffect::new([0.0; 3], BEGINSPELL5).with_life_ms(Some(3000));
        step_frames(&mut c, 12);
        let prims = rings(&c);
        assert_eq!(prims.len(), NUM_GI);
        let arcs: Vec<f32> = prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::RadialRing { full_arc_rad, .. } => Some(*full_arc_rad),
                _ => None,
            })
            .collect();
        let skirts = arcs
            .iter()
            .filter(|a| (**a - 315.0_f32.to_radians()).abs() < 1e-4)
            .count();
        assert_eq!(skirts, 3, "three 315° skirt arcs");
        step_frames(&mut c, 78);
        let (heights, _, _) = pillar(&c);
        let tallest = heights.iter().cloned().fold(0.0_f32, f32::max);
        assert!(
            tallest > 200.0,
            "the revealed pillar towers over the 25-unit skirt: {tallest}"
        );
        assert!(heights[12] > 0.0, "the whole ring is awake by frame 90");
    }

    #[test]
    fn option_three_draws_the_pillar_alone_at_fixed_alpha() {
        let mut c = CastCircleEffect::new([0.0; 3], CHANGE_FIRE);
        step_frames(&mut c, 12);
        let prims = rings(&c);
        assert_eq!(prims.len(), 1, "m_size 3 skips the skirt");
        let (_, arc, alpha) = pillar(&c);
        assert!((arc - std::f32::consts::TAU).abs() < 1e-4);
        assert!((alpha - SIZE3_ALPHA / 255.0).abs() < 1e-4);
    }

    #[test]
    fn variants_carry_their_own_texture_tint_and_blend() {
        let cases = [
            (BEGINSPELL2, "ring_blue.tga", BlendKind::Alpha, 195.0 / 255.0),
            (BEGINSPELL3, "ring_yellow.tga", BlendKind::Additive, 1.0),
            (BEGINSPELL8, "ring_green.tga", BlendKind::Additive, 1.0),
            (BEGINSPELL4, "Magic_Green.tga", BlendKind::Alpha, 1.0),
        ];
        for (params, texture, blend, red) in cases {
            let mut c = CastCircleEffect::new([0.0; 3], params);
            step_frames(&mut c, 12);
            let p = rings(&c);
            let (t, b, col) = p
                .iter()
                .find_map(|p| match p {
                    EffectPrimitiveDraw::RadialRing {
                        texture,
                        blend,
                        color,
                        ..
                    } => Some((*texture, *blend, *color)),
                    _ => None,
                })
                .expect("a ring");
            assert_eq!(t, texture);
            assert_eq!(b, blend);
            assert!((col[0] - red).abs() < 1e-3, "{texture}: red {}", col[0]);
        }
    }

    #[test]
    fn breathing_pillar_pulses_once_it_tops_out() {
        let mut c = CastCircleEffect::new([0.0; 3], BEGINSPELL4).with_life_ms(Some(4000));
        step_frames(&mut c, 95);
        let (a, _, _) = pillar(&c);
        step_frames(&mut c, 45);
        let (b, _, _) = pillar(&c);
        assert!(
            (a[10] - b[10]).abs() > 1.0,
            "the topped-out pillar breathes: {} -> {}",
            a[10],
            b[10]
        );
    }
}
