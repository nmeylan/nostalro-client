use super::draw::{EffectDrawList, EffectStatus};

#[derive(Clone, Copy, Debug, Default)]
pub struct CameraView {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
}

#[derive(Default, Clone, Copy)]
pub struct EffectUpdateCtx {
    pub delta: f32,
    pub camera_target: Option<[f32; 3]>,
    pub caster_yaw: Option<f32>,
}

pub struct EffectRenderCtx {
    pub camera: CameraView,
    pub screen_w: f32,
    pub screen_h: f32,
    pub elapsed: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyTint {
    pub rgb: [u8; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraShake {
    pub amplitude: f32,
    pub duration_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Afterimage {
    pub tint: [u8; 3],
    pub start_alpha: f32,
    pub fade_per_frame: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyAction {
    pub action_index: usize,
    pub start_frame: usize,
    pub duration_ms: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyVertical {
    pub lift_px: f32,
    pub alpha: f32,
    pub squeeze: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyCopy {
    pub offset_px: [f32; 2],
    pub scale: [f32; 2],
    /// Margin added symmetrically on every edge in screen pixels (on top of `scale`).
    pub margin_px: f32,
    pub tint: [u8; 3],
    pub alpha: f32,
    pub additive: bool,
    pub behind: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberRequest {
    pub value: i32,
    pub color: [f32; 3],
}

pub trait Effect: Send {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus;
    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx);

    fn is_placeholder(&self) -> bool {
        false
    }

    fn set_link_endpoints(&mut self, _caster: [f32; 3], _target: [f32; 3]) {}

    fn set_position(&mut self, _pos: [f32; 3]) {}

    fn str_overlay(&self) -> Option<&'static str> {
        None
    }

    fn weapon_trail(&self) -> bool {
        false
    }

    fn body_tint(&self) -> Option<BodyTint> {
        None
    }

    fn body_additive(&self) -> bool {
        false
    }

    fn take_sfx_request(&mut self) -> Option<&'static str> {
        None
    }

    fn take_number_request(&mut self) -> Option<NumberRequest> {
        None
    }

    fn take_camera_shake(&mut self) -> Option<CameraShake> {
        None
    }

    /// Independent per-edge jitter in screen pixels, ordered top, bottom, left,
    /// right. Unequal edges stretch the body quad as well as move it.
    fn body_edge_jitter(&self) -> Option<[f32; 4]> {
        None
    }

    /// One extra additive draw of the weapon layer, over its normal draw, this
    /// frame.
    fn body_weapon_glow(&self) -> bool {
        false
    }

    fn body_afterimage(&self) -> Option<Afterimage> {
        None
    }

    fn body_yaw(&self) -> Option<f32> {
        None
    }

    fn body_scale(&self) -> Option<f32> {
        None
    }

    fn take_body_action(&mut self) -> Option<BodyAction> {
        None
    }

    fn body_vertical(&self) -> Option<BodyVertical> {
        None
    }

    fn body_angle(&self) -> Option<f32> {
        None
    }

    fn body_copies(&self) -> Option<Vec<BodyCopy>> {
        None
    }
}
