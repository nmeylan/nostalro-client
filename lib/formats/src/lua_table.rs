use std::collections::HashMap;

/// Decodes EUC-KR bytes to a UTF-8 string (lossy).
pub fn decode_euc_kr(data: &[u8]) -> String {
    let (decoded, _, _) = encoding_rs::EUC_KR.decode(data);
    decoded.into_owned()
}

/// Parses `id#resource_name#` format from EUC-KR data.
/// Returns id → `_resource_name` (with underscore prefix for accessory path convention).
pub fn parse_item_res_table(data: &[u8]) -> HashMap<u16, String> {
    let content = decode_euc_kr(data);
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let parts: Vec<&str> = line.split('#').collect();
        if parts.len() >= 2 {
            if let Ok(id) = parts[0].parse::<u16>() {
                let name = parts[1];
                if !name.is_empty() {
                    map.insert(id, format!("_{name}"));
                }
            }
        }
    }
    map
}

/// Parses `id#display_name#` format from EUC-KR data.
/// Returns id → display_name (no prefix, for UI display).
pub fn parse_item_name_table(data: &[u8]) -> HashMap<u16, String> {
    let content = decode_euc_kr(data);
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let parts: Vec<&str> = line.split('#').collect();
        if parts.len() >= 2 {
            if let Ok(id) = parts[0].parse::<u16>() {
                let name = parts[1].replace("_", " ");
                if !name.is_empty() {
                    map.insert(id, name.to_string());
                }
            }
        }
    }
    map
}

/// Parses item description tables (`idnum2itemdesctable.txt` / `num2itemdesctable.txt`).
/// Format: `ID#\ndescription lines...\n#\n` repeating.
/// Returns id → Vec of description lines (may contain `^RRGGBB` color codes).
pub fn parse_item_description_table(data: &[u8]) -> HashMap<u16, Vec<String>> {
    let content = decode_euc_kr(data);
    let mut map: HashMap<u16, Vec<String>> = HashMap::new();
    let mut current_id: Option<u16> = None;

    for token in content.split('#') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(id) = trimmed.parse::<u16>() {
            current_id = Some(id);
        } else if let Some(id) = current_id {
            let lines: Vec<String> = token
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.to_string())
                .collect();
            if !lines.is_empty() {
                map.entry(id).or_default().extend(lines);
            }
        }
    }
    map
}

/// Parses `id#` format (id-only lines). Returns a HashSet of ids.
pub fn parse_id_set_table(data: &[u8]) -> std::collections::HashSet<u16> {
    let content = decode_euc_kr(data);
    let mut set = std::collections::HashSet::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let parts: Vec<&str> = line.split('#').collect();
        if !parts.is_empty() {
            if let Ok(id) = parts[0].parse::<u16>() {
                set.insert(id);
            }
        }
    }
    set
}

/// Parses RO accessory data from `accessoryid.lua` and `accname.lua`.
/// Returns a map from view_id to sprite name suffix.
///
/// accessoryid.lua format: `ACCESSORY_GOGGLES = 1,`
/// accname.lua format: `[ACCESSORY_IDs.ACCESSORY_GOGGLES] = "_고글",`
pub fn build_accessory_table(id_content: &str, name_content: &str) -> HashMap<u16, String> {
    let name_to_id = parse_assignments(id_content);
    let name_to_suffix = parse_table_entries(name_content, "ACCESSORY_IDs.ACCESSORY_");

    let mut table = HashMap::new();
    for (name, id) in &name_to_id {
        if let Some(suffix) = name_to_suffix.get(name) {
            table.insert(*id as u16, suffix.clone());
        }
    }
    table
}

/// Parses `NAME = 123,` assignments. Returns name → number.
fn parse_assignments(content: &str) -> HashMap<String, u32> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("--") || line.is_empty() {
            continue;
        }
        // Match: ACCESSORY_NAME = 123
        if let Some((name_part, value_part)) = line.split_once('=') {
            let name = name_part.trim().trim_start_matches("ACCESSORY_");
            let value_str = value_part.trim().trim_end_matches(',').trim();
            if let Ok(id) = value_str.parse::<u32>() {
                map.insert(name.to_string(), id);
            }
        }
    }
    map
}

/// Parses `[prefix.NAME] = "value",` table entries. Returns name → value.
fn parse_table_entries(content: &str, prefix: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("--") || line.is_empty() {
            continue;
        }
        // Match: [ACCESSORY_IDs.ACCESSORY_NAME] = "value"
        let Some(rest) = line.strip_prefix('[') else { continue };
        let Some((key_part, value_part)) = rest.split_once(']') else { continue };
        let name = key_part.trim().strip_prefix(prefix).unwrap_or(key_part.trim());

        let Some((_, val_rest)) = value_part.split_once('=') else { continue };
        let val = val_rest.trim().trim_end_matches(',').trim();
        if val.starts_with('"') && val.ends_with('"') {
            let inner = &val[1..val.len() - 1];
            map.insert(name.to_string(), inner.to_string());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accessory_id_and_name() {
        let id_content = r#"
ACCESSORY_GOGGLES = 1,
ACCESSORY_RIBBON = 4,
ACCESSORY_HEADBAND = 6,
"#;
        let name_content = r#"
[ACCESSORY_IDs.ACCESSORY_GOGGLES] = "_고글",
[ACCESSORY_IDs.ACCESSORY_RIBBON] = "_리본",
[ACCESSORY_IDs.ACCESSORY_HEADBAND] = "_머리띠",
"#;
        let table = build_accessory_table(id_content, name_content);
        assert_eq!(table.get(&1).unwrap(), "_고글");
        assert_eq!(table.get(&4).unwrap(), "_리본");
        assert_eq!(table.get(&6).unwrap(), "_머리띠");
        assert_eq!(table.len(), 3);
    }

    #[test]
    fn skips_comments_and_empty_lines() {
        let id_content = "-- comment\nACCESSORY_TEST = 5,\n\n";
        let name_content = "-- comment\n[ACCESSORY_IDs.ACCESSORY_TEST] = \"_테스트\",\n";
        let table = build_accessory_table(id_content, name_content);
        assert_eq!(table.get(&5).unwrap(), "_테스트");
    }

    #[test]
    fn unmatched_entries_are_skipped() {
        let id_content = "ACCESSORY_A = 1,\nACCESSORY_B = 2,\n";
        let name_content = "[ACCESSORY_IDs.ACCESSORY_A] = \"_a\",\n";
        let table = build_accessory_table(id_content, name_content);
        assert_eq!(table.len(), 1);
        assert!(table.get(&2).is_none());
    }

    #[test]
    fn parse_item_description_table_multiple_items() {
        let data = b"501#\nA red potion.\n^FFFFFF_^000000\nClass:^0000FF Restorative^000000\nWeight:^009900 7^000000\n#\n502#\nAn orange potion.\nWeight:^009900 10^000000\n#\n";
        let table = parse_item_description_table(data);
        assert_eq!(table.len(), 2);
        let desc_501 = table.get(&501).unwrap();
        assert_eq!(desc_501.len(), 4);
        assert_eq!(desc_501[0], "A red potion.");
        assert_eq!(desc_501[1], "^FFFFFF_^000000");
        assert_eq!(desc_501[2], "Class:^0000FF Restorative^000000");
        assert_eq!(desc_501[3], "Weight:^009900 7^000000");
        let desc_502 = table.get(&502).unwrap();
        assert_eq!(desc_502.len(), 2);
        assert_eq!(desc_502[0], "An orange potion.");
    }

    #[test]
    fn parse_item_description_table_missing_item_returns_none() {
        let data = b"501#\nSome desc\n#\n";
        let table = parse_item_description_table(data);
        assert!(table.get(&999).is_none());
    }

    #[test]
    fn parse_id_set_table_extracts_ids() {
        let data = b"4001#\n4002#\n// comment\n\n4003#\n";
        let set = parse_id_set_table(data);
        assert_eq!(set.len(), 3);
        assert!(set.contains(&4001));
        assert!(set.contains(&4002));
        assert!(set.contains(&4003));
        assert!(!set.contains(&9999));
    }
}
