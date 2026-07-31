use crate::App;
use ragnarok_game::gm::{GmStatus, manner_given_line, manner_result_line};

impl App {
    pub(super) fn handle_manner_point_result(&mut self, result: u32) {
        let Some(line) = manner_result_line(result) else {
            return;
        };
        if result >= 3 {
            self.windows.chat_window.add_error(line.to_string());
        } else {
            self.windows.chat_window.add_notice(line.to_string());
        }
    }

    pub(super) fn handle_manner_point_given(&mut self, positive: bool, other_name: String) {
        self.windows
            .chat_window
            .add_notice(manner_given_line(positive, &other_name));
    }

    pub(super) fn handle_gm_status(&mut self, status: &GmStatus) {
        let name = self
            .game
            .pending_confirms
            .pending_gm_check
            .take()
            .unwrap_or_else(|| "Status".to_string());
        self.windows.chat_window.add_system(format!("== {name} =="));
        for line in status.lines() {
            self.windows.chat_window.add_system(format!("  {line}"));
        }
    }

    pub(super) fn handle_account_name(&mut self, aid: u32, name: String) {
        let line = if name.is_empty() {
            format!("Account {aid}: name unavailable.")
        } else {
            format!("Account {aid}: {name}")
        };
        self.windows.chat_window.add_system(line);
    }
}
