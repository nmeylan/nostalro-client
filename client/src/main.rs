mod config;
mod game_state;
mod input;

use std::collections::HashMap;
use config::{Config, WindowStateEntry};
use game_state::GameState;
use input::InputState;
use models::enums::EnumWithNumberValue;
use models::enums::status::StatusTypes;
use ragnarok_formats::act::SpriteActionType;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::app_state::AppState;
use ragnarok_game::cursor::{
    CursorType, RenderEntry, RenderEntryKind, cursor_type_for_cell, hovered_entity_cursor_type,
};
use ragnarok_game::entity::{Entity, EntityState, EntityType};
use ragnarok_game::event::GameEvent;
use ragnarok_game::item::Item;
use ragnarok_game::name_table::NameTable;
use ragnarok_game::path::{path_search, try_move_to};
use ragnarok_game::shadow::shadow_size;
use ragnarok_game::sprite_path::{
    WeaponType, entity_sprite_base_path, entity_type_from_job, weapon_view_id_to_type,
};
use ragnarok_game::{map_loader, sprite_loader};
use ragnarok_network::session::Session;
use ragnarok_network::{
    KeepaliveMode, NetworkCommand, build_action_request_packet, build_char_enter_packet,
    build_chat_packet, build_contact_npc_packet, build_drop_item_packet, build_equip_item_packet,
    build_login_packet, build_map_loaded_packet, build_npc_close_packet,
    build_npc_deal_type_packet, build_npc_input_number_packet, build_npc_input_string_packet,
    build_npc_menu_select_packet, build_npc_next_packet, build_pickup_item_packet,
    build_purchase_item_list_packet, build_reqname_packet, build_request_move_packet,
    build_restart_packet, build_select_char_packet, build_sell_item_list_packet,
    build_unequip_item_packet, build_use_item_packet, build_zone_enter_packet, ip_u32_to_string,
    network_loop,
};
use ragnarok_renderer::ui_renderer::UiVertex;
use ragnarok_renderer::{
    GridSelectorRenderer, Renderer, SpriteBatch, SpriteVertex, UiDrawCall, UiTextureRef, block_on,
    build_clip_quad, build_entity_sprite, scale_clip_vertices, upload_sprite_textures,
};
use ragnarok_ui::context::UiContext;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::state::StateCache;
use ragnarok_ui_component::account::char_select_window::CharSelectWindow;
use ragnarok_ui_component::account::login_window::{LoginFocus, LoginWindow};
use ragnarok_ui_component::account::server_list_window::ServerListWindow;
use ragnarok_ui_component::game::drop_quantity_dialog::DropQuantityDialog;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};
use ragnarok_ui_component::Window as UiWindow;

type ClipData = (Vec<SpriteVertex>, Vec<u32>, usize);

fn preload_window<W: UiWindow>(window: &mut W, renderer: &mut Renderer, grf: &GrfArchive) {
    if !window.has_grf_textures() {
        let paths = W::grf_texture_paths();
        let loaded = renderer.preload_textures(&paths, grf);
        window.set_has_grf_textures(loaded);
        if loaded {
            window.set_texture_sizes(&|name| renderer.texture_cache.texture_size(name));
        }
    }
}

struct App {
    config: Config,
    saved_window_positions: HashMap<u32, [f32; 2]>,
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
        let saved_window_positions = config.window_state.iter()
            .map(|(&id, entry)| (id, entry.position))
            .collect();
        Self {
            config,
            saved_window_positions,
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
                    &renderer.device.device,
                    gat,
                    map_data.gnd.width,
                    map_data.gnd.height,
                    map_data.gnd.zoom,
                );
                renderer.grid_selector = Some(grid);
            }
        }
    }

    fn position_camera_at(&mut self, cell_x: f32, cell_y: f32) {
        if let (Some(coords), Some(renderer)) = (&self.game.map_coords, &mut self.renderer) {
            input::position_camera_at(
                &mut renderer.camera,
                self.game.gat.as_ref(),
                coords,
                cell_x,
                cell_y,
            );
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
            renderer.device.surface_config.width as f32 / renderer.dpi_scale,
            renderer.device.surface_config.height as f32 / renderer.dpi_scale,
            coords,
            self.game.gat.as_ref(),
        )
    }

    fn handle_left_click(&mut self) {
        if self.game.npc_dialog.dialog.is_open() || self.game.npc_shop.shop.is_open() {
            return;
        }
        // Click on floor item to pick up
        if let Some(item_id) = self.game.hovered_floor_item_id {
            self.game.pending_pickup_item_id = None;
            if let Some(floor_item) = self.game.floor_items.get(&item_id) {
                let (px, py) = self
                    .game
                    .entities
                    .player()
                    .map(|e| e.movement.cell_position())
                    .unwrap_or((0, 0));
                let dx = (px as i32 - floor_item.x as i32).unsigned_abs();
                let dy = (py as i32 - floor_item.y as i32).unsigned_abs();
                if dx <= 1 && dy <= 1 {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_pickup_item_packet(item_id, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                    if let Some(entity) = self.game.entities.player_mut() {
                        entity.enter_pickup(0.5);
                    }
                } else if let Some(gat) = &self.game.gat {
                    let dest_x = floor_item.x as i32;
                    let dest_y = floor_item.y as i32;
                    if let Some(move_action) = try_move_to(gat, px, py, dest_x, dest_y) {
                        if let Some(tx) = &self.network_cmd_tx {
                            let packet = build_request_move_packet(
                                move_action.dest_x,
                                move_action.dest_y,
                                self.config.packetver,
                            );
                            let _ = tx.send(NetworkCommand::SendPacket(packet));
                        }
                        let elapsed = self.start_time.elapsed().as_secs_f32();
                        if let Some(entity) = self.game.entities.player_mut() {
                            entity.movement.start_move(move_action.path, elapsed);
                        }
                        self.game.pending_pickup_item_id = Some(item_id);
                    }
                }
            }
            return;
        }
        // Click on NPC to talk
        if let Some(entity_id) = self.game.hovered_entity_id {
            if let Some(entity) = self.game.entities.get(entity_id) {
                if entity.entity_type == EntityType::Npc && entity.job != 45 {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_contact_npc_packet(entity_id, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                    return;
                }
            }
        }
        self.game.pending_pickup_item_id = None;
        let (dest_x, dest_y) = match self.hovered_cell() {
            Some(c) => c,
            None => return,
        };
        let gat = match &self.game.gat {
            Some(g) => g,
            None => return,
        };

        let (src_x, src_y) = self
            .game
            .entities
            .player()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));

        let move_action = match try_move_to(gat, src_x, src_y, dest_x, dest_y) {
            Some(a) => a,
            None => return,
        };

        if let Some(tx) = &self.network_cmd_tx {
            let packet = build_request_move_packet(
                move_action.dest_x,
                move_action.dest_y,
                self.config.packetver,
            );
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
        let trace_packets = self.config.trace_packets;
        // Spawn on dedicated thread with single-threaded runtime
        // because network_loop uses non-Send packet types
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create network runtime");
            rt.block_on(network_loop(
                cmd_rx,
                event_tx,
                packetver,
                debug_delay_ms,
                trace_packets,
            ));
        });
    }

    fn handle_game_events(&mut self, event_loop: &ActiveEventLoop) {
        let events: Vec<_> = self
            .game_event_rx
            .as_mut()
            .map(|rx| std::iter::from_fn(|| rx.try_recv().ok()).collect())
            .unwrap_or_default();
        for event in events {
            match event {
                GameEvent::LoginAccepted {
                    account_id,
                    login_id1,
                    login_id2,
                    sex,
                    servers,
                } => {
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
                        preload_window(&mut char_win, renderer, grf);
                    }
                    self.char_select_window = Some(char_win);
                    self.game.app_state = AppState::CharacterSelect;
                }
                GameEvent::ZoneServerConnectInfo {
                    char_id,
                    map_name,
                    ip,
                    port,
                } => {
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
                GameEvent::RestartAck => {
                    self.char_select_window = None;
                    self.game.character.clear();
                    self.game.entities.clear();
                    self.game.sprites.clear();
                    self.game.sprite_cache.clear();
                    self.game.floor_items.clear();
                    self.game.floor_item_sprites.clear();
                    self.game.waiting_item_throw_ack = false;
                    self.game.drop_quantity_dialog = None;
                    self.game.pending_pickup_item_id = None;
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
                GameEvent::MapEntered { x, y, dir, .. } => {
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
                            )
                        })
                        .unwrap_or((0, session_sex, 0, 0, 0, 0, 0, 0, 0));

                    let entity = Entity::new_player(
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
                        self.game.drop_dialog_has_grf_textures =
                            renderer.preload_textures(&DropQuantityDialog::grf_texture_paths(), grf);
                    }

                    if let Some(info) = &self.game.selected_character {
                        self.game.character.init_from_info(info);
                    }

                    self.game.app_state = AppState::InGame;
                    self.game.apply_window_state(&self.config.window_state);

                    if let Some(tx) = &self.network_cmd_tx {
                        let _ = tx.send(NetworkCommand::SendPacket(build_map_loaded_packet(
                            self.config.packetver,
                        )));
                    }
                }
                GameEvent::PlayerMoved {
                    start_x,
                    start_y,
                    dest_x,
                    dest_y,
                    start_time,
                } => {
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
                            // Start from rendered position (like original client)
                            let (sx, sy) = self
                                .game
                                .entities
                                .player()
                                .map(|e| e.movement.cell_position())
                                .unwrap_or((start_x, start_y));
                            let path = path_search(gat, sx, sy, dest_x, dest_y);
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
                GameEvent::MapChanged { map_name, x, y } => {
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
                        // Clear non-player entities only on actual map change;
                        // same-map warps rely on server FOV updates
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
                        if let (Some(pid), Some(sprite)) =
                            (self.game.entities.player_id(), player_sprite)
                        {
                            self.game.sprites.insert(pid, sprite);
                        }
                    }
                    if let Some(entity) = self.game.entities.player_mut() {
                        entity.movement.set_position(x as f32, y as f32);
                    }
                    self.position_camera_at(x as f32, y as f32);

                    if let Some(tx) = &self.network_cmd_tx {
                        let _ = tx.send(NetworkCommand::SendPacket(build_map_loaded_packet(
                            self.config.packetver,
                        )));
                    }
                }
                GameEvent::EntitySpawned {
                    gid,
                    job,
                    speed,
                    sex,
                    head,
                    weapon,
                    shield,
                    head_top,
                    head_mid,
                    head_bottom,
                    hair_color,
                    x,
                    y,
                    direction,
                    body_state,
                } => {
                    if self.game.entities.player_id() == Some(gid) {
                        continue;
                    }
                    let entity_type = entity_type_from_job(job);
                    tracing::info!(
                        "EntitySpawned: gid={gid} job={job} type={entity_type:?} pos=({x},{y})"
                    );
                    let mut entity = Entity::new(
                        gid,
                        entity_type,
                        job,
                        sex,
                        head,
                        hair_color,
                        weapon,
                        head_top,
                        head_mid,
                        head_bottom,
                        shield,
                        x,
                        y,
                        direction,
                        speed,
                    );
                    if body_state == 2 {
                        entity.state = EntityState::Sitting;
                    }
                    self.game.entities.insert(entity);
                    self.load_entity_sprite(
                        gid,
                        entity_type,
                        job,
                        sex,
                        head,
                        weapon,
                        shield,
                        head_top,
                        head_mid,
                        head_bottom,
                        hair_color,
                        direction,
                    );
                }
                GameEvent::EntityMoved {
                    gid,
                    start_x,
                    start_y,
                    dest_x,
                    dest_y,
                    start_time,
                } => {
                    if let Some(gat) = &self.game.gat {
                        let path = path_search(gat, start_x, start_y, dest_x, dest_y);
                        if !path.is_empty() {
                            let local_ms = self.start_time.elapsed().as_millis() as u32;
                            let move_start = self
                                .game
                                .server_time
                                .server_to_local_secs(start_time, local_ms);
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
                GameEvent::EntityAction {
                    gid,
                    target_gid,
                    action,
                    damage,
                    left_damage,
                    attack_mt,
                    attacked_mt,
                    ..
                } => match action {
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
                },
                GameEvent::EntityDirectionChanged { gid, head_dir, dir } => {
                    if let Some(entity) = self.game.entities.get_mut(gid) {
                        entity.head_dir = head_dir;
                        entity.direction = dir;
                    }
                }
                GameEvent::EntityNameReceived { gid, name } => {
                    if let Some(entity) = self.game.entities.get_mut(gid) {
                        entity.name = Some(name);
                    }
                }
                GameEvent::EntityHpChanged { gid, hp, max_hp } => {
                    if self.game.entities.is_player(gid) {
                        self.game.character.hp = hp;
                        self.game.character.max_hp = max_hp;
                    } else if let Some(entity) = self.game.entities.get_mut(gid) {
                        entity.hp = Some(hp);
                        entity.max_hp = Some(max_hp);
                    }
                }
                GameEvent::NpcDialogText { npc_id, text } => {
                    self.game.npc_dialog.dialog.open_text(npc_id, &text);
                }
                GameEvent::NpcDialogNext { npc_id } => {
                    self.game.npc_dialog.dialog.wait_for_next(npc_id);
                }
                GameEvent::NpcDialogClose { npc_id } => {
                    self.game.npc_dialog.dialog.wait_for_close(npc_id);
                }
                GameEvent::NpcDialogMenu { npc_id, items } => {
                    self.game.npc_dialog.dialog.show_menu(npc_id, items);
                }
                GameEvent::NpcInputNumber { npc_id } => {
                    self.game.npc_dialog.dialog.wait_for_number_input(npc_id);
                }
                GameEvent::NpcInputString { npc_id } => {
                    self.game.npc_dialog.dialog.wait_for_string_input(npc_id);
                }
                GameEvent::NpcDealTypeSelect { npc_id } => {
                    self.game.npc_dialog.dialog.show_deal_type(npc_id);
                }
                GameEvent::NpcShopBuyList { npc_id, items } => {
                    let buy_items: Vec<_> = items
                        .into_iter()
                        .map(|(item_id, price, discount_price, item_type)| {
                            let name = self
                                .game
                                .data_table.item_name
                                .as_ref()
                                .map(|t| t.get_name_or_id(item_id))
                                .unwrap_or_else(|| format!("Item #{item_id}"));
                            let resource_name =
                                self.game.data_table.item_resource.as_ref().and_then(|t| {
                                    t.get_resource_name(item_id).map(|s| s.to_string())
                                });
                            ragnarok_game::npc_shop::ShopBuyItem {
                                item: Item {
                                    index: 0,
                                    item_id,
                                    item_type,
                                    count: 1,
                                    is_identified: true,
                                    is_damaged: false,
                                    refining_level: 0,
                                    slot: [0; 4],
                                    location: 0,
                                    wear_state: 0,
                                    name,
                                    resource_name,
                                },
                                price,
                                discount_price,
                            }
                        })
                        .collect();
                    let shop_npc_id = if npc_id != 0 {
                        npc_id
                    } else {
                        self.game.npc_dialog.dialog.npc_id
                    };
                    self.game.npc_shop.shop.open_buy(shop_npc_id, buy_items);
                    self.game.npc_dialog.dialog.close();
                    let icon_paths: Vec<String> = self.game.npc_shop.shop.buy_items.iter()
                        .filter_map(|i| i.item.icon_path())
                        .collect();
                    self.preload_item_icons(icon_paths);
                }
                GameEvent::NpcShopSellList { npc_id, items } => {
                    let sell_items = items
                        .into_iter()
                        .filter_map(|(index, price, overcharge_price)| {
                            let inv_item = self
                                .game
                                .character
                                .inventory
                                .get_item(index as u16)?;
                            Some(ragnarok_game::npc_shop::ShopSellItem {
                                item: inv_item.clone(),
                                price,
                                overcharge_price,
                            })
                        })
                        .collect();
                    let shop_npc_id = if npc_id != 0 {
                        npc_id
                    } else {
                        self.game.npc_dialog.dialog.npc_id
                    };
                    self.game.npc_shop.shop.open_sell(shop_npc_id, sell_items);
                    self.game.npc_dialog.dialog.close();
                    let icon_paths: Vec<String> = self.game.npc_shop.shop.sell_items.iter()
                        .filter_map(|i| i.item.icon_path())
                        .collect();
                    self.preload_item_icons(icon_paths);
                }
                GameEvent::NpcShopBuyResult { result } => {
                    self.game.npc_shop.shop.close();
                    match result {
                        0 => {
                            self.game.chat_window.add_chat("Purchase completed.".into());
                        }
                        1 => {
                            self.game.chat_window.add_chat("Not enough zeny.".into());
                        }
                        2 => {
                            self.game.chat_window.add_chat("You are overweight.".into());
                        }
                        _ => {
                            self.game.chat_window.add_chat("Purchase failed.".into());
                        }
                    }
                }
                GameEvent::NpcShopSellResult { result } => {
                    self.game.npc_shop.close();
                    match result {
                        0 => {
                            self.game.chat_window.add_chat("Sale completed.".into());
                        }
                        _ => {
                            self.game.chat_window.add_chat("Sell failed.".into());
                        }
                    }
                }
                GameEvent::InventoryNormalItems { items } => {
                    for info in items {
                        let name = self
                            .game
                            .data_table.item_name
                            .as_ref()
                            .map(|t| t.get_name_or_id_for(info.item_id, info.is_identified))
                            .unwrap_or_else(|| format!("Item #{}", info.item_id));
                        let resource_name = self.game.data_table.item_resource.as_ref().and_then(|t| {
                            t.get_resource_name_for(info.item_id, info.is_identified)
                                .map(|s| s.to_string())
                        });
                        self.game.character.inventory.add_item(Item {
                            index: info.index as u16,
                            item_id: info.item_id,
                            item_type: info.item_type,
                            count: info.count,
                            is_identified: info.is_identified,
                            is_damaged: false,
                            refining_level: 0,
                            slot: [0; 4],
                            location: info.wear_state,
                            wear_state: 0,
                            name,
                            resource_name,
                        });
                    }
                    let icon_paths: Vec<String> = self.game.character.inventory.all_items().iter()
                        .filter_map(|item| item.icon_path())
                        .collect();
                    self.preload_item_icons(icon_paths);
                }
                GameEvent::InventoryEquipmentItems { items } => {
                    for info in items {
                        let name = self
                            .game
                            .data_table.item_name
                            .as_ref()
                            .map(|t| t.get_name_or_id_for(info.item_id, info.is_identified))
                            .unwrap_or_else(|| format!("Item #{}", info.item_id));
                        let resource_name = self.game.data_table.item_resource.as_ref().and_then(|t| {
                            t.get_resource_name_for(info.item_id, info.is_identified)
                                .map(|s| s.to_string())
                        });
                        tracing::debug!(
                            "Equipment item: idx={} id={} type={} name={} loc={} wear={}",
                            info.index,
                            info.item_id,
                            info.item_type,
                            name,
                            info.location,
                            info.wear_state,
                        );
                        self.game.character.inventory.add_item(Item {
                            index: info.index as u16,
                            item_id: info.item_id,
                            item_type: info.item_type,
                            count: 1,
                            is_identified: info.is_identified,
                            is_damaged: info.is_damaged,
                            refining_level: info.refining_level,
                            slot: info.slot,
                            location: info.location,
                            wear_state: info.wear_state,
                            name,
                            resource_name,
                        });
                    }
                    let icon_paths: Vec<String> = self.game.character.inventory.all_items().iter()
                        .filter_map(|item| item.icon_path())
                        .collect();
                    self.preload_item_icons(icon_paths);
                }
                GameEvent::InventoryItemPickup {
                    index,
                    item_id,
                    count,
                    item_type,
                    is_identified,
                    is_damaged,
                    refining_level,
                    slot,
                    location,
                    result,
                } => {
                    if result == 0 {
                        let name = self
                            .game
                            .data_table.item_name
                            .as_ref()
                            .map(|t| t.get_name_or_id_for(item_id, is_identified))
                            .unwrap_or_else(|| format!("Item #{item_id}"));
                        let resource_name = self.game.data_table.item_resource.as_ref().and_then(|t| {
                            t.get_resource_name_for(item_id, is_identified)
                                .map(|s| s.to_string())
                        });
                        self.game.character.inventory.add_item(Item {
                            index,
                            item_id,
                            item_type,
                            count: count as i16,
                            is_identified,
                            is_damaged,
                            refining_level,
                            slot,
                            location,
                            wear_state: 0,
                            name: name.clone(),
                            resource_name,
                        });
                        self.game
                            .chat_window
                            .add_system(format!("Picked up {name} x{count}"));
                        let icon_path = self
                            .game
                            .character
                            .inventory
                            .get_item(index)
                            .and_then(|item| item.icon_path());
                        if let Some(path) = &icon_path {
                            self.preload_item_icons(vec![path.clone()]);
                        }
                        self.game
                            .item_pickup_notification
                            .show(name, count, icon_path);
                    }
                }
                GameEvent::InventoryUseItemResult {
                    index,
                    count,
                    success,
                } => {
                    if success {
                        self.game
                            .character
                            .inventory
                            .update_item_count(index, count);
                    }
                }
                GameEvent::InventoryEquipResult {
                    index,
                    wear_location,
                    view_id,
                    success,
                } => {
                    tracing::debug!(
                        "EquipResult: idx={} wear_loc={} view_id={} success={}",
                        index,
                        wear_location,
                        view_id,
                        success,
                    );
                    if success {
                        self.game
                            .character
                            .inventory
                            .update_wear_state(index, wear_location);
                        if view_id != 0 {
                            if let Some(sprite_type) =
                                Entity::wear_location_to_sprite_type(wear_location)
                            {
                                if let Some(player_id) = self.game.entities.player_id() {
                                    if let Some(entity) = self.game.entities.get_mut(player_id) {
                                        entity.apply_sprite_change(sprite_type, view_id);
                                    }
                                    self.reload_player_sprite(player_id);
                                }
                            }
                        }
                    }
                }
                GameEvent::InventoryUnequipResult {
                    index,
                    success,
                    wear_location,
                } => {
                    tracing::debug!(
                        "UnequipResult: idx={} wear_loc={} success={}",
                        index,
                        wear_location,
                        success,
                    );
                    if success {
                        self.game.character.inventory.clear_wear_state(index);
                        if let Some(sprite_type) =
                            Entity::wear_location_to_sprite_type(wear_location)
                        {
                            if let Some(player_id) = self.game.entities.player_id() {
                                if let Some(entity) = self.game.entities.get_mut(player_id) {
                                    entity.apply_sprite_change(sprite_type, 0);
                                }
                                self.reload_player_sprite(player_id);
                            }
                        }
                    }
                }
                GameEvent::InventoryItemRemoved { index, count } => {
                    self.game
                        .character
                        .inventory
                        .subtract_item_count(index, count);
                    self.game.waiting_item_throw_ack = false;
                }
                GameEvent::FloorItemAppeared {
                    id,
                    item_id,
                    is_identified,
                    x,
                    y,
                    sub_x,
                    sub_y,
                    count,
                    is_falling,
                } => {
                    self.handle_floor_item_appeared(
                        id,
                        item_id,
                        is_identified,
                        x,
                        y,
                        sub_x,
                        sub_y,
                        count,
                        is_falling,
                    );
                }
                GameEvent::FloorItemDisappeared { id } => {
                    self.game.floor_items.remove(&id);
                    self.game.floor_item_sprites.remove(&id);
                }
                GameEvent::ChatMessage { gid, message } => {
                    if let Some(bubble_text) = message.split(" : ").nth(1) {
                        if let Some(entity) = self.game.entities.get_mut(gid) {
                            entity.chat_bubble = Some(ragnarok_game::entity::ChatBubbleState::new(
                                bubble_text.to_string(),
                            ));
                        }
                    }
                    self.game.chat_window.add_chat(message);
                }
                GameEvent::OwnChatMessage { message } => {
                    if let Some(bubble_text) = message.split(" : ").nth(1) {
                        if let Some(player_id) = self.game.entities.player_id() {
                            if let Some(entity) = self.game.entities.get_mut(player_id) {
                                entity.chat_bubble =
                                    Some(ragnarok_game::entity::ChatBubbleState::new(
                                        bubble_text.to_string(),
                                    ));
                            }
                        }
                    }
                    self.game.chat_window.add_own_chat(message);
                }
                GameEvent::ServerTick {
                    server_tick,
                    local_send_time_ms,
                } => {
                    let local_now_ms = self.start_time.elapsed().as_millis() as u32;
                    if self.config.enhanced_lag_compensation {
                        self.game.server_time.on_server_tick_enhanced(
                            server_tick,
                            local_now_ms,
                            local_send_time_ms,
                        );
                    } else {
                        self.game.server_time.on_server_tick(
                            server_tick,
                            local_now_ms,
                            local_send_time_ms,
                        );
                    }
                }
                GameEvent::ParameterChanged { var_id, value } => {
                    if let Ok(status) = StatusTypes::try_from_value(var_id as usize) {
                        match status {
                            StatusTypes::Speed => {
                                if let Some(entity) = self.game.entities.player_mut() {
                                    entity.speed = value as u16;
                                    entity.movement.set_speed(value as u16);
                                }
                            }
                            StatusTypes::Hp => {
                                self.game.character.hp = value as u32;
                            }
                            StatusTypes::Maxhp => {
                                self.game.character.max_hp = value as u32;
                            }
                            StatusTypes::Sp => {
                                self.game.character.sp = value as u16;
                            }
                            StatusTypes::Maxsp => {
                                self.game.character.max_sp = value as u16;
                            }
                            StatusTypes::Baselevel => {
                                self.game.character.base_level = value as u16;
                            }
                            StatusTypes::Str => {
                                self.game.character.str = value as u8;
                            }
                            StatusTypes::Agi => {
                                self.game.character.agi = value as u8;
                            }
                            StatusTypes::Vit => {
                                self.game.character.vit = value as u8;
                            }
                            StatusTypes::Int => {
                                self.game.character.int = value as u8;
                            }
                            StatusTypes::Dex => {
                                self.game.character.dex = value as u8;
                            }
                            StatusTypes::Luk => {
                                self.game.character.luk = value as u8;
                            }
                            StatusTypes::Joblevel => {
                                self.game.character.job_level = value as u32;
                            }
                            StatusTypes::Weight => {
                                self.game.character.inventory.weight = value;
                            }
                            StatusTypes::Maxweight => {
                                self.game.character.inventory.max_weight = value;
                            }
                            StatusTypes::Zeny => {
                                self.game.character.inventory.zeny = value;
                            }
                            _ => {}
                        }
                    }
                }
                GameEvent::StatusChanged {
                    status_type, base, ..
                } => {
                    if let Ok(status) = StatusTypes::try_from_value(status_type as usize) {
                        match status {
                            StatusTypes::Str => self.game.character.str = base as u8,
                            StatusTypes::Agi => self.game.character.agi = base as u8,
                            StatusTypes::Vit => self.game.character.vit = base as u8,
                            StatusTypes::Int => self.game.character.int = base as u8,
                            StatusTypes::Dex => self.game.character.dex = base as u8,
                            StatusTypes::Luk => self.game.character.luk = base as u8,
                            _ => {}
                        }
                    }
                }
                GameEvent::AttackRangeChanged { range } => {
                    self.game.attack_range = range;
                }
                GameEvent::EntitySpriteChanged {
                    gid,
                    sprite_type,
                    value,
                    ..
                } => {
                    if let Some(entity) = self.game.entities.get_mut(gid) {
                        entity.apply_sprite_change(sprite_type, value);
                        let (
                            job,
                            sex,
                            head,
                            weapon,
                            shield,
                            head_top,
                            head_mid,
                            head_bottom,
                            hair_color,
                            cloth_color,
                        ) = {
                            (
                                entity.job,
                                entity.sex,
                                entity.head,
                                entity.weapon.map(|w| w as u16).unwrap_or(0),
                                entity.shield,
                                entity.head_top,
                                entity.head_mid,
                                entity.head_bottom,
                                entity.hair_color,
                                entity.cloth_color,
                            )
                        };
                        let entity_type = entity.entity_type;
                        let is_player = self.game.entities.player_id() == Some(gid);
                        if is_player {
                            let weapon_type = weapon_view_id_to_type(weapon);
                            self.load_player_sprite(
                                gid,
                                job,
                                sex,
                                head,
                                hair_color,
                                cloth_color,
                                weapon_type,
                                head_top,
                                head_mid,
                                head_bottom,
                                shield,
                            );
                        } else {
                            self.load_entity_sprite(
                                gid,
                                entity_type,
                                job,
                                sex,
                                head,
                                weapon,
                                shield,
                                head_top,
                                head_mid,
                                head_bottom,
                                hair_color,
                                0,
                            );
                        }
                    }
                }
                GameEvent::SkillCasting { gid, delay_ms, .. } => {
                    if let Some(entity) = self.game.entities.get_mut(gid) {
                        let duration = (delay_ms as f32 / 1000.0).max(0.3);
                        entity.enter_attack(duration);
                    }
                }
                GameEvent::EntityEmotion { gid, emotion_type } => {
                    if let Some(entity) = self.game.entities.get_mut(gid) {
                        entity.emotion =
                            Some(ragnarok_game::entity::EmotionState::new(emotion_type));
                    }
                }
                GameEvent::Disconnected(reason) => {
                    self.game.server_time.reset();
                    self.game.character.inventory.clear();
                    self.game.floor_items.clear();
                    self.game.floor_item_sprites.clear();
                    self.game.waiting_item_throw_ack = false;
                    self.game.drop_quantity_dialog = None;
                    self.game.pending_pickup_item_id = None;
                    if reason == "User exit" {
                        event_loop.exit();
                    } else {
                        self.login_window
                            .set_error(&format!("Disconnected: {reason}"));
                    }
                }
                _ => {}
            }
        }
    }

    fn reload_player_sprite(&mut self, gid: u32) {
        let entity = match self.game.entities.get(gid) {
            Some(e) => e,
            None => return,
        };
        let job = entity.job;
        let sex = entity.sex;
        let head = entity.head;
        let weapon = entity.weapon.map(|w| w as u16).unwrap_or(0);
        let shield = entity.shield;
        let head_top = entity.head_top;
        let head_mid = entity.head_mid;
        let head_bottom = entity.head_bottom;
        let hair_color = entity.hair_color;
        let cloth_color = entity.cloth_color;
        let weapon_type = weapon_view_id_to_type(weapon);
        self.load_player_sprite(
            gid,
            job,
            sex,
            head,
            hair_color,
            cloth_color,
            weapon_type,
            head_top,
            head_mid,
            head_bottom,
            shield,
        );
    }

    fn load_player_sprite(
        &mut self,
        gid: u32,
        job: u16,
        sex: u8,
        head: u16,
        hair_color: u16,
        cloth_color: u16,
        weapon: Option<WeaponType>,
        head_top: u16,
        head_mid: u16,
        head_bottom: u16,
        shield_id: u16,
    ) {
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };
        let empty_table = ragnarok_game::accessory_table::AccessoryTable::empty();
        let accessory_table = self.game.data_table.accessory.as_ref().unwrap_or(&empty_table);
        let data = match sprite_loader::load_player_sprite_data(
            grf,
            accessory_table,
            job,
            sex,
            head,
            hair_color,
            cloth_color,
            weapon,
            head_top,
            head_mid,
            head_bottom,
            shield_id,
        ) {
            Some(d) => d,
            None => return,
        };
        let sprite = Rc::new(build_entity_sprite(
            &renderer.device.device,
            &renderer.device.queue,
            &renderer.texture_cache.bind_group_layout,
            data.body,
            data.head,
            data.weapon,
            data.headgear_top,
            data.headgear_mid,
            data.headgear_bottom,
            data.shield,
            data.shadow,
        ));
        self.game.sprites.insert(gid, sprite);
    }

    fn load_entity_sprite(
        &mut self,
        gid: u32,
        entity_type: EntityType,
        job: u16,
        sex: u8,
        head: u16,
        weapon: u16,
        shield: u16,
        head_top: u16,
        head_mid: u16,
        head_bottom: u16,
        hair_color: u16,
        _direction: u8,
    ) {
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };

        match entity_type {
            EntityType::Player => {
                let weapon_type = weapon_view_id_to_type(weapon);
                self.load_player_sprite(
                    gid,
                    job,
                    sex,
                    head,
                    hair_color,
                    0,
                    weapon_type,
                    head_top,
                    head_mid,
                    head_bottom,
                    shield,
                );
            }
            EntityType::Npc | EntityType::Monster => {
                let name_table = match &self.game.data_table.name {
                    Some(t) => t,
                    None => {
                        tracing::warn!("No name table for job {job}");
                        return;
                    }
                };
                let cache_key = match entity_sprite_base_path(name_table, job) {
                    Some(p) => p,
                    None => {
                        tracing::warn!("No sprite path for job {job}");
                        return;
                    }
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
                    &renderer.device.device,
                    &renderer.device.queue,
                    &renderer.texture_cache.bind_group_layout,
                    data.body,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    data.shadow,
                ));
                self.game.sprite_cache.insert(cache_key, Rc::clone(&sprite));
                self.game.sprites.insert(gid, sprite);
            }
        }
    }

    fn load_missing_entity_sprites(&mut self) {
        let missing: Vec<_> = self
            .game
            .entities
            .iter()
            .filter(|e| {
                self.game.entities.player_id() != Some(e.id)
                    && !self.game.sprites.contains_key(&e.id)
                    && !self.game.failed_sprite_loads.contains(&e.id)
            })
            .map(|e| {
                (
                    e.id,
                    e.entity_type,
                    e.job,
                    e.sex,
                    e.head,
                    e.head_top,
                    e.head_mid,
                    e.head_bottom,
                    e.shield,
                    e.hair_color,
                    e.direction,
                )
            })
            .collect();
        for (
            gid,
            entity_type,
            job,
            sex,
            head,
            head_top,
            head_mid,
            head_bottom,
            shield,
            hair_color,
            direction,
        ) in &missing
        {
            tracing::info!(
                "Retrying sprite load for entity gid={gid} job={job} type={entity_type:?}"
            );
            self.load_entity_sprite(
                *gid,
                *entity_type,
                *job,
                *sex,
                *head,
                0,
                *shield,
                *head_top,
                *head_mid,
                *head_bottom,
                *hair_color,
                *direction,
            );
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

    fn load_emotion_sprite(&mut self, grf: &GrfArchive) {
        let renderer = match &self.renderer {
            Some(r) => r,
            None => return,
        };
        if let Some(sprite_data) = sprite_loader::load_emotion_sprite(grf) {
            let textures = upload_sprite_textures(
                &sprite_data.images,
                sprite_data.indexed_count,
                &renderer.device.device,
                &renderer.device.queue,
                &renderer.texture_cache.bind_group_layout,
            );
            self.game.emotion_textures = Some(textures);
            self.game.emotion_act = Some(sprite_data.act);
        }
    }

    fn preload_item_icons(&mut self, icon_paths: Vec<String>) {
        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            let icon_refs: Vec<&str> = icon_paths.iter().map(|s| s.as_str()).collect();
            renderer.preload_textures(&icon_refs, grf);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_floor_item_appeared(
        &mut self,
        id: u32,
        item_id: u16,
        is_identified: bool,
        x: i16,
        y: i16,
        sub_x: u8,
        sub_y: u8,
        count: i16,
        is_falling: bool,
    ) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let name = self
            .game
            .data_table.item_name
            .as_ref()
            .and_then(|t| t.get_name(item_id))
            .unwrap_or("Unknown Item")
            .to_string();
        let resource_name = self
            .game
            .data_table.item_resource
            .as_ref()
            .and_then(|t| t.get_resource_name_for(item_id, is_identified))
            .map(|s| s.to_string());

        // Compute initial_y for fall animation
        let cell_x = x as f32 + sub_x as f32 / 16.0;
        let cell_y = y as f32 + sub_y as f32 / 16.0;
        let ground_y = self
            .game
            .gat
            .as_ref()
            .map(|gat| gat.get_height(cell_x + 0.5, cell_y + 0.5))
            .unwrap_or(0.0);

        let floor_item = ragnarok_game::floor_item::FloorItem {
            id,
            item_id,
            is_identified,
            x,
            y,
            sub_x,
            sub_y,
            count,
            name,
            resource_name: resource_name.clone(),
            drop_time: elapsed,
            is_falling,
            initial_y: ground_y,
        };
        self.game.floor_items.insert(id, floor_item);

        // Load item SPR/ACT sprite
        if let Some(res_name) = &resource_name {
            if let (Some(grf), Some(renderer)) = (&self.grf, &self.renderer) {
                let base = format!("data/sprite/아이템/{res_name}");
                let spr_path = format!("{base}.spr");
                let act_path = format!("{base}.act");
                if let Some(data) = sprite_loader::load_sprite_data(grf, &spr_path, &act_path) {
                    let tex = upload_sprite_textures(
                        &data.images,
                        data.indexed_count,
                        &renderer.device.device,
                        &renderer.device.queue,
                        &renderer.texture_cache.bind_group_layout,
                    );
                    self.game
                        .floor_item_sprites
                        .insert(id, (Rc::new(tex), data.act));
                }
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
                        let packet =
                            build_login_packet(&username, &password, self.config.packetver);
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
                                    let _ = tx.send(NetworkCommand::SetKeepalive(
                                        KeepaliveMode::CharServer {
                                            account_id: session.account_id,
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
                GameEvent::RequestSelectCharacter { slot } => {
                    if let Some(char_win) = &self.char_select_window {
                        self.game.selected_character = char_win
                            .characters
                            .iter()
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
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_restart_packet(self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::QuitGame => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let _ = tx.send(NetworkCommand::Disconnect);
                    }
                    event_loop.exit();
                }
                GameEvent::RequestNpcContact { npc_id } => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_contact_npc_packet(npc_id, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestNpcNext { npc_id } => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_npc_next_packet(npc_id, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestNpcClose { npc_id } => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_npc_close_packet(npc_id, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestNpcMenuSelect { npc_id, choice } => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet =
                            build_npc_menu_select_packet(npc_id, choice, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestNpcInputNumber { npc_id, value } => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet =
                            build_npc_input_number_packet(npc_id, value, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestNpcInputString { npc_id, text } => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet =
                            build_npc_input_string_packet(npc_id, &text, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestNpcDealType { npc_id, deal_type } => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet =
                            build_npc_deal_type_packet(npc_id, deal_type, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestNpcShopBuy { items } => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_purchase_item_list_packet(&items, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestNpcShopSell { items } => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_sell_item_list_packet(&items, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestNpcShopClose => {
                    if let Some(tx) = &self.network_cmd_tx {
                        match self.game.npc_shop.shop.mode {
                            Some(ragnarok_game::npc_shop::NpcShopMode::Buy) => {
                                let packet =
                                    build_purchase_item_list_packet(&[], self.config.packetver);
                                let _ = tx.send(NetworkCommand::SendPacket(packet));
                            }
                            Some(ragnarok_game::npc_shop::NpcShopMode::Sell) => {
                                let packet =
                                    build_sell_item_list_packet(&[], self.config.packetver);
                                let _ = tx.send(NetworkCommand::SendPacket(packet));
                            }
                            None => {}
                        }
                    }
                    self.game.npc_shop.close();
                }
                GameEvent::ShowItemInfo { index } => {
                    if let Some(item) = self.game.character.inventory.get_item(index) {
                        self.game.item_info_window.show(item, &self.game.data_table);
                        let tex_paths = self.game.item_info_window.pending_texture_paths();
                        self.preload_item_icons(tex_paths);
                    }
                }
                GameEvent::ShowCardInfo { item_id } => {
                    self.game.item_info_window.show_card(item_id, &self.game.data_table);
                    let tex_paths = self.game.item_info_window.pending_card_texture_paths();
                    self.preload_item_icons(tex_paths);
                }
                GameEvent::RequestUseItem { index } => {
                    let account_id = self
                        .game
                        .login_session
                        .as_ref()
                        .map(|s| s.account_id)
                        .unwrap_or(0);
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet =
                            build_use_item_packet(index, account_id, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestEquipItem { index, location } => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet =
                            build_equip_item_packet(index, location, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestUnequipItem { index } => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_unequip_item_packet(index, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestDropItem { index, count } => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_drop_item_packet(index, count, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::RequestPickupItem { id } => {
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_pickup_item_packet(id, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                    if let Some(entity) = self.game.entities.player_mut() {
                        entity.enter_pickup(0.5);
                    }
                }
                GameEvent::RequestSendChat { message } => {
                    if message.starts_with('/') {
                        self.handle_slash_command(&message);
                    } else {
                        let char_name = self
                            .game
                            .selected_character
                            .as_ref()
                            .map(|c| c.name.as_str())
                            .unwrap_or("Unknown");
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

    fn reconnect_to_char_server(&mut self) -> bool {
        let Some(tx) = &self.network_cmd_tx else {
            return false;
        };
        let Some(session) = &self.game.login_session else {
            return false;
        };
        let Some(addr) = &session.char_server_addr else {
            return false;
        };
        let _ = tx.send(NetworkCommand::Disconnect);
        let _ = tx.send(NetworkCommand::Connect(addr.clone()));
        let packet = build_char_enter_packet(session);
        let _ = tx.send(NetworkCommand::SendPacket(packet));
        let _ = tx.send(NetworkCommand::SetKeepalive(KeepaliveMode::CharServer {
            account_id: session.account_id,
        }));
        // Switch to CharacterSelect immediately; char_select_window is None
        // until CharacterListReceived arrives, so the screen will be blank briefly
        self.game.app_state = AppState::CharacterSelect;
        true
    }

    fn handle_slash_command(&mut self, command: &str) {
        let cmd = command.split_whitespace().next().unwrap_or("");
        match cmd {
            "/sit" => {
                if let Some(entity) = self.game.entities.player() {
                    let action = if entity.state == EntityState::Sitting {
                        3u8
                    } else {
                        2u8
                    };
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
                self.game
                    .chat_window
                    .add_system(format!("Unknown command: {cmd}"));
            }
        }
    }

    fn build_ui(&mut self, elapsed: f32) -> (Vec<UiDrawCall>, Vec<GameEvent>, bool, bool) {
        match self.game.app_state {
            AppState::Login => {
                if let (Some(ui_ctx), Some(renderer)) = (&self.ui_context, &self.renderer) {
                    let initial_focus = match self.login_window.focus {
                        LoginFocus::Username => Some(WidgetId(0)),
                        LoginFocus::Password => Some(WidgetId(1)),
                    };
                    let mut ui = UiFrame::new(
                        ui_ctx,
                        &renderer.font_atlas,
                        &mut self.ui_state_cache,
                        elapsed,
                        self.login_window.has_grf_textures,
                        initial_focus,
                        &self.saved_window_positions,
                    );
                    let events = self.login_window.build(&mut ui);
                    let any_hovered = ui.any_hovered;
                    let any_interactive = ui.any_interactive_hovered;
                    (ui.draw_calls, events, any_hovered, any_interactive)
                } else {
                    (Vec::new(), Vec::new(), false, false)
                }
            }
            AppState::ServerSelect => {
                if let (Some(ui_ctx), Some(renderer), Some(server_win)) = (
                    &self.ui_context,
                    &self.renderer,
                    &mut self.server_list_window,
                ) {
                    let mut ui = UiFrame::new(
                        ui_ctx,
                        &renderer.font_atlas,
                        &mut self.ui_state_cache,
                        elapsed,
                        server_win.has_grf_textures,
                        None,
                        &self.saved_window_positions,
                    );
                    let events = server_win.build(&mut ui);
                    let any_hovered = ui.any_hovered;
                    let any_interactive = ui.any_interactive_hovered;
                    (ui.draw_calls, events, any_hovered, any_interactive)
                } else {
                    (Vec::new(), Vec::new(), false, false)
                }
            }
            AppState::CharacterSelect => {
                if let (Some(ui_ctx), Some(renderer), Some(char_win)) = (
                    &self.ui_context,
                    &self.renderer,
                    &mut self.char_select_window,
                ) {
                    let mut ui = UiFrame::new(
                        ui_ctx,
                        &renderer.font_atlas,
                        &mut self.ui_state_cache,
                        elapsed,
                        char_win.has_grf_textures,
                        None,
                        &self.saved_window_positions,
                    );
                    let events = char_win.build(&mut ui);
                    let any_hovered = ui.any_hovered;
                    let any_interactive = ui.any_interactive_hovered;
                    (ui.draw_calls, events, any_hovered, any_interactive)
                } else {
                    (Vec::new(), Vec::new(), false, false)
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
                        ui_ctx,
                        &renderer.font_atlas,
                        &mut self.ui_state_cache,
                        elapsed,
                        self.game.system_menu.has_grf_textures,
                        initial_focus,
                        &self.saved_window_positions,
                    );
                    let events = self.game.build_in_game_ui(
                        &mut ui,
                        &|name| renderer.texture_cache.texture_size(name),
                    );

                    let any_hovered = ui.any_hovered;
                    let any_interactive = ui.any_interactive_hovered;
                    (ui.draw_calls, events, any_hovered, any_interactive)
                } else {
                    (Vec::new(), Vec::new(), false, false)
                }
            }
        }
    }

    fn process_continuous_walk(&mut self, delta: f32) {
        if !self.input.left_mouse_down || self.game.app_state != AppState::InGame {
            return;
        }
        if self.input.ui_dragging {
            return;
        }
        if self.game.chat_window.is_active() {
            return;
        }
        if self.game.npc_dialog.dialog.is_open() || self.game.npc_shop.shop.is_open() {
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

    fn update_floor_items(&mut self, elapsed: f32) {
        for floor_item in self.game.floor_items.values_mut() {
            if floor_item.is_falling {
                let t = (elapsed - floor_item.drop_time) * 1000.0 / 24.0;
                // original game: position = initial_y + (-0.6 + 0.083*t) * t
                // initial_y is ground_y - 15 (15 units above ground)
                // Item lands when offset from ground >= 0:  -15 + (-0.6 + 0.083*t) * t >= 0
                let fall_offset = -15.0 + (-0.6 + 0.083 * t as f64) * t as f64;
                if fall_offset >= 0.0 {
                    floor_item.is_falling = false;
                }
            }
        }
    }

    // When pickup happen after move
    fn check_pending_pickup(&mut self) {
        let item_id = match self.game.pending_pickup_item_id {
            Some(id) => id,
            None => return,
        };
        if !self.game.floor_items.contains_key(&item_id) {
            self.game.pending_pickup_item_id = None;
            return;
        }
        let (px, py) = self
            .game
            .entities
            .player()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));
        let floor_item = &self.game.floor_items[&item_id];
        let dx = (px as i32 - floor_item.x as i32).unsigned_abs();
        let dy = (py as i32 - floor_item.y as i32).unsigned_abs();
        if dx <= 1 && dy <= 1 {
            if let Some(tx) = &self.network_cmd_tx {
                let packet = build_pickup_item_packet(item_id, self.config.packetver);
                let _ = tx.send(NetworkCommand::SendPacket(packet));
            }
            if let Some(entity) = self.game.entities.player_mut() {
                entity.movement.stop();
                entity.enter_pickup(0.5);
            }
            self.game.pending_pickup_item_id = None;
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
                    EntityState::Hurt
                        | EntityState::Attacking
                        | EntityState::Dead
                        | EntityState::Pickup
                );
                if is_transient {
                    entity.animation.set_action_one_shot(action);
                } else {
                    entity.animation.set_action(action);
                }
                entity.animation.set_direction(entity.direction);
                let is_composite = entity.entity_type == EntityType::Player;
                let animated = !is_composite
                    || SpriteActionType::from_index(entity.animation.action())
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

    fn compute_render_list(&self) -> Vec<RenderEntry> {
        let mut render_list = Vec::new();
        if let (Some(renderer), Some(coords)) = (&self.renderer, &self.game.map_coords) {
            for entity in self.game.entities.iter() {
                if let Some((screen_anchor, depth, camera_dir, sprite_scale, depth_gradient)) =
                    input::entity_screen_params(
                        entity.movement.position(),
                        self.game.gat.as_ref(),
                        coords,
                        &renderer.camera,
                        renderer.device.surface_config.width as f32 / renderer.dpi_scale,
                        renderer.device.surface_config.height as f32 / renderer.dpi_scale,
                    )
                {
                    let pick_bounds = match self.game.sprites.get(&entity.id) {
                        Some(sprite) => sprite.compute_pick_bounds(
                            &entity.animation,
                            Some(camera_dir),
                            entity.head_dir,
                            screen_anchor,
                            depth,
                            sprite_scale,
                        ),
                        None => {
                            let half = 50.0;
                            [
                                screen_anchor[0] - half,
                                screen_anchor[1] - 100.0,
                                screen_anchor[0] + half,
                                screen_anchor[1],
                            ]
                        }
                    };
                    render_list.push(RenderEntry {
                        kind: RenderEntryKind::Entity,
                        id: entity.id,
                        screen_anchor,
                        depth,
                        depth_gradient,
                        camera_dir,
                        sprite_scale,
                        pick_bounds,
                    });
                }
            }
        }
        render_list.sort_by(|a, b| {
            b.depth
                .partial_cmp(&a.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        render_list
    }

    fn compute_floor_item_render_list(&self) -> Vec<RenderEntry> {
        let mut render_list = Vec::new();
        if let (Some(renderer), Some(coords)) = (&self.renderer, &self.game.map_coords) {
            let screen_w = renderer.device.surface_config.width as f32 / renderer.dpi_scale;
            let screen_h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
            for floor_item in self.game.floor_items.values() {
                let pos = floor_item.world_position();
                if let Some((screen_anchor, depth, _camera_dir, sprite_scale, depth_gradient)) =
                    input::entity_screen_params(
                        pos,
                        self.game.gat.as_ref(),
                        coords,
                        &renderer.camera,
                        screen_w,
                        screen_h,
                    )
                {
                    let half = 17.0 * sprite_scale;
                    let pick_bounds = [
                        screen_anchor[0] - half,
                        screen_anchor[1] - half,
                        screen_anchor[0] + half,
                        screen_anchor[1] + half,
                    ];
                    render_list.push(RenderEntry {
                        kind: RenderEntryKind::FloorItem,
                        id: floor_item.id,
                        screen_anchor,
                        depth,
                        depth_gradient,
                        camera_dir: 0,
                        sprite_scale,
                        pick_bounds,
                    });
                }
            }
        }
        render_list.sort_by(|a, b| {
            b.depth
                .partial_cmp(&a.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        render_list
    }

    fn update_cursor_type(
        &mut self,
        hovered: Option<(i32, i32)>,
        ui_any_hovered: bool,
        ui_any_interactive_hovered: bool,
        render_list: &[RenderEntry],
    ) -> Option<u32> {
        let (cursor, hovered_entity_id) = if self.game.app_state == AppState::InGame {
            if self.input.right_mouse_down {
                (CursorType::Rotate, None)
            } else if ui_any_interactive_hovered {
                (CursorType::Click, None)
            } else if ui_any_hovered {
                (CursorType::Default, None)
            } else if let Some((entity_cursor, entity_id)) = hovered_entity_cursor_type(
                self.input.mouse_position,
                &self.game.entities,
                render_list,
            ) {
                (entity_cursor, Some(entity_id))
            } else if let Some(gat) = &self.game.gat {
                (cursor_type_for_cell(gat, hovered), None)
            } else {
                (CursorType::Default, None)
            }
        } else if ui_any_interactive_hovered {
            (CursorType::Click, None)
        } else {
            (CursorType::Default, None)
        };
        self.game.cursor_animation.set_cursor_type(cursor);
        hovered_entity_id
    }

    fn build_cursor_sprite_clips(&mut self, dt: f32) -> Vec<ClipData> {
        let cursor_act = match &self.game.cursor_act {
            Some(a) => a,
            None => return Vec::new(),
        };

        self.game.cursor_animation.update(dt, cursor_act);
        let action_idx = self.game.cursor_animation.action_index();
        let action_idx = if action_idx < cursor_act.actions.len() {
            action_idx
        } else {
            0
        };
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
            if let Some((vertices, indices, tex_idx)) =
                build_clip_quad(clip, cursor_tex, [mx as f32, my as f32], 0.0, [0, 0])
            {
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
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.screen_width,
                self.config.screen_height,
            ));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let os_scale = window.scale_factor() as f32;
        let dpi_scale = if self.config.dpi_scale > 0.0 {
            self.config.dpi_scale / 100.0
        } else {
            os_scale
        };
        let renderer = block_on(Renderer::new(
            window.clone(),
            self.config.font_px_height(),
            dpi_scale,
        ));

        let physical_size = window.inner_size();
        self.window = Some(window);
        self.renderer = Some(renderer);
        let mut ui_ctx = UiContext::new(
            physical_size.width as f32 / dpi_scale,
            physical_size.height as f32 / dpi_scale,
        );
        ui_ctx.dpi_scale = dpi_scale;
        self.ui_context = Some(ui_ctx);

        // Load GRF
        if let Some(grf_path) = self.config.grf_paths.first() {
            match GrfArchive::open(Path::new(grf_path)) {
                Ok(grf) => {
                    println!("GRF loaded: {} ({} files)", grf_path, grf.file_count());

                    if let Some(renderer) = &mut self.renderer {
                        renderer.try_load_grf_font(&grf);
                        preload_window(&mut self.login_window, renderer, &grf);
                    }

                    self.load_cursor_sprite(&grf);
                    self.load_emotion_sprite(&grf);
                    self.game.data_table.accessory =
                        Some(ragnarok_game::accessory_table::AccessoryTable::load_from_grf(&grf));
                    self.game.data_table.name = Some(NameTable::load(&grf));
                    self.game.data_table.item_name =
                        Some(ragnarok_game::item_name_table::ItemNameTable::load(&grf));
                    self.game.data_table.item_resource = Some(
                        ragnarok_game::item_resource_table::ItemResourceTable::load(&grf),
                    );
                    self.game.data_table.item_slot_count =
                        Some(ragnarok_game::item_slot_count_table::ItemSlotCountTable::load(&grf));
                    self.game.data_table.card_name =
                        Some(ragnarok_game::card_name_table::CardNameTable::load(&grf));
                    self.game.data_table.item_description =
                        Some(ragnarok_game::item_description_table::ItemDescriptionTable::load(&grf));
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
                let positions = self.ui_state_cache.extract_window_positions();
                let open_collapsed = self.game.extract_window_state(&self.ui_state_cache);
                let mut window_state = HashMap::new();
                for (id, pos) in &positions {
                    let (open, collapsed) = open_collapsed.get(id)
                        .copied().unwrap_or((false, false));
                    window_state.insert(*id, WindowStateEntry {
                        position: *pos,
                        open,
                        collapsed,
                    });
                }
                self.config.window_state = window_state;
                self.config.save("config.json");
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
                                if self.input.ui_hovered {
                                    self.input.ui_dragging = true;
                                } else {
                                    self.handle_left_click();
                                    self.input.walk_packet_cooldown = 0.5;
                                    self.input.walk_server_acked = false;
                                }
                            } else {
                                self.input.ui_dragging = false;
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let dpi = self.renderer.as_ref().map_or(1.0, |r| r.dpi_scale) as f64;
                let logical_pos = (position.x / dpi, position.y / dpi);
                self.input.mouse_position = logical_pos;
                if self.game.app_state == AppState::InGame && self.input.right_mouse_down {
                    if let Some((lx, ly)) = self.input.last_mouse_pos {
                        let dx = (logical_pos.0 - lx) as f32;
                        let dy = (logical_pos.1 - ly) as f32;
                        if let Some(renderer) = &mut self.renderer {
                            input::handle_camera_drag(
                                &mut renderer.camera,
                                dx,
                                dy,
                                self.config.free_camera,
                            );
                        }
                    }
                    self.input.last_mouse_pos = Some(logical_pos);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.game.app_state == AppState::InGame {
                    if !self.input.ui_hovered {
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
                            self.game.debug_show_pick_bounds = !self.game.debug_show_pick_bounds;
                        }
                        PhysicalKey::Code(KeyCode::Insert) => {
                            if let Some(entity) = self.game.entities.player() {
                                let action = if entity.state == EntityState::Sitting {
                                    3u8
                                } else {
                                    2u8
                                };
                                if let Some(tx) = &self.network_cmd_tx {
                                    let packet = build_action_request_packet(
                                        0,
                                        action,
                                        self.config.packetver,
                                    );
                                    let _ = tx.send(NetworkCommand::SendPacket(packet));
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::KeyE) if self.input.alt_pressed => {
                            self.game.character.inventory.toggle();
                        }
                        PhysicalKey::Code(KeyCode::KeyQ) if self.input.alt_pressed => {
                            self.game.equipment_window.toggle();
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.input.alt_pressed = modifiers.state().alt_key();
            }
            WindowEvent::RedrawRequested => {
                let elapsed = self.start_time.elapsed().as_secs_f32();

                self.handle_game_events(event_loop);

                let (ui_draw_calls, ui_events, ui_any_hovered, ui_any_interactive) =
                    self.build_ui(elapsed);
                self.input.ui_hovered = ui_any_hovered;
                self.handle_ui_events(ui_events, event_loop);
                let mut world_overlay_calls: Vec<UiDrawCall> = Vec::new();

                self.update_movement(elapsed);
                let delta = elapsed - self.last_render_time;
                self.last_render_time = elapsed;
                self.process_continuous_walk(delta);
                self.update_entity_state(delta);
                self.update_floor_items(elapsed);
                self.check_pending_pickup();
                self.load_missing_entity_sprites();
                self.update_sprite_animation(delta);

                let hovered = self.update_grid_hover();
                let render_list = self.compute_render_list();
                let floor_item_render_list = self.compute_floor_item_render_list();
                let hovered_entity_id = self.update_cursor_type(
                    hovered,
                    ui_any_hovered,
                    ui_any_interactive,
                    &render_list,
                );
                self.game.hovered_entity_id = hovered_entity_id;
                if let Some(entity_id) = hovered_entity_id {
                    if let Some(entity) = self.game.entities.get_mut(entity_id) {
                        if !entity.name_requested {
                            entity.name_requested = true;
                            if let Some(tx) = &self.network_cmd_tx {
                                let packet = build_reqname_packet(entity_id, self.config.packetver);
                                let _ = tx.send(NetworkCommand::SendPacket(packet));
                            }
                        }
                    }
                }

                // Floor item hover detection (entities have priority)
                let hovered_floor_item_id = if hovered_entity_id.is_none()
                    && !ui_any_hovered
                    && !self.input.right_mouse_down
                {
                    let (mx, my) = self.input.mouse_position;
                    let mx = mx as f32;
                    let my = my as f32;
                    floor_item_render_list
                        .iter()
                        .find(|entry| {
                            mx >= entry.pick_bounds[0]
                                && mx <= entry.pick_bounds[2]
                                && my >= entry.pick_bounds[1]
                                && my <= entry.pick_bounds[3]
                        })
                        .map(|entry| entry.id)
                } else {
                    None
                };
                self.game.hovered_floor_item_id = hovered_floor_item_id;
                if hovered_floor_item_id.is_some() {
                    self.game.cursor_animation.set_cursor_type(CursorType::Pick);
                }

                let cursor_clips = self.build_cursor_sprite_clips(delta);

                if let (Some(entity_id), Some(renderer)) = (hovered_entity_id, &self.renderer) {
                    if let Some(entity) = self.game.entities.get(entity_id) {
                        let hovered_entry = render_list.iter().find(|e| e.id == entity_id);
                        if let Some(entry) = hovered_entry {
                            let mut bar_y = entry.pick_bounds[3] + 5.0;
                            let hp_ratio = if self.game.entities.is_player(entity_id) {
                                Some(self.game.character.hp_percentage())
                            } else {
                                entity.hp_percentage()
                            };
                            if let Some(ratio) = hp_ratio {
                                let (_x, y) = render_hp_bar(
                                    &entry,
                                    ratio,
                                    entity.entity_type,
                                    &mut world_overlay_calls,
                                );
                                bar_y = y;
                                if self.game.entities.is_player(entity_id) {
                                    let sp_y = y + HP_BAR_HEIGHT;
                                    render_bar(entry.screen_anchor[0], sp_y, self.game.character.sp_percentage(), SP_BAR_COLOR, &mut world_overlay_calls);
                                    bar_y = sp_y;
                                }
                            }
                            if let Some(name) = &entity.name {
                                let text_width = renderer.font_atlas.measure_text(name);
                                let text_x = entry.screen_anchor[0] - text_width / 2.0;
                                let text_y = bar_y + HP_BAR_HEIGHT + 13.0;
                                let outline_color = [0.0, 0.0, 0.0, 1.0];
                                for &(dx, dy) in
                                    &[(-1.0_f32, 0.0_f32), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)]
                                {
                                    let (verts, indices) = ragnarok_ui::draw::text_vertices(
                                        name,
                                        text_x + dx,
                                        text_y + dy,
                                        outline_color,
                                        &renderer.font_atlas,
                                    );
                                    if !verts.is_empty() {
                                        world_overlay_calls.push(UiDrawCall {
                                            vertices: verts,
                                            indices,
                                            texture: ragnarok_renderer::UiTextureRef::FontAtlas,
                                        });
                                    }
                                }
                                let text_color = entity_name_color(entity.entity_type);
                                let (verts, indices) = ragnarok_ui::draw::text_vertices(
                                    name,
                                    text_x,
                                    text_y,
                                    text_color,
                                    &renderer.font_atlas,
                                );
                                if !verts.is_empty() {
                                    world_overlay_calls.push(UiDrawCall {
                                        vertices: verts,
                                        indices,
                                        texture: ragnarok_renderer::UiTextureRef::FontAtlas,
                                    });
                                }
                            }
                        }
                    }
                }

                // Player HP bar (always visible)
                if self.renderer.is_some() && self.game.entities.player().is_some() {
                    if hovered_entity_id != self.game.entities.player_id() {
                        let ratio = self.game.character.hp_percentage();
                        if let Some(entry) = render_list
                            .iter()
                            .find(|e| Some(e.id) == self.game.entities.player_id())
                        {
                            let (_x, y) = render_hp_bar(
                                entry,
                                ratio,
                                EntityType::Player,
                                &mut world_overlay_calls,
                            );
                            render_bar(entry.screen_anchor[0], y + HP_BAR_HEIGHT, self.game.character.sp_percentage(), SP_BAR_COLOR, &mut world_overlay_calls);
                        }
                    }
                }

                if let Some(renderer) = &self.renderer {
                    for entry in &render_list {
                        if let Some(entity) = self.game.entities.get(entry.id) {
                            if let Some(bubble) = &entity.chat_bubble {
                                let padding = 4.0;
                                let lines =
                                    ragnarok_ui::draw::word_wrap(&bubble.message, 150.0, |t| {
                                        renderer.font_atlas.measure_text(t)
                                    });

                                let line_h = renderer.font_atlas.line_height;
                                let total_h = line_h * lines.len() as f32 + padding * 2.0;
                                let widest = lines
                                    .iter()
                                    .map(|l| renderer.font_atlas.measure_text(l))
                                    .fold(0.0_f32, f32::max);
                                let box_w = widest + padding * 2.0;
                                let box_x = entry.screen_anchor[0] - box_w / 2.0;
                                let box_y = entry.pick_bounds[1] - 5.0 - total_h;

                                let (bg_verts, bg_idx) = ragnarok_ui::draw::quad_vertices(
                                    box_x,
                                    box_y,
                                    box_w,
                                    total_h,
                                    [0.0, 0.0, 0.0, 0.8],
                                );
                                world_overlay_calls.push(UiDrawCall {
                                    vertices: bg_verts.to_vec(),
                                    indices: bg_idx.to_vec(),
                                    texture: ragnarok_renderer::UiTextureRef::White,
                                });

                                for (i, line) in lines.iter().enumerate() {
                                    let line_w = renderer.font_atlas.measure_text(line);
                                    let lx = entry.screen_anchor[0] - line_w / 2.0;
                                    let ly = box_y + padding + line_h / 2.0 + line_h * i as f32;
                                    let (verts, indices) = ragnarok_ui::draw::text_vertices(
                                        line,
                                        lx,
                                        ly,
                                        [1.0, 1.0, 1.0, 1.0],
                                        &renderer.font_atlas,
                                    );
                                    if !verts.is_empty() {
                                        world_overlay_calls.push(UiDrawCall {
                                            vertices: verts,
                                            indices,
                                            texture: ragnarok_renderer::UiTextureRef::FontAtlas,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                // Floor item tooltip (when hovered)
                if let Some(fi_id) = hovered_floor_item_id {
                    if let Some(floor_item) = self.game.floor_items.get(&fi_id) {
                        if let Some(fi_entry) =
                            floor_item_render_list.iter().find(|e| e.id == fi_id)
                        {
                            if let Some(renderer) = &self.renderer {
                                let tooltip = if floor_item.count > 1 {
                                    format!("{} : {} ea.", floor_item.name, floor_item.count)
                                } else {
                                    floor_item.name.clone()
                                };
                                let text_w = renderer.font_atlas.measure_text(&tooltip);
                                let text_x = fi_entry.screen_anchor[0] - text_w / 2.0;
                                let text_y = fi_entry.pick_bounds[1] - 5.0;
                                let padding = 3.0;

                                let (bg_v, bg_i) = ragnarok_ui::draw::quad_vertices(
                                    text_x - padding,
                                    text_y - padding - 12.0,
                                    text_w + padding * 2.0,
                                    12.0 + padding * 2.0,
                                    [0.0, 0.0, 0.0, 0.85],
                                );
                                world_overlay_calls.push(UiDrawCall {
                                    vertices: bg_v.to_vec(),
                                    indices: bg_i.to_vec(),
                                    texture: ragnarok_renderer::UiTextureRef::White,
                                });

                                let (verts, indices) = ragnarok_ui::draw::text_vertices(
                                    &tooltip,
                                    text_x,
                                    text_y,
                                    [1.0, 1.0, 1.0, 1.0],
                                    &renderer.font_atlas,
                                );
                                if !verts.is_empty() {
                                    world_overlay_calls.push(UiDrawCall {
                                        vertices: verts,
                                        indices,
                                        texture: ragnarok_renderer::UiTextureRef::FontAtlas,
                                    });
                                }
                            }
                        }
                    }
                }

                if self.game.debug_show_pick_bounds {
                    let debug_color = [1.0, 0.0, 0.0, 0.7];
                    let line_thickness = 1.0;
                    for entry in render_list.iter().chain(floor_item_render_list.iter()) {
                        let [left, top, right, bottom] = entry.pick_bounds;
                        let w = right - left;
                        let h = bottom - top;
                        // Outline: top, bottom, left, right edges
                        for (x, y, bw, bh) in [
                            (left, top, w, line_thickness),
                            (left, bottom - line_thickness, w, line_thickness),
                            (left, top, line_thickness, h),
                            (right - line_thickness, top, line_thickness, h),
                        ] {
                            let (v, i) = ragnarok_ui::draw::quad_vertices(x, y, bw, bh, debug_color);
                            world_overlay_calls.push(UiDrawCall {
                                vertices: v.to_vec(),
                                indices: i.to_vec(),
                                texture: UiTextureRef::White,
                            });
                        }
                        // Screen center: red dot
                        let dot = 3.0;
                        let (v, i) = ragnarok_ui::draw::quad_vertices(
                            entry.screen_anchor[0] - dot,
                            entry.screen_anchor[1] - dot,
                            dot * 2.0,
                            dot * 2.0,
                            debug_color,
                        );
                        world_overlay_calls.push(UiDrawCall {
                            vertices: v.to_vec(),
                            indices: i.to_vec(),
                            texture: UiTextureRef::White,
                        });
                    }
                }

                {
                    let mut sprite_batches: Vec<SpriteBatch> = Vec::new();
                    let mut cursor_batches: Vec<SpriteBatch> = Vec::new();

                    // Merge entity and floor item render lists for unified depth sorting
                    let mut unified_list: Vec<&RenderEntry> = render_list
                        .iter()
                        .chain(floor_item_render_list.iter())
                        .collect();
                    unified_list.sort_by(|a, b| {
                        b.depth
                            .partial_cmp(&a.depth)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });

                    for entry in &unified_list {
                        match entry.kind {
                            RenderEntryKind::Entity => {
                                if let (Some(sprite), Some(entity)) = (
                                    self.game.sprites.get(&entry.id),
                                    self.game.entities.get(entry.id),
                                ) {
                                    let shadow_scale = entry.sprite_scale * shadow_size(entity.job);
                                    let mut shadow = sprite.build_shadow_batches(
                                        entry.screen_anchor,
                                        entry.depth,
                                        shadow_scale,
                                    );
                                    sprite_batches.append(&mut shadow);
                                    let mut batches = sprite.build_batches(
                                        &entity.animation,
                                        Some(entry.camera_dir),
                                        entity.head_dir,
                                        entry.screen_anchor,
                                        entry.depth,
                                        entry.sprite_scale,
                                        entry.depth_gradient,
                                    );
                                    sprite_batches.append(&mut batches);

                                    if let (Some(emo), Some(emo_act), Some(emo_tex)) = (
                                        &entity.emotion,
                                        &self.game.emotion_act,
                                        &self.game.emotion_textures,
                                    ) {
                                        let action_idx = emo.emotion_type as usize;
                                        if action_idx < emo_act.actions.len() {
                                            let delay_ms = emo_act
                                                .delays
                                                .get(action_idx)
                                                .map(|d| d * 25.0)
                                                .filter(|d| *d > 0.0)
                                                .unwrap_or(150.0);
                                            let motion_count =
                                                emo_act.actions[action_idx].motions.len();
                                            let motion_idx = if motion_count > 0 {
                                                ((emo.elapsed * 1000.0) / delay_ms) as usize
                                                    % motion_count
                                            } else {
                                                0
                                            };
                                            if motion_idx < motion_count {
                                                let motion = &emo_act.actions[action_idx].motions
                                                    [motion_idx];
                                                let emo_center = [
                                                    entry.screen_anchor[0],
                                                    entry.screen_anchor[1] - 100.0,
                                                ];
                                                for clip in &motion.clips {
                                                    if let Some((vertices, indices, tex_idx)) =
                                                        build_clip_quad(
                                                            clip,
                                                            emo_tex,
                                                            emo_center,
                                                            entry.depth,
                                                            [0, 0],
                                                        )
                                                    {
                                                        if tex_idx < emo_tex.bind_groups.len() {
                                                            sprite_batches.push(SpriteBatch {
                                                                vertices,
                                                                indices,
                                                                texture: &emo_tex.bind_groups
                                                                    [tex_idx],
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            RenderEntryKind::FloorItem => {
                                if let Some(floor_item) = self.game.floor_items.get(&entry.id) {
                                    if let Some((tex, act)) =
                                        self.game.floor_item_sprites.get(&entry.id)
                                    {
                                        let y_offset = if floor_item.is_falling {
                                            let t =
                                                (elapsed - floor_item.drop_time) * 1000.0 / 24.0;
                                            let fall_y =
                                                -15.0 + (-0.6 + 0.083 * t as f64) * t as f64;
                                            (fall_y.min(0.0) as f32) * entry.sprite_scale
                                        } else {
                                            0.0
                                        };

                                        let blink_frame = ((elapsed * 1000.0 / 24.0) as u32) % 92;
                                        let blink_active = blink_frame >= 90;

                                        let center = [
                                            entry.screen_anchor[0],
                                            entry.screen_anchor[1] + y_offset,
                                        ];

                                        if !act.actions.is_empty() {
                                            let action = &act.actions[0];
                                            let motion_count = action.motions.len();
                                            let delay_ms = act
                                                .delays
                                                .first()
                                                .map(|d| d * 25.0)
                                                .filter(|d| *d > 0.0)
                                                .unwrap_or(150.0);
                                            let item_elapsed = elapsed - floor_item.drop_time;
                                            let motion_idx = if motion_count > 0 {
                                                ((item_elapsed * 1000.0) / delay_ms) as usize
                                                    % motion_count
                                            } else {
                                                0
                                            };
                                            if motion_idx < motion_count {
                                                let motion = &action.motions[motion_idx];
                                                for clip in &motion.clips {
                                                    if let Some((mut vertices, indices, tex_idx)) =
                                                        build_clip_quad(
                                                            clip,
                                                            tex,
                                                            center,
                                                            entry.depth,
                                                            [0, 0],
                                                        )
                                                    {
                                                        scale_clip_vertices(
                                                            &mut vertices,
                                                            center,
                                                            entry.sprite_scale,
                                                            entry.depth_gradient,
                                                        );
                                                        if blink_active {
                                                            for v in &mut vertices {
                                                                v.color = [1.0, 1.0, 1.0, 1.0];
                                                            }
                                                        }
                                                        if tex_idx < tex.bind_groups.len() {
                                                            sprite_batches.push(SpriteBatch {
                                                                vertices,
                                                                indices,
                                                                texture: &tex.bind_groups[tex_idx],
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Build paperdoll as UI draw calls so it renders with the equipment window
                    let mut inline_textures = Vec::new();
                    let mut paperdoll_calls: Vec<UiDrawCall> = Vec::new();
                    if let Some(center) = self.game.equipment_window.character_center() {
                        if let Some(player_id) = self.game.entities.player_id() {
                            if let Some(sprite) = self.game.sprites.get(&player_id) {
                                let idle_anim = ragnarok_formats::act::SpriteAnimationState::new(0);
                                let batches = sprite
                                    .build_batches(&idle_anim, None, 0, center, 0.0, 1.0, 0.0);
                                for batch in batches {
                                    let idx = inline_textures.len();
                                    inline_textures.push(batch.texture);
                                    paperdoll_calls.push(UiDrawCall {
                                        vertices: batch
                                            .vertices
                                            .iter()
                                            .map(|sv| UiVertex {
                                                position: [sv.position[0], sv.position[1]],
                                                tex_coord: sv.tex_coord,
                                                color: sv.color,
                                            })
                                            .collect(),
                                        indices: batch.indices,
                                        texture: UiTextureRef::Inline(idx),
                                    });
                                }
                            }
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

                    let mut all_ui_calls = world_overlay_calls;
                    let overlay_len = all_ui_calls.len();
                    all_ui_calls.extend(ui_draw_calls);

                    if let Some(insert_idx) = self.game.equipment_window.paperdoll_insert_index() {
                        let abs_idx = (overlay_len + insert_idx).min(all_ui_calls.len());
                        for (i, dc) in paperdoll_calls.into_iter().enumerate() {
                            all_ui_calls.insert(abs_idx + i, dc);
                        }
                    }

                    if let Some(renderer) = &mut self.renderer {
                        renderer.render(
                            &all_ui_calls,
                            &sprite_batches,
                            &cursor_batches,
                            &inline_textures,
                            elapsed,
                        );
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

fn entity_name_color(entity_type: EntityType) -> [f32; 4] {
    match entity_type {
        EntityType::Player => [1.0, 1.0, 1.0, 1.0], // #FFFFFF
        EntityType::Monster => [1.0, 0.776, 0.776, 1.0], // #ffc6c6
        EntityType::Npc => [0.39, 0.54, 0.76, 1.0], // #648bc2
    }
}

fn hp_bar_color(ratio: f32, entity_type: EntityType) -> [f32; 4] {
    match entity_type {
        EntityType::Monster => {
            if ratio >= 0.25 {
                [1.0, 0.0, 0.906, 1.0]
            }
            // #FF00E7 magenta
            else {
                [1.0, 1.0, 0.0, 1.0]
            } // #FFFF00 yellow
        }
        _ => {
            if ratio >= 0.25 {
                [0.063, 0.937, 0.129, 1.0]
            }
            // #10ef21 bright green
            else {
                [1.0, 0.0, 0.0, 1.0]
            } // #FF0000 red
        }
    }
}

const HP_BAR_WIDTH: f32 = 60.0;
const HP_BAR_HEIGHT: f32 = 5.0;
const SP_BAR_COLOR: [f32; 4] = [0.063, 0.094, 0.61, 1.0];

fn render_bar(
    center_x: f32,
    y: f32,
    ratio: f32,
    fill_color: [f32; 4],
    draw_calls: &mut Vec<UiDrawCall>,
) {
    let border_x = center_x - HP_BAR_WIDTH / 2.0;
    let (border_verts, border_idx) = ragnarok_ui::draw::quad_vertices(
        border_x,
        y,
        HP_BAR_WIDTH,
        HP_BAR_HEIGHT,
        [0.063, 0.094, 0.612, 1.0],
    );
    draw_calls.push(UiDrawCall {
        vertices: border_verts.to_vec(),
        indices: border_idx.to_vec(),
        texture: ragnarok_renderer::UiTextureRef::White,
    });
    let (bg_verts, bg_idx) = ragnarok_ui::draw::quad_vertices(
        border_x + 1.0,
        y + 1.0,
        HP_BAR_WIDTH - 2.0,
        HP_BAR_HEIGHT - 2.0,
        [0.259, 0.259, 0.259, 1.0],
    );
    draw_calls.push(UiDrawCall {
        vertices: bg_verts.to_vec(),
        indices: bg_idx.to_vec(),
        texture: ragnarok_renderer::UiTextureRef::White,
    });
    let fill_ratio = ratio.clamp(0.0, 1.0);
    let fill_w = (HP_BAR_WIDTH - 2.0) * fill_ratio;
    let (fill_verts, fill_idx) = ragnarok_ui::draw::quad_vertices(
        border_x + 1.0,
        y + 1.0,
        fill_w,
        HP_BAR_HEIGHT - 2.0,
        fill_color,
    );
    draw_calls.push(UiDrawCall {
        vertices: fill_verts.to_vec(),
        indices: fill_idx.to_vec(),
        texture: ragnarok_renderer::UiTextureRef::White,
    });
}

fn render_hp_bar(
    entry: &RenderEntry,
    ratio: f32,
    entity_type: EntityType,
    draw_calls: &mut Vec<UiDrawCall>,
) -> (f32, f32) {
    let center_x = entry.screen_anchor[0];
    let y = entry.pick_bounds[3];
    render_bar(center_x, y, ratio, hp_bar_color(ratio, entity_type), draw_calls);
    (center_x, y)
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
