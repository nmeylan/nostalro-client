use crate::App;
use ragnarok_game::banner::BannerKind;
use ragnarok_game::entity::ChatBubbleState;
use ragnarok_game::event::{FameKind, MvpFeedbackKind};
use ragnarok_game::skill_msg::{SKILL_MSG_COLOR, skill_msg_line};
use ragnarok_ui_component::game::chat_room_member_window::{OTHER_MSG_COLOR, OWN_MSG_COLOR};
use ragnarok_ui_component::game::chat_window::ChatChannel;
use ragnarok_ui_component::helper::colors::{CYAN, RED};

impl App {
    pub(super) fn handle_broadcast_message(
        &mut self,
        message: String,
        color: [f32; 4],
        banner: BannerKind,
    ) {
        match banner {
            BannerKind::Once => {
                self.game.broadcast.banner.enqueue(message, 1);
            }
            BannerKind::Repeat(times) => {
                self.game.broadcast.banner.enqueue(message.clone(), times);
                self.windows
                    .chat_window
                    .add_message(message, color, ChatChannel::System);
            }
            BannerKind::None => {
                self.game.broadcast.poptip.push(message.clone());
                self.windows
                    .chat_window
                    .add_message(message, color, ChatChannel::System);
            }
        }
    }

    pub(super) fn handle_chat_message(&mut self, gid: u32, message: String) {
        if let Some(bubble_text) = message.split(" : ").nth(1)
            && let Some(entity) = self.game.world.entities.get_mut(gid)
        {
            entity.chat_bubble = Some(ChatBubbleState::new(bubble_text.to_string()));
        }
        if self.windows.chat_room_member_window.is_open()
            && let Some((sender, _)) = message.split_once(" : ")
            && self.windows.chat_room_member_window.has_member(sender)
        {
            self.windows
                .chat_room_member_window
                .push_message(message.clone(), OTHER_MSG_COLOR);
        }
        let is_own = self
            .game
            .session
            .selected_character
            .as_ref()
            .zip(message.split_once(" : "))
            .is_some_and(|(c, (sender, _))| sender == c.name);
        if is_own || !self.game.prefs.hide_public_chat {
            if self.config.is_gm_account(gid) {
                self.windows
                    .chat_window
                    .add_chat_colored(message, ragnarok_game::targeting::GM_TEXT_COLOR);
            } else {
                self.windows.chat_window.add_chat(message);
            }
        }
    }

    pub(super) fn handle_ranking_received(&mut self, title: &str, entries: Vec<(String, i32)>) {
        self.windows
            .chat_window
            .add_system(format!("== {title} =="));
        if entries.is_empty() {
            self.windows
                .chat_window
                .add_system("  (no entries)".to_string());
        }
        for (i, (name, point)) in entries.iter().enumerate() {
            self.windows
                .chat_window
                .add_system(format!("  {}. {name} - {point}", i + 1));
        }
    }

    pub(super) fn handle_own_chat_message(&mut self, message: String) {
        if let Some(bubble_text) = message.split(" : ").nth(1)
            && let Some(player_id) = self.game.world.entities.player_id()
            && let Some(entity) = self.game.world.entities.get_mut(player_id)
        {
            entity.chat_bubble = Some(ChatBubbleState::new(bubble_text.to_string()));
        }
        if self.windows.chat_room_member_window.is_open() {
            self.windows
                .chat_room_member_window
                .push_message(message.clone(), OWN_MSG_COLOR);
        }
        let self_is_gm = self
            .game
            .world
            .entities
            .player_id()
            .and_then(|id| self.game.world.entities.get(id))
            .is_some_and(|e| e.is_gm);
        if self_is_gm {
            self.windows
                .chat_window
                .add_chat_colored(message, ragnarok_game::targeting::GM_TEXT_COLOR);
        } else {
            self.windows.chat_window.add_own_chat(message);
        }
    }

    pub(super) fn handle_guild_chat_message(&mut self, message: String) {
        self.windows.chat_window.add_guild(message);
    }

    pub(super) fn handle_whisper_received(&mut self, sender: String, message: String) {
        self.windows.chat_window.remember_whisper(sender.clone());
        self.windows.chat_window.add_whisper_in(sender, message);
    }

    pub(super) fn handle_whisper_ack(&mut self, result: u8) {
        let text = match result {
            0 => return,
            1 => "The target character is not logged in.",
            2 => "The target character is ignoring whispers.",
            3 => "The target character is ignoring everyone.",
            _ => "Failed to send whisper.",
        };
        self.windows.chat_window.add_error(text.to_string());
    }

    pub(super) fn handle_exp_gained(
        &mut self,
        aid: u32,
        amount: i32,
        is_base: bool,
        is_quest: bool,
    ) {
        if !self.game.prefs.show_exp {
            return;
        }
        let message = if is_base {
            format!("Gained {amount} base experience")
        } else {
            format!("Gained {amount} job experience")
        };
        self.windows.chat_window.add_system(message);
    }

    pub(super) fn handle_whisper_setting_result(&mut self, allow: bool, result: u8, all: bool) {
        let pending = self.game.pending_confirms.pending_whisper_block.take();
        let block = !allow;
        if all {
            let message = match (result, block) {
                (0, true) => "Blocking all whispers.",
                (0, false) => "Accepting all whispers.",
                _ => "Could not change your whisper setting.",
            };
            self.windows.chat_window.add_system(message.to_string());
            return;
        }
        let Some((name, _)) = pending else {
            return;
        };
        match result {
            0 => {
                self.game.prefs.blocked_whispers.retain(|n| n != &name);
                if block {
                    self.game.prefs.blocked_whispers.push(name.clone());
                }
                let verb = if block { "Blocked" } else { "Unblocked" };
                self.windows
                    .chat_window
                    .add_system(format!("{verb} whispers from {name}."));
            }
            2 => self
                .windows
                .chat_window
                .add_error("Your block list is full.".to_string()),
            _ => self
                .windows
                .chat_window
                .add_error(format!("Could not change whisper setting for {name}.")),
        }
    }

    pub(super) fn handle_memo_result(&mut self, result: u8) {
        match result {
            0 => self
                .windows
                .chat_window
                .add_system("Saved location as a Memo Point for Warp skill.".to_string()),
            1 => self
                .windows
                .chat_window
                .add_error("Skill Level is not high enough.".to_string()),
            _ => self
                .windows
                .chat_window
                .add_error("You haven't learned Warp.".to_string()),
        }
    }

    pub(super) fn handle_server_msg(&mut self, msg_id: u16) {
        let Some(message) = self
            .game
            .data_table
            .msg_string
            .as_ref()
            .and_then(|t| t.get(msg_id))
            .map(str::to_string)
        else {
            tracing::debug!("Unknown server msg id {msg_id}");
            return;
        };
        if ragnarok_game::data_table::msg_string_table::is_error_msg(msg_id) {
            self.windows.chat_window.add_error(message);
        } else {
            self.windows.chat_window.add_notice(message);
        }
    }

    pub(super) fn handle_skill_msg(&mut self, msg_no: i32) {
        let Some(line) = skill_msg_line(msg_no) else {
            tracing::debug!("Unknown skill msg {msg_no}");
            return;
        };
        self.windows.chat_window.add_message(
            line.to_string(),
            SKILL_MSG_COLOR,
            ChatChannel::System,
        );
        self.game.broadcast.poptip.push(line.to_string());
    }

    pub(super) fn handle_mvp_feedback(&mut self, kind: MvpFeedbackKind) {
        let (line, color) = match kind {
            MvpFeedbackKind::Item { item_id } => {
                let name = self
                    .game
                    .data_table
                    .item_name
                    .as_ref()
                    .map(|t| t.get_name_or_id(item_id))
                    .unwrap_or_else(|| format!("Item #{item_id}"));
                (
                    format!("Congratulations! You are the MVP! Your reward item is {name} !!"),
                    CYAN,
                )
            }
            MvpFeedbackKind::Exp { exp } => (
                format!("Congratulations! You are the MVP! Your reward EXP Points are {exp} !!"),
                CYAN,
            ),
            MvpFeedbackKind::ItemDropped => (
                "You are the MVP, but cannot obtain the reward because you are overweight."
                    .to_string(),
                RED,
            ),
        };
        self.windows
            .chat_window
            .add_message(line, color, ChatChannel::System);
    }

    pub(super) fn handle_fame_points_gained(&mut self, kind: FameKind, point: i32, total: i32) {
        self.windows
            .chat_window
            .add_system(kind.point_line(point, total));
    }

    pub(super) fn handle_pvp_points_received(&mut self, win: i32, lose: i32, point: i32) {
        self.windows.chat_window.add_message(
            format!("You have {win} win(s), {lose} loss(es) and {point} PvP point(s)."),
            CYAN,
            ChatChannel::System,
        );
    }
}
