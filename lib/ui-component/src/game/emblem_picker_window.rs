use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    FOOTER_TEX, SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_footer,
    draw_sys_button, draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const EMBLEM_PICKER_WINDOW_ID: WidgetId = WidgetId(3480);
const CLOSE_BTN_ID: WidgetId = WidgetId(3481);
const OK_BTN_ID: WidgetId = WidgetId(3482);
const CANCEL_BTN_ID: WidgetId = WidgetId(3483);
const EMBLEM_SCROLL: ScrollbarIds = ScrollbarIds {
    up: WidgetId(3484),
    down: WidgetId(3485),
    thumb: WidgetId(3486),
};
const THUMB_BASE_ID: u32 = 3490;

const CLOSE_OFF_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_OFF;
const CLOSE_ON_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_ON;

const OK_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_OK,
    hover: ragnarok_resources::ui::BTN_OK_A,
    pressed: ragnarok_resources::ui::BTN_OK_B,
};
const CANCEL_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_CANCEL,
    hover: ragnarok_resources::ui::BTN_CANCEL_A,
    pressed: ragnarok_resources::ui::BTN_CANCEL_B,
};

const WIN_W: f32 = 322.0;
const TITLE_H: f32 = 17.0;
const CONTENT_H: f32 = 214.0;
const FOOTER_H: f32 = 27.0;
const CLOSE_BTN_SIZE: f32 = 11.0;

const GRID_W: f32 = 200.0;
const CELL_W: f32 = 46.0;
const CELL_H: f32 = 52.0;
const THUMB: f32 = 32.0;
const COLS: usize = 4;
const PREVIEW: f32 = 80.0;

const BORDER: [f32; 4] = [0.761, 0.761, 0.761, 1.0];
const SELECTION_COLOR: [f32; 4] = [0.451, 0.62, 0.937, 1.0];
const LISTBOX_BG: [f32; 4] = [0.808, 0.808, 0.808, 1.0];
const TEXT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const OFFLINE_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
const OK_GREEN: [f32; 4] = [0.06, 0.42, 0.12, 1.0];
const ERR_RED: [f32; 4] = [0.7, 0.1, 0.1, 1.0];

#[derive(Clone)]
pub struct EmblemEntry {
    pub name: String,
    pub path: String,
    pub key: String,
    pub valid: bool,
    pub verdict: String,
}

pub struct EmblemPickerWindow {
    pub open: bool,
    pub has_grf_textures: bool,
    entries: Vec<EmblemEntry>,
    selected: Option<usize>,
    scroll: usize,
}

impl Default for EmblemPickerWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl EmblemPickerWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            has_grf_textures: false,
            entries: Vec::new(),
            selected: None,
            scroll: 0,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, entries: Vec<EmblemEntry>) {
        self.selected = entries.iter().position(|e| e.valid);
        self.entries = entries;
        self.scroll = 0;
        self.open = true;
    }

    fn fill(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let (v, i) = draw::quad_vertices(x, y, w, h, color);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
    }

    fn draw_named(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, key: &str) {
        let (v, i) = draw::quad_vertices(x, y, w, h, [1.0; 4]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(key.to_string()),
        });
    }
}

impl Window for EmblemPickerWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn window_size(&self) -> (f32, f32) {
        (WIN_W, TITLE_H + CONTENT_H + FOOTER_H)
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            FOOTER_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
        ];
        paths.extend(scrollbar::grf_texture_paths());
        paths
    }
}

impl InGameWindow for EmblemPickerWindow {
    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.open
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.open = false;
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let _character = &mut *ctx.character;
        let _data = ctx.data;
        if !self.open {
            return Vec::new();
        }

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let mut events = Vec::new();

        let win_h = TITLE_H + CONTENT_H + FOOTER_H;
        let win = ui.window_at(EMBLEM_PICKER_WINDOW_ID, WIN_W, win_h, TITLE_H, 100.0, 70.0);
        let x = win.x;
        let y = win.y;
        ui.interact(EMBLEM_PICKER_WINDOW_ID, Rect::new(x, y, WIN_W, win_h));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        ui.text(x + 17.0, y + TITLE_H - 3.0, "Select emblem", tc);

        let close_rect = Rect::new(
            x + WIN_W - CLOSE_BTN_SIZE - 3.0,
            y + (TITLE_H - CLOSE_BTN_SIZE) / 2.0,
            CLOSE_BTN_SIZE,
            CLOSE_BTN_SIZE,
        );
        let close_resp = ui.interact(CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        draw_sys_button(
            ui,
            close_rect,
            (CLOSE_BTN_SIZE, CLOSE_BTN_SIZE),
            close_resp.hovered(),
            grf,
            CLOSE_ON_TEX,
            CLOSE_OFF_TEX,
            Some('x'),
        );
        if close_resp.clicked() {
            self.open = false;
            ui.has_grf_textures = prev_grf;
            return events;
        }

        let content_y = y + TITLE_H;
        draw_container(ui, x, content_y, WIN_W, CONTENT_H, grf);
        self.build_grid(ui, x + 6.0, content_y + 6.0);
        self.build_preview(ui, x + GRID_W + 12.0, content_y + 6.0);

        let footer_y = content_y + CONTENT_H;
        draw_footer(ui, x, footer_y, WIN_W, FOOTER_H, grf);
        events.extend(self.build_footer(ui, x, footer_y));

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl EmblemPickerWindow {
    fn build_grid(&mut self, ui: &mut UiFrame, gx: f32, gy: f32) {
        let box_h = CONTENT_H - 12.0;
        Self::fill(ui, gx, gy, GRID_W, box_h, LISTBOX_BG);
        if self.entries.is_empty() {
            ui.text(gx + 6.0, gy + 18.0, "No .bmp emblems found.", OFFLINE_COLOR);
            return;
        }

        let total_rows = self.entries.len().div_ceil(COLS);
        let visible_rows = (((box_h - 8.0) / CELL_H) as usize).max(1);
        let max_scroll = total_rows.saturating_sub(visible_rows);
        if max_scroll > 0 {
            let content_rect = Rect::new(gx, gy, GRID_W, box_h);
            self.scroll = scrollbar::scrollbar(
                ui,
                EMBLEM_SCROLL,
                self.scroll,
                visible_rows,
                max_scroll,
                content_rect,
                gx + GRID_W - SCROLLBAR_W,
                gy,
                box_h,
            );
        } else {
            self.scroll = 0;
        }

        let start = self.scroll * COLS;
        let end = (start + visible_rows * COLS).min(self.entries.len());
        for idx in start..end {
            let entry = &self.entries[idx];
            let vis = idx - start;
            let col = vis % COLS;
            let row = vis / COLS;
            let cx = gx + 4.0 + col as f32 * CELL_W;
            let cy = gy + 4.0 + row as f32 * CELL_H;
            let cell = Rect::new(cx, cy, CELL_W - 4.0, CELL_H - 4.0);
            if self.selected == Some(idx) {
                Self::fill(ui, cell.x, cell.y, cell.w, cell.h, SELECTION_COLOR);
            }
            let thumb_x = cx + (CELL_W - 4.0 - THUMB) / 2.0;
            Self::fill(ui, thumb_x, cy + 2.0, THUMB, THUMB, [1.0; 4]);
            Self::draw_named(ui, thumb_x, cy + 2.0, THUMB, THUMB, &entry.key);
            let label: String = entry.name.chars().take(8).collect();
            let color = if entry.valid { TEXT } else { ERR_RED };
            ui.text(cx + 2.0, cy + THUMB + 13.0, &label, color);
            let resp = ui.interact(WidgetId(THUMB_BASE_ID + idx as u32), cell);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() {
                self.selected = Some(idx);
            }
        }
    }

    fn build_preview(&self, ui: &mut UiFrame, px: f32, py: f32) {
        ui.text(px, py + 10.0, "Preview", TEXT);
        let box_x = px + (WIN_W - GRID_W - 12.0 - PREVIEW) / 2.0 - 6.0;
        let box_y = py + 16.0;
        Self::fill_border(ui, box_x - 1.0, box_y - 1.0, PREVIEW + 2.0, PREVIEW + 2.0);
        Self::fill(ui, box_x, box_y, PREVIEW, PREVIEW, LISTBOX_BG);
        let Some(entry) = self.selected.and_then(|i| self.entries.get(i)) else {
            return;
        };
        Self::draw_named(ui, box_x, box_y, PREVIEW, PREVIEW, &entry.key);
        let (verdict_color, verdict) = if entry.valid {
            (OK_GREEN, "Valid 24x24 emblem")
        } else {
            (ERR_RED, entry.verdict.as_str())
        };
        ui.text(px, box_y + PREVIEW + 16.0, &entry.name, TEXT);
        for (i, line) in wrap(verdict, 16).iter().enumerate() {
            ui.text(
                px,
                box_y + PREVIEW + 32.0 + i as f32 * 13.0,
                line,
                verdict_color,
            );
        }
    }

    fn build_footer(&mut self, ui: &mut UiFrame, x: f32, footer_y: f32) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let can_ok = self
            .selected
            .and_then(|i| self.entries.get(i))
            .map(|e| e.valid)
            .unwrap_or(false);

        let cancel_rect = Rect::new(x + WIN_W - 50.0, footer_y + 4.0, 42.0, 20.0);
        let cancel = ui.button(CANCEL_BTN_ID, cancel_rect, &CANCEL_BTN, "Cancel");
        if cancel.hovered() {
            ui.any_interactive_hovered = true;
        }
        if cancel.clicked() {
            self.open = false;
            return events;
        }

        if can_ok {
            let ok_rect = Rect::new(x + WIN_W - 96.0, footer_y + 4.0, 42.0, 20.0);
            let ok = ui.button(OK_BTN_ID, ok_rect, &OK_BTN, "OK");
            if ok.hovered() {
                ui.any_interactive_hovered = true;
            }
            if ok.clicked() {
                if let Some(entry) = self.selected.and_then(|i| self.entries.get(i)) {
                    events.push(GameEvent::RequestUploadEmblem {
                        path: entry.path.clone(),
                    });
                }
                self.open = false;
            }
        }
        events
    }

    fn fill_border(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32) {
        Self::fill(ui, x, y, w, 1.0, BORDER);
        Self::fill(ui, x, y + h - 1.0, w, 1.0, BORDER);
        Self::fill(ui, x, y, 1.0, h, BORDER);
        Self::fill(ui, x + w - 1.0, y, 1.0, h, BORDER);
    }
}

fn wrap(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > max_chars {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
