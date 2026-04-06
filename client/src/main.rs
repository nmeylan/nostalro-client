mod config;
mod game_state;
mod input;

use config::Config;
use game_state::GameState;
use input::InputState;
use models::enums::EnumWithNumberValue;
use models::enums::status::StatusTypes;
use ragnarok_formats::act::SpriteActionType;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::app_state::AppState;
use ragnarok_game::cursor::{
    CursorType, RenderEntry, cursor_type_for_cell, hovered_entity_cursor_type,
};
use ragnarok_game::entity::{Entity, EntityState, EntityType};
use ragnarok_game::event::GameEvent;
use ragnarok_game::inventory::InventoryItem;
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
    build_npc_menu_select_packet, build_npc_next_packet, build_purchase_item_list_packet,
    build_reqname_packet, build_request_move_packet, build_restart_packet,
    build_select_char_packet, build_sell_item_list_packet, build_unequip_item_packet,
    build_use_item_packet, build_zone_enter_packet, ip_u32_to_string, network_loop,
};
use ragnarok_renderer::{
    GridSelectorRenderer, Renderer, SpriteBatch, SpriteVertex, UiDrawCall, block_on,
    build_clip_quad, build_entity_sprite, upload_sprite_textures,
};
use ragnarok_ui::context::UiContext;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::state::StateCache;
use ragnarok_ui_component::char_select_window::CharSelectWindow;
use ragnarok_ui_component::chat_window::ChatWindow;
use ragnarok_ui_component::inventory_window::InventoryWindow;
use ragnarok_ui_component::login_window::{LoginFocus, LoginWindow};
use ragnarok_ui_component::npc_dialog::NpcDialog;
use ragnarok_ui_component::npc_shop::NpcShop;
use ragnarok_ui_component::server_list_window::ServerListWindow;
use ragnarok_ui_component::system_menu::SystemMenu;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::info;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};
use ragnarok_ui_component::equipment_window::EquipmentWindow;

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
            rt.block_on(network_loop(cmd_rx, event_tx, packetver, debug_delay_ms, trace_packets));
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
                        server_win.has_grf_textures =
                            renderer.preload_textures(&ServerListWindow::grf_texture_paths(), grf);
                        if server_win.has_grf_textures {
                            server_win.set_texture_sizes(|name| renderer.texture_cache.texture_size(name));
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
                        char_win.has_grf_textures =
                            renderer.preload_textures(&CharSelectWindow::grf_texture_paths(), grf);
                        if char_win.has_grf_textures {
                            char_win.set_texture_sizes(|name| renderer.texture_cache.texture_size(name));
                        }
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
                    self.game.entities.clear();
                    self.game.sprites.clear();
                    self.game.sprite_cache.clear();
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
                        self.game.chat_window.has_grf_textures =
                            renderer.preload_textures(&ChatWindow::grf_texture_paths(), grf);
                        self.game.system_menu.has_grf_textures =
                            renderer.preload_textures(&SystemMenu::grf_texture_paths(), grf);
                        if self.game.system_menu.has_grf_textures {
                            self.game.system_menu.set_texture_sizes(|name| renderer.texture_cache.texture_size(name));
                        }
                        self.game.inventory_window.has_grf_textures =
                            renderer.preload_textures(&InventoryWindow::grf_texture_paths(), grf);
                        if self.game.inventory_window.has_grf_textures {
                            self.game.inventory_window.set_texture_sizes(|name| renderer.texture_cache.texture_size(name));
                        }
                        self.game.equipment_window.has_grf_textures =
                            renderer.preload_textures(&EquipmentWindow::grf_texture_paths(), grf);
                        if self.game.equipment_window.has_grf_textures {
                            self.game.equipment_window.set_texture_sizes(|name| renderer.texture_cache.texture_size(name));
                        }
                    }

                    self.game.app_state = AppState::InGame;

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
                    if let Some(entity) = self.game.entities.get_mut(gid) {
                        entity.hp = Some(hp);
                        entity.max_hp = Some(max_hp);
                    }
                }
                GameEvent::NpcDialogText { npc_id, text } => {
                    self.game.npc_dialog.dialog.open_text(npc_id, &text);
                    self.preload_npc_dialog_textures();
                }
                GameEvent::NpcDialogNext { npc_id } => {
                    self.game.npc_dialog.dialog.wait_for_next(npc_id);
                }
                GameEvent::NpcDialogClose { npc_id } => {
                    self.game.npc_dialog.dialog.wait_for_close(npc_id);
                }
                GameEvent::NpcDialogMenu { npc_id, items } => {
                    self.game.npc_dialog.dialog.show_menu(npc_id, items);
                    self.preload_npc_dialog_textures();
                }
                GameEvent::NpcInputNumber { npc_id } => {
                    self.game.npc_dialog.dialog.wait_for_number_input(npc_id);
                    self.preload_npc_dialog_textures();
                }
                GameEvent::NpcInputString { npc_id } => {
                    self.game.npc_dialog.dialog.wait_for_string_input(npc_id);
                    self.preload_npc_dialog_textures();
                }
                GameEvent::NpcDealTypeSelect { npc_id } => {
                    self.game.npc_dialog.dialog.show_deal_type(npc_id);
                    self.preload_npc_dialog_textures();
                }
                GameEvent::NpcShopBuyList { npc_id, items } => {
                    let buy_items: Vec<_> = items
                        .into_iter()
                        .map(|(item_id, price, discount_price, item_type)| {
                            let name = self
                                .game
                                .item_name_table
                                .as_ref()
                                .map(|t| t.get_name_or_id(item_id))
                                .unwrap_or_else(|| format!("Item #{item_id}"));
                            let resource_name =
                                self.game.item_resource_table.as_ref().and_then(|t| {
                                    t.get_resource_name(item_id).map(|s| s.to_string())
                                });
                            ragnarok_game::npc_shop::ShopBuyItem {
                                item_id,
                                price,
                                discount_price,
                                item_type,
                                name,
                                resource_name,
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
                    self.preload_npc_shop_textures();
                }
                GameEvent::NpcShopSellList { npc_id, items } => {
                    let sell_items = items
                        .into_iter()
                        .map(|(index, price, overcharge_price)| {
                            let inv_item = self.game.inventory_window.inventory.get_item(index as u16);
                            let name = inv_item.map(|i| i.name.clone())
                                .unwrap_or_else(|| format!("Item #{index}"));
                            let resource_name = inv_item.and_then(|i| i.resource_name.clone());
                            let count = inv_item.map(|i| i.count).unwrap_or(1);
                            ragnarok_game::npc_shop::ShopSellItem {
                                index,
                                price,
                                overcharge_price,
                                name,
                                resource_name,
                                count,
                            }
                        })
                        .collect();
                    let shop_npc_id = if npc_id != 0 {
                        npc_id
                    } else {
                        self.game.npc_dialog.dialog.npc_id
                    };
                    self.game.npc_shop.shop.open_sell(shop_npc_id, sell_items);
                    self.game.npc_dialog.dialog.close();
                    self.preload_npc_shop_textures();
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
                            .item_name_table
                            .as_ref()
                            .and_then(|t| t.get_name(info.item_id))
                            .unwrap_or("Unknown")
                            .to_string();
                        let resource_name =
                            self.game.item_resource_table.as_ref().and_then(|t| {
                                t.get_resource_name(info.item_id).map(|s| s.to_string())
                            });
                        self.game
                            .inventory_window
                            .inventory
                            .add_item(InventoryItem {
                                index: info.index as u16,
                                item_id: info.item_id,
                                item_type: info.item_type,
                                count: info.count,
                                is_identified: info.is_identified,
                                is_damaged: false,
                                refining_level: 0,
                                slot: [0; 4],
                                location: 0,
                                wear_state: info.wear_state,
                                name,
                                resource_name,
                            });
                    }
                    self.preload_inventory_textures();
                }
                GameEvent::InventoryEquipmentItems { items } => {
                    for info in items {
                        let name = self
                            .game
                            .item_name_table
                            .as_ref()
                            .and_then(|t| t.get_name(info.item_id))
                            .unwrap_or("Unknown")
                            .to_string();
                        let resource_name =
                            self.game.item_resource_table.as_ref().and_then(|t| {
                                t.get_resource_name(info.item_id).map(|s| s.to_string())
                            });
                        tracing::debug!(
                            "Equipment item: idx={} id={} type={} name={} loc={} wear={}",
                            info.index, info.item_id, info.item_type, name, info.location, info.wear_state,
                        );
                        self.game
                            .inventory_window
                            .inventory
                            .add_item(InventoryItem {
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
                    self.preload_inventory_textures();
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
                            .item_name_table
                            .as_ref()
                            .and_then(|t| t.get_name(item_id))
                            .unwrap_or("Unknown")
                            .to_string();
                        let resource_name = self
                            .game
                            .item_resource_table
                            .as_ref()
                            .and_then(|t| t.get_resource_name(item_id).map(|s| s.to_string()));
                        self.game
                            .inventory_window
                            .inventory
                            .add_item(InventoryItem {
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
                        self.preload_inventory_textures();
                    }
                }
                GameEvent::InventoryUseItemResult {
                    index,
                    count,
                    success,
                } => {
                    if success {
                        self.game
                            .inventory_window
                            .inventory
                            .update_item_count(index, count);
                    }
                }
                GameEvent::InventoryEquipResult {
                    index,
                    wear_location,
                    success,
                } => {
                    tracing::debug!(
                        "EquipResult: idx={} wear_loc={} success={}",
                        index, wear_location, success,
                    );
                    if success {
                        self.game
                            .inventory_window
                            .inventory
                            .update_wear_state(index, wear_location);
                    }
                }
                GameEvent::InventoryUnequipResult { index, success, wear_location } => {
                    tracing::debug!(
                        "UnequipResult: idx={} wear_loc={} success={}",
                        index, wear_location, success,
                    );
                    if success {
                        self.game.inventory_window.inventory.clear_wear_state(index);
                    }
                }
                GameEvent::InventoryItemRemoved { index, count } => {
                    self.game
                        .inventory_window
                        .inventory
                        .subtract_item_count(index, count);
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
                                if let Some(c) = &mut self.game.selected_character {
                                    c.hp = value as u32;
                                }
                                if let Some(e) = self.game.entities.player_mut() {
                                    e.hp = Some(value as u32);
                                }
                            }
                            StatusTypes::Maxhp => {
                                if let Some(c) = &mut self.game.selected_character {
                                    c.max_hp = value as u32;
                                }
                                if let Some(e) = self.game.entities.player_mut() {
                                    e.max_hp = Some(value as u32);
                                }
                            }
                            StatusTypes::Sp => {
                                if let Some(c) = &mut self.game.selected_character {
                                    c.sp = value as u16;
                                }
                            }
                            StatusTypes::Maxsp => {
                                if let Some(c) = &mut self.game.selected_character {
                                    c.max_sp = value as u16;
                                }
                            }
                            StatusTypes::Baselevel => {
                                if let Some(c) = &mut self.game.selected_character {
                                    c.base_level = value as u16;
                                }
                            }
                            StatusTypes::Str => {
                                if let Some(c) = &mut self.game.selected_character {
                                    c.str = value as u8;
                                }
                            }
                            StatusTypes::Agi => {
                                if let Some(c) = &mut self.game.selected_character {
                                    c.agi = value as u8;
                                }
                            }
                            StatusTypes::Vit => {
                                if let Some(c) = &mut self.game.selected_character {
                                    c.vit = value as u8;
                                }
                            }
                            StatusTypes::Int => {
                                if let Some(c) = &mut self.game.selected_character {
                                    c.int = value as u8;
                                }
                            }
                            StatusTypes::Dex => {
                                if let Some(c) = &mut self.game.selected_character {
                                    c.dex = value as u8;
                                }
                            }
                            StatusTypes::Luk => {
                                if let Some(c) = &mut self.game.selected_character {
                                    c.luk = value as u8;
                                }
                            }
                            StatusTypes::Joblevel => {
                                if let Some(c) = &mut self.game.selected_character {
                                    c.job_level = value as u32;
                                }
                            }
                            StatusTypes::Weight => {
                                self.game.inventory_window.inventory.weight = value;
                            }
                            StatusTypes::Maxweight => {
                                self.game.inventory_window.inventory.max_weight = value;
                            }
                            StatusTypes::Zeny => {
                                self.game.inventory_window.inventory.zeny = value;
                            }
                            _ => {}
                        }
                    }
                }
                GameEvent::StatusChanged {
                    status_type, base, ..
                } => {
                    if let Some(c) = &mut self.game.selected_character {
                        if let Ok(status) = StatusTypes::try_from_value(status_type as usize) {
                            match status {
                                StatusTypes::Str => c.str = base as u8,
                                StatusTypes::Agi => c.agi = base as u8,
                                StatusTypes::Vit => c.vit = base as u8,
                                StatusTypes::Int => c.int = base as u8,
                                StatusTypes::Dex => c.dex = base as u8,
                                StatusTypes::Luk => c.luk = base as u8,
                                _ => {}
                            }
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
                    self.game.inventory_window.inventory.clear();
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
        let accessory_table = self.game.accessory_table.as_ref().unwrap_or(&empty_table);
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
                let name_table = match &self.game.name_table {
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

    fn preload_npc_dialog_textures(&mut self) {
        if self.game.npc_dialog.has_grf_textures {
            return;
        }
        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            self.game.npc_dialog.has_grf_textures =
                renderer.preload_textures(&NpcDialog::grf_texture_paths(), grf);
            if self.game.npc_dialog.has_grf_textures {
                self.game.npc_dialog.set_texture_sizes(|name| renderer.texture_cache.texture_size(name));
            }
        }
    }

    fn preload_npc_shop_textures(&mut self) {
        if self.game.npc_shop.has_grf_textures {
            return;
        }
        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            self.game.npc_shop.has_grf_textures =
                renderer.preload_textures(&NpcShop::grf_texture_paths(), grf);
            if self.game.npc_shop.has_grf_textures {
                self.game.npc_shop.set_texture_sizes(|name| renderer.texture_cache.texture_size(name));
            }
            // Preload item icon textures
            let icon_paths: Vec<String> = self
                .game
                .npc_shop
                .shop
                .buy_items
                .iter()
                .filter_map(|item| {
                    item.resource_name
                        .as_ref()
                        .map(|name| format!("data/texture/유저인터페이스/item/{name}.bmp"))
                })
                .chain(
                    self.game.npc_shop.shop.sell_items.iter().filter_map(|item| {
                        item.resource_name
                            .as_ref()
                            .map(|name| format!("data/texture/유저인터페이스/item/{name}.bmp"))
                    }),
                )
                .collect();
            let icon_refs: Vec<&str> = icon_paths.iter().map(|s| s.as_str()).collect();
            renderer.preload_textures(&icon_refs, grf);
        }
    }

    fn preload_inventory_textures(&mut self) {
        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            if !self.game.inventory_window.has_grf_textures {
                self.game.inventory_window.has_grf_textures =
                    renderer.preload_textures(&InventoryWindow::grf_texture_paths(), grf);
                if self.game.inventory_window.has_grf_textures {
                    self.game.inventory_window.set_texture_sizes(|name| renderer.texture_cache.texture_size(name));
                }
            }
            if !self.game.equipment_window.has_grf_textures {
                self.game.equipment_window.has_grf_textures =
                    renderer.preload_textures(&EquipmentWindow::grf_texture_paths(), grf);
                if self.game.equipment_window.has_grf_textures {
                    self.game.equipment_window.set_texture_sizes(|name| renderer.texture_cache.texture_size(name));
                }
            }
            // Preload item icon textures
            let icon_paths: Vec<String> = self
                .game
                .inventory_window
                .inventory
                .all_items()
                .iter()
                .filter_map(|item| item.icon_path())
                .collect();
            let icon_refs: Vec<&str> = icon_paths.iter().map(|s| s.as_str()).collect();
            renderer.preload_textures(&icon_refs, grf);
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
                    );
                    let chat_was_active = self.game.chat_window.is_active();
                    let mut events = self.game.chat_window.build(&mut ui);

                    let npc_dialog_open = self.game.npc_dialog.dialog.is_open();
                    let npc_events = self.game.npc_dialog.build(&mut ui);
                    events.extend(npc_events);

                    let shop_open = self.game.npc_shop.shop.is_open();
                    let shop_events = self.game.npc_shop.build(&mut ui);
                    events.extend(shop_events);

                    let inv_open = self.game.inventory_window.inventory.is_open();
                    let inv_events = self.game.inventory_window.build(&mut ui);
                    events.extend(inv_events);

                    let eq_open = self.game.equipment_window.is_open();
                    let eq_events = self.game.equipment_window.build(
                        &mut ui,
                        &self.game.inventory_window.inventory,
                        self.game.item_slot_count_table.as_ref(),
                        self.game.card_name_table.as_ref(),
                    );
                    events.extend(eq_events);

                    let allow_escape =
                        !chat_was_active && !npc_dialog_open && !shop_open && !inv_open;
                    let menu_events = self.game.system_menu.build(&mut ui, allow_escape);
                    events.extend(menu_events);

                    ui.draw_drag_icon();

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
                if let Some((screen_center, depth, camera_dir, sprite_scale, depth_gradient)) =
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
                            screen_center,
                            depth,
                            sprite_scale,
                        ),
                        None => {
                            let half = 50.0;
                            [
                                screen_center[0] - half,
                                screen_center[1] - 100.0,
                                screen_center[0] + half,
                                screen_center[1],
                            ]
                        }
                    };
                    render_list.push(RenderEntry {
                        id: entity.id,
                        screen_center,
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

                        self.login_window.has_grf_textures =
                            renderer.preload_textures(&LoginWindow::grf_texture_paths(), &grf);
                        if self.login_window.has_grf_textures {
                            self.login_window.set_texture_sizes(|name| renderer.texture_cache.texture_size(name));
                        }
                    }

                    self.load_cursor_sprite(&grf);
                    self.load_emotion_sprite(&grf);
                    self.game.accessory_table =
                        Some(ragnarok_game::accessory_table::AccessoryTable::load_from_grf(&grf));
                    self.game.name_table = Some(NameTable::load(&grf));
                    self.game.item_name_table =
                        Some(ragnarok_game::item_name_table::ItemNameTable::load(&grf));
                    self.game.item_resource_table = Some(
                        ragnarok_game::item_resource_table::ItemResourceTable::load(&grf),
                    );
                    self.game.item_slot_count_table = Some(
                        ragnarok_game::item_slot_count_table::ItemSlotCountTable::load(&grf),
                    );
                    self.game.card_name_table = Some(
                        ragnarok_game::card_name_table::CardNameTable::load(&grf),
                    );
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
                            self.game.inventory_window.inventory.toggle();
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

                let (ui_draw_calls, ui_events, ui_any_hovered, ui_any_interactive) = self.build_ui(elapsed);
                self.input.ui_hovered = ui_any_hovered;
                self.handle_ui_events(ui_events, event_loop);
                let mut world_overlay_calls: Vec<UiDrawCall> = Vec::new();

                self.update_movement(elapsed);
                let delta = elapsed - self.last_render_time;
                self.last_render_time = elapsed;
                self.process_continuous_walk(delta);
                self.update_entity_state(delta);
                self.load_missing_entity_sprites();
                self.update_sprite_animation(delta);

                let hovered = self.update_grid_hover();
                let render_list = self.compute_render_list();
                let hovered_entity_id =
                    self.update_cursor_type(hovered, ui_any_hovered, ui_any_interactive, &render_list);
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

                let cursor_clips = self.build_cursor_sprite_clips(delta);

                if let (Some(entity_id), Some(renderer)) = (hovered_entity_id, &self.renderer) {
                    if let Some(entity) = self.game.entities.get(entity_id) {
                        let hovered_entry = render_list.iter().find(|e| e.id == entity_id);
                        if let Some(entry) = hovered_entry {
                            let bar_y = entry.pick_bounds[3] + 2.0;
                            if let Some(ratio) = entity.hp_percentage() {
                                render_hp_bar(
                                    entry.screen_center[0],
                                    bar_y,
                                    ratio,
                                    entity.entity_type,
                                    &mut world_overlay_calls,
                                );
                            }
                            if let Some(name) = &entity.name {
                                let text_width = renderer.font_atlas.measure_text(name);
                                let text_x = entry.screen_center[0] - text_width / 2.0;
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
                if let (Some(_), Some(player)) = (&self.renderer, self.game.entities.player()) {
                    if hovered_entity_id != self.game.entities.player_id() {
                        if let Some(ratio) = player.hp_percentage() {
                            if let Some(entry) = render_list
                                .iter()
                                .find(|e| Some(e.id) == self.game.entities.player_id())
                            {
                                let bar_y = entry.pick_bounds[3] + 2.0;
                                render_hp_bar(
                                    entry.screen_center[0],
                                    bar_y,
                                    ratio,
                                    EntityType::Player,
                                    &mut world_overlay_calls,
                                );
                            }
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
                                let box_x = entry.screen_center[0] - box_w / 2.0;
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
                                    let lx = entry.screen_center[0] - line_w / 2.0;
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

                {
                    let mut sprite_batches: Vec<SpriteBatch> = Vec::new();
                    let mut cursor_batches: Vec<SpriteBatch> = Vec::new();

                    for entry in &render_list {
                        if let (Some(sprite), Some(entity)) = (
                            self.game.sprites.get(&entry.id),
                            self.game.entities.get(entry.id),
                        ) {
                            let shadow_scale = entry.sprite_scale * shadow_size(entity.job);
                            let mut shadow = sprite.build_shadow_batches(
                                entry.screen_center,
                                entry.depth,
                                shadow_scale,
                            );
                            sprite_batches.append(&mut shadow);
                            let mut batches = sprite.build_batches(
                                &entity.animation,
                                Some(entry.camera_dir),
                                entity.head_dir,
                                entry.screen_center,
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
                                    let motion_count = emo_act.actions[action_idx].motions.len();
                                    let motion_idx = if motion_count > 0 {
                                        ((emo.elapsed * 1000.0) / delay_ms) as usize % motion_count
                                    } else {
                                        0
                                    };
                                    if motion_idx < motion_count {
                                        let motion =
                                            &emo_act.actions[action_idx].motions[motion_idx];
                                        let emo_center = [
                                            entry.screen_center[0],
                                            entry.screen_center[1] - 100.0,
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
                                                        texture: &emo_tex.bind_groups[tex_idx],
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(center) = self.game.equipment_window.character_center() {
                        if let Some(player_id) = self.game.entities.player_id() {
                            if let Some(sprite) = self.game.sprites.get(&player_id) {
                                let idle_anim = ragnarok_formats::act::SpriteAnimationState::new(0);
                                let mut paperdoll = sprite.build_batches(
                                    &idle_anim, None, 0, center, 0.0, 1.0, 0.0,
                                );
                                cursor_batches.append(&mut paperdoll);
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
                    all_ui_calls.extend(ui_draw_calls);

                    if let Some(renderer) = &mut self.renderer {
                        renderer.render(&all_ui_calls, &sprite_batches, &cursor_batches, elapsed);
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
        EntityType::Player => [1.0, 1.0, 1.0, 1.0], // #FFFFFFEntityType::Npc => [0.580, 0.741, 0.969, 1.0],
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

fn render_hp_bar(
    center_x: f32,
    y: f32,
    ratio: f32,
    entity_type: EntityType,
    draw_calls: &mut Vec<UiDrawCall>,
) {
    let border_x = center_x - HP_BAR_WIDTH / 2.0;
    // Border: #10189c dark blue
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
    // Background: #424242 dark gray (1px inside border)
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
    // Fill
    let fill_ratio = ratio.clamp(0.0, 1.0);
    let fill_w = (HP_BAR_WIDTH - 2.0) * fill_ratio;
    let (fill_verts, fill_idx) = ragnarok_ui::draw::quad_vertices(
        border_x + 1.0,
        y + 1.0,
        fill_w,
        HP_BAR_HEIGHT - 2.0,
        hp_bar_color(ratio, entity_type),
    );
    draw_calls.push(UiDrawCall {
        vertices: fill_verts.to_vec(),
        indices: fill_idx.to_vec(),
        texture: ragnarok_renderer::UiTextureRef::White,
    });
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
