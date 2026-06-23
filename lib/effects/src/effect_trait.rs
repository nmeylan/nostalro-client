//! Behavior contract for custom effects.
//!
//! Each effect under `effects/` implements [`Effect`]. The renderer crate
//! drives them through `EffectHolder`, calling `update` each frame and
//! `collect_draws` to gather the [`super::draw::EffectPrimitiveDraw`] entries
//! to render.

use super::draw::{EffectDrawList, EffectStatus};

/// Minimal renderer-agnostic camera snapshot. Effects that need orientation
/// (billboards, screen-space flashes) read this; full wgpu `Camera` stays in
/// the renderer.
#[derive(Clone, Copy, Debug, Default)]
pub struct CameraView {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
}


#[derive(Default, Clone, Copy)]
pub struct EffectUpdateCtx {
    pub delta: f32,
    /// Current camera target in world coordinates, when available. Used by
    /// effects whose spawn point should follow the active view (snow, rain,
    /// other camera-anchored ambient burst emitters). `None` for callers
    /// that don't track a camera (most tests).
    pub camera_target: Option<[f32; 3]>,
    /// World-space facing yaw (radians) of the caster this frame, for effects
    /// that orient by the caster's direction (the caster's body
    /// facing: AttackEnergy's comet, AttackEnergy2's rings, Guard's
    /// shell). `None` when the effect isn't entity-attached or the caster's
    /// facing can't be resolved — such effects fall back to a fixed front.
    /// Set per-effect by the holder from the attached entity's facing.
    pub caster_yaw: Option<f32>,
}

pub struct EffectRenderCtx {
    pub camera: CameraView,
    pub screen_w: f32,
    pub screen_h: f32,
    pub elapsed: f32,
}

/// Transient tint applied to the master sprite while an effect is active.
/// Matches the original game's body recolour while a buff is up
/// (PortalWind, GumGang, etc.). RGB only — alpha is always opaque
/// (the body's own opacity is left untouched).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyTint {
    pub rgb: [u8; 3],
}

/// One-shot screen-shake request — the original game's screen quake. An effect
/// returns this once (then `None`); a camera-shake controller owns the
/// per-frame decay and offsets the view, so the whole scene trembles. The
/// effect does not track the shake itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraShake {
    /// Peak displacement in world units, decaying linearly to zero.
    pub amplitude: f32,
    pub duration_ms: u32,
}

/// Movement afterimage ("blur") request — the original game's motion-blur
/// trail spawned by `EF_TWOHANDQUICKEN` / `EF_SPEARQUICKEN` / `EF_OVERTHRUST`
/// while the caster moves. The effect only declares *what* the trail looks
/// like; the actor pass drops a snapshot per displayed animation frame (it has
/// the sprite frame + world transform) and a controller decays each snapshot's
/// alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Afterimage {
    /// Tint applied to each snapshot, e.g. Quicken's `[200,200,0]`.
    pub tint: [u8; 3],
    /// Starting opacity of a fresh snapshot — `180/255`.
    pub start_alpha: f32,
    /// Opacity lost per 60 fps frame — 4 of 255 per frame,
    /// giving a ~0.75 s trail.
    pub fade_per_frame: f32,
}

/// One-shot forced actor animation — the original game's forced attack pose
/// (Jumpkick's kick pose). An effect returns this the *first* frame
/// it's armed, then `None` forever. The game-update step drains it and plays
/// the action on the entity, which then animates itself and reverts to its real
/// state action when finished (matching the original's revert to the attack pose).
/// Drained in `update_sprite_animation`, not the draw pass, because an action
/// change is stateful — the animation must advance frame by frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyAction {
    pub action_index: usize,
    pub start_frame: usize,
    pub duration_ms: f32,
}

/// Per-frame vertical body translate + fade + squeeze — the original game's
/// jump-up / land-down body lights (Jumpbody flies up the Y axis and vanishes,
/// Landbody drops in from above) and the pressed-flat body squash.
/// `lift_px` raises the actor's screen anchor (screen pixels, positive = up);
/// `alpha` (0..=1) multiplies the body opacity; `squeeze` (1.0 = none) is a
/// vertical scale about the feet anchor — `<1.0` presses the top toward the
/// bottom (Pressedbody). Multiple returning effects multiply `squeeze`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyVertical {
    pub lift_px: f32,
    pub alpha: f32,
    pub squeeze: f32,
}

/// One extra copy of the master sprite drawn behind the live one — the original
/// game's multi-render body lights (Asura halo, 4-way sliding ghosts,
/// blue hit flash, doubled-body stretch). `offset_px` shifts the
/// copy's anchor (screen pixels), `scale` stretches it about that anchor
/// (x, y), `tint` multiplies its RGB, `alpha` its opacity, and `additive`
/// selects additive blending (white blooms) vs alpha (ghosts).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyCopy {
    pub offset_px: [f32; 2],
    pub scale: [f32; 2],
    /// Grow the copy outward by this many **screen pixels on every edge**
    /// (an even margin). Use this — not [`scale`](BodyCopy::scale) —
    /// for a small concentric halo/ripple: `scale` adds a *proportional* margin
    /// that balloons on a tall sprite, whereas a few pixels is what the body
    /// lights actually do (the reflect-body ripple sweeps 0..20px). Applied on top of
    /// `scale`; `0.0` = none.
    pub margin_px: f32,
    pub tint: [u8; 3],
    pub alpha: f32,
    pub additive: bool,
    /// Draw this copy BEHIND the live body (`true`) vs ON TOP of it (`false`).
    /// Independent of [`additive`](BodyCopy::additive): an additive copy *behind*
    /// the (opaque) body only adds a soft glow at the margin and leaves the body
    /// untouched (Assumptio's doubled body); an additive copy *on top* blooms
    /// over the body (Asura halo, hit flash). Alpha ghosts usually go behind
    /// (4-way slide, reflect-body ripple).
    pub behind: bool,
}

/// One-shot floating-number request — the original game's recovery/regen
/// rising number drifting up off the actor. An effect that spawns a number
/// instead of a primitive returns this
/// the *first* frame it's ready, then `None`. The holder drains it keyed by the
/// attached entity; the client feeds it into the damage-number manager so the
/// number floats over that actor. RGB only — the floating number animates its
/// own alpha, so the source colour's alpha is irrelevant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberRequest {
    pub value: i32,
    pub color: [f32; 3],
}

pub trait Effect: Send {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus;
    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx);

    /// Feed live `(caster, partner)` world positions to an entity-linked
    /// effect (Linelink) before each `update`. The holder
    /// calls this for `Attach::Link` effects once both endpoints resolve;
    /// effects keep their spawn-time anchor until then (the effect viewer's
    /// static fake-entity path never calls it). Default no-op — only the
    /// link family overrides it.
    fn set_link_endpoints(&mut self, _caster: [f32; 3], _target: [f32; 3]) {}

    /// Re-anchor an entity-attached effect to its master's current world
    /// position. The holder calls this every frame for `Attach::Entity`
    /// effects (the original game re-copies `m_pos = m_master->m_pos` each
    /// frame, so auras/buffs/casting-circles follow the actor as it walks).
    /// Default no-op: one-shot bursts and hit sparks keep their spawn-time
    /// anchor, so existing entity-attached effects are unaffected. Persistent
    /// caster-anchored effects override this to move their stored origin and
    /// re-emit primitives relative to it.
    fn set_position(&mut self, _pos: [f32; 3]) {}

    /// STR animation that plays alongside this effect's primitives. Holder
    /// emits a `StrSnapshot` for non-`None` returns each frame, attached to
    /// the same world position. Default `None` — pure-primitive effects.
    fn str_overlay(&self) -> Option<&'static str> {
        None
    }

    /// Whether this buff shows the weapon-swing trail (`검광`) on the attached
    /// actor — the Quicken family (Two/One Hand, Spear, LK Concentration). The
    /// actor pass renders the per-weapon trail sprite while this is true.
    fn weapon_trail(&self) -> bool {
        false
    }

    /// Per-frame body tint to apply to the master sprite. Returns `Some`
    /// only during the effect's tint window (e.g. PortalWind's
    /// frames 5..=25). Default `None` — most effects do not tint.
    /// The renderer's actor pass is responsible for reading this and
    /// composing it with the sprite's base colour.
    fn body_tint(&self) -> Option<BodyTint> {
        None
    }

    /// Render the whole live body **additively** (the original game's
    /// glowing-body light): dark texture pixels add nothing so the
    /// background shows through (the see-through look), while lit pixels glow in
    /// the [`body_tint`](Effect::body_tint) colour. Returns `true` only while
    /// active. Default `false` — most tints are an opaque multiply.
    fn body_additive(&self) -> bool {
        false
    }

    /// One-shot SFX request — the effect returns the wave path the *first*
    /// time it's ready, then `None` on every subsequent call. The
    /// holder/audio bridge drains this once per frame and queues the sound.
    /// Path uses the backslash-separated form (e.g.
    /// `"effect\\windwalk.wav"`) so the lookup matches GRF / file-system
    /// naming. Default `None` — most effects don't trigger SFX.
    fn take_sfx_request(&mut self) -> Option<&'static str> {
        None
    }

    /// One-shot floating-number request — returned the *first* time it's ready,
    /// then `None`. The holder drains it once per frame keyed by the attached
    /// entity; the client emits a recoloured floating number on that actor.
    /// Default `None` — only the damage-number effects override it.
    fn take_number_request(&mut self) -> Option<NumberRequest> {
        None
    }

    /// One-shot screen-shake request (screen quake) — returned the *first* time
    /// it's ready, then `None`. The holder drains it once per frame and feeds
    /// a camera-shake controller. Default `None` — most effects don't shake
    /// the screen.
    fn take_camera_shake(&mut self) -> Option<CameraShake> {
        None
    }

    /// Per-frame **body shake** — a screen-space pixel offset to jitter the
    /// attached actor's sprite (the original game's body-quake light),
    /// distinct from the whole-screen [`take_camera_shake`]. Returns `Some`
    /// only during the effect's shake window. The client's actor pass adds
    /// this to the entity's screen anchor. Default `None`.
    fn body_shake(&self) -> Option<[f32; 2]> {
        None
    }

    /// Movement afterimage trail (motion blur). Returns `Some` while the trail
    /// should emit; the actor pass snapshots the moving sprite on the
    /// declared interval and renders the fading copies. Default `None`.
    fn body_afterimage(&self) -> Option<Afterimage> {
        None
    }

    /// Per-frame additive yaw offset (radians) applied to the master sprite's
    /// facing while the effect is active — the caster's body
    /// spin (StormKick). The actor pass adds this to
    /// the entity's facing angle before picking the 8-direction sprite frame,
    /// so the caster appears to whirl. Returns `Some` only during the spin
    /// window. Default `None` — most effects don't spin the caster.
    fn body_yaw(&self) -> Option<f32> {
        None
    }

    /// Per-frame uniform scale multiplier on the master sprite — the original
    /// game's body-enlarge light (Giant/Giant2). The
    /// actor pass multiplies the entity's `sprite_scale` by this before drawing
    /// the body. Returns `Some` only while the effect enlarges/shrinks the
    /// actor; `None` (the default) leaves the scale untouched. Multiple
    /// returning effects multiply.
    fn body_scale(&self) -> Option<f32> {
        None
    }

    /// One-shot forced actor animation ([`BodyAction`]) — returned the *first*
    /// frame it's armed, then `None`. The game-update step drains it (mutating)
    /// and force-plays the action on the attached entity. Default `None` — only
    /// action-driven body effects (Jumpkick) override it.
    fn take_body_action(&mut self) -> Option<BodyAction> {
        None
    }

    /// Per-frame vertical translate + fade ([`BodyVertical`]) applied to the
    /// master sprite (jump-up / land-down). The actor pass raises the
    /// screen anchor by `lift_px` and folds `alpha` into the body opacity.
    /// Default `None`.
    fn body_vertical(&self) -> Option<BodyVertical> {
        None
    }

    /// Per-frame rotation (radians) of the master sprite quad about its screen
    /// anchor — the original game's spinning-body barrel-roll. The actor pass
    /// rotates the built vertices. Multiple returning effects sum. Default
    /// `None` — most effects don't roll the actor.
    fn body_angle(&self) -> Option<f32> {
        None
    }

    /// Extra sprite copies ([`BodyCopy`]) drawn behind the live body this frame
    /// — the original game's multi-render body lights (Asura halo, 4-way slide,
    /// blue hit flash, doubled body). Default `None` — most effects draw no
    /// copies.
    fn body_copies(&self) -> Option<Vec<BodyCopy>> {
        None
    }
}
