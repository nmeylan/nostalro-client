use std::collections::HashMap;

/// Parses `data/mp3nametable.txt`: lines like `prt_church.rsw#bgm\10.mp3#`
/// (the shipped file uses a literal double backslash). Keyed by `<map>.rsw`,
/// value is the bare track filename (e.g. `10.mp3`).
pub fn parse_mp3_name_table(text: &str) -> HashMap<String, String> {
    let mut table = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let mut parts = line.split('#');
        let (Some(key), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        let normalized = value.replace('\\', "/");
        let track = normalized.rsplit('/').next().unwrap_or(&normalized);
        table.insert(key.to_ascii_lowercase(), track.to_string());
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_lines() {
        let text = "// header comment\n\
                    prt_church.rsw#bgm\\\\10.mp3#\n\
                    geffen.rsw#bgm\\05.mp3#\n\
                    \n\
                    // trailing comment\n";
        let table = parse_mp3_name_table(text);
        assert_eq!(table.get("prt_church.rsw").map(String::as_str), Some("10.mp3"));
        assert_eq!(table.get("geffen.rsw").map(String::as_str), Some("05.mp3"));
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn lookup_is_case_insensitive_on_key() {
        let table = parse_mp3_name_table("Prontera.rsw#bgm\\01.mp3#\n");
        assert_eq!(table.get("prontera.rsw").map(String::as_str), Some("01.mp3"));
    }
}
