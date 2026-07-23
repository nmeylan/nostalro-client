use crate::App;
use ragnarok_game::mail::{MailboxMode, OpenedMail};
use ragnarok_network::build_mail_get_list_packet;

impl App {
    pub(super) fn handle_mail_window(&mut self, open: bool) {
        if open {
            let mail = &mut self.game.character.mail;
            mail.clear();
            mail.window_open = true;
            mail.mode = MailboxMode::Inbox;
            self.channel
                .send_packet(build_mail_get_list_packet(self.active_packetver));
        } else {
            self.game.character.mail.clear();
        }
    }

    pub(super) fn handle_mail_inbox_received(
        &mut self,
        entries: Vec<ragnarok_game::mail::MailEntry>,
    ) {
        let mail = &mut self.game.character.mail;
        mail.inbox = entries;
        mail.clamp_page();
    }

    pub(super) fn handle_mail_opened(&mut self, mail: OpenedMail) {
        if let Some(item) = &mail.item
            && let Some(path) = self.mail_item_icon(item.nameid, item.identified)
        {
            self.preload_item_icons(vec![path]);
        }
        let state = &mut self.game.character.mail;
        state.opened = Some(mail);
        state.read_open = true;
    }

    pub(super) fn handle_mail_delete_ack(&mut self, mail_id: u32, ok: bool) {
        if ok {
            let mail = &mut self.game.character.mail;
            mail.inbox.retain(|m| m.mail_id != mail_id);
            mail.clamp_page();
            if mail.opened.as_ref().is_some_and(|o| o.mail_id == mail_id) {
                mail.opened = None;
                mail.read_open = false;
            }
        } else {
            self.windows
                .chat_window
                .add_error("Cannot delete a mail that still has attachments.".to_string());
        }
    }

    pub(super) fn handle_mail_get_item_ack(&mut self, result: u8) {
        match result {
            0 => {
                if let Some(opened) = &mut self.game.character.mail.opened {
                    opened.zeny = 0;
                    opened.item = None;
                }
            }
            1 => self
                .windows
                .chat_window
                .add_error("Cannot receive zeny: it would exceed the limit.".to_string()),
            _ => self
                .windows
                .chat_window
                .add_error("Cannot receive item: not enough space or weight.".to_string()),
        }
    }

    pub(super) fn handle_mail_add_item_ack(&mut self, _index: u16, ok: bool) {
        let mail = &mut self.game.character.mail;
        if ok {
            if let Some(pending) = mail.compose.pending_item.take() {
                mail.compose.item = Some(pending);
            }
        } else {
            mail.compose.pending_item = None;
            self.windows
                .chat_window
                .add_error("This item cannot be attached.".to_string());
        }
    }

    pub(super) fn handle_mail_send_ack(&mut self, ok: bool) {
        let mail = &mut self.game.character.mail;
        mail.send_pending = false;
        if ok {
            mail.switch_to_inbox();
            self.windows
                .chat_window
                .add_system("Mail sent.".to_string());
            self.channel
                .send_packet(build_mail_get_list_packet(self.active_packetver));
        } else {
            self.windows
                .chat_window
                .add_error("The recipient does not exist.".to_string());
        }
    }

    pub(super) fn handle_mail_new_received(
        &mut self,
        mail_id: u32,
        _title: String,
        _sender: String,
    ) {
        if mail_id == 0 {
            return;
        }
        self.windows
            .chat_window
            .add_system("You've got mail.".to_string());
        let mail = &self.game.character.mail;
        if mail.window_open && mail.mode == MailboxMode::Inbox {
            self.channel
                .send_packet(build_mail_get_list_packet(self.active_packetver));
        }
    }

    pub(super) fn handle_mail_return_ack(&mut self, mail_id: u32, ok: bool) {
        if ok {
            let mail = &mut self.game.character.mail;
            mail.inbox.retain(|m| m.mail_id != mail_id);
            mail.clamp_page();
            if mail.opened.as_ref().is_some_and(|o| o.mail_id == mail_id) {
                mail.opened = None;
                mail.read_open = false;
            }
            self.channel
                .send_packet(build_mail_get_list_packet(self.active_packetver));
        } else {
            self.windows
                .chat_window
                .add_error("Failed to return the mail.".to_string());
        }
    }

    fn mail_item_icon(&self, nameid: u16, identified: bool) -> Option<String> {
        self.game
            .data_table
            .item_resource
            .as_ref()
            .and_then(|t| t.get_resource_name_for(nameid, identified))
            .map(|name| format!("data/texture/유저인터페이스/item/{name}.bmp"))
    }
}
