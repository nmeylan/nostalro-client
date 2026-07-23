use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    FOOTER_TEX, SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_footer, draw_sys_button,
    draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_game::quest::Quest;
use ragnarok_game::quest::QuestLog;
use ragnarok_ui::draw::{self, DrawCall, TextureRef, strip_color_codes, word_wrap};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const QUEST_WINDOW_ID: WidgetId = WidgetId(3700);
const CLOSE_BTN_ID: WidgetId = WidgetId(3701);
const VIEW_BTN_ID: WidgetId = WidgetId(3702);
const FOOTER_CLOSE_BTN_ID: WidgetId = WidgetId(3703);
const TAB_BASE_ID: u32 = 3710;
const ROW_BASE_ID: u32 = 3720;
const LIST_SCROLL: ScrollbarIds = ScrollbarIds {
    up: WidgetId(3704),
    down: WidgetId(3705),
    thumb: WidgetId(3706),
};

pub const QUEST_DETAIL_WINDOW_ID: WidgetId = WidgetId(3750);
const DETAIL_CLOSE_BTN_ID: WidgetId = WidgetId(3751);

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";
const TAB_QUE_TEX: [&str; 3] = [
    "data/texture/유저인터페이스/basic_interface/tab_que_01.bmp",
    "data/texture/유저인터페이스/basic_interface/tab_que_02.bmp",
    "data/texture/유저인터페이스/basic_interface/tab_que_03.bmp",
];
const DETAIL_BG_TEX: &str = "data/texture/유저인터페이스/basic_interface/quest_window.bmp";

const VIEW_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_view.bmp",
    hover: "data/texture/유저인터페이스/btn_view_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_view_b.bmp",
};
const CLOSE_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_close.bmp",
    hover: "data/texture/유저인터페이스/btn_close_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_close_b.bmp",
};

const WIN_W: f32 = 280.0;
const WIN_H: f32 = 200.0;
const TITLE_H: f32 = 17.0;
const FOOTER_H: f32 = 24.0;
const CLOSE_BTN_SIZE: f32 = 11.0;
const TAB_STRIP_W: f32 = 20.0;
const TAB_STRIP_H: f32 = 94.0;
const TAB_COUNT: usize = 3;
const ROW_H: f32 = 36.0;
const ICON: f32 = 24.0;

const DETAIL_W: f32 = 350.0;
const DETAIL_H: f32 = 375.0;
const DETAIL_IMG_W: f32 = 90.0;
const DETAIL_IMG_H: f32 = 60.0;
const DESC_W: f32 = 300.0;

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const PANEL_BG: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const BORDER: [f32; 4] = [0.761, 0.761, 0.761, 1.0];
const SELECTION_COLOR: [f32; 4] = [0.451, 0.62, 0.937, 1.0];
const TAB_BAR_BG: [f32; 4] = [0.71, 0.714, 0.71, 1.0];
const TAB_INACTIVE: [f32; 4] = [0.808, 0.808, 0.808, 1.0];
const TEXT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const LABEL_COLOR: [f32; 4] = [0.14, 0.20, 0.38, 1.0];

const TAB_LABELS: [&str; TAB_COUNT] = ["On", "Off", "All"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum QuestTab {
    On = 0,
    Off = 1,
    All = 2,
}

impl QuestTab {
    fn from_index(i: usize) -> Self {
        match i {
            0 => Self::On,
            1 => Self::Off,
            _ => Self::All,
        }
    }

    fn matches(self, quest: &Quest) -> bool {
        match self {
            Self::On => quest.active,
            Self::Off => !quest.active,
            Self::All => true,
        }
    }
}

fn fill(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    let (v, i) = draw::quad_vertices(x, y, w, h, color);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::White,
    });
}

fn textured(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, path: &str) {
    let (v, i) = draw::quad_vertices(x, y, w, h, WHITE);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::Named(path.to_string()),
    });
}

fn quest_title(data: &DataTable, id: u32) -> String {
    data.quest_display
        .as_ref()
        .map(|t| t.title(id))
        .unwrap_or_else(|| format!("Quest {id}"))
}

pub struct QuestWindow {
    open: bool,
    has_grf_textures: bool,
    tab: QuestTab,
    scroll: usize,
    selected: Option<u32>,
    pending_toggle: Option<(u32, bool)>,
}

impl Default for QuestWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl QuestWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            has_grf_textures: false,
            tab: QuestTab::On,
            scroll: 0,
            selected: None,
            pending_toggle: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    fn tab_strip(&mut self, ui: &mut UiFrame, x: f32, y: f32, grf: bool) {
        if grf {
            textured(
                ui,
                x,
                y,
                TAB_STRIP_W,
                TAB_STRIP_H,
                TAB_QUE_TEX[self.tab as usize],
            );
        } else {
            fill(ui, x, y, TAB_STRIP_W, TAB_STRIP_H, TAB_BAR_BG);
        }
        let tab_h = TAB_STRIP_H / TAB_COUNT as f32;
        for i in 0..TAB_COUNT {
            let ty = y + i as f32 * tab_h;
            let rect = Rect::new(x, ty, TAB_STRIP_W, tab_h);
            let selected = self.tab as usize == i;
            if !grf {
                fill(
                    ui,
                    x + 1.0,
                    ty + 1.0,
                    TAB_STRIP_W - 2.0,
                    tab_h - 2.0,
                    if selected { WHITE } else { TAB_INACTIVE },
                );
                ui.text(x + 2.0, ty + tab_h / 2.0 + 4.0, TAB_LABELS[i], TEXT);
            }
            let resp = ui.interact(WidgetId(TAB_BASE_ID + i as u32), rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() && !selected {
                self.tab = QuestTab::from_index(i);
                self.scroll = 0;
            }
        }
    }

    fn list_panel(
        &mut self,
        ui: &mut UiFrame,
        quest_log: &QuestLog,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        data: &DataTable,
    ) {
        fill(ui, x, y, w, h, PANEL_BG);

        let ids: Vec<u32> = quest_log
            .quests
            .iter()
            .filter(|q| self.tab.matches(q))
            .map(|q| q.id)
            .collect();

        let visible = (h / ROW_H).max(1.0) as usize;
        let max_scroll = ids.len().saturating_sub(visible);
        let has_bar = max_scroll > 0;
        let bar_w = if has_bar { SCROLLBAR_W } else { 0.0 };
        if has_bar {
            let content_rect = Rect::new(x, y, w, h);
            self.scroll = scrollbar::scrollbar(
                ui,
                LIST_SCROLL,
                self.scroll,
                visible,
                max_scroll,
                content_rect,
                x + w - SCROLLBAR_W,
                y,
                h,
            );
        } else {
            self.scroll = 0;
        }

        for row in 0..visible {
            let Some(&id) = ids.get(self.scroll + row) else {
                break;
            };
            let row_y = y + row as f32 * ROW_H;
            let row_rect = Rect::new(x, row_y, w - bar_w, ROW_H);
            if self.selected == Some(id) {
                fill(
                    ui,
                    row_rect.x,
                    row_rect.y,
                    row_rect.w,
                    row_rect.h,
                    SELECTION_COLOR,
                );
            }
            fill(ui, x, row_y + ROW_H - 1.0, w - bar_w, 1.0, BORDER);

            let icon_y = row_y + (ROW_H - ICON) / 2.0;
            let icon = data
                .quest_display
                .as_ref()
                .map(|t| t.icon_texture(id))
                .unwrap_or_default();
            if self.has_grf_textures && !icon.is_empty() {
                textured(ui, x + 4.0, icon_y, ICON, ICON, &icon);
            } else {
                fill(ui, x + 4.0, icon_y, ICON, ICON, TAB_INACTIVE);
            }

            let title = quest_title(data, id);
            ui.text(
                x + 4.0 + ICON + 6.0,
                row_y + ROW_H / 2.0 + 4.0,
                &title,
                TEXT,
            );

            let resp = ui.interact(WidgetId(ROW_BASE_ID + row as u32), row_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() {
                self.selected = Some(id);
            }
            if resp.right_clicked() {
                self.selected = Some(id);
                let active = quest_log
                    .quests
                    .iter()
                    .find(|q| q.id == id)
                    .map(|q| q.active)
                    .unwrap_or(false);
                self.pending_toggle = Some((id, !active));
            }
        }
    }
}

impl Window for QuestWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn window_size(&self) -> (f32, f32) {
        (WIN_W, WIN_H)
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            FOOTER_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            TAB_QUE_TEX[0],
            TAB_QUE_TEX[1],
            TAB_QUE_TEX[2],
            VIEW_BTN.normal,
            VIEW_BTN.hover,
            VIEW_BTN.pressed,
            CLOSE_BTN.normal,
            CLOSE_BTN.hover,
            CLOSE_BTN.pressed,
        ];
        paths.extend(scrollbar::grf_texture_paths());
        paths
    }
}

impl InGameWindow for QuestWindow {
    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let data = ctx.data;
        let quest_log = ctx.quest_log;
        if !self.open {
            return Vec::new();
        }
        if let Some(sel) = self.selected
            && !quest_log.quests.iter().any(|q| q.id == sel)
        {
            self.selected = None;
        }
        let mut events = Vec::new();
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        self.pending_toggle = None;

        let win = ui.window_at(QUEST_WINDOW_ID, WIN_W, WIN_H, TITLE_H, 0.0, 120.0);
        let (x, y) = (win.x, win.y);
        ui.interact(QUEST_WINDOW_ID, Rect::new(x, y, WIN_W, WIN_H));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        ui.text(x + 16.0, y + TITLE_H - 3.0, "Quest List", tc);

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

        let body_y = y + TITLE_H;
        let body_h = WIN_H - TITLE_H - FOOTER_H;
        self.tab_strip(ui, x, body_y, grf);
        let list_x = x + TAB_STRIP_W;
        let list_w = WIN_W - TAB_STRIP_W;
        self.list_panel(ui, quest_log, list_x, body_y, list_w, body_h, data);
        fill(ui, x + TAB_STRIP_W, body_y, 1.0, body_h, BORDER);

        let footer_y = y + WIN_H - FOOTER_H;
        draw_footer(ui, x, footer_y, WIN_W, FOOTER_H, grf);
        let view_rect = Rect::new(x + 6.0, footer_y + 2.0, 42.0, 20.0);
        if ui
            .button(VIEW_BTN_ID, view_rect, &VIEW_BTN, "View")
            .clicked()
        {
            if let Some(id) = self.selected {
                events.push(GameEvent::OpenQuestDetail { quest_id: id });
            }
        }
        let close2_rect = Rect::new(x + WIN_W - 48.0, footer_y + 2.0, 42.0, 20.0);
        if ui
            .button(FOOTER_CLOSE_BTN_ID, close2_rect, &CLOSE_BTN, "Close")
            .clicked()
        {
            self.open = false;
            ui.has_grf_textures = prev_grf;
            return events;
        }

        if let Some((quest_id, active)) = self.pending_toggle.take() {
            events.push(GameEvent::RequestToggleQuestActive { quest_id, active });
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

pub struct QuestDetailWindow {
    open: bool,
    has_grf_textures: bool,
    quest_id: Option<u32>,
    wrapped_desc: Vec<String>,
    desc_source: u32,
}

impl Default for QuestDetailWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl QuestDetailWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            has_grf_textures: false,
            quest_id: None,
            wrapped_desc: Vec::new(),
            desc_source: u32::MAX,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn quest_id(&self) -> Option<u32> {
        self.quest_id
    }

    pub fn open(&mut self, quest_id: u32) {
        self.quest_id = Some(quest_id);
        self.open = true;
        self.wrapped_desc.clear();
        self.desc_source = u32::MAX;
    }

    pub fn close(&mut self) {
        self.open = false;
    }
}

impl Window for QuestDetailWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn window_size(&self) -> (f32, f32) {
        (DETAIL_W, DETAIL_H)
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        vec![DETAIL_BG_TEX, CLOSE_OFF_TEX, CLOSE_ON_TEX]
    }
}

impl InGameWindow for QuestDetailWindow {
    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let data = ctx.data;
        if !self.open {
            return Vec::new();
        }
        let Some(quest) = self.quest_id.and_then(|id| ctx.quest_log.get(id)) else {
            self.open = false;
            return Vec::new();
        };
        let events = Vec::new();
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;

        let win = ui.window_at(
            QUEST_DETAIL_WINDOW_ID,
            DETAIL_W,
            DETAIL_H,
            20.0,
            370.0,
            60.0,
        );
        let (x, y) = (win.x, win.y);
        ui.interact(QUEST_DETAIL_WINDOW_ID, Rect::new(x, y, DETAIL_W, DETAIL_H));

        if grf {
            textured(ui, x, y, DETAIL_W, DETAIL_H, DETAIL_BG_TEX);
        } else {
            fill(ui, x, y, DETAIL_W, DETAIL_H, PANEL_BG);
            fill(ui, x, y, DETAIL_W, 1.0, BORDER);
        }

        let close_rect = Rect::new(x + DETAIL_W - 14.0, y + 4.0, CLOSE_BTN_SIZE, CLOSE_BTN_SIZE);
        let close_resp = ui.interact(DETAIL_CLOSE_BTN_ID, close_rect);
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

        let id = quest.id;
        if grf {
            let img = data
                .quest_display
                .as_ref()
                .map(|t| t.image_texture(id))
                .unwrap_or_default();
            textured(ui, x + 15.0, y + 30.0, DETAIL_IMG_W, DETAIL_IMG_H, &img);
        } else {
            fill(
                ui,
                x + 15.0,
                y + 30.0,
                DETAIL_IMG_W,
                DETAIL_IMG_H,
                TAB_INACTIVE,
            );
        }

        ui.text(x + 120.0, y + 70.0, &quest_title(data, id), TEXT);

        if let Some(end) = quest.end_time {
            ui.text(x + 120.0, y + 68.0, &format_end_time(end), LABEL_COLOR);
        }

        let summary = data
            .quest_display
            .as_ref()
            .map(|t| t.summary(id))
            .unwrap_or_default();
        ui.colored_text(x + 25.0, y + 154.0, &summary, TEXT);

        if self.desc_source != id {
            let full = data
                .quest_display
                .as_ref()
                .map(|t| t.description(id))
                .unwrap_or_default();
            self.wrapped_desc = word_wrap(
                &full,
                DESC_W,
                |t| ui.atlas.measure_text(&strip_color_codes(t)),
                false,
            );
            self.desc_source = id;
        }
        for (i, line) in self.wrapped_desc.iter().enumerate() {
            ui.colored_text(x + 25.0, y + 160.0 + i as f32 * 14.0, line, TEXT);
        }

        for (i, obj) in quest.objectives.iter().take(3).enumerate() {
            let oy = y + 338.0 + i as f32 * 14.0;
            ui.text(x + 25.0, oy, &obj.name, TEXT);
            ui.text(
                x + 180.0,
                oy,
                &format!("{}/{}", obj.current, obj.required),
                TEXT,
            );
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

fn format_end_time(end: u32) -> String {
    let secs = end as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };

    format!("~ {year:04}/{month:02}/{day:02} {hour:02}:{minute:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;
    use ragnarok_game::quest::{QuestListEntry, QuestObjective};
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    #[test]
    fn right_click_row_requests_active_toggle() {
        let mut log = QuestLog::default();
        log.set_list_entry(QuestListEntry {
            id: 500,
            active: true,
        });
        log.set_mission(ragnarok_game::quest::QuestMissionData {
            id: 500,
            end_time: None,
            objectives: vec![QuestObjective {
                mob_id: 1002,
                name: "Poring".into(),
                current: 1,
                required: 5,
            }],
        });

        let mut win = QuestWindow::new();
        win.toggle();

        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        // First row, inside the list panel (below the 17px titlebar).
        ctx.mouse_x = TAB_STRIP_W + 40.0;
        ctx.mouse_y = 120.0 + TITLE_H + ROW_H / 2.0;
        ctx.mouse_right_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        let mut build_ctx = crate::BuildCtx::test(&mut character, &data);
        build_ctx.quest_log = &log;
        let events = win.build(&mut ui, &mut build_ctx);

        assert!(
            events.iter().any(|e| matches!(
                e,
                GameEvent::RequestToggleQuestActive {
                    quest_id: 500,
                    active: false
                }
            )),
            "expected toggle-active event, got {events:?}"
        );
    }
}
