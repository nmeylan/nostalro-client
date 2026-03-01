use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::{UiDrawCall, UiTextureRef};
use ragnarok_ui::draw::{text_vertices, quad_vertices};

const LINE_HEIGHT: f32 = 20.0;
const PADDING: f32 = 10.0;
const BG_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 0.85];
const TEXT_COLOR: [f32; 4] = [0.9, 0.9, 0.9, 1.0];
const DIM_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
const HIGHLIGHT_COLOR: [f32; 4] = [0.2, 0.4, 0.7, 0.8];
const CURSOR_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub struct SpriteBrowser {
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

    pub fn set_items(&mut self, mut items: Vec<String>, label: &str) {
        items.sort();
        self.filtered = (0..items.len()).collect();
        self.items = items;
        self.label = label.to_string();
        self.filter_text.clear();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn handle_char(&mut self, ch: char) {
        self.filter_text.push(ch);
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
        self.filtered.get(self.selected)
            .map(|&idx| self.items[idx].as_str())
    }

    pub fn update_visible_rows(&mut self, screen_height: f32) {
        let available = screen_height - PADDING * 2.0 - LINE_HEIGHT * 2.0;
        self.visible_rows = (available / LINE_HEIGHT).max(1.0) as usize;
    }

    fn apply_filter(&mut self) {
        let needle = self.filter_text.to_lowercase();
        self.filtered = self.items.iter().enumerate()
            .filter(|(_, name)| needle.is_empty() || name.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn build_draw_calls(&self, atlas: &FontAtlas, screen_w: f32, screen_h: f32) -> Vec<UiDrawCall> {
        let mut calls = Vec::new();

        let (bg_verts, bg_idx) = quad_vertices(0.0, 0.0, screen_w, screen_h, BG_COLOR);
        calls.push(UiDrawCall {
            vertices: bg_verts.to_vec(),
            indices: bg_idx.to_vec(),
            texture: UiTextureRef::White,
        });

        let x = PADDING;
        let mut y = PADDING;

        let prompt = format!("> {}_", self.filter_text);
        let (tv, ti) = text_vertices(&prompt, x, y + atlas.ascent, CURSOR_COLOR, atlas);
        if !tv.is_empty() {
            calls.push(UiDrawCall { vertices: tv, indices: ti, texture: UiTextureRef::FontAtlas });
        }
        y += LINE_HEIGHT;

        let status = format!("{} / {} {}", self.filtered.len(), self.items.len(), self.label);
        let (sv, si) = text_vertices(&status, x, y + atlas.ascent, DIM_COLOR, atlas);
        if !sv.is_empty() {
            calls.push(UiDrawCall { vertices: sv, indices: si, texture: UiTextureRef::FontAtlas });
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
            let color = if i == self.selected { CURSOR_COLOR } else { TEXT_COLOR };
            let (tv, ti) = text_vertices(name, x, row_y + atlas.ascent, color, atlas);
            if !tv.is_empty() {
                calls.push(UiDrawCall { vertices: tv, indices: ti, texture: UiTextureRef::FontAtlas });
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
        for ch in "poring".chars() { browser.handle_char(ch); }
        assert_eq!(browser.filtered.len(), 2); // poring + poporing
        assert!(browser.selected_item().unwrap().contains("poring"));
    }

    #[test]
    fn filter_is_case_insensitive() {
        let mut browser = SpriteBrowser::new(sample_sprites(), "sprites");
        for ch in "KAF".chars() { browser.handle_char(ch); }
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
        for ch in "zzz".chars() { browser.handle_char(ch); }
        assert!(browser.selected_item().is_none());
    }

    #[test]
    fn set_items_replaces_content() {
        let mut browser = SpriteBrowser::new(sample_sprites(), "sprites");
        for ch in "kafra".chars() { browser.handle_char(ch); }
        assert_eq!(browser.filtered.len(), 1);

        let grf_files = vec!["a.grf".to_string(), "b.grf".to_string()];
        browser.set_items(grf_files, "GRF files");
        assert_eq!(browser.items.len(), 2);
        assert_eq!(browser.filtered.len(), 2);
        assert!(browser.filter_text.is_empty());
        assert_eq!(browser.selected, 0);
        assert_eq!(browser.label, "GRF files");
    }
}
