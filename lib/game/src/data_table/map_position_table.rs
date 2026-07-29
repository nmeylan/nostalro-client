use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;

const MAP_POSITION_PATH: &str = "data/mappostable.txt";

/// The map's rect on the world map image, in that image's pixel space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldMapRect {
    pub region: u8,
    pub left: u16,
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
}

impl WorldMapRect {
    pub fn width(&self) -> u16 {
        self.right.saturating_sub(self.left)
    }

    pub fn height(&self) -> u16 {
        self.bottom.saturating_sub(self.top)
    }

    pub fn center(&self) -> (f32, f32) {
        (
            (self.left as f32 + self.right as f32) / 2.0,
            (self.top as f32 + self.bottom as f32) / 2.0,
        )
    }
}

/// Which world-map rect each map occupies. Only outdoor maps are listed, so a
/// miss is the normal case in dungeons and indoor maps.
#[derive(Default)]
pub struct MapPositionTable {
    entries: HashMap<String, WorldMapRect>,
}

impl MapPositionTable {
    pub fn parse(data: &[u8]) -> Self {
        let text = String::from_utf8_lossy(data);
        let mut entries = HashMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            let mut fields = line.split('#').map(str::trim);
            let Some(region) = fields.next().and_then(|f| f.parse::<u8>().ok()) else {
                continue;
            };
            let Some(name) = fields.next().filter(|n| !n.is_empty()) else {
                continue;
            };
            let coords: Vec<u16> = fields.filter_map(|f| f.parse::<u16>().ok()).collect();
            let [left, top, right, bottom] = coords[..] else {
                continue;
            };
            entries.insert(
                crate::map_key(name),
                WorldMapRect {
                    region,
                    left,
                    top,
                    right,
                    bottom,
                },
            );
        }
        Self { entries }
    }

    pub fn load(grf: &GrfArchive) -> Self {
        let table = grf
            .read_file(MAP_POSITION_PATH)
            .map(|data| Self::parse(&data))
            .unwrap_or_default();
        tracing::info!("Loaded map position table: {} entries", table.len());
        table
    }

    pub fn rect(&self, map: &str) -> Option<WorldMapRect> {
        self.entries.get(&crate::map_key(map)).copied()
    }

    /// The map whose rect covers an image pixel. Rects do not overlap, so the
    /// smallest match is returned only to keep the result stable.
    pub fn at_pixel(&self, x: u16, y: u16) -> Option<(&str, WorldMapRect)> {
        self.entries
            .iter()
            .filter(|(_, r)| x >= r.left && x <= r.right && y >= r.top && y <= r.bottom)
            .min_by_key(|(_, r)| r.width() as u32 * r.height() as u32)
            .map(|(name, rect)| (name.as_str(), *rect))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = concat!(
        "//지역번호//맵이름//Xlt//yLT//xRB//yRB\n",
        "12@\n",
        "0#hugel.rsw#871#0#927#57#\n",
        "8#prontera.rsw#812#587#870#643#\n",
        "11#pay_fild03.rsw#1004#885#1058#931# \n",
    );

    #[test]
    fn parses_rows_and_skips_header_lines() {
        let table = MapPositionTable::parse(SAMPLE.as_bytes());
        assert_eq!(table.len(), 3);

        let prontera = table.rect("prontera").expect("prontera");
        assert_eq!(
            prontera,
            WorldMapRect {
                region: 8,
                left: 812,
                top: 587,
                right: 870,
                bottom: 643,
            }
        );
        assert_eq!(prontera.center(), (841.0, 615.0));

        // Trailing space on the row must not lose the last coordinate.
        assert_eq!(table.rect("pay_fild03").expect("pay_fild03").bottom, 931);
    }

    #[test]
    fn lookup_accepts_any_map_name_form() {
        let table = MapPositionTable::parse(SAMPLE.as_bytes());
        for name in ["hugel", "hugel.rsw", "hugel.gat", "Hugel.GAT"] {
            assert!(table.rect(name).is_some(), "{name}");
        }
        assert!(table.rect("prt_in").is_none());
    }
}
