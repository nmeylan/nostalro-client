use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::radial_emitter::RADIAL_EMITTER_DIVISION;

const FRAMES_PER_SECOND: f32 = 60.0;
const TOTAL_FRAMES: f32 = 56.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const DIVISION: usize = RADIAL_EMITTER_DIVISION;
const SEGMENTS: u32 = (RADIAL_EMITTER_DIVISION - 1) as u32;
const FULL_ARC_RAD: f32 = std::f32::consts::TAU;

const INIT_DISTANCE: f32 = 4.1;
const INIT_RISE_DEG: f32 = 80.0;
const DISTANCE_GROW_PER_FRAME: f32 = 0.07;
const RISE_SHRINK_PER_FRAME: f32 = 1.0;
const RISE_FLOOR_DEG: f32 = 10.0;
const RESET_DISTANCE: f32 = 3.37;
const ALPHA_DRAIN_PER_FRAME: f32 = 3.0;
const ALPHA_REFILL_DISTANCE_GATE: f32 = 4.0;
const RESET_PROCESS_MARGIN: i32 = 30;
const PROCESS_STAGGER: i32 = 5;
/// Above this the emitters start together with `alpha_b` seeded from the pass
/// time instead of cascading in from zero.
const STAGGER_MAX_FRAMES: f32 = 60.0;
const SEED_ALPHA_STEP: f32 = 45.0;

pub const NUM_EMITTERS: usize = 4;
/// Per-emitter starting azimuths (degrees).
const ROT_START_DEG: [f32; NUM_EMITTERS] = [180.0, 270.0, 0.0, 90.0];
/// Two passes fire at spawn. The `time` argument only seeds alpha on the
/// long-duration path; on our short path it's overwritten to 0, so these
/// values just count the passes and pick per-pass textures.
pub const PASS_TIMES: [f32; 2] = [45.0, 25.0];

const WAVE_REL_AMPLITUDE: f32 = 0.3;
const HEIGHT_LOBE_STEP_DEG: f32 = 9.0;
const WAVE_PHASE_PER_FRAME_DEG: [f32; NUM_EMITTERS] = [1.0, 1.0, 2.0, 2.0];

#[derive(Clone, Copy)]
pub struct SaintCastingConfig {
    pub texture: &'static str,
    pub pass_textures: Option<[&'static str; 2]>,
    pub max_heights: [f32; NUM_EMITTERS],
    pub color_rgb: [f32; 3],
    pub blend: BlendKind,
    pub refill_per_frame: f32,
    pub reset_rise_deg: f32,
}

#[derive(Clone, Copy)]
struct Emitter {
    distance: f32,
    rise_deg: f32,
    alpha: f32,
    rot_start_deg: f32,
    max_height: f32,
    process: i32,
    wave_rate_deg: f32,
    wave_base_deg: f32,
    heights: [f32; DIVISION],
    texture: &'static str,
}

impl Emitter {
    fn step(&mut self, refill_per_frame: f32, reset_rise_deg: f32, reset_process_limit: i32) {
        self.process += 1;
        if self.process <= 0 {
            return;
        }
        self.distance += DISTANCE_GROW_PER_FRAME;
        let next_rise = self.rise_deg - RISE_SHRINK_PER_FRAME;
        if next_rise < RISE_FLOOR_DEG {
            self.rise_deg = RISE_FLOOR_DEG;
            self.alpha = 0.0;
        } else {
            self.rise_deg = next_rise;
        }

        if self.distance >= ALPHA_REFILL_DISTANCE_GATE {
            self.alpha -= ALPHA_DRAIN_PER_FRAME;
            if self.alpha <= 0.0 {
                self.alpha = 0.0;
                if self.process < reset_process_limit {
                    self.distance = RESET_DISTANCE;
                    self.rise_deg = reset_rise_deg;
                }
            }
        } else {
            self.alpha += refill_per_frame;
        }

        let wave = WAVE_REL_AMPLITUDE * self.max_height * self.wave_phase_rad().sin();
        for (i, h) in self.heights.iter_mut().enumerate() {
            let lobe = (i as f32 * HEIGHT_LOBE_STEP_DEG).to_radians().sin();
            *h = self.max_height + wave * lobe;
        }
    }

    fn wave_phase_rad(&self) -> f32 {
        (self.process.max(0) as f32 * self.wave_rate_deg + self.wave_base_deg).to_radians()
    }

    fn alpha_unit(&self, blend: BlendKind, refill_per_frame: f32) -> f32 {
        // 8 emitters all rendering at the same world position with additive
        // blending saturate our framebuffer to white at the centre, erasing
        // the ring texture's striped flame-tongue pattern. Pre-attenuate so
        // the additive sum at peak stays below 1.0 and the texture detail
        // survives.
        //
        // An emitter refills over ~8 frames before the distance gate, so its
        // peak `alpha ≈ refill_per_frame · 8`. Scaling the divisor by the
        // refill rate keeps every variant's per-emitter peak at the same
        // visible brightness (≈0.16) regardless of whether it refills at +10
        // (begin-spell family) or +5 (Aura Blade) — otherwise Aura Blade, at
        // half the alpha, washes out to nothing. Alpha-blended variants
        // (DarkCasting's dark dome) can't saturate — full strength so the
        // stack genuinely darkens the scene.
        let overdraw_divisor: f32 = match blend {
            BlendKind::Additive => refill_per_frame / 5.0,
            _ => 1.0,
        };
        (self.alpha / (255.0 * overdraw_divisor)).clamp(0.0, 1.0)
    }
}

fn seed_emitters(cfg: &SaintCastingConfig, life_frames: f32) -> Vec<Emitter> {
    let cascade = life_frames <= STAGGER_MAX_FRAMES;
    let mut emitters = Vec::with_capacity(PASS_TIMES.len() * NUM_EMITTERS);
    for (pass_idx, pass_time) in PASS_TIMES.iter().enumerate() {
        for ec in 0..NUM_EMITTERS {
            let texture = cfg
                .pass_textures
                .map(|t| t[pass_idx])
                .unwrap_or(cfg.texture);
            let (alpha, process) = if cascade {
                (0.0, -(ec as i32) * PROCESS_STAGGER)
            } else {
                (
                    pass_time + (NUM_EMITTERS - 1 - ec) as f32 * SEED_ALPHA_STEP,
                    0,
                )
            };
            emitters.push(Emitter {
                distance: INIT_DISTANCE,
                rise_deg: INIT_RISE_DEG,
                alpha,
                rot_start_deg: ROT_START_DEG[ec],
                max_height: cfg.max_heights[ec],
                process,
                wave_rate_deg: WAVE_PHASE_PER_FRAME_DEG[ec],
                wave_base_deg: ec as f32 * 90.0,
                heights: [0.0; DIVISION],
                texture,
            });
        }
    }
    emitters
}

pub struct SaintCastingEffect {
    world_pos: [f32; 3],
    age: f32,
    emitters: Vec<Emitter>,
    cfg: SaintCastingConfig,
    life_frames: f32,
}

impl SaintCastingEffect {
    pub fn new(world_pos: [f32; 3], cfg: SaintCastingConfig) -> Self {
        Self {
            world_pos,
            age: 0.0,
            emitters: seed_emitters(&cfg, TOTAL_FRAMES),
            cfg,
            life_frames: TOTAL_FRAMES,
        }
    }

    pub fn with_life_ms(mut self, ms: Option<u32>) -> Self {
        if let Some(ms) = ms {
            self.life_frames = (ms as f32 / 1000.0 * FRAMES_PER_SECOND).max(TOTAL_FRAMES);
            self.emitters = seed_emitters(&self.cfg, self.life_frames);
        }
        self
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }
}

impl Effect for SaintCastingEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frame_before = self.frame();
        self.age += ctx.delta;
        let frame_after = self.frame();
        let steps = (frame_after.floor() - frame_before.floor()).max(0.0) as i32;
        let margin = (RESET_PROCESS_MARGIN as f32 * (self.life_frames / TOTAL_FRAMES))
            .min(RESET_PROCESS_MARGIN as f32);
        let reset_limit = self.life_frames as i32 - margin as i32;
        for _ in 0..steps {
            for em in &mut self.emitters {
                em.step(
                    self.cfg.refill_per_frame,
                    self.cfg.reset_rise_deg,
                    reset_limit,
                );
            }
        }
        if frame_after >= self.life_frames {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.frame();
        if frame > self.life_frames {
            return;
        }
        for em in &self.emitters {
            let alpha = em.alpha_unit(self.cfg.blend, self.cfg.refill_per_frame);
            if alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::RadialRing {
                center: self.world_pos,
                distance: em.distance,
                rise_angle_rad: em.rise_deg.to_radians(),
                rot_start_rad: em.rot_start_deg.to_radians(),
                full_arc_rad: FULL_ARC_RAD,
                segments: SEGMENTS,
                height_scale: 1.0,
                heights: em.heights,
                texture: em.texture,
                color: [
                    self.cfg.color_rgb[0],
                    self.cfg.color_rgb[1],
                    self.cfg.color_rgb[2],
                    alpha,
                ],
                blend: self.cfg.blend,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CONFIG: SaintCastingConfig = SaintCastingConfig {
        texture: "ring_test.tga",
        pass_textures: None,
        max_heights: [17.0, 18.0, 19.0, 20.0],
        color_rgb: [1.0, 1.0, 1.0],
        blend: BlendKind::Additive,
        refill_per_frame: 10.0,
        reset_rise_deg: 74.0,
    };

    struct Ring {
        distance: f32,
        rise_rad: f32,
        rot_start_rad: f32,
        arc_rad: f32,
        segments: u32,
        heights: [f32; DIVISION],
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn rings(e: &SaintCastingEffect) -> Vec<Ring> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::RadialRing {
                    distance,
                    rise_angle_rad,
                    rot_start_rad,
                    full_arc_rad,
                    segments,
                    heights,
                    ..
                } => Some(Ring {
                    distance: *distance,
                    rise_rad: *rise_angle_rad,
                    rot_start_rad: *rot_start_rad,
                    arc_rad: *full_arc_rad,
                    segments: *segments,
                    heights: *heights,
                }),
                _ => None,
            })
            .collect()
    }

    fn step_frames(e: &mut SaintCastingEffect, n: u32) -> EffectStatus {
        let mut status = EffectStatus::Running;
        for _ in 0..n {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
        }
        status
    }

    #[test]
    fn cascade_brings_the_emitters_up_one_at_a_time() {
        let mut e = SaintCastingEffect::new([0.0; 3], TEST_CONFIG);
        assert!(
            rings(&e).is_empty(),
            "everything fades in — frame 0 is empty"
        );
        step_frames(&mut e, 4);
        let early = rings(&e).len();
        assert!(
            early > 0 && early < 8,
            "only the lead emitters are up: {early}"
        );
        step_frames(&mut e, 14);
        assert_eq!(rings(&e).len(), 8, "two passes × 4 emitters by frame 18");
    }

    #[test]
    fn every_ring_closes_and_stands_full_height_all_the_way_round() {
        let mut e = SaintCastingEffect::new([0.0; 3], TEST_CONFIG);
        let starts: Vec<f32> = ROT_START_DEG.iter().map(|d| d.to_radians()).collect();
        for _ in 0..(TOTAL_FRAMES as u32) {
            for r in rings(&e) {
                assert_eq!(r.segments, SEGMENTS);
                assert!((r.arc_rad - FULL_ARC_RAD).abs() < 1e-5, "closed ring");
                assert!(
                    starts.iter().any(|s| (s - r.rot_start_rad).abs() < 1e-5),
                    "rot start {} must stay on its initial azimuth",
                    r.rot_start_rad
                );
                let tallest = r.heights.iter().cloned().fold(0.0_f32, f32::max);
                for (i, h) in r.heights.iter().enumerate() {
                    assert!(
                        *h >= tallest * 0.7,
                        "height[{i}] = {h} dips below the ±30% wave around {tallest}"
                    );
                }
            }
            step_frames(&mut e, 1);
        }
    }

    #[test]
    fn each_pulse_pushes_the_ring_out_and_flattens_it() {
        let mut e = SaintCastingEffect::new([0.0; 3], TEST_CONFIG);
        step_frames(&mut e, 4);
        let early = rings(&e);
        step_frames(&mut e, 12);
        let late = rings(&e);
        assert!(
            late[0].distance > early[0].distance,
            "ring widens ({} → {})",
            early[0].distance,
            late[0].distance
        );
        assert!(
            late[0].rise_rad < early[0].rise_rad,
            "ring flattens ({} → {})",
            early[0].rise_rad,
            late[0].rise_rad
        );
    }

    #[test]
    fn the_aura_lasts_as_long_as_the_cast() {
        let mut default = SaintCastingEffect::new([0.0; 3], TEST_CONFIG);
        for f in 0..(TOTAL_FRAMES as u32) {
            assert_eq!(
                step_frames(&mut default, 1),
                EffectStatus::Running,
                "still alive at frame {f}"
            );
        }
        assert_eq!(step_frames(&mut default, 1), EffectStatus::Dead);

        // A 2s cast (120 frames) keeps the aura re-pulsing well past the
        // default 56-frame lifetime, then it ends near the cast's end.
        let mut long = SaintCastingEffect::new([0.0; 3], TEST_CONFIG).with_life_ms(Some(2000));
        let past_default = TOTAL_FRAMES as u32 + 24;
        assert_eq!(step_frames(&mut long, past_default), EffectStatus::Running);
        assert!(!rings(&long).is_empty(), "emitters keep re-pulsing");
        assert_eq!(
            step_frames(&mut long, 120 - past_default + 1),
            EffectStatus::Dead
        );

        let mut short = SaintCastingEffect::new([0.0; 3], TEST_CONFIG).with_life_ms(Some(280));
        let mut emitted = false;
        for _ in 0..17 {
            step_frames(&mut short, 1);
            if !rings(&short).is_empty() {
                emitted = true;
                break;
            }
        }
        assert!(emitted, "a short cast still gets its aura");
    }
}
