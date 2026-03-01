mod config;

use config::Config;
use ragnarok_formats::gat::GatFile;
use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsw::RswFile;
use ragnarok_game::event::GameEvent;
use ragnarok_network::{build_char_enter_packet, build_login_packet, build_select_char_packet, build_zone_enter_packet, ip_u32_to_string, network_loop, NetworkCommand};
use ragnarok_network::session::Session;
use ragnarok_renderer::Renderer;
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
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppState {
    Login,
    ServerSelect,
    CharacterSelect,
    InGame,
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
    map_zoom: Option<f32>,
    gat_dimensions: Option<(i32, i32)>,
    gnd_dimensions: Option<(i32, i32)>,
    current_map: Option<String>,
    start_time: Instant,
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
            map_zoom: None,
            gat_dimensions: None,
            gnd_dimensions: None,
            current_map: None,
            start_time: Instant::now(),
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

        // Load GAT for server→world coordinate mapping
        let gat_path = format!("data/{map_name}.gat");
        if let Ok(gat_data) = grf.read_file(&gat_path) {
            if let Ok(gat) = GatFile::parse(&gat_data) {
                self.gat_dimensions = Some((gat.width, gat.height));
            }
        }

        if let Some(renderer) = &mut self.renderer {
            renderer.load_map(&gnd, &rsw, grf);
        }
    }

    fn position_camera_at(&mut self, cell_x: f32, cell_y: f32) {
        if let (Some(zoom), Some(renderer)) = (self.map_zoom, &mut self.renderer) {
            // Server sends coordinates in GAT space; scale to GND world space
            let (mut wx, mut wz) = (cell_x * zoom, cell_y * zoom);
            if let (Some((gat_w, gat_h)), Some((gnd_w, gnd_h))) = (self.gat_dimensions, self.gnd_dimensions) {
                if gat_w != gnd_w || gat_h != gnd_h {
                    wx = cell_x * (gnd_w as f32 / gat_w as f32) * zoom;
                    wz = cell_y * (gnd_h as f32 / gat_h as f32) * zoom;
                }
            }
            // Clamp to GND bounds
            if let Some((gnd_w, gnd_h)) = self.gnd_dimensions {
                wx = wx.clamp(0.0, gnd_w as f32 * zoom);
                wz = wz.clamp(0.0, gnd_h as f32 * zoom);
            }
            renderer.camera.set_target(wx, 0.0, wz);
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
                    GameEvent::MapEntered { x, y, .. } => {
                        let map_name = self.login_session.as_ref().map(|s| {
                            s.map_name.strip_suffix(".gat")
                                .unwrap_or(&s.map_name).to_string()
                        });
                        if let Some(map_name) = &map_name {
                            tracing::info!("Entering map: {map_name}");
                            self.load_map(map_name);
                            self.current_map = Some(map_name.clone());
                        }
                        self.position_camera_at(x as f32, y as f32);
                        self.char_select_window = None;
                        self.app_state = AppState::InGame;
                    }
                    GameEvent::MapChanged { map_name, x, y } => {
                        let map_name = map_name.strip_suffix(".gat")
                            .unwrap_or(&map_name).to_string();
                        if self.current_map.as_deref() != Some(&map_name) {
                            tracing::info!("Map change: {map_name}");
                            self.load_map(&map_name);
                            self.current_map = Some(map_name);
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
        let renderer = pollster::block_on(Renderer::new(window.clone()));

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
                if self.app_state == AppState::InGame && button == MouseButton::Right {
                    self.right_mouse_down = state == ElementState::Pressed;
                    if !self.right_mouse_down {
                        self.last_mouse_pos = None;
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
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

                if let Some(renderer) = &mut self.renderer {
                    renderer.render(&ui_draw_calls, elapsed);
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
