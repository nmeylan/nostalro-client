/// RSW ambient effect kinds. Maps `RswObject::Effect.effect_type` to a
/// renderer dispatch.
///
/// Coverage is intentionally narrow: only the entries that appear as ambient
/// emitters in the maps the player visits. The skill/hit catalog (~1000
/// entries in the original game) is not worth carrying until skill effects
/// need it.
#[derive(Debug, Clone)]
pub enum EffectKind {
    /// Looping single-sprite billboard. Used for torches (type 47), poison
    /// pufs, etc. Sprite path is the GRF key without extension; renderer
    /// appends `.spr`/`.act`.
    Spr {
        sprite_path: &'static str,
        /// Looping animation period in milliseconds (one full cycle).
        duration_ms: f32,
    },
    /// 3D-positioned billboard with a finite lifetime, vertical drift, and
    /// fade-out. Used for chimney smoke (type 44), some fire effects.
    Smoke3D {
        sprite_path: &'static str,
        /// Lifetime per particle in milliseconds.
        duration_ms: f32,
        /// Pixel-space radius of the sprite at scale=1.
        size: f32,
        /// Z (vertical) offset where the particle is born, in world units
        /// (native RO coords: more negative = higher).
        pos_z_start: f32,
        /// Z offset reached at end of lifetime.
        pos_z_end: f32,
        /// Initial alpha, before fadeOut.
        alpha_max: f32,
        /// Random number of particles spawned in the initial burst (inclusive).
        burst_count_range: (u32, u32),
        /// Random per-particle vertical speed multiplier picked from this range.
        speed_range: (f32, f32),
        /// SPR animation frame interval in ticks (60 ticks/sec). Original game default = 4.
        anim_speed: f32,
    },
    /// Animated multi-layer STR effect (bubbles, fountain, gaspush, most
    /// spell visuals). Loaded from `data/texture/effect/<file>.str`. The
    /// `file_pattern` may contain `%d` which the renderer should replace
    /// with a value uniformly drawn from `rand_range[0]..=rand_range[1]`.
    ///
    /// **Renderer support is out of scope** - emitters created from this
    /// variant are skipped at draw time. Kept here so the data path
    /// (RSW → table → manager) is wired and can be exercised later when an
    /// STR animation pass is added.
    Str {
        file_pattern: &'static str,
        rand_range: Option<(u32, u32)>,
    },
}

/// Map an RSW effect type id to its kind. Returns `None` for ids the client
/// does not need to render - the manager ignores those, matching the
/// original game's behavior of silently skipping unimplemented effects.
pub fn effect_kind(effect_type: u32) -> Option<EffectKind> {
    match effect_type {
        // EF_SMOKE - chimney smoke
        44 => Some(EffectKind::Smoke3D {
            sprite_path: "data/sprite/이팩트/굴뚝연기",
            duration_ms: 833.0,
            size: 150.0,
            pos_z_start: 0.0,
            pos_z_end: 9.0,
            alpha_max: 1.0,
            burst_count_range: (1, 4),
            speed_range: (0.3, 0.5),
            anim_speed: 4.0,
        }),
        // EF_TORCH - torch flame
        47 => Some(EffectKind::Spr {
            sprite_path: "data/sprite/이팩트/torch_01",
            duration_ms: 250.0,
        }),
        // EF_BUBBLE - bybalan Blue bubbles (iz_dun04)
        109 => Some(EffectKind::Str {
            file_pattern: "bubble%d",
            rand_range: Some((1, 4)),
        }),
        // EF_GASPUSH - Giearth trap gas
        110 => Some(EffectKind::Str {
            file_pattern: "gaspush",
            rand_range: None,
        }),
        // EF_SPRINGTRAP
        111 => Some(EffectKind::Str {
            file_pattern: "spring",
            rand_range: None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_ambient_ids_resolve_and_unknown_ids_dont() {
        assert!(matches!(effect_kind(47), Some(EffectKind::Spr { .. })));
        assert!(matches!(effect_kind(44), Some(EffectKind::Smoke3D { .. })));
        assert!(matches!(effect_kind(109), Some(EffectKind::Str { .. })));
        assert!(effect_kind(0).is_none());
        assert!(effect_kind(9999).is_none());
    }
}
