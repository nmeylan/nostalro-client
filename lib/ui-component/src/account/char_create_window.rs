use crate::Window;
use crate::helper::window_chrome;
use ragnarok_game::event::GameEvent;
use ragnarok_renderer::ui_renderer::UiVertex;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

// ---- v1 (stat-allocation) layout: login_interface/win_make.bmp, 576x342 ----
const WIN_W: f32 = 576.0;
const WIN_H: f32 = 342.0;
const TITLE_BAR_H: f32 = 22.0;

const NAME_X: f32 = 62.0;
const NAME_Y: f32 = 244.0;
const NAME_W: f32 = 97.0;
const NAME_H: f32 = 14.0;

const PREVIEW_X: f32 = 62.0;
const PREVIEW_Y: f32 = 120.0;
const PREVIEW_ANCHOR_X: f32 = 32.0;
const PREVIEW_ANCHOR_Y: f32 = 115.0;

const HEAD_PREV_X: f32 = 47.0;
const HEAD_NEXT_X: f32 = 127.0;
const HEAD_ARROW_Y: f32 = 135.0;
const HAIR_UP_X: f32 = 87.0;
const HAIR_UP_Y: f32 = 105.0;

// ---- v2 (no-stat) layout: login_interface/win_make2.bmp, compact 150x286 ----
const V2_W: f32 = 150.0;
const V2_HEADER_H: f32 = 17.0;
const V2_CONTENT_H: f32 = 240.0;
const V2_FOOTER_H: f32 = 29.0;
const V2_H: f32 = V2_HEADER_H + V2_CONTENT_H + V2_FOOTER_H;
const V2_NAME_X: f32 = 40.0;
const V2_NAME_Y: f32 = 142.0;
const V2_NAME_W: f32 = 96.0;
const V2_NAME_H: f32 = 18.0;
const V2_PREVIEW_X: f32 = 40.0;
const V2_PREVIEW_Y: f32 = -10.0;
const V2_STYLE_Y: f32 = 185.0;
const V2_COLOR_Y: f32 = 225.0;
const V2_ARROW_L_X: f32 = 15.0;
const V2_ARROW_R_X: f32 = 125.0;

const ARROW_W: f32 = 13.0;
const ARROW_H: f32 = 13.0;

const BTN_W: f32 = 42.0;
const BTN_H: f32 = 20.0;
const BTN_BOTTOM: f32 = 4.0;
const MAKE_RIGHT: f32 = 50.0;
const CANCEL_RIGHT: f32 = 4.0;

const NAME_MAX_LEN: usize = 23;
const HEAD_MIN: u16 = 1;
const HEAD_MAX: u16 = 26;
const HAIR_COLOR_COUNT: u16 = 9;

// Stat indices: 0=STR 1=AGI 2=VIT 3=INT 4=DEX 5=LUK.
// Pairs (STR↔INT, AGI↔LUK, VIT↔DEX) each sum to 10; raising one lowers its partner.
const STAT_PARTNER: [usize; 6] = [3, 5, 4, 0, 2, 1];
const STAT_START: u8 = 5;
const STAT_MIN: u8 = 1;
const STAT_MAX: u8 = 9;
const STAT_ARROW_W: f32 = 36.0;
const STAT_ARROW_H: f32 = 36.0;
const STAT_POS: [(f32, f32); 6] = [
    (270.0, 50.0),  // str (top)
    (191.0, 103.0), // agi
    (348.0, 104.0), // vit
    (270.0, 243.0), // int (bottom)
    (191.0, 190.0), // dex
    (348.0, 190.0), // luk
];
const STAT_INFO_X: f32 = 480.0;
const STAT_INFO_Y: f32 = 40.0;
const STAT_ROW_H: f32 = 16.0;

// Radar polygon over the stat arrows: each spoke runs from the hexagon center
// (value 0) to its arrow (value 10), so a stat of 5 sits at the spoke midpoint.
// Perimeter order (clockwise from the top vertex) so the triangle fan winds cleanly.
const STAT_HEX_ORDER: [usize; 6] = [0, 2, 5, 3, 4, 1];
const STAT_HEX_MAX: f32 = 10.0;
const STAT_HEX_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 0.7];

pub const CHAR_CREATE_WINDOW_ID: WidgetId = WidgetId(220);
const NAME_ID: WidgetId = WidgetId(221);
const STYLE_L_ID: WidgetId = WidgetId(222);
const STYLE_R_ID: WidgetId = WidgetId(223);
const COLOR_L_ID: WidgetId = WidgetId(224);
const MAKE_ID: WidgetId = WidgetId(225);
const CANCEL_ID: WidgetId = WidgetId(226);
const STAT_ARROW_BASE: u32 = 227;
const COLOR_R_ID: WidgetId = WidgetId(233);
const SKIN_TOGGLE_ID: WidgetId = WidgetId(234);

const WIN_TEXTURE: &str = "data/texture/유저인터페이스/login_interface/win_make.bmp";
const WIN2_TEXTURE: &str = "data/texture/유저인터페이스/login_interface/win_make2.bmp";
const NAME_EDIT_TEXTURE: &str = "data/texture/유저인터페이스/login_interface/name-edit.bmp";
const ARROW_L_TEXTURE: &str = "data/texture/유저인터페이스/scroll1left.bmp";
const ARROW_R_TEXTURE: &str = "data/texture/유저인터페이스/scroll1right.bmp";
const ARROW_UP_TEXTURE: &str = "data/texture/유저인터페이스/scroll0up.bmp";

const MAKE_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_make.bmp",
    hover: "data/texture/유저인터페이스/btn_make_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_make_b.bmp",
};
const CANCEL_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_cancel.bmp",
    hover: "data/texture/유저인터페이스/btn_cancel_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_cancel_b.bmp",
};
const ARROW_L_BTN: ButtonTextures = ButtonTextures {
    normal: ARROW_L_TEXTURE,
    hover: ARROW_L_TEXTURE,
    pressed: ARROW_L_TEXTURE,
};
const ARROW_R_BTN: ButtonTextures = ButtonTextures {
    normal: ARROW_R_TEXTURE,
    hover: ARROW_R_TEXTURE,
    pressed: ARROW_R_TEXTURE,
};
const ARROW_UP_BTN: ButtonTextures = ButtonTextures {
    normal: ARROW_UP_TEXTURE,
    hover: ARROW_UP_TEXTURE,
    pressed: ARROW_UP_TEXTURE,
};

const STAT_ARROWS: [ButtonTextures; 6] = [
    ButtonTextures {
        normal: "data/texture/유저인터페이스/login_interface/arw-str0.bmp",
        hover: "data/texture/유저인터페이스/login_interface/arw-str0.bmp",
        pressed: "data/texture/유저인터페이스/login_interface/arw-str1.bmp",
    },
    ButtonTextures {
        normal: "data/texture/유저인터페이스/login_interface/arw-agi0.bmp",
        hover: "data/texture/유저인터페이스/login_interface/arw-agi0.bmp",
        pressed: "data/texture/유저인터페이스/login_interface/arw-agi1.bmp",
    },
    ButtonTextures {
        normal: "data/texture/유저인터페이스/login_interface/arw-vit0.bmp",
        hover: "data/texture/유저인터페이스/login_interface/arw-vit0.bmp",
        pressed: "data/texture/유저인터페이스/login_interface/arw-vit1.bmp",
    },
    ButtonTextures {
        normal: "data/texture/유저인터페이스/login_interface/arw-int0.bmp",
        hover: "data/texture/유저인터페이스/login_interface/arw-int0.bmp",
        pressed: "data/texture/유저인터페이스/login_interface/arw-int1.bmp",
    },
    ButtonTextures {
        normal: "data/texture/유저인터페이스/login_interface/arw-dex0.bmp",
        hover: "data/texture/유저인터페이스/login_interface/arw-dex0.bmp",
        pressed: "data/texture/유저인터페이스/login_interface/arw-dex1.bmp",
    },
    ButtonTextures {
        normal: "data/texture/유저인터페이스/login_interface/arw-luk0.bmp",
        hover: "data/texture/유저인터페이스/login_interface/arw-luk0.bmp",
        pressed: "data/texture/유저인터페이스/login_interface/arw-luk1.bmp",
    },
];

const TEXT_COLOR: [f32; 4] = [0.15, 0.16, 0.32, 1.0];
const ERROR_COLOR: [f32; 4] = [0.8, 0.2, 0.2, 1.0];

pub struct CharCreateWindow {
    pub name: TextInput,
    pub slot: u8,
    pub hair_style: u16,
    pub hair_color: u16,
    /// STR, AGI, VIT, INT, DEX, LUK.
    pub stats: [u8; 6],
    /// Whether the packetver accepts starting stats (< 20120307). Also selects the
    /// window skin: with stats = win_make.bmp hexagon layout, without = win_make2.bmp.
    pub with_stats: bool,
    pub error_message: Option<String>,
    /// Dev aid: draw a small button that flips the skin (v1 hexagon ↔ v2 compact).
    /// Off in the real client, where the packetver fixes the skin.
    pub show_skin_toggle: bool,
    pub has_grf_textures: bool,
    win_origin: (f32, f32),
}

impl CharCreateWindow {
    pub fn new(slot: u8, with_stats: bool) -> Self {
        Self {
            name: TextInput::new(NAME_MAX_LEN, false),
            slot,
            hair_style: HEAD_MIN,
            hair_color: 0,
            stats: [STAT_START; 6],
            with_stats,
            error_message: None,
            show_skin_toggle: false,
            has_grf_textures: false,
            win_origin: (0.0, 0.0),
        }
    }

    /// Screen-space feet anchor for the rotating preview sprite (skin-dependent).
    pub fn preview_anchor(&self) -> [f32; 2] {
        let (px, py) = if self.with_stats {
            (PREVIEW_X + PREVIEW_ANCHOR_X, PREVIEW_Y + PREVIEW_ANCHOR_Y)
        } else {
            (
                V2_PREVIEW_X + PREVIEW_ANCHOR_X,
                V2_HEADER_H + V2_PREVIEW_Y + PREVIEW_ANCHOR_Y,
            )
        };
        [self.win_origin.0 + px, self.win_origin.1 + py]
    }

    /// Textures for the active skin, so a GRF missing the other skin's background
    /// doesn't force the whole window to fallback rendering.
    pub fn layout_texture_paths(&self) -> Vec<&'static str> {
        let mut paths = vec![
            ARROW_L_TEXTURE,
            ARROW_R_TEXTURE,
            MAKE_BTN.normal,
            MAKE_BTN.hover,
            MAKE_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
        ];
        if self.with_stats {
            paths.extend([WIN_TEXTURE, ARROW_UP_TEXTURE]);
        } else {
            paths.extend([
                WIN2_TEXTURE,
                NAME_EDIT_TEXTURE,
                window_chrome::TITLEBAR_TEX,
                window_chrome::FOOTER_TEX,
                window_chrome::SYS_BASE_OFF_TEX,
                window_chrome::SYS_BASE_ON_TEX,
            ]);
        }
        paths
    }

    /// Hexagon stat arrows are preloaded best-effort (kept out of the layout set so a
    /// newer GRF lacking them doesn't drop the window to fallback).
    pub fn stat_arrow_texture_paths() -> Vec<&'static str> {
        let mut paths = Vec::with_capacity(12);
        for arrow in &STAT_ARROWS {
            paths.push(arrow.normal);
            paths.push(arrow.pressed);
        }
        paths
    }

    fn cycle_head(&mut self, delta: i32) {
        let span = (HEAD_MAX - HEAD_MIN + 1) as i32;
        let cur = (self.hair_style - HEAD_MIN) as i32;
        self.hair_style = HEAD_MIN + (cur + delta).rem_euclid(span) as u16;
    }

    fn cycle_color(&mut self, delta: i32) {
        let n = HAIR_COLOR_COUNT as i32;
        self.hair_color = (self.hair_color as i32 + delta).rem_euclid(n) as u16;
    }

    fn raise_stat(&mut self, i: usize) {
        let p = STAT_PARTNER[i];
        if self.stats[i] < STAT_MAX && self.stats[p] > STAT_MIN {
            self.stats[i] += 1;
            self.stats[p] -= 1;
        }
    }

    fn try_submit(&mut self, submit: bool, events: &mut Vec<GameEvent>) {
        if !submit {
            return;
        }
        if self.name.text.trim().is_empty() {
            self.error_message = Some("Please enter a name.".into());
        } else {
            events.push(GameEvent::RequestMakeCharacter {
                name: self.name.text.clone(),
                slot: self.slot,
                hair_style: self.hair_style,
                hair_color: self.hair_color,
                stats: self.stats,
            });
        }
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        if self.with_stats {
            self.build_stat_layout(ui, &mut events);
        } else {
            self.build_compact_layout(ui, &mut events);
        }
        self.maybe_draw_skin_toggle(ui);
        ui.has_grf_textures = prev_grf;
        events
    }

    fn maybe_draw_skin_toggle(&mut self, ui: &mut UiFrame) {
        if !self.show_skin_toggle {
            return;
        }
        let (ox, oy) = self.win_origin;
        let rect = Rect::new(ox + 2.0, oy + 2.0, 40.0, 14.0);
        let label = if self.with_stats { "v1" } else { "v2" };
        // Force the fallback (labelled box) look so the dev toggle reads the same
        // regardless of the active skin's GRF state.
        let prev = ui.has_grf_textures;
        ui.has_grf_textures = false;
        let clicked = ui.button(SKIN_TOGGLE_ID, rect, &MAKE_BTN, label).clicked();
        ui.has_grf_textures = prev;
        if clicked {
            self.with_stats = !self.with_stats;
            self.error_message = None;
        }
    }

    fn build_stat_layout(&mut self, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        let win = ui.window(CHAR_CREATE_WINDOW_ID, WIN_W, WIN_H, TITLE_BAR_H);
        self.win_origin = (win.x, win.y);
        let (ox, oy) = (win.x, win.y);

        if self.has_grf_textures {
            push_quad(ui, ox, oy, WIN_W, WIN_H, TextureRef::Named(WIN_TEXTURE.to_string()));
        } else {
            push_color_quad(ui, ox, oy, WIN_W, WIN_H, [0.08, 0.08, 0.12, 0.95]);
            let title = "Create Character";
            let tw = ui.atlas.measure_text(title);
            ui.text(ox + (WIN_W - tw) / 2.0, oy + ui.atlas.line_height, title, [1.0; 4]);
        }

        let name_bg = if self.has_grf_textures {
            TextInputBg::Transparent
        } else {
            TextInputBg::Default
        };
        let name_rect = Rect::new(ox + NAME_X, oy + NAME_Y, NAME_W, NAME_H);
        ui.text_input(NAME_ID, name_rect, &mut self.name, name_bg);

        let head_prev = Rect::new(ox + HEAD_PREV_X, oy + HEAD_ARROW_Y, ARROW_W, ARROW_H);
        let head_next = Rect::new(ox + HEAD_NEXT_X, oy + HEAD_ARROW_Y, ARROW_W, ARROW_H);
        if ui.button(STYLE_L_ID, head_prev, &ARROW_L_BTN, "<").clicked() {
            self.cycle_head(-1);
        }
        if ui.button(STYLE_R_ID, head_next, &ARROW_R_BTN, ">").clicked() {
            self.cycle_head(1);
        }
        let hair_up = Rect::new(ox + HAIR_UP_X, oy + HAIR_UP_Y, ARROW_W, ARROW_H);
        if ui.button(COLOR_L_ID, hair_up, &ARROW_UP_BTN, "^").clicked() {
            self.cycle_color(1);
        }

        self.build_stats(ui, ox, oy);

        if let Some(msg) = &self.error_message {
            ui.text(ox + NAME_X, oy + NAME_Y + NAME_H + ui.atlas.line_height, msg, ERROR_COLOR);
        }

        let btn_y = oy + WIN_H - BTN_BOTTOM - BTN_H;
        let make = Rect::new(ox + WIN_W - MAKE_RIGHT - BTN_W, btn_y, BTN_W, BTN_H);
        let cancel = Rect::new(ox + WIN_W - CANCEL_RIGHT - BTN_W, btn_y, BTN_W, BTN_H);
        let submit = ui.button(MAKE_ID, make, &MAKE_BTN, "Make").clicked() || ui.ctx.key_enter;
        self.try_submit(submit, events);
        if ui.button(CANCEL_ID, cancel, &CANCEL_BTN, "Cancel").clicked() || ui.ctx.key_escape {
            events.push(GameEvent::CancelCreateCharacter);
        }
    }

    fn build_stats(&mut self, ui: &mut UiFrame, ox: f32, oy: f32) {
        self.draw_stat_hexagon(ui, ox, oy);
        for (i, &(sx, sy)) in STAT_POS.iter().enumerate() {
            let rect = Rect::new(ox + sx, oy + sy, STAT_ARROW_W, STAT_ARROW_H);
            if ui
                .button(WidgetId(STAT_ARROW_BASE + i as u32), rect, &STAT_ARROWS[i], "+")
                .clicked()
            {
                self.raise_stat(i);
            }
        }
        for i in 0..6 {
            let y = oy + STAT_INFO_Y + i as f32 * STAT_ROW_H + ui.atlas.line_height / 1.5;
            ui.text(ox + STAT_INFO_X, y, &self.stats[i].to_string(), TEXT_COLOR);
        }
    }

    fn draw_stat_hexagon(&self, ui: &mut UiFrame, ox: f32, oy: f32) {
        let arrow_center = |i: usize| -> [f32; 2] {
            [
                ox + STAT_POS[i].0 + STAT_ARROW_W / 2.0,
                oy + STAT_POS[i].1 + STAT_ARROW_H / 2.0,
            ]
        };
        let (mut cx, mut cy) = (0.0, 0.0);
        for i in 0..6 {
            let [ax, ay] = arrow_center(i);
            cx += ax;
            cy += ay;
        }
        cx /= 6.0;
        cy /= 6.0;

        let mut vertices = Vec::with_capacity(7);
        vertices.push(UiVertex {
            position: [cx, cy],
            tex_coord: [0.5, 0.5],
            color: STAT_HEX_COLOR,
        });
        for &i in STAT_HEX_ORDER.iter() {
            let [ax, ay] = arrow_center(i);
            let t = self.stats[i] as f32 / STAT_HEX_MAX;
            vertices.push(UiVertex {
                position: [cx + (ax - cx) * t, cy + (ay - cy) * t],
                tex_coord: [0.0, 0.0],
                color: STAT_HEX_COLOR,
            });
        }
        let n = STAT_HEX_ORDER.len() as u32;
        let mut indices = Vec::with_capacity((n * 3) as usize);
        for k in 0..n {
            indices.push(0);
            indices.push(1 + k);
            indices.push(1 + (k + 1) % n);
        }
        ui.draw_calls.push(DrawCall {
            vertices,
            indices,
            texture: TextureRef::White,
        });
    }

    fn build_compact_layout(&mut self, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        let win = ui.window(CHAR_CREATE_WINDOW_ID, V2_W, V2_H, V2_HEADER_H);
        self.win_origin = (win.x, win.y);
        let (ox, oy) = (win.x, win.y);
        let has_grf = self.has_grf_textures;
        let content_y = oy + V2_HEADER_H;
        let footer_y = content_y + V2_CONTENT_H;

        window_chrome::draw_titlebar(ui, ox, oy, V2_W, V2_HEADER_H, has_grf);
        ui.text(
            ox + 18.0,
            oy + 2.0 + ui.atlas.ascent,
            "New Character",
            window_chrome::text_color(has_grf),
        );

        if has_grf {
            window_chrome::draw_textured_quad(ui, ox, content_y, V2_W, V2_CONTENT_H, WIN2_TEXTURE);
        } else {
            window_chrome::draw_container(ui, ox, content_y, V2_W, V2_CONTENT_H, has_grf);
        }
        window_chrome::draw_footer(ui, ox, footer_y, V2_W, V2_FOOTER_H, has_grf);

        let name_bg = if has_grf {
            TextInputBg::Texture(NAME_EDIT_TEXTURE)
        } else {
            TextInputBg::Default
        };
        let name_rect = Rect::new(ox + V2_NAME_X, content_y + V2_NAME_Y - V2_NAME_H, V2_NAME_W, V2_NAME_H);
        ui.text_input(NAME_ID, name_rect, &mut self.name, name_bg);

        let style_l = Rect::new(ox + V2_ARROW_L_X, content_y + V2_STYLE_Y - ARROW_H, ARROW_W, ARROW_H);
        let style_r = Rect::new(ox + V2_ARROW_R_X, content_y + V2_STYLE_Y - ARROW_H, ARROW_W, ARROW_H);
        if ui.button(STYLE_L_ID, style_l, &ARROW_L_BTN, "<").clicked() {
            self.cycle_head(-1);
        }
        if ui.button(STYLE_R_ID, style_r, &ARROW_R_BTN, ">").clicked() {
            self.cycle_head(1);
        }
        let color_l = Rect::new(ox + V2_ARROW_L_X, content_y + V2_COLOR_Y - ARROW_H, ARROW_W, ARROW_H);
        let color_r = Rect::new(ox + V2_ARROW_R_X, content_y + V2_COLOR_Y - ARROW_H, ARROW_W, ARROW_H);
        if ui.button(COLOR_L_ID, color_l, &ARROW_L_BTN, "<").clicked() {
            self.cycle_color(-1);
        }
        if ui.button(COLOR_R_ID, color_r, &ARROW_R_BTN, ">").clicked() {
            self.cycle_color(1);
        }

        if let Some(msg) = &self.error_message {
            ui.text(ox + 8.0, content_y + V2_NAME_Y + V2_NAME_H + ui.atlas.line_height, msg, ERROR_COLOR);
        }

        let btn_y = footer_y + V2_FOOTER_H - BTN_BOTTOM - BTN_H;
        let make = Rect::new(ox + V2_W - MAKE_RIGHT - BTN_W, btn_y, BTN_W, BTN_H);
        let cancel = Rect::new(ox + V2_W - CANCEL_RIGHT - BTN_W, btn_y, BTN_W, BTN_H);
        let submit = ui.button(MAKE_ID, make, &MAKE_BTN, "Make").clicked() || ui.ctx.key_enter;
        self.try_submit(submit, events);
        if ui.button(CANCEL_ID, cancel, &CANCEL_BTN, "Cancel").clicked() || ui.ctx.key_escape {
            events.push(GameEvent::CancelCreateCharacter);
        }
    }
}

fn push_quad(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, texture: TextureRef) {
    let (v, i) = draw::quad_vertices(x, y, w, h, [1.0, 1.0, 1.0, 1.0]);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture,
    });
}

fn push_color_quad(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    let (v, i) = draw::quad_vertices(x, y, w, h, color);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::White,
    });
}

impl Window for CharCreateWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }

    fn window_size(&self) -> (f32, f32) {
        if self.with_stats {
            (WIN_W, WIN_H)
        } else {
            (V2_W, V2_H)
        }
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            WIN_TEXTURE,
            WIN2_TEXTURE,
            NAME_EDIT_TEXTURE,
            ARROW_L_TEXTURE,
            ARROW_R_TEXTURE,
            ARROW_UP_TEXTURE,
            window_chrome::TITLEBAR_TEX,
            window_chrome::FOOTER_TEX,
            MAKE_BTN.normal,
            MAKE_BTN.hover,
            MAKE_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn head_cycle_wraps() {
        let mut win = CharCreateWindow::new(0, false);
        assert_eq!(win.hair_style, HEAD_MIN);
        win.cycle_head(-1);
        assert_eq!(win.hair_style, HEAD_MAX);
        win.cycle_head(1);
        assert_eq!(win.hair_style, HEAD_MIN);
    }

    #[test]
    fn color_cycle_wraps_both_ways() {
        let mut win = CharCreateWindow::new(0, false);
        win.cycle_color(-1);
        assert_eq!(win.hair_color, HAIR_COLOR_COUNT - 1);
        win.cycle_color(1);
        assert_eq!(win.hair_color, 0);
    }

    #[test]
    fn raising_a_stat_lowers_its_partner_and_holds_pair_sum() {
        let mut win = CharCreateWindow::new(0, true);
        win.raise_stat(0); // STR ↔ INT
        assert_eq!(win.stats[0], 6);
        assert_eq!(win.stats[3], 4);
        assert_eq!(win.stats.iter().map(|&s| s as u32).sum::<u32>(), 30);
    }

    #[test]
    fn stat_clamps_at_bounds() {
        let mut win = CharCreateWindow::new(0, true);
        for _ in 0..10 {
            win.raise_stat(0);
        }
        assert_eq!(win.stats[0], STAT_MAX);
        assert_eq!(win.stats[3], STAT_MIN);
    }

    #[test]
    fn make_emits_request_with_stats() {
        let mut win = CharCreateWindow::new(2, true);
        win.name.text = "Hero".into();
        win.raise_stat(2); // VIT up, DEX down
        let mut state = StateCache::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);

        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::RequestMakeCharacter { slot: 2, stats, name, .. }
                if name == "Hero" && stats == &[5, 5, 6, 5, 4, 5]
        )));
    }

    #[test]
    fn compact_layout_make_emits_request() {
        let mut win = CharCreateWindow::new(1, false);
        win.name.text = "Novice".into();
        let mut state = StateCache::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);

        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::RequestMakeCharacter { slot: 1, name, .. } if name == "Novice"
        )));
    }

    #[test]
    fn make_with_empty_name_sets_error_and_no_event() {
        let mut win = CharCreateWindow::new(0, true);
        let mut state = StateCache::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);

        assert!(win.error_message.is_some());
        assert!(!events.iter().any(|e| matches!(e, GameEvent::RequestMakeCharacter { .. })));
    }
}
