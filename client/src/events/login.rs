use super::preload_window;
use crate::App;
use ragnarok_game::app_state::AppState;
use ragnarok_network::session::Session;
use ragnarok_ui_component::account::server_list_window::ServerListWindow;

impl App {
    pub(super) fn handle_login_accepted(
        &mut self,
        account_id: u32,
        login_id1: i32,
        login_id2: u32,
        sex: u8,
        servers: Vec<ragnarok_game::event::ServerInfo>,
    ) {
        tracing::info!("Login accepted, {} server(s)", servers.len());
        let mut session = Session::new(self.config.packetver);
        session.store_login(account_id, login_id1, login_id2, sex);
        self.game.login_session = Some(session);
        let mut server_win = ServerListWindow::new(servers);
        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            preload_window(&mut server_win, renderer, grf);
        }
        self.server_list_window = Some(server_win);
        self.game.app_state = AppState::ServerSelect;
    }

    pub(super) fn handle_login_refused(&mut self, error_code: u8) {
        let msg = match error_code {
            0 => "Unregistered ID",
            1 => "Incorrect Password",
            2 => "ID expired",
            3 => "Rejected from server",
            4 => "Blocked by GM",
            5 => "Not latest client",
            6 => "Banned",
            _ => "Unknown error",
        };
        self.login_window.set_error(msg);
    }
}
