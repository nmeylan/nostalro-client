//! Lv99 / Transcendant aura — persistent layered billboards around a char.
//!
//! The official game splits the aura into multiple cooperating effects fired by
//! the server at the same time:
//!   * `EF_LEVEL99` (top sparks), `EF_LEVEL99_2` (middle ring),
//!     `EF_LEVEL99_3` (bottom bubble) — the gold Lv99 aura.
//!   * `EF_LEVEL99_4` / `_5` / `_6` — Transcendant (white-blue) variant.
//!
//! Each EF_* is a *single layer*: one camera-facing textured billboard,
//! tinted, with a slow pulse. We model one billboard per effect so the
//! server can compose them just like the original game.
//!
//! When proper aura textures aren't packaged in the GRF, the renderer falls
//! back to a tinted white square — silhouette-correct, colour-correct, just
//! softer.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

/// Per-variant Aura parameters. Each EF_LEVEL99* maps to one [`AuraParams`].
#[derive(Clone, Copy, Debug)]
pub struct AuraParams {
    pub texture: &'static str,
    /// RGBA tint applied to the billboard (alpha gates how strong the layer
    /// is in the final composite).
    pub color: [f32; 4],
    /// Half-extent in world units (billboard's actual size is `2 * radius`).
    pub radius: f32,
    /// Vertical offset above the character's anchor, in world units.
    /// Negative = up in native RO coords.
    pub vertical_offset: f32,
    /// Slow pulse frequency in rad/s.
    pub pulse_freq: f32,
    /// Pulse depth (fraction of radius oscillation, 0..1).
    pub pulse_amplitude: f32,
}

/// `EF_LEVEL99` — top layer of the gold Lv99 aura. Largest billboard.
pub const LV99_LARGE: AuraParams = AuraParams {
    texture: "",
    color: [1.00, 0.85, 0.20, 0.35],
    radius: 13.0,
    vertical_offset: 0.0,
    pulse_freq: 4.4,
    pulse_amplitude: 0.06,
};

/// `EF_LEVEL99_2` — middle ring of the gold Lv99 aura.
pub const LV99_MIDDLE: AuraParams = AuraParams {
    texture: "",
    color: [1.00, 0.65, 0.10, 0.28],
    radius: 9.0,
    vertical_offset: -0.5,
    pulse_freq: 4.4,
    pulse_amplitude: 0.06,
};

/// `EF_LEVEL99_3` — bottom bubble of the gold Lv99 aura.
pub const LV99_BOTTOM: AuraParams = AuraParams {
    texture: "",
    color: [0.95, 0.45, 0.10, 0.22],
    radius: 6.0,
    vertical_offset: -1.0,
    pulse_freq: 4.4,
    pulse_amplitude: 0.05,
};

/// `EF_LEVEL99_4` — top layer of the Transcendant aura (blue-white).
pub const LV99_TRANSCENDANT: AuraParams = AuraParams {
    texture: "",
    color: [0.75, 0.90, 1.00, 0.35],
    radius: 14.0,
    vertical_offset: 0.0,
    pulse_freq: 3.6,
    pulse_amplitude: 0.07,
};

/// `EF_LEVEL99_5` — middle ring of the Transcendant aura.
pub const LV99_TRANSCENDANT_MIDDLE: AuraParams = AuraParams {
    texture: "",
    color: [0.55, 0.80, 1.00, 0.28],
    radius: 9.0,
    vertical_offset: -0.5,
    pulse_freq: 3.6,
    pulse_amplitude: 0.07,
};

/// `EF_LEVEL99_6` — bottom bubble of the Transcendant aura.
pub const LV99_TRANSCENDANT_BOTTOM: AuraParams = AuraParams {
    texture: "",
    color: [0.35, 0.70, 1.00, 0.22],
    radius: 6.0,
    vertical_offset: -1.0,
    pulse_freq: 3.6,
    pulse_amplitude: 0.06,
};

pub const TEXTURES: &[&str] = &[];

pub struct AuraEffect {
    params: AuraParams,
    world_pos: [f32; 3],
    age: f32,
}

impl AuraEffect {
    pub fn new(attach: Attach, params: AuraParams) -> Self {
        let world_pos = match attach {
            Attach::WorldPos(p) => p,
            Attach::Entity(_) | Attach::Projectile { .. } => [0.0; 3],
        };
        Self {
            params,
            world_pos,
            age: 0.0,
        }
    }
}

impl Effect for AuraEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        // Auras are persistent; lifetime is managed by the holder via the
        // duration table (Lv99 ids ship with u32::MAX → infinite).
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let pulse = 1.0 + self.params.pulse_amplitude * (self.age * self.params.pulse_freq).sin();
        let radius = self.params.radius * pulse;
        let pos = [
            self.world_pos[0],
            self.world_pos[1] + self.params.vertical_offset,
            self.world_pos[2],
        ];
        out.push(EffectPrimitiveDraw::Billboard {
            pos,
            size: [radius * 2.0, radius * 2.0],
            uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            texture: self.params.texture,
            color: self.params.color,
            blend: BlendKind::Additive,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn billboard_size(prim: &EffectPrimitiveDraw) -> f32 {
        match prim {
            EffectPrimitiveDraw::Billboard { size, .. } => size[0],
            _ => panic!("expected billboard"),
        }
    }

    #[test]
    fn emits_a_single_billboard_at_spawn_position() {
        let a = AuraEffect::new(Attach::WorldPos([3.0, -2.0, 5.0]), LV99_LARGE);
        let mut list = EffectDrawList::new();
        a.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.len(), 1);
        let EffectPrimitiveDraw::Billboard { pos, blend, .. } = &list.primitives[0] else {
            panic!("expected Billboard");
        };
        assert!((pos[0] - 3.0).abs() < 1e-4);
        assert!((pos[2] - 5.0).abs() < 1e-4);
        assert_eq!(*blend, BlendKind::Additive);
    }

    #[test]
    fn pulse_changes_size_over_time() {
        let mut a = AuraEffect::new(Attach::WorldPos([0.0; 3]), LV99_LARGE);
        let mut list = EffectDrawList::new();
        a.collect_draws(&mut list, &render_ctx());
        let s0 = billboard_size(&list.primitives[0]);
        list.clear();
        // Quarter-period of the pulse — guaranteed off-zero crossing.
        let dt = (std::f32::consts::FRAC_PI_2) / LV99_LARGE.pulse_freq;
        a.update(&EffectUpdateCtx { delta: dt });
        a.collect_draws(&mut list, &render_ctx());
        let s1 = billboard_size(&list.primitives[0]);
        assert!((s1 - s0).abs() > 1e-3, "size should oscillate: {s0} → {s1}");
    }

    #[test]
    fn variants_have_distinct_tints() {
        let layers = [
            LV99_LARGE.color,
            LV99_MIDDLE.color,
            LV99_BOTTOM.color,
            LV99_TRANSCENDANT.color,
            LV99_TRANSCENDANT_MIDDLE.color,
            LV99_TRANSCENDANT_BOTTOM.color,
        ];
        for i in 0..layers.len() {
            for j in (i + 1)..layers.len() {
                assert_ne!(layers[i], layers[j], "layers {i} and {j} share a tint");
            }
        }
    }

    #[test]
    fn aura_stays_running_indefinitely() {
        let mut a = AuraEffect::new(Attach::WorldPos([0.0; 3]), LV99_LARGE);
        for _ in 0..1000 {
            assert_eq!(a.update(&EffectUpdateCtx { delta: 0.1 }), EffectStatus::Running);
        }
    }
}
