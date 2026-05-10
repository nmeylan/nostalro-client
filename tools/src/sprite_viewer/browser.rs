use ragnarok_game::accessory_table::AccessoryTable;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::{UiDrawCall, UiTextureRef};
use ragnarok_ui::draw::{quad_vertices, text_vertices};

const LINE_HEIGHT: f32 = 20.0;
const PADDING: f32 = 10.0;
const BG_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 0.85];
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
    Npc,
    Monster,
    Character,
    Headgear,
    Other,
}

const JOB_LIST: &[(u16, &str)] = &[
    (0, "Novice"),
    (1, "Swordsman"),
    (2, "Mage"),
    (3, "Archer"),
    (4, "Acolyte"),
    (5, "Merchant"),
    (6, "Thief"),
    (7, "Knight"),
    (8, "Priest"),
    (9, "Wizard"),
    (10, "Blacksmith"),
    (11, "Hunter"),
    (12, "Assassin"),
    (13, "Peco Knight"),
    (14, "Crusader"),
    (15, "Monk"),
    (16, "Sage"),
    (17, "Rogue"),
    (18, "Alchemist"),
    (19, "Bard"),
    (20, "Dancer"),
    (21, "Peco Crusader"),
    (23, "Super Novice"),
    (4008, "Lord Knight"),
    (4009, "High Priest"),
    (4010, "High Wizard"),
    (4011, "Whitesmith"),
    (4012, "Sniper"),
    (4013, "Assassin Cross"),
    (4014, "Peco Lord Knight"),
    (4015, "Paladin"),
    (4016, "Champion"),
    (4017, "Professor"),
    (4018, "Stalker"),
    (4019, "Creator"),
    (4020, "Clown"),
    (4021, "Gypsy"),
    (4022, "Peco Paladin"),
];

struct TabData {
    active: BrowserTab,
    npc_sprites: Vec<String>,
    monster_sprites: Vec<String>,
    char_names: Vec<String>,
    char_job_ids: Vec<u16>,
    headgear_names: Vec<String>,
    headgear_ids: Vec<u16>,
    other_sprites: Vec<String>,
}

pub struct SpriteBrowser {
    tabs: Option<TabData>,
    items: Vec<String>,
    filtered: Vec<usize>,
    filter_text: String,
    selected: usize,
    scroll_offset: usize,
    visible_rows: usize,
    label: String,
    pub open: bool,
}

impl SpriteBrowser {
    pub fn new(mut items: Vec<String>, label: &str) -> Self {
        items.sort();
        let filtered: Vec<usize> = (0..items.len()).collect();
        Self {
            tabs: None,
            items,
            filtered,
            filter_text: String::new(),
            selected: 0,
            scroll_offset: 0,
            visible_rows: 20,
            label: label.to_string(),
            open: true,
        }
    }

    pub fn new_with_tabs(all_sprites: Vec<String>, accessory_table: &AccessoryTable) -> Self {
        let mut npc_sprites: Vec<String> = all_sprites
            .iter()
            .filter(|s| s.starts_with("data/sprite/npc/"))
            .cloned()
            .collect();
        npc_sprites.sort();

        let mut monster_sprites: Vec<String> = all_sprites
            .iter()
            .filter(|s| s.starts_with("data/sprite/몬스터/"))
            .cloned()
            .collect();
        monster_sprites.sort();

        let char_names: Vec<String> = JOB_LIST.iter().map(|(_, name)| name.to_string()).collect();
        let char_job_ids: Vec<u16> = JOB_LIST.iter().map(|(id, _)| *id).collect();

        let excluded_prefixes = [
            "data/sprite/npc/",
            "data/sprite/몬스터/",
            "data/sprite/인간족/",
            "data/sprite/악세사리/",
            "data/sprite/방패/",
        ];
        let mut other_sprites: Vec<String> = all_sprites
            .iter()
            .filter(|s| !excluded_prefixes.iter().any(|prefix| s.starts_with(prefix)))
            .cloned()
            .collect();
        other_sprites.sort();

        let sorted_accessories = accessory_table.sorted_entries();
        let headgear_names: Vec<String> = sorted_accessories
            .iter()
            .map(|(id, suffix)| format!("{id}: {suffix}"))
            .collect();
        let headgear_ids: Vec<u16> = sorted_accessories.iter().map(|(id, _)| *id).collect();

        let filtered: Vec<usize> = (0..npc_sprites.len()).collect();
        let items = npc_sprites.clone();

        Self {
            tabs: Some(TabData {
                active: BrowserTab::Npc,
                npc_sprites,
                monster_sprites,
                char_names,
                char_job_ids,
                headgear_names,
                headgear_ids,
                other_sprites,
            }),
            items,
            filtered,
            filter_text: String::new(),
            selected: 0,
            scroll_offset: 0,
            visible_rows: 20,
            label: "NPC sprites".to_string(),
            open: true,
        }
    }

    pub fn set_items(&mut self, mut items: Vec<String>, label: &str) {
        items.sort();
        self.filtered = (0..items.len()).collect();
        self.items = items;
        self.label = label.to_string();
        self.filter_text.clear();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn has_tabs(&self) -> bool {
        self.tabs.is_some()
    }

    pub fn active_tab(&self) -> Option<BrowserTab> {
        self.tabs.as_ref().map(|t| t.active)
    }

    pub fn switch_tab(&mut self, tab: BrowserTab) {
        let Some(tabs) = &mut self.tabs else { return };
        if tabs.active == tab {
            return;
        }
        tabs.active = tab;
        let (items, label) = match tab {
            BrowserTab::Npc => (tabs.npc_sprites.clone(), "NPC sprites"),
            BrowserTab::Monster => (tabs.monster_sprites.clone(), "monster sprites"),
            BrowserTab::Character => (tabs.char_names.clone(), "characters"),
            BrowserTab::Headgear => (tabs.headgear_names.clone(), "headgear"),
            BrowserTab::Other => (tabs.other_sprites.clone(), "other sprites"),
        };
        self.items = items;
        self.label = label.to_string();
        self.filtered = (0..self.items.len()).collect();
        self.filter_text.clear();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn selected_job_id(&self) -> Option<u16> {
        let tabs = self.tabs.as_ref()?;
        if tabs.active != BrowserTab::Character {
            return None;
        }
        let &idx = self.filtered.get(self.selected)?;
        tabs.char_job_ids.get(idx).copied()
    }

    pub fn selected_headgear_id(&self) -> Option<u16> {
        let tabs = self.tabs.as_ref()?;
        if tabs.active != BrowserTab::Headgear {
            return None;
        }
        let &idx = self.filtered.get(self.selected)?;
        tabs.headgear_ids.get(idx).copied()
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

    pub fn selected_item(&self) -> Option<&str> {
        self.filtered
            .get(self.selected)
            .map(|&idx| self.items[idx].as_str())
    }

    pub fn update_visible_rows(&mut self, screen_height: f32) {
        let tab_offset = if self.tabs.is_some() {
            LINE_HEIGHT
        } else {
            0.0
        };
        let available = screen_height - PADDING * 2.0 - LINE_HEIGHT * 2.0 - tab_offset;
        self.visible_rows = (available / LINE_HEIGHT).max(1.0) as usize;
    }

    fn apply_filter(&mut self) {
        let needle = self.filter_text.to_lowercase();
        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, name)| needle.is_empty() || name.to_lowercase().contains(&needle))
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

        if let Some(tabs) = &self.tabs {
            let tab_labels = [
                (BrowserTab::Npc, "1:NPC"),
                (BrowserTab::Monster, "2:MONSTER"),
                (BrowserTab::Character, "3:CHARACTER"),
                (BrowserTab::Headgear, "4:HEADGEAR"),
                (BrowserTab::Other, "5:OTHER"),
            ];
            let mut tab_x = x;
            for (tab, label) in &tab_labels {
                let color = if *tab == tabs.active {
                    TAB_ACTIVE_COLOR
                } else {
                    TAB_INACTIVE_COLOR
                };
                let (tv, ti) = text_vertices(label, tab_x, y + atlas.ascent, color, atlas);
                let label_w = atlas.measure_text(label);
                if *tab == tabs.active {
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
        }

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

        let status = format!(
            "{} / {} {}",
            self.filtered.len(),
            self.items.len(),
            self.label
        );
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

            let name = &self.items[self.filtered[i]];
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_sprites() -> Vec<String> {
        vec![
            "data/sprite/monsters/poring.spr".to_string(),
            "data/sprite/monsters/drops.spr".to_string(),
            "data/sprite/npc/kafra.spr".to_string(),
            "data/sprite/monsters/poporing.spr".to_string(),
        ]
    }

    #[test]
    fn new_sorts_and_shows_all() {
        let browser = SpriteBrowser::new(sample_sprites(), "sprites");
        assert_eq!(browser.filtered.len(), 4);
        assert_eq!(browser.items[0], "data/sprite/monsters/drops.spr");
        assert_eq!(browser.items[3], "data/sprite/npc/kafra.spr");
    }

    #[test]
    fn filter_narrows_results() {
        let mut browser = SpriteBrowser::new(sample_sprites(), "sprites");
        for ch in "poring".chars() {
            browser.handle_char(ch);
        }
        assert_eq!(browser.filtered.len(), 2); // poring + poporing
        assert!(browser.selected_item().unwrap().contains("poring"));
    }

    #[test]
    fn filter_is_case_insensitive() {
        let mut browser = SpriteBrowser::new(sample_sprites(), "sprites");
        for ch in "KAF".chars() {
            browser.handle_char(ch);
        }
        assert_eq!(browser.filtered.len(), 1);
        assert!(browser.selected_item().unwrap().contains("kafra"));
    }

    #[test]
    fn backspace_widens_filter() {
        let mut browser = SpriteBrowser::new(sample_sprites(), "sprites");
        browser.handle_char('k');
        browser.handle_char('a');
        assert_eq!(browser.filtered.len(), 1);
        browser.handle_backspace();
        browser.handle_backspace();
        assert_eq!(browser.filtered.len(), 4);
    }

    #[test]
    fn navigation_clamps() {
        let mut browser = SpriteBrowser::new(sample_sprites(), "sprites");
        browser.handle_up();
        assert_eq!(browser.selected, 0);
        browser.handle_down();
        browser.handle_down();
        browser.handle_down();
        assert_eq!(browser.selected, 3);
        browser.handle_down();
        assert_eq!(browser.selected, 3);
    }

    #[test]
    fn page_navigation() {
        let sprites: Vec<String> = (0..50).map(|i| format!("sprite_{i:03}.spr")).collect();
        let mut browser = SpriteBrowser::new(sprites, "sprites");
        browser.visible_rows = 10;
        browser.handle_page_down();
        assert_eq!(browser.selected, 10);
        browser.handle_page_up();
        assert_eq!(browser.selected, 0);
    }

    #[test]
    fn selected_item_empty_filter() {
        let mut browser = SpriteBrowser::new(sample_sprites(), "sprites");
        for ch in "zzz".chars() {
            browser.handle_char(ch);
        }
        assert!(browser.selected_item().is_none());
    }

    #[test]
    fn set_items_replaces_content() {
        let mut browser = SpriteBrowser::new(sample_sprites(), "sprites");
        for ch in "kafra".chars() {
            browser.handle_char(ch);
        }
        assert_eq!(browser.filtered.len(), 1);

        let grf_files = vec!["a.grf".to_string(), "b.grf".to_string()];
        browser.set_items(grf_files, "GRF files");
        assert_eq!(browser.items.len(), 2);
        assert_eq!(browser.filtered.len(), 2);
        assert!(browser.filter_text.is_empty());
        assert_eq!(browser.selected, 0);
        assert_eq!(browser.label, "GRF files");
    }

    #[test]
    fn tabs_categorize_sprites() {
        let sprites = vec![
            "data/sprite/npc/kafra.spr".to_string(),
            "data/sprite/npc/merchant.spr".to_string(),
            "data/sprite/몬스터/poring.spr".to_string(),
        ];
        let browser = SpriteBrowser::new_with_tabs(sprites, &AccessoryTable::empty());
        assert!(browser.has_tabs());
        assert_eq!(browser.active_tab(), Some(BrowserTab::Npc));
        assert_eq!(browser.items.len(), 2);
    }

    #[test]
    fn tab_switch_changes_items() {
        let sprites = vec![
            "data/sprite/npc/kafra.spr".to_string(),
            "data/sprite/몬스터/poring.spr".to_string(),
        ];
        let mut browser = SpriteBrowser::new_with_tabs(sprites, &AccessoryTable::empty());
        assert_eq!(browser.items.len(), 1); // 1 NPC

        browser.switch_tab(BrowserTab::Monster);
        assert_eq!(browser.active_tab(), Some(BrowserTab::Monster));
        assert_eq!(browser.items.len(), 1); // 1 monster

        browser.switch_tab(BrowserTab::Character);
        assert_eq!(browser.active_tab(), Some(BrowserTab::Character));
        assert_eq!(browser.items.len(), JOB_LIST.len());
    }

    #[test]
    fn character_tab_returns_job_id() {
        let mut browser = SpriteBrowser::new_with_tabs(Vec::new(), &AccessoryTable::empty());
        browser.switch_tab(BrowserTab::Character);
        assert_eq!(browser.selected_job_id(), Some(0)); // Novice
        browser.handle_down();
        assert_eq!(browser.selected_job_id(), Some(1)); // Swordsman
    }

    #[test]
    fn tab_switch_resets_filter() {
        let sprites = vec![
            "data/sprite/npc/kafra.spr".to_string(),
            "data/sprite/npc/merchant.spr".to_string(),
        ];
        let mut browser = SpriteBrowser::new_with_tabs(sprites, &AccessoryTable::empty());
        browser.handle_char('k');
        assert_eq!(browser.filtered.len(), 1);

        browser.switch_tab(BrowserTab::Monster);
        assert!(browser.filter_text.is_empty());
        assert_eq!(browser.selected, 0);
    }

    #[test]
    fn other_tab_captures_uncategorized_sprites() {
        let sprites = vec![
            "data/sprite/npc/kafra.spr".to_string(),
            "data/sprite/몬스터/poring.spr".to_string(),
            "data/sprite/인간족/몸통/남/초보자_남.spr".to_string(),
            "data/sprite/악세사리/남/남_1.spr".to_string(),
            "data/sprite/방패/기사/기사_남_1.spr".to_string(),
            "data/sprite/cursors.spr".to_string(),
            "data/sprite/shadow.spr".to_string(),
            "data/sprite/이팩트/effect.spr".to_string(),
        ];
        let mut browser = SpriteBrowser::new_with_tabs(sprites, &AccessoryTable::empty());
        browser.switch_tab(BrowserTab::Other);
        assert_eq!(browser.active_tab(), Some(BrowserTab::Other));
        assert_eq!(browser.items.len(), 3);
        assert_eq!(browser.items[0], "data/sprite/cursors.spr");
        assert_eq!(browser.items[1], "data/sprite/shadow.spr");
        assert_eq!(browser.items[2], "data/sprite/이팩트/effect.spr");
    }
}
