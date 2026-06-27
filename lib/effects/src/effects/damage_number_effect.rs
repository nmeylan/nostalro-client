use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx, NumberRequest};

const FPS: f32 = 60.0;
const DURATION_FRAMES: f32 = 0.5 * FPS;

#[derive(Clone, Copy)]
pub struct NumberParams {
    pub color: [f32; 3],
}

pub const DAMAGE1: NumberParams = NumberParams {
    color: [1.0, 0.0, 0.0],
};
pub const DAMAGE1_3: NumberParams = NumberParams {
    color: [1.0, 100.0 / 255.0, 1.0],
};
pub const GREEN_NUMBER: NumberParams = NumberParams {
    color: [0.0, 1.0, 0.0],
};
pub const BLUE_NUMBER: NumberParams = NumberParams {
    color: [64.0 / 255.0, 124.0 / 255.0, 1.0],
};
pub const RED_NUMBER: NumberParams = NumberParams {
    color: [1.0, 0.0, 0.0],
};
pub const PURPLE_NUMBER: NumberParams = NumberParams {
    color: [1.0, 50.0 / 255.0, 1.0],
};
pub const BLACK_NUMBER: NumberParams = NumberParams {
    color: [0.0, 0.0, 0.0],
};
pub const WHITE_NUMBER: NumberParams = NumberParams {
    color: [1.0, 1.0, 1.0],
};
pub const YELLOW_NUMBER: NumberParams = NumberParams {
    color: [1.0, 1.0, 0.0],
};
pub const PINK_NUMBER: NumberParams = NumberParams {
    color: [1.0, 85.0 / 255.0, 177.0 / 255.0],
};

const NUMBER_VALUE: i32 = 1;

pub struct DamageNumberEffect {
    color: [f32; 3],
    age_frames: f32,
    request_pending: bool,
}

impl DamageNumberEffect {
    pub fn new(params: NumberParams) -> Self {
        Self {
            color: params.color,
            age_frames: 0.0,
            request_pending: true,
        }
    }
}

impl Effect for DamageNumberEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn take_number_request(&mut self) -> Option<NumberRequest> {
        if self.request_pending {
            self.request_pending = false;
            Some(NumberRequest {
                value: NUMBER_VALUE,
                color: self.color,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut DamageNumberEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        })
    }

    #[test]
    fn emits_one_number_request_then_none() {
        let mut e = DamageNumberEffect::new(DAMAGE1);
        let req = e
            .take_number_request()
            .expect("first call yields a request");
        assert_eq!(req.value, 1);
        assert_eq!(req.color, [1.0, 0.0, 0.0]);
        assert!(e.take_number_request().is_none(), "request is one-shot");
    }

    #[test]
    fn purple_variant_carries_its_colour() {
        let mut e = DamageNumberEffect::new(DAMAGE1_3);
        let req = e.take_number_request().unwrap();
        assert_eq!(req.color, [1.0, 100.0 / 255.0, 1.0]);
    }

    #[test]
    fn dies_after_duration() {
        let mut e = DamageNumberEffect::new(DAMAGE1);
        assert_eq!(step(&mut e, 1.0), EffectStatus::Running);
        assert_eq!(step(&mut e, DURATION_FRAMES), EffectStatus::Dead);
    }
}
