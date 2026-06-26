//! Categorizes every effect that the viewers' `ensure_str_loaded_for` would
//! ask the STR loader to open, so we can tell real backlog from dead entries.
//!
//! Run with: `cargo run -p ragnarok-effects --example categorize_str_loads`
//!
//! Buckets (an effect attempts an STR load only in these cases):
//!   * STR_BACKLOG  — `EffectSpec::Str`, not in `is_noop_bucket`: a genuine
//!     STR effect with no Rust renderer yet (or asset-blocked in classic GRF).
//!   * NOOP_TAGGED_WITH_ALIAS — `EffectSpec::Str`, but the id IS in
//!     `is_noop_bucket`, so `str_aliases` shadows the Noop classification
//!     (`bucket_default` checks str_aliases before is_noop_bucket). NOTE:
//!     `is_noop_bucket` membership is NOT a reliable "delete me" signal — some
//!     of these are real STR effects that are merely asset-blocked in the
//!     classic GRF (e.g. `EndureZhan/Sou/Shan/Jing`). Review each individually
//!     before touching `str_aliases.rs`.
//!   * HYBRID_OVERLAY — `EffectSpec::Custom` whose effect declares
//!     `str_overlay()` (intentional STR layered on a custom primitive).

use models::enums::EnumWithNumberValue;
use models::enums::effect_id::EffectId;

use ragnarok_effects::buckets::is_noop_bucket;
use ragnarok_effects::factory::make_effect;
use ragnarok_effects::spec::EffectAnchor;
use ragnarok_effects::{EffectSpec, effect_spec};

fn main() {
    let mut backlog: Vec<(EffectId, &'static str)> = Vec::new();
    let mut shadowed: Vec<(EffectId, &'static str)> = Vec::new();
    let mut hybrid: Vec<(EffectId, String)> = Vec::new();

    for v in 0..=2027usize {
        let Ok(id) = EffectId::try_from_value(v) else {
            continue;
        };
        match effect_spec(id) {
            Some(EffectSpec::Str { file, .. }) => {
                if is_noop_bucket(id) {
                    shadowed.push((id, file));
                } else {
                    backlog.push((id, file));
                }
            }
            Some(EffectSpec::Custom { .. }) => {
                if let Some(eff) =
                    make_effect(id, EffectAnchor::Point([0.0, 0.0, 0.0]), None, None, None)
                    && let Some(overlay) = eff.str_overlay()
                {
                    hybrid.push((id, overlay.to_string()));
                }
            }
            _ => {}
        }
    }

    print_group(
        "NOOP_TAGGED_WITH_ALIAS (review individually — some are real asset-blocked effects)",
        &shadowed,
    );
    print_group(
        "STR_BACKLOG (real STR effect, unimplemented / asset-missing)",
        &backlog,
    );
    print_group_owned(
        "HYBRID_OVERLAY (Custom effect intentionally plays an STR)",
        &hybrid,
    );

    eprintln!(
        "\nTOTAL str-load attempts: {} ({} noop-tagged-with-alias, {} backlog, {} hybrid)",
        shadowed.len() + backlog.len() + hybrid.len(),
        shadowed.len(),
        backlog.len(),
        hybrid.len()
    );
}

fn print_group(title: &str, rows: &[(EffectId, &'static str)]) {
    println!("\n=== {} ({}) ===", title, rows.len());
    for (id, name) in rows {
        println!("  {:>4}  {:<28}  {}", id.value(), format!("{id:?}"), name);
    }
}

fn print_group_owned(title: &str, rows: &[(EffectId, String)]) {
    println!("\n=== {} ({}) ===", title, rows.len());
    for (id, name) in rows {
        println!("  {:>4}  {:<28}  {}", id.value(), format!("{id:?}"), name);
    }
}
