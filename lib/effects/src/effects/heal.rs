use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::particle_up::{ParticleUpEffect, ParticleUpParams};
use crate::radial_emitter::RADIAL_EMITTER_DIVISION;

const FRAMES_PER_SECOND: f32 = 60.0;
const DIVISION: usize = RADIAL_EMITTER_DIVISION;
const SEGMENTS: u32 = (RADIAL_EMITTER_DIVISION - 1) as u32;
const FULL_ARC_RAD: f32 = std::f32::consts::TAU;

const ALPHA_MAX: f32 = 200.0 / 255.0;
const FADE_IN_STEP: f32 = 5.0 / 255.0;
const FADE_IN_STEP_ENTRY2: f32 = 3.0 / 255.0;
const FADE_IN_STEP_PORTAL3: f32 = 2.0 / 255.0;
const FADE_OUT_STEP: f32 = 2.0 / 255.0;
const FADE_IN_FRAMES: u32 = 16;
const GROW_IN_FRAMES: u32 = 90;
const ENTRY2_PULSE_FRAMES: u32 = 45;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RiseLaw {
    Heal,
    Teleport2,
}

#[derive(Clone, Copy)]
pub struct SlotSeed {
    pub ec: u8,
    pub distance: f32,
    pub max_height: f32,
    pub rise_angle_deg: f32,
    pub rot_start_deg: f32,
    pub alpha_t: f32,
    pub alpha_init: f32,
    pub flag1: u8,
}

pub struct HealParams {
    pub texture: &'static str,
    pub tint_rgb: [f32; 3],
    pub blend: BlendKind,
    pub height_scale: f32,
    pub law: RiseLaw,
    pub slots: &'static [SlotSeed],
    pub duration_frames: f32,
    pub particle_up: Option<&'static ParticleUpParams>,
}

impl HealParams {
    pub const fn total_duration_ms(&self) -> u32 {
        (self.duration_frames / FRAMES_PER_SECOND * 1000.0) as u32
    }
}

fn sin_deg(deg: f32) -> f32 {
    deg.to_radians().sin()
}

fn ec_speed(ec: u8) -> f32 {
    match ec {
        0 => 4.0,
        1 => 3.0,
        _ => 2.0,
    }
}

#[derive(Clone, Copy)]
struct RingSlot {
    ec: u8,
    distance: f32,
    max_height: f32,
    rise_angle_deg: f32,
    rot_start_deg: f32,
    alpha: f32,
    alpha_t: f32,
    flag1: u8,
    process: u32,
    heights: [f32; DIVISION],
}

impl RingSlot {
    fn from_seed(seed: &SlotSeed) -> Self {
        Self {
            ec: seed.ec,
            distance: seed.distance,
            max_height: seed.max_height,
            rise_angle_deg: seed.rise_angle_deg,
            rot_start_deg: seed.rot_start_deg,
            alpha: seed.alpha_init,
            alpha_t: seed.alpha_t,
            flag1: seed.flag1,
            process: 0,
            heights: [0.0; DIVISION],
        }
    }

    fn tick_heal(&mut self) {
        self.process += 1;
        let p = self.process;
        let pf = p as f32;

        if self.flag1 == 2 {
            let inc = if self.ec < 2 {
                self.ec as f32 + 4.0
            } else {
                self.ec as f32 + 2.0
            };
            self.rot_start_deg = (self.rot_start_deg + inc).rem_euclid(360.0);
        } else if self.flag1 == 3 {
            self.rot_start_deg = (self.rot_start_deg + self.ec as f32 + 6.0).rem_euclid(360.0);
        }

        if pf >= self.alpha_t {
            self.alpha = (self.alpha - FADE_OUT_STEP).max(0.0);
        } else if p < FADE_IN_FRAMES {
            let step = match self.flag1 {
                2 => FADE_IN_STEP_ENTRY2,
                3 => FADE_IN_STEP_PORTAL3,
                _ => FADE_IN_STEP,
            };
            self.alpha = (self.alpha + step).min(ALPHA_MAX);
        }

        for i in 0..DIVISION {
            let h = if self.flag1 == 2 {
                if p <= ENTRY2_PULSE_FRAMES {
                    self.max_height * sin_deg(pf * 4.0)
                } else {
                    0.0
                }
            } else if self.flag1 == 3 {
                let mut h = self.max_height;
                if p <= GROW_IN_FRAMES {
                    h *= sin_deg(pf);
                }
                h
            } else {
                let sin_limit = (i as f32 * 34.0 + pf * ec_speed(self.ec)).rem_euclid(360.0);
                let mut h = self.max_height * 0.75 + self.max_height * 0.25 * sin_deg(sin_limit);
                if p <= GROW_IN_FRAMES {
                    h *= sin_deg(pf);
                }
                h
            };
            self.heights[i] = h.max(0.0);
        }
    }

    fn tick_teleport2(&mut self) {
        self.process += 1;
        let p = self.process;
        let pf = p as f32;
        let pr = pf * 2.0;

        self.rot_start_deg = (self.rot_start_deg + (5.0 - self.ec as f32)).rem_euclid(360.0);

        let cur_max = self.max_height * sin_deg(pr.min(180.0));
        let pr_spatial = if self.ec < 2 {
            (pf + self.ec as f32 * 90.0).rem_euclid(360.0)
        } else {
            (pf * 3.0 + self.ec as f32 * 90.0).rem_euclid(360.0)
        };
        let sin_pr = sin_deg(pr_spatial);
        for i in 0..DIVISION {
            let sin_limit = 90.0 + (i as f32 - 10.0) * 9.0;
            let h = cur_max + cur_max * sin_deg(sin_limit) * 0.3 * sin_pr;
            self.heights[i] = h.max(0.0);
        }

        if pr >= 180.0 {
            self.alpha = (self.alpha - 10.0 / 255.0).max(0.0);
        } else if pr >= 135.0 {
            self.alpha = (self.alpha - 1.0 / 255.0).max(0.0);
        }
    }
}

pub struct HealEffect {
    world_pos: [f32; 3],
    params: &'static HealParams,
    slots: Vec<RingSlot>,
    particles: Option<ParticleUpEffect>,
    age_frames: f32,
    last_frame: u32,
}

impl HealEffect {
    pub fn new(world_pos: [f32; 3], params: &'static HealParams) -> Self {
        let slots = params.slots.iter().map(RingSlot::from_seed).collect();
        let particles = params
            .particle_up
            .map(|p| ParticleUpEffect::new(world_pos, *p));
        Self {
            world_pos,
            params,
            slots,
            particles,
            age_frames: 0.0,
            last_frame: 0,
        }
    }
}

impl Effect for HealEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let target = self.age_frames as u32;
        while self.last_frame < target {
            for slot in &mut self.slots {
                match self.params.law {
                    RiseLaw::Heal => slot.tick_heal(),
                    RiseLaw::Teleport2 => slot.tick_teleport2(),
                }
            }
            self.last_frame += 1;
        }

        if let Some(particles) = &mut self.particles {
            particles.update(ctx);
        }

        if self.age_frames >= self.params.duration_frames {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
        if let Some(particles) = &mut self.particles {
            particles.set_position(pos);
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for slot in &self.slots {
            if slot.alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::RadialRing {
                center: self.world_pos,
                distance: slot.distance,
                rise_angle_rad: slot.rise_angle_deg.to_radians(),
                rot_start_rad: slot.rot_start_deg.to_radians(),
                full_arc_rad: FULL_ARC_RAD,
                segments: SEGMENTS,
                height_scale: self.params.height_scale,
                heights: slot.heights,
                texture: self.params.texture,
                color: [
                    self.params.tint_rgb[0],
                    self.params.tint_rgb[1],
                    self.params.tint_rgb[2],
                    slot.alpha,
                ],
                blend: self.params.blend,
            });
        }

        if let Some(particles) = &self.particles {
            particles.collect_draws(out, _ctx);
        }
    }
}

const BLUE: [f32; 3] = [100.0 / 255.0, 100.0 / 255.0, 1.0];
const WHITE: [f32; 3] = [1.0, 1.0, 1.0];

const ABSORBSPIRITS_SLOTS: &[SlotSeed] = &[
    SlotSeed {
        ec: 0,
        distance: 4.6,
        max_height: 60.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 0.0,
        alpha_t: 40.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 1,
        distance: 5.0,
        max_height: 60.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 180.0,
        alpha_t: 40.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 2,
        distance: 5.0,
        max_height: 18.0,
        rise_angle_deg: 50.0,
        rot_start_deg: 60.0,
        alpha_t: 40.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 3,
        distance: 5.2,
        max_height: 18.0,
        rise_angle_deg: 48.0,
        rot_start_deg: 240.0,
        alpha_t: 40.0,
        alpha_init: 0.0,
        flag1: 0,
    },
];
pub const ABSORBSPIRITS: HealParams = HealParams {
    texture: "ring_blue.tga",
    tint_rgb: BLUE,
    blend: BlendKind::Additive,
    height_scale: 0.18,
    law: RiseLaw::Heal,
    slots: ABSORBSPIRITS_SLOTS,
    duration_frames: 82.0,
    particle_up: None,
};

const EXIT2_SLOTS: &[SlotSeed] = &[
    SlotSeed {
        ec: 0,
        distance: 1.8,
        max_height: 180.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 270.0,
        alpha_t: 58.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 1,
        distance: 2.0,
        max_height: 70.0,
        rise_angle_deg: 88.0,
        rot_start_deg: 180.0,
        alpha_t: 60.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 2,
        distance: 2.2,
        max_height: 45.0,
        rise_angle_deg: 86.0,
        rot_start_deg: 90.0,
        alpha_t: 60.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 3,
        distance: 2.4,
        max_height: 20.0,
        rise_angle_deg: 84.0,
        rot_start_deg: 0.0,
        alpha_t: 60.0,
        alpha_init: 0.0,
        flag1: 0,
    },
];
pub const EXIT2: HealParams = HealParams {
    texture: "ring_purple.tga",
    tint_rgb: BLUE,
    blend: BlendKind::Additive,
    height_scale: 0.15,
    law: RiseLaw::Heal,
    slots: EXIT2_SLOTS,
    duration_frames: 100.0,
    particle_up: None,
};

const ENTRY2_SLOTS: &[SlotSeed] = &[
    SlotSeed {
        ec: 0,
        distance: 3.7,
        max_height: 30.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 0.0,
        alpha_t: 1400.0,
        alpha_init: 0.0,
        flag1: 2,
    },
    SlotSeed {
        ec: 1,
        distance: 3.4,
        max_height: 30.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 90.0,
        alpha_t: 1400.0,
        alpha_init: 0.0,
        flag1: 2,
    },
    SlotSeed {
        ec: 2,
        distance: 3.6,
        max_height: 4.0,
        rise_angle_deg: 10.0,
        rot_start_deg: 180.0,
        alpha_t: 1400.0,
        alpha_init: 0.0,
        flag1: 2,
    },
    SlotSeed {
        ec: 3,
        distance: 3.7,
        max_height: 4.0,
        rise_angle_deg: 5.0,
        rot_start_deg: 270.0,
        alpha_t: 1400.0,
        alpha_init: 0.0,
        flag1: 2,
    },
];
pub const ENTRY2: HealParams = HealParams {
    texture: "ring_blue.tga",
    tint_rgb: BLUE,
    blend: BlendKind::Additive,
    height_scale: 0.3,
    law: RiseLaw::Heal,
    slots: ENTRY2_SLOTS,
    duration_frames: 48.0,
    particle_up: None,
};

const SMDEF_SLOTS: &[SlotSeed] = &[
    SlotSeed {
        ec: 0,
        distance: 4.5,
        max_height: 50.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 0.0,
        alpha_t: 30.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 1,
        distance: 4.7,
        max_height: 50.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 180.0,
        alpha_t: 30.0,
        alpha_init: 0.0,
        flag1: 0,
    },
];
pub const SMDEF: HealParams = HealParams {
    texture: "alpha_down.tga",
    tint_rgb: WHITE,
    blend: BlendKind::Additive,
    height_scale: 0.2,
    law: RiseLaw::Heal,
    slots: SMDEF_SLOTS,
    duration_frames: 72.0,
    particle_up: None,
};

const TELEPORTATION2_SLOTS: &[SlotSeed] = &[
    SlotSeed {
        ec: 0,
        distance: 1.5,
        max_height: 100.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 0.0,
        alpha_t: 0.0,
        alpha_init: 100.0 / 255.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 1,
        distance: 3.0,
        max_height: 70.0,
        rise_angle_deg: 89.0,
        rot_start_deg: 90.0,
        alpha_t: 0.0,
        alpha_init: 80.0 / 255.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 2,
        distance: 4.0,
        max_height: 40.0,
        rise_angle_deg: 88.0,
        rot_start_deg: 180.0,
        alpha_t: 0.0,
        alpha_init: 60.0 / 255.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 3,
        distance: 5.0,
        max_height: 15.0,
        rise_angle_deg: 87.0,
        rot_start_deg: 270.0,
        alpha_t: 0.0,
        alpha_init: 40.0 / 255.0,
        flag1: 0,
    },
];
pub const TELEPORTATION2: HealParams = HealParams {
    texture: "Magic_Violet.tga",
    tint_rgb: BLUE,
    blend: BlendKind::Additive,
    height_scale: 0.4,
    law: RiseLaw::Teleport2,
    slots: TELEPORTATION2_SLOTS,
    duration_frames: 105.0,
    particle_up: None,
};

const GREEN: [f32; 3] = [76.0 / 255.0, 230.0 / 255.0, 90.0 / 255.0];

const HEAL_SLOTS: &[SlotSeed] = &[
    SlotSeed {
        ec: 0,
        distance: 4.6,
        max_height: 40.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 0.0,
        alpha_t: 60.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 1,
        distance: 4.8,
        max_height: 40.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 180.0,
        alpha_t: 60.0,
        alpha_init: 0.0,
        flag1: 0,
    },
];
pub const HEAL: HealParams = HealParams {
    texture: "alpha_down.tga",
    tint_rgb: GREEN,
    blend: BlendKind::Additive,
    height_scale: 0.45,
    law: RiseLaw::Heal,
    slots: HEAL_SLOTS,
    duration_frames: 102.0,
    particle_up: Some(&crate::effects::particle_up::HEAL_MOTE),
};

const HEAL2_SLOTS: &[SlotSeed] = &[
    SlotSeed {
        ec: 0,
        distance: 4.6,
        max_height: 60.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 0.0,
        alpha_t: 60.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 1,
        distance: 5.0,
        max_height: 60.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 180.0,
        alpha_t: 60.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 2,
        distance: 5.0,
        max_height: 12.0,
        rise_angle_deg: 50.0,
        rot_start_deg: 60.0,
        alpha_t: 60.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 3,
        distance: 5.2,
        max_height: 12.0,
        rise_angle_deg: 48.0,
        rot_start_deg: 240.0,
        alpha_t: 60.0,
        alpha_init: 0.0,
        flag1: 0,
    },
];
pub const HEAL2: HealParams = HealParams {
    texture: "ring_white.tga",
    tint_rgb: GREEN,
    blend: BlendKind::Additive,
    height_scale: 0.55,
    law: RiseLaw::Heal,
    slots: HEAL2_SLOTS,
    duration_frames: 102.0,
    particle_up: Some(&crate::effects::particle_up::HEAL_MOTE),
};

const HEAL4_SLOTS: &[SlotSeed] = &[
    SlotSeed {
        ec: 0,
        distance: 4.6,
        max_height: 60.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 0.0,
        alpha_t: 40.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 1,
        distance: 5.0,
        max_height: 60.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 180.0,
        alpha_t: 40.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 2,
        distance: 5.0,
        max_height: 18.0,
        rise_angle_deg: 50.0,
        rot_start_deg: 60.0,
        alpha_t: 40.0,
        alpha_init: 0.0,
        flag1: 0,
    },
    SlotSeed {
        ec: 3,
        distance: 5.2,
        max_height: 18.0,
        rise_angle_deg: 48.0,
        rot_start_deg: 240.0,
        alpha_t: 40.0,
        alpha_init: 0.0,
        flag1: 0,
    },
];
pub const HEAL4: HealParams = HealParams {
    texture: "ring_white.tga",
    tint_rgb: GREEN,
    blend: BlendKind::Additive,
    height_scale: 0.55,
    law: RiseLaw::Heal,
    slots: HEAL4_SLOTS,
    duration_frames: 82.0,
    particle_up: Some(&crate::effects::particle_up::HEAL_MOTE),
};

const BIGPORTAL_VIOLET: [f32; 3] = [170.0 / 255.0, 120.0 / 255.0, 1.0];
const BIGPORTAL_SLOTS: &[SlotSeed] = &[
    SlotSeed {
        ec: 0,
        distance: 7.0,
        max_height: 80.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 0.0,
        alpha_t: 1400.0,
        alpha_init: 0.0,
        flag1: 3,
    },
    SlotSeed {
        ec: 1,
        distance: 5.5,
        max_height: 80.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 120.0,
        alpha_t: 1400.0,
        alpha_init: 0.0,
        flag1: 3,
    },
    SlotSeed {
        ec: 2,
        distance: 4.0,
        max_height: 80.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 240.0,
        alpha_t: 1400.0,
        alpha_init: 0.0,
        flag1: 3,
    },
];
pub const BIGPORTAL: HealParams = HealParams {
    texture: "Magic_Violet.tga",
    tint_rgb: BIGPORTAL_VIOLET,
    blend: BlendKind::Additive,
    height_scale: 0.6,
    law: RiseLaw::Heal,
    slots: BIGPORTAL_SLOTS,
    duration_frames: 1200.0,
    particle_up: None,
};

const BIGPORTAL2_SLOTS: &[SlotSeed] = &[
    SlotSeed {
        ec: 0,
        distance: 7.0,
        max_height: 80.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 0.0,
        alpha_t: 1.0e9,
        alpha_init: 0.0,
        flag1: 3,
    },
    SlotSeed {
        ec: 1,
        distance: 5.5,
        max_height: 80.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 120.0,
        alpha_t: 1.0e9,
        alpha_init: 0.0,
        flag1: 3,
    },
    SlotSeed {
        ec: 2,
        distance: 4.0,
        max_height: 80.0,
        rise_angle_deg: 90.0,
        rot_start_deg: 240.0,
        alpha_t: 1.0e9,
        alpha_init: 0.0,
        flag1: 3,
    },
];
pub const BIGPORTAL2: HealParams = HealParams {
    texture: "Magic_Violet.tga",
    tint_rgb: BIGPORTAL_VIOLET,
    blend: BlendKind::Additive,
    height_scale: 0.6,
    law: RiseLaw::Heal,
    slots: BIGPORTAL2_SLOTS,
    duration_frames: 5999.0,
    particle_up: None,
};

pub const TEXTURES: &[&str] = &[
    "ring_blue.tga",
    "ring_purple.tga",
    "ring_white.tga",
    "alpha_down.tga",
    "Magic_Violet.tga",
];

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

    fn step(e: &mut HealEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn rings(e: &HealEffect) -> Vec<(f32, f32, [f32; DIVISION], BlendKind)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::RadialRing {
                    distance,
                    color,
                    heights,
                    blend,
                    ..
                } => Some((*distance, color[3], *heights, *blend)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn absorbspirits_emits_four_blue_additive_rings_that_grow_in() {
        let mut e = HealEffect::new([1.0, 0.0, -2.0], &ABSORBSPIRITS);
        step(&mut e, 3.0);
        let early = rings(&e);
        assert_eq!(early.len(), 4, "four rings");
        let radii: Vec<f32> = early.iter().map(|r| r.0).collect();
        assert_eq!(radii, vec![4.6, 5.0, 5.0, 5.2]);
        for r in &early {
            assert_eq!(r.3, BlendKind::Additive);
            assert!(r.1 > 0.0, "alpha ramped off zero");
        }
        let h_early = early[0].2[5];

        step(&mut e, 25.0);
        let mid = rings(&e);
        let h_mid = mid[0].2[5];
        assert!(h_mid > h_early, "height grows in over the first frames");
    }

    #[test]
    fn entry2_pulses_height_up_then_back_to_zero() {
        let mut e = HealEffect::new([0.0; 3], &ENTRY2);
        step(&mut e, 11.0); // process*4 ≈ 44° — climbing
        let climbing = rings(&e)[0].2[0];
        step(&mut e, 12.0); // process ≈ 23 → process*4 ≈ 90° (peak)
        let peak = rings(&e)[0].2[0];
        assert!(peak > climbing, "height pulses up toward the peak");

        step(&mut e, 25.0); // process ≈ 48 > 45 → collapsed
        let after = rings(&e);
        if let Some(r) = after.first() {
            assert!(r.2[0] <= 1e-4, "height collapses to zero after frame 45");
        }
    }

    #[test]
    fn teleport2_swells_then_dies() {
        let mut e = HealEffect::new([0.0; 3], &TELEPORTATION2);
        step(&mut e, 10.0);
        let early = rings(&e)[0].2[0];
        step(&mut e, 35.0); // ~frame 45 → process*2 = 90° peak
        let peak = rings(&e)[0].2[0];
        assert!(peak > early, "height swells toward the peak");

        let status = step(&mut e, 70.0);
        assert!(matches!(status, EffectStatus::Dead), "dies by duration");
    }
}
