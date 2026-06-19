use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::{UiDrawCall, UiTextureRef};
use ragnarok_ui::draw::{quad_vertices, text_vertices};

const LINE_HEIGHT: f32 = 20.0;
const PADDING: f32 = 10.0;
const BG_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 0.9];
const TEXT_COLOR: [f32; 4] = [0.9, 0.9, 0.9, 1.0];
const DIM_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
const HIGHLIGHT_COLOR: [f32; 4] = [0.2, 0.4, 0.7, 0.8];
const CURSOR_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const TAB_ACTIVE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const TAB_INACTIVE_COLOR: [f32; 4] = [0.4, 0.4, 0.4, 1.0];
const TAB_UNDERLINE_COLOR: [f32; 4] = [0.3, 0.6, 1.0, 1.0];
const TAB_GAP: f32 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BrowserTab {
    Towns,
    Dungeons,
    Fields,
    Other,
}

const TOWN_NAMES: &[&str] = &[
    "prontera",
    "geffen",
    "payon",
    "alberta",
    "morocc",
    "izlude",
    "aldebaran",
    "comodo",
    "umbala",
    "amatsu",
    "gonryun",
    "louyang",
    "ayothaya",
    "hugel",
    "einbroch",
    "einbech",
    "lighthalzen",
    "yuno",
    "veins",
    "rachel",
    "moscovia",
    "brasilis",
    "dewata",
    "malangdo",
    "eclage",
    "manuk",
    "splendide",
];

const DUNGEON_PREFIXES: &[&str] = &[
    "abyss_",
    "alde_dun",
    "anthell",
    "ayo_dun",
    "beach_dun",
    "c_tower",
    "cmd_fild07",
    "ein_dun",
    "gef_dun",
    "gefenia",
    "gl_",
    "gld_dun",
    "gon_dun",
    "hu_fild05",
    "ice_dun",
    "iz_dun",
    "juperos_",
    "kh_dun",
    "lhz_dun",
    "lou_dun",
    "mag_dun",
    "moc_pryd",
    "moc_prydb1",
    "mjo_",
    "mjolnir_",
    "mosk_dun",
    "nameless_n",
    "odin_tem",
    "orcsdun",
    "pay_dun",
    "prt_maze",
    "prt_sewb",
    "ra_san",
    "ra_temin",
    "schg_dun",
    "tha_t",
    "thor_v",
    "treasure",
    "tur_dun",
    "um_dun",
    "xmas_dun",
    "yuno_fild03",
    "abbey",
    "1@cata",
    "2@cata",
    "1@gl_",
    "2@gl_",
];

const FIELD_PREFIXES: &[&str] = &[
    "prt_fild",
    "gef_fild",
    "pay_fild",
    "moc_fild",
    "cmd_fild",
    "xmas_fild",
    "ein_fild",
    "lhz_fild",
    "yuno_fild",
    "iz_int",
    "izlu2dun",
    "mjolnir_",
    "ayo_fild",
    "ama_fild",
    "gon_fild",
    "lou_fild",
    "umbala",
    "hu_fild",
    "ra_fild",
    "ve_fild",
    "mosk_fild",
    "bra_fild",
    "dew_fild",
    "spl_fild",
    "man_fild",
    "ecl_fild",
    "n_castle",
];

struct TabData {
    active: BrowserTab,
    towns: Vec<usize>,
    dungeons: Vec<usize>,
    fields: Vec<usize>,
    other: Vec<usize>,
}

pub struct MapBrowser {
    /// Bare map names (no `data/` prefix, no `.rsw` suffix), sorted.
    all_names: Vec<String>,
    tabs: TabData,
    /// Indices into `all_names` for the active tab.
    items: Vec<usize>,
    /// Indices into `items` after filter is applied.
    filtered: Vec<usize>,
    filter_text: String,
    selected: usize,
    scroll_offset: usize,
    visible_rows: usize,
    pub open: bool,
}

impl MapBrowser {
    /// Build from full GRF paths like "data/prontera.rsw" - strips prefix/suffix.
    pub fn from_grf_paths(paths: Vec<String>) -> Self {
        let mut all_names: Vec<String> = paths
            .into_iter()
            .filter_map(|p| {
                let stripped = p.strip_prefix("data/").unwrap_or(&p);
                stripped.strip_suffix(".rsw").map(|n| n.to_string())
            })
            .collect();
        all_names.sort();
        all_names.dedup();

        let mut towns = Vec::new();
        let mut dungeons = Vec::new();
        let mut fields = Vec::new();
        let mut other = Vec::new();
        for (i, name) in all_names.iter().enumerate() {
            match classify(name) {
                BrowserTab::Towns => towns.push(i),
                BrowserTab::Dungeons => dungeons.push(i),
                BrowserTab::Fields => fields.push(i),
                BrowserTab::Other => other.push(i),
            }
        }

        let mut browser = Self {
            all_names,
            tabs: TabData {
                active: BrowserTab::Towns,
                towns,
                dungeons,
                fields,
                other,
            },
            items: Vec::new(),
            filtered: Vec::new(),
            filter_text: String::new(),
            selected: 0,
            scroll_offset: 0,
            visible_rows: 20,
            open: true,
        };
        browser.refresh_active_tab();
        browser
    }

    pub fn active_tab(&self) -> BrowserTab {
        self.tabs.active
    }

    pub fn switch_tab(&mut self, tab: BrowserTab) {
        if self.tabs.active == tab {
            return;
        }
        self.tabs.active = tab;
        self.filter_text.clear();
        self.refresh_active_tab();
    }

    fn refresh_active_tab(&mut self) {
        let src = match self.tabs.active {
            BrowserTab::Towns => &self.tabs.towns,
            BrowserTab::Dungeons => &self.tabs.dungeons,
            BrowserTab::Fields => &self.tabs.fields,
            BrowserTab::Other => &self.tabs.other,
        };
        self.items = src.clone();
        self.filtered = (0..self.items.len()).collect();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn handle_char(&mut self, ch: char) {
        self.filter_text.push(ch);
        self.apply_filter();
    }

    pub fn handle_paste(&mut self, text: &str) {
        for ch in text.chars() {
            if !ch.is_control() {
                self.filter_text.push(ch);
            }
        }
        self.apply_filter();
    }

    pub fn handle_backspace(&mut self) {
        self.filter_text.pop();
        self.apply_filter();
    }

    pub fn handle_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    pub fn handle_down(&mut self) {
        if !self.filtered.is_empty() && self.selected < self.filtered.len() - 1 {
            self.selected += 1;
            if self.selected >= self.scroll_offset + self.visible_rows {
                self.scroll_offset = self.selected + 1 - self.visible_rows;
            }
        }
    }

    pub fn handle_page_up(&mut self) {
        if self.selected >= self.visible_rows {
            self.selected -= self.visible_rows;
        } else {
            self.selected = 0;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }
    }

    pub fn handle_page_down(&mut self) {
        if !self.filtered.is_empty() {
            let max = self.filtered.len() - 1;
            self.selected = (self.selected + self.visible_rows).min(max);
            if self.selected >= self.scroll_offset + self.visible_rows {
                self.scroll_offset = self.selected + 1 - self.visible_rows;
            }
        }
    }

    pub fn selected_map(&self) -> Option<&str> {
        let &filtered_idx = self.filtered.get(self.selected)?;
        let &items_idx = self.items.get(filtered_idx)?;
        self.all_names.get(items_idx).map(|s| s.as_str())
    }

    pub fn update_visible_rows(&mut self, screen_height: f32) {
        let available = screen_height - PADDING * 2.0 - LINE_HEIGHT * 3.0;
        self.visible_rows = (available / LINE_HEIGHT).max(1.0) as usize;
    }

    fn apply_filter(&mut self) {
        let needle = self.filter_text.to_lowercase();
        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, name_idx)| {
                needle.is_empty() || self.all_names[**name_idx].to_lowercase().contains(&needle)
            })
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn build_draw_calls(
        &self,
        atlas: &FontAtlas,
        screen_w: f32,
        screen_h: f32,
    ) -> Vec<UiDrawCall> {
        let mut calls = Vec::new();

        let (bg_verts, bg_idx) = quad_vertices(0.0, 0.0, screen_w, screen_h, BG_COLOR);
        calls.push(UiDrawCall {
            vertices: bg_verts.to_vec(),
            indices: bg_idx.to_vec(),
            texture: UiTextureRef::White,
        });

        let x = PADDING;
        let mut y = PADDING;

        let tab_labels = [
            (BrowserTab::Towns, "1:TOWNS"),
            (BrowserTab::Dungeons, "2:DUNGEONS"),
            (BrowserTab::Fields, "3:FIELDS"),
            (BrowserTab::Other, "4:OTHER"),
        ];
        let mut tab_x = x;
        for (tab, label) in &tab_labels {
            let color = if *tab == self.tabs.active {
                TAB_ACTIVE_COLOR
            } else {
                TAB_INACTIVE_COLOR
            };
            let (tv, ti) = text_vertices(label, tab_x, y + atlas.ascent, color, atlas);
            let label_w = atlas.measure_text(label);
            if *tab == self.tabs.active {
                let (uv, ui) = quad_vertices(
                    tab_x,
                    y + LINE_HEIGHT - 2.0,
                    label_w,
                    2.0,
                    TAB_UNDERLINE_COLOR,
                );
                calls.push(UiDrawCall {
                    vertices: uv.to_vec(),
                    indices: ui.to_vec(),
                    texture: UiTextureRef::White,
                });
            }
            if !tv.is_empty() {
                calls.push(UiDrawCall {
                    vertices: tv,
                    indices: ti,
                    texture: UiTextureRef::FontAtlas,
                });
            }
            tab_x += label_w + TAB_GAP;
        }
        y += LINE_HEIGHT;

        let prompt = format!("> {}_", self.filter_text);
        let (tv, ti) = text_vertices(&prompt, x, y + atlas.ascent, CURSOR_COLOR, atlas);
        if !tv.is_empty() {
            calls.push(UiDrawCall {
                vertices: tv,
                indices: ti,
                texture: UiTextureRef::FontAtlas,
            });
        }
        y += LINE_HEIGHT;

        let label = match self.tabs.active {
            BrowserTab::Towns => "towns",
            BrowserTab::Dungeons => "dungeons",
            BrowserTab::Fields => "fields",
            BrowserTab::Other => "other maps",
        };
        let status = format!("{} / {} {label}", self.filtered.len(), self.items.len());
        let (sv, si) = text_vertices(&status, x, y + atlas.ascent, DIM_COLOR, atlas);
        if !sv.is_empty() {
            calls.push(UiDrawCall {
                vertices: sv,
                indices: si,
                texture: UiTextureRef::FontAtlas,
            });
        }
        y += LINE_HEIGHT;

        let end = (self.scroll_offset + self.visible_rows).min(self.filtered.len());
        for i in self.scroll_offset..end {
            let row_y = y + (i - self.scroll_offset) as f32 * LINE_HEIGHT;

            if i == self.selected {
                let (hv, hi) = quad_vertices(0.0, row_y, screen_w, LINE_HEIGHT, HIGHLIGHT_COLOR);
                calls.push(UiDrawCall {
                    vertices: hv.to_vec(),
                    indices: hi.to_vec(),
                    texture: UiTextureRef::White,
                });
            }

            let name_idx = self.items[self.filtered[i]];
            let name = &self.all_names[name_idx];
            let color = if i == self.selected {
                CURSOR_COLOR
            } else {
                TEXT_COLOR
            };
            let (tv, ti) = text_vertices(name, x, row_y + atlas.ascent, color, atlas);
            if !tv.is_empty() {
                calls.push(UiDrawCall {
                    vertices: tv,
                    indices: ti,
                    texture: UiTextureRef::FontAtlas,
                });
            }
        }

        calls
    }
}

fn classify(name: &str) -> BrowserTab {
    let lower = name.to_lowercase();
    if TOWN_NAMES.iter().any(|t| lower == *t) {
        return BrowserTab::Towns;
    }
    if DUNGEON_PREFIXES.iter().any(|p| lower.starts_with(p)) || lower.contains("_dun") {
        return BrowserTab::Dungeons;
    }
    if FIELD_PREFIXES.iter().any(|p| lower.starts_with(p)) || lower.contains("_fild") {
        return BrowserTab::Fields;
    }
    BrowserTab::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_paths() -> Vec<String> {
        vec![
            "data/prontera.rsw".to_string(),
            "data/geffen.rsw".to_string(),
            "data/prt_fild01.rsw".to_string(),
            "data/gef_dun00.rsw".to_string(),
            "data/abbey01.rsw".to_string(),
            "data/some_arena.rsw".to_string(),
            "data/in_sphinx1.rsw".to_string(),
        ]
    }

    #[test]
    fn classifies_and_filters_maps() {
        let mut b = MapBrowser::from_grf_paths(sample_paths());

        // Towns tab is default.
        assert_eq!(b.active_tab(), BrowserTab::Towns);
        assert!(
            b.items
                .iter()
                .map(|&i| &b.all_names[i])
                .any(|n| n == "prontera")
        );
        assert!(
            b.items
                .iter()
                .map(|&i| &b.all_names[i])
                .any(|n| n == "geffen")
        );

        // Dungeons: gef_dun00 (prefix) + abbey01 (prefix); in_sphinx1 falls through to Other.
        b.switch_tab(BrowserTab::Dungeons);
        let dungeon_names: Vec<&str> = b.items.iter().map(|&i| b.all_names[i].as_str()).collect();
        assert!(dungeon_names.contains(&"gef_dun00"));
        assert!(dungeon_names.contains(&"abbey01"));

        // Fields: prt_fild01 by prefix.
        b.switch_tab(BrowserTab::Fields);
        let field_names: Vec<&str> = b.items.iter().map(|&i| b.all_names[i].as_str()).collect();
        assert!(field_names.contains(&"prt_fild01"));

        // Filter on the active tab.
        for ch in "prt".chars() {
            b.handle_char(ch);
        }
        assert_eq!(b.selected_map(), Some("prt_fild01"));

        // Other tab catches uncategorized maps.
        b.switch_tab(BrowserTab::Other);
        let other_names: Vec<&str> = b.items.iter().map(|&i| b.all_names[i].as_str()).collect();
        assert!(other_names.contains(&"some_arena"));
        assert!(other_names.contains(&"in_sphinx1"));
    }
}
