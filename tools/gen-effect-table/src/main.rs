//! One-shot generator: parses the EF_* enum and
//! the effect source (the duration values) and emits
//! `lib/game/src/effect/generated.rs`.
//!
//! The generated file exposes:
//!   * `pub enum EffectId { ... }`            — every effect variant
//!   * `pub fn default_duration_ms(id) -> u32` — derived from SET_DURATION
//!   * `pub const ALL_EFFECT_IDS: &[EffectId]` — for iteration / picker UIs
//!   * `pub fn from_u16(value) -> Option<EffectId>` — reverse lookup
//!
//! Discriminants match the implicit enum ordering (EF_HIT1 = 0,
//! EF_HIT2 = 1, ...). Centisecond durations from the original game are converted to
//! milliseconds. The special `EF_TEST` sentinel is skipped.
//!
//! Usage (from the workspace root):
//!
//! ```ignore
//! cargo run -p gen-effect-table -- \
//!     --effect-enum <effect-header> \
//!     --effect-source <effect-source> \
//!     --out lib/game/src/effect/generated.rs
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

const DEFAULT_H: &str = "../../../reference/effects-enum.txt";
const DEFAULT_CPP: &str = "../../../reference/effects-source.txt";
const DEFAULT_OUT: &str = "../../lib/game/src/effect/generated.rs";

fn main() -> ExitCode {
    let mut h_path = PathBuf::from(DEFAULT_H);
    let mut cpp_path = PathBuf::from(DEFAULT_CPP);
    let mut out_path = PathBuf::from(DEFAULT_OUT);

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--effect-enum" => h_path = args.next().expect("--effect-enum needs a path").into(),
            "--effect-source" => cpp_path = args.next().expect("--effect-source needs a path").into(),
            "--out" => out_path = args.next().expect("--out needs a path").into(),
            "--help" | "-h" => {
                println!("gen-effect-table — port the EF_* enum to Rust");
                println!("Usage:");
                println!("  cargo run -p gen-effect-table -- [--effect-enum PATH] [--effect-source PATH] [--out PATH]");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown argument: {other}");
                return ExitCode::FAILURE;
            }
        }
    }

    let h_bytes = match std::fs::read(&h_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("failed to read {}: {e}", h_path.display());
            return ExitCode::FAILURE;
        }
    };
    let cpp_bytes = match std::fs::read(&cpp_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("failed to read {}: {e}", cpp_path.display());
            return ExitCode::FAILURE;
        }
    };

    // The files are ISO-8859 / EUC-KR — we only care about ASCII identifiers
    // and digits, so a lossy UTF-8 view is sufficient.
    let h_text = String::from_utf8_lossy(&h_bytes).into_owned();
    let cpp_text = String::from_utf8_lossy(&cpp_bytes).into_owned();

    let entries = match parse_enum_entries(&h_text) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("parse effects-enum.txt: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "parsed {} EF_* enum variants (discriminants {}..={})",
        entries.len(),
        entries.first().map(|e| e.value).unwrap_or(0),
        entries.last().map(|e| e.value).unwrap_or(0),
    );

    let durations_cs = extract_set_durations(&cpp_text);
    eprintln!("parsed {} SET_DURATION entries", durations_cs.len());

    let output = emit_rust(&entries, &durations_cs);
    if let Err(e) = std::fs::write(&out_path, output) {
        eprintln!("failed to write {}: {e}", out_path.display());
        return ExitCode::FAILURE;
    }
    eprintln!("wrote {}", out_path.display());
    ExitCode::SUCCESS
}

/// One parsed enum entry: the source identifier and its resolved
/// discriminant. Order matches the source (which is also discriminant
/// order, since C enums increment monotonically within a section).
pub struct EnumEntry {
    pub name: String,
    pub value: i64,
}

#[cfg(test)]
fn extract_enum_identifiers(text: &str) -> Result<Vec<String>, String> {
    let entries = parse_enum_entries(text)?;
    Ok(entries.into_iter().map(|e| e.name).collect())
}

fn parse_enum_entries(text: &str) -> Result<Vec<EnumEntry>, String> {
    let start = text
        .find("enum")
        .ok_or_else(|| "no `enum` keyword found".to_string())?;
    let body_start = text[start..]
        .find('{')
        .ok_or_else(|| "no `{` after enum keyword".to_string())?
        + start
        + 1;

    // Walk until matching `}` while stripping comments. Keep commas + `=`
    // intact so we can split into per-entry items afterwards.
    let bytes = text.as_bytes();
    let mut i = body_start;
    let mut clean = String::new();
    let mut depth = 1usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        clean.push(b as char);
        i += 1;
    }
    if depth != 0 {
        return Err("unbalanced braces in enum body".to_string());
    }

    let mut entries: Vec<EnumEntry> = Vec::new();
    let mut name_to_value: HashMap<String, i64> = HashMap::new();
    let mut next_value: i64 = 0;

    for raw_item in clean.split(',') {
        let item = raw_item.trim();
        if item.is_empty() {
            continue;
        }
        // Item shape: `EF_NAME` or `EF_NAME = EXPR`.
        let (name, expr) = match item.split_once('=') {
            Some((n, e)) => (n.trim(), Some(e.trim())),
            None => (item, None),
        };
        if !name.starts_with("EF_") || !name.chars().all(is_ident_char) {
            // Non-EF identifier (defensive) — skip but don't bump value.
            continue;
        }
        if name == "EF_TEST" {
            continue;
        }
        let value = match expr {
            Some(e) => evaluate_expr(e, &name_to_value)
                .ok_or_else(|| format!("can't evaluate `{e}` for {name}"))?,
            None => next_value,
        };
        next_value = value + 1;

        // Aliases (same name twice) → keep the first; for the alias case
        // (`EF_NEW = EF_OLD` with a different name), we *do* add the new
        // name but it gets the same value as EF_OLD. Rust enums can't have
        // two variants with the same discriminant, so we skip same-value
        // duplicates after the first.
        if name_to_value.contains_key(name) {
            // Re-declaration of an existing name (alias to self). Ignore.
            continue;
        }
        if entries.iter().any(|e| e.value == value) {
            // Different name, same value as an existing entry → alias.
            // Skip so the generated Rust enum has unique discriminants.
            continue;
        }
        name_to_value.insert(name.to_string(), value);
        entries.push(EnumEntry {
            name: name.to_string(),
            value,
        });
    }
    Ok(entries)
}

fn evaluate_expr(expr: &str, names: &HashMap<String, i64>) -> Option<i64> {
    // Accept: integer literal, EF_NAME, EF_NAME [+|-] integer.
    let expr = expr.trim();
    if let Ok(n) = expr.parse::<i64>() {
        return Some(n);
    }
    // Try to split on +/- (last operator wins for simple expressions).
    let (lhs, op, rhs) = match expr.rfind(|c| c == '+' || c == '-') {
        Some(i) => {
            let op = expr.as_bytes()[i] as char;
            (expr[..i].trim(), Some(op), expr[i + 1..].trim())
        }
        None => (expr, None, ""),
    };
    let base = if lhs.starts_with("EF_") {
        *names.get(lhs)?
    } else {
        lhs.parse::<i64>().ok()?
    };
    match op {
        None => Some(base),
        Some('+') => Some(base + rhs.parse::<i64>().ok()?),
        Some('-') => Some(base - rhs.parse::<i64>().ok()?),
        _ => None,
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Returns the latest centisecond duration set for each `EF_*` name via
/// `SET_DURATION(time, EF_NAME)`. Multiple calls for the same name resolve
/// to the *last* one wins (matching the original game's runtime map behaviour).
fn extract_set_durations(text: &str) -> HashMap<String, u32> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let line = strip_line_comment(line);
        let Some(call_start) = line.find("SET_DURATION") else {
            continue;
        };
        let after = &line[call_start + "SET_DURATION".len()..];
        let Some(open) = after.find('(') else { continue };
        let Some(close_rel) = after[open + 1..].find(')') else {
            continue;
        };
        let args = &after[open + 1..open + 1 + close_rel];
        // args = "  50   ,   EF_HIT1  "
        let parts: Vec<&str> = args.split(',').collect();
        if parts.len() != 2 {
            continue;
        }
        let time_str = parts[0].trim();
        let name_str = parts[1].trim();
        if !name_str.starts_with("EF_") {
            continue;
        }
        let Ok(time) = time_str.parse::<u64>() else {
            continue;
        };
        // Clamp huge sentinels (`99999999`) to u32::MAX → "persistent".
        let ms = if time >= 1_000_000 {
            u32::MAX
        } else {
            // source stores centiseconds; convert to ms.
            (time * 10).min(u32::MAX as u64) as u32
        };
        out.insert(name_str.to_string(), ms);
    }
    out
}

fn strip_line_comment(line: &str) -> &str {
    if let Some(idx) = line.find("//") {
        &line[..idx]
    } else {
        line
    }
}

/// Convert `EF_FIRE_BOLT` → `FireBolt`, `EF_HIT1` → `Hit1`, `EF_LVUP` → `Lvup`.
/// Names that would start with a digit after stripping `EF_` get an `Ef`
/// prefix (`EF_4WAYBODY` → `Ef4Waybody`) since Rust identifiers can't lead
/// with a digit.
fn to_pascal_case(ef_name: &str) -> String {
    let stripped = ef_name.strip_prefix("EF_").unwrap_or(ef_name);
    let mut out = String::with_capacity(stripped.len());
    let mut capitalize_next = true;
    for c in stripped.chars() {
        if c == '_' {
            capitalize_next = true;
            continue;
        }
        if capitalize_next {
            out.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            out.extend(c.to_lowercase());
        }
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("Ef{out}")
    } else {
        out
    }
}

fn emit_rust(entries: &[EnumEntry], durations: &HashMap<String, u32>) -> String {
    let mut s = String::new();
    s.push_str("//! GENERATED by `cargo run -p gen-effect-table`. Do not edit by hand.\n");
    s.push_str("//! Source: reference/effects-enum.txt (EF_* enum) + reference/effects-source.txt (SET_DURATION).\n");
    s.push_str("\n");
    s.push_str("#![allow(non_camel_case_types, clippy::enum_clike_unportable_variant)]\n");
    s.push_str("\n");
    s.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\n");
    s.push_str("#[repr(u16)]\n");
    s.push_str("pub enum EffectId {\n");

    // (pascal_name, ef_original_name, value, ms)
    let mut variants: Vec<(String, String, i64, u32)> = Vec::with_capacity(entries.len());
    let mut pascal_seen: HashMap<String, usize> = HashMap::new();
    for entry in entries {
        let mut pascal = to_pascal_case(&entry.name);
        let counter = pascal_seen.entry(pascal.clone()).or_insert(0);
        *counter += 1;
        if *counter > 1 {
            pascal = format!("{pascal}_{}", *counter);
        }
        let ms = durations.get(&entry.name).copied().unwrap_or(0);
        variants.push((pascal, entry.name.clone(), entry.value, ms));
    }

    for (pascal, ef_orig, value, _) in &variants {
        s.push_str(&format!("    /// `{}`\n", ef_orig));
        s.push_str(&format!("    {} = {},\n", pascal, value));
    }
    s.push_str("}\n\n");

    s.push_str("impl EffectId {\n");
    s.push_str("    pub fn as_u16(self) -> u16 { self as u16 }\n\n");

    s.push_str("    pub fn from_u16(value: u16) -> Option<EffectId> {\n");
    s.push_str("        Some(match value {\n");
    for (pascal, _, value, _) in &variants {
        s.push_str(&format!("            {} => EffectId::{},\n", value, pascal));
    }
    s.push_str("            _ => return None,\n");
    s.push_str("        })\n");
    s.push_str("    }\n");
    s.push_str("}\n\n");

    s.push_str("/// Default duration in milliseconds for an effect, from the original runtime\n");
    s.push_str("/// `durationTable`. `u32::MAX` means \"persistent\" (e.g. auras, walls).\n");
    s.push_str("pub fn default_duration_ms(id: EffectId) -> u32 {\n");
    s.push_str("    match id {\n");
    for (pascal, _, _, ms) in &variants {
        s.push_str(&format!("        EffectId::{} => {},\n", pascal, ms));
    }
    s.push_str("    }\n");
    s.push_str("}\n\n");

    s.push_str("pub const ALL_EFFECT_IDS: &[EffectId] = &[\n");
    for (pascal, _, _, _) in &variants {
        s.push_str(&format!("    EffectId::{},\n", pascal));
    }
    s.push_str("];\n\n");

    // Display name (Pascal): matches the variant identifier. Useful for
    // picker UIs that want a stable, ASCII-only label.
    s.push_str("/// Pascal-case display name (matches the Rust variant identifier).\n");
    s.push_str("pub fn effect_name(id: EffectId) -> &'static str {\n");
    s.push_str("    match id {\n");
    for (pascal, _, _, _) in &variants {
        s.push_str(&format!("        EffectId::{} => \"{}\",\n", pascal, pascal));
    }
    s.push_str("    }\n");
    s.push_str("}\n\n");

    // Original EF_* identifier — handy for cross-referencing.
    s.push_str("/// Original `EF_*` identifier.\n");
    s.push_str("pub fn effect_ef_name(id: EffectId) -> &'static str {\n");
    s.push_str("    match id {\n");
    for (pascal, ef_orig, _, _) in &variants {
        s.push_str(&format!(
            "        EffectId::{} => \"{}\",\n",
            pascal, ef_orig
        ));
    }
    s.push_str("    }\n");
    s.push_str("}\n\n");

    // Default STR filename. Convention: lowercase EF_ name with the `EF_`
    // prefix stripped. Many of the original game's STR effects do follow this naming
    // (e.g. EF_BUBBLE → bubble.str); the rest fall back gracefully —
    // StrEffectCache::load logs a warning and the effect simply doesn't
    // render. The hand-curated overrides in `table.rs` fix the cases that
    // need a non-trivial mapping.
    s.push_str("/// Default STR filename (without `.str`) — lowercase EF_ name.\n");
    s.push_str("pub fn default_str_file(id: EffectId) -> &'static str {\n");
    s.push_str("    match id {\n");
    for (pascal, ef_orig, _, _) in &variants {
        let default = ef_orig
            .strip_prefix("EF_")
            .unwrap_or(ef_orig)
            .to_ascii_lowercase();
        s.push_str(&format!(
            "        EffectId::{} => \"{}\",\n",
            pascal, default
        ));
    }
    s.push_str("    }\n");
    s.push_str("}\n");

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_case_basics() {
        assert_eq!(to_pascal_case("EF_HIT1"), "Hit1");
        assert_eq!(to_pascal_case("EF_FIRE_BOLT"), "FireBolt");
        assert_eq!(to_pascal_case("EF_LVUP"), "Lvup");
        assert_eq!(to_pascal_case("EF_LEVEL99_2"), "Level992");
    }

    #[test]
    fn pascal_case_prefixes_digit_leading_names() {
        assert_eq!(to_pascal_case("EF_4WAYBODY"), "Ef4waybody");
        assert_eq!(to_pascal_case("EF_05VAL"), "Ef05val");
    }

    #[test]
    fn extract_enum_skips_test_sentinel_and_dedupes() {
        let h = r#"
            #include "x.h"
            enum {
                EF_HIT1,
                EF_HIT2,
                EF_FOO,
                /* block */ EF_BAR,
                EF_TEST,
            };
        "#;
        let names = extract_enum_identifiers(h).unwrap();
        assert_eq!(names, vec!["EF_HIT1", "EF_HIT2", "EF_FOO", "EF_BAR"]);
    }

    #[test]
    fn extract_enum_handles_explicit_values() {
        let h = r#"
            enum {
                EF_HIT1,
                EF_HIT2,
                EF_FOO,
                EF_SECTION_BEGIN = 1000,
                EF_BAR,
                EF_BAZ = EF_BAR + 5,
                EF_TEST,
            };
        "#;
        let entries = parse_enum_entries(h).unwrap();
        let by_name: std::collections::HashMap<&str, i64> = entries
            .iter()
            .map(|e| (e.name.as_str(), e.value))
            .collect();
        assert_eq!(by_name["EF_HIT1"], 0);
        assert_eq!(by_name["EF_HIT2"], 1);
        assert_eq!(by_name["EF_FOO"], 2);
        assert_eq!(by_name["EF_SECTION_BEGIN"], 1000);
        assert_eq!(by_name["EF_BAR"], 1001);
        assert_eq!(by_name["EF_BAZ"], 1006);
    }

    #[test]
    fn aliases_with_duplicate_values_are_skipped() {
        let h = r#"
            enum {
                EF_A,
                EF_B,
                EF_ALIAS = EF_A,
            };
        "#;
        let entries = parse_enum_entries(h).unwrap();
        // EF_ALIAS would share discriminant 0 with EF_A → dropped.
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["EF_A", "EF_B"]);
    }

    #[test]
    fn extract_durations_picks_last_value() {
        let cpp = r#"
            SET_DURATION( 50,  EF_HIT1 );
            SET_DURATION( 80,  EF_HIT1 );  // overrides
            SET_DURATION( 100, EF_BASH );
            // SET_DURATION( 999, EF_BASH );  -- commented out, ignored
            SET_DURATION( 99999999, EF_AURA );
        "#;
        let d = extract_set_durations(cpp);
        assert_eq!(d.get("EF_HIT1"), Some(&800)); // 80 cs → 800 ms
        assert_eq!(d.get("EF_BASH"), Some(&1000));
        assert_eq!(d.get("EF_AURA"), Some(&u32::MAX));
    }
}
