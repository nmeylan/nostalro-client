use crate::helper::window_chrome::text_color;
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_game::item::Item;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const READ_MAIL_WINDOW_ID: WidgetId = WidgetId(3950);
const READ_CLOSE_BTN_ID: WidgetId = WidgetId(3951);
const READ_GET_ATTACH_ID: WidgetId = WidgetId(3952);
const READ_RETURN_BTN_ID: WidgetId = WidgetId(3953);
const READ_REPLY_BTN_ID: WidgetId = WidgetId(3954);
const READ_DELETE_BTN_ID: WidgetId = WidgetId(3955);

const BG_TEX: &str = "data/texture/유저인터페이스/basic_interface/maillist3_bg.bmp";

macro_rules! btn {
    ($name:literal) => {
        ButtonTextures {
            normal: concat!(
                "data/texture/유저인터페이스/basic_interface/",
                $name,
                ".bmp"
            ),
            hover: concat!(
                "data/texture/유저인터페이스/basic_interface/",
                $name,
                "_a.bmp"
            ),
            pressed: concat!(
                "data/texture/유저인터페이스/basic_interface/",
                $name,
                "_a.bmp"
            ),
        }
    };
}

const CLOSE_BTN: ButtonTextures = btn!("close2");
const RETURN_BTN: ButtonTextures = btn!("return");
const REPLY_BTN: ButtonTextures = btn!("remail");
const DELETE_BTN: ButtonTextures = btn!("del");

const WIN_W: f32 = 300.0;
const WIN_H: f32 = 400.0;
const TITLE_H: f32 = 16.0;
const BODY_X_OFF: f32 = 20.0;
const BODY_Y_OFF: f32 = 110.0;
const BODY_W: f32 = 264.0;
const LINE_H: f32 = 14.0;
const SLOT_SIZE: f32 = 24.0;

const LABEL_COLOR: [f32; 4] = [0.65, 0.59, 0.55, 1.0];

fn thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    let bytes = s.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

fn wrap_lines(ui: &UiFrame, text: &str, max_w: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for raw in text.split('\n') {
        let mut line = String::new();
        for word in raw.split(' ') {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if ui.atlas.measure_text(&candidate) > max_w && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    lines
}

pub struct ReadMailWindow {
    pub has_grf_textures: bool,
}

impl Default for ReadMailWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadMailWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
        }
    }
}

impl Window for ReadMailWindow {
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
        vec![
            BG_TEX,
            CLOSE_BTN.normal,
            CLOSE_BTN.hover,
            RETURN_BTN.normal,
            RETURN_BTN.hover,
            REPLY_BTN.normal,
            REPLY_BTN.hover,
            DELETE_BTN.normal,
            DELETE_BTN.hover,
        ]
    }
}

impl InGameWindow for ReadMailWindow {
    fn wants_escape(&self, ctx: &BuildCtx) -> bool {
        ctx.character.mail.read_open && ctx.character.mail.opened.is_some()
    }

    fn on_escape(&mut self, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        ctx.character.mail.read_open = false;
        ctx.character.mail.opened = None;
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let character = &mut *ctx.character;
        let data = ctx.data;
        if !character.mail.read_open || character.mail.opened.is_none() {
            return Vec::new();
        }
        let mut events = Vec::new();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);

        let win = ui.window_at(READ_MAIL_WINDOW_ID, WIN_W, WIN_H, TITLE_H, 560.0, 60.0);
        let (x, y) = (win.x, win.y);
        ui.interact(READ_MAIL_WINDOW_ID, Rect::new(x, y, WIN_W, WIN_H));

        if grf {
            let (v, i) = draw::quad_vertices(x, y, WIN_W, WIN_H, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(BG_TEX.to_string()),
            });
        } else {
            crate::helper::fallback::window_body(ui, x, y, WIN_W, WIN_H);
            crate::helper::fallback::titlebar(ui, x, y, WIN_W, TITLE_H);
        }

        let opened = character.mail.opened.as_ref().unwrap();
        let mail_id = opened.mail_id;
        let sender = opened.sender.clone();
        let has_attachment = opened.has_attachment();

        ui.text(x + 25.0, y + TITLE_H - 4.0, "Read Mail", tc);

        ui.text(x + BODY_X_OFF, y + 40.0, "From:", LABEL_COLOR);
        ui.text(x + 80.0, y + 40.0, &opened.sender, tc);
        ui.text(x + BODY_X_OFF, y + 58.0, "Title:", LABEL_COLOR);
        ui.text(x + 80.0, y + 58.0, &opened.title, tc);

        let body = opened.body.clone();
        let zeny = opened.zeny;
        let item = opened.item.clone();

        // Body (wrapped, read-only).
        let lines = wrap_lines(ui, &body, BODY_W);
        for (i, line) in lines.iter().take(14).enumerate() {
            ui.text(x + BODY_X_OFF, y + BODY_Y_OFF + i as f32 * LINE_H, line, tc);
        }

        // Attachment row.
        let attach_y = y + WIN_H - 68.0;
        let slot_rect = Rect::new(
            x + WIN_W - BODY_X_OFF - SLOT_SIZE - 8.0,
            attach_y,
            SLOT_SIZE,
            SLOT_SIZE,
        );
        if !grf {
            crate::helper::fallback::slot_cell(ui, slot_rect.x, slot_rect.y, SLOT_SIZE, SLOT_SIZE);
        }
        let slot_resp = ui.interact(READ_GET_ATTACH_ID, slot_rect);
        if slot_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        let mut show_item: Option<Box<Item>> = None;
        if let Some(mail_item) = &item {
            let name = data
                .item_name
                .as_ref()
                .map(|t| t.get_name_or_id_for(mail_item.nameid, mail_item.identified))
                .unwrap_or_else(|| format!("Item #{}", mail_item.nameid));
            let resource_name = data.item_resource.as_ref().and_then(|t| {
                t.get_resource_name_for(mail_item.nameid, mail_item.identified)
                    .map(|s| s.to_string())
            });
            if let Some(res) = &resource_name {
                let (v, i) =
                    draw::quad_vertices(slot_rect.x, slot_rect.y, SLOT_SIZE, SLOT_SIZE, [1.0; 4]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(format!(
                        "data/texture/유저인터페이스/item/{res}.bmp"
                    )),
                });
            }
            if mail_item.amount > 1 {
                let cnt = mail_item.amount.to_string();
                let cw = ui.atlas.measure_text(&cnt);
                ui.text(
                    slot_rect.x + SLOT_SIZE - cw,
                    slot_rect.y + SLOT_SIZE,
                    &cnt,
                    [0.0, 0.0, 0.0, 1.0],
                );
            }
            if slot_resp.hovered() {
                ui.tooltip(slot_rect.x, slot_rect.y - 16.0, &name);
            }
            if slot_resp.right_clicked() {
                show_item = Some(Box::new(mail_item.to_item(name, resource_name)));
            }
        }

        if zeny > 0 {
            ui.text(
                x + BODY_X_OFF + 120.0,
                attach_y + SLOT_SIZE / 2.0 + 4.0,
                &format!("{} z", thousands(zeny)),
                tc,
            );
        }

        // Get-attachment: clicking the slot/zeny fetches both.
        if has_attachment && slot_resp.clicked() {
            events.push(GameEvent::RequestMailGetItem { mail_id });
        }

        // Buttons: Return / Reply / Delete (bottom-right).
        let btn_y = y + WIN_H - 27.0;
        let return_rect = Rect::new(x + WIN_W - 148.0, btn_y, 43.0, 21.0);
        let reply_rect = Rect::new(x + WIN_W - 100.0, btn_y, 43.0, 21.0);
        let delete_rect = Rect::new(x + WIN_W - 52.0, btn_y, 43.0, 21.0);

        if ui
            .button(READ_RETURN_BTN_ID, return_rect, &RETURN_BTN, "Return")
            .clicked()
        {
            events.push(GameEvent::RequestMailReturn {
                mail_id,
                sender: sender.clone(),
            });
        }
        if ui
            .button(READ_REPLY_BTN_ID, reply_rect, &REPLY_BTN, "Reply")
            .clicked()
        {
            let title = character
                .mail
                .opened
                .as_ref()
                .map(|o| o.title.clone())
                .unwrap_or_default();
            character.mail.window_open = true;
            character.mail.mode = ragnarok_game::mail::MailboxMode::Compose;
            character.mail.compose_prefill = Some((sender.clone(), format!("RE:{title}")));
        }
        let delete_clicked = ui
            .button(READ_DELETE_BTN_ID, delete_rect, &DELETE_BTN, "Delete")
            .clicked();
        if delete_clicked {
            if has_attachment {
                ui.tooltip(
                    delete_rect.x - 120.0,
                    delete_rect.y - 16.0,
                    "Remove the attachment before deleting.",
                );
            } else {
                events.push(GameEvent::RequestMailDelete { mail_id });
            }
        }

        // Close.
        let close_rect = Rect::new(x + WIN_W - 17.0, y + 3.0, 16.0, 11.0);
        if ui
            .button(READ_CLOSE_BTN_ID, close_rect, &CLOSE_BTN, "x")
            .clicked()
        {
            character.mail.read_open = false;
            character.mail.opened = None;
        }

        if let Some(item) = show_item {
            events.push(GameEvent::ShowItemInfoDirect { item });
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}
