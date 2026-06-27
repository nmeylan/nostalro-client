use std::collections::HashMap;

use crate::FormatError;

#[derive(Debug, Clone, Copy)]
pub struct FogEntry {
    pub near: f32,
    pub far: f32,
    pub color: [f32; 3],
    pub factor: f32,
}

pub struct FogTable {
    pub entries: HashMap<String, FogEntry>,
}

impl FogTable {
    pub fn parse(data: &[u8]) -> Result<Self, FormatError> {
        let text = std::str::from_utf8(data).map_err(|_| FormatError::InvalidString)?;
        let mut tokens: Vec<&str> = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }
            for piece in trimmed.split('#') {
                let piece = piece.trim();
                if !piece.is_empty() {
                    tokens.push(piece);
                }
            }
        }

        let mut entries = HashMap::new();
        let mut i = 0;
        while i + 4 < tokens.len() {
            let key = tokens[i].to_ascii_lowercase();
            let near = tokens[i + 1].parse::<f32>().ok();
            let far = tokens[i + 2].parse::<f32>().ok();
            let color = parse_hex_color(tokens[i + 3]);
            let factor = tokens[i + 4].parse::<f32>().ok();

            if let (Some(near), Some(far), Some(color), Some(factor)) = (near, far, color, factor) {
                entries.insert(
                    key,
                    FogEntry {
                        near,
                        far,
                        color,
                        factor,
                    },
                );
            }
            i += 5;
        }

        Ok(FogTable { entries })
    }

    pub fn get(&self, map_filename: &str) -> Option<FogEntry> {
        self.entries
            .get(&map_filename.to_ascii_lowercase())
            .copied()
    }
}

fn parse_hex_color(s: &str) -> Option<[f32; 3]> {
    let s = s.trim();
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    let value = u64::from_str_radix(s, 16).ok()?;
    let rgb = (value & 0x00FF_FFFF) as u32;
    let r = ((rgb >> 16) & 0xFF) as f32 / 255.0;
    let g = ((rgb >> 8) & 0xFF) as f32 / 255.0;
    let b = (rgb & 0xFF) as f32 / 255.0;
    Some([r, g, b])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_entries_and_lookup_lowercase() {
        let raw = b"\
mist.rsw#
0.2#
0.8#
0xDEB4FE#
0.5#
// commented out line
iz_dun04.rsw#
0.0#
0.5#
0xff000ca8#
0.5#
";
        let table = FogTable::parse(raw).expect("parse");
        assert_eq!(table.entries.len(), 2);

        let mist = table.get("mist.rsw").unwrap();
        assert!((mist.near - 0.2).abs() < 1e-6);
        assert!((mist.far - 0.8).abs() < 1e-6);
        assert!((mist.factor - 0.5).abs() < 1e-6);
        assert!((mist.color[0] - 222.0 / 255.0).abs() < 1e-6);
        assert!((mist.color[1] - 180.0 / 255.0).abs() < 1e-6);
        assert!((mist.color[2] - 254.0 / 255.0).abs() < 1e-6);

        let iz = table.get("IZ_DUN04.RSW").unwrap();
        assert!((iz.color[0] - 0.0).abs() < 1e-6);
        assert!((iz.color[1] - 0x0C as f32 / 255.0).abs() < 1e-6);
        assert!((iz.color[2] - 0xA8 as f32 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn missing_entry_returns_none() {
        let table = FogTable::parse(b"").unwrap();
        assert!(table.get("nope.rsw").is_none());
    }
}
