use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_sys_button,
    draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::emotion::EMOTION_TABLE;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, EMOTION_ICON_PREFIX, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const EMOTION_WINDOW_ID: WidgetId = WidgetId(3500);
const CLOSE_BTN_ID: WidgetId = WidgetId(3501);
const CELL_BASE_ID: u32 = 3510;

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

const COLS: usize = 6;
const CELL: f32 = 32.0;
const ICON_PAD: f32 = 3.0;
const PAD: f32 = 8.0;
const TITLE_H: f32 = 17.0;
const CLOSE_BTN_SIZE: f32 = 11.0;

fn grid_rows() -> usize {
    EMOTION_TABLE.len().div_ceil(COLS)
}
fn win_w() -> f32 {
    PAD * 2.0 + COLS as f32 * CELL
}
fn win_h() -> f32 {
    TITLE_H + PAD * 2.0 + grid_rows() as f32 * CELL
}

pub struct EmotionWindow {
    has_grf_textures: bool,
    open: bool,
    icon_sizes: Vec<Option<(f32, f32)>>,
}

impl Default for EmotionWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl EmotionWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            open: false,
            icon_sizes: vec![None; EMOTION_TABLE.len()],
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
}

impl Window for EmotionWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        for (i, entry) in EMOTION_TABLE.iter().enumerate() {
            self.icon_sizes[i] = size_fn(&format!("{EMOTION_ICON_PREFIX}{}", entry.sprite_action))
                .map(|(w, h)| (w as f32, h as f32));
        }
    }
    fn window_size(&self) -> (f32, f32) {
        (win_w(), win_h())
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            TITLEBAR_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            // Sentinel: loading it registers every @emo/<action> icon at once.
            "@emo/0",
        ]
    }
}

impl InGameWindow for EmotionWindow {
    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.is_open()
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
        let mut events = Vec::new();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);

        let (w, h) = (win_w(), win_h());
        let win = ui.window_at(EMOTION_WINDOW_ID, w, h, TITLE_H, 500.0, 200.0);
        let (x, y) = (win.x, win.y);
        ui.interact(EMOTION_WINDOW_ID, Rect::new(x, y, w, h));

        draw_titlebar(ui, x, y, w, TITLE_H, grf);
        ui.text(x + 16.0, y + TITLE_H - 3.0, "Emoticon", tc);

        let close_rect = Rect::new(
            x + w - CLOSE_BTN_SIZE - 3.0,
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

        let body_y = y + TITLE_H;
        draw_container(ui, x, body_y, w, h - TITLE_H, grf);

        let grid_x = x + PAD;
        let grid_y = body_y + PAD;
        for (i, entry) in EMOTION_TABLE.iter().enumerate() {
            let col = i % COLS;
            let row = i / COLS;
            let cell_x = grid_x + col as f32 * CELL;
            let cell_y = grid_y + row as f32 * CELL;
            let cell_rect = Rect::new(cell_x, cell_y, CELL, CELL);
            let resp = ui.interact(WidgetId(CELL_BASE_ID + i as u32), cell_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
                let (v, idx) =
                    draw::quad_vertices(cell_x, cell_y, CELL, CELL, [1.0, 1.0, 1.0, 0.25]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::White,
                });
                ui.tooltip(cell_x, cell_y, &format!("/{}", entry.command));
            }

            let box_size = CELL - ICON_PAD * 2.0;
            let (iw, ih) = match self.icon_sizes[i] {
                Some((tw, th)) if tw > 0.0 && th > 0.0 => {
                    let s = (box_size / tw).min(box_size / th).min(1.0);
                    (tw * s, th * s)
                }
                _ => (box_size, box_size),
            };
            let ix = cell_x + (CELL - iw) / 2.0;
            let iy = cell_y + (CELL - ih) / 2.0;
            let (v, idx) = draw::quad_vertices_uv(ix, iy, iw, ih, [0.0, 0.0], [1.0, 1.0], [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::Named(format!("{EMOTION_ICON_PREFIX}{}", entry.sprite_action)),
            });

            if resp.clicked() {
                events.push(GameEvent::RequestEmotion {
                    emote_type: entry.emote_type,
                });
            }
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}
