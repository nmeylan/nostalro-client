use crate::Window;
use ragnarok_game::event::GameEvent;
use ragnarok_game::skill::SkillEnum;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId, WindowOrder};
use ragnarok_ui::rect::Rect;

pub const WARP_LIST_WINDOW_ID: WidgetId = WidgetId(4600);
const OK_ID: WidgetId = WidgetId(4601);
const CANCEL_ID: WidgetId = WidgetId(4602);
const OVERLAY_ID: WidgetId = WidgetId(4603);
const ROW_BASE_ID: u32 = 4610;

const WIN_W: f32 = 200.0;
const PADDING: f32 = 8.0;
const ROW_H: f32 = 18.0;
const MIN_H: f32 = 90.0;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;
const BTN_BOTTOM: f32 = 4.0;
const BTN_FIRST_RIGHT: f32 = 5.0;
const BTN_SPACING: f32 = 3.0;

const WIN_TEXTURE: &str = ragnarok_resources::ui::WIN_MSGBOX;

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

const SELECTED_COLOR: [f32; 4] = [0.3, 0.3, 0.5, 0.5];

struct Destination {
    raw: String,
    display: String,
}

fn display_name(raw: &str) -> String {
    let stripped = raw
        .strip_suffix(".gat")
        .or_else(|| raw.strip_suffix(".rsw"))
        .or_else(|| raw.strip_suffix(".gnd"))
        .unwrap_or(raw);
    stripped.to_string()
}

#[derive(Default)]
pub struct WarpListWindow {
    has_grf_textures: bool,
    open: bool,
    skill: Option<SkillEnum>,
    destinations: Vec<Destination>,
    selected_index: usize,
    btn_size: (f32, f32),
}

impl WarpListWindow {
    pub fn new() -> Self {
        Self {
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            ..Default::default()
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, skill: SkillEnum, destinations: Vec<String>) {
        self.skill = Some(skill);
        self.destinations = destinations
            .into_iter()
            .map(|raw| {
                let display = display_name(&raw);
                Destination { raw, display }
            })
            .collect();
        self.selected_index = 0;
        self.open = !self.destinations.is_empty();
    }

    fn confirm(&mut self, events: &mut Vec<GameEvent>) {
        if let (Some(skill), Some(dest)) = (self.skill, self.destinations.get(self.selected_index))
        {
            events.push(GameEvent::RequestSelectWarppoint {
                skill,
                map_name: dest.raw.clone(),
            });
        }
        self.close();
    }

    pub fn cancel(&mut self, events: &mut Vec<GameEvent>) {
        if let Some(skill) = self.skill {
            events.push(GameEvent::RequestSelectWarppoint {
                skill,
                map_name: "cancel".to_string(),
            });
        }
        self.close();
    }

    fn close(&mut self) {
        self.open = false;
        self.destinations.clear();
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        if !self.open {
            return Vec::new();
        }
        let mut events = Vec::new();

        if ui.ctx.key_up && self.selected_index > 0 {
            self.selected_index -= 1;
        }
        if ui.ctx.key_down && self.selected_index + 1 < self.destinations.len() {
            self.selected_index += 1;
        }
        if ui.ctx.key_enter {
            self.confirm(&mut events);
            return events;
        }
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;

        let (btn_w, btn_h) = self.btn_size;
        let rows_h = self.destinations.len() as f32 * ROW_H;
        let win_h = (PADDING + rows_h + PADDING + btn_h + PADDING).max(MIN_H);
        let dx = ((ui.ctx.screen_width - WIN_W) / 2.0).max(0.0).floor();
        let dy = ((ui.ctx.screen_height - win_h) / 2.0).max(0.0).floor();

        let screen = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);
        ui.interact(OVERLAY_ID, screen);
        ui.ensure_in_z_order_with(WARP_LIST_WINDOW_ID, WindowOrder::Foreground);
        let win = ui.window_fixed(WARP_LIST_WINDOW_ID, WIN_W, win_h, dx, dy);
        ui.interact(WARP_LIST_WINDOW_ID, win);

        if self.has_grf_textures {
            let (v, i) = draw::quad_vertices(dx, dy, WIN_W, win_h, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(WIN_TEXTURE.to_string()),
            });
        } else {
            let (v, i) = draw::quad_vertices(dx, dy, WIN_W, win_h, [0.2, 0.2, 0.28, 0.97]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
            let border = [0.5, 0.5, 0.6, 1.0];
            for (bx, by, bw, bh) in [
                (dx, dy, WIN_W, 1.0),
                (dx, dy + win_h - 1.0, WIN_W, 1.0),
                (dx, dy, 1.0, win_h),
                (dx + WIN_W - 1.0, dy, 1.0, win_h),
            ] {
                let (v, i) = draw::quad_vertices(bx, by, bw, bh, border);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }
        }

        let text_color = if self.has_grf_textures {
            [0.0, 0.0, 0.0, 1.0]
        } else {
            [1.0, 1.0, 1.0, 1.0]
        };

        let mut clicked_index = None;
        for (idx, dest) in self.destinations.iter().enumerate() {
            let row_y = dy + PADDING + idx as f32 * ROW_H;
            let row_rect = Rect::new(dx + PADDING, row_y, WIN_W - PADDING * 2.0, ROW_H);
            let row = ui.interact(WidgetId(ROW_BASE_ID + idx as u32), row_rect);
            if row.hovered() {
                ui.any_interactive_hovered = true;
            }
            if row.clicked() {
                clicked_index = Some(idx);
            }
            if idx == self.selected_index {
                let (v, i) = draw::quad_vertices(
                    row_rect.x,
                    row_rect.y,
                    row_rect.w,
                    row_rect.h,
                    SELECTED_COLOR,
                );
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }
            ui.text(
                row_rect.x + 4.0,
                row_y + ROW_H - 4.0,
                &dest.display,
                text_color,
            );
        }
        if let Some(idx) = clicked_index {
            self.selected_index = idx;
        }

        let win_rect = Rect::new(dx, dy, WIN_W, win_h);
        let btns = win_rect.buttons_bottom_right(
            2,
            btn_w,
            btn_h,
            BTN_BOTTOM,
            BTN_FIRST_RIGHT,
            BTN_SPACING,
        );
        let cancel = ui.button(CANCEL_ID, btns[0], &CANCEL_BTN, "Cancel");
        let ok = ui.button(OK_ID, btns[1], &OK_BTN, "OK");

        if ok.clicked() {
            self.confirm(&mut events);
        } else if cancel.clicked() {
            self.cancel(&mut events);
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl Window for WarpListWindow {
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
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            WIN_TEXTURE,
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_ui::test_support::test_frame;

    #[test]
    fn enter_confirms_selected_destination_with_raw_name() {
        let mut win = WarpListWindow::new();
        win.open(
            SkillEnum::AlWarp,
            vec!["Random".into(), "prontera.gat".into()],
        );
        assert!(win.is_open());
        win.selected_index = 1;

        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        let events = win.build(&mut ui);

        assert!(!win.is_open());
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::RequestSelectWarppoint { skill: SkillEnum::AlWarp, map_name } if map_name == "prontera.gat"
        )));
    }

    #[test]
    fn escape_sends_cancel() {
        let mut win = WarpListWindow::new();
        win.open(SkillEnum::AlTeleport, vec!["Random".into()]);

        let mut events = Vec::new();
        win.cancel(&mut events);

        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::RequestSelectWarppoint { map_name, .. } if map_name == "cancel"
        )));
    }
}
