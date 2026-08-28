use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_sys_button,
    draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::emotion::EMOTION_TABLE;
use ragnarok_game::event::GameEvent;
use ragnarok_game::keybinding::{EmotionKeys, HotkeyAction, KeyBindings, KeyChord};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const HOTKEY_CONFIG_WINDOW_ID: WidgetId = WidgetId(4300);
const CLOSE_BTN_ID: WidgetId = WidgetId(4301);
const OK_BTN_ID: WidgetId = WidgetId(4302);
const CANCEL_BTN_ID: WidgetId = WidgetId(4303);
const RESET_BTN_ID: WidgetId = WidgetId(4304);
const TAB_BASE_ID: u32 = 4305;
const PREV_BTN_ID: WidgetId = WidgetId(4308);
const NEXT_BTN_ID: WidgetId = WidgetId(4309);
const FOOTER_CLOSE_BTN_ID: WidgetId = WidgetId(4310);
const CELL_BASE_ID: u32 = 4350;

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
const RESET_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_RESET,
    hover: ragnarok_resources::ui::BTN_RESET_A,
    pressed: ragnarok_resources::ui::BTN_RESET_B,
};

const WIN_W: f32 = 380.0;
const TITLE_H: f32 = 17.0;
const TAB_H: f32 = 20.0;
const FOOTER_H: f32 = 30.0;
const WARN_H: f32 = 16.0;
const ROW_H: f32 = 20.0;
const ROWS_PER_PAGE: usize = 18;
const ITEMS_PER_PAGE: usize = ROWS_PER_PAGE * 2;
const PAD: f32 = 8.0;
const COL_W: f32 = (WIN_W - PAD * 2.0) / 2.0;
const LABEL_W: f32 = 96.0;
const CELL_GAP: f32 = 4.0;
const CELL_W: f32 = COL_W - LABEL_W - CELL_GAP;
const CELL_H: f32 = 16.0;
const CLOSE_BTN_SIZE: f32 = 11.0;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

const WIN_H: f32 = TITLE_H + TAB_H + PAD + ROWS_PER_PAGE as f32 * ROW_H + WARN_H + FOOTER_H;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Interface,
    Emotion,
}

impl Tab {
    const ALL: [Tab; 2] = [Tab::Interface, Tab::Emotion];
    fn label(self) -> &'static str {
        match self {
            Tab::Interface => "Interface",
            Tab::Emotion => "Emotion",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CaptureTarget {
    Interface(HotkeyAction),
    Emotion(u8),
}

/// The key-binding sets committed together when OK is pressed.
pub struct DirtyBindings {
    pub interface: KeyBindings,
    pub emotion: EmotionKeys,
}

struct Row {
    label: String,
    target: CaptureTarget,
    chord: Option<KeyChord>,
}

pub struct HotkeyConfigWindow {
    has_grf_textures: bool,
    open: bool,
    staged_interface: KeyBindings,
    staged_emotion: EmotionKeys,
    active_tab: Tab,
    page: usize,
    capturing: Option<CaptureTarget>,
    warning: Option<String>,
    dirty: bool,
    btn_size: (f32, f32),
    reset_size: (f32, f32),
}

impl Default for HotkeyConfigWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl HotkeyConfigWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            open: false,
            staged_interface: KeyBindings::defaults(),
            staged_emotion: EmotionKeys::default(),
            active_tab: Tab::Interface,
            page: 0,
            capturing: None,
            warning: None,
            dirty: false,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            reset_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if !self.open {
            self.reset_transient();
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.reset_transient();
    }

    fn reset_transient(&mut self) {
        self.capturing = None;
        self.warning = None;
    }

    pub fn set_bindings(&mut self, interface: &KeyBindings, emotion: &EmotionKeys) {
        self.staged_interface = interface.clone();
        self.staged_emotion = emotion.clone();
        self.page = 0;
        self.reset_transient();
    }

    pub fn is_capturing(&self) -> bool {
        self.open && self.capturing.is_some()
    }

    pub fn cancel_capture(&mut self) {
        self.capturing = None;
    }

    pub fn capture_key(&mut self, chord: KeyChord) {
        let Some(target) = self.capturing else {
            return;
        };
        let reserved = match target {
            CaptureTarget::Interface(_) => chord.is_reserved(),
            CaptureTarget::Emotion(_) => chord.is_reserved_trigger(),
        };
        if reserved {
            self.warning = Some("Key reserved".to_string());
            return;
        }
        if let Some(other) = self.conflict(target, &chord) {
            self.warning = Some(format!("Already assigned to {other}"));
            return;
        }
        match target {
            CaptureTarget::Interface(action) => self.staged_interface.set(action, chord),
            CaptureTarget::Emotion(emote) => self.staged_emotion.set(emote, chord),
        }
        self.capturing = None;
        self.warning = None;
    }

    fn conflict(&self, target: CaptureTarget, chord: &KeyChord) -> Option<String> {
        match target {
            CaptureTarget::Interface(action) => self
                .staged_interface
                .conflict(chord, action)
                .map(|a| a.label().to_string()),
            CaptureTarget::Emotion(emote) => {
                self.staged_emotion.conflict(chord, emote).map(emote_label)
            }
        }
    }

    pub fn take_dirty_bindings(&mut self) -> Option<DirtyBindings> {
        if self.dirty {
            self.dirty = false;
            Some(DirtyBindings {
                interface: self.staged_interface.clone(),
                emotion: self.staged_emotion.clone(),
            })
        } else {
            None
        }
    }

    fn rows(&self) -> Vec<Row> {
        match self.active_tab {
            Tab::Interface => HotkeyAction::ALL
                .iter()
                .map(|&action| Row {
                    label: action.label().to_string(),
                    target: CaptureTarget::Interface(action),
                    chord: self.staged_interface.get(action).cloned(),
                })
                .collect(),
            Tab::Emotion => EMOTION_TABLE
                .iter()
                .map(|e| Row {
                    label: emote_label(e.emote_type),
                    target: CaptureTarget::Emotion(e.emote_type),
                    chord: self.staged_emotion.get(e.emote_type).cloned(),
                })
                .collect(),
        }
    }

    fn reset_active_tab(&mut self) {
        match self.active_tab {
            Tab::Interface => self.staged_interface = KeyBindings::defaults(),
            Tab::Emotion => self.staged_emotion = EmotionKeys::default(),
        }
        self.reset_transient();
    }

    fn draw_cell(&self, ui: &mut UiFrame, rect: Rect, capturing: bool) {
        let push = |ui: &mut UiFrame, x, y, w, h, c| {
            let (v, i) = draw::quad_vertices(x, y, w, h, c);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        };
        let inner = if capturing {
            [0.78, 0.86, 1.0, 1.0]
        } else {
            [1.0, 1.0, 0.87, 1.0]
        };
        push(ui, rect.x, rect.y, rect.w, rect.h, [0.55, 0.55, 0.6, 1.0]);
        push(
            ui,
            rect.x + 1.0,
            rect.y + 1.0,
            rect.w - 2.0,
            rect.h - 2.0,
            inner,
        );
    }
}

fn emote_label(emote_type: u8) -> String {
    EMOTION_TABLE
        .iter()
        .find(|e| e.emote_type == emote_type)
        .map(|e| format!("/{}", e.command))
        .unwrap_or_else(|| format!("emote {emote_type}"))
}

impl Window for HotkeyConfigWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(OK_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(RESET_BTN.normal) {
            self.reset_size = (w as f32, h as f32);
        }
    }
    fn window_size(&self) -> (f32, f32) {
        (WIN_W, WIN_H)
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            TITLEBAR_TEX,
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
            RESET_BTN.normal,
            RESET_BTN.hover,
            RESET_BTN.pressed,
        ]
    }
}

impl InGameWindow for HotkeyConfigWindow {
    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.is_open() && self.capturing.is_none()
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.close();
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let _character = &mut *ctx.character;
        let _data = ctx.data;
        if !self.open {
            return Vec::new();
        }
        let events = Vec::new();
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);

        let default_x = (ui.ctx.screen_width - WIN_W) / 2.0;
        let default_y = (ui.ctx.screen_height - WIN_H) / 2.0;
        let win = ui.window_at(
            HOTKEY_CONFIG_WINDOW_ID,
            WIN_W,
            WIN_H,
            TITLE_H,
            default_x,
            default_y,
        );
        let (x, y) = (win.x, win.y);
        ui.interact(HOTKEY_CONFIG_WINDOW_ID, Rect::new(x, y, WIN_W, WIN_H));

        let body_y = y + TITLE_H;
        let body_h = WIN_H - TITLE_H - FOOTER_H;
        draw_container(ui, x, body_y, WIN_W, body_h, grf);

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        ui.text(
            x + 16.0,
            y + TITLE_H - 3.0,
            "Shortcut key setting window",
            tc,
        );

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
            self.close();
            ui.has_grf_textures = prev_grf;
            return events;
        }

        // Tab bar
        let tab_w = WIN_W / Tab::ALL.len() as f32;
        for (i, &tab) in Tab::ALL.iter().enumerate() {
            let tab_rect = Rect::new(x + i as f32 * tab_w, body_y, tab_w, TAB_H);
            let resp = ui.interact(WidgetId(TAB_BASE_ID + i as u32), tab_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            let active = self.active_tab == tab;
            let bg = if active {
                [1.0, 1.0, 0.87, 1.0]
            } else {
                [0.72, 0.72, 0.72, 1.0]
            };
            let (v, idx) = draw::quad_vertices(tab_rect.x, tab_rect.y, tab_rect.w, tab_rect.h, bg);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::White,
            });
            ui.text(tab_rect.x + 10.0, tab_rect.y + TAB_H - 6.0, tab.label(), tc);
            if resp.clicked() && !active {
                self.active_tab = tab;
                self.page = 0;
                self.reset_transient();
            }
        }

        let rows = self.rows();
        let total_pages = rows.len().div_ceil(ITEMS_PER_PAGE).max(1);
        if self.page >= total_pages {
            self.page = 0;
        }
        let start = self.page * ITEMS_PER_PAGE;
        let page_rows = &rows[start..(start + ITEMS_PER_PAGE).min(rows.len())];

        let grid_y = body_y + TAB_H + PAD;
        let left_count = page_rows.len().div_ceil(2);
        for (j, row) in page_rows.iter().enumerate() {
            let (col, line) = if j < left_count {
                (0usize, j)
            } else {
                (1usize, j - left_count)
            };
            let col_x = x + PAD + col as f32 * COL_W;
            let row_y = grid_y + line as f32 * ROW_H;

            ui.text(col_x, row_y + 12.0, &row.label, tc);

            let cell_rect = Rect::new(col_x + LABEL_W + CELL_GAP, row_y, CELL_W, CELL_H);
            let cell_resp = ui.interact(WidgetId(CELL_BASE_ID + j as u32), cell_rect);
            if cell_resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if cell_resp.clicked() {
                self.capturing = Some(row.target);
                self.warning = None;
            }
            let capturing = self.capturing == Some(row.target);
            self.draw_cell(ui, cell_rect, capturing);
            let text = if capturing {
                "...".to_string()
            } else {
                row.chord
                    .as_ref()
                    .map(|c| c.display())
                    .unwrap_or_else(|| "undesignated".to_string())
            };
            ui.text(cell_rect.x + 4.0, cell_rect.y + 12.0, &text, tc);
        }

        // Pagination indicator + arrows
        if total_pages > 1 {
            let indicator = format!("{} / {}", self.page + 1, total_pages);
            ui.text(x + WIN_W - 84.0, body_y + TAB_H - 6.0, &indicator, tc);
            if ui
                .text_button(
                    PREV_BTN_ID,
                    Rect::new(x + WIN_W - 40.0, body_y + 2.0, 16.0, 15.0),
                    "<",
                )
                .clicked()
                && self.page > 0
            {
                self.page -= 1;
                self.reset_transient();
            }
            if ui
                .text_button(
                    NEXT_BTN_ID,
                    Rect::new(x + WIN_W - 20.0, body_y + 2.0, 16.0, 15.0),
                    ">",
                )
                .clicked()
                && self.page + 1 < total_pages
            {
                self.page += 1;
                self.reset_transient();
            }
        }

        if let Some(warning) = &self.warning {
            let warn_y = grid_y + ROWS_PER_PAGE as f32 * ROW_H;
            ui.text(x + PAD, warn_y + 12.0, warning, [0.75, 0.15, 0.15, 1.0]);
        }

        let footer_y = y + WIN_H - FOOTER_H;
        draw_footer(ui, x, footer_y, WIN_W, FOOTER_H, grf);

        let (bw, bh) = self.btn_size;
        let (rw, rh) = self.reset_size;
        let reset_rect = Rect::new(x + PAD, footer_y + (FOOTER_H - rh) / 2.0, rw, rh);
        if ui
            .button(RESET_BTN_ID, reset_rect, &RESET_BTN, "reset")
            .clicked()
        {
            self.reset_active_tab();
        }

        let by = footer_y + (FOOTER_H - bh) / 2.0;
        let close_rect = Rect::new(x + WIN_W - PAD - bw, by, bw, bh);
        if ui
            .text_button(FOOTER_CLOSE_BTN_ID, close_rect, "close")
            .clicked()
        {
            self.close();
            ui.has_grf_textures = prev_grf;
            return events;
        }
        let cancel_rect = Rect::new(close_rect.x - CELL_GAP - bw, by, bw, bh);
        if ui
            .button(CANCEL_BTN_ID, cancel_rect, &CANCEL_BTN, "cancel")
            .clicked()
        {
            self.close();
            ui.has_grf_textures = prev_grf;
            return events;
        }
        let ok_rect = Rect::new(cancel_rect.x - CELL_GAP - bw, by, bw, bh);
        if ui.button(OK_BTN_ID, ok_rect, &OK_BTN, "ok").clicked() {
            self.dirty = true;
            self.close();
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;

    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_ui::test_support::test_frame;

    #[test]
    fn staged_capture_flow_across_tabs_produces_dirty_bindings_once() {
        let mut win = HotkeyConfigWindow::new();
        win.set_bindings(&KeyBindings::defaults(), &EmotionKeys::default());
        win.toggle();
        assert!(win.is_open());

        // Interface: reject a conflict, then accept a free chord.
        win.capturing = Some(CaptureTarget::Interface(HotkeyAction::ToggleInventory));
        win.capture_key(KeyChord::new("KeyG", true, false, false));
        assert!(win.is_capturing(), "conflict keeps capture active");
        win.capture_key(KeyChord::new("KeyB", true, false, false));
        assert!(!win.is_capturing());

        // Emotion: F-keys/bare printables are allowed here (reserved only blocks
        // Enter/Escape).
        win.capturing = Some(CaptureTarget::Emotion(0));
        win.capture_key(KeyChord::new("F10", false, false, false));
        assert_eq!(
            win.staged_emotion.get(0),
            Some(&KeyChord::new("F10", false, false, false))
        );

        // Reserved key on Interface is rejected.
        win.capturing = Some(CaptureTarget::Interface(HotkeyAction::ToggleStatus));
        win.capture_key(KeyChord::new("F1", false, false, false));
        assert!(win.is_capturing());
        assert!(win.warning.is_some());

        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 700.0);
        let mut ui = test_frame(&mut ctx, &mut state);
        let mut character = Character::new();
        let data = DataTable::new();
        win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        win.dirty = true;
        win.close();
        let dirty = win.take_dirty_bindings().unwrap();
        assert_eq!(
            dirty.interface.get(HotkeyAction::ToggleInventory),
            Some(&KeyChord::new("KeyB", true, false, false))
        );
        assert_eq!(
            dirty.emotion.get(0),
            Some(&KeyChord::new("F10", false, false, false))
        );
        assert!(win.take_dirty_bindings().is_none());
    }
}
