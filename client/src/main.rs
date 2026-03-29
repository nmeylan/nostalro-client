mod config;
mod game_state;
mod input;

use config::Config;
use game_state::GameState;
use input::InputState;
use ragnarok_formats::act::SpriteActionType;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::app_state::AppState;
use ragnarok_game::cursor::{CursorType, cursor_type_for_cell, hovered_entity_cursor_type};
use ragnarok_game::entity::{Entity, EntityState, EntityType};
use ragnarok_game::event::GameEvent;
use ragnarok_game::name_table::NameTable;
use ragnarok_game::sprite_path::{WeaponType, weapon_view_id_to_type, entity_type_from_job, entity_sprite_base_path};
use ragnarok_game::path::{path_search, try_move_to};
use ragnarok_game::shadow::shadow_size;
use ragnarok_game::{map_loader, sprite_loader};
use ragnarok_network::{build_action_request_packet, build_char_enter_packet, build_chat_packet, build_login_packet, build_map_loaded_packet, build_request_move_packet, build_select_char_packet, build_zone_enter_packet, ip_u32_to_string, network_loop, NetworkCommand, KeepaliveMode};
use ragnarok_network::session::Session;
use ragnarok_renderer::{GridSelectorRenderer, Renderer, SpriteBatch, SpriteVertex, UiDrawCall, build_clip_quad, upload_sprite_textures, build_entity_sprite, block_on};
use ragnarok_ui::context::UiContext;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui_component::chat_window::ChatWindow;
use ragnarok_ui_component::login_window::{LoginFocus, LoginWindow};
use ragnarok_ui_component::char_select_window::CharSelectWindow;
use ragnarok_ui_component::server_list_window::ServerListWindow;
use ragnarok_ui::state::StateCache;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

type ClipData = (Vec<SpriteVertex>, Vec<u32>, usize);

struct App {
    config: Config,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    grf: Option<GrfArchive>,
    input: InputState,
    ui_context: Option<UiContext>,
    ui_state_cache: StateCache,
    login_window: LoginWindow,
    server_list_window: Option<ServerListWindow>,
    char_select_window: Option<CharSelectWindow>,
    network_cmd_tx: Option<mpsc::UnboundedSender<NetworkCommand>>,
    game_event_rx: Option<mpsc::UnboundedReceiver<GameEvent>>,
    game: GameState,
    start_time: Instant,
    last_render_time: f32,
}

impl App {
    fn new(config: Config) -> Self {
        Self {
            config,
            window: None,
            renderer: None,
            grf: None,
            input: InputState::new(),
            ui_context: None,
            ui_state_cache: StateCache::new(),
            login_window: LoginWindow::new(),
            server_list_window: None,
            char_select_window: None,
            network_cmd_tx: None,
            game_event_rx: None,
            game: GameState::new(),
            start_time: Instant::now(),
            last_render_time: 0.0,
        }
    }

    fn load_map(&mut self, map_name: &str) {
        let grf = match &self.grf {
            Some(g) => g,
            None => return,
        };

        let map_data = match map_loader::load_map_data(grf, map_name) {
            Some(d) => d,
            None => return,
        };

        self.game.map_coords = map_data.coordinates;
        self.game.gat = map_data.gat;

        if let Some(renderer) = &mut self.renderer {
            renderer.load_map(&map_data.gnd, &map_data.rsw, grf);

            if let Some(gat) = &self.game.gat {
                let mut grid = GridSelectorRenderer::new(
                    &renderer.device.device,
                    &renderer.device.queue,
                    renderer.device.surface_format,
                    &renderer.global_uniforms,
                    &mut renderer.texture_cache,
                    grf,
                );
                grid.build_grid_mesh(
                    &renderer.device.device, gat,
                    map_data.gnd.width, map_data.gnd.height, map_data.gnd.zoom,
                );
                renderer.grid_selector = Some(grid);
            }
        }
    }

    fn position_camera_at(&mut self, cell_x: f32, cell_y: f32) {
        if let (Some(coords), Some(renderer)) = (&self.game.map_coords, &mut self.renderer) {
            input::position_camera_at(&mut renderer.camera, self.game.gat.as_ref(), coords, cell_x, cell_y);
        }
    }

    fn hovered_cell(&self) -> Option<(i32, i32)> {
        let (renderer, coords) = match (&self.renderer, &self.game.map_coords) {
            (Some(r), Some(c)) => (r, c),
            _ => return None,
        };
        input::hovered_cell(
            self.input.mouse_position,
            &renderer.camera,
            renderer.device.surface_config.width as f32,
            renderer.device.surface_config.height as f32,
            coords,
        )
    }

    fn handle_left_click(&mut self) {
        let (dest_x, dest_y) = match self.hovered_cell() {
            Some(c) => c,
            None => return,
        };
        let gat = match &self.game.gat {
            Some(g) => g,
            None => return,
        };

        let (src_x, src_y) = self.game.entities.player()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));

        let move_action = match try_move_to(gat, src_x, src_y, dest_x, dest_y) {
            Some(a) => a,
            None => return,
        };

        if let Some(tx) = &self.network_cmd_tx {
            let packet = build_request_move_packet(move_action.dest_x, move_action.dest_y, self.config.packetver);
            let _ = tx.send(NetworkCommand::SendPacket(packet));
        }

        let elapsed = self.start_time.elapsed().as_secs_f32();
        if let Some(entity) = self.game.entities.player_mut() {
            entity.movement.start_move(move_action.path, elapsed);
        }
    }

    fn spawn_network(&mut self) {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        self.network_cmd_tx = Some(cmd_tx);
        self.game_event_rx = Some(event_rx);

        let packetver = self.config.packetver;
        let debug_delay_ms = self.config.debug_network_delay_ms;
        // Spawn on dedicated thread with single-threaded runtime
        // because network_loop uses non-Send packet types
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create network runtime");
            rt.block_on(network_loop(cmd_rx, event_tx, packetver, debug_delay_ms));
        });
    }

    fn handle_game_events(&mut self, event_loop: &ActiveEventLoop) {
        let events: Vec<_> = self.game_event_rx.as_mut()
            .map(|rx| std::iter::from_fn(|| rx.try_recv().ok()).collect())
            .unwrap_or_default();
        for event in events {
                match event {
                    GameEvent::LoginAccepted { account_id, login_id1, login_id2, sex, servers } => {
                        tracing::info!("Login accepted, {} server(s)", servers.len());
                        let mut session = Session::new(self.config.packetver);
                        session.store_login(account_id, login_id1, login_id2, sex);
                        self.game.login_session = Some(session);
                        let mut server_win = ServerListWindow::new(servers);
                        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
                            server_win.has_grf_textures = renderer.preload_textures(&ServerListWindow::grf_texture_paths(), grf);
                            if server_win.has_grf_textures {
                                server_win.set_texture_sizes(|name| {
                                    renderer.texture_cache.texture_size(name)
                                });
                            }
                        }
                        self.server_list_window = Some(server_win);
                        self.game.app_state = AppState::ServerSelect;
                    }
                    GameEvent::LoginRefused { error_code } => {
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
                    GameEvent::CharacterListReceived { characters } => {
                        tracing::info!("Received {} character(s)", characters.len());
                        let mut char_win = CharSelectWindow::new(characters);
                        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
                            char_win.has_grf_textures = renderer.preload_textures(&CharSelectWindow::grf_texture_paths(), grf);
                            if char_win.has_grf_textures {
                                char_win.set_texture_sizes(|name| {
                                    renderer.texture_cache.texture_size(name)
                                });
                            }
                        }
                        self.char_select_window = Some(char_win);
                        self.game.app_state = AppState::CharacterSelect;
                    }
                    GameEvent::ZoneServerConnectInfo { char_id, map_name, ip, port } => {
                        if let Some(session) = &mut self.game.login_session {
                            session.store_zone_info(char_id, map_name);
                        }
                        let addr = format!("{}:{}", ip_u32_to_string(ip), port);
                        if let Some(tx) = &self.network_cmd_tx {
                            let _ = tx.send(NetworkCommand::Disconnect);
                            let _ = tx.send(NetworkCommand::Connect(addr));
                            if let Some(session) = &self.game.login_session {
                                let packet = build_zone_enter_packet(session);
                                let _ = tx.send(NetworkCommand::SendPacket(packet));
                            }
                            let _ = tx.send(NetworkCommand::SetKeepalive(KeepaliveMode::MapServer));
                        }
                    }
                    GameEvent::MapEntered { x, y, dir, .. } => {
                        let map_name = self.game.login_session.as_ref().map(|s| {
                            s.map_name.strip_suffix(".gat")
                                .unwrap_or(&s.map_name).to_string()
                        });
                        if let Some(map_name) = &map_name {
                            tracing::info!("Entering map: {map_name}");
                            self.load_map(map_name);
                            self.game.current_map = Some(map_name.clone());
                        }

                        let session_sex = self.game.login_session.as_ref().map(|s| s.sex).unwrap_or(1);
                        let (job, sex, head, hair_color, weapon, head_top, head_mid, head_bottom, shield_id, char_id) = self.game.selected_character.as_ref()
                            .map(|c| {
                                let sex = if self.config.packetver >= 20141016 { c.sex } else { session_sex };
                                (c.class, sex, c.head, c.hair_color, c.weapon, c.head_top, c.head_mid, c.head_bottom, c.shield, c.gid)
                            })
                            .unwrap_or((0, session_sex, 0, 0, 0, 0, 0, 0, 0, 0));

                        let entity = Entity::new_player(char_id, job, sex, head, hair_color, weapon, head_top, head_mid, head_bottom, shield_id, x, y, dir);
                        self.game.entities.set_player_id(char_id);
                        self.game.entities.insert(entity);

                        let weapon_type = weapon_view_id_to_type(weapon);
                        self.load_player_sprite(char_id, job, sex, head, weapon_type, head_top, head_mid, head_bottom, shield_id);

                        self.position_camera_at(x as f32, y as f32);
                        self.char_select_window = None;

                        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
                            self.game.chat_window.has_grf_textures = renderer.preload_textures(
                                &ChatWindow::grf_texture_paths(), grf,
                            );
                        }

                        self.game.app_state = AppState::InGame;

                        if let Some(tx) = &self.network_cmd_tx {
                            let _ = tx.send(NetworkCommand::SendPacket(build_map_loaded_packet(self.config.packetver)));
                        }
                    }
                    GameEvent::PlayerMoved { start_x, start_y, dest_x, dest_y, start_time } => {
                        self.input.walk_server_acked = true;
                        let already_moving_to_dest = self.game.entities.player()
                            .filter(|e| e.movement.is_moving())
                            .and_then(|e| e.movement.destination())
                            .is_some_and(|(dx, dy)| dx == dest_x && dy == dest_y);
                        if !already_moving_to_dest {
                            if let Some(gat) = &self.game.gat {
                                // Start from rendered position (like original client)
                                let (sx, sy) = self.game.entities.player()
                                    .map(|e| e.movement.cell_position())
                                    .unwrap_or((start_x, start_y));
                                let path = path_search(gat, sx, sy, dest_x, dest_y);
                                if !path.is_empty() {
                                    let local_ms = self.start_time.elapsed().as_millis() as u32;
                                    let move_start = self.game.server_time.server_to_local_secs(start_time, local_ms);
                                    if let Some(entity) = self.game.entities.player_mut() {
                                        entity.movement.set_position(sx as f32, sy as f32);
                                        entity.movement.start_move(path, move_start);
                                    }
                                }
                            }
                        }
                    }
                    GameEvent::MapChanged { map_name, x, y } => {
                        let map_name = map_name.strip_suffix(".gat")
                            .unwrap_or(&map_name).to_string();
                        tracing::info!("MapChanged: {map_name} ({x},{y}) current={:?}", self.game.current_map);
                        if self.game.current_map.as_deref() != Some(&map_name) {
                            tracing::info!("Different map, clearing entities");
                            self.load_map(&map_name);
                            self.game.current_map = Some(map_name);
                            // Clear non-player entities only on actual map change;
                            // same-map warps rely on server FOV updates
                            let player_sprite = self.game.entities.player_id()
                                .and_then(|pid| self.game.sprites.remove(&pid));
                            self.game.sprites.clear();
                            self.game.sprite_cache.clear();
                            self.game.entities.clear_non_player();
                            self.game.failed_sprite_loads.clear();
                            if let (Some(pid), Some(sprite)) = (self.game.entities.player_id(), player_sprite) {
                                self.game.sprites.insert(pid, sprite);
                            }
                        }
                        if let Some(entity) = self.game.entities.player_mut() {
                            entity.movement.set_position(x as f32, y as f32);
                        }
                        self.position_camera_at(x as f32, y as f32);

                        if let Some(tx) = &self.network_cmd_tx {
                            let _ = tx.send(NetworkCommand::SendPacket(build_map_loaded_packet(self.config.packetver)));
                        }
                    }
                    GameEvent::EntitySpawned { gid, job, speed, sex, head, weapon, shield,
                                             head_top, head_mid, head_bottom, hair_color,
                                             x, y, direction, body_state } => {
                        if self.game.entities.player_id() == Some(gid) {
                            continue;
                        }
                        let entity_type = entity_type_from_job(job);
                        tracing::info!("EntitySpawned: gid={gid} job={job} type={entity_type:?} pos=({x},{y})");
                        let mut entity = Entity::new(gid, entity_type, job, sex, head, hair_color,
                                                 weapon, head_top, head_mid, head_bottom, shield,
                                                 x, y, direction, speed);
                        if body_state == 2 {
                            entity.state = EntityState::Sitting;
                        }
                        self.game.entities.insert(entity);
                        self.load_entity_sprite(gid, entity_type, job, sex, head, weapon,
                                                shield, head_top, head_mid, head_bottom,
                                                hair_color, direction);
                    }
                    GameEvent::EntityMoved { gid, start_x, start_y, dest_x, dest_y, start_time } => {
                        if let Some(gat) = &self.game.gat {
                            let path = path_search(gat, start_x, start_y, dest_x, dest_y);
                            if !path.is_empty() {
                                let local_ms = self.start_time.elapsed().as_millis() as u32;
                                let move_start = self.game.server_time.server_to_local_secs(start_time, local_ms);
                                if let Some(entity) = self.game.entities.get_mut(gid) {
                                    entity.movement.set_position(start_x as f32, start_y as f32);
                                    entity.movement.start_move(path, move_start);
                                }
                            }
                        }
                    }
                    GameEvent::EntityVanished { gid } => {
                        let r1 = self.game.entities.remove(gid).is_some();
                        let r2 = self.game.sprites.remove(&gid).is_some();
                        tracing::info!("EntityVanished: gid={gid} r1={r1} r2={r2}");
                    }
                    GameEvent::EntityStopMove { gid, x, y } => {
                        if let Some(entity) = self.game.entities.get_mut(gid) {
                            entity.movement.set_position(x as f32, y as f32);
                            if entity.state == EntityState::Moving {
                                entity.state = EntityState::Standing;
                            }
                        }
                    }
                    GameEvent::EntityAction { gid, target_gid, action, damage, left_damage, attack_mt, attacked_mt, .. } => {
                        match action {
                            2 => {
                                if let Some(entity) = self.game.entities.get_mut(gid) {
                                    entity.state = EntityState::Sitting;
                                    entity.state_timer = 0.0;
                                }
                            }
                            3 => {
                                if let Some(entity) = self.game.entities.get_mut(gid) {
                                    entity.state = EntityState::Standing;
                                    entity.state_timer = 0.0;
                                }
                            }
                            0 | 8 => {
                                if let Some(entity) = self.game.entities.get_mut(gid) {
                                    let duration = (attack_mt as f32 / 1000.0).max(0.5);
                                    entity.enter_attack(duration);
                                }
                                if damage > 0 || left_damage > 0 {
                                    if let Some(target) = self.game.entities.get_mut(target_gid) {
                                        let duration = (attacked_mt as f32 / 1000.0).max(0.3);
                                        target.enter_hurt(duration);
                                    }
                                }
                            }
                            1 => {
                                if let Some(entity) = self.game.entities.get_mut(gid) {
                                    entity.enter_pickup(0.5);
                                }
                            }
                            _ => {}
                        }
                    }
                    GameEvent::EntityDirectionChanged { gid, head_dir, dir } => {
                        if let Some(entity) = self.game.entities.get_mut(gid) {
                            entity.head_dir = head_dir;
                            entity.direction = dir;
                        }
                    }
                    GameEvent::ChatMessage { message } => {
                        self.game.chat_window.add_chat(message);
                    }
                    GameEvent::OwnChatMessage { message } => {
                        self.game.chat_window.add_own_chat(message);
                    }
                    GameEvent::ServerTick { server_tick, local_send_time_ms } => {
                        let local_now_ms = self.start_time.elapsed().as_millis() as u32;
                        if self.config.enhanced_lag_compensation {
                            self.game.server_time.on_server_tick_enhanced(server_tick, local_now_ms, local_send_time_ms);
                        } else {
                            self.game.server_time.on_server_tick(server_tick, local_now_ms, local_send_time_ms);
                        }
                    }
                    GameEvent::Disconnected(reason) => {
                        self.game.server_time.reset();
                        if reason == "User exit" {
                            event_loop.exit();
                        } else {
                            self.login_window.set_error(&format!("Disconnected: {reason}"));
                        }
                    }
                    _ => {}
                }
        }
    }

    fn load_player_sprite(&mut self, gid: u32, job: u16, sex: u8, head: u16, weapon: Option<WeaponType>, head_top: u16, head_mid: u16, head_bottom: u16, shield_id: u16) {
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };
        let empty_table = ragnarok_game::accessory_table::AccessoryTable::empty();
        let accessory_table = self.game.accessory_table.as_ref().unwrap_or(&empty_table);
        let data = match sprite_loader::load_player_sprite_data(grf, accessory_table, job, sex, head, weapon, head_top, head_mid, head_bottom, shield_id) {
            Some(d) => d,
            None => return,
        };
        let sprite = Rc::new(build_entity_sprite(
            &renderer.device.device, &renderer.device.queue, &renderer.texture_cache.bind_group_layout,
            data.body, data.head, data.weapon, data.headgear_top, data.headgear_mid, data.headgear_bottom, data.shield, data.shadow,
        ));
        self.game.sprites.insert(gid, sprite);
    }

    fn load_entity_sprite(&mut self, gid: u32, entity_type: EntityType, job: u16,
                           sex: u8, head: u16, weapon: u16, shield: u16,
                           head_top: u16, head_mid: u16, head_bottom: u16,
                           _hair_color: u16, _direction: u8) {
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };

        match entity_type {
            EntityType::Player => {
                let weapon_type = weapon_view_id_to_type(weapon);
                self.load_player_sprite(gid, job, sex, head, weapon_type, head_top, head_mid, head_bottom, shield);
            }
            EntityType::Npc | EntityType::Monster => {
                let name_table = match &self.game.name_table {
                    Some(t) => t,
                    None => { tracing::warn!("No name table for job {job}"); return; },
                };
                let cache_key = match entity_sprite_base_path(name_table, job) {
                    Some(p) => p,
                    None => { tracing::warn!("No sprite path for job {job}"); return; },
                };

                if let Some(cached) = self.game.sprite_cache.get(&cache_key) {
                    self.game.sprites.insert(gid, Rc::clone(cached));
                    return;
                }

                let data = match sprite_loader::load_entity_sprite_data(grf, name_table, job) {
                    Some(d) => d,
                    None => return,
                };
                let sprite = Rc::new(build_entity_sprite(
                    &renderer.device.device, &renderer.device.queue, &renderer.texture_cache.bind_group_layout,
                    data.body, None, None, None, None, None, None, data.shadow,
                ));
                self.game.sprite_cache.insert(cache_key, Rc::clone(&sprite));
                self.game.sprites.insert(gid, sprite);
            }
        }
    }

    fn load_missing_entity_sprites(&mut self) {
        let missing: Vec<_> = self.game.entities.iter()
            .filter(|e| {
                self.game.entities.player_id() != Some(e.id)
                    && !self.game.sprites.contains_key(&e.id)
                    && !self.game.failed_sprite_loads.contains(&e.id)
            })
            .map(|e| (e.id, e.entity_type, e.job, e.sex, e.head, e.head_top, e.head_mid, e.head_bottom, e.shield, e.hair_color, e.direction))
            .collect();
        for (gid, entity_type, job, sex, head, head_top, head_mid, head_bottom, shield, hair_color, direction) in &missing {
            tracing::info!("Retrying sprite load for entity gid={gid} job={job} type={entity_type:?}");
            self.load_entity_sprite(*gid, *entity_type, *job, *sex, *head, 0, *shield, *head_top, *head_mid, *head_bottom, *hair_color, *direction);
            if !self.game.sprites.contains_key(gid) {
                self.game.failed_sprite_loads.insert(*gid);
            }
        }
    }

    fn load_cursor_sprite(&mut self, grf: &GrfArchive) {
        let renderer = match &self.renderer {
            Some(r) => r,
            None => return,
        };

        if let Some(sprite_data) = sprite_loader::load_cursor_sprite(grf) {
            let textures = upload_sprite_textures(
                &sprite_data.images,
                sprite_data.indexed_count,
                &renderer.device.device,
                &renderer.device.queue,
                &renderer.texture_cache.bind_group_layout,
            );
            self.game.cursor_textures = Some(textures);
            self.game.cursor_act = Some(sprite_data.act);

            if let Some(window) = &self.window {
                window.set_cursor_visible(false);
            }
        }
    }

    fn handle_ui_events(&mut self, events: Vec<GameEvent>, event_loop: &ActiveEventLoop) {
        for event in events {
            match event {
                GameEvent::RequestLogin { username, password } => {
                    let addr = format!("{}:{}", self.config.login_ip, self.config.login_port);
                    if let Some(tx) = &self.network_cmd_tx {
                        let _ = tx.send(NetworkCommand::Connect(addr));
                        let packet = build_login_packet(&username, &password, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestSelectServer { index } => {
                    if let Some(server_win) = &self.server_list_window {
                        if let Some(server) = server_win.servers.get(index) {
                            let addr = format!("{}:{}", ip_u32_to_string(server.ip), server.port);
                            if let Some(tx) = &self.network_cmd_tx {
                                let _ = tx.send(NetworkCommand::Disconnect);
                                let _ = tx.send(NetworkCommand::Connect(addr.clone()));
                                if let Some(session) = &mut self.game.login_session {
                                    session.char_server_addr = Some(addr);
                                    let packet = build_char_enter_packet(session);
                                    let _ = tx.send(NetworkCommand::SendPacket(packet));
                                    let _ = tx.send(NetworkCommand::SetKeepalive(KeepaliveMode::CharServer { account_id: session.account_id }));
                                }
                            }
                        }
                    }
                }
                GameEvent::RequestSelectCharacter { slot } => {
                    if let Some(char_win) = &self.char_select_window {
                        self.game.selected_character = char_win.characters.iter()
                            .find(|c| c.slot == slot as i8)
                            .cloned();
                    }
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_select_char_packet(slot, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::BackToServerSelect => {
                    self.game.app_state = AppState::ServerSelect;
                    self.char_select_window = None;
                    self.game.system_menu.open = false;
                    if let Some(tx) = &self.network_cmd_tx {
                        let _ = tx.send(NetworkCommand::Disconnect);
                    }
                }
                GameEvent::BackToLogin => {
                    self.game.app_state = AppState::Login;
                    self.server_list_window = None;
                    self.char_select_window = None;
                    self.game.login_session = None;
                    self.game.system_menu.open = false;
                    if let Some(tx) = &self.network_cmd_tx {
                        let _ = tx.send(NetworkCommand::Disconnect);
                    }
                }
                GameEvent::BackToCharacterSelect => {
                    self.game.system_menu.open = false;
                    self.char_select_window = None;
                    let reconnected = self.reconnect_to_char_server();
                    if !reconnected {
                        self.game.app_state = AppState::Login;
                        self.server_list_window = None;
                        self.game.login_session = None;
                        if let Some(tx) = &self.network_cmd_tx {
                            let _ = tx.send(NetworkCommand::Disconnect);
                        }
                    }
                }
                GameEvent::QuitGame => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let _ = tx.send(NetworkCommand::Disconnect);
                    }
                    event_loop.exit();
                }
                GameEvent::RequestSendChat { message } => {
                    if message.starts_with('/') {
                        self.handle_slash_command(&message);
                    } else {
                        let char_name = self.game.selected_character.as_ref()
                            .map(|c| c.name.as_str()).unwrap_or("Unknown");
                        let full_msg = format!("{char_name} : {message}");
                        if let Some(tx) = &self.network_cmd_tx {
                            let packet = build_chat_packet(&full_msg, self.config.packetver);
                            let _ = tx.send(NetworkCommand::SendPacket(packet));
                        }
                    }
                }
                GameEvent::Disconnected(ref reason) if reason == "User exit" => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let _ = tx.send(NetworkCommand::Disconnect);
                    }
                    event_loop.exit();
                }
                _ => {}
            }
        }
    }

    fn handle_slash_command(&mut self, command: &str) {
        let cmd = command.split_whitespace().next().unwrap_or("");
        match cmd {
            "/sit" => {
                if let Some(entity) = self.game.entities.player() {
                    let action = if entity.state == EntityState::Sitting { 3u8 } else { 2u8 };
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_action_request_packet(0, action, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
            }
            "/doridori" => {
                if let Some(entity) = self.game.entities.player_mut() {
                    entity.head_dir = if entity.head_dir == 0 { 1 } else { 0 };
                }
            }
            _ => {
                self.game.chat_window.add_system(format!("Unknown command: {cmd}"));
            }
        }
    }

    fn build_ui(&mut self, elapsed: f32) -> (Vec<UiDrawCall>, Vec<GameEvent>, bool) {
        match self.game.app_state {
            AppState::Login => {
                if let (Some(ui_ctx), Some(renderer)) = (&self.ui_context, &self.renderer) {
                    let initial_focus = match self.login_window.focus {
                        LoginFocus::Username => Some(WidgetId(0)),
                        LoginFocus::Password => Some(WidgetId(1)),
                    };
                    let mut ui = UiFrame::new(
                        ui_ctx, &renderer.font_atlas, &mut self.ui_state_cache, elapsed,
                        self.login_window.has_grf_textures, initial_focus,
                    );
                    let events = self.login_window.build(&mut ui);
                    let any_hovered = ui.any_hovered;
                    (ui.draw_calls, events, any_hovered)
                } else {
                    (Vec::new(), Vec::new(), false)
                }
            }
            AppState::ServerSelect => {
                if let (Some(ui_ctx), Some(renderer), Some(server_win)) =
                    (&self.ui_context, &self.renderer, &mut self.server_list_window)
                {
                    let mut ui = UiFrame::new(
                        ui_ctx, &renderer.font_atlas, &mut self.ui_state_cache, elapsed,
                        server_win.has_grf_textures, None,
                    );
                    let events = server_win.build(&mut ui);
                    let any_hovered = ui.any_hovered;
                    (ui.draw_calls, events, any_hovered)
                } else {
                    (Vec::new(), Vec::new(), false)
                }
            }
            AppState::CharacterSelect => {
                if let (Some(ui_ctx), Some(renderer), Some(char_win)) =
                    (&self.ui_context, &self.renderer, &mut self.char_select_window)
                {
                    let mut ui = UiFrame::new(
                        ui_ctx, &renderer.font_atlas, &mut self.ui_state_cache, elapsed,
                        char_win.has_grf_textures, None,
                    );
                    let events = char_win.build(&mut ui);
                    let any_hovered = ui.any_hovered;
                    (ui.draw_calls, events, any_hovered)
                } else {
                    (Vec::new(), Vec::new(), false)
                }
            }
            AppState::InGame => {
                if let (Some(ui_ctx), Some(renderer)) = (&self.ui_context, &self.renderer) {
                    let initial_focus = if self.game.chat_window.is_active() {
                        Some(self.game.chat_window.focused_input)
                    } else {
                        None
                    };
                    let mut ui = UiFrame::new(
                        ui_ctx, &renderer.font_atlas, &mut self.ui_state_cache, elapsed,
                        false, initial_focus,
                    );
                    let chat_was_active = self.game.chat_window.is_active();
                    let mut events = self.game.chat_window.build(&mut ui);

                    let allow_escape = !chat_was_active;
                    let menu_events = self.game.system_menu.build(&mut ui, allow_escape);
                    events.extend(menu_events);

                    let any_hovered = ui.any_hovered;
                    (ui.draw_calls, events, any_hovered)
                } else {
                    (Vec::new(), Vec::new(), false)
                }
            }
        }
    }

    fn process_continuous_walk(&mut self, delta: f32) {
        if !self.input.left_mouse_down || self.game.app_state != AppState::InGame {
            return;
        }
        if self.game.chat_window.is_active() {
            return;
        }
        if self.game.chat_window.contains_point(
            self.input.mouse_position.0 as f32,
            self.input.mouse_position.1 as f32,
        ) {
            return;
        }
        self.input.walk_packet_cooldown -= delta;
        if self.input.walk_packet_cooldown > 0.0 && !self.input.walk_server_acked {
            return;
        }
        if self.input.walk_packet_cooldown > 0.0 {
            return;
        }
        self.handle_left_click();
        self.input.walk_packet_cooldown = 0.5;
        self.input.walk_server_acked = false;
    }

    fn update_movement(&mut self, elapsed: f32) {
        for entity in self.game.entities.iter_mut() {
            if entity.movement.is_moving() {
                entity.movement.update(elapsed);
            }
        }
        if let Some(player) = self.game.entities.player() {
            let (px, py) = player.movement.position();
            self.position_camera_at(px, py);
        }
    }

    fn update_entity_state(&mut self, delta: f32) {
        for entity in self.game.entities.iter_mut() {
            entity.update_state(delta);
            if let Some(move_dir) = entity.movement.movement_direction() {
                entity.direction = move_dir;
            }
        }
    }

    fn update_sprite_animation(&mut self, delta: f32) {
        let camera_dir = self.renderer.as_ref().map(|r| r.camera.direction_index());
        let sprites = &self.game.sprites;
        for entity in self.game.entities.iter_mut() {
            if let Some(sprite) = sprites.get(&entity.id) {
                let dir = camera_dir.unwrap_or(0);
                let action = entity.action_index();
                let is_transient = matches!(
                    entity.state,
                    EntityState::Hurt | EntityState::Attacking | EntityState::Dead | EntityState::Pickup
                );
                if is_transient {
                    entity.animation.set_action_one_shot(action);
                } else {
                    entity.animation.set_action(action);
                }
                entity.animation.set_direction(entity.direction);
                let is_composite = entity.entity_type == EntityType::Player;
                let animated = !is_composite || SpriteActionType::from_index(entity.animation.action())
                    .is_none_or(|a| a.is_animated());
                if animated {
                    entity.animation.update(delta, &sprite.body_act, dir);
                }
            }
        }
    }

    fn update_grid_hover(&mut self) -> Option<(i32, i32)> {
        let hovered = if self.game.app_state == AppState::InGame {
            self.hovered_cell()
        } else {
            None
        };

        let hover_corners = hovered.and_then(|(cx, cy)| {
            let coords = self.game.map_coords.as_ref()?;
            let gat = self.game.gat.as_ref()?;
            Some(coords.cell_corners_world(gat, cx, cy))
        });

        if let Some(renderer) = &mut self.renderer {
            if let Some(grid) = &mut renderer.grid_selector {
                if let Some(corners) = hover_corners {
                    grid.update_hover(&renderer.device.queue, corners);
                    grid.set_hover_visible(true);
                } else {
                    grid.set_hover_visible(false);
                }
            }
        }

        hovered
    }

    fn compute_render_list(&self) -> Vec<(u32, [f32; 2], f32, u8, f32)> {
        let mut render_list = Vec::new();
        if let (Some(renderer), Some(coords)) = (&self.renderer, &self.game.map_coords) {
            for entity in self.game.entities.iter() {
                if let Some(params) = input::entity_screen_params(
                    entity.movement.position(),
                    self.game.gat.as_ref(),
                    coords,
                    &renderer.camera,
                    renderer.device.surface_config.width as f32,
                    renderer.device.surface_config.height as f32,
                ) {
                    render_list.push((entity.id, params.0, params.1, params.2, params.3));
                }
            }
        }
        render_list.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        render_list
    }

    fn update_cursor_type(
        &mut self,
        hovered: Option<(i32, i32)>,
        ui_any_hovered: bool,
        render_list: &[(u32, [f32; 2], f32, u8, f32)],
    ) {
        let cursor = if self.game.app_state == AppState::InGame {
            if self.input.right_mouse_down {
                CursorType::Rotate
            } else if ui_any_hovered {
                CursorType::Click
            } else if let Some(entity_cursor) = hovered_entity_cursor_type(
                self.input.mouse_position,
                &self.game.entities,
                render_list,
            ) {
                entity_cursor
            } else if let Some(gat) = &self.game.gat {
                cursor_type_for_cell(gat, hovered)
            } else {
                CursorType::Default
            }
        } else if ui_any_hovered {
            CursorType::Click
        } else {
            CursorType::Default
        };
        self.game.cursor_animation.set_cursor_type(cursor);
    }

    fn build_cursor_sprite_clips(&mut self, dt: f32) -> Vec<ClipData> {
        let cursor_act = match &self.game.cursor_act {
            Some(a) => a,
            None => return Vec::new(),
        };
        
        self.game.cursor_animation.update(dt, cursor_act);
        let action_idx = self.game.cursor_animation.action_index();
        let action_idx = if action_idx < cursor_act.actions.len() { action_idx } else { 0 };
        let action = &cursor_act.actions[action_idx];
        if action.motions.is_empty() {
            return Vec::new();
        }
        let motion_idx = self.game.cursor_animation.motion_index() % action.motions.len();
        let motion = &action.motions[motion_idx];
        let (mx, my) = self.input.mouse_position;
        let cursor_tex = match &self.game.cursor_textures {
            Some(t) => t,
            None => return Vec::new(),
        };
        let mut clips = Vec::new();
        for clip in &motion.clips {
            if let Some((vertices, indices, tex_idx)) = build_clip_quad(clip, cursor_tex, [mx as f32, my as f32], 0.0, [0, 0]) {
                if tex_idx < cursor_tex.bind_groups.len() {
                    clips.push((vertices, indices, tex_idx));
                }
            }
        }
        clips
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("Ragnarok Online")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                self.config.screen_width,
                self.config.screen_height,
            ));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let renderer = block_on(Renderer::new(window.clone()));

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.ui_context = Some(UiContext::new(
            self.config.screen_width as f32,
            self.config.screen_height as f32,
        ));

        // Load GRF
        if let Some(grf_path) = self.config.grf_paths.first() {
            match GrfArchive::open(Path::new(grf_path)) {
                Ok(grf) => {
                    println!("GRF loaded: {} ({} files)", grf_path, grf.file_count());

                    if let Some(renderer) = &mut self.renderer {
                        renderer.try_load_grf_font(&grf);

                        self.login_window.has_grf_textures = renderer.preload_textures(&LoginWindow::grf_texture_paths(), &grf);
                        if self.login_window.has_grf_textures {
                            self.login_window.set_texture_sizes(|name| {
                                renderer.texture_cache.texture_size(name)
                            });
                        }
                    }

                    self.load_cursor_sprite(&grf);
                    self.game.accessory_table = Some(ragnarok_game::accessory_table::AccessoryTable::load_from_grf(&grf));
                    self.game.name_table = Some(NameTable::load(&grf));
                    self.grf = Some(grf);
                }
                Err(e) => {
                    tracing::error!("Failed to open GRF {grf_path}: {e}");
                }
            }
        }

        self.spawn_network();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(ui_ctx) = &mut self.ui_context {
            ui_ctx.handle_event(&event);
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if self.game.app_state == AppState::InGame {
                    match button {
                        MouseButton::Right => {
                            self.input.right_mouse_down = state == ElementState::Pressed;
                            if !self.input.right_mouse_down {
                                self.input.last_mouse_pos = None;
                            }
                        }
                        MouseButton::Left => {
                            let pressed = state == ElementState::Pressed;
                            self.input.left_mouse_down = pressed;
                            if pressed {
                                let mouse_on_chat = self.game.chat_window.contains_point(
                                    self.input.mouse_position.0 as f32,
                                    self.input.mouse_position.1 as f32,
                                );
                                if !mouse_on_chat && !self.game.system_menu.open {
                                    self.handle_left_click();
                                    self.input.walk_packet_cooldown = 0.5;
                                    self.input.walk_server_acked = false;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.mouse_position = (position.x, position.y);
                if self.game.app_state == AppState::InGame && self.input.right_mouse_down {
                    if let Some((lx, ly)) = self.input.last_mouse_pos {
                        let dx = (position.x - lx) as f32;
                        let dy = (position.y - ly) as f32;
                        if let Some(renderer) = &mut self.renderer {
                            input::handle_camera_drag(&mut renderer.camera, dx, dy, self.config.free_camera);
                        }
                    }
                    self.input.last_mouse_pos = Some((position.x, position.y));
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.game.app_state == AppState::InGame {
                    let mouse_on_chat = self.game.chat_window.contains_point(
                        self.input.mouse_position.0 as f32,
                        self.input.mouse_position.1 as f32,
                    );
                    if !mouse_on_chat {
                        let scroll = match delta {
                            MouseScrollDelta::LineDelta(_, y) => y,
                            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 40.0,
                        };
                        if let Some(renderer) = &mut self.renderer {
                            input::handle_camera_zoom(&mut renderer.camera, scroll);
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed
                    && self.game.app_state == AppState::InGame
                    && !self.game.chat_window.is_active()
                    && !self.game.system_menu.open
                {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::F11) => {
                            if let Some(renderer) = &mut self.renderer {
                                if let Some(grid) = &mut renderer.grid_selector {
                                    grid.show_grid = !grid.show_grid;
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::Insert) => {
                            if let Some(entity) = self.game.entities.player() {
                                let action = if entity.state == EntityState::Sitting { 3u8 } else { 2u8 };
                                if let Some(tx) = &self.network_cmd_tx {
                                    let packet = build_action_request_packet(0, action, self.config.packetver);
                                    let _ = tx.send(NetworkCommand::SendPacket(packet));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let elapsed = self.start_time.elapsed().as_secs_f32();

                self.handle_game_events(event_loop);

                let (ui_draw_calls, ui_events, ui_any_hovered) = self.build_ui(elapsed);
                self.handle_ui_events(ui_events, event_loop);

                self.update_movement(elapsed);
                let delta = elapsed - self.last_render_time;
                self.last_render_time = elapsed;
                self.process_continuous_walk(delta);
                self.update_entity_state(delta);
                self.load_missing_entity_sprites();
                self.update_sprite_animation(delta);

                let hovered = self.update_grid_hover();
                let render_list = self.compute_render_list();
                self.update_cursor_type(hovered, ui_any_hovered, &render_list);

                let cursor_clips = self.build_cursor_sprite_clips(delta);

                {
                    let mut sprite_batches: Vec<SpriteBatch> = Vec::new();
                    let mut cursor_batches: Vec<SpriteBatch> = Vec::new();

                    for &(id, center, depth, camera_dir, sprite_scale) in &render_list {
                        if let (Some(sprite), Some(entity)) = (self.game.sprites.get(&id), self.game.entities.get(id)) {
                            let shadow_scale = sprite_scale * shadow_size(entity.job);
                            let mut shadow = sprite.build_shadow_batches(center, depth, shadow_scale);
                            sprite_batches.append(&mut shadow);
                            let mut batches = sprite.build_batches(&entity.animation, Some(camera_dir), entity.head_dir, center, depth, sprite_scale);
                            sprite_batches.append(&mut batches);
                        }
                    }

                    if let Some(cursor_tex) = &self.game.cursor_textures {
                        for (vertices, indices, tex_idx) in cursor_clips {
                            cursor_batches.push(SpriteBatch {
                                vertices,
                                indices,
                                texture: &cursor_tex.bind_groups[tex_idx],
                            });
                        }
                    }

                    if let Some(renderer) = &mut self.renderer {
                        renderer.render(&ui_draw_calls, &sprite_batches, &cursor_batches, elapsed);
                    }
                }

                if let Some(ui_ctx) = &mut self.ui_context {
                    ui_ctx.begin_frame();
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::load_or_default("config.json");
    println!("ragnarok-client (packetver: {})", config.packetver);

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(config);
    event_loop.run_app(&mut app).unwrap();
}
