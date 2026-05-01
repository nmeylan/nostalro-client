use crate::App;
use ragnarok_game::app_state::AppState;
use ragnarok_game::entity::Entity;
use ragnarok_game::sprite_path::weapon_view_id_to_type;
use ragnarok_network::{
    build_map_loaded_packet, build_zone_enter_packet, ip_u32_to_string,
    KeepaliveMode, NetworkCommand,
};
use ragnarok_ui_component::account::char_select_window::CharSelectWindow;
use ragnarok_ui_component::game::card_insert_dialog::CardInsertDialog;
use ragnarok_ui_component::game::drop_quantity_dialog::DropQuantityDialog;
use ragnarok_ui_component::Window as UiWindow;
use winit::event_loop::ActiveEventLoop;
use super::preload_window;

impl App {
    pub(super) fn handle_character_list_received(
        &mut self,
        characters: Vec<ragnarok_game::event::CharacterInfo>,
    ) {
        tracing::info!("Received {} character(s)", characters.len());
        let mut char_win = CharSelectWindow::new(characters);
        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            preload_window(&mut char_win, renderer, grf);
        }
        self.char_select_window = Some(char_win);
        self.game.app_state = AppState::CharacterSelect;
    }

    pub(super) fn handle_zone_server_connect_info(
        &mut self,
        char_id: u32,
        map_name: String,
        ip: u32,
        port: i16,
    ) {
        if let Some(session) = &mut self.game.login_session {
            session.store_zone_info(char_id, map_name);
        }
        let addr = format!("{}:{}", ip_u32_to_string(ip), port);
        self.channel.send_cmd(NetworkCommand::Disconnect);
        self.channel.send_cmd(NetworkCommand::Connect(addr));
        if let Some(session) = &self.game.login_session {
            self.channel.send_packet(build_zone_enter_packet(session));
        }
        self.channel
            .send_cmd(NetworkCommand::SetKeepalive(KeepaliveMode::MapServer));
    }

    pub(super) fn handle_restart_ack(&mut self) {
        self.char_select_window = None;
        self.game.character.clear();
        self.game.entities.clear();
        self.game.sprites.clear();
        self.game.sprite_cache.clear();
        self.game.floor_items.clear();
        self.game.floor_item_sprites.clear();
        self.game.waiting_item_throw_ack = false;
        self.game.drop_quantity_dialog = None;
        self.game.card_insert_dialog = None;
        self.game.pending_card_composition_index = None;
        self.game.pending_pickup_item_id = None;
        self.game.attack_target_id = None;
        self.game.current_map = None;
        self.game.map_coords = None;
        self.game.gat = None;
        if let Some(renderer) = &mut self.renderer {
            renderer.ground_renderer = None;
            renderer.model_renderer = None;
            renderer.water_renderer = None;
            renderer.grid_selector = None;
        }
        self.reconnect_to_char_server();
    }

    pub(super) fn handle_map_entered(&mut self, x: u16, y: u16, dir: u8) {
        let map_name = self.game.login_session.as_ref().map(|s| {
            s.map_name
                .strip_suffix(".gat")
                .unwrap_or(&s.map_name)
                .to_string()
        });
        if let Some(map_name) = &map_name {
            tracing::info!("Entering map: {map_name}");
            self.load_map(map_name);
            self.game.current_map = Some(map_name.clone());
        }

        let session_sex = self.game.login_session.as_ref().map(|s| s.sex).unwrap_or(1);
        let account_id = self
            .game
            .login_session
            .as_ref()
            .map(|s| s.account_id)
            .unwrap_or(0);
        let (job, sex, head, hair_color, weapon, head_top, head_mid, head_bottom, shield_id) = self
            .game
            .selected_character
            .as_ref()
            .map(|c| {
                let sex = if self.config.packetver >= 20141016 {
                    c.sex
                } else {
                    session_sex
                };
                (
                    c.class,
                    sex,
                    c.head,
                    c.hair_color,
                    c.weapon,
                    c.head_top,
                    c.head_mid,
                    c.head_bottom,
                    c.shield,
                )
            })
            .unwrap_or((0, session_sex, 0, 0, 0, 0, 0, 0, 0));

        let entity = Entity::new_player(
            account_id, job, sex, head, hair_color, weapon, head_top, head_mid, head_bottom,
            shield_id, x, y, dir,
        );
        self.game.entities.set_player_id(account_id);
        self.game.entities.insert(entity);

        let weapon_type = weapon_view_id_to_type(weapon);
        self.load_player_sprite(
            account_id,
            job,
            sex,
            head,
            hair_color,
            0,
            weapon_type,
            head_top,
            head_mid,
            head_bottom,
            shield_id,
        );

        self.position_camera_at(x as f32, y as f32);
        self.char_select_window = None;

        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            preload_window(&mut self.game.chat_window, renderer, grf);
            preload_window(&mut self.game.system_menu, renderer, grf);
            preload_window(&mut self.game.inventory_window, renderer, grf);
            preload_window(&mut self.game.equipment_window, renderer, grf);
            preload_window(&mut self.game.npc_dialog, renderer, grf);
            preload_window(&mut self.game.npc_shop, renderer, grf);
            preload_window(&mut self.game.item_info_window, renderer, grf);
            preload_window(&mut self.game.item_pickup_notification, renderer, grf);
            preload_window(&mut self.game.skill_tree_window, renderer, grf);
            preload_window(&mut self.game.hotkey_bar, renderer, grf);
            self.game.drop_dialog_has_grf_textures =
                renderer.preload_textures(&DropQuantityDialog::grf_texture_paths(), grf);
            self.game.card_insert_dialog_has_grf_textures =
                renderer.preload_textures(&CardInsertDialog::grf_texture_paths(), grf);
        }

        if let Some(info) = &self.game.selected_character {
            self.game.character.init_from_info(info);
        }

        self.game.app_state = AppState::InGame;
        self.game.apply_window_state(&self.config.window_state);
        self.game
            .character
            .hotkeys
            .set_visible_rows(self.config.hotkey_visible_rows);
        self.game
            .character
            .hotkeys
            .set_battle_mode(self.config.battle_mode);

        self.channel
            .send_packet(build_map_loaded_packet(self.config.packetver));
    }

    pub(super) fn handle_map_changed(&mut self, map_name: String, x: i16, y: i16) {
        self.game.pending_skill_target = None;
        self.game.pending_skill_id = None;
        self.game.pending_skill_level = None;
        self.game.attack_target_id = None;
        let map_name = map_name
            .strip_suffix(".gat")
            .unwrap_or(&map_name)
            .to_string();
        tracing::info!(
            "MapChanged: {map_name} ({x},{y}) current={:?}",
            self.game.current_map
        );
        if self.game.current_map.as_deref() != Some(&map_name) {
            tracing::info!("Different map, clearing entities");
            self.load_map(&map_name);
            self.game.current_map = Some(map_name);
            let player_sprite = self
                .game
                .entities
                .player_id()
                .and_then(|pid| self.game.sprites.remove(&pid));
            self.game.sprites.clear();
            self.game.sprite_cache.clear();
            self.game.entities.clear_non_player();
            self.game.failed_sprite_loads.clear();
            self.game.floor_items.clear();
            self.game.floor_item_sprites.clear();
            if let (Some(pid), Some(sprite)) = (self.game.entities.player_id(), player_sprite) {
                self.game.sprites.insert(pid, sprite);
            }
        }
        if let Some(entity) = self.game.entities.player_mut() {
            entity.movement.set_position(x as f32, y as f32);
        }
        self.position_camera_at(x as f32, y as f32);

        self.channel
            .send_packet(build_map_loaded_packet(self.config.packetver));
    }

    pub(super) fn handle_player_moved(
        &mut self,
        start_x: u16,
        start_y: u16,
        dest_x: u16,
        dest_y: u16,
        start_time: u32,
    ) {
        self.input.walk_server_acked = true;
        let already_moving_to_dest = self
            .game
            .entities
            .player()
            .filter(|e| e.movement.is_moving())
            .and_then(|e| e.movement.destination())
            .is_some_and(|(dx, dy)| dx == dest_x && dy == dest_y);
        if !already_moving_to_dest {
            if let Some(gat) = &self.game.gat {
                let (sx, sy) = self
                    .game
                    .entities
                    .player()
                    .map(|e| e.movement.cell_position())
                    .unwrap_or((start_x, start_y));
                let path = ragnarok_game::path::path_search(gat, sx, sy, dest_x, dest_y);
                if !path.is_empty() {
                    let local_ms = self.start_time.elapsed().as_millis() as u32;
                    let move_start = self
                        .game
                        .server_time
                        .server_to_local_secs(start_time, local_ms);
                    if let Some(entity) = self.game.entities.player_mut() {
                        entity.movement.set_position(sx as f32, sy as f32);
                        entity.movement.start_move(path, move_start);
                    }
                }
            }
        }
    }

    pub(super) fn handle_server_tick(&mut self, server_tick: u32, local_send_time_ms: u32) {
        let local_now_ms = self.start_time.elapsed().as_millis() as u32;
        if self.config.enhanced_lag_compensation {
            self.game
                .server_time
                .on_server_tick_enhanced(server_tick, local_now_ms, local_send_time_ms);
        } else {
            self.game
                .server_time
                .on_server_tick(server_tick, local_now_ms, local_send_time_ms);
        }
    }

    pub(super) fn handle_disconnected(&mut self, reason: String, event_loop: &ActiveEventLoop) {
        self.game.server_time.reset();
        self.game.character.inventory.clear();
        self.game.floor_items.clear();
        self.game.floor_item_sprites.clear();
        self.game.waiting_item_throw_ack = false;
        self.game.drop_quantity_dialog = None;
        self.game.card_insert_dialog = None;
        self.game.pending_card_composition_index = None;
        self.game.pending_pickup_item_id = None;
        if reason == "User exit" {
            event_loop.exit();
        } else {
            self.login_window
                .set_error(&format!("Disconnected: {reason}"));
        }
    }
}
