mod config;

use config::Config;
use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsw::RswFile;
use ragnarok_formats::spr::SprFile;
use ragnarok_game::animation::SpriteAnimationState;
use ragnarok_game::cursor::{CursorAnimationState, CursorType};
use ragnarok_game::entity::Entity;
use ragnarok_game::event::{CharacterInfo, GameEvent};
use ragnarok_game::path::path_search;
use ragnarok_game::sprite_path::body_sprite_path;
use ragnarok_network::{build_char_enter_packet, build_login_packet, build_request_move_packet, build_select_char_packet, build_zone_enter_packet, ip_u32_to_string, network_loop, NetworkCommand};
use ragnarok_network::session::Session;
use ragnarok_renderer::{GridSelectorRenderer, Renderer, SpriteBatch, SpriteTextures, build_clip_quad, upload_sprite_textures};
use ragnarok_ui::context::UiContext;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui_component::login_window::{LoginFocus, LoginWindow};
use ragnarok_ui_component::char_select_window::CharSelectWindow;
use ragnarok_ui_component::server_list_window::ServerListWindow;
use ragnarok_ui::state::StateCache;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppState {
    Login,
    ServerSelect,
    CharacterSelect,
    InGame,
}

struct EntitySprite {
    textures: SpriteTextures,
    act: ActFile,
    animation: SpriteAnimationState,
}

struct App {
    config: Config,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    grf: Option<GrfArchive>,
    right_mouse_down: bool,
    last_mouse_pos: Option<(f64, f64)>,
    ui_context: Option<UiContext>,
    ui_state_cache: StateCache,
    login_window: LoginWindow,
    server_list_window: Option<ServerListWindow>,
    char_select_window: Option<CharSelectWindow>,
    login_session: Option<Session>,
    network_cmd_tx: Option<mpsc::UnboundedSender<NetworkCommand>>,
    game_event_rx: Option<mpsc::UnboundedReceiver<GameEvent>>,
    app_state: AppState,
    mouse_position: (f64, f64),
    map_zoom: Option<f32>,
    gat: Option<GatFile>,
    gat_dimensions: Option<(i32, i32)>,
    gnd_dimensions: Option<(i32, i32)>,
    current_map: Option<String>,
    selected_character: Option<CharacterInfo>,
    player_entity: Option<Entity>,
    player_sprite: Option<EntitySprite>,
    cursor_textures: Option<SpriteTextures>,
    cursor_act: Option<ActFile>,
    cursor_animation: CursorAnimationState,
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
            right_mouse_down: false,
            last_mouse_pos: None,
            ui_context: None,
            ui_state_cache: StateCache::new(),
            login_window: LoginWindow::new(),
            server_list_window: None,
            char_select_window: None,
            login_session: None,
            network_cmd_tx: None,
            game_event_rx: None,
            app_state: AppState::Login,
            mouse_position: (0.0, 0.0),
            map_zoom: None,
            gat: None,
            gat_dimensions: None,
            gnd_dimensions: None,
            current_map: None,
            selected_character: None,
            player_entity: None,
            player_sprite: None,
            cursor_textures: None,
            cursor_act: None,
            cursor_animation: CursorAnimationState::new(),
            start_time: Instant::now(),
            last_render_time: 0.0,
        }
    }

    fn load_map(&mut self, map_name: &str) {
        let grf = match &self.grf {
            Some(g) => g,
            None => return,
        };

        let rsw_path = format!("data/{map_name}.rsw");
        let rsw_data = match grf.read_file(&rsw_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to read RSW {rsw_path}: {e}");
                return;
            }
        };
        let rsw = match RswFile::parse(&rsw_data) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to parse RSW: {e}");
                return;
            }
        };

        let gnd_path = format!("data/{map_name}.gnd");
        let gnd_data = match grf.read_file(&gnd_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to read GND {gnd_path}: {e}");
                return;
            }
        };
        let gnd = match GndFile::parse(&gnd_data) {
            Ok(g) => g,
            Err(e) => {
                tracing::error!("Failed to parse GND: {e}");
                return;
            }
        };

        println!(
            "Map: {map_name} ({}x{}, {} textures, {} surfaces, {} lightmaps)",
            gnd.width,
            gnd.height,
            gnd.textures.len(),
            gnd.surfaces.len(),
            gnd.lightmaps.len()
        );

        self.map_zoom = Some(gnd.zoom);
        self.gnd_dimensions = Some((gnd.width, gnd.height));

        let gat_path = format!("data/{map_name}.gat");
        if let Ok(gat_data) = grf.read_file(&gat_path) {
            if let Ok(gat) = GatFile::parse(&gat_data) {
                self.gat_dimensions = Some((gat.width, gat.height));
                self.gat = Some(gat);
            }
        }

        if let Some(renderer) = &mut self.renderer {
            renderer.load_map(&gnd, &rsw, grf);

            if let Some(gat) = &self.gat {
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
                    gnd.width, gnd.height, gnd.zoom,
                );
                renderer.grid_selector = Some(grid);
            }
        }
    }

    fn position_camera_at(&mut self, cell_x: f32, cell_y: f32) {
        if let (Some(zoom), Some(renderer)) = (self.map_zoom, &mut self.renderer) {
            let (gat_w, gat_h) = self.gat_dimensions.unwrap_or((0, 0));
            let (gnd_w, gnd_h) = self.gnd_dimensions.unwrap_or((gat_w, gat_h));

            let gnd_cell_x = (cell_x + 0.5) * (gnd_w as f32 / gat_w as f32);
            let gnd_cell_y = (cell_y + 0.5) * (gnd_h as f32 / gat_h as f32);

            let mut wx = gnd_cell_x * zoom;
            let mut wz = gnd_cell_y * zoom;

            wx = wx.clamp(0.0, gnd_w as f32 * zoom);
            wz = wz.clamp(0.0, gnd_h as f32 * zoom);
            renderer.camera.set_target(wx, 0.0, wz);
        }
    }

    fn hovered_cell(&self) -> Option<(i32, i32)> {
        let (gat, renderer, zoom) = match (&self.gat, &self.renderer, self.map_zoom) {
            (Some(g), Some(r), Some(z)) => (g, r, z),
            _ => return None,
        };
        let (mx, my) = self.mouse_position;
        let size = renderer.device.surface_config.width as f32;
        let size_h = renderer.device.surface_config.height as f32;
        let (origin, dir) = renderer.camera.screen_to_ray(mx as f32, my as f32, size, size_h);

        if dir.y.abs() < 1e-6 {
            return None;
        }
        let t = -origin.y / dir.y;
        if t < 0.0 {
            return None;
        }
        let hit = origin + dir * t;

        let (gat_w, gat_h) = self.gat_dimensions.unwrap_or((gat.width, gat.height));
        let (gnd_w, gnd_h) = self.gnd_dimensions.unwrap_or((gat_w, gat_h));
        let gnd_cell_x = hit.x / zoom;
        let gnd_cell_y = hit.z / zoom;
        let cell_x = (gnd_cell_x * (gat_w as f32 / gnd_w as f32)) as i32;
        let cell_y = (gnd_cell_y * (gat_h as f32 / gnd_h as f32)) as i32;

        if cell_x < 0 || cell_y < 0 || cell_x >= gat_w || cell_y >= gat_h {
            return None;
        }
        Some((cell_x, cell_y))
    }

    fn handle_left_click(&mut self) {
        let (dest_x, dest_y) = match self.hovered_cell() {
            Some(c) => c,
            None => return,
        };
        let gat = match &self.gat {
            Some(g) => g,
            None => return,
        };

        if !gat.is_walkable(dest_x, dest_y) {
            return;
        }

        let (src_x, src_y) = self.player_entity.as_ref()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));

        let path = path_search(gat, src_x, src_y, dest_x as u16, dest_y as u16);
        if path.is_empty() {
            return;
        }

        if let Some(tx) = &self.network_cmd_tx {
            let packet = build_request_move_packet(dest_x as u16, dest_y as u16, self.config.packetver);
            let _ = tx.send(NetworkCommand::SendPacket(packet));
        }

        let elapsed = self.start_time.elapsed().as_secs_f32();
        if let Some(entity) = &mut self.player_entity {
            entity.movement.start_move(path, elapsed);
        }
    }

    fn spawn_network(&mut self) {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        self.network_cmd_tx = Some(cmd_tx);
        self.game_event_rx = Some(event_rx);

        let packetver = self.config.packetver;
        // Spawn on dedicated thread with single-threaded runtime
        // because network_loop uses non-Send packet types
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create network runtime");
            rt.block_on(network_loop(cmd_rx, event_tx, packetver));
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
                        self.login_session = Some(session);
                        let mut server_win = ServerListWindow::new(servers);
                        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
                            let mut all_loaded = true;
                            for path in ServerListWindow::grf_texture_paths() {
                                if renderer.texture_cache.get_or_load(
                                    path, grf, &renderer.device.device, &renderer.device.queue,
                                ).is_none() {
                                    all_loaded = false;
                                }
                            }
                            server_win.has_grf_textures = all_loaded;
                            if all_loaded {
                                server_win.set_texture_sizes(|name| {
                                    renderer.texture_cache.texture_size(name)
                                });
                            }
                        }
                        self.server_list_window = Some(server_win);
                        self.app_state = AppState::ServerSelect;
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
                            let mut all_loaded = true;
                            for path in CharSelectWindow::grf_texture_paths() {
                                if renderer.texture_cache.get_or_load(
                                    path, grf, &renderer.device.device, &renderer.device.queue,
                                ).is_none() {
                                    all_loaded = false;
                                }
                            }
                            char_win.has_grf_textures = all_loaded;
                            if all_loaded {
                                char_win.set_texture_sizes(|name| {
                                    renderer.texture_cache.texture_size(name)
                                });
                            }
                        }
                        self.char_select_window = Some(char_win);
                        self.app_state = AppState::CharacterSelect;
                    }
                    GameEvent::ZoneServerConnectInfo { char_id, map_name, ip, port } => {
                        if let Some(session) = &mut self.login_session {
                            session.store_zone_info(char_id, map_name);
                        }
                        let addr = format!("{}:{}", ip_u32_to_string(ip), port);
                        if let Some(tx) = &self.network_cmd_tx {
                            let _ = tx.send(NetworkCommand::Disconnect);
                            let _ = tx.send(NetworkCommand::Connect(addr));
                            if let Some(session) = &self.login_session {
                                let packet = build_zone_enter_packet(session);
                                let _ = tx.send(NetworkCommand::SendPacket(packet));
                            }
                        }
                    }
                    GameEvent::MapEntered { x, y, dir, .. } => {
                        let map_name = self.login_session.as_ref().map(|s| {
                            s.map_name.strip_suffix(".gat")
                                .unwrap_or(&s.map_name).to_string()
                        });
                        if let Some(map_name) = &map_name {
                            tracing::info!("Entering map: {map_name}");
                            self.load_map(map_name);
                            self.current_map = Some(map_name.clone());
                        }

                        let session_sex = self.login_session.as_ref().map(|s| s.sex).unwrap_or(1);
                        let (job, sex, head, hair_color, char_id) = self.selected_character.as_ref()
                            .map(|c| {
                                // Per-character sex only available on packetver >= 20141016;
                                // older versions default to 0, so use session sex instead
                                let sex = if self.config.packetver >= 20141016 { c.sex } else { session_sex };
                                (c.class, sex, c.head, c.hair_color, c.gid)
                            })
                            .unwrap_or((0, session_sex, 0, 0, 0));

                        let entity = Entity::new_player(char_id, job, sex, head, hair_color, x, y, dir);
                        self.player_entity = Some(entity);

                        self.load_player_sprite(job, sex, dir);

                        self.position_camera_at(x as f32, y as f32);
                        self.char_select_window = None;
                        self.app_state = AppState::InGame;
                    }
                    GameEvent::PlayerMoved { start_x, start_y, dest_x, dest_y, .. } => {
                        let already_moving_to_dest = self.player_entity.as_ref()
                            .filter(|e| e.movement.is_moving())
                            .and_then(|e| e.movement.destination())
                            .is_some_and(|(dx, dy)| dx == dest_x && dy == dest_y);
                        if !already_moving_to_dest {
                            if let Some(gat) = &self.gat {
                                let path = path_search(gat, start_x, start_y, dest_x, dest_y);
                                if !path.is_empty() {
                                    let elapsed = self.start_time.elapsed().as_secs_f32();
                                    if let Some(entity) = &mut self.player_entity {
                                        entity.movement.set_position(start_x as f32, start_y as f32);
                                        entity.movement.start_move(path, elapsed);
                                    }
                                }
                            }
                        }
                    }
                    GameEvent::MapChanged { map_name, x, y } => {
                        let map_name = map_name.strip_suffix(".gat")
                            .unwrap_or(&map_name).to_string();
                        if self.current_map.as_deref() != Some(&map_name) {
                            tracing::info!("Map change: {map_name}");
                            self.load_map(&map_name);
                            self.current_map = Some(map_name);
                        }
                        if let Some(entity) = &mut self.player_entity {
                            entity.movement.set_position(x as f32, y as f32);
                        }
                        self.position_camera_at(x as f32, y as f32);
                    }
                    GameEvent::Disconnected(reason) => {
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

    fn load_player_sprite(&mut self, job: u16, sex: u8, direction: u8) {
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };

        let base_path = body_sprite_path(job, sex);
        let spr_path = format!("{base_path}.spr");
        let act_path = format!("{base_path}.act");

        let spr_data = match grf.read_file(&spr_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Failed to read SPR {spr_path}: {e}");
                return;
            }
        };
        let spr = match SprFile::parse(&spr_data) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to parse SPR: {e}");
                return;
            }
        };

        let act_data = match grf.read_file(&act_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Failed to read ACT {act_path}: {e}");
                return;
            }
        };
        let act = match ActFile::parse(&act_data) {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("Failed to parse ACT: {e}");
                return;
            }
        };

        let textures = upload_sprite_textures(
            &spr,
            &renderer.device.device,
            &renderer.device.queue,
            &renderer.texture_cache.bind_group_layout,
        );

        tracing::info!("Loaded body sprite: {spr_path} ({} indexed + {} rgba, {} actions)",
            spr.indexed_sprites.len(), spr.rgba_sprites.len(), act.actions.len());

        self.player_sprite = Some(EntitySprite {
            textures,
            act,
            animation: SpriteAnimationState::new(direction),
        });
    }

    fn load_cursor_sprite(&mut self, grf: &ragnarok_formats::grf::GrfArchive) {
        let renderer = match &self.renderer {
            Some(r) => r,
            None => return,
        };

        let spr_path = "data/sprite/cursors.spr";
        let act_path = "data/sprite/cursors.act";

        let spr_data = match grf.read_file(spr_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Failed to read cursor SPR: {e}");
                return;
            }
        };
        let spr = match SprFile::parse(&spr_data) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to parse cursor SPR: {e}");
                return;
            }
        };

        let act_data = match grf.read_file(act_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Failed to read cursor ACT: {e}");
                return;
            }
        };
        let act = match ActFile::parse(&act_data) {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("Failed to parse cursor ACT: {e}");
                return;
            }
        };

        let textures = upload_sprite_textures(
            &spr,
            &renderer.device.device,
            &renderer.device.queue,
            &renderer.texture_cache.bind_group_layout,
        );

        tracing::info!("Loaded cursor sprite ({} indexed + {} rgba, {} actions)",
            spr.indexed_sprites.len(), spr.rgba_sprites.len(), act.actions.len());

        self.cursor_textures = Some(textures);
        self.cursor_act = Some(act);

        if let Some(window) = &self.window {
            window.set_cursor_visible(false);
        }
    }

    /// Convert GAT cell position to world coordinates (wx, wy, wz).
    fn cell_to_world(&self, cell_x: f32, cell_y: f32) -> Option<(f32, f32, f32)> {
        let zoom = self.map_zoom?;
        let (gat_w, gat_h) = self.gat_dimensions?;
        let (gnd_w, gnd_h) = self.gnd_dimensions.unwrap_or((gat_w, gat_h));

        let gnd_cell_x = cell_x * (gnd_w as f32 / gat_w as f32);
        let gnd_cell_y = cell_y * (gnd_h as f32 / gat_h as f32);

        let wx = (gnd_cell_x * zoom).clamp(0.0, gnd_w as f32 * zoom);
        let wz = (gnd_cell_y * zoom).clamp(0.0, gnd_h as f32 * zoom);
        Some((wx, 0.0, wz))
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
                                let _ = tx.send(NetworkCommand::Connect(addr));
                                if let Some(session) = &self.login_session {
                                    let packet = build_char_enter_packet(session);
                                    let _ = tx.send(NetworkCommand::SendPacket(packet));
                                }
                            }
                        }
                    }
                }
                GameEvent::RequestSelectCharacter { slot } => {
                    if let Some(char_win) = &self.char_select_window {
                        self.selected_character = char_win.characters.iter()
                            .find(|c| c.slot == slot as i8)
                            .cloned();
                    }
                    if let Some(tx) = &self.network_cmd_tx {
                        let packet = build_select_char_packet(slot, self.config.packetver);
                        let _ = tx.send(NetworkCommand::SendPacket(packet));
                    }
                }
                GameEvent::BackToServerSelect => {
                    self.app_state = AppState::ServerSelect;
                    self.char_select_window = None;
                    if let Some(tx) = &self.network_cmd_tx {
                        let _ = tx.send(NetworkCommand::Disconnect);
                    }
                }
                GameEvent::BackToLogin => {
                    self.app_state = AppState::Login;
                    self.server_list_window = None;
                    self.char_select_window = None;
                    self.login_session = None;
                    if let Some(tx) = &self.network_cmd_tx {
                        let _ = tx.send(NetworkCommand::Disconnect);
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

                        // Preload login UI textures
                        let mut all_loaded = true;
                        for path in LoginWindow::grf_texture_paths() {
                            if renderer.texture_cache.get_or_load(
                                path, &grf, &renderer.device.device, &renderer.device.queue,
                            ).is_none() {
                                all_loaded = false;
                            }
                        }
                        self.login_window.has_grf_textures = all_loaded;
                        if all_loaded {
                            self.login_window.set_texture_sizes(|name| {
                                renderer.texture_cache.texture_size(name)
                            });
                        }
                    }

                    self.load_cursor_sprite(&grf);
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
                if self.app_state == AppState::InGame {
                    match button {
                        MouseButton::Right => {
                            self.right_mouse_down = state == ElementState::Pressed;
                            if !self.right_mouse_down {
                                self.last_mouse_pos = None;
                            }
                        }
                        MouseButton::Left if state == ElementState::Pressed => {
                            self.handle_left_click();
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = (position.x, position.y);
                if self.app_state == AppState::InGame && self.right_mouse_down {
                    if let Some((lx, ly)) = self.last_mouse_pos {
                        let dx = (position.x - lx) as f32;
                        let dy = (position.y - ly) as f32;
                        if let Some(renderer) = &mut self.renderer {
                            renderer.camera.yaw += dx * 0.005;
                            if self.config.free_camera {
                                renderer.camera.pitch = (renderer.camera.pitch - dy * 0.005)
                                    .clamp(0.1, std::f32::consts::FRAC_PI_2 - 0.01);
                            }
                        }
                    }
                    self.last_mouse_pos = Some((position.x, position.y));
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.app_state == AppState::InGame {
                    let scroll = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 40.0,
                    };
                    if let Some(renderer) = &mut self.renderer {
                        renderer.camera.distance =
                            (renderer.camera.distance - scroll * 20.0).clamp(50.0, 1500.0);
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed
                    && event.physical_key == PhysicalKey::Code(KeyCode::F11)
                    && self.app_state == AppState::InGame
                {
                    if let Some(renderer) = &mut self.renderer {
                        if let Some(grid) = &mut renderer.grid_selector {
                            grid.show_grid = !grid.show_grid;
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let elapsed = self.start_time.elapsed().as_secs_f32();

                self.handle_game_events(event_loop);

                let (ui_draw_calls, ui_events) = match self.app_state {
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
                            (ui.draw_calls, events)
                        } else {
                            (Vec::new(), Vec::new())
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
                            (ui.draw_calls, events)
                        } else {
                            (Vec::new(), Vec::new())
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
                            (ui.draw_calls, events)
                        } else {
                            (Vec::new(), Vec::new())
                        }
                    }
                    AppState::InGame => (Vec::new(), Vec::new()),
                };
                self.handle_ui_events(ui_events, event_loop);

                // Movement tick: interpolate position and update camera
                let move_pos = self.player_entity.as_mut()
                    .filter(|e| e.movement.is_moving())
                    .map(|e| e.movement.update(elapsed));
                if let Some((px, py)) = move_pos {
                    self.position_camera_at(px, py);
                }

                // Update entity state and animation direction from movement
                if let Some(entity) = &mut self.player_entity {
                    entity.update_state();
                    if let Some(move_dir) = entity.movement.movement_direction() {
                        entity.direction = move_dir;
                    }
                }

                // Update sprite animation
                let dt = elapsed - self.last_render_time;
                self.last_render_time = elapsed;
                if let (Some(entity), Some(sprite), Some(renderer)) =
                    (&self.player_entity, &mut self.player_sprite, &self.renderer)
                {
                    let camera_dir = renderer.camera.direction_index();
                    sprite.animation.set_action(entity.action_index());
                    sprite.animation.set_direction(entity.direction);
                    sprite.animation.update(dt, &sprite.act, camera_dir);
                }

                // Update grid selector hover
                let hovered = if self.app_state == AppState::InGame {
                    self.hovered_cell()
                } else {
                    None
                };
                let hover_corners = hovered.and_then(|(cx, cy)| {
                    let c0 = self.cell_to_world(cx as f32, cy as f32)?;
                    let c1 = self.cell_to_world(cx as f32 + 1.0, cy as f32)?;
                    let c2 = self.cell_to_world(cx as f32, cy as f32 + 1.0)?;
                    let c3 = self.cell_to_world(cx as f32 + 1.0, cy as f32 + 1.0)?;
                    let gat = self.gat.as_ref()?;
                    let cell = &gat.cells[(cy * gat.width + cx) as usize];
                    let h = &cell.heights;
                    let y_off = -0.2_f32;
                    Some([
                        [c0.0, h[0] + y_off, c0.2],
                        [c1.0, h[1] + y_off, c1.2],
                        [c2.0, h[2] + y_off, c2.2],
                        [c3.0, h[3] + y_off, c3.2],
                    ])
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

                // Update cursor type based on hovered cell
                if self.app_state == AppState::InGame {
                    let cursor_type = match hovered {
                        Some((cx, cy)) => {
                            if self.gat.as_ref().is_some_and(|g| g.is_walkable(cx, cy)) {
                                CursorType::Default
                            } else {
                                CursorType::NoWalk
                            }
                        }
                        None => CursorType::Default,
                    };
                    self.cursor_animation.set_cursor_type(cursor_type);
                } else {
                    self.cursor_animation.set_cursor_type(CursorType::Default);
                }

                // Build owned sprite clip data (vertices, indices, texture index)
                let mut clip_data: Vec<(Vec<ragnarok_renderer::UiVertex>, Vec<u32>, usize)> = Vec::new();
                if let (Some(entity), Some(sprite), Some(renderer)) =
                    (&self.player_entity, &self.player_sprite, &self.renderer)
                {
                    let (cell_x, cell_y) = entity.movement.position();
                    if let Some((wx, wy, wz)) = self.cell_to_world(cell_x + 0.5, cell_y + 0.5) {
                        let screen_w = renderer.device.surface_config.width as f32;
                        let screen_h = renderer.device.surface_config.height as f32;
                        if let Some((sx, sy)) = renderer.camera.world_to_screen(wx, wy, wz, screen_w, screen_h) {
                            let camera_dir = renderer.camera.direction_index();
                            let action_idx = sprite.animation.action_index(&sprite.act, camera_dir);
                            let action = &sprite.act.actions[action_idx];

                            // Scale sprite based on perspective: pixels_per_world_unit * ground_zoom
                            // gives us how many screen pixels one cell occupies.
                            // Divide by a reference value to get a scale where 1.0 = correct size.
                            let ppu = renderer.camera.perspective_scale(wx, wy, wz, screen_h);
                            let ground_zoom = self.map_zoom.unwrap_or(5.0);
                            let sprite_scale = ppu * ground_zoom / 75.0;

                            if !action.motions.is_empty() {
                                let motion_idx = sprite.animation.motion_index() % action.motions.len();
                                let motion = &action.motions[motion_idx];
                                for clip in &motion.clips {
                                    if let Some((mut vertices, indices, tex_idx)) = build_clip_quad(clip, &sprite.textures, [sx, sy]) {
                                        if tex_idx < sprite.textures.bind_groups.len() {
                                            for v in &mut vertices {
                                                v.position[0] = sx + (v.position[0] - sx) * sprite_scale;
                                                v.position[1] = sy + (v.position[1] - sy) * sprite_scale;
                                            }
                                            clip_data.push((vertices, indices, tex_idx));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Build cursor sprite clip data
                let mut cursor_clip_data: Vec<(Vec<ragnarok_renderer::UiVertex>, Vec<u32>, usize)> = Vec::new();
                if let Some(cursor_act) = &self.cursor_act {
                    self.cursor_animation.update(dt, cursor_act);
                    let action_idx = self.cursor_animation.action_index();
                    if action_idx < cursor_act.actions.len() {
                        let action = &cursor_act.actions[action_idx];
                        if !action.motions.is_empty() {
                            let motion_idx = self.cursor_animation.motion_index() % action.motions.len();
                            let motion = &action.motions[motion_idx];
                            let (mx, my) = self.mouse_position;
                            if let Some(cursor_tex) = &self.cursor_textures {
                                for clip in &motion.clips {
                                    if let Some((vertices, indices, tex_idx)) = build_clip_quad(clip, cursor_tex, [mx as f32, my as f32]) {
                                        if tex_idx < cursor_tex.bind_groups.len() {
                                            cursor_clip_data.push((vertices, indices, tex_idx));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Create SpriteBatch references and render
                {
                    let mut sprite_batches: Vec<SpriteBatch> = Vec::new();

                    if let Some(sprite) = &self.player_sprite {
                        for (vertices, indices, tex_idx) in clip_data {
                            sprite_batches.push(SpriteBatch {
                                vertices,
                                indices,
                                texture: &sprite.textures.bind_groups[tex_idx],
                            });
                        }
                    }

                    if let Some(cursor_tex) = &self.cursor_textures {
                        for (vertices, indices, tex_idx) in cursor_clip_data {
                            sprite_batches.push(SpriteBatch {
                                vertices,
                                indices,
                                texture: &cursor_tex.bind_groups[tex_idx],
                            });
                        }
                    }

                    if let Some(renderer) = &mut self.renderer {
                        renderer.render(&ui_draw_calls, &sprite_batches, elapsed);
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

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(val) => return val,
            std::task::Poll::Pending => std::hint::spin_loop(),
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
