use crate::helper::CHECKBOX;
use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_sys_button, draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const SOUND_OPTIONS_WINDOW_ID: WidgetId = WidgetId(2800);
const BGM_SLIDER_ID: WidgetId = WidgetId(2801);
const SFX_SLIDER_ID: WidgetId = WidgetId(2802);
const BGM_MUTE_ID: WidgetId = WidgetId(2803);
const SFX_MUTE_ID: WidgetId = WidgetId(2804);
const CLOSE_BTN_ID: WidgetId = WidgetId(2805);
const UNFOCUSED_ID: WidgetId = WidgetId(2806);
const STEREO_ID: WidgetId = WidgetId(2807);

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

const WIN_W: f32 = 260.0;
const WIN_H: f32 = 180.0;
const TITLE_H: f32 = 20.0;
const CLOSE_SIZE: f32 = 11.0;
const ROW_H: f32 = 40.0;
const SLIDER_W: f32 = 150.0;
const SLIDER_H: f32 = 18.0;
const CB_SIZE: f32 = 12.0;
/// Checkbox-only rows sit tighter than the slider rows.
const CB_ROW_H: f32 = 24.0;

#[derive(Default)]
pub struct SoundOptionsWindow {
    pub open: bool,
    pub has_grf_textures: bool,
    bgm_volume: f32,
    sfx_volume: f32,
    bgm_enabled: bool,
    sfx_enabled: bool,
    stereo: bool,
    play_when_unfocused: bool,
}

impl SoundOptionsWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            has_grf_textures: false,
            bgm_volume: 0.8,
            sfx_volume: 0.8,
            bgm_enabled: true,
            sfx_enabled: true,
            stereo: true,
            play_when_unfocused: false,
        }
    }

    /// Sync the sliders/toggles to the current config values.
    pub fn set_values(
        &mut self,
        bgm: f32,
        sfx: f32,
        bgm_on: bool,
        sfx_on: bool,
        stereo: bool,
        play_when_unfocused: bool,
    ) {
        self.bgm_volume = bgm;
        self.sfx_volume = sfx;
        self.bgm_enabled = bgm_on;
        self.sfx_enabled = sfx_on;
        self.stereo = stereo;
        self.play_when_unfocused = play_when_unfocused;
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    fn changed_event(&self, persist: bool) -> GameEvent {
        GameEvent::SoundSettingsChanged {
            bgm_volume: self.bgm_volume,
            sfx_volume: self.sfx_volume,
            bgm_enabled: self.bgm_enabled,
            sfx_enabled: self.sfx_enabled,
            stereo: self.stereo,
            play_when_unfocused: self.play_when_unfocused,
            persist,
        }
    }
}

impl Window for SoundOptionsWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            TITLEBAR_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
        ]
    }
}

impl InGameWindow for SoundOptionsWindow {
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
            SOUND_OPTIONS_WINDOW_ID,
            WIN_W,
            WIN_H,
            TITLE_H,
            default_x,
            default_y,
        );
        ui.interact(
            SOUND_OPTIONS_WINDOW_ID,
            Rect::new(win.x, win.y, WIN_W, WIN_H),
        );

        crate::helper::fallback::window_body(ui, win.x, win.y + TITLE_H, WIN_W, WIN_H - TITLE_H);

        draw_titlebar(ui, win.x, win.y, WIN_W, TITLE_H, grf);
        ui.text(
            win.x + 20.0,
            win.y + TITLE_H - 5.0,
            "Sound",
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
            ui.has_grf_textures = prev_grf;
            return events;
        }

        let label_color = text_color(grf);
        let row = |n: usize| win.y + TITLE_H + 12.0 + n as f32 * ROW_H;
        let slider_x = win.x + 60.0;
        let mute_x = win.x + 60.0 + SLIDER_W + 8.0;

        // BGM row
        ui.text(win.x + 12.0, row(0) + SLIDER_H - 4.0, "BGM", label_color);
        let bgm_rect = Rect::new(slider_x, row(0), SLIDER_W, SLIDER_H);
        let bgm_resp = ui.slider(BGM_SLIDER_ID, bgm_rect, &mut self.bgm_volume, 0.0, 1.0);
        if bgm_resp.changed {
            events.push(self.changed_event(false));
        }
        if bgm_resp.released {
            events.push(self.changed_event(true));
        }
        let bgm_cb = Rect::new(
            mute_x,
            row(0) + (SLIDER_H - CB_SIZE) / 2.0,
            CB_SIZE,
            CB_SIZE,
        );
        if ui
            .checkbox(BGM_MUTE_ID, bgm_cb, &mut self.bgm_enabled, &CHECKBOX)
            .clicked()
        {
            events.push(self.changed_event(true));
        }
        ui.text(
            bgm_cb.x + CB_SIZE + 4.0,
            row(0) + SLIDER_H - 4.0,
            "On",
            label_color,
        );

        // SFX row
        ui.text(win.x + 12.0, row(1) + SLIDER_H - 4.0, "SFX", label_color);
        let sfx_rect = Rect::new(slider_x, row(1), SLIDER_W, SLIDER_H);
        let sfx_resp = ui.slider(SFX_SLIDER_ID, sfx_rect, &mut self.sfx_volume, 0.0, 1.0);
        if sfx_resp.changed {
            events.push(self.changed_event(false));
        }
        if sfx_resp.released {
            events.push(self.changed_event(true));
        }
        let sfx_cb = Rect::new(
            mute_x,
            row(1) + (SLIDER_H - CB_SIZE) / 2.0,
            CB_SIZE,
            CB_SIZE,
        );
        if ui
            .checkbox(SFX_MUTE_ID, sfx_cb, &mut self.sfx_enabled, &CHECKBOX)
            .clicked()
        {
            events.push(self.changed_event(true));
        }
        ui.text(
            sfx_cb.x + CB_SIZE + 4.0,
            row(1) + SLIDER_H - 4.0,
            "On",
            label_color,
        );

        let cb_row = |n: usize| row(2) + n as f32 * CB_ROW_H;
        let mut changed = false;
        for (n, id, checked, label) in [
            (0, STEREO_ID, &mut self.stereo, "Stereo"),
            (
                1,
                UNFOCUSED_ID,
                &mut self.play_when_unfocused,
                "Play while unfocused",
            ),
        ] {
            let cb = Rect::new(
                win.x + 12.0,
                cb_row(n) + (SLIDER_H - CB_SIZE) / 2.0,
                CB_SIZE,
                CB_SIZE,
            );
            if ui.checkbox(id, cb, checked, &CHECKBOX).clicked() {
                changed = true;
            }
            ui.text(
                cb.x + CB_SIZE + 6.0,
                cb_row(n) + SLIDER_H - 4.0,
                label,
                label_color,
            );
        }
        if changed {
            events.push(self.changed_event(true));
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mute_toggle_and_value_sync() {
        let mut w = SoundOptionsWindow::new();
        w.set_values(0.5, 0.3, true, false, false, true);
        assert_eq!(w.bgm_volume, 0.5);
        assert!(!w.sfx_enabled);
        assert!(!w.stereo);
        assert!(w.play_when_unfocused);
        w.toggle();
        assert!(w.open);
    }
}
