use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::process::ExitCode;

use ragnarok_game::effect::{ALL_EFFECT_IDS, effect_ef_name, str_file_override};

/// EF name → suggested `CustomFamily` variant name (e.g. "RadialBurst").
/// `None` for effects with no detected dispatched primitive. The generator
/// combines this with `str_file_override` to emit a full default EffectSpec.
pub fn classify_efs(
    effects_source: &Path,
    effects_source2: &Path,
) -> std::collections::HashMap<String, &'static str> {
    let cpp1 = read_lossy(effects_source).unwrap_or_default();
    let cpp2 = read_lossy(effects_source2).unwrap_or_default();
    let init_switch_range = locate_init_switch(&cpp1);
    let methods = collect_methods(&cpp1, &cpp2);
    let dispatches = collect_dispatches(&cpp1, init_switch_range);
    let mut out = std::collections::HashMap::new();
    for (ef_name, called_methods) in &dispatches {
        let mut prims: BTreeSet<String> = BTreeSet::new();
        for m in called_methods {
            if let Some(body) = methods.get(m) {
                for p in extract_primitives(body) {
                    prims.insert(p);
                }
            }
        }
        if let Some(family) = suggest_family(&prims) {
            out.insert(ef_name.clone(), family);
        }
    }
    out
}

pub fn run(effects_source: &Path, effects_source2: &Path) -> ExitCode {
    let cpp1 = match read_lossy(effects_source) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("read {}: {e}", effects_source.display());
            return ExitCode::FAILURE;
        }
    };
    let cpp2 = match read_lossy(effects_source2) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("read {}: {e}", effects_source2.display());
            return ExitCode::FAILURE;
        }
    };

    let init_switch_range = locate_init_switch(&cpp1);
    let methods = collect_methods(&cpp1, &cpp2);
    let dispatches = collect_dispatches(&cpp1, init_switch_range);
    eprintln!(
        "parsed {} Effect methods, {} dispatched EF_ entries",
        methods.len(),
        dispatches.len()
    );

    let mut by_ef: BTreeMap<String, EfRecord> = BTreeMap::new();
    for (ef_name, called_methods) in &dispatches {
        let mut prims: BTreeSet<String> = BTreeSet::new();
        for m in called_methods {
            if let Some(body) = methods.get(m) {
                for p in extract_primitives(body) {
                    prims.insert(p);
                }
            }
        }
        by_ef.insert(
            ef_name.clone(),
            EfRecord {
                methods: called_methods.iter().cloned().collect(),
                primitives: prims,
            },
        );
    }

    let mut primitive_counts: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut suggested_family: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    let mut hybrid_records: Vec<(&str, &'static str)> = Vec::new();
    let mut hybrid_family_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut none_efs: Vec<&str> = Vec::new();
    let mut total_custom = 0usize;
    let mut total_pure_str = 0usize;

    for id in ALL_EFFECT_IDS {
        let ef = effect_ef_name(*id);
        let has_str_in_init = str_file_override(*id).is_some();
        let rec = by_ef.get(ef);
        match (has_str_in_init, rec) {
            (true, Some(r)) if !r.primitives.is_empty() => {
                let family = suggest_family(&r.primitives).unwrap_or("Bespoke");
                hybrid_records.push((ef, family));
                *hybrid_family_counts.entry(family).or_insert(0) += 1;
            }
            (true, _) => {
                total_pure_str += 1;
            }
            (false, Some(r)) if !r.primitives.is_empty() => {
                total_custom += 1;
                for p in &r.primitives {
                    primitive_counts.entry(p.clone()).or_default().push(ef.into());
                }
                if let Some(family) = suggest_family(&r.primitives) {
                    suggested_family.entry(family).or_default().push(ef.into());
                }
            }
            (false, _) => none_efs.push(ef),
        }
    }

    println!("# Effect classification\n");
    println!("Sources:\n- `{}`\n- `{}`\n", effects_source.display(), effects_source2.display());
    println!("Classification rule:");
    println!("- `Str` - sprintf in Init switch, no dispatch primitives");
    println!("- `StrHybrid(family)` - sprintf in Init switch AND dispatch emits PP_* primitives");
    println!("- `Custom(family)` - no sprintf in Init, dispatch emits PP_*");
    println!("- `None` - no sprintf, no dispatch primitives (probably pass-through / no-op)\n");
    println!("| Bucket | Count |");
    println!("|---|---:|");
    println!("| Str (pure) | {} |", total_pure_str);
    println!("| StrHybrid | {} |", hybrid_records.len());
    println!("| Custom | {} |", total_custom);
    println!("| None | {} |", none_efs.len());

    println!("\n## Primitive frequency (custom-only)\n");
    println!("| Primitive | Count |");
    println!("|---|---:|");
    let mut sorted: Vec<_> = primitive_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    for (p, list) in &sorted {
        println!("| {} | {} |", p, list.len());
    }

    println!("\n## Suggested family groupings (custom-only)\n");
    println!("| Suggested family | EF count | EF examples |");
    println!("|---|---:|---|");
    let mut family_sorted: Vec<_> = suggested_family.iter().collect();
    family_sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    for (family, efs) in &family_sorted {
        let sample = efs.iter().take(6).cloned().collect::<Vec<_>>().join(", ");
        println!("| **{}** | {} | {} |", family, efs.len(), sample);
    }

    println!("\n## StrHybrid breakdown\n");
    println!("These play an STR file *and* run a custom-primitive layer. Rendering both is needed for visual parity with the original game.\n");
    println!("| EF | STR-implied family overlay |");
    println!("|---|---|");
    for (ef, family) in &hybrid_records {
        println!("| {} | {} |", ef, family);
    }
    println!("\nFamily distribution among hybrids:\n");
    println!("| Family | Count |");
    println!("|---|---:|");
    let mut sorted_hybrid: Vec<_> = hybrid_family_counts.iter().collect();
    sorted_hybrid.sort_by(|a, b| b.1.cmp(a.1));
    for (family, count) in &sorted_hybrid {
        println!("| {} | {} |", family, count);
    }

    println!("\n## None bucket (no STR, no primitive dispatch)\n");
    println!("These EF_ ids have neither a sprintf in Init nor a dispatched method with PP_* primitives. They are pass-throughs, status markers, or pure-data effects.\n");
    println!("- Total: **{}**", none_efs.len());
    println!("- First 50: {}", none_efs.iter().take(50).copied().collect::<Vec<_>>().join(", "));

    println!("\n## Full per-effect classification (custom only - no STR)\n");
    println!("| EF | Methods | Primitives | Suggested family |");
    println!("|---|---|---|---|");
    for (ef, rec) in &by_ef {
        if rec.primitives.is_empty() {
            continue;
        }
        // Skip hybrids and STR effects.
        let id = ALL_EFFECT_IDS.iter().find(|i| effect_ef_name(**i) == ef.as_str());
        if let Some(id) = id {
            if str_file_override(*id).is_some() {
                continue;
            }
        }
        let family = suggest_family(&rec.primitives).unwrap_or("-");
        println!(
            "| {} | {} | {} | {} |",
            ef,
            rec.methods.join(", "),
            rec.primitives.iter().cloned().collect::<Vec<_>>().join(", "),
            family
        );
    }

    ExitCode::SUCCESS
}

struct EfRecord {
    methods: Vec<String>,
    primitives: BTreeSet<String>,
}

fn read_lossy(p: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(p)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn locate_init_switch(cpp: &str) -> (usize, usize) {
    let Some(init) = cpp.find("Effect :: Init(") else {
        return (0, 0);
    };
    let Some(brace_rel) = cpp[init..].find('{') else {
        return (0, 0);
    };
    let start = init + brace_rel;
    let end = find_matching(cpp, start, b'{', b'}').unwrap_or(cpp.len() - 1);
    (start, end + 1)
}

fn collect_methods(cpp1: &str, cpp2: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for cpp in [cpp1, cpp2] {
        for (name, body) in find_methods(cpp) {
            out.entry(name).or_insert(body);
        }
    }
    out
}

fn find_methods(cpp: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let needle = "Effect :: ";
    let mut search_from = 0;
    while let Some(rel) = cpp[search_from..].find(needle) {
        let start = search_from + rel + needle.len();
        let end_name = start
            + cpp[start..]
                .find(|c: char| !is_ident_char(c))
                .unwrap_or(0);
        if end_name == start {
            search_from = start + 1;
            continue;
        }
        let name = cpp[start..end_name].to_string();
        let after = match cpp[end_name..].find('(') {
            Some(p) => end_name + p,
            None => {
                search_from = end_name;
                continue;
            }
        };
        let after_close = match find_matching(cpp, after, b'(', b')') {
            Some(i) => i + 1,
            None => {
                search_from = end_name;
                continue;
            }
        };
        let body_open = match cpp[after_close..].find('{') {
            Some(p) => after_close + p,
            None => {
                search_from = after_close;
                continue;
            }
        };
        match find_matching(cpp, body_open, b'{', b'}') {
            Some(close) => {
                out.push((name, cpp[body_open..=close].to_string()));
                search_from = close + 1;
            }
            None => {
                search_from = body_open + 1;
            }
        }
    }
    out
}

fn find_matching(cpp: &str, start: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = cpp.as_bytes();
    if bytes.get(start) != Some(&open) {
        return None;
    }
    let mut depth = 0i32;
    let mut i = start;
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
        if b == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            i += 1;
            continue;
        }
        if b == b'\'' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'\'' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            i += 1;
            continue;
        }
        if b == open {
            depth += 1;
        } else if b == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn collect_dispatches(cpp: &str, init_range: (usize, usize)) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let needle = "case";
    let mut from = 0;
    while let Some(rel) = cpp[from..].find(needle) {
        let idx = from + rel;
        if idx >= init_range.0 && idx < init_range.1 {
            from = idx + needle.len();
            continue;
        }
        let after = &cpp[idx + needle.len()..];
        let after_trimmed = after.trim_start();
        if !after_trimmed.starts_with("EF_") {
            from = idx + needle.len();
            continue;
        }
        let name_end = after_trimmed
            .find(|c: char| !is_ident_char(c))
            .unwrap_or(after_trimmed.len());
        let ef_name = after_trimmed[..name_end].to_string();
        let rest = &after_trimmed[name_end..];
        let Some(colon) = rest.find(':') else {
            from = idx + needle.len();
            continue;
        };
        let body_start_abs = idx + needle.len() + (after.len() - after_trimmed.len()) + name_end + colon + 1;
        let body_end = find_case_end(cpp, body_start_abs);
        let body = &cpp[body_start_abs..body_end];
        let methods = extract_method_calls(body);
        out.push((ef_name, methods));
        from = body_end;
    }
    out
}

fn find_case_end(cpp: &str, start: usize) -> usize {
    let bytes = cpp.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        if starts_with_bytes(bytes, i, b"break") {
            let mut j = i + 5;
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b';' {
                return j + 1;
            }
        }
        if starts_with_bytes(bytes, i, b"default:") {
            return i;
        }
        if starts_with_bytes(bytes, i, b"case") && i > start {
            let next = bytes.get(i + 4).copied();
            if matches!(next, Some(b' ' | b'\t' | b'\n')) {
                return i;
            }
        }
        i += 1;
    }
    bytes.len()
}

fn starts_with_bytes(bytes: &[u8], i: usize, needle: &[u8]) -> bool {
    if i + needle.len() > bytes.len() {
        return false;
    }
    &bytes[i..i + needle.len()] == needle
}

fn extract_method_calls(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < bytes.len() && is_ident_char(bytes[i] as char) {
                i += 1;
            }
            let name = &body[start..i];
            let after = body[i..].trim_start_matches(|c: char| c == ' ' || c == '\t');
            if after.starts_with('(') && !is_keyword(name) {
                out.push(name.to_string());
            }
            continue;
        }
        i += 1;
    }
    let mut dedup: Vec<String> = Vec::new();
    for n in out {
        if !dedup.contains(&n) {
            dedup.push(n);
        }
    }
    dedup
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "if" | "while" | "for" | "switch" | "return" | "sizeof" | "case" | "do" | "else"
            | "sprintf" | "memset" | "memcpy" | "random" | "strcpy" | "strlen" | "PlayWave"
            | "GetRadian" | "GetSin" | "GetCos" | "sAssert" | "CalcDist" | "CalcDir"
    )
}

fn extract_primitives(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = body[from..].find("LaunchEffectPrim(") {
        let start = from + rel + "LaunchEffectPrim(".len();
        let rest = &body[start..];
        let end = rest
            .find(|c: char| !is_ident_char(c))
            .unwrap_or(rest.len());
        let name = &rest[..end];
        if name.starts_with("PP_") && !out.iter().any(|n: &String| n == name) {
            out.push(name.to_string());
        }
        from = start + end;
    }
    out
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn suggest_family(prims: &BTreeSet<String>) -> Option<&'static str> {
    if prims.is_empty() {
        return None;
    }
    let any = |needles: &[&str]| -> bool {
        prims.iter().any(|p| needles.iter().any(|n| p == n))
    };
    let contains = |needle: &str| -> bool {
        prims.iter().any(|p| p.contains(needle))
    };
    if any(&[
        "PP_3DCASTING",
        "PP_3DCASTING_2",
        "PP_3DCASTING_3",
        "PP_3DCASTING_4",
        "PP_3DCASTING_5",
        "PP_ASURACASTING",
        "PP_HEARTCASTING",
    ]) {
        return Some("CastCircle");
    }
    if any(&["PP_3DAURA", "PP_3DAURA_2", "PP_3DAURA_3", "PP_CARTTER"]) {
        return Some("Aura");
    }
    if any(&[
        "PP_3DCYLINDER",
        "PP_GUMGANG",
        "PP_DOUBLEGUMGANG",
        "PP_MAPPILLAR",
        "PP_PILLAR",
    ]) {
        return Some("CylinderPillar");
    }
    if any(&["PP_3DCROSSTEXTURE"]) {
        return Some("CrossBeam");
    }
    if any(&["PP_3DQUADHORN", "PP_3DQUADHORN2"]) {
        return Some("SpikeRow");
    }
    if any(&["PP_3DPARTICLESPLINE", "PP_LINELINK", "PP_BOWLINGBASH"]) {
        return Some("SplineProjectile");
    }
    if any(&["PP_2DFLASH", "PP_GROUNDSHAKE"]) {
        return Some("ScreenFlash");
    }
    if any(&[
        "PP_HEAL",
        "PP_PARTICLE_UP",
        "PP_FIRSTAID",
        "PP_PEONGUP",
        "PP_GLORIA",
        "PP_RESURRECTION",
        "PP_RECT_UP",
        "PP_RECT_UP2",
    ]) {
        return Some("HealBurst");
    }
    if any(&[
        "PP_WIND",
        "PP_CLOUD",
        "PP_AIRTEXTURE",
        "PP_SPHEREWIND2",
        "PP_TWILIGHT",
    ]) {
        return Some("AirSwirl");
    }
    if any(&["PP_WATERFALL", "PP_WATERFALLPARTICLE", "PP_BUBBLE_DROP"]) {
        return Some("Waterfall");
    }
    if any(&[
        "PP_BASH3D",
        "PP_TANJI2",
        "PP_TANJI3",
        "PP_TEIHIT1",
        "PP_TEIHIT2",
        "PP_TEIHIT3",
        "PP_SLASH1",
        "PP_TRIPLEATTACK",
        "PP_SHIELDBOOMERANG",
    ]) {
        return Some("MeleeImpact");
    }
    if any(&[
        "PP_BLIND",
        "PP_POISON",
        "PP_BLEEDING",
        "PP_CHEMICAL2",
        "PP_CHEMICAL3",
        "PP_CHEMICALPROTECTION",
        "PP_DEFENDER",
        "PP_REFLECTSHIELD",
    ]) {
        return Some("StatusOrb");
    }
    if any(&[
        "PP_GHOST",
        "PP_BAT",
        "PP_FOREST",
        "PP_FOOT",
    ]) {
        return Some("FloatingSpirit");
    }
    if contains("2DCIRCLE") || contains("3DCIRCLE") || any(&["PP_LANDPROTECTOR", "PP_HERMODE"]) {
        return Some("GroundRing");
    }
    if any(&[
        "PP_BOTTOM",
        "PP_BOTTOM2",
        "PP_BOTTOMOUT",
        "PP_BOTTOMSPR",
        "PP_MAPMAGICZONE",
    ]) {
        return Some("GroundRing");
    }
    if any(&[
        "PP_3DPARTICLE",
        "PP_3DPARTICLEGRAVITY",
        "PP_3DPARTICLE_NOMASTER",
        "PP_3DPARTICLEORBIT",
        "PP_GI_2",
        "PP_GI_4",
        "PP_COLORPAPER",
        "PP_THROWITEM",
    ]) {
        return Some("RadialBurst");
    }
    if any(&[
        "PP_2DQUAD",
        "PP_2DTEXTURE",
        "PP_3DTEXTURE",
        "PP_3DQUAD",
        "PP_EFFECTTEXTURE",
        "PP_EFFECTTEXTURE2",
        "PP_EFFECTTEXTURE_ANI",
    ]) {
        return Some("FlatQuad");
    }
    Some("Bespoke")
}
