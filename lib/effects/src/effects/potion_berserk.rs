use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{BodyTint, CameraShake, Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::potion_pillar::{BERSERK, PotionPillarEffect, PotionPillarParams};

pub const STR_OVERLAY: &str = "버서크";
pub const SPARK_TEXTURE: &str = "ac_center2.tga";
pub const TEXTURES: &[&str] = &[SPARK_TEXTURE, super::potion_pillar::TEXTURE];

const QUAKE_FRAME: f32 = 18.0;
const QUAKE_AMPLITUDE: f32 = 1.0;
const QUAKE_DURATION_MS: u32 = 350;
const SFX_PATH: &str = "effect\\ac_concentration.wav";

const BODY_TINT_RGB: [u8; 3] = [250, 120, 120];

const FRAMES_PER_SECOND: f32 = 60.0;

const PARENT_TOTAL_FRAMES: f32 = 151.0;
const SPARK_FIRST_FRAME: f32 = 12.0;
const SPARK_LAST_FRAME: f32 = 110.0;
const SPARK_INTERVAL_FRAMES: f32 = 4.0;
const SPARK_LIFETIME_FRAMES: f32 = 40.0;

const SPARK_WIDTH_MIN: f32 = 3.0; // (random(60)+30)/10 lower bound
const SPARK_WIDTH_MAX: f32 = 9.0; // (random(60)+30)/10 upper bound
const SPARK_HEIGHT: f32 = 0.2;
const SPARK_SPEED_MIN: f32 = 20.0 / 60.0;
const SPARK_SPEED_MAX: f32 = 70.0 / 60.0;
const SPARK_RADIAL_MIN: f32 = 2.0; // random(7)+2 lower bound
const SPARK_RADIAL_MAX: f32 = 9.0; // random(7)+2 upper bound
const SPARK_ALPHA_MAX: f32 = 180.0 / 255.0;
const SPARK_ALPHA_RAMP_FRAMES: f32 = 20.0;
const SPARK_FADE_OUT_FRAMES: f32 = 20.0;
const SPARK_COLOR_R: f32 = 240.0 / 255.0;

pub const TOTAL_DURATION_MS: u32 =
    ((PARENT_TOTAL_FRAMES + SPARK_LIFETIME_FRAMES) * 1000.0 / FRAMES_PER_SECOND) as u32;

#[derive(Clone, Copy)]
struct Rng(u32);

impl Rng {
    fn from_seed(seed: u32) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9).wrapping_add(1))
    }
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        let r = self.next_u32() as f32 / u32::MAX as f32;
        lo + (hi - lo) * r
    }
}

#[derive(Clone, Copy, Debug)]
struct Spark {
    spawn_frame: f32,
    origin: [f32; 3],
    /// Per-frame vertical velocity in native RO coords (negative = upward).
    speed_per_frame_y: f32,
    width: f32,
}

impl Spark {
    fn alive_at(&self, frame: f32) -> Option<f32> {
        let local = frame - self.spawn_frame;
        if local < 0.0 || local >= SPARK_LIFETIME_FRAMES {
            None
        } else {
            Some(local)
        }
    }

    fn position(&self, local: f32) -> [f32; 3] {
        [
            self.origin[0],
            self.origin[1] + self.speed_per_frame_y * local,
            self.origin[2],
        ]
    }

    fn alpha(&self, local: f32) -> f32 {
        let fade_start = SPARK_LIFETIME_FRAMES - SPARK_FADE_OUT_FRAMES;
        if local < SPARK_ALPHA_RAMP_FRAMES {
            SPARK_ALPHA_MAX * (local / SPARK_ALPHA_RAMP_FRAMES)
        } else if local < fade_start {
            SPARK_ALPHA_MAX
        } else {
            let span = SPARK_LIFETIME_FRAMES - fade_start;
            SPARK_ALPHA_MAX * ((SPARK_LIFETIME_FRAMES - local) / span).max(0.0)
        }
    }
}

pub struct PotionBerserkEffect {
    world_pos: [f32; 3],
    age: f32,
    pillar: PotionPillarEffect,
    sparks: Vec<Spark>,
    next_spark_frame: f32,
    quake_fired: bool,
    sfx_fired: bool,
    rng: Rng,
}

impl PotionBerserkEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
            pillar: PotionPillarEffect::new(world_pos, BERSERK as PotionPillarParams),
            sparks: Vec::new(),
            next_spark_frame: SPARK_FIRST_FRAME,
            quake_fired: false,
            sfx_fired: false,
            rng: Rng::from_seed((world_pos[0].to_bits() ^ world_pos[2].to_bits()) ^ 0xBE_15_E1_C0),
        }
    }

    fn spawn_one_spark(&mut self, frame: f32) {
        let yaw_rad = self.rng.range_f32(0.0, std::f32::consts::TAU);
        let radial = self.rng.range_f32(SPARK_RADIAL_MIN, SPARK_RADIAL_MAX);
        let (s, c) = yaw_rad.sin_cos();
        let origin = [
            self.world_pos[0] + radial * s,
            self.world_pos[1],
            self.world_pos[2] + radial * c,
        ];
        let speed = self.rng.range_f32(SPARK_SPEED_MIN, SPARK_SPEED_MAX);
        let width = self.rng.range_f32(SPARK_WIDTH_MIN, SPARK_WIDTH_MAX);
        self.sparks.push(Spark {
            spawn_frame: frame,
            origin,
            speed_per_frame_y: -speed,
            width,
        });
    }

    fn push_cross_texture(&self, out: &mut EffectDrawList, spark: &Spark, local: f32, alpha: f32) {
        let pos = spark.position(local);
        let h = SPARK_HEIGHT;
        let w = spark.width;
        let color = [SPARK_COLOR_R, 0.0, 0.0, alpha];
        let uv = [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]];
        let q1 = [
            [pos[0] - h, pos[1] - w, pos[2]],
            [pos[0] - h, pos[1] + w, pos[2]],
            [pos[0] + h, pos[1] - w, pos[2]],
            [pos[0] + h, pos[1] + w, pos[2]],
        ];
        out.push(EffectPrimitiveDraw::WorldQuad {
            corners: q1,
            uv,
            texture: SPARK_TEXTURE,
            color,
            blend: BlendKind::Alpha,
            no_depth: false,
        });

        let q2 = [
            [pos[0], pos[1] - w, pos[2] - h],
            [pos[0], pos[1] + w, pos[2] - h],
            [pos[0], pos[1] - w, pos[2] + h],
            [pos[0], pos[1] + w, pos[2] + h],
        ];
        out.push(EffectPrimitiveDraw::WorldQuad {
            corners: q2,
            uv,
            texture: SPARK_TEXTURE,
            color,
            blend: BlendKind::Alpha,
            no_depth: false,
        });
    }
}

impl Effect for PotionBerserkEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        self.pillar.update(ctx);

        let frame = self.age * FRAMES_PER_SECOND;

        while frame >= self.next_spark_frame && self.next_spark_frame <= SPARK_LAST_FRAME {
            let spawn_at = self.next_spark_frame;
            self.spawn_one_spark(spawn_at);
            self.next_spark_frame += SPARK_INTERVAL_FRAMES;
        }
        self.sparks.retain(|s| s.alive_at(frame).is_some());

        if frame >= PARENT_TOTAL_FRAMES + SPARK_LIFETIME_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        self.pillar.collect_draws(out, ctx);
        let frame = self.age * FRAMES_PER_SECOND;
        for s in &self.sparks {
            let Some(local) = s.alive_at(frame) else {
                continue;
            };
            let alpha = s.alpha(local);
            if alpha <= 0.0 {
                continue;
            }
            self.push_cross_texture(out, s, local, alpha);
        }
    }

    fn str_overlay(&self) -> Option<&'static str> {
        Some(STR_OVERLAY)
    }

    fn body_tint(&self) -> Option<BodyTint> {
        let frame = self.age * FRAMES_PER_SECOND;
        (frame < PARENT_TOTAL_FRAMES).then_some(BodyTint { rgb: BODY_TINT_RGB })
    }

    fn take_camera_shake(&mut self) -> Option<CameraShake> {
        let frame = self.age * FRAMES_PER_SECOND;
        (!self.quake_fired && frame >= QUAKE_FRAME).then(|| {
            self.quake_fired = true;
            CameraShake {
                amplitude: QUAKE_AMPLITUDE,
                duration_ms: QUAKE_DURATION_MS,
            }
        })
    }

    fn take_sfx_request(&mut self) -> Option<&'static str> {
        let frame = self.age * FRAMES_PER_SECOND;
        (!self.sfx_fired && frame >= QUAKE_FRAME).then(|| {
            self.sfx_fired = true;
            SFX_PATH
        })
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

    fn collect(e: &PotionBerserkEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &ctx());
        list.primitives
    }

    fn step(e: &mut PotionBerserkEffect, dt: f32) {
        e.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    #[test]
    fn pillar_visible_from_frame_zero_sparks_appear_at_frame_12() {
        let mut e = PotionBerserkEffect::new([0.0; 3]);
        step(&mut e, 1.0 / FRAMES_PER_SECOND);
        let prims0 = collect(&e);
        assert!(
            prims0
                .iter()
                .any(|p| matches!(p, EffectPrimitiveDraw::Cylinder { .. })),
            "pillar cylinder visible immediately"
        );
        assert!(
            !prims0
                .iter()
                .any(|p| matches!(p, EffectPrimitiveDraw::WorldQuad { .. })),
            "no sparks yet at frame 1"
        );

        step(&mut e, 12.0 / FRAMES_PER_SECOND);
        let prims12 = collect(&e);
        let quads = prims12
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::WorldQuad { .. }))
            .count();
        assert!(
            quads >= 2,
            "expected at least 2 spark quads at frame ~13, got {quads}"
        );
        assert!(quads.is_multiple_of(2), "cross-texture emits in pairs");
    }

    #[test]
    fn reports_berserk_str_overlay() {
        let e = PotionBerserkEffect::new([0.0; 3]);
        assert_eq!(e.str_overlay(), Some(STR_OVERLAY));
    }

    #[test]
    fn tints_body_then_quakes_and_plays_sfx_at_frame_eighteen() {
        let mut e = PotionBerserkEffect::new([0.0; 3]);
        step(&mut e, 1.0 / FRAMES_PER_SECOND);
        assert_eq!(e.body_tint(), Some(BodyTint { rgb: BODY_TINT_RGB }));
        assert!(e.take_camera_shake().is_none(), "no quake before frame 18");
        assert!(e.take_sfx_request().is_none(), "no sfx before frame 18");

        step(&mut e, 18.0 / FRAMES_PER_SECOND);
        assert!(e.take_camera_shake().is_some(), "quake fires at frame 18");
        assert_eq!(
            e.take_sfx_request(),
            Some(SFX_PATH),
            "wave fires at frame 18"
        );
        assert!(e.take_camera_shake().is_none(), "quake is one-shot");
        assert!(e.take_sfx_request().is_none(), "wave is one-shot");

        step(&mut e, 140.0 / FRAMES_PER_SECOND);
        assert_eq!(e.body_tint(), None, "tint cleared past frame 150");
    }

    #[test]
    fn dies_after_full_duration() {
        let mut e = PotionBerserkEffect::new([0.0; 3]);
        let total_s = TOTAL_DURATION_MS as f32 / 1000.0;
        let s = e.update(&EffectUpdateCtx {
            delta: total_s + 0.5,
            camera_target: None,
            caster_yaw: None,
        });
        assert!(matches!(s, EffectStatus::Dead));
    }
}
