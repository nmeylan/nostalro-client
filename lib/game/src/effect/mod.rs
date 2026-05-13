pub mod effect_queue;
pub mod generated;
pub mod spec;
pub mod str_aliases;
pub mod table;

pub use effect_queue::{EffectQueue, SpawnRequest};
pub use generated::{
    ALL_EFFECT_IDS, EffectId, classified_family, default_duration_ms, default_str_file,
    effect_ef_name, effect_name, str_file_override,
};
pub use spec::{
    Attach, CustomFamily, CustomFamilyParams, EffectBlend, EffectSpec, GroundRingParams,
};
pub use str_aliases::str_aliases;
pub use table::effect_spec;

/// Distinct GRF texture paths referenced by any `EffectSpec::Custom` in the
/// effect table. Empty-string textures are skipped. Used by the renderer to
/// preload effect textures at app boot so first-spawn doesn't hitch.
pub fn effect_texture_paths() -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    for &id in ALL_EFFECT_IDS {
        let Some(spec) = effect_spec(id) else {
            continue;
        };
        let EffectSpec::Custom { params, .. } = spec else {
            continue;
        };
        match params {
            CustomFamilyParams::GroundRing(p) if !p.texture.is_empty() => {
                seen.insert(format!("data/texture/effect/{}", p.texture));
            }
            _ => {}
        }
    }
    seen.into_iter().collect()
}
