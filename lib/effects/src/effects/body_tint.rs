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
//! Tint-pulse family (the §8a body-tint effects). In-game these run a three-part
//! sequence: the body **blinks white** (a bright additive wash you can see
//! through — white over the additive glowing body) — two flashes, a
//! short pause, then four more — and then the **colour** comes in as a darkening
//! **multiply** tint that **fades back to normal**. So the white flashes and the
//! coloured phase are distinct stages, not a per-frame colour↔white flicker. The
//! schedule lives in `pulse_render`; only the colour varies per id:
//!
//! * **Chemicalbody** (500) — blue `(0,0,255)`.
//! * **Piercebody** (502) — yellow `(250,250,100)`.
//! * **Memorize** (505) — yellow `(250,250,100)`.
//! * **Doublecastbody** (521) — red `(255,0,0)`.
//! * **Greenbody** (538) — green `(0,255,0)`.
//! * **Shrink** (599) — yellow `(250,250,100)`; the original game sets no pixel
//!   ratio, so this is a tint-pulse, **not** a resize.
//!
//! (SFX cues are omitted for now.) The original game also adds a double-body on most
//! of these, but over the additive body it is only a faint same-size bloom — **no
//! distinct halo** — so we render the body alone.
//!
//! Body-flash family (the §8b effects) — a *different* mechanism from the pulse:
//! one fixed colour glows over the still-opaque body (an additive overlay drawn
//! twice = a 2× additive pass) and its alpha breathes in → holds →
//! out exactly once. No white flashes, no multiply tint. Only the colour and the
//! rate/cap of the glow clock differ per id:
//!
//! * **Bluebody** (542) — blue `(5,5,255)`, slow.
//! * **Redlightbody** (544) — red `(255,5,5)`, slow; the clock caps so it holds lit.
//! * **RedHit** (548) — red `(255,5,5)`, fast flash.
//! * **BlueHit** (549) — blue `(5,5,255)`, fast flash.
//!
//! Madness strobe (§9a) — the same hit-flash colours blinked on/off (the flash
//! re-arms every 4 frames at the hold alpha), a solid-colour strobe:
//!
//! * **MadnessBlue** (625) — blue `(5,5,255)`.
//! * **MadnessRed** (626) — red `(255,5,5)`.
//!
//! All emit no world primitives — the tint / additive overlay / spin are applied
//! to the actor sprite by the shared composer.

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{
    BodyCopy, BodyTint, BodyVertical, CameraShake, Effect, EffectRenderCtx, EffectUpdateCtx,
};

const FPS: f32 = 60.0;
const QUAKE_AMPLITUDE: f32 = 1.6;
const QUAKE_DURATION_MS: u32 = 600;

// §8a pulse timeline (frames @ 60 fps). Two white additive flashes, a short
// pause, then four more flashes (white over the additive body), after
// which the colour appears as a darkening multiply tint that fades back to
// normal — matching the in-game Chemicalbody sequence.
const PULSE_FLASH_W: f32 = 3.0; // white-on frames per blink
const PULSE_BLINK_P: f32 = 6.0; // blink period (white on for half)
const PULSE_PAUSE_START: f32 = 12.0; // after 2 blinks
const PULSE_PAUSE_END: f32 = 20.0; // 4 more blinks begin
const PULSE_BLINK_END: f32 = 44.0; // flashes done
const PULSE_COLOR_FULL: f32 = 56.0; // colour fully in by here
const PULSE_TOTAL: f32 = 96.0; // colour faded out by here
const WHITE: [u8; 3] = [255, 255, 255];

/// Per-channel linear blend `a → b` (`t` clamped to `0..=1`).
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
    /// Smooth additive coloured glow (the §8a glowing-body tint family).
    /// The body keeps this hue and its glow **intensity** follows a smooth
    /// envelope (build up → hold with a gentle breathing ripple → fade) — the
    /// integrated, on-screen result of the original game's per-frame colour
    /// flicker + body-light pulse, not a hard on/off blink.
    Pulse([u8; 3]),
    /// Additive white flash — no body_tint, one additive copy instead.
    WhiteFlash,
    /// Red / blue body-flash (§8b): one fixed colour laid over
    /// the (still opaque) body as an additive overlay whose alpha ramps up →
    /// holds → down. The ramp runs on a glow clock `= process *
    /// bt2_scale`, clamped at `bt2_cap` (the clock advances at a per-effect
    /// rate, and Redlightbody caps it so the
    /// glow holds lit). Drawn as **two** additive copies = a 2×
    /// additive sprite pass. Distinct from the §8a `Pulse` family: no
    /// white flashes, no multiply tint, just a coloured glow that breathes once.
    HitFlash { rgb: [u8; 3], bt2_scale: f32, bt2_cap: f32 },
    /// Red/blue **strobed** (the Madness effects): the flash
    /// re-sets every `period` frames at the hold alpha (glow clock 20
    /// → 160) with no ramp, so the body blinks a solid colour on/off. Rendered
    /// like `HitFlash` (two additive copies) but only on the `% period == 0` frame.
    Strobe { rgb: [u8; 3], period: f32 },
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
    /// Light-body: the whole body sprite blends **additively** while keeping its
    /// multiply tint — dark texels vanish (see-through) and lit texels glow the
    /// tint over the scene. This is what makes Berserk's red body look almost
    /// translucent, distinct from `body_alpha` (which uniformly fades the body).
    light_body: bool,
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
    // Light-body: a red multiply tint over an additively-blended body, so the
    // body reads as an almost-translucent red glow (the Berserk look) for the
    // whole buff. The translucency is the additive blend, not lowered alpha.
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
    // Light-body pink (translucent additive body, like Redbody) + a pink ghost
    // halo behind it. Same BL_LIGHT_BODY translucency as Redbody, plus the
    // double-body margin around the silhouette.
    glow: 0.0,
    body_alpha: 1.0,
    light_body: true,
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
    sfx: Some((25.0, "effect\\magiccrash.wav")),
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
    // Glowing body with no colour → a white additive glow (brightening) while
    // the facing spins; the white *flash* some skills show is a separate effect.
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

// Tint-pulse family — the whole body is an additive glow of `rgb` whose
// intensity follows a smooth build → hold → fade envelope (the on-screen result
// of the original game's per-frame colour flicker + body-light pulse),
// so the colour transitions smoothly and stays tinted (never a hard blink). The
// original game also adds a double-body on some, but over an already-additive body
// it is only a faint same-size bloom (no distinct halo), so we render the
// additive body alone. SFX wiring is omitted for now. The full timeline (white
// flashes → colour → fade) is fixed in `pulse_render`; only the colour varies.
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
// `EF_SHRINK` is a yellow tint-pulse (the original sets no pixel ratio), not a resize.
pub const SHRINK: Params = pulse([250, 250, 100]);
// `EF_REJECTSWORD` flickers the caster gray while `sword.str` plays (wired with
// `with_str_overlay`). The original alternates two grays; the pulse blink reads
// the same against the body.
pub const REJECTSWORD: Params = pulse([150, 150, 150]);

// Body-flash family (the §8b red / blue hit-flash effects). One fixed
// colour is laid over the opaque body as an additive overlay whose alpha breathes
// in → holds → out once. `end` is the lifetime in frames; the only per-effect
// knobs are colour, the glow-clock advance rate, and its cap.
const fn hit_flash(rgb: [u8; 3], bt2_scale: f32, bt2_cap: f32, end: f32) -> Params {
    Params {
        mode: TintMode::HitFlash { rgb, bt2_scale, bt2_cap },
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

// Slow blue glow: glow clock `= frame/3`, lit over its 0..=150 window.
pub const BLUEBODY: Params = hit_flash([5, 5, 255], 1.0 / 3.0, 1.0e9, 150.0);
// Slow red light: glow clock `= frame/8` capped at 25, so the glow ramps up
// then holds (≈alpha 130) — the original game keeps this as a persistent buff; we
// give it a finite self-contained window (the holder kills it at `total_frames`).
pub const REDLIGHTBODY: Params = hit_flash([255, 5, 5], 1.0 / 8.0, 25.0, 200.0);
// Fast red/blue flash: glow clock `= frame*3`, gone (`bt2 > 50`) just past frame 16.
pub const REDHIT: Params = hit_flash([255, 5, 5], 3.0, 1.0e9, 18.0);
pub const BLUEHIT: Params = hit_flash([5, 5, 255], 3.0, 1.0e9, 18.0);

// Madness blink (the §9a hit-flash strobe): a solid colour flashes on every 4th
// frame over a 60-frame window.
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

/// Red / blue hit-flash overlay alpha on a
/// glow clock: ramp in `×15` (0→150), hold `160`, fade `155-(t-20)·5`,
/// then `None` past 50.
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
    /// Status-tied lifetime in frames. When set (a persistent buff like
    /// Berserk), the fixed colour holds and the effect stays alive for this
    /// long instead of the authored `total_frames` — the EFST's `remain_ms`
    /// drives it. `None` keeps the one-shot skill-flash timing.
    life_frames: Option<f32>,
    /// STR played alongside the body flash (the original's hybrid effects that
    /// recolor the body *and* play a world STR — e.g. Reject Sword's
    /// `sword.str`). `None` for the pure body flashes.
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

    /// Play `name`'s STR alongside the body flash (the hybrid effects that both
    /// recolor the body and draw a world STR).
    pub fn with_str_overlay(mut self, name: &'static str) -> Self {
        self.str_overlay = Some(name);
        self
    }

    /// Tie this tint to a status duration so it persists (and holds its colour)
    /// for `ms` instead of its authored window. Only meaningful for the fixed
    /// buff tints (Berserk/Marionette); skill-flash modes pass `None`.
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
            // Pulse is driven by `pulse_render`, not the generic tint path.
            TintMode::Pulse(_) => None,
            // The flash modes deliver their colour via additive `body_copies`.
            TintMode::WhiteFlash | TintMode::HitFlash { .. } | TintMode::Strobe { .. } => None,
        }
    }

    /// The §8a pulse state for this frame: `(additive, tint)`. White additive
    /// flashes (`Some(WHITE)`, additive) blink against the normal body (2, pause,
    /// 4), then the colour comes in as a darkening multiply tint (`additive =
    /// false`) and fades back to normal.
    fn pulse_render(&self) -> (bool, Option<[u8; 3]>) {
        let TintMode::Pulse(color) = self.params.mode else {
            return (false, None);
        };
        let p = self.process;
        if p < PULSE_BLINK_END {
            // White additive flash: on for the first half of each blink period,
            // with a pause between the 2-blink and 4-blink groups.
            let in_pause = (PULSE_PAUSE_START..PULSE_PAUSE_END).contains(&p);
            let phase = if p < PULSE_PAUSE_START { p } else { p - PULSE_PAUSE_END };
            let on = !in_pause && (phase.rem_euclid(PULSE_BLINK_P) < PULSE_FLASH_W);
            (on, on.then_some(WHITE))
        } else if p < PULSE_COLOR_FULL {
            // Colour builds in as a multiply tint (white → full colour).
            let t = (p - PULSE_BLINK_END) / (PULSE_COLOR_FULL - PULSE_BLINK_END);
            (false, Some(lerp_rgb(WHITE, color, t)))
        } else {
            // Colour fades back to normal (full colour → white).
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
        // The §8a pulse family: white flash (additive) or the multiply colour.
        if let TintMode::Pulse(_) = self.params.mode {
            return self.pulse_render().1.map(|rgb| BodyTint { rgb });
        }
        // When the body glows (`glow > 0`), the colour is delivered as an
        // additive overlay copy (body stays opaque), not a darkening multiply.
        if self.params.glow > 0.0 {
            return None;
        }
        self.current_color().map(|rgb| BodyTint { rgb })
    }

    fn body_additive(&self) -> bool {
        // Only the white flash frames blend additively; the colour phase is a
        // (darkening) multiply tint.
        if let TintMode::Pulse(_) = self.params.mode {
            return self.pulse_render().0;
        }
        // Light-body (Redbody/Pinkbody): the whole body blends additively under
        // its multiply tint, so it reads as an almost-translucent coloured glow.
        self.params.light_body && self.in_window()
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

        if let TintMode::HitFlash { rgb, bt2_scale, bt2_cap } = self.params.mode {
            // Body-flash: a fixed colour over the opaque body, drawn as two
            // additive overlays (a 2× additive pass) with the
            // glow-clock-driven alpha ramp.
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
            // Solid colour blinks on for one frame every `period`, at the
            // hold alpha (glow clock 20 → 160), drawn 2x additive.
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
    fn redbody_is_a_translucent_additive_red_body_held_for_the_status() {
        // Berserk: a red multiply tint over an additively-blended (almost
        // see-through) body, persisting for the EFST duration, not the 120-frame
        // authored window.
        let mut e = BodyTintEffect::new(REDBODY).with_life_ms(Some(60_000));
        assert!(e.body_additive(), "light-body blends additively");
        assert_eq!(e.body_tint().map(|t| t.rgb), Some([255, 100, 100]), "red multiply tint");
        // Well past the 120-frame default, still alive and still glowing.
        assert_eq!(step(&mut e, 300.0), EffectStatus::Running, "persists for the status");
        assert!(e.body_additive() && e.body_tint().is_some(), "still a red light-body");
    }

    #[test]
    fn pinkbody_is_a_translucent_additive_pink_body_with_a_ghost_halo() {
        let e = BodyTintEffect::new(PINKBODY);
        // Light-body pink (additive, translucent like Redbody) + a ghost copy
        // behind it showing as a margin around the silhouette.
        assert!(e.body_additive(), "light-body blends additively");
        assert_eq!(e.body_tint().map(|t| t.rgb), Some([255, 89, 182]), "pink multiply tint");
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

    #[test]
    fn chemicalbody_flashes_white_then_fades_a_blue_multiply_tint() {
        // Phase 1: a white additive flash (frame 0 is white-on). No halo.
        let mut e = BodyTintEffect::new(CHEMICALBODY);
        assert!(e.body_additive(), "white flash blends additively");
        assert_eq!(e.body_tint().map(|t| t.rgb), Some([255, 255, 255]), "flash is white");
        assert!(e.body_copies().is_none(), "no halo / glow copies");
        // Between flashes within a blink → normal body.
        step(&mut e, PULSE_FLASH_W); // into the off-half of the first blink
        assert!(!e.body_additive() && e.body_tint().is_none(), "normal between flashes");
        // Colour phase: a darkening multiply blue (not additive), fading out.
        let mut e = BodyTintEffect::new(CHEMICALBODY);
        step(&mut e, PULSE_COLOR_FULL); // colour fully in
        assert!(!e.body_additive(), "colour phase is a multiply, not additive");
        let full = e.body_tint().expect("blue tint").rgb;
        assert!(full[2] > full[0] && full[2] > full[1], "blue dominant");
        step(&mut e, (PULSE_TOTAL - PULSE_COLOR_FULL) * 0.6); // partway through the fade
        let faded = e.body_tint().expect("fading tint").rgb;
        assert!(faded[0] > full[0], "tint fades back toward normal (white)");
    }

    #[test]
    fn reject_sword_pairs_a_gray_flicker_with_the_sword_str() {
        // The hybrid: a gray body flicker (pulse) plus the `sword.str` world
        // overlay played for the effect's lifetime.
        let e = BodyTintEffect::new(REJECTSWORD).with_str_overlay("sword");
        assert_eq!(e.str_overlay(), Some("sword"), "plays the world STR");
        // First frame is the pulse's white additive flash, like the other
        // gray/colour flickers.
        assert!(e.body_additive());
        assert_eq!(e.body_tint().map(|t| t.rgb), Some([255, 255, 255]));
        // A pure body flash (no str) keeps str_overlay None.
        assert_eq!(BodyTintEffect::new(PIERCEBODY).str_overlay(), None);
    }

    #[test]
    fn pulse_family_ends_with_its_timeline() {
        let mut e = BodyTintEffect::new(MEMORIZE);
        assert_eq!(step(&mut e, PULSE_TOTAL + 1.0), EffectStatus::Dead, "dies at PULSE_TOTAL");
    }

    #[test]
    fn redhit_ramps_two_additive_red_copies_then_dies() {
        let mut e = BodyTintEffect::new(REDHIT);
        // No multiply tint / whole-body additive — the colour rides the overlays.
        assert_eq!(e.body_tint(), None);
        assert!(!e.body_additive());
        // Hold frame (glow clock ~15 at process 5) → two additive red copies.
        step(&mut e, 5.0);
        let hold = e.body_copies().expect("flashing");
        assert_eq!(hold.len(), 2, "drawn 2x additive");
        assert!(hold[0].additive && !hold[0].behind && hold[0].tint == [255, 5, 5]);
        let hold_alpha = hold[0].alpha;
        // Later in the fade the overlay is dimmer than at the hold.
        step(&mut e, 8.0); // process ~13, glow clock ~39 → fading
        let fade = e.body_copies().expect("still fading");
        assert!(fade[0].alpha < hold_alpha, "alpha fades after the hold");
        assert_eq!(step(&mut e, REDHIT.total_frames), EffectStatus::Dead);
    }

    #[test]
    fn madnessblue_strobes_solid_blue_then_dies() {
        let mut e = BodyTintEffect::new(MADNESSBLUE);
        // Frame 0 (% 4 == 0) → on: two additive blue copies, no tint/whole-body-additive.
        assert_eq!(e.body_tint(), None);
        assert!(!e.body_additive());
        let on = e.body_copies().expect("blink on");
        assert_eq!(on.len(), 2, "drawn 2x additive");
        assert!(on[0].additive && !on[0].behind && on[0].tint == [5, 5, 255]);
        // A frame between blinks → off.
        step(&mut e, 2.0);
        assert!(e.body_copies().is_none(), "off between blinks");
        assert_eq!(step(&mut e, 60.0), EffectStatus::Dead);
    }

    #[test]
    fn bluebody_is_slower_than_bluehit() {
        // Same elapsed frames: the slow clock (Bluebody) is dimmer than the fast
        // one (BlueHit), and both flash pure blue.
        let mut slow = BodyTintEffect::new(BLUEBODY);
        let mut fast = BodyTintEffect::new(BLUEHIT);
        step(&mut slow, 6.0);
        step(&mut fast, 6.0);
        let s = slow.body_copies().expect("blue glow");
        let f = fast.body_copies().expect("blue flash");
        assert_eq!(s[0].tint, [5, 5, 255]);
        assert_eq!(f[0].tint, [5, 5, 255]);
        assert!(s[0].alpha < f[0].alpha, "the slow clock ramps later");
    }
}
