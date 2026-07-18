use super::preload_window;
use crate::App;
use ragnarok_game::app_state::AppState;
use ragnarok_game::effect::EffectQueue;
use ragnarok_game::entity::Entity;
use ragnarok_game::sprite_path::weapon_view_id_to_type;
use ragnarok_game::targeting::MapProperties;
use ragnarok_network::{
    KeepaliveMode, NetworkCommand, build_map_loaded_packet, build_zone_enter_packet,
    ip_u32_to_string,
};
use ragnarok_ui_component::Window as UiWindow;
use ragnarok_ui_component::account::char_select_window::CharSelectWindow;
use ragnarok_ui_component::game::card_insert_dialog::CardInsertDialog;
use ragnarok_ui_component::game::chat_room_board;
use ragnarok_ui_component::game::drop_quantity_dialog::DropQuantityDialog;
use ragnarok_ui_component::game::vending_board;
use winit::event_loop::ActiveEventLoop;

impl App {
    pub(super) fn handle_character_list_received(
        &mut self,
        mut characters: Vec<ragnarok_game::event::CharacterInfo>,
    ) {
        tracing::info!("Received {} character(s)", characters.len());
        // Per-character sex is only sent from packetver 20141016; before that every
        // character on the account shares the account sex.
        let account_sex = self.game.login_session.as_ref().map(|s| s.sex).unwrap_or(0);
        for ch in &mut characters {
            ch.sex = account_sex;
        }
        self.game.sprites.clear();
        self.account_anims.clear();
        for ch in &characters {
            self.load_char_select_sprite(ch);
        }
        let mut char_win = CharSelectWindow::new(characters);
        char_win.preselect_slot(self.config.last_char_slot);
        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            preload_window(&mut char_win, renderer, grf);
        }
        self.char_select_window = Some(char_win);
        self.game.app_state = AppState::CharacterSelect;
    }

    pub(crate) fn handle_character_created(
        &mut self,
        mut character: ragnarok_game::event::CharacterInfo,
    ) {
        tracing::info!("Character '{}' created in slot {}", character.name, character.slot);
        character.sex = self.game.login_session.as_ref().map(|s| s.sex).unwrap_or(0);
        self.load_char_select_sprite(&character);
        if let Some(win) = &mut self.char_select_window {
            win.characters.push(character);
        }
        self.char_create_window = None;
        self.game.app_state = AppState::CharacterSelect;
    }

    fn load_char_select_sprite(&mut self, ch: &ragnarok_game::event::CharacterInfo) {
        let weapon = weapon_view_id_to_type(ch.weapon);
        self.load_player_sprite(
            ch.gid,
            ch.class,
            ch.sex,
            ch.head,
            ch.hair_color,
            0,
            weapon,
            ch.head_top,
            ch.head_mid,
            ch.head_bottom,
            ch.shield,
        );
        self.account_anims
            .insert(ch.gid, ragnarok_formats::act::SpriteAnimationState::new(0));
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

    pub(crate) fn handle_restart_ack(&mut self) {
        self.capture_window_state();
        self.config.save("config.json");
        self.window_state_restored = false;
        self.char_select_window = None;
        self.game.character.clear();
        self.game.entities.clear();
        self.game.sprites.clear();
        self.game.gr2_models.clear();
        if let Some(renderer) = &mut self.renderer {
            renderer.gr2_models.clear();
        }
        self.game.sprite_cache.clear();
        self.game.floor_items.clear();
        self.game.floor_item_sprites.clear();
        self.game.chat_rooms.clear();
        self.game.waiting_item_throw_ack = false;
        self.game.drop_quantity_dialog = None;
        self.game.guild_expel_dialog = None;
        self.game.card_insert_dialog = None;
        self.game.pending_card_composition_index = None;
        self.game.pending_pickup_item_id = None;
        self.game.attack_target_id = None;
        self.game.homunculus = None;
        self.game.mercenary = None;
        self.game.pet = ragnarok_game::pet::PetState::default();
        self.game.capture_targeting = false;
        self.game.pet_roulette = None;
        self.game.quest_log.clear();
        self.game.quest_markers.clear();
        self.game.pet_window.set_visible(false);
        self.game.companion_attack_target = [None; 2];
        self.game.homunculus_window.set_visible(false);
        self.game.mercenary_window.set_visible(false);
        self.game.guild = None;
        self.game.guild_head_sprites.clear();
        self.game.guild_window.open = false;
        self.game.current_map = None;
        self.game.map_coords = None;
        self.game.gat = None;
        self.effect_holder.clear();
        self.effect_queue = EffectQueue::new();
        self.game.ambient_effects = ragnarok_game::effects::AmbientEffectScheduler::empty();
        self.game.ambient_sounds =
            ragnarok_game::sound::ambient::AmbientSoundScheduler::empty();
        self.game.repeat_sounds.clear();
        self.game.status_buff_keys.clear();
        self.game.next_status_buff_key = 0;
        self.game.level_aura_keys.clear();
        self.game.boss_aura_keys.clear();
        self.game.warp_portal_keys.clear();
        self.game.sight_aura_keys.clear();
        self.game.ruwach_aura_keys.clear();
        self.game.day_night.reset();
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
        let (
            job,
            sex,
            head,
            hair_color,
            weapon,
            head_top,
            head_mid,
            head_bottom,
            shield_id,
            effect_state,
        ) = self
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
                    c.effect_state,
                )
            })
            .unwrap_or((0, session_sex, 0, 0, 0, 0, 0, 0, 0, 0));

        let mut entity = Entity::new_player(
            account_id,
            job,
            sex,
            head,
            hair_color,
            weapon,
            head_top,
            head_mid,
            head_bottom,
            shield_id,
            x,
            y,
            dir,
        );
        entity.effect_state = effect_state;
        self.game.entities.set_player_id(account_id);
        self.game.entities.insert(entity);

        for &(bit, efst) in ragnarok_game::sprite_path::OPTION_STATUS_ICONS {
            if effect_state & bit != 0 {
                self.set_status_icon(efst, true, 0, 0);
            }
        }

        let sprite_job = ragnarok_game::sprite_path::visual_job(job, effect_state);
        let weapon_type = weapon_view_id_to_type(weapon);
        self.load_player_sprite(
            account_id,
            sprite_job,
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

        if ragnarok_game::sprite_path::has_falcon(effect_state) {
            self.spawn_falcon_visual(account_id);
        }

        self.position_camera_at(x as f32, y as f32);
        self.char_select_window = None;

        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            preload_window(&mut self.game.chat_window, renderer, grf);
            preload_window(&mut self.game.system_menu, renderer, grf);
            preload_window(&mut self.game.inventory_window, renderer, grf);
            preload_window(&mut self.game.cart_window, renderer, grf);
            preload_window(&mut self.game.storage_window, renderer, grf);
            preload_window(&mut self.game.trade_window, renderer, grf);
            preload_window(&mut self.game.mailbox_window, renderer, grf);
            preload_window(&mut self.game.read_mail_window, renderer, grf);
            preload_window(&mut self.game.cart_select_window, renderer, grf);
            preload_window(&mut self.game.equipment_window, renderer, grf);
            preload_window(&mut self.game.npc_dialog, renderer, grf);
            preload_window(&mut self.game.warp_list_window, renderer, grf);
            preload_window(&mut self.game.item_list_selection_window, renderer, grf);
            preload_window(&mut self.game.make_item_window, renderer, grf);
            preload_window(&mut self.game.vending_shop_window, renderer, grf);
            preload_window(&mut self.game.vending_setup_window, renderer, grf);
            preload_window(&mut self.game.my_shop_window, renderer, grf);
            preload_window(&mut self.game.npc_shop, renderer, grf);
            renderer.preload_textures(&chat_room_board::grf_texture_paths(), grf);
            preload_window(&mut self.game.chat_room_create_window, renderer, grf);
            preload_window(&mut self.game.chat_room_member_window, renderer, grf);
            preload_window(&mut self.game.emotion_window, renderer, grf);
            preload_window(&mut self.game.shortcut_list_window, renderer, grf);
            preload_window(&mut self.game.quest_window, renderer, grf);
            preload_window(&mut self.game.quest_detail_window, renderer, grf);
            preload_window(&mut self.game.item_info_window, renderer, grf);
            preload_window(&mut self.game.book_window, renderer, grf);
            preload_window(&mut self.game.sound_options, renderer, grf);
            preload_window(&mut self.game.graphic_options, renderer, grf);
            preload_window(&mut self.game.hotkey_config_window, renderer, grf);
            preload_window(&mut self.game.item_pickup_notification, renderer, grf);
            preload_window(&mut self.game.skill_tree_window, renderer, grf);
            preload_window(&mut self.game.hotkey_bar, renderer, grf);
            preload_window(&mut self.game.basic_info_window, renderer, grf);
            preload_window(&mut self.game.minimap_window, renderer, grf);
            preload_window(&mut self.game.status_window, renderer, grf);
            preload_window(&mut self.game.levelup_notification, renderer, grf);
            preload_window(&mut self.game.party_friends_window, renderer, grf);
            preload_window(&mut self.game.party_helper_window, renderer, grf);
            preload_window(&mut self.game.guild_window, renderer, grf);
            preload_window(&mut self.game.emblem_picker_window, renderer, grf);
            preload_window(&mut self.game.homunculus_window, renderer, grf);
            preload_window(&mut self.game.mercenary_window, renderer, grf);
            preload_window(&mut self.game.pet_window, renderer, grf);
            preload_window(&mut self.game.mercenary_skill_window, renderer, grf);
            preload_window(&mut self.game.homun_skill_window, renderer, grf);
            preload_window(&mut self.game.confirm_dialog, renderer, grf);
            self.game.drop_dialog_has_grf_textures =
                renderer.preload_textures(&DropQuantityDialog::grf_texture_paths(), grf);
            self.game.card_insert_dialog_has_grf_textures =
                renderer.preload_textures(&CardInsertDialog::grf_texture_paths(), grf);

            renderer.preload_textures(&vending_board::grf_texture_paths(), grf);

            if let Some(current_map) = &self.game.current_map {
                let minimap_path = format!("data/texture/유저인터페이스/map/{}.bmp", current_map);
                if renderer.preload_textures(&[minimap_path.as_str()], grf) {
                    self.game.minimap_window.set_map_texture(Some(minimap_path));
                } else {
                    tracing::warn!("Minimap texture not found: {minimap_path}");
                    self.game.minimap_window.set_map_texture(None);
                }
            }
        }

        if let Some(info) = &self.game.selected_character {
            self.game.character.init_from_info(info);
        }
        self.refresh_level_aura(account_id);

        self.game.app_state = AppState::InGame;
        if !self.window_state_restored {
            self.game.apply_window_state(&self.config.window_state);
            self.window_state_restored = true;
        }
        self.game
            .character
            .hotkeys
            .set_visible_rows(self.config.hotkey_visible_rows);
        self.game
            .character
            .hotkeys
            .set_battle_mode(self.config.battle_mode);

        self.game.character.inventory.clear();
        self.channel
            .send_packet(build_map_loaded_packet(self.config.packetver));
    }

    fn clear_transient_effects_and_sounds(&mut self) {
        self.effect_queue.clear();
        self.effect_holder.clear();
        self.sound_queue.clear();
        self.sound.stop_all_sfx();
        self.game.arrows.clear();
        self.game.damage_numbers.clear();
        self.game.status_buff_keys.clear();
        self.game.next_status_buff_key = 0;
        self.game.level_aura_keys.clear();
        self.game.boss_aura_keys.clear();
        self.game.warp_portal_keys.clear();
        self.game.spirit_keys.clear();
        self.game.sight_aura_keys.clear();
        self.game.ruwach_aura_keys.clear();
    }

    pub(super) fn handle_map_changed(&mut self, map_name: String, x: i16, y: i16) {
        self.clear_transient_effects_and_sounds();
        self.game.character.storage.clear();
        self.game.character.trade.reset();
        self.game.trade_window.reset_input();
        self.game.pending_trade_partner = None;
        self.game.map_properties = MapProperties::default();
        self.game.pending_skill_target = None;
        self.game.pending_skill_id = None;
        self.game.pending_skill_level = None;
        self.game.attack_target_id = None;
        self.game.npc_cutins = [None, None, None];
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
            self.game.current_map = Some(map_name.clone());
            let player_sprite = self
                .game
                .entities
                .player_id()
                .and_then(|pid| self.game.sprites.remove(&pid));
            self.game.sprites.clear();
            // Renderer-side gr2 models were already dropped by load_map.
            self.game.gr2_models.clear();
            self.game.sprite_cache.clear();
            self.game.entities.clear_non_player();
            self.game.pet.clear_entity();
            self.game.quest_markers.clear();
            self.game.failed_sprite_loads.clear();
            self.game.floor_items.clear();
            self.game.floor_item_sprites.clear();
            if let Some(guild) = &mut self.game.guild {
                guild.clear_live_positions();
            }
            if let (Some(pid), Some(sprite)) = (self.game.entities.player_id(), player_sprite) {
                self.game.sprites.insert(pid, sprite);
            }

            if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
                let minimap_path = format!("data/texture/유저인터페이스/map/{}.bmp", map_name);
                if renderer.preload_textures(&[minimap_path.as_str()], grf) {
                    self.game.minimap_window.set_map_texture(Some(minimap_path));
                } else {
                    self.game.minimap_window.set_map_texture(None);
                }
            }
            self.game.minimap_window.on_map_changed();
        }
        if self.game.player_dead {
            if let Some(entity) = self.game.entities.player_mut() {
                entity.revive();
            }
            self.game.player_dead = false;
            self.game.system_menu.close_dead();
        }
        if let Some(entity) = self.game.entities.player_mut() {
            entity.movement.set_position(x as f32, y as f32);
        }
        self.position_camera_at(x as f32, y as f32);

        let surviving: Vec<u32> = self.game.entities.iter().map(|e| e.id).collect();
        for gid in surviving {
            self.refresh_level_aura(gid);
            self.refresh_boss_aura(gid);
            self.refresh_detect_aura(gid);
        }

        self.game.character.inventory.clear();
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
        let local_ms = self.start_time.elapsed().as_millis() as u32;
        self.game
            .server_time
            .observe_server_tick(start_time, local_ms);
        if !already_moving_to_dest && let Some(gat) = &self.game.gat {
            let path = ragnarok_game::path::path_search(gat, start_x, start_y, dest_x, dest_y);
            // Start at local now, not the server tick: fast-forwarding jumps the
            // player forward by one round-trip at each segment seam.
            let now = local_ms as f32 / 1000.0;
            if let Some(entity) = self.game.entities.player_mut() {
                entity.movement.correct_to_cell(start_x as f32, start_y as f32);
                if !path.is_empty() {
                    entity.movement.start_move(path, now);
                }
            }
        }
    }

    pub(super) fn handle_server_tick(&mut self, server_tick: u32, local_send_time_ms: u32) {
        let local_now_ms = self.start_time.elapsed().as_millis() as u32;
        if self.config.enhanced_lag_compensation {
            self.game.server_time.on_server_tick_enhanced(
                server_tick,
                local_now_ms,
                local_send_time_ms,
            );
        } else {
            self.game
                .server_time
                .on_server_tick(server_tick, local_now_ms, local_send_time_ms);
        }
    }

    pub(super) fn handle_disconnect_ack(&mut self, allowed: bool, event_loop: &ActiveEventLoop) {
        if allowed {
            self.capture_window_state();
            self.config.save("config.json");
            self.channel.send_cmd(NetworkCommand::Disconnect);
            event_loop.exit();
        } else {
            self.game
                .chat_window
                .add_system("You cannot exit now.".to_string());
        }
    }

    pub(super) fn handle_disconnected(&mut self, reason: String, event_loop: &ActiveEventLoop) {
        if reason == "User exit" {
            event_loop.exit();
        } else if self.game.app_state == AppState::InGame {
            let reason_clone = reason.clone();
            self.game.disconnect_dialog_shown = true;
            self.game.confirm_dialog.show(
                &format!("Disconnected from server: {reason_clone}"),
                false,
                |_| {},
            );
        } else {
            self.login_window
                .set_error(&format!("Disconnected: {reason}"));
        }
    }
}
