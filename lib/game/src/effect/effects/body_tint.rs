//! Actor body-tint effects (per-frame body colour + body-light glow):
//!
//! * **Redbody** (368) — glowing body + fixed soft red `(255,100,100)`.
//! * **Transbluebody** (379) — colour `(255-(cnt+50), 255-(cnt+50), 255)` over
//!   `cnt` 0..=200: R/G fall while B stays 255, so the body bleeds to blue.
//! * **Pinkbody** (396) — glowing body + double-body halo + fixed pink
//!   `(255,89,182)` plus a concentric additive double-body halo.
//! * **Linklight** (385) — warm yellow `(200,150,50)` only during `cnt` 40..=70.
//! * **Magiccrasher** (380) — random colour flicker `cnt` 30..=60, `magiccrash.wav`
//!   at 25, quake at 30.
//! * **Magiccrasher2** (403) — random colour flicker, no light-body/quake/sound.
//! * **Hitbody** (417) — a single **additive white** flash with an alpha
//!   ramp over a 0..=11 frame clock (not a tint multiply — white multiply is a no-op).
//! * **Falconassault** (387) — glowing body + the target's facing spun
//!   `+30°`/frame during `cnt` 30..=54, quake at 30 (no colour, so no tint).
//!
//! All are persistent buffs in the original (kept alive by their status); here
//! each runs a self-contained window. They emit no world primitives — the tint /
//! double-body / spin are applied to the actor sprite by the shared composer.

use crate::effect::draw::{EffectDrawList, EffectStatus};
use crate::effect::effect_trait::{
    BodyCopy, BodyTint, BodyVertical, CameraShake, Effect, EffectRenderCtx, EffectUpdateCtx,
};

const FPS: f32 = 60.0;
const QUAKE_AMPLITUDE: f32 = 1.6;
const QUAKE_DURATION_MS: u32 = 600;

#[derive(Clone, Copy)]
enum TintMode {
    Fixed([u8; 3]),
    AnimatedToBlue,
    RandomFlicker,
    /// Additive white flash — no body_tint, one additive copy instead.
    WhiteFlash,
}

/// Double-body halo: a tinted alpha copy behind the body, showing as a
/// margin around the silhouette.
#[derive(Clone, Copy)]
struct DoubleBody {
    /// Halo thickness — pixels grown on every edge. Bump for a fatter halo.
    margin_px: f32,
    /// Halo opacity.
    alpha: f32,
}

#[derive(Clone, Copy)]
pub struct Params {
    mode: TintMode,
    /// Frames `[start, end)` the modifier (tint / spin) is active.
    window: (f32, f32),
    /// Effect lifetime in frames.
    total_frames: f32,
    /// Body-glow strength (0 = none → the colour is a plain multiply
    /// tint instead). `>0` keeps the body opaque and lays the colour over it as
    /// an **additive** overlay of this alpha — a glow, not a wash-out. **Tune
    /// this for glow intensity.**
    glow: f32,
    /// Body opacity (1.0 = opaque). `<1.0` makes the whole body translucent —
    /// **tune this for see-through-ness** (Pinkbody is the most translucent).
    body_alpha: f32,
    /// Concentric double-body halo behind the body (Pinkbody), or `None`.
    double_body: Option<DoubleBody>,
    /// Camera-quake frame (one-shot), if any.
    quake_at: Option<f32>,
    /// One-shot SFX `(frame, path)`.
    sfx: Option<(f32, &'static str)>,
    /// Falconassault facing spin per frame (radians); accumulates from `window.0`.
    yaw_per_frame: Option<f32>,
}

impl Params {
    pub const fn total_duration_ms(&self) -> u32 {
        (self.total_frames / FPS * 1000.0) as u32
    }
}

pub const REDBODY: Params = Params {
    mode: TintMode::Fixed([255, 100, 100]),
    window: (0.0, 120.0),
    total_frames: 120.0,
    glow: 1.0,
    body_alpha: 1.0,
    double_body: None,
    quake_at: None,
    sfx: None,
    yaw_per_frame: None,
};

pub const TRANSBLUEBODY: Params = Params {
    mode: TintMode::AnimatedToBlue,
    window: (0.0, 200.0),
    total_frames: 200.0,
    glow: 0.0,
    body_alpha: 1.0,
    double_body: None,
    quake_at: None,
    sfx: None,
    yaw_per_frame: None,
};

pub const PINKBODY: Params = Params {
    mode: TintMode::Fixed([255, 89, 182]),
    window: (0.0, 120.0),
    total_frames: 120.0,
    // Opaque pink body (multiply tint) + a pink ghost halo behind. The body is
    // opaque so the larger halo only shows as a margin around it (a translucent
    // body would let the enlarged copy bleed through and double the sprite).
    glow: 0.0,
    body_alpha: 1.0,
    double_body: Some(DoubleBody { margin_px: 16.0, alpha: 0.4 }),
    quake_at: None,
    sfx: None,
    yaw_per_frame: None,
};

pub const LINKLIGHT: Params = Params {
    mode: TintMode::Fixed([200, 150, 50]),
    window: (40.0, 70.0),
    total_frames: 70.0,
    glow: 1.0,
    body_alpha: 1.0,
    double_body: None,
    quake_at: None,
    sfx: None,
    yaw_per_frame: None,
};

pub const MAGICCRASHER: Params = Params {
    mode: TintMode::RandomFlicker,
    window: (30.0, 60.0),
    total_frames: 60.0,
    glow: 1.0,
    body_alpha: 1.0,
    double_body: None,
    quake_at: Some(30.0),
    sfx: Some((25.0, "effect\\magiccrash.wav")),
    yaw_per_frame: None,
};

pub const MAGICCRASHER2: Params = Params {
    mode: TintMode::RandomFlicker,
    window: (0.0, 60.0),
    total_frames: 60.0,
    glow: 0.0,
    body_alpha: 1.0,
    double_body: None,
    quake_at: None,
    sfx: None,
    yaw_per_frame: None,
};

pub const HITBODY: Params = Params {
    mode: TintMode::WhiteFlash,
    window: (0.0, 15.0),
    total_frames: 15.0,
    glow: 0.0,
    body_alpha: 1.0,
    double_body: None,
    quake_at: None,
    sfx: None,
    yaw_per_frame: None,
};

pub const FALCONASSAULT: Params = Params {
    // Glowing body with no colour → a white additive glow (brightening) while
    // the facing spins; the white *flash* some skills show is a separate effect.
    mode: TintMode::Fixed([255, 255, 255]),
    window: (30.0, 54.0),
    total_frames: 54.0,
    glow: 0.8,
    body_alpha: 1.0,
    double_body: None,
    quake_at: Some(30.0),
    sfx: None,
    yaw_per_frame: Some(30.0 * std::f32::consts::PI / 180.0),
};

pub const TEXTURES: &[&str] = &[];

/// Stable per-frame random RGB (deterministic xorshift), so the flicker is
/// reproducible and dependency-free.
fn flicker_rgb(frame: u32) -> [u8; 3] {
    let mut s = frame.wrapping_mul(2_654_435_761).wrapping_add(1);
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        (s >> 24) as u8
    };
    [next(), next(), next()]
}

/// Hit-flash alpha, `None` past frame 11.
fn hit_alpha(t: f32) -> Option<f32> {
    let a = if t <= 5.0 {
        t * 50.0
    } else if t <= 8.0 {
        255.0
    } else if t <= 11.0 {
        255.0 - (t - 8.0) * 80.0
    } else {
        return None;
    };
    Some((a / 255.0).clamp(0.0, 1.0))
}

pub struct BodyTintEffect {
    params: Params,
    process: f32,
    quake_pending: bool,
    sfx_pending: bool,
}

impl BodyTintEffect {
    pub fn new(params: Params) -> Self {
        Self { params, process: 0.0, quake_pending: false, sfx_pending: false }
    }

    fn in_window(&self) -> bool {
        self.process >= self.params.window.0 && self.process < self.params.window.1
    }

    /// The tint colour for this frame, or `None` outside
    /// the window / for the white-flash mode.
    fn current_color(&self) -> Option<[u8; 3]> {
        if !self.in_window() {
            return None;
        }
        match self.params.mode {
            TintMode::Fixed(rgb) => Some(rgb),
            TintMode::AnimatedToBlue => {
                let v = (255.0 - (self.process + 50.0)).clamp(0.0, 255.0) as u8;
                Some([v, v, 255])
            }
            TintMode::RandomFlicker => Some(flicker_rgb(self.process as u32)),
            TintMode::WhiteFlash => None,
        }
    }
}

impl Effect for BodyTintEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let before = self.process;
        self.process += ctx.delta * FPS;
        if let Some(at) = self.params.quake_at {
            if before < at && self.process >= at {
                self.quake_pending = true;
            }
        }
        if let Some((at, _)) = self.params.sfx {
            if before < at && self.process >= at {
                self.sfx_pending = true;
            }
        }
        if self.process >= self.params.total_frames {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn body_tint(&self) -> Option<BodyTint> {
        // When the body glows (`glow > 0`), the colour is delivered as an
        // additive overlay copy (body stays opaque), not a darkening multiply.
        if self.params.glow > 0.0 {
            return None;
        }
        self.current_color().map(|rgb| BodyTint { rgb })
    }

    fn body_vertical(&self) -> Option<BodyVertical> {
        // Body translucency knob — `<1.0` makes the whole body see-through.
        (self.params.body_alpha < 1.0 && self.in_window())
            .then_some(BodyVertical { lift_px: 0.0, alpha: self.params.body_alpha, squeeze: 1.0 })
    }

    fn body_yaw(&self) -> Option<f32> {
        let y = self.params.yaw_per_frame?;
        self.in_window().then(|| (self.process - self.params.window.0) * y)
    }

    fn body_copies(&self) -> Option<Vec<BodyCopy>> {
        if let TintMode::WhiteFlash = self.params.mode {
            // Hitbody: one additive white copy with the hit-flash alpha ramp.
            let alpha = hit_alpha(self.process)?;
            return Some(vec![BodyCopy {
                offset_px: [0.0, 0.0],
                margin_px: 0.0,
                scale: [1.0, 1.0],
                tint: [255, 255, 255],
                alpha,
                additive: true,
                behind: false,
            }]);
        }

        let mut copies = Vec::new();
        // Body glow — an in-place additive overlay of the current colour
        // over the (still opaque) body at the body's own size; `glow` is the
        // overlay alpha (intensity). Same size, NOT a larger copy — a larger one
        // would read as a concentric double-body halo.
        if self.params.glow > 0.0 {
            if let Some(tint) = self.current_color() {
                copies.push(BodyCopy {
                    offset_px: [0.0, 0.0],
                    margin_px: 0.0,
                    scale: [1.0, 1.0],
                    tint,
                    alpha: self.params.glow,
                    additive: true,
                    behind: false,
                });
            }
        }
        // Double-body — a ghost copy BEHIND the body (alpha-blended), grown by
        // a fixed pixel margin on every edge so it shows as a halo around the
        // silhouette. Size/opacity come from the `DoubleBody` params.
        if let (Some(halo), true) = (self.params.double_body, self.in_window()) {
            let tint = self.current_color().unwrap_or([255, 255, 255]);
            copies.push(BodyCopy {
                offset_px: [0.0, 0.0],
                margin_px: halo.margin_px,
                scale: [1.0, 1.0],
                tint,
                alpha: halo.alpha,
                additive: false,
                behind: true,
            });
        }
        (!copies.is_empty()).then_some(copies)
    }

    fn take_camera_shake(&mut self) -> Option<CameraShake> {
        self.quake_pending.then(|| {
            self.quake_pending = false;
            CameraShake { amplitude: QUAKE_AMPLITUDE, duration_ms: QUAKE_DURATION_MS }
        })
    }

    fn take_sfx_request(&mut self) -> Option<&'static str> {
        if self.sfx_pending {
            self.sfx_pending = false;
            return self.params.sfx.map(|(_, path)| path);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut BodyTintEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx { delta: frames / FPS, camera_target: None, caster_yaw: None })
    }

    #[test]
    fn linklight_glows_additively_in_its_delayed_window() {
        // Linklight is delayed to frames 40..=70 and glows (no multiply tint).
        let mut e = BodyTintEffect::new(LINKLIGHT);
        assert!(e.body_copies().is_none(), "no glow before the window");
        step(&mut e, 50.0);
        assert_eq!(e.body_tint(), None, "glow effects don't multiply-tint");
        let glow = e.body_copies().expect("glowing");
        assert!(glow[0].additive && glow[0].tint == [200, 150, 50], "yellow additive glow");
        assert_eq!(step(&mut e, 30.0), EffectStatus::Dead);
    }

    #[test]
    fn pinkbody_is_an_opaque_pink_body_with_a_ghost_halo() {
        let e = BodyTintEffect::new(PINKBODY);
        // Opaque pink body (multiply tint) + a larger ghost copy behind it; the
        // body stays opaque so the halo only shows as a margin (no bleed-through).
        assert_eq!(e.body_tint().map(|t| t.rgb), Some([255, 89, 182]), "pink multiply tint");
        assert!(e.body_vertical().is_none(), "body stays opaque");
        let copies = e.body_copies().expect("halo");
        let halo = copies.iter().find(|c| !c.additive).expect("behind ghost");
        assert!(halo.margin_px > 0.0 && halo.tint == [255, 89, 182], "pink halo margin");
    }

    #[test]
    fn flicker_glows_during_window_with_one_shot_quake_and_sfx() {
        let mut e = BodyTintEffect::new(MAGICCRASHER);
        step(&mut e, 26.0); // past the SFX frame (25), before the flicker (30)
        assert_eq!(e.take_sfx_request(), Some("effect\\magiccrash.wav"));
        assert_eq!(e.take_sfx_request(), None, "sfx is one-shot");
        step(&mut e, 9.0); // ~frame 35, inside 30..=60
        assert!(e.body_copies().is_some(), "flickering glow");
        assert!(e.take_camera_shake().is_some(), "quake fired at 30");
        assert!(e.take_camera_shake().is_none(), "quake is one-shot");
    }

    #[test]
    fn hitbody_is_a_single_additive_white_flash_no_tint() {
        let mut e = BodyTintEffect::new(HITBODY);
        step(&mut e, 4.0); // frame ~4, fade-in
        assert_eq!(e.body_tint(), None, "white flash is a copy, not a tint multiply");
        let copies = e.body_copies().expect("flashing");
        assert_eq!(copies.len(), 1);
        assert!(copies[0].additive && copies[0].tint == [255, 255, 255]);
        step(&mut e, 12.0); // past frame 11
        assert!(e.body_copies().is_none(), "flash gone after BodyTime 11");
    }

    #[test]
    fn falconassault_spins_the_facing_without_tinting() {
        let mut e = BodyTintEffect::new(FALCONASSAULT);
        assert!(e.body_yaw().is_none(), "no spin before frame 30");
        step(&mut e, 40.0); // inside 30..=54
        assert_eq!(e.body_tint(), None, "Falconassault has no multiply tint");
        assert!(e.body_yaw().unwrap() > 0.0, "facing spins");
        let glow = e.body_copies().expect("glowing");
        assert!(glow[0].additive && glow[0].tint == [255, 255, 255], "white BL_LIGHT_BODY glow");
    }

    #[test]
    fn transbluebody_bleeds_toward_blue() {
        let mut e = BodyTintEffect::new(TRANSBLUEBODY);
        let early = e.body_tint().unwrap().rgb;
        step(&mut e, 150.0);
        let late = e.body_tint().unwrap().rgb;
        assert_eq!(early[2], 255, "blue channel stays maxed");
        assert!(late[0] < early[0], "red falls toward blue");
    }
}
