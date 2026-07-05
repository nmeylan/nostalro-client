use crate::App;
use ragnarok_game::entity::ChatBubbleState;

impl App {
    pub(super) fn handle_chat_message(&mut self, gid: u32, message: String) {
        if let Some(bubble_text) = message.split(" : ").nth(1)
            && let Some(entity) = self.game.entities.get_mut(gid)
        {
            entity.chat_bubble = Some(ChatBubbleState::new(bubble_text.to_string()));
        }
        self.game.chat_window.add_chat(message);
    }

    pub(super) fn handle_own_chat_message(&mut self, message: String) {
        if let Some(bubble_text) = message.split(" : ").nth(1)
            && let Some(player_id) = self.game.entities.player_id()
            && let Some(entity) = self.game.entities.get_mut(player_id)
        {
            entity.chat_bubble = Some(ChatBubbleState::new(bubble_text.to_string()));
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
        let message = if is_base {
            format!("Gained {amount} base experience")
        } else {
            format!("Gained {amount} job experience")
        };
        self.game.chat_window.add_system(message);
    }
}
