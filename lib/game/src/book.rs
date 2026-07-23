use ragnarok_formats::lua_table::decode_euc_kr;

const DEFAULT_BG: [f32; 3] = [0.96, 0.96, 0.86];

pub struct BookContent {
    pub bg_color: [f32; 3],
    pub lines: Vec<String>,
}

impl BookContent {
    pub fn parse(data: &[u8]) -> Self {
        let text = decode_euc_kr(data);
        let (bg_color, body) = match text.strip_prefix('%') {
            Some(rest)
                if rest.len() >= 6 && rest.as_bytes()[..6].iter().all(u8::is_ascii_hexdigit) =>
            {
                (
                    parse_hex_color(&rest[..6]).unwrap_or(DEFAULT_BG),
                    &rest[6..],
                )
            }
            _ => (DEFAULT_BG, text.as_str()),
        };
        let lines = body
            .split('\n')
            .map(|l| l.trim_end_matches('\r').to_string())
            .collect();
        Self { bg_color, lines }
    }
}

fn parse_hex_color(s: &str) -> Option<[f32; 3]> {
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bg_color_and_body_lines() {
        let raw = b"%f5f5dc^000088First line\nsecond line\r\nthird";
        let book = BookContent::parse(raw);
        assert_eq!(
            book.bg_color,
            [
                0xf5 as f32 / 255.0,
                0xf5 as f32 / 255.0,
                0xdc as f32 / 255.0
            ]
        );
        assert_eq!(
            book.lines,
            vec!["^000088First line", "second line", "third"]
        );
    }

    #[test]
    fn falls_back_when_no_color_header() {
        let book = BookContent::parse(b"plain text\nline two");
        assert_eq!(book.bg_color, DEFAULT_BG);
        assert_eq!(book.lines, vec!["plain text", "line two"]);
    }
}
