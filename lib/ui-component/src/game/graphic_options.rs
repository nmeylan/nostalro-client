use crate::helper::CHECKBOX;
use crate::helper::dropdown::Dropdown;
use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_sys_button, draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::display::DisplayOptions;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const GRAPHIC_OPTIONS_WINDOW_ID: WidgetId = WidgetId(4200);
const CLOSE_BTN_ID: WidgetId = WidgetId(4201);
const UI_SCALE_DROPDOWN_ID: WidgetId = WidgetId(4202);
const FULLSCREEN_CB_ID: WidgetId = WidgetId(4203);
const FOG_CB_ID: WidgetId = WidgetId(4204);
const EFFECTS_CB_ID: WidgetId = WidgetId(4205);
const AURA_CB_ID: WidgetId = WidgetId(4206);
const DAMAGE_CB_ID: WidgetId = WidgetId(4207);
const CASTBAR_CB_ID: WidgetId = WidgetId(4208);
const NAME_PLAYER_CB_ID: WidgetId = WidgetId(4209);
const NAME_MONSTER_CB_ID: WidgetId = WidgetId(4210);
const NAME_NPC_CB_ID: WidgetId = WidgetId(4211);
const REFUSE_TRADE_CB_ID: WidgetId = WidgetId(4212);
const REFUSE_PARTY_CB_ID: WidgetId = WidgetId(4213);
const UI_SCALE_OPTION_BASE: u32 = 4230;

const UI_SCALE_OPTIONS: [u32; 6] = [75, 100, 125, 150, 175, 200];

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

const WIN_W: f32 = 280.0;
const TITLE_H: f32 = 20.0;
const CLOSE_SIZE: f32 = 11.0;
const ROW_H: f32 = 22.0;
const ROW_COUNT: f32 = 8.0;
const PAD: f32 = 8.0;
const WIN_H: f32 = TITLE_H + PAD + ROW_COUNT * ROW_H + PAD;
const CB_SIZE: f32 = 11.0;

#[derive(Default)]
pub struct GraphicOptionsWindow {
    pub open: bool,
    pub has_grf_textures: bool,
    selected_ui_scale: usize,
    fullscreen: bool,
    fog: bool,
    show_skill_effects: bool,
    display: DisplayOptions,
    refuse_trade: bool,
    refuse_party_invite: bool,
    dropdown: Dropdown,
}

impl GraphicOptionsWindow {
    pub fn new() -> Self {
        Self {
            show_skill_effects: true,
            ..Default::default()
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_values(
        &mut self,
        ui_scale_percent: f32,
        fullscreen: bool,
        fog: bool,
        show_skill_effects: bool,
        display: DisplayOptions,
        refuse_trade: bool,
        refuse_party_invite: bool,
    ) {
        self.selected_ui_scale = (0..UI_SCALE_OPTIONS.len())
            .min_by_key(|&i| (UI_SCALE_OPTIONS[i] as f32 - ui_scale_percent).abs() as u32)
            .unwrap_or(1);
        self.fullscreen = fullscreen;
        self.fog = fog;
        self.show_skill_effects = show_skill_effects;
        self.display = display;
        self.refuse_trade = refuse_trade;
        self.refuse_party_invite = refuse_party_invite;
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    fn changed_event(&self) -> GameEvent {
        let ui_scale = UI_SCALE_OPTIONS
            .get(self.selected_ui_scale)
            .copied()
            .unwrap_or(100) as f32;
        GameEvent::GraphicsSettingsChanged {
            ui_scale,
            fullscreen: self.fullscreen,
            fog: self.fog,
            show_skill_effects: self.show_skill_effects,
            display: self.display.clone(),
            refuse_trade: self.refuse_trade,
            refuse_party_invite: self.refuse_party_invite,
            persist: true,
        }
    }
}

impl Window for GraphicOptionsWindow {
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
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            CHECKBOX.off,
            CHECKBOX.on,
        ];
        paths.extend(crate::helper::dropdown::grf_texture_paths());
        paths
    }
}

impl InGameWindow for GraphicOptionsWindow {
    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let _character = &mut *ctx.character;
        let _data = ctx.data;
        if !self.open {
            return Vec::new();
        }
        let mut events = Vec::new();
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;

        let default_x = (ui.ctx.screen_width - WIN_W) / 2.0;
        let default_y = (ui.ctx.screen_height - WIN_H) / 2.0;
        let win = ui.window_at(
            GRAPHIC_OPTIONS_WINDOW_ID,
            WIN_W,
            WIN_H,
            TITLE_H,
            default_x,
            default_y,
        );
        ui.interact(
            GRAPHIC_OPTIONS_WINDOW_ID,
            Rect::new(win.x, win.y, WIN_W, WIN_H),
        );

        crate::helper::fallback::window_body(ui, win.x, win.y + TITLE_H, WIN_W, WIN_H - TITLE_H);

        draw_titlebar(ui, win.x, win.y, WIN_W, TITLE_H, grf);
        ui.text(
            win.x + 20.0,
            win.y + TITLE_H - 5.0,
            "Graphic Settings",
            text_color(grf),
        );

        let close_rect = Rect::new(
            win.x + WIN_W - CLOSE_SIZE - 4.0,
            win.y + 4.0,
            CLOSE_SIZE,
            CLOSE_SIZE,
        );
        let close_resp = ui.interact(CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        draw_sys_button(
            ui,
            close_rect,
            (CLOSE_SIZE, CLOSE_SIZE),
            close_resp.hovered(),
            grf,
            CLOSE_ON_TEX,
            CLOSE_OFF_TEX,
            Some('x'),
        );
        if close_resp.clicked() || ui.ctx.key_escape {
            self.open = false;
            self.dropdown.open = false;
            ui.has_grf_textures = prev_grf;
            return events;
        }

        let label_color = text_color(grf);
        let row_y = |n: usize| win.y + TITLE_H + PAD + n as f32 * ROW_H;
        let cb_y = |n: usize| row_y(n) + (ROW_H - CB_SIZE) / 2.0 - 2.0;
        let text_y = |n: usize| row_y(n) + 13.0;
        let mut changed = false;

        // Row 0: UI scale dropdown + full screen
        ui.text(win.x + 10.0, text_y(0), "UI Scale", label_color);
        let labels: Vec<String> = UI_SCALE_OPTIONS.iter().map(|p| format!("{p}%")).collect();
        let selected_label = labels
            .get(self.selected_ui_scale)
            .cloned()
            .unwrap_or_default();
        let dd_rect = Rect::new(win.x + 75.0, row_y(0), 100.0, 16.0);
        let screen = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);
        self.dropdown.begin_frame();
        let dd_resp = self.dropdown.show(
            ui,
            UI_SCALE_DROPDOWN_ID,
            dd_rect,
            &selected_label,
            labels.len(),
            screen,
            false,
        );

        let fs_cb = Rect::new(win.x + 185.0, cb_y(0), CB_SIZE, CB_SIZE);
        changed |= ui
            .checkbox(FULLSCREEN_CB_ID, fs_cb, &mut self.fullscreen, &CHECKBOX)
            .clicked();
        ui.text(
            fs_cb.x + CB_SIZE + 4.0,
            text_y(0),
            "Full Screen",
            label_color,
        );

        let check_row = |ui: &mut UiFrame,
                         n: usize,
                         id: WidgetId,
                         x: f32,
                         label: &str,
                         value: &mut bool|
         -> bool {
            let rect = Rect::new(x, cb_y(n), CB_SIZE, CB_SIZE);
            let clicked = ui.checkbox(id, rect, value, &CHECKBOX).clicked();
            ui.text(rect.x + CB_SIZE + 4.0, text_y(n), label, label_color);
            clicked
        };

        let x0 = win.x + 10.0;
        changed |= check_row(ui, 1, FOG_CB_ID, x0, "Fog", &mut self.fog);
        changed |= check_row(
            ui,
            2,
            EFFECTS_CB_ID,
            x0,
            "Skill effects",
            &mut self.show_skill_effects,
        );
        changed |= check_row(
            ui,
            3,
            AURA_CB_ID,
            x0,
            "Level 99 aura",
            &mut self.display.show_level_aura,
        );
        changed |= check_row(
            ui,
            4,
            DAMAGE_CB_ID,
            x0,
            "Show other players' damage",
            &mut self.display.show_other_damage,
        );
        changed |= check_row(
            ui,
            5,
            CASTBAR_CB_ID,
            x0,
            "Show cast bars of others",
            &mut self.display.show_other_cast_bars,
        );

        ui.text(x0, text_y(6), "Name plates:", label_color);
        let mut show_player = !self.display.hide_name_player;
        let mut show_monster = !self.display.hide_name_monster;
        let mut show_npc = !self.display.hide_name_npc;
        let names_changed =
            check_row(
                ui,
                6,
                NAME_PLAYER_CB_ID,
                win.x + 92.0,
                "Player",
                &mut show_player,
            ) | check_row(
                ui,
                6,
                NAME_MONSTER_CB_ID,
                win.x + 152.0,
                "Monster",
                &mut show_monster,
            ) | check_row(ui, 6, NAME_NPC_CB_ID, win.x + 220.0, "NPC", &mut show_npc);
        if names_changed {
            self.display.hide_name_player = !show_player;
            self.display.hide_name_monster = !show_monster;
            self.display.hide_name_npc = !show_npc;
            changed = true;
        }

        ui.text(x0, text_y(7), "Auto-refuse:", label_color);
        changed |= check_row(
            ui,
            7,
            REFUSE_TRADE_CB_ID,
            win.x + 92.0,
            "Trade",
            &mut self.refuse_trade,
        );
        changed |= check_row(
            ui,
            7,
            REFUSE_PARTY_CB_ID,
            win.x + 152.0,
            "Party invite",
            &mut self.refuse_party_invite,
        );

        if let Some(overlay) = dd_resp.overlay_rect {
            let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            if let Some(idx) =
                self.dropdown
                    .show_overlay(ui, overlay, UI_SCALE_OPTION_BASE, &label_refs)
            {
                if idx != self.selected_ui_scale {
                    self.selected_ui_scale = idx;
                    changed = true;
                }
            }
        }

        if changed {
            events.push(self.changed_event());
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InGameWindow;
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;
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

    fn build_at(
        win: &mut GraphicOptionsWindow,
        state: &mut StateCache,
        mouse: Option<(f32, f32)>,
    ) -> Vec<GameEvent> {
        let mut ctx = UiContext::new(800.0, 600.0);
        if let Some((x, y)) = mouse {
            ctx.mouse_x = x;
            ctx.mouse_y = y;
            ctx.mouse_clicked = true;
        }
        let mut ui = make_frame(&ctx, state);
        let mut character = Character::new();
        let data = DataTable::new();
        win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
    }

    #[test]
    fn fog_checkbox_and_ui_scale_change_emit_snapshot() {
        let mut win = GraphicOptionsWindow::new();
        win.set_values(
            100.0,
            false,
            false,
            true,
            DisplayOptions::default(),
            false,
            false,
        );
        assert_eq!(win.selected_ui_scale, 1, "100% is the second option");
        win.toggle();
        assert!(win.open);
        let mut state = StateCache::new();

        let wx = (800.0 - WIN_W) / 2.0;
        let wy = (600.0 - WIN_H) / 2.0;
        let row_y = |n: usize| wy + TITLE_H + PAD + n as f32 * ROW_H;
        let cb_y = |n: usize| row_y(n) + (ROW_H - CB_SIZE) / 2.0 - 2.0;

        // Fog checkbox (row 1)
        let events = build_at(
            &mut win,
            &mut state,
            Some((wx + 10.0 + CB_SIZE / 2.0, cb_y(1) + CB_SIZE / 2.0)),
        );
        assert_eq!(events.len(), 1);
        match &events[0] {
            GameEvent::GraphicsSettingsChanged {
                fog,
                persist,
                ui_scale,
                ..
            } => {
                assert!(*fog);
                assert!(*persist);
                assert_eq!(*ui_scale, 100.0);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        // Open the UI-scale dropdown, then pick the second option next frame.
        let dd_x = wx + 75.0 + 10.0;
        let dd_y = row_y(0) + 8.0;
        let events = build_at(&mut win, &mut state, Some((dd_x, dd_y)));
        assert!(events.is_empty());

        let option_y = row_y(0) + 16.0 + 2.0 * crate::helper::dropdown::OPTION_H + 8.0;
        let events = build_at(&mut win, &mut state, Some((dd_x, option_y)));
        assert_eq!(events.len(), 1);
        match &events[0] {
            GameEvent::GraphicsSettingsChanged { ui_scale, fog, .. } => {
                assert_eq!(*ui_scale, 125.0, "picked the third option (125%)");
                assert!(*fog, "earlier fog flip is part of the snapshot");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
