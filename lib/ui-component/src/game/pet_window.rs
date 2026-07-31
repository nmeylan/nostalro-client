use crate::helper::colors;
use crate::helper::dropdown::{self, Dropdown};
use crate::helper::window_chrome::{
    TITLEBAR_TEX, draw_container, draw_sys_button, draw_titlebar, label_color, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_game::pet::PetState;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const PET_WINDOW_ID: WidgetId = WidgetId(4000);
const CLOSE_BTN_ID: WidgetId = WidgetId(4001);
const NAME_INPUT_ID: WidgetId = WidgetId(4002);
const RENAME_BTN_ID: WidgetId = WidgetId(4003);
const COMMAND_DROPDOWN_ID: WidgetId = WidgetId(4004);
const COMMAND_OPTION_BASE: u32 = 4010;

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

const RENAME_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_rewrite.bmp",
    hover: "data/texture/유저인터페이스/btn_rewrite_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_rewrite_b.bmp",
};

const WIN_W: f32 = 280.0;
const WIN_H: f32 = 160.0;
const TITLE_H: f32 = 17.0;
const PANEL_H: f32 = WIN_H - TITLE_H;
const PAD: f32 = 6.0;
const ILLUST_W: f32 = 100.0;
const ILLUST_H: f32 = 110.0;
const ROW_H: f32 = 18.0;
const BASELINE: f32 = 12.0;

/// Command dropdown → CZ_COMMAND_PET cSub. Feed routes through a confirm dialog.
const COMMANDS: [(&str, i8); 4] = [
    ("Feed Pet", 1),
    ("Performance", 2),
    ("Return to Egg Shell", 3),
    ("Unequip Accessory", 4),
];

pub struct PetWindow {
    pub has_grf_textures: bool,
    visible: bool,
    name_input: TextInput,
    command: Dropdown,
    rename_size: (f32, f32),
}

impl Default for PetWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl PetWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            visible: false,
            name_input: TextInput::new(23, false),
            command: Dropdown::default(),
            rename_size: (42.0, 20.0),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
    pub fn set_visible(&mut self, value: bool) {
        self.visible = value;
    }
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    fn build_body(&mut self, ui: &mut UiFrame, pet: &PetState) -> Vec<GameEvent> {
        if !self.visible {
            return Vec::new();
        }
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let mut events = Vec::new();
        let tc = text_color(grf);
        let lc = label_color(grf);
        self.command.begin_frame();

        let win = ui.window_at(PET_WINDOW_ID, WIN_W, WIN_H, TITLE_H, 250.0, 140.0);
        let x = win.x;
        let y = win.y;
        ui.interact(PET_WINDOW_ID, Rect::new(x, y, WIN_W, WIN_H));
        let bounds = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        ui.text(x + 16.0, y + 13.0, "Pet Information", tc);

        let sys_w = 11.0;
        let close_rect = Rect::new(x + WIN_W - 3.0 - sys_w, y + 3.0, sys_w, sys_w);
        let close_resp = ui.interact(CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if close_resp.clicked() {
            self.visible = false;
        }
        draw_sys_button(
            ui,
            close_rect,
            (sys_w, sys_w),
            close_resp.hovered(),
            grf,
            CLOSE_ON_TEX,
            CLOSE_OFF_TEX,
            Some('x'),
        );

        draw_container(ui, x, y + TITLE_H, WIN_W, PANEL_H, grf);

        // Illustration (left column).
        let ill_x = x + PAD;
        let ill_y = y + TITLE_H + PAD;
        if grf {
            let (v, i) =
                draw::quad_vertices(ill_x, ill_y, ILLUST_W, ILLUST_H, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(pet.illust_path().to_string()),
            });
        }

        // Right column.
        let rx = x + ILLUST_W + PAD * 2.0;
        let right_edge = x + WIN_W - PAD;
        let mut ry = y + TITLE_H + PAD;

        // Name + rename (once only).
        if pet.renamed {
            let (color, shadow) = colors::GREEN_WITH_SHADOW;
            ui.text_with_shadow(rx, ry + BASELINE, &pet.name, color, shadow);
            ry += ROW_H;
        } else {
            let (bw, bh) = self.rename_size;
            let input_w = (right_edge - rx - bw - 4.0).max(40.0);
            let input_rect = Rect::new(rx, ry, input_w, 14.0);
            let bg = if grf {
                TextInputBg::Gray
            } else {
                TextInputBg::Default
            };
            ui.text_input(NAME_INPUT_ID, input_rect, &mut self.name_input, bg);
            let btn_rect = Rect::new(rx + input_w + 4.0, ry, bw, bh.min(16.0));
            if ui
                .button(RENAME_BTN_ID, btn_rect, &RENAME_BTN, "Name")
                .clicked()
            {
                let name = self.name_input.text.trim().to_string();
                if !name.is_empty() {
                    events.push(GameEvent::RequestRenamePet { name });
                    self.name_input.text.clear();
                }
            }
            ry += ROW_H + 2.0;
        }

        ui.text_bold(rx, ry + BASELINE, "Level", lc);
        ui.text_right(right_edge, ry + BASELINE, &pet.level.to_string(), tc);
        ry += ROW_H;

        ui.text_bold(rx, ry + BASELINE, "Hunger", lc);
        ui.text_right(right_edge, ry + BASELINE, pet.hunger_state().label(), tc);
        ry += ROW_H;

        ui.text_bold(rx, ry + BASELINE, "Intimacy", lc);
        ui.text_right(right_edge, ry + BASELINE, pet.intimacy_state().label(), tc);
        ry += ROW_H;

        ui.text_bold(rx, ry + BASELINE, "Accessory", lc);
        let acc = if pet.accessory != 0 {
            "Equipped"
        } else {
            "Not equipped"
        };
        ui.text_right(right_edge, ry + BASELINE, acc, tc);
        ry += ROW_H + 2.0;

        // Command dropdown.
        let dd_rect = Rect::new(rx, ry, right_edge - rx, 16.0);
        let labels: Vec<&str> = COMMANDS.iter().map(|(l, _)| *l).collect();
        let dd = self.command.show(
            ui,
            COMMAND_DROPDOWN_ID,
            dd_rect,
            "Command",
            labels.len(),
            bounds,
            false,
        );
        if let Some(rect) = dd.overlay_rect
            && let Some(idx) = self
                .command
                .show_overlay(ui, rect, COMMAND_OPTION_BASE, &labels)
        {
            let csub = COMMANDS[idx].1;
            if csub == 1 {
                events.push(GameEvent::RequestPetFeed);
            } else {
                events.push(GameEvent::RequestPetCommand { csub });
            }
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl InGameWindow for PetWindow {
    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.visible
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.visible = false;
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.build_body(ui, ctx.pet)
    }
}

impl Window for PetWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(RENAME_BTN.normal) {
            self.rename_size = (w as f32, h as f32);
        }
    }
    fn window_size(&self) -> (f32, f32) {
        (WIN_W, WIN_H)
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            RENAME_BTN.normal,
            RENAME_BTN.hover,
            RENAME_BTN.pressed,
        ];
        paths.extend(dropdown::grf_texture_paths());
        paths
    }
}
