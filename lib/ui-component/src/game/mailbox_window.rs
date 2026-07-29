use super::input_dialog::{InputDialog, InputDialogConfig, InputDialogResult};
use super::inventory_window::INV_WINDOW_ID;
use crate::helper::dialog_container::DialogContainer;
use crate::helper::window_chrome::text_color;
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_game::mail::{
    ComposeItem, MAIL_BODY_MAX, MAIL_ROWS_PER_PAGE, MAIL_TITLE_MAX, MAIL_TO_MAX, MailboxMode,
    format_mail_date,
};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const MAILBOX_WINDOW_ID: WidgetId = WidgetId(3900);
const MAIL_CLOSE_BTN_ID: WidgetId = WidgetId(3901);
const MAIL_INBOX_BTN_ID: WidgetId = WidgetId(3902);
const MAIL_WRITE_BTN_ID: WidgetId = WidgetId(3903);
const MAIL_PREV_PAGE_ID: WidgetId = WidgetId(3904);
const MAIL_NEXT_PAGE_ID: WidgetId = WidgetId(3905);
const ROW_TITLE_BASE: u32 = 3910;
const ROW_SENDER_BASE: u32 = 3920;
const TO_INPUT_ID: WidgetId = WidgetId(3930);
const TITLE_INPUT_ID: WidgetId = WidgetId(3931);
const BODY_INPUT_ID: WidgetId = WidgetId(3932);
const ZENY_INPUT_ID: WidgetId = WidgetId(3934);
const ITEM_SLOT_ID: WidgetId = WidgetId(3935);
const SEND_BTN_ID: WidgetId = WidgetId(3936);
const CANCEL_BTN_ID: WidgetId = WidgetId(3937);
const AMOUNT_DIALOG_ID: WidgetId = WidgetId(3938);

fn set_input(input: &mut TextInput, text: String) {
    input.cursor_pos = text.chars().count();
    input.text = text;
}

const BG_INBOX_TEX: &str = "data/texture/유저인터페이스/basic_interface/maillist1_bg.bmp";
const BG_COMPOSE_TEX: &str = "data/texture/유저인터페이스/basic_interface/maillist2_bg.bmp";
const ENVELOPE_TEX: &str = "data/texture/유저인터페이스/basic_interface/envelop.bmp";

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
const SEND_BTN: ButtonTextures = btn!("send");
const CANCEL_BTN: ButtonTextures = btn!("cancel2");

const WIN_W: f32 = 300.0;
const WIN_H: f32 = 400.0;
const TITLE_H: f32 = 16.0;

const BTN_COL_W: f32 = 22.0;
const LIST_X_OFF: f32 = BTN_COL_W + 4.0;
const LIST_TOP_OFF: f32 = TITLE_H + 6.0;
const ROW_H: f32 = 50.5;
const ENVELOPE_W: f32 = 16.0;
const ENVELOPE_H: f32 = 11.0;
const PAGER_H: f32 = 18.0;

const SENDER_MAX_CHARS: usize = 15;
const TITLE_ROW_MAX_CHARS: usize = 23;

const LABEL_COLOR: [f32; 4] = [0.48, 0.40, 0.34, 1.0];
const READ_COLOR: [f32; 4] = [0.55, 0.50, 0.46, 1.0];

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}

fn draw_tex(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, tex: &str) {
    let (v, i) = draw::quad_vertices(x, y, w, h, [1.0, 1.0, 1.0, 1.0]);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::Named(tex.to_string()),
    });
}

pub struct MailboxWindow {
    pub has_grf_textures: bool,
    to_input: TextInput,
    title_input: TextInput,
    body_input: TextInput,
    zeny_input: TextInput,
    container: DialogContainer,
    amount_dialog: Option<(ComposeItem, InputDialog)>,
    last_mode: MailboxMode,
}

impl Default for MailboxWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl MailboxWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            to_input: TextInput::new(MAIL_TO_MAX, false),
            title_input: TextInput::new(MAIL_TITLE_MAX, false),
            body_input: TextInput::new(MAIL_BODY_MAX, false),
            zeny_input: TextInput::new(9, false).with_numeric_only(true),
            container: DialogContainer::new(),
            amount_dialog: None,
            last_mode: MailboxMode::Inbox,
        }
    }

    fn clear_compose_inputs(&mut self) {
        set_input(&mut self.to_input, String::new());
        set_input(&mut self.title_input, String::new());
        set_input(&mut self.body_input, String::new());
        set_input(&mut self.zeny_input, String::new());
        self.amount_dialog = None;
    }
}

impl Window for MailboxWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
        self.container.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        self.container.set_texture_sizes(size_fn);
    }
    fn window_size(&self) -> (f32, f32) {
        (WIN_W, WIN_H)
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            BG_INBOX_TEX,
            BG_COMPOSE_TEX,
            ENVELOPE_TEX,
            CLOSE_BTN.normal,
            CLOSE_BTN.hover,
            SEND_BTN.normal,
            SEND_BTN.hover,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
        ];
        paths.extend(InputDialog::grf_texture_paths());
        paths
    }
}

impl InGameWindow for MailboxWindow {
    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let character = &mut *ctx.character;
        let data = ctx.data;
        if !character.mail.window_open {
            self.clear_compose_inputs();
            return Vec::new();
        }
        let mut events = Vec::new();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);

        let mode = character.mail.mode;
        if mode != self.last_mode {
            self.last_mode = mode;
            if mode == MailboxMode::Inbox {
                self.clear_compose_inputs();
            }
        }
        // Consume a reply prefill produced by the read window.
        if let Some((to, title)) = character.mail.compose_prefill.take() {
            set_input(&mut self.to_input, to);
            set_input(&mut self.title_input, title);
        }

        if let Some((_, dialog)) = &self.amount_dialog {
            ui.set_modal(&[dialog.win_id()]);
        }

        let win = ui.window_at(MAILBOX_WINDOW_ID, WIN_W, WIN_H, TITLE_H, 260.0, 60.0);
        let (x, y) = (win.x, win.y);
        ui.interact(MAILBOX_WINDOW_ID, Rect::new(x, y, WIN_W, WIN_H));

        let bg = if mode == MailboxMode::Inbox {
            BG_INBOX_TEX
        } else {
            BG_COMPOSE_TEX
        };
        if grf {
            draw_tex(ui, x, y, WIN_W, WIN_H, bg);
        } else {
            crate::helper::fallback::window_body(ui, x, y, WIN_W, WIN_H);
            crate::helper::fallback::titlebar(ui, x, y, WIN_W, TITLE_H);
        }
        let title = if mode == MailboxMode::Inbox {
            "Mail"
        } else {
            "Write Mail"
        };
        ui.text(x + 25.0, y + TITLE_H - 4.0, title, tc);

        // Close button (top-right).
        let close_rect = Rect::new(x + WIN_W - 17.0, y + 3.0, 16.0, 11.0);
        if ui
            .button(MAIL_CLOSE_BTN_ID, close_rect, &CLOSE_BTN, "x")
            .clicked()
        {
            character.mail.window_open = false;
            character.mail.read_open = false;
            ui.has_grf_textures = prev_grf;
            return events;
        }

        // Inbox / Write tabs on the left column.
        let inbox_zone = Rect::new(x + 2.0, y + TITLE_H + 4.0, BTN_COL_W, 58.0);
        let write_zone = Rect::new(x + 2.0, y + TITLE_H + 64.0, BTN_COL_W, 58.0);
        let inbox_resp = ui.interact(MAIL_INBOX_BTN_ID, inbox_zone);
        let write_resp = ui.interact(MAIL_WRITE_BTN_ID, write_zone);
        if inbox_resp.hovered() || write_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if !grf {
            crate::helper::fallback::cell(
                ui,
                inbox_zone.x,
                inbox_zone.y,
                inbox_zone.w,
                inbox_zone.h,
                mode == MailboxMode::Inbox,
            );
            crate::helper::fallback::cell(
                ui,
                write_zone.x,
                write_zone.y,
                write_zone.w,
                write_zone.h,
                mode == MailboxMode::Compose,
            );
            ui.text(inbox_zone.x + 3.0, inbox_zone.y + 30.0, "In", tc);
            ui.text(write_zone.x + 3.0, write_zone.y + 30.0, "Wr", tc);
        }
        if inbox_resp.clicked() && mode != MailboxMode::Inbox {
            character.mail.switch_to_inbox();
            self.clear_compose_inputs();
            events.push(GameEvent::RequestMailList);
        }
        if write_resp.clicked() && mode != MailboxMode::Compose {
            character.mail.mode = MailboxMode::Compose;
        }

        if mode == MailboxMode::Inbox {
            events.extend(self.build_inbox(ui, character, x, y, tc));
        } else {
            events.extend(self.build_compose(ui, character, data, x, y, grf));
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl MailboxWindow {
    fn build_inbox(
        &mut self,
        ui: &mut UiFrame,
        character: &mut Character,
        x: f32,
        y: f32,
        tc: [f32; 4],
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mail = &mut character.mail;
        mail.clamp_page();

        let list_x = x + LIST_X_OFF;
        let list_y = y + LIST_TOP_OFF;
        let list_w = WIN_W - LIST_X_OFF - 8.0;

        let start = mail.page * MAIL_ROWS_PER_PAGE;
        let mut open_id: Option<u32> = None;
        let mut reply_to: Option<String> = None;

        for row in 0..MAIL_ROWS_PER_PAGE {
            let Some(entry) = mail.inbox.get(start + row) else {
                break;
            };
            let ry = list_y + row as f32 * ROW_H;

            if !entry.read && self.has_grf_textures {
                draw_tex(
                    ui,
                    list_x + 2.0,
                    ry + (ROW_H - ENVELOPE_H) / 2.0,
                    ENVELOPE_W,
                    ENVELOPE_H,
                    ENVELOPE_TEX,
                );
            }

            let row_color = if entry.read { READ_COLOR } else { tc };
            let text_x = list_x + ENVELOPE_W + 6.0;
            let sender_str = truncate(&entry.sender, SENDER_MAX_CHARS);
            let sender_rect = Rect::new(text_x, ry + 4.0, 130.0, 16.0);
            let sender_resp = ui.interact(WidgetId(ROW_SENDER_BASE + row as u32), sender_rect);
            if sender_resp.hovered() {
                ui.any_interactive_hovered = true;
                ui.tooltip(text_x, ry - 4.0, &entry.sender);
            }
            ui.text(text_x, ry + 16.0, &sender_str, row_color);

            let date = format_mail_date(entry.time);
            let dw = ui.atlas.measure_text(&date);
            ui.text(list_x + list_w - dw - 4.0, ry + 16.0, &date, row_color);

            let title_str = truncate(&entry.title, TITLE_ROW_MAX_CHARS);
            let title_rect = Rect::new(text_x, ry + 22.0, list_w - ENVELOPE_W - 10.0, 20.0);
            let title_resp = ui.interact(WidgetId(ROW_TITLE_BASE + row as u32), title_rect);
            if title_resp.hovered() {
                ui.any_interactive_hovered = true;
                if entry.title.chars().count() > TITLE_ROW_MAX_CHARS {
                    ui.tooltip(text_x, ry + 6.0, &entry.title);
                }
            }
            ui.text(text_x, ry + 36.0, &title_str, row_color);

            if title_resp.clicked() {
                open_id = Some(entry.mail_id);
            }
            if sender_resp.clicked() {
                reply_to = Some(entry.sender.clone());
            }
        }

        // Pager.
        let pager_y = y + WIN_H - PAGER_H - 6.0;
        let page_count = mail.page_count();
        let prev_rect = Rect::new(list_x + 20.0, pager_y, 40.0, PAGER_H);
        let next_rect = Rect::new(list_x + list_w - 60.0, pager_y, 40.0, PAGER_H);
        let prev_resp = ui.interact(MAIL_PREV_PAGE_ID, prev_rect);
        let next_resp = ui.interact(MAIL_NEXT_PAGE_ID, next_rect);
        if prev_resp.hovered() || next_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        ui.text(prev_rect.x, pager_y + 15.0, "Prev", tc);
        let page_label = format!("{}/{}", mail.page + 1, page_count);
        let plw = ui.atlas.measure_text(&page_label);
        ui.text(
            x + WIN_W / 2.0 - plw / 2.0 + 4.0,
            pager_y + 15.0,
            &page_label,
            tc,
        );
        ui.text(next_rect.x, pager_y + 15.0, "Next", tc);
        if prev_resp.clicked() && mail.page > 0 {
            mail.page -= 1;
        }
        if next_resp.clicked() && mail.page + 1 < page_count {
            mail.page += 1;
        }

        if let Some(mail_id) = open_id {
            events.push(GameEvent::RequestMailOpen { mail_id });
        }
        if let Some(to) = reply_to {
            character.mail.mode = MailboxMode::Compose;
            set_input(&mut self.to_input, to);
        }
        events
    }

    #[allow(clippy::too_many_arguments)]
    fn build_compose(
        &mut self,
        ui: &mut UiFrame,
        character: &mut Character,
        data: &DataTable,
        x: f32,
        y: f32,
        grf: bool,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();

        let field_x = x + 55.0;
        let field_w = 150.0;

        ui.text(field_x, y + 44.0, "To :", LABEL_COLOR);
        ui.text_input(
            TO_INPUT_ID,
            Rect::new(field_x + 33.0, y + 34.0, field_w, 16.0),
            &mut self.to_input,
            TextInputBg::Transparent,
        );

        ui.text(field_x, y + 74.0, "Title :", LABEL_COLOR);
        ui.text_input(
            TITLE_INPUT_ID,
            Rect::new(field_x + 33.0, y + 64.0, field_w + 20.0, 16.0),
            &mut self.title_input,
            TextInputBg::Transparent,
        );

        ui.text_input(
            BODY_INPUT_ID,
            Rect::new(x + 38.0, y + 98.0, WIN_W - 56.0, 220.0),
            &mut self.body_input,
            TextInputBg::Transparent,
        );

        // Zeny attach: type an amount; it is sent with the mail (see Send below).
        ui.text_input(
            ZENY_INPUT_ID,
            Rect::new(x + 135.0, y + 340.0, 65.0, 16.0),
            &mut self.zeny_input,
            TextInputBg::Gray,
        );

        // Item attach slot.
        let slot_rect = Rect::new(x + 247.0, y + 333.0, 24.0, 24.0);
        if !grf {
            crate::helper::fallback::slot_cell(
                ui,
                slot_rect.x,
                slot_rect.y,
                slot_rect.w,
                slot_rect.h,
            );
        }
        let slot_resp = ui.interact(ITEM_SLOT_ID, slot_rect);
        if slot_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if let Some(attach) = character.mail.compose.item {
            if let Some(res) = data
                .item_resource
                .as_ref()
                .and_then(|t| t.get_resource_name_for(attach.item_id, attach.identified))
            {
                draw_tex(
                    ui,
                    slot_rect.x,
                    slot_rect.y,
                    24.0,
                    24.0,
                    &format!("data/texture/유저인터페이스/item/{res}.bmp"),
                );
            }
            if attach.amount > 1 {
                let cnt = attach.amount.to_string();
                let cw = ui.atlas.measure_text(&cnt);
                ui.text(
                    slot_rect.x + 24.0 - cw,
                    slot_rect.y + 24.0,
                    &cnt,
                    [0.0, 0.0, 0.0, 1.0],
                );
            }
            // Click the filled slot to detach.
            if slot_resp.clicked() {
                events.push(GameEvent::RequestMailResetItem { ty: 1 });
                character.mail.compose.item = None;
            }
        }
        if let Some((source_id, item_index)) = ui.drop_zone(slot_rect)
            && source_id == INV_WINDOW_ID
        {
            let index = item_index as u16;
            if let Some(item) = character.inventory.get_item(index) {
                let count = item.count;
                let attach = ComposeItem {
                    inv_index: index,
                    item_id: item.item_id,
                    amount: 1,
                    identified: item.is_identified,
                };
                if count > 1 {
                    let mut dialog = InputDialog::new(
                        InputDialogConfig {
                            label: None,
                            show_cancel: true,
                            escape_cancels: true,
                            default_value: count.to_string(),
                            max_len: 6,
                            numeric_only: true,
                        },
                        AMOUNT_DIALOG_ID,
                    );
                    dialog.init_container(&self.container);
                    self.amount_dialog = Some((attach, dialog));
                } else {
                    events.push(GameEvent::RequestMailResetItem { ty: 1 });
                    events.push(GameEvent::RequestMailAddItem { index, amount: 1 });
                    character.mail.compose.pending_item = Some(attach);
                }
            }
        }

        // Attachment amount dialog.
        if let Some((attach, dialog)) = &mut self.amount_dialog {
            match dialog.build(ui) {
                InputDialogResult::Submitted => {
                    let amount = dialog.value_i16().unwrap_or(0).max(0) as u32;
                    let mut attach = *attach;
                    let index = attach.inv_index;
                    self.amount_dialog = None;
                    if amount > 0 {
                        attach.amount = amount;
                        events.push(GameEvent::RequestMailResetItem { ty: 1 });
                        events.push(GameEvent::RequestMailAddItem { index, amount });
                        character.mail.compose.pending_item = Some(attach);
                    }
                }
                InputDialogResult::Cancel => self.amount_dialog = None,
                InputDialogResult::None => {}
            }
        }

        // Send / Cancel.
        let send_rect = Rect::new(x + WIN_W - 100.0, y + WIN_H - 27.0, 43.0, 21.0);
        let cancel_rect = Rect::new(x + WIN_W - 52.0, y + WIN_H - 27.0, 43.0, 21.0);
        let send_clicked = ui
            .button(SEND_BTN_ID, send_rect, &SEND_BTN, "Send")
            .clicked();
        let cancel_clicked = ui
            .button(CANCEL_BTN_ID, cancel_rect, &CANCEL_BTN, "Cancel")
            .clicked();

        if send_clicked && !character.mail.send_pending {
            let title = self.title_input.text.clone();
            if title.trim().is_empty() {
                ui.tooltip(
                    send_rect.x - 40.0,
                    send_rect.y - 16.0,
                    "Please enter a title.",
                );
            } else {
                character.mail.send_pending = true;
                let zeny: u32 = self.zeny_input.text.trim().parse().unwrap_or(0);
                if zeny > 0 {
                    events.push(GameEvent::RequestMailAddItem {
                        index: 0,
                        amount: zeny,
                    });
                }
                events.push(GameEvent::RequestMailSend {
                    to: self.to_input.text.clone(),
                    title,
                    body: self.body_input.text.clone(),
                });
            }
        }
        if cancel_clicked {
            events.push(GameEvent::RequestMailResetItem { ty: 0 });
            character.mail.switch_to_inbox();
            self.clear_compose_inputs();
            events.push(GameEvent::RequestMailList);
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_game::mail::MailEntry;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = Box::leak(Box::new(FontAtlas::from_embedded(14.0, 1.0)));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    fn open_inbox() -> Character {
        let mut character = Character::new();
        character.mail.window_open = true;
        character.mail.inbox = vec![MailEntry {
            mail_id: 1001,
            title: "Hello".into(),
            read: false,
            sender: "Alice".into(),
            time: 1_615_680_000,
        }];
        character
    }

    #[test]
    fn dropping_inventory_item_on_slot_attaches_it() {
        use models::enums::item::ItemType;
        use ragnarok_game::item::Item;

        let mut win = MailboxWindow::new();
        let mut character = open_inbox();
        character.mail.mode = MailboxMode::Compose;
        win.last_mode = MailboxMode::Compose;
        character.inventory.add_item(Item {
            index: 5,
            item_id: 1101,
            item_type: ItemType::Weapon,
            count: 1,
            is_identified: true,
            is_damaged: false,
            refining_level: 0,
            slot: [0; 4],
            location: 0,
            wear_state: 0,
            name: "Sword".into(),
            resource_name: None,
        });
        let data = DataTable::new();
        let mut state = StateCache::new();

        let slot_cx = 260.0 + 247.0 + 12.0;
        let slot_cy = 60.0 + 333.0 + 12.0;

        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = 100.0;
        ctx.mouse_y = 100.0;
        ctx.mouse_down = true;
        ctx.mouse_clicked = true;
        {
            let mut ui = make_frame(&ctx, &mut state);
            ui.drag_source(INV_WINDOW_ID, 5, None, (24.0, 24.0));
            ui.draw_drag_icon();
        }

        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = 140.0;
        ctx.mouse_y = 100.0;
        ctx.mouse_down = true;
        {
            let mut ui = make_frame(&ctx, &mut state);
            ui.draw_drag_icon();
            assert!(ui.is_dragging());
        }

        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = slot_cx;
        ctx.mouse_y = slot_cy;
        ctx.mouse_down = false;
        let events = {
            let mut ui = make_frame(&ctx, &mut state);
            win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
        };
        assert!(
            events.iter().any(|e| matches!(
                e,
                GameEvent::RequestMailAddItem {
                    index: 5,
                    amount: 1
                }
            )),
            "expected attach request, got {events:?}"
        );
        assert!(character.mail.compose.pending_item.is_some());
    }

    #[test]
    fn clicking_title_row_opens_that_mail() {
        let mut win = MailboxWindow::new();
        let mut character = open_inbox();
        let data = DataTable::new();
        let mut state = StateCache::new();

        // Row 0 title zone sits below the sender line (window default 260,60).
        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = 260.0 + LIST_X_OFF + ENVELOPE_W + 30.0;
        ctx.mouse_y = 60.0 + LIST_TOP_OFF + 32.0;
        ctx.mouse_clicked = true;
        let events = {
            let mut ui = make_frame(&ctx, &mut state);
            win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
        };
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestMailOpen { mail_id: 1001 })),
            "expected open request, got {events:?}"
        );
    }

    #[test]
    fn send_requires_title_and_latches_after_first_send() {
        let mut win = MailboxWindow::new();
        let mut character = open_inbox();
        character.mail.mode = MailboxMode::Compose;
        win.last_mode = MailboxMode::Compose;
        let data = DataTable::new();
        let mut state = StateCache::new();

        let send_cx = 260.0 + WIN_W - 100.0 + 20.0;
        let send_cy = 60.0 + WIN_H - 30.0 + 10.0;

        // Empty title: clicking Send must not emit a send request.
        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = send_cx;
        ctx.mouse_y = send_cy;
        ctx.mouse_clicked = true;
        let events = {
            let mut ui = make_frame(&ctx, &mut state);
            win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
        };
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestMailSend { .. }))
        );
        assert!(!character.mail.send_pending);

        // With a title, the first click sends and latches.
        set_input(&mut win.title_input, "Subject".into());
        let events = {
            let mut ui = make_frame(&ctx, &mut state);
            win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
        };
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, GameEvent::RequestMailSend { .. }))
                .count(),
            1
        );
        assert!(character.mail.send_pending);

        // Latched: a second click does not re-send.
        let events = {
            let mut ui = make_frame(&ctx, &mut state);
            win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
        };
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestMailSend { .. }))
        );
    }
}
