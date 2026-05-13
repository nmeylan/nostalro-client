pub mod effect_queue;
pub mod generated;
pub mod spec;
pub mod table;

pub use effect_queue::{EffectQueue, SpawnRequest};
pub use generated::{
    ALL_EFFECT_IDS, EffectId, classified_family, default_duration_ms, default_str_file,
    effect_ef_name, effect_name, str_file_override,
};
pub use spec::{Attach, CustomFamily, EffectSpec};
pub use table::effect_spec;
