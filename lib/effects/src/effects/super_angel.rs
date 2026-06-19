//! The Super Novice / Taekwon level-up
//! angel. An angel sprite descends above the caster, flaps its wings, sheds
//! feathers, then a blue eight-blade ring flashes out beneath it. Angel2 (338)
//! and Angel3 (582) differ only by the sprite layer set.
//!
//! Several sprite layers stack, one per layer index (`f1`):
//!   * Angel2 = `f1` {0,1,2,3} — angel body + wings + two feathers.
//!   * Angel3 = `f1` {11,10} — hanbok-angel wings + costume body.
//! The wing layers (`f1` 1 and 11) respawn every 3 frames through frame 18,
//! leaving a fan of progressively fainter after-images that reads as a flap.
//!
//! Each layer drifts up `0.1`/frame (`−Y` is up), fades in
//! on a per-layer curve, holds, then fades out, and its animation advances
//! once it has been alive 30 frames (every 3 frames for the angel/wings/feathers
//! to motion 29, every 4 for the costume/hanbok to motion 15). The first hanbok
//! wing is invisible (`alpha=0`); only its echoes show.
//!
//! At frame 65 the body layer (`f1` 0 or 10) launches a blue ring —
//! the same eight-blade burst as Kaizel — reused here via
//! [`super::slash::SlashEffect`] with the `ring_blue.tga` variant.
//!
//! Blending matters: every effect layer (body, wings, feathers, hanbok
//! wings) is **additive**, so the primary wing plus its echo fan saturate into
//! an opaque white cocoon that hides the angel until the wings spread. Only the
//! `f1` 10 costume body is alpha-blended; the render queue flushes the Alpha
//! bucket before the Additive one, so the additive hanbok wings always draw
//! over the costume — the Angel3 body sits behind its wings too.
//!
//! Sprites live under the Korean effect dir `data/sprite/이팩트/`; the
//! English names (`angel`, `angelwings`, …) map to `천사`, `천사날개`,
//! `천사날개깃털`, `한복천사(본체)`, `한복천사(날개)`. The classic GRF ships a
//! single feather sprite, so both feather layers share it.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

use super::slash::{SlashEffect, SUPERANGEL_RING};

const FRAMES_PER_SECOND: f32 = 60.0;

/// Angel body fades out from frame 100 at `−4/frame` from 245 → dies near
/// frame 161; the ring and other layers finish sooner. Pin the wall-clock end.
pub const TOTAL_DURATION_MS: u32 = 2700;
const TOTAL_FRAMES: f32 = TOTAL_DURATION_MS as f32 / 1000.0 * FRAMES_PER_SECOND;

/// The y-offsets (`−40` / `−30`) and the `0.1`/frame drift are
/// native-scale literals; downscale so the angel hovers ~one character-height
/// above the caster rather than 40 units up.
const WORLD_SCALE: f32 = 0.25;

/// Frame the body layer launches the blue ring.
const RING_SPAWN_FRAME: f32 = 65.0;

/// Wing layers respawn at these frames (every 3 frames through frame 18), each echo
/// dimmer than the last.
const WING_BIRTH_FRAMES: [f32; 7] = [0.0, 3.0, 6.0, 9.0, 12.0, 15.0, 18.0];

const SPR_ANGEL: &str = "data/sprite/이팩트/천사";
const SPR_ANGEL_WINGS: &str = "data/sprite/이팩트/천사날개";
const SPR_FEATHER: &str = "data/sprite/이팩트/천사날개깃털";
const SPR_HANBOK_BODY: &str = "data/sprite/이팩트/한복천사(본체)";
const SPR_HANBOK_WINGS: &str = "data/sprite/이팩트/한복천사(날개)";

pub const SPRITES: &[&str] = &[
    SPR_ANGEL,
    SPR_ANGEL_WINGS,
    SPR_FEATHER,
    SPR_HANBOK_BODY,
    SPR_HANBOK_WINGS,
];

/// One sprite layer of the composite, keyed by a layer index `f1` (which
/// drives every per-frame curve).
#[derive(Clone, Copy)]
pub struct AngelLayer {
    pub sprite: &'static str,
    pub f1: u8,
    /// Vertical offset in native units (`−40` body/wings/feathers, `−30`
    /// hanbok/costume).
    pub y_offset: f32,
    /// Wing layers respawn into a fan of fading echoes.
    pub wing: bool,
}

#[derive(Clone, Copy)]
pub struct SuperAngelParams {
    pub layers: &'static [AngelLayer],
}

// 338 Angel2 — angel body + flapping wings + two feathers, all lifted 40.
pub const ANGEL2: SuperAngelParams = SuperAngelParams {
    layers: &[
        AngelLayer { sprite: SPR_ANGEL, f1: 0, y_offset: -40.0, wing: false },
        AngelLayer { sprite: SPR_ANGEL_WINGS, f1: 1, y_offset: -40.0, wing: true },
        AngelLayer { sprite: SPR_FEATHER, f1: 2, y_offset: -40.0, wing: false },
        AngelLayer { sprite: SPR_FEATHER, f1: 3, y_offset: -40.0, wing: false },
    ],
};

// 582 Angel3 — hanbok-angel wings + costume body, lifted 30.
pub const ANGEL3: SuperAngelParams = SuperAngelParams {
    layers: &[
        AngelLayer { sprite: SPR_HANBOK_WINGS, f1: 11, y_offset: -30.0, wing: true },
        AngelLayer { sprite: SPR_HANBOK_BODY, f1: 10, y_offset: -30.0, wing: false },
    ],
};

/// One live sprite instance (a layer, or one wing echo).
struct Instance {
    sprite: &'static str,
    f1: u8,
    y_offset: f32,
    birth_frame: f32,
}

/// Alpha (0–255) for an instance, as a closed-form per-frame curve.
fn layer_alpha(f1: u8, birth_frame: f32, process: f32) -> f32 {
    if process <= 0.0 {
        return 0.0;
    }
    // Fade-in: cap + rate. Wing echoes (birth>0) ramp +1/frame to a low cap;
    // primaries follow their per-layer curve.
    let (cap, rate) = if birth_frame > 0.0 {
        ((40.0 - birth_frame).max(0.0), 1.0)
    } else {
        match f1 {
            0 => (245.0, 5.0),
            1 => (200.0, 5.0),
            10 => (255.0, 10.0),
            11 => (0.0, 0.0),
            _ => (150.0, 5.0), // feathers (2,3,4)
        }
    };
    let rise = (rate * process).min(cap);

    // Fade-out window + rate.
    let (fade_t, fade_rate) = if f1 == 1 || f1 == 11 {
        if birth_frame != 0.0 { (75.0, 3.0) } else { (75.0, 6.0) }
    } else {
        match f1 {
            0 => (100.0, 4.0),
            2 => (30.0, 3.0),
            3 => (60.0, 3.0),
            10 => (100.0, 6.0),
            _ => (70.0, 3.0),
        }
    };
    let fade = if process > fade_t { fade_rate * (process - fade_t) } else { 0.0 };
    (rise - fade).clamp(0.0, cap)
}

/// Effect layers are additive; only the `f1` 10 costume body is alpha-blended.
fn layer_blend(f1: u8) -> BlendKind {
    if f1 == 10 { BlendKind::Alpha } else { BlendKind::Additive }
}

/// Motion index within action 0: holds frame 0 for 30 frames, then advances.
fn layer_motion(f1: u8, process: f32) -> usize {
    if process <= 30.0 {
        return 0;
    }
    let steps = process - 30.0;
    if f1 == 10 || f1 == 11 {
        ((steps / 4.0) as usize).min(15)
    } else {
        ((steps / 3.0) as usize).min(29)
    }
}

pub struct SuperAngelEffect {
    anchor: [f32; 3],
    age_frames: f32,
    instances: Vec<Instance>,
    /// Frame the body layer fires the ring (`f1` 0 or 10 present).
    ring: Option<SlashEffect>,
    ring_spawned: bool,
}

impl SuperAngelEffect {
    pub fn new(anchor: [f32; 3], params: SuperAngelParams) -> Self {
        let mut instances = Vec::new();
        for layer in params.layers {
            if layer.wing {
                for &birth in &WING_BIRTH_FRAMES {
                    instances.push(Instance {
                        sprite: layer.sprite,
                        f1: layer.f1,
                        y_offset: layer.y_offset,
                        birth_frame: birth,
                    });
                }
            } else {
                instances.push(Instance {
                    sprite: layer.sprite,
                    f1: layer.f1,
                    y_offset: layer.y_offset,
                    birth_frame: 0.0,
                });
            }
        }
        Self { anchor, age_frames: 0.0, instances, ring: None, ring_spawned: false }
    }
}

impl Effect for SuperAngelEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if !self.ring_spawned && self.age_frames >= RING_SPAWN_FRAME {
            self.ring_spawned = true;
            self.ring = Some(SlashEffect::new(self.anchor, SUPERANGEL_RING));
        }
        if let Some(ring) = &mut self.ring {
            if ring.update(ctx) == EffectStatus::Dead {
                self.ring = None;
            }
        }
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        for inst in &self.instances {
            let process = self.age_frames - inst.birth_frame;
            let alpha = layer_alpha(inst.f1, inst.birth_frame, process);
            if alpha <= 0.0 {
                continue;
            }
            // Drift up over time (−Y is up), scaled to viewer units.
            let y = self.anchor[1] + (inst.y_offset - 0.1 * process) * WORLD_SCALE;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: inst.sprite,
                position: [self.anchor[0], y, self.anchor[2]],
                action_index: 0,
                motion_index: layer_motion(inst.f1, process),
                size_scale: 1.0,
                color: [1.0, 1.0, 1.0, alpha / 255.0],
                blend: layer_blend(inst.f1),
                aim_target: None,
                no_depth: false,
            });
        }
        if let Some(ring) = &self.ring {
            ring.collect_draws(out, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn step(e: &mut SuperAngelEffect, frames: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..frames {
            s = e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None });
        }
        s
    }

    fn sprites(e: &SuperAngelEffect) -> Vec<(&'static str, f32, usize, f32, BlendKind)> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &ctx());
        l.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { sprite_path, position, motion_index, color, blend, .. } => {
                    Some((*sprite_path, position[1], *motion_index, color[3], *blend))
                }
                _ => None,
            })
            .collect()
    }

    fn ring_quads(e: &SuperAngelEffect) -> usize {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &ctx());
        l.primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::WorldQuad { .. }))
            .count()
    }

    #[test]
    fn angel2_layers_present_with_wing_echo_fan() {
        let mut e = SuperAngelEffect::new([0.0, 0.0, 0.0], ANGEL2);
        // Past the wing-respawn window all seven wing echoes are alive alongside
        // the body and two feathers.
        step(&mut e, 20);
        let s = sprites(&e);
        let wings = s.iter().filter(|x| x.0 == SPR_ANGEL_WINGS).count();
        assert_eq!(wings, WING_BIRTH_FRAMES.len(), "wing fan of echoes");
        assert!(s.iter().any(|x| x.0 == SPR_ANGEL), "body present");
        assert!(s.iter().any(|x| x.0 == SPR_FEATHER), "feathers present");
        // Effect layers blend additively — the echo fan saturates into the
        // white cocoon that hides the body early.
        assert!(
            s.iter().all(|x| matches!(x.4, BlendKind::Additive)),
            "all Angel2 layers are additive"
        );
    }

    #[test]
    fn layers_fade_in_drift_up_and_animate() {
        let mut e = SuperAngelEffect::new([0.0, 0.0, 0.0], ANGEL2);
        step(&mut e, 5);
        let a_early = sprites(&e).iter().find(|x| x.0 == SPR_ANGEL).unwrap().1;
        let alpha_early = sprites(&e).iter().find(|x| x.0 == SPR_ANGEL).unwrap().3;
        let motion_early = sprites(&e).iter().find(|x| x.0 == SPR_ANGEL).unwrap().2;
        assert_eq!(motion_early, 0, "animation holds frame 0 for first 30 frames");

        step(&mut e, 45); // frame 50: faded in, drifted up, animating
        let body = *sprites(&e).iter().find(|x| x.0 == SPR_ANGEL).unwrap();
        assert!(body.3 > alpha_early, "angel fades in: {} -> {}", alpha_early, body.3);
        assert!(body.1 < a_early, "angel drifts up (−Y up): {} -> {}", a_early, body.1);
        assert!(body.2 > 0, "animation advances past frame 30");
    }

    #[test]
    fn ring_flashes_at_frame_65_then_clears() {
        let mut e = SuperAngelEffect::new([0.0, 0.0, 0.0], ANGEL2);
        step(&mut e, 60);
        assert_eq!(ring_quads(&e), 0, "no ring before frame 65");
        step(&mut e, 8); // ring spawned ~frame 65, blades alpha ramped in
        assert!(ring_quads(&e) > 0, "blue ring blades flash after frame 65");
    }

    #[test]
    fn angel3_uses_hanbok_layers_at_lower_offset() {
        let mut e = SuperAngelEffect::new([0.0, 10.0, 0.0], ANGEL3);
        step(&mut e, 20);
        let s = sprites(&e);
        // Hanbok body present; its first wing is invisible (alpha 0) so only the
        // echoes show.
        assert!(s.iter().any(|x| x.0 == SPR_HANBOK_BODY), "costume body present");
        let wings = s.iter().filter(|x| x.0 == SPR_HANBOK_WINGS).count();
        assert!(wings >= 1 && wings < WING_BIRTH_FRAMES.len(), "only dim wing echoes show, not the invisible primary");
        // Costume body is alpha-blended, wings additive — the Alpha
        // bucket flushes first, so the wings draw over the body.
        let body = s.iter().find(|x| x.0 == SPR_HANBOK_BODY).unwrap();
        assert!(matches!(body.4, BlendKind::Alpha), "costume body alpha-blended");
        assert!(
            s.iter().filter(|x| x.0 == SPR_HANBOK_WINGS).all(|x| matches!(x.4, BlendKind::Additive)),
            "hanbok wings additive"
        );
    }

    #[test]
    fn effect_self_terminates() {
        let mut e = SuperAngelEffect::new([0.0, 0.0, 0.0], ANGEL2);
        let status = step(&mut e, TOTAL_FRAMES as u32 + 1);
        assert_eq!(status, EffectStatus::Dead);
    }
}
