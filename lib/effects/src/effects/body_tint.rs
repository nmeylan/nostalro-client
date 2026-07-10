//! Actor body-tint effects: Redbody/Pinkbody/Transbluebody, tint-pulse family,
//! body-flash family (Bluebody/Redlightbody/RedHit/BlueHit), Madness strobe.

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{
    BodyCopy, BodyTint, BodyVertical, CameraShake, Effect, EffectRenderCtx, EffectUpdateCtx,
};

const FPS: f32 = 60.0;
const QUAKE_AMPLITUDE: f32 = 1.6;
const QUAKE_DURATION_MS: u32 = 600;

const PULSE_FLASH_W: f32 = 3.0;
const PULSE_BLINK_P: f32 = 6.0;
const PULSE_PAUSE_START: f32 = 12.0;
const PULSE_PAUSE_END: f32 = 20.0;
const PULSE_BLINK_END: f32 = 44.0;
const PULSE_COLOR_FULL: f32 = 56.0;
const PULSE_TOTAL: f32 = 96.0;
const WHITE: [u8; 3] = [255, 255, 255];

fn lerp_rgb(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [l(a[0], b[0]), l(a[1], b[1]), l(a[2], b[2])]
}

#[derive(Clone, Copy)]
enum TintMode {
    Fixed([u8; 3]),
    AnimatedToBlue,
    RandomFlicker,
    Pulse([u8; 3]),
    WhiteFlash,
    HitFlash {
        rgb: [u8; 3],
        bt2_scale: f32,
        bt2_cap: f32,
    },
    Strobe {
        rgb: [u8; 3],
        period: f32,
    },
}

#[derive(Clone, Copy)]
struct DoubleBody {
    margin_px: f32,
    alpha: f32,
}

#[derive(Clone, Copy)]
pub struct Params {
    mode: TintMode,
    window: (f32, f32),
    total_frames: f32,
    glow: f32,
    body_alpha: f32,
    light_body: bool,
    double_body: Option<DoubleBody>,
    quake_at: Option<f32>,
    sfx: Option<(f32, &'static str)>,
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
    glow: 0.0,
    body_alpha: 1.0,
    light_body: true,
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
    light_body: false,
    double_body: None,
    quake_at: None,
    sfx: None,
    yaw_per_frame: None,
};

pub const PINKBODY: Params = Params {
    mode: TintMode::Fixed([255, 89, 182]),
    window: (0.0, 120.0),
    total_frames: 120.0,
    glow: 0.0,
    body_alpha: 1.0,
    light_body: true,
    double_body: Some(DoubleBody {
        margin_px: 16.0,
        alpha: 0.4,
    }),
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
    light_body: false,
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
    light_body: false,
    double_body: None,
    quake_at: Some(30.0),
    sfx: Some((25.0, "effect\\매직 크래쉬.wav")),
    yaw_per_frame: None,
};

pub const MAGICCRASHER2: Params = Params {
    mode: TintMode::RandomFlicker,
    window: (0.0, 60.0),
    total_frames: 60.0,
    glow: 0.0,
    body_alpha: 1.0,
    light_body: false,
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
    light_body: false,
    double_body: None,
    quake_at: None,
    sfx: None,
    yaw_per_frame: None,
};

pub const FALCONASSAULT: Params = Params {
    mode: TintMode::Fixed([255, 255, 255]),
    window: (30.0, 54.0),
    total_frames: 54.0,
    glow: 0.8,
    body_alpha: 1.0,
    light_body: false,
    double_body: None,
    quake_at: Some(30.0),
    sfx: None,
    yaw_per_frame: Some(30.0 * std::f32::consts::PI / 180.0),
};

const fn pulse(rgb: [u8; 3]) -> Params {
    Params {
        mode: TintMode::Pulse(rgb),
        window: (0.0, PULSE_TOTAL),
        total_frames: PULSE_TOTAL,
        glow: 0.0,
        body_alpha: 1.0,
        light_body: false,
        double_body: None,
        quake_at: None,
        sfx: None,
        yaw_per_frame: None,
    }
}

pub const CHEMICALBODY: Params = pulse([0, 0, 255]);
pub const PIERCEBODY: Params = pulse([250, 250, 100]);
pub const MEMORIZE: Params = pulse([250, 250, 100]);
pub const DOUBLECASTBODY: Params = pulse([255, 0, 0]);
pub const GREENBODY: Params = pulse([0, 255, 0]);
pub const SHRINK: Params = pulse([250, 250, 100]);
pub const REJECTSWORD: Params = pulse([150, 150, 150]);

const fn hit_flash(rgb: [u8; 3], bt2_scale: f32, bt2_cap: f32, end: f32) -> Params {
    Params {
        mode: TintMode::HitFlash {
            rgb,
            bt2_scale,
            bt2_cap,
        },
        window: (0.0, end),
        total_frames: end,
        glow: 0.0,
        body_alpha: 1.0,
        light_body: false,
        double_body: None,
        quake_at: None,
        sfx: None,
        yaw_per_frame: None,
    }
}

pub const BLUEBODY: Params = hit_flash([5, 5, 255], 1.0 / 3.0, 1.0e9, 150.0);
pub const REDLIGHTBODY: Params = hit_flash([255, 5, 5], 1.0 / 8.0, 25.0, 200.0);
pub const REDHIT: Params = hit_flash([255, 5, 5], 3.0, 1.0e9, 18.0);
pub const BLUEHIT: Params = hit_flash([5, 5, 255], 3.0, 1.0e9, 18.0);

const fn strobe(rgb: [u8; 3]) -> Params {
    Params {
        mode: TintMode::Strobe { rgb, period: 4.0 },
        window: (0.0, 60.0),
        total_frames: 60.0,
        glow: 0.0,
        body_alpha: 1.0,
        light_body: false,
        double_body: None,
        quake_at: None,
        sfx: None,
        yaw_per_frame: None,
    }
}

pub const MADNESSBLUE: Params = strobe([5, 5, 255]);
pub const MADNESSRED: Params = strobe([255, 5, 5]);

pub const TEXTURES: &[&str] = &[];

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

fn hit_flash_alpha(bt2: f32) -> Option<f32> {
    let a = if bt2 <= 10.0 {
        bt2 * 15.0
    } else if bt2 <= 20.0 {
        160.0
    } else if bt2 <= 50.0 {
        155.0 - (bt2 - 20.0) * 5.0
    } else {
        return None;
    };
    Some((a / 255.0).clamp(0.0, 1.0))
}

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
    life_frames: Option<f32>,
    str_overlay: Option<&'static str>,
}

impl BodyTintEffect {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            process: 0.0,
            quake_pending: false,
            sfx_pending: false,
            life_frames: None,
            str_overlay: None,
        }
    }

    pub fn with_str_overlay(mut self, name: &'static str) -> Self {
        self.str_overlay = Some(name);
        self
    }

    pub fn with_life_ms(mut self, ms: Option<u32>) -> Self {
        self.life_frames = ms.map(|m| m as f32 / 1000.0 * FPS);
        self
    }

    fn window_end(&self) -> f32 {
        self.life_frames.unwrap_or(self.params.window.1)
    }

    fn in_window(&self) -> bool {
        self.process >= self.params.window.0 && self.process < self.window_end()
    }

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
            TintMode::Pulse(_) => None,
            TintMode::WhiteFlash | TintMode::HitFlash { .. } | TintMode::Strobe { .. } => None,
        }
    }

    fn pulse_render(&self) -> (bool, Option<[u8; 3]>) {
        let TintMode::Pulse(color) = self.params.mode else {
            return (false, None);
        };
        let p = self.process;
        if p < PULSE_BLINK_END {
            let in_pause = (PULSE_PAUSE_START..PULSE_PAUSE_END).contains(&p);
            let phase = if p < PULSE_PAUSE_START {
                p
            } else {
                p - PULSE_PAUSE_END
            };
            let on = !in_pause && (phase.rem_euclid(PULSE_BLINK_P) < PULSE_FLASH_W);
            (on, on.then_some(WHITE))
        } else if p < PULSE_COLOR_FULL {
            let t = (p - PULSE_BLINK_END) / (PULSE_COLOR_FULL - PULSE_BLINK_END);
            (false, Some(lerp_rgb(WHITE, color, t)))
        } else {
            let t = (PULSE_TOTAL - p) / (PULSE_TOTAL - PULSE_COLOR_FULL);
            (false, Some(lerp_rgb(WHITE, color, t)))
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
        if self.process >= self.life_frames.unwrap_or(self.params.total_frames) {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn str_overlay(&self) -> Option<&'static str> {
        self.str_overlay
    }

    fn body_tint(&self) -> Option<BodyTint> {
        if let TintMode::Pulse(_) = self.params.mode {
            return self.pulse_render().1.map(|rgb| BodyTint { rgb });
        }
        if self.params.glow > 0.0 {
            return None;
        }
        self.current_color().map(|rgb| BodyTint { rgb })
    }

    fn body_additive(&self) -> bool {
        if let TintMode::Pulse(_) = self.params.mode {
            return self.pulse_render().0;
        }
        self.params.light_body && self.in_window()
    }

    fn body_vertical(&self) -> Option<BodyVertical> {
        (self.params.body_alpha < 1.0 && self.in_window()).then_some(BodyVertical {
            lift_px: 0.0,
            alpha: self.params.body_alpha,
            squeeze: 1.0,
        })
    }

    fn body_yaw(&self) -> Option<f32> {
        let y = self.params.yaw_per_frame?;
        self.in_window()
            .then(|| (self.process - self.params.window.0) * y)
    }

    fn body_copies(&self) -> Option<Vec<BodyCopy>> {
        if let TintMode::WhiteFlash = self.params.mode {
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

        if let TintMode::HitFlash {
            rgb,
            bt2_scale,
            bt2_cap,
        } = self.params.mode
        {
            let bt2 = (self.process * bt2_scale).min(bt2_cap);
            let alpha = hit_flash_alpha(bt2)?;
            let copy = BodyCopy {
                offset_px: [0.0, 0.0],
                margin_px: 0.0,
                scale: [1.0, 1.0],
                tint: rgb,
                alpha,
                additive: true,
                behind: false,
            };
            return Some(vec![copy, copy]);
        }

        if let TintMode::Strobe { rgb, period } = self.params.mode {
            if (self.process.floor() as u32) % (period as u32) != 0 {
                return None;
            }
            let copy = BodyCopy {
                offset_px: [0.0, 0.0],
                margin_px: 0.0,
                scale: [1.0, 1.0],
                tint: rgb,
                alpha: 160.0 / 255.0,
                additive: true,
                behind: false,
            };
            return Some(vec![copy, copy]);
        }

        let mut copies = Vec::new();
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
            CameraShake {
                amplitude: QUAKE_AMPLITUDE,
                duration_ms: QUAKE_DURATION_MS,
            }
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
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        })
    }

    #[test]
    fn linklight_glows_additively_in_its_delayed_window() {
        let mut e = BodyTintEffect::new(LINKLIGHT);
        assert!(e.body_copies().is_none());
        step(&mut e, 50.0);
        assert_eq!(e.body_tint(), None);
        let glow = e.body_copies().expect("glowing");
        assert!(glow[0].additive && glow[0].tint == [200, 150, 50]);
        assert_eq!(step(&mut e, 30.0), EffectStatus::Dead);
    }

    #[test]
    fn redbody_is_a_translucent_additive_red_body_held_for_the_status() {
        let mut e = BodyTintEffect::new(REDBODY).with_life_ms(Some(60_000));
        assert!(e.body_additive());
        assert_eq!(e.body_tint().map(|t| t.rgb), Some([255, 100, 100]));
        assert_eq!(step(&mut e, 300.0), EffectStatus::Running);
        assert!(e.body_additive() && e.body_tint().is_some());
    }

    #[test]
    fn pinkbody_is_a_translucent_additive_pink_body_with_a_ghost_halo() {
        let e = BodyTintEffect::new(PINKBODY);
        assert!(e.body_additive());
        assert_eq!(e.body_tint().map(|t| t.rgb), Some([255, 89, 182]));
        let copies = e.body_copies().expect("halo");
        let halo = copies.iter().find(|c| !c.additive).expect("behind ghost");
        assert!(halo.margin_px > 0.0 && halo.tint == [255, 89, 182]);
    }

    #[test]
    fn flicker_glows_during_window_with_one_shot_quake_and_sfx() {
        let mut e = BodyTintEffect::new(MAGICCRASHER);
        step(&mut e, 26.0);
        assert_eq!(e.take_sfx_request(), Some("effect\\매직 크래쉬.wav"));
        assert_eq!(e.take_sfx_request(), None);
        step(&mut e, 9.0);
        assert!(e.body_copies().is_some());
        assert!(e.take_camera_shake().is_some());
        assert!(e.take_camera_shake().is_none());
    }

    #[test]
    fn hitbody_is_a_single_additive_white_flash_no_tint() {
        let mut e = BodyTintEffect::new(HITBODY);
        step(&mut e, 4.0);
        assert_eq!(e.body_tint(), None);
        let copies = e.body_copies().expect("flashing");
        assert_eq!(copies.len(), 1);
        assert!(copies[0].additive && copies[0].tint == [255, 255, 255]);
        step(&mut e, 12.0);
        assert!(e.body_copies().is_none());
    }

    #[test]
    fn falconassault_spins_the_facing_without_tinting() {
        let mut e = BodyTintEffect::new(FALCONASSAULT);
        assert!(e.body_yaw().is_none());
        step(&mut e, 40.0);
        assert_eq!(e.body_tint(), None);
        assert!(e.body_yaw().unwrap() > 0.0);
        let glow = e.body_copies().expect("glowing");
        assert!(glow[0].additive && glow[0].tint == [255, 255, 255]);
    }

    #[test]
    fn transbluebody_bleeds_toward_blue() {
        let mut e = BodyTintEffect::new(TRANSBLUEBODY);
        let early = e.body_tint().unwrap().rgb;
        step(&mut e, 150.0);
        let late = e.body_tint().unwrap().rgb;
        assert_eq!(early[2], 255);
        assert!(late[0] < early[0]);
    }

    #[test]
    fn chemicalbody_flashes_white_then_fades_a_blue_multiply_tint() {
        let mut e = BodyTintEffect::new(CHEMICALBODY);
        assert!(e.body_additive());
        assert_eq!(e.body_tint().map(|t| t.rgb), Some([255, 255, 255]));
        assert!(e.body_copies().is_none());
        step(&mut e, PULSE_FLASH_W);
        assert!(!e.body_additive() && e.body_tint().is_none());
        let mut e = BodyTintEffect::new(CHEMICALBODY);
        step(&mut e, PULSE_COLOR_FULL);
        assert!(!e.body_additive());
        let full = e.body_tint().expect("blue tint").rgb;
        assert!(full[2] > full[0] && full[2] > full[1]);
        step(&mut e, (PULSE_TOTAL - PULSE_COLOR_FULL) * 0.6);
        let faded = e.body_tint().expect("fading tint").rgb;
        assert!(faded[0] > full[0]);
    }

    #[test]
    fn reject_sword_pairs_a_gray_flicker_with_the_sword_str() {
        let e = BodyTintEffect::new(REJECTSWORD).with_str_overlay("sword");
        assert_eq!(e.str_overlay(), Some("sword"));
        assert!(e.body_additive());
        assert_eq!(e.body_tint().map(|t| t.rgb), Some([255, 255, 255]));
        assert_eq!(BodyTintEffect::new(PIERCEBODY).str_overlay(), None);
    }

    #[test]
    fn pulse_family_ends_with_its_timeline() {
        let mut e = BodyTintEffect::new(MEMORIZE);
        assert_eq!(step(&mut e, PULSE_TOTAL + 1.0), EffectStatus::Dead);
    }

    #[test]
    fn redhit_ramps_two_additive_red_copies_then_dies() {
        let mut e = BodyTintEffect::new(REDHIT);
        assert_eq!(e.body_tint(), None);
        assert!(!e.body_additive());
        step(&mut e, 5.0);
        let hold = e.body_copies().expect("flashing");
        assert_eq!(hold.len(), 2);
        assert!(hold[0].additive && !hold[0].behind && hold[0].tint == [255, 5, 5]);
        let hold_alpha = hold[0].alpha;
        step(&mut e, 8.0);
        let fade = e.body_copies().expect("still fading");
        assert!(fade[0].alpha < hold_alpha);
        assert_eq!(step(&mut e, REDHIT.total_frames), EffectStatus::Dead);
    }

    #[test]
    fn madnessblue_strobes_solid_blue_then_dies() {
        let mut e = BodyTintEffect::new(MADNESSBLUE);
        assert_eq!(e.body_tint(), None);
        assert!(!e.body_additive());
        let on = e.body_copies().expect("blink on");
        assert_eq!(on.len(), 2);
        assert!(on[0].additive && !on[0].behind && on[0].tint == [5, 5, 255]);
        step(&mut e, 2.0);
        assert!(e.body_copies().is_none());
        assert_eq!(step(&mut e, 60.0), EffectStatus::Dead);
    }

    #[test]
    fn bluebody_is_slower_than_bluehit() {
        let mut slow = BodyTintEffect::new(BLUEBODY);
        let mut fast = BodyTintEffect::new(BLUEHIT);
        step(&mut slow, 6.0);
        step(&mut fast, 6.0);
        let s = slow.body_copies().expect("blue glow");
        let f = fast.body_copies().expect("blue flash");
        assert_eq!(s[0].tint, [5, 5, 255]);
        assert_eq!(f[0].tint, [5, 5, 255]);
        assert!(s[0].alpha < f[0].alpha);
    }
}
