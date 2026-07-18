use crate::App;
use ragnarok_game::banner::BannerKind;
use ragnarok_game::entity::ChatBubbleState;
use ragnarok_ui_component::game::chat_room_member_window::{OTHER_MSG_COLOR, OWN_MSG_COLOR};
use ragnarok_ui_component::game::chat_window::ChatChannel;

impl App {
    pub(super) fn handle_broadcast_message(
        &mut self,
        message: String,
        color: [f32; 4],
        banner: BannerKind,
    ) {
        match banner {
            BannerKind::Once => {
                self.game.banner.enqueue(message, 1);
            }
            BannerKind::Repeat(times) => {
                self.game.banner.enqueue(message.clone(), times);
                self.game
                    .chat_window
                    .add_message(message, color, ChatChannel::System);
            }
            BannerKind::None => {
                self.game.poptip.push(message.clone());
                self.game
                    .chat_window
                    .add_message(message, color, ChatChannel::System);
            }
        }
    }

    pub(super) fn handle_chat_message(&mut self, gid: u32, message: String) {
        if let Some(bubble_text) = message.split(" : ").nth(1)
            && let Some(entity) = self.game.entities.get_mut(gid)
        {
            entity.chat_bubble = Some(ChatBubbleState::new(bubble_text.to_string()));
        }
        if self.game.chat_room_member_window.is_open()
            && let Some((sender, _)) = message.split_once(" : ")
            && self.game.chat_room_member_window.has_member(sender)
        {
            self.game
                .chat_room_member_window
                .push_message(message.clone(), OTHER_MSG_COLOR);
        }
        let is_own = self
            .game
            .selected_character
            .as_ref()
            .zip(message.split_once(" : "))
            .is_some_and(|(c, (sender, _))| sender == c.name);
        if is_own || !self.game.hide_public_chat {
            self.game.chat_window.add_chat(message);
        }
    }

    pub(super) fn handle_own_chat_message(&mut self, message: String) {
        if let Some(bubble_text) = message.split(" : ").nth(1)
            && let Some(player_id) = self.game.entities.player_id()
            && let Some(entity) = self.game.entities.get_mut(player_id)
        {
            entity.chat_bubble = Some(ChatBubbleState::new(bubble_text.to_string()));
        }
        if self.game.chat_room_member_window.is_open() {
            self.game
                .chat_room_member_window
                .push_message(message.clone(), OWN_MSG_COLOR);
        }
        self.game.chat_window.add_own_chat(message);
    }

    pub(super) fn handle_exp_gained(
        &mut self,
        aid: u32,
        amount: i32,
        is_base: bool,
        is_quest: bool,
    ) {
        tracing::debug!(aid, amount, is_base, is_quest, "exp gained");
        if !self.game.show_exp {
            return;
        }
        let message = if is_base {
            format!("Gained {amount} base experience")
        } else {
            format!("Gained {amount} job experience")
        };
        self.game.chat_window.add_system(message);
    }
}
