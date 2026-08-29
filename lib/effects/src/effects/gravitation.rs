use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{CameraShake, Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::spike_util::{FRAMES_PER_SECOND, apex_velocity};

const STONE: &str = "stone.bmp";
const ICE: &str = "ice.tga";
pub const TEXTURES: &[&str] = &[STONE, ICE];

const FRAME_DT: f32 = 1.0 / FRAMES_PER_SECOND;

/// One effect is spawned per cell of the field, so a level-5 field is 25 of
/// these; four shards each is already a dense wall.
const SPIKE_COUNT: usize = 4;
const HEIGHT: f32 = 18.0;
const BURY_DEPTH: f32 = 10.0;
const ALPHA: f32 = 20.0 / 255.0;

const SPEED_INIT: f32 = 1.0;
const ACCEL_INIT: f32 = 0.01;
const RETRACT_SPEED: f32 = -1.2;
const EXTEND_SPEED: f32 = 1.18;
const EXTEND_ACCEL: f32 = 0.01;
/// Three frames out, three frames back: the shards buzz in place.
const VIBRATION_PERIOD: u32 = 6;
/// Past this the shards stop buzzing and sink for good.
const SETTLE_FRAME: u32 = 550;

const QUAKE_AMPLITUDE: f32 = 1.0;
const QUAKE_DURATION_MS: u32 = 200;

struct Rng(u32);
impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next_u32() as f32 / u32::MAX as f32)
    }
}

struct Spike {
    base: [f32; 3],
    axis: [f32; 3],
    size: f32,
    latitude_deg: f32,
    longitude_deg: f32,
    texture: &'static str,
    distance: f32,
    speed: f32,
    accel: f32,
}

impl Spike {
    fn step(&mut self, frame: u32) {
        if frame >= SETTLE_FRAME {
            if frame == SETTLE_FRAME {
                self.speed = RETRACT_SPEED;
                self.accel = 0.0;
            }
        } else if frame % VIBRATION_PERIOD == 2 {
            self.speed = RETRACT_SPEED;
            self.accel = 0.0;
        } else if frame % VIBRATION_PERIOD == 5 {
            self.speed = EXTEND_SPEED;
            self.accel = EXTEND_ACCEL;
        }
        self.speed += self.accel;
        self.distance += self.speed;
    }

    fn position(&self) -> [f32; 3] {
        [
            self.base[0] + self.axis[0] * self.distance,
            self.base[1] + self.axis[1] * self.distance,
            self.base[2] + self.axis[2] * self.distance,
        ]
    }
}

pub struct GravitationEffect {
    spikes: Vec<Spike>,
    frame: u32,
    time_accum: f32,
    shake_fired: bool,
}

impl GravitationEffect {
    pub fn new(anchor: [f32; 3]) -> Self {
        let [ax, ay, az] = anchor;
        let seed = ax.to_bits() ^ az.to_bits() ^ 0x6BA1_7A70;
        let mut rng = Rng(seed | 1);
        let spikes = (0..SPIKE_COUNT)
            .map(|i| {
                let size = rng.range(3.0, 3.5);
                let longitude = rng.range(0.0, 360.0);
                let latitude = rng.range(60.0, 100.0);
                Spike {
                    base: [ax, ay + BURY_DEPTH, az],
                    axis: apex_velocity(latitude, longitude, 1.0),
                    size,
                    latitude_deg: latitude,
                    longitude_deg: longitude,
                    texture: if i < SPIKE_COUNT / 2 { STONE } else { ICE },
                    distance: 0.0,
                    speed: SPEED_INIT,
                    accel: ACCEL_INIT,
                }
            })
            .collect();
        Self {
            spikes,
            frame: 0,
            time_accum: 0.0,
            shake_fired: false,
        }
    }
}

impl Effect for GravitationEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.time_accum += ctx.delta;
        while self.time_accum >= FRAME_DT {
            self.time_accum -= FRAME_DT;
            for s in &mut self.spikes {
                s.step(self.frame);
            }
            self.frame += 1;
        }
        // The field lives as long as its skill unit, which despawns the effect.
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for s in &self.spikes {
            out.push(EffectPrimitiveDraw::QuadHorn {
                base: s.position(),
                size: s.size,
                height: HEIGHT,
                tilt_x_deg: s.latitude_deg,
                rotation_y_deg: s.longitude_deg,
                texture: s.texture,
                color: [1.0, 1.0, 1.0, ALPHA],
                blend: BlendKind::Alpha,
            });
        }
    }

    fn take_camera_shake(&mut self) -> Option<CameraShake> {
        if !self.shake_fired {
            self.shake_fired = true;
            Some(CameraShake {
                amplitude: QUAKE_AMPLITUDE,
                duration_ms: QUAKE_DURATION_MS,
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
            screen_w: 256.0,
            screen_h: 256.0,
            elapsed: 0.0,
        }
    }

    fn step(e: &mut GravitationEffect, frames: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..frames {
            s = e.update(&EffectUpdateCtx {
                delta: FRAME_DT,
                camera_target: None,
                caster_yaw: None,
            });
        }
        s
    }

    fn horns(e: &GravitationEffect) -> Vec<([f32; 3], f32, &'static str)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::QuadHorn {
                    base,
                    color,
                    texture,
                    blend: BlendKind::Alpha,
                    ..
                } => (*base, color[3], *texture),
                other => panic!("expected alpha QuadHorn, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn four_faint_shards_buried_at_the_cell_centre() {
        let e = GravitationEffect::new([0.0, 0.0, 0.0]);
        let h = horns(&e);
        assert_eq!(h.len(), SPIKE_COUNT);
        assert!(h.iter().any(|(_, _, t)| *t == STONE));
        assert!(h.iter().any(|(_, _, t)| *t == ICE));
        for (base, alpha, _) in &h {
            assert!((alpha - ALPHA).abs() < 1e-6, "barely visible: {alpha}");
            assert!(base[0] == 0.0 && base[2] == 0.0, "no radial scatter");
            assert!(base[1] > 0.0, "starts below the ground: {}", base[1]);
        }
    }

    #[test]
    fn shards_buzz_in_place_and_outlive_the_field() {
        let mut e = GravitationEffect::new([0.0; 3]);
        step(&mut e, 2);
        let out = horns(&e)[0].0[1];
        step(&mut e, 3);
        let back = horns(&e)[0].0[1];
        assert!(back > out, "retracts on the second half of the cycle");

        // Net travel over whole cycles stays near zero — it vibrates, it does
        // not creep away.
        step(&mut e, 295);
        let settled = horns(&e)[0].0[1];
        assert!(
            (settled - BURY_DEPTH).abs() < 4.0,
            "still around its buried origin: {settled}"
        );
        assert_eq!(step(&mut e, 1), EffectStatus::Running, "never self-expires");
    }

    #[test]
    fn fires_one_short_camera_shake() {
        let mut e = GravitationEffect::new([0.0; 3]);
        let shake = e.take_camera_shake().expect("shake fires at spawn");
        assert_eq!(shake.duration_ms, QUAKE_DURATION_MS);
        assert!(e.take_camera_shake().is_none(), "shake fires only once");
    }
}
