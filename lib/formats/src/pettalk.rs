use crate::lua_table::decode_euc_kr;
use std::collections::HashMap;

/// Pet talk lines from `data/pettalktable.xml`. The shipped file is Atbash-
/// ciphered EUC-KR; tag names and body both decode once the cipher is undone.
/// Layout: `monster_talk_table > <mob> > <hunger> > <act>* sentences`.
#[derive(Debug, Default)]
pub struct PetTalkTable {
    entries: HashMap<String, HashMap<String, HashMap<String, Vec<String>>>>,
}

fn atbash(byte: u8) -> u8 {
    match byte {
        b'A'..=b'Z' => b'A' + b'Z' - byte,
        b'a'..=b'z' => b'a' + b'z' - byte,
        other => other,
    }
}

impl PetTalkTable {
    pub fn parse(data: &[u8]) -> Self {
        let deciphered: Vec<u8> = data.iter().map(|&b| atbash(b)).collect();
        let text = decode_euc_kr(&deciphered);
        Self::parse_text(&text)
    }

    fn parse_text(text: &str) -> Self {
        let mut entries: HashMap<String, HashMap<String, HashMap<String, Vec<String>>>> =
            HashMap::new();
        // Depth 1 = mob, depth 2 = hunger, depth 3 = act. `monster_talk_table` is
        // the only depth-0 element and is skipped.
        let mut stack: Vec<String> = Vec::new();
        let mut rest = text;
        while let Some(open) = rest.find('<') {
            let content_before = &rest[..open];
            if let (Some(mob), Some(hunger), Some(act)) =
                (stack.first(), stack.get(1), stack.get(2))
            {
                let sentence = content_before.trim();
                if !sentence.is_empty() {
                    entries
                        .entry(mob.clone())
                        .or_default()
                        .entry(hunger.clone())
                        .or_default()
                        .entry(act.clone())
                        .or_default()
                        .push(sentence.to_string());
                }
            }
            let Some(close) = rest[open..].find('>') else {
                break;
            };
            let tag = &rest[open + 1..open + close];
            rest = &rest[open + close + 1..];
            if tag.starts_with('?') || tag.starts_with('!') {
                continue;
            }
            if let Some(name) = tag.strip_prefix('/') {
                if stack.last().map(String::as_str) == Some(name.trim()) {
                    stack.pop();
                }
                continue;
            }
            let name = tag.trim().trim_end_matches('/');
            if name == "monster_talk_table" {
                continue;
            }
            if !tag.trim().ends_with('/') {
                stack.push(name.to_string());
            }
        }
        PetTalkTable { entries }
    }

    /// All sentences for a (mob, hunger, act) key, if present.
    pub fn lines(&self, mob: &str, hunger: &str, act: &str) -> Option<&[String]> {
        self.entries
            .get(mob)?
            .get(hunger)?
            .get(act)
            .map(Vec::as_slice)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_keys() {
        // Whole document is Atbash-ciphered: tags and body both decode. Bodies
        // "sr"/"bl" decipher to "hi"/"yo".
        let xml = "<nlmhgvi_gzop_gzyov><klirmt><sfmtib><uvvwrmt>sr</uvvwrmt>\
                   <uvvwrmt>bl</uvvwrmt></sfmtib></klirmt></nlmhgvi_gzop_gzyov>";
        let table = PetTalkTable::parse(xml.as_bytes());
        let lines = table.lines("poring", "hungry", "feeding").unwrap();
        assert_eq!(lines, &["hi".to_string(), "yo".to_string()]);
    }
}
