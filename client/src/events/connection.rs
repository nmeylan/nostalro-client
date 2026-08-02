use super::lifecycle::SessionChange;
use super::preload_window;
use crate::App;
use ragnarok_game::app_state::AppState;
use ragnarok_game::entity::Entity;
use ragnarok_game::sprite_path::weapon_view_id_to_type;
use ragnarok_network::{
    KeepaliveMode, NetworkCommand, build_map_loaded_packet, build_select_accessible_map_packet,
    build_zone_enter_packet, ip_u32_to_string,
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
        let account_sex = self
            .game
            .session
            .login_session
            .as_ref()
            .map(|s| s.sex)
            .unwrap_or(0);
        for ch in &mut characters {
            ch.sex = account_sex;
        }
        self.game.sprite_caches.sprites.clear();
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
        self.game.session.app_state = AppState::CharacterSelect;
    }

    pub(crate) fn handle_character_created(
        &mut self,
        mut character: ragnarok_game::event::CharacterInfo,
    ) {
        tracing::info!(
            "Character '{}' created in slot {}",
            character.name,
            character.slot
        );
        character.sex = self
            .game
            .session
            .login_session
            .as_ref()
            .map(|s| s.sex)
            .unwrap_or(0);
        self.load_char_select_sprite(&character);
        if let Some(win) = &mut self.char_select_window {
            win.characters.push(character);
        }
        self.char_create_window = None;
        self.game.session.app_state = AppState::CharacterSelect;
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
        if let Some(session) = &mut self.game.session.login_session {
            session.store_zone_info(char_id, map_name);
        }
        let addr = format!("{}:{}", ip_u32_to_string(ip), port);
        self.channel.send_cmd(NetworkCommand::Disconnect);
        self.channel.send_cmd(NetworkCommand::Connect {
            addr,
            expect_aid: true,
        });
        if let Some(session) = &self.game.session.login_session {
            self.channel.send_packet(build_zone_enter_packet(session));
        }
        self.channel
            .send_cmd(NetworkCommand::SetKeepalive(KeepaliveMode::MapServer));
    }

    /// The re-entry handshake resends the landing cell in `ZC_ACCEPT_ENTER`, so the
    /// coordinates carried here are not needed.
    pub(super) fn handle_zone_server_changed(&mut self, map_name: String, ip: u32, port: i16) {
        tracing::info!(
            "Zone server change to '{map_name}' at {}:{port}",
            ip_u32_to_string(ip)
        );
        self.on_session_change(SessionChange::MapChange);
        self.clear_map_actors();
        self.game.session.current_map = None;
        self.game.character.inventory.clear();
        if let Some(session) = &mut self.game.session.login_session {
            session.map_name = map_name;
        }

        let addr = format!("{}:{}", ip_u32_to_string(ip), port);
        self.channel.send_cmd(NetworkCommand::Disconnect);
        self.channel.send_cmd(NetworkCommand::Connect {
            addr,
            expect_aid: true,
        });
        if let Some(session) = &self.game.session.login_session {
            self.channel.send_packet(build_zone_enter_packet(session));
        }
        self.channel
            .send_cmd(NetworkCommand::SetKeepalive(KeepaliveMode::MapServer));
    }

    pub(super) fn handle_accessible_maps_received(
        &mut self,
        maps: Vec<ragnarok_game::event::AccessibleMap>,
    ) {
        let Some(index) = maps.iter().position(|m| m.status == 0) else {
            tracing::warn!("No accessible map-server available for character");
            return;
        };
        let slot = self.config.last_char_slot.or_else(|| {
            self.game
                .session
                .selected_character
                .as_ref()
                .map(|c| c.slot as u8)
        });
        let Some(slot) = slot else {
            tracing::warn!("Received accessible maps with no selected character slot");
            return;
        };
        tracing::info!(
            "Redirecting character to accessible map '{}'",
            maps[index].name
        );
        self.channel.send_packet(build_select_accessible_map_packet(
            slot,
            index as u8,
            self.active_packetver,
        ));
    }

    pub(crate) fn handle_restart_ack(&mut self) {
        self.capture_window_state();
        self.config.save("config.json");
        self.on_session_change(SessionChange::Logout);
        self.reconnect_to_char_server();
    }

    pub(super) fn handle_map_entered(&mut self, x: u16, y: u16, dir: u8) {
        let map_name = self.game.session.login_session.as_ref().map(|s| {
            s.map_name
                .strip_suffix(".gat")
                .unwrap_or(&s.map_name)
                .to_string()
        });
        if let Some(map_name) = &map_name {
            tracing::info!("Entering map: {map_name}");
            self.load_map(map_name);
            self.game.session.current_map = Some(map_name.clone());
        }

        let session_sex = self
            .game
            .session
            .login_session
            .as_ref()
            .map(|s| s.sex)
            .unwrap_or(1);
        let account_id = self
            .game
            .session
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
            .session
            .selected_character
            .as_ref()
            .map(|c| {
                let sex = if self.active_packetver >= 20141016 {
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
        entity.is_gm = self.config.is_gm_account(account_id) && self.config.see_self_as_gm_when_gm;
        if let Some(guild) = self.game.guild.as_ref() {
            entity.guild_id = guild.gdid;
            entity.guild_emblem_version = guild.emblem_version;
        }
        self.game.world.entities.set_player_id(account_id);
        self.game.world.entities.insert(entity);
        self.game.character.hand_look = (weapon, shield_id);

        for &(bit, efst) in ragnarok_game::sprite_path::OPTION_STATUS_ICONS {
            if effect_state & bit != 0 {
                self.track_player_status(efst, true, 0, 0);
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
        if let Some((pid, design)) = self.game.player_cart_from_option() {
            self.spawn_cart_visual(pid, design);
        }
        self.refresh_player_status_buffs();

        self.warp_camera_to(x as f32, y as f32);
        self.char_select_window = None;

        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            preload_window(&mut self.windows.chat_window, renderer, grf);
            preload_window(&mut self.windows.system_menu, renderer, grf);
            preload_window(&mut self.windows.inventory_window, renderer, grf);
            preload_window(&mut self.windows.cart_window, renderer, grf);
            preload_window(&mut self.windows.storage_window, renderer, grf);
            preload_window(&mut self.windows.storage_password_window, renderer, grf);
            preload_window(&mut self.windows.trade_window, renderer, grf);
            preload_window(&mut self.windows.mailbox_window, renderer, grf);
            preload_window(&mut self.windows.read_mail_window, renderer, grf);
            preload_window(&mut self.windows.cart_select_window, renderer, grf);
            preload_window(&mut self.windows.equipment_window, renderer, grf);
            preload_window(&mut self.windows.npc_dialog, renderer, grf);
            preload_window(&mut self.windows.warp_list_window, renderer, grf);
            preload_window(&mut self.windows.item_list_selection_window, renderer, grf);
            preload_window(&mut self.windows.make_item_window, renderer, grf);
            preload_window(&mut self.windows.vending_shop_window, renderer, grf);
            preload_window(&mut self.windows.vending_setup_window, renderer, grf);
            preload_window(&mut self.windows.my_shop_window, renderer, grf);
            preload_window(&mut self.windows.npc_shop, renderer, grf);
            renderer.preload_textures(&chat_room_board::grf_texture_paths(), grf);
            preload_window(&mut self.windows.chat_room_create_window, renderer, grf);
            preload_window(&mut self.windows.chat_room_member_window, renderer, grf);
            preload_window(&mut self.windows.emotion_window, renderer, grf);
            preload_window(&mut self.windows.shortcut_list_window, renderer, grf);
            preload_window(&mut self.windows.quest_window, renderer, grf);
            preload_window(&mut self.windows.quest_detail_window, renderer, grf);
            preload_window(&mut self.windows.item_info_window, renderer, grf);
            preload_window(&mut self.windows.book_window, renderer, grf);
            preload_window(&mut self.windows.monster_info_window, renderer, grf);
            preload_window(&mut self.windows.sound_options, renderer, grf);
            preload_window(&mut self.windows.graphic_options, renderer, grf);
            preload_window(&mut self.windows.hotkey_config_window, renderer, grf);
            preload_window(&mut self.windows.item_pickup_notification, renderer, grf);
            preload_window(&mut self.windows.skill_tree_window, renderer, grf);
            preload_window(&mut self.windows.hotkey_bar, renderer, grf);
            preload_window(&mut self.windows.basic_info_window, renderer, grf);
            preload_window(&mut self.windows.minimap_window, renderer, grf);
            preload_window(&mut self.windows.world_map_window, renderer, grf);
            preload_window(&mut self.windows.status_window, renderer, grf);
            preload_window(&mut self.windows.levelup_notification, renderer, grf);
            preload_window(&mut self.windows.party_friends_window, renderer, grf);
            preload_window(&mut self.windows.party_helper_window, renderer, grf);
            preload_window(&mut self.windows.guild_window, renderer, grf);
            preload_window(&mut self.windows.emblem_picker_window, renderer, grf);
            preload_window(&mut self.windows.homunculus_window, renderer, grf);
            preload_window(&mut self.windows.mercenary_window, renderer, grf);
            preload_window(&mut self.windows.pet_window, renderer, grf);
            preload_window(&mut self.windows.mercenary_skill_window, renderer, grf);
            preload_window(&mut self.windows.homun_skill_window, renderer, grf);
            preload_window(&mut self.windows.companion_ai_config_window, renderer, grf);
            preload_window(&mut self.windows.confirm_dialog, renderer, grf);
            self.windows.drop_dialog_has_grf_textures =
                renderer.preload_textures(&DropQuantityDialog::grf_texture_paths(), grf);
            self.windows.card_insert_dialog_has_grf_textures =
                renderer.preload_textures(&CardInsertDialog::grf_texture_paths(), grf);

            renderer.preload_textures(&vending_board::grf_texture_paths(), grf);

            if let Some(current_map) = &self.game.session.current_map {
                let minimap_path = format!("data/texture/유저인터페이스/map/{}.bmp", current_map);
                if renderer.preload_textures(&[minimap_path.as_str()], grf) {
                    self.windows
                        .minimap_window
                        .set_map_texture(Some(minimap_path));
                } else {
                    tracing::warn!("Minimap texture not found: {minimap_path}");
                    self.windows.minimap_window.set_map_texture(None);
                }
            }
        }

        if let Some(info) = &self.game.session.selected_character {
            self.game.character.init_from_info(info);
        }
        self.refresh_level_aura(account_id);

        self.game.session.app_state = AppState::InGame;
        if !self.window_state_restored {
            self.game
                .apply_window_state(&mut self.windows, &self.config.window_state);
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
            .send_packet(build_map_loaded_packet(self.active_packetver));
    }

    /// Drop every actor but the player. A warp is authoritative about what is
    /// around us afterwards, so it applies to a warp landing on the map we are
    /// already standing on as much as to a real map change.
    fn clear_map_actors(&mut self) {
        let player_sprite = self
            .game
            .world
            .entities
            .player_id()
            .and_then(|pid| self.game.sprite_caches.sprites.remove(&pid));
        self.game.sprite_caches.sprites.clear();
        self.game.sprite_caches.gr2_models.clear();
        if let Some(renderer) = &mut self.renderer {
            renderer.gr2_models.clear();
        }
        self.game.world.entities.clear_non_player();
        self.game.companions.pet.clear_entity();
        self.game.quest_markers.clear();
        self.game.world.floor_items.clear();
        self.game.world.graffiti.clear();
        self.game.world.cast_marks.clear();
        self.game.world.talkbox_bubbles.clear();
        self.game.boss_mark = None;
        self.game.assets.floor_item_sprites.clear();
        self.game.schedulers.repeat_sounds.clear();
        if let Some(guild) = &mut self.game.guild {
            guild.clear_live_positions();
        }
        if let (Some(pid), Some(sprite)) = (self.game.world.entities.player_id(), player_sprite) {
            self.game.sprite_caches.sprites.insert(pid, sprite);
        }
        self.game.sprite_caches.carts.clear();
        self.game.sprite_caches.falcons.clear();
        if let Some(pid) = self.game.world.entities.player_id() {
            let (cart, falcon) = self
                .game
                .world
                .entities
                .get(pid)
                .map(|e| {
                    (
                        e.cart_type,
                        ragnarok_game::sprite_path::has_falcon(e.effect_state),
                    )
                })
                .unwrap_or((None, false));
            if let Some(design) = cart {
                self.spawn_cart_visual(pid, design);
            }
            if falcon {
                self.spawn_falcon_visual(pid);
            }
        }
    }

    pub(super) fn handle_map_changed(&mut self, map_name: String, x: i16, y: i16) {
        self.on_session_change(SessionChange::MapChange);
        let map_name = map_name
            .strip_suffix(".gat")
            .unwrap_or(&map_name)
            .to_string();
        if self.game.session.current_map.as_deref() != Some(&map_name) {
            self.load_map(&map_name);
            self.game.session.current_map = Some(map_name.clone());
            // Renderer-side gr2 models were already dropped by load_map.
            self.clear_map_actors();
            self.game.sprite_caches.sprite_cache.clear();
            self.game.sprite_caches.failed_sprite_loads.clear();

            if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
                let minimap_path = format!("data/texture/유저인터페이스/map/{}.bmp", map_name);
                if renderer.preload_textures(&[minimap_path.as_str()], grf) {
                    self.windows
                        .minimap_window
                        .set_map_texture(Some(minimap_path));
                } else {
                    self.windows.minimap_window.set_map_texture(None);
                }
            }
            self.windows.minimap_window.on_map_changed();
            self.game.minimap_marks.clear();
            self.windows.world_map_window.on_map_changed();
        } else {
            self.clear_map_actors();
        }
        if self.game.session.player_dead {
            if let Some(entity) = self.game.world.entities.player_mut() {
                entity.revive();
            }
            self.on_session_change(SessionChange::Resurrect);
        }
        if let Some(entity) = self.game.world.entities.player_mut() {
            entity.movement.set_position(x as f32, y as f32);
        }
        self.warp_camera_to(x as f32, y as f32);

        let surviving: Vec<u32> = self.game.world.entities.iter().map(|e| e.id).collect();
        for gid in surviving {
            self.refresh_level_aura(gid);
            self.refresh_boss_aura(gid);
            self.refresh_detect_aura(gid);
            self.refresh_warp_portal(gid);
        }
        self.refresh_player_status_buffs();

        self.game.character.inventory.clear();
        self.channel
            .send_packet(build_map_loaded_packet(self.active_packetver));
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
            .world
            .entities
            .player()
            .filter(|e| e.movement.is_moving())
            .and_then(|e| e.movement.destination())
            .is_some_and(|(dx, dy)| dx == dest_x && dy == dest_y);
        let local_ms = self.start_time.elapsed().as_millis() as u32;
        self.game
            .session
            .server_time
            .observe_server_tick(start_time, local_ms);
        if !already_moving_to_dest && let Some(gat) = &self.game.session.gat {
            let path = ragnarok_game::path::path_search(gat, start_x, start_y, dest_x, dest_y);
            // Start at local now, not the server tick: fast-forwarding jumps the
            // player forward by one round-trip at each segment seam.
            let now = local_ms as f32 / 1000.0;
            if let Some(entity) = self.game.world.entities.player_mut() {
                entity
                    .movement
                    .correct_to_cell(start_x as f32, start_y as f32);
                if !path.is_empty() {
                    entity.begin_move(path, now);
                }
            }
        }
    }

    pub(super) fn handle_server_tick(&mut self, server_tick: u32, local_send_time_ms: u32) {
        let local_now_ms = self.start_time.elapsed().as_millis() as u32;
        if self.config.enhanced_lag_compensation {
            self.game.session.server_time.on_server_tick_enhanced(
                server_tick,
                local_now_ms,
                local_send_time_ms,
            );
        } else {
            self.game.session.server_time.on_server_tick(
                server_tick,
                local_now_ms,
                local_send_time_ms,
            );
        }
    }

    pub(super) fn handle_disconnect_ack(&mut self, allowed: bool, event_loop: &ActiveEventLoop) {
        if allowed {
            self.capture_window_state();
            self.config.save("config.json");
            self.channel.send_cmd(NetworkCommand::Disconnect);
            event_loop.exit();
        } else {
            self.windows
                .chat_window
                .add_system("You cannot exit now.".to_string());
        }
    }

    pub(super) fn handle_disconnected(&mut self, reason: String, event_loop: &ActiveEventLoop) {
        if reason == "User exit" {
            event_loop.exit();
        } else if self.game.session.app_state == AppState::InGame {
            let reason_clone = reason.clone();
            self.game.session.disconnect_dialog_shown = true;
            self.windows.confirm_dialog.show(
                &format!("Disconnected from server: {reason_clone}"),
                false,
                |_| {},
            );
        } else {
            self.account_dialog
                .show(&format!("Disconnected: {reason}"), false, |_| {});
        }
    }
}

impl App {
    fn reconnect_to_char_server(&mut self) -> bool {
        if self.channel.cmd_tx.is_none() {
            return false;
        }
        let Some(session) = &self.game.session.login_session else {
            return false;
        };
        let Some(addr) = &session.char_server_addr else {
            return false;
        };
        self.channel.send_cmd(NetworkCommand::Disconnect);
        self.channel.send_cmd(NetworkCommand::Connect {
            addr: addr.clone(),
            expect_aid: true,
        });
        self.channel
            .send_packet(ragnarok_network::build_char_enter_packet(session));
        self.channel
            .send_cmd(NetworkCommand::SetKeepalive(KeepaliveMode::CharServer {
                account_id: session.account_id,
            }));
        self.game.session.app_state = AppState::CharacterSelect;
        true
    }
}
