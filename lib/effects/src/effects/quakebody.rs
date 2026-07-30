use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{BodyTint, Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub total_frames: f32,
    /// Inclusive frame window during which the body shakes.
    pub shake_start: f32,
    pub shake_end: f32,
    /// Peak jitter in screen pixels.
    pub amplitude: f32,
    pub red_tint: bool,
}

pub const QUAKEBODY: Params = Params {
    total_frames: 14.0,
    shake_start: 0.0,
    shake_end: 14.0,
    amplitude: 3.0,
    red_tint: false,
};
pub const QUAKEBODY2: Params = Params {
    total_frames: 35.0,
    shake_start: 0.0,
    shake_end: 35.0,
    amplitude: 3.0,
    red_tint: false,
};
pub const QUAKEBODY3: Params = Params {
    total_frames: 60.0,
    shake_start: 30.0,
    shake_end: 50.0,
    amplitude: 3.5,
    red_tint: false,
};
pub const QUAKEBODY4: Params = Params {
    total_frames: 60.0,
    shake_start: 20.0,
    shake_end: 60.0,
    amplitude: 4.0,
    red_tint: true,
};

pub const fn total_duration_ms(p: &Params) -> u32 {
    (p.total_frames / FPS * 1000.0) as u32
}

fn jitter(frame: u32, salt: u32) -> f32 {
    let x = frame
        .wrapping_mul(2_654_435_761)
        .wrapping_add(salt.wrapping_mul(40_503))
        .wrapping_add(0x9E37_79B9);
    let x = x ^ (x >> 15);
    ((x % 100_000) as f32 / 100_000.0) * 2.0 - 1.0
}

pub struct QuakeBodyEffect {
    params: Params,
    process: f32,
}

impl QuakeBodyEffect {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            process: 0.0,
        }
    }

    fn shaking(&self) -> bool {
        self.process >= self.params.shake_start && self.process < self.params.shake_end
    }
}

impl Effect for QuakeBodyEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.process += ctx.delta * FPS;
        if self.process >= self.params.total_frames {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn body_edge_jitter(&self) -> Option<[f32; 4]> {
        if !self.shaking() {
            return None;
        }
        let frame = self.process.floor() as u32;
        let a = self.params.amplitude;
        Some([
            jitter(frame, 1) * a,
            jitter(frame, 2) * a,
            jitter(frame, 3) * a,
            jitter(frame, 4) * a,
        ])
    }

    fn body_tint(&self) -> Option<BodyTint> {
        if self.params.red_tint && self.shaking() {
            let on = (self.process.floor() as u32) % 2 == 0;
            Some(BodyTint {
                rgb: if on { [250, 50, 50] } else { [255, 255, 255] },
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut QuakeBodyEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        })
    }

    #[test]
    fn quakebody3_jitters_each_edge_independently_only_in_its_window() {
        let mut e = QuakeBodyEffect::new(QUAKEBODY3);
        step(&mut e, 10.0);
        assert!(e.body_edge_jitter().is_none(), "idle before frame 30");
        step(&mut e, 25.0);
        let j = e.body_edge_jitter().expect("shaking inside window");
        assert!(j.iter().all(|v| v.abs() <= QUAKEBODY3.amplitude));
        assert!(j[0] != j[1] || j[2] != j[3], "opposite edges move apart");
        step(&mut e, 15.0);
        assert!(e.body_edge_jitter().is_none(), "idle after frame 50");
    }

    #[test]
    fn quakebody4_alternates_red_tint_while_shaking_others_dont() {
        let mut e = QuakeBodyEffect::new(QUAKEBODY4);
        step(&mut e, 25.0);
        assert!(e.body_edge_jitter().is_some());
        let t0 = e.body_tint().expect("Quakebody4 tints while shaking");
        step(&mut e, 1.0);
        let t1 = e.body_tint().unwrap();
        assert_ne!(t0, t1, "tint alternates frame to frame");

        let mut plain = QuakeBodyEffect::new(QUAKEBODY);
        step(&mut plain, 5.0);
        assert!(plain.body_tint().is_none());
    }

    #[test]
    fn dies_after_total_frames_and_emits_no_primitives() {
        let mut e = QuakeBodyEffect::new(QUAKEBODY);
        let mut list = EffectDrawList::new();
        e.collect_draws(
            &mut list,
            &EffectRenderCtx {
                camera: Default::default(),
                screen_w: 800.0,
                screen_h: 600.0,
                elapsed: 0.0,
            },
        );
        assert!(list.primitives.is_empty(), "body shake draws nothing");
        assert_eq!(
            step(&mut e, QUAKEBODY.total_frames + 1.0),
            EffectStatus::Dead
        );
    }
}
