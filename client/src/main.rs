mod command;
mod config;
mod events;
mod game_state;
mod game_updates;
mod hover;
mod input;
mod input_action;
mod overlay;
mod scene;
mod sound;
mod sprite;
mod ui;

use config::Config;
use game_state::{CursorInput, CursorPending, GameState, cursor_type_from_hover};
use input::InputState;
use models::enums::skill_enums::SkillEnum;
use ragnarok_audio::SoundManager;
use ragnarok_formats::act::SpriteAnimationState;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::app_state::AppState;
use ragnarok_game::cursor::RenderEntry;
use ragnarok_game::data_table::accessory_table::AccessoryTable;
use ragnarok_game::data_table::card_illustration_table::CardIllustrationTable;
use ragnarok_game::data_table::card_name_table::CardNameTable;
use ragnarok_game::data_table::item_description_table::ItemDescriptionTable;
use ragnarok_game::data_table::item_name_table::ItemNameTable;
use ragnarok_game::data_table::item_resource_table::ItemResourceTable;
use ragnarok_game::data_table::item_slot_count_table::ItemSlotCountTable;
use ragnarok_game::data_table::name_table::NameTable;
use ragnarok_game::data_table::skill_description_table::SkillDescriptionTable;
use ragnarok_game::data_table::skill_name_table::SkillNameTable;
use ragnarok_game::data_table::skill_tree_table::SkillTreeTable;
use ragnarok_game::data_table::skill_use_level_table::SkillUseLevelTable;
use ragnarok_game::effect::EffectQueue;
use ragnarok_game::event::GameEvent;
use ragnarok_game::map_loader;
use ragnarok_game::sound::SoundQueue;
use ragnarok_game::sprite_path::{HiddenRender, hidden_render};
use ragnarok_network::{NetworkCommand, build_reqname_packet, network_loop};
use ragnarok_renderer::effect::EffectHolder;
use ragnarok_renderer::{
    EffectSpriteCache, GridSelectorRenderer, Renderer, SpriteVertex, StrEffectCache, block_on,
};
use ragnarok_ui::context::UiContext;
use ragnarok_ui::frame::{UiFrame, UiOutput};
use ragnarok_ui::state::StateCache;
use ragnarok_ui_component::account::char_create_window::CharCreateWindow;
use ragnarok_ui_component::account::char_select_window::CharSelectWindow;
use ragnarok_ui_component::account::login_server_list_window::{
    LoginServerEntry, LoginServerListWindow,
};
use ragnarok_ui_component::account::login_window::{
    LoginFocus, LoginWindow, PASSWORD_ID, USERNAME_ID,
};
use ragnarok_ui_component::account::server_list_window::ServerListWindow;
use ragnarok_ui_component::game::confirm_dialog::ConfirmDialog;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

type ClipData = (Vec<SpriteVertex>, Vec<u32>, usize);

const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);

struct GameChannel {
    cmd_tx: Option<mpsc::UnboundedSender<NetworkCommand>>,
    event_rx: Option<mpsc::UnboundedReceiver<GameEvent>>,
}

impl GameChannel {
    fn new() -> Self {
        Self {
            cmd_tx: None,
            event_rx: None,
        }
    }

    fn send_packet(&self, packet: Vec<u8>) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(NetworkCommand::SendPacket(packet));
        }
    }

    fn send_cmd(&self, cmd: NetworkCommand) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(cmd);
        }
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        self.event_rx
            .as_mut()
            .map(|rx| std::iter::from_fn(|| rx.try_recv().ok()).collect())
            .unwrap_or_default()
    }
}

struct App {
    config: Config,
    saved_window_positions: HashMap<u32, [f32; 2]>,
    window_state_restored: bool,
    renderer: Option<Renderer>,
    effect_sprites: EffectSpriteCache,
    str_effects: StrEffectCache,
    effect_holder: EffectHolder,
    effect_queue: EffectQueue,
    map_fog: Option<ragnarok_formats::fog_table::FogEntry>,
    grf: Option<GrfArchive>,
    input: InputState,
    ui_context: Option<UiContext>,
    ui_state_cache: StateCache,
    login_window: LoginWindow,
    account_dialog: ConfirmDialog,
    login_server_list_window: Option<LoginServerListWindow>,
    selected_login_server: usize,
    active_packetver: u32,
    server_list_window: Option<ServerListWindow>,
    char_select_window: Option<CharSelectWindow>,
    char_create_window: Option<CharCreateWindow>,
    account_background: Option<String>,
    account_anims: HashMap<u32, SpriteAnimationState>,
    char_create_built_appearance: Option<(u16, u16)>,
    roulette_act: Option<ragnarok_formats::act::ActFile>,
    roulette_textures: Option<ragnarok_renderer::SpriteTextures>,
    channel: GameChannel,
    game: GameState,
    windows: ui::Windows,
    sound: SoundManager,
    sound_queue: SoundQueue,
    bgm_table: HashMap<String, String>,
    sfx_rng: u32,
    start_time: Instant,
    last_frame_instant: Instant,
    fps_smoothed: f32,
    next_frame: Instant,
    /// GameEvents produced by raw keyboard handling (skill-bar / emotion hotkeys),
    /// drained into `handle_ui_events` on the next redraw.
    pending_events: Vec<GameEvent>,
    window_focused: bool,
    profiler: ragnarok_profiling::Profiler,
    /// Window must be dropped last
    window: Option<Arc<Window>>,
}

impl App {
    fn new(config: Config) -> Self {
        let saved_window_positions = config
            .window_state
            .iter()
            .map(|(&id, entry)| (id, entry.position))
            .collect();
        let mut game = GameState::new();
        let mut windows = ui::Windows::new();
        windows.sound_options.set_values(
            config.bgm_volume,
            config.sfx_volume,
            config.bgm_enabled,
            config.sfx_enabled,
            config.custom.sound.stereo,
            config.custom.sound.play_when_unfocused,
        );
        windows
            .escape
            .set_excluded(&config.custom.window.exclude_close_via_esc);
        windows.item_info_window.wrap_title = config.custom.window.wrap_item_info_title;
        game.prefs.self_config.refuse_party_invite = config.refuse_party_invite;
        let mut effect_queue = EffectQueue::new();
        effect_queue.set_effects_enabled(config.show_skill_effects);
        let mut sound =
            SoundManager::new(config.effective_bgm_volume(), config.effective_sfx_volume());
        sound.set_stereo(config.custom.sound.stereo);
        Self {
            config,
            saved_window_positions,
            window_state_restored: false,
            window: None,
            renderer: None,
            effect_sprites: EffectSpriteCache::new(),
            str_effects: StrEffectCache::new(),
            effect_holder: EffectHolder::new(),
            effect_queue,
            map_fog: None,
            grf: None,
            input: InputState::new(),
            ui_context: None,
            ui_state_cache: StateCache::new(),
            login_window: LoginWindow::new(),
            account_dialog: ConfirmDialog::new(),
            login_server_list_window: None,
            selected_login_server: 0,
            active_packetver: 0,
            server_list_window: None,
            char_select_window: None,
            char_create_window: None,
            account_background: None,
            account_anims: HashMap::new(),
            char_create_built_appearance: None,
            roulette_act: None,
            roulette_textures: None,
            channel: GameChannel::new(),
            game,
            windows,
            sound,
            sound_queue: SoundQueue::new(),
            bgm_table: HashMap::new(),
            sfx_rng: 0x1234_5678,
            start_time: Instant::now(),
            last_frame_instant: Instant::now(),
            fps_smoothed: 0.0,
            next_frame: Instant::now(),
            pending_events: Vec::new(),
            window_focused: true,
            profiler: ragnarok_profiling::Profiler::default(),
        }
    }

    /// Show the connection-server selection screen when more than one server is
    /// configured; otherwise go straight to the login screen with the sole server.
    fn setup_initial_screen(&mut self) {
        self.select_login_server(0);
        if self.config.login_servers.len() <= 1 {
            self.game.session.app_state = AppState::Login;
            return;
        }
        let entries = self
            .config
            .login_servers
            .iter()
            .map(|s| LoginServerEntry {
                name: s.name.clone(),
                detail: format!("{}:{}", s.host, s.port),
            })
            .collect();
        let mut win = LoginServerListWindow::new(entries);
        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            events::preload_window(&mut win, renderer, grf);
        }
        self.login_server_list_window = Some(win);
        self.game.session.app_state = AppState::LoginServerSelect;
    }

    /// Adopt the connection server at `index`, resolving the packetver it speaks.
    fn select_login_server(&mut self, index: usize) {
        if let Some(server) = self.config.login_servers.get(index) {
            self.selected_login_server = index;
            self.active_packetver = server.packetver;
            self.channel
                .send_cmd(NetworkCommand::SetPacketver(self.active_packetver));
        }
    }

    fn load_map(&mut self, map_name: &str) {
        let grf = match &self.grf {
            Some(g) => g,
            None => return,
        };

        let map_data = match map_loader::load_map_data(grf, map_name) {
            Some(d) => d,
            None => {
                self.windows.map_missing_window.show(map_name.to_string());
                return;
            }
        };

        self.windows.map_missing_window.hide();
        self.game.session.map_coords = map_data.coordinates;
        let ground_sampler = map_data
            .gat
            .as_ref()
            .zip(map_data.coordinates.as_ref())
            .map(|(gat, coords)| ragnarok_game::map_cloud::ground_sampler(gat, coords));
        self.game.session.gat = map_data.gat;
        self.game.session.actor_lightmap = map_data.actor_lightmap;
        let was_indoor = self.game.session.camera_locked;
        self.game.session.camera_locked = map_data.indoor;
        if let Some(renderer) = &mut self.renderer {
            let [r, g, b] =
                ragnarok_game::data_table::map_cloud_table::map_background_color(map_name)
                    .unwrap_or([0.0, 0.0, 0.0]);
            renderer.clear_color = ragnarok_renderer::wgpu::Color {
                r: r as f64,
                g: g as f64,
                b: b as f64,
                a: 1.0,
            };
            let leaving = renderer.camera.saved_view();
            if was_indoor {
                self.game.session.saved_camera_indoor = leaving;
            } else {
                self.game.session.saved_camera_outdoor = leaving;
            }
            let restore = if map_data.indoor {
                self.game.session.saved_camera_indoor
            } else {
                self.game.session.saved_camera_outdoor
            };
            renderer.camera.on_map_enter(map_data.indoor, restore);
        }

        self.game
            .schedulers
            .ambient_effects
            .clear(&mut self.effect_queue);
        self.game.schedulers.ambient_effects =
            ragnarok_game::effects::AmbientEffectScheduler::from_rsw(&map_data.rsw, &map_data.gnd);
        self.game.schedulers.map_cloud.clear(&mut self.effect_queue);
        self.game.schedulers.map_cloud.set_map(map_name);
        self.effect_holder.set_ground_sampler(ground_sampler);
        self.game.schedulers.ambient_sounds =
            ragnarok_game::sound::ambient::AmbientSoundScheduler::from_rsw(
                &map_data.rsw,
                &map_data.gnd,
            );
        self.game.schedulers.repeat_sounds.clear();

        self.game.schedulers.day_night.on_map_loaded(
            map_data.rsw.light.diffuse.unwrap_or([1.0, 1.0, 1.0]),
            map_data.rsw.light.ambient.unwrap_or([0.3, 0.3, 0.3]),
        );

        let rsw_key = map_name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(map_name)
            .rsplit_once('.')
            .map(|(stem, _)| stem)
            .unwrap_or(map_name)
            .to_ascii_lowercase();
        let bgm_track = self.bgm_table.get(&format!("{rsw_key}.rsw")).cloned();

        let (mut spr_paths, mut str_names) =
            ragnarok_game::effects::ambient_effect_assets(&map_data.rsw);

        self.map_fog = map_data.fog;

        if let Some(renderer) = &mut self.renderer {
            let fog = if self.config.fog { map_data.fog } else { None };
            renderer.load_map(&map_data.gnd, &map_data.rsw, grf, fog);

            let effect_textures = ragnarok_game::effect::effect_texture_paths();
            renderer.preload_effect_textures(&effect_textures, grf);

            spr_paths.extend(ragnarok_game::effect::custom_effect_sprite_paths());
            spr_paths.extend(ragnarok_game::effect::effect_spr_paths());
            spr_paths.extend(ragnarok_game::effect::skill_unit_sprite_paths());
            spr_paths.sort();
            spr_paths.dedup();
            for path in spr_paths {
                self.effect_sprites.load(
                    path,
                    grf,
                    &renderer.device.device,
                    &renderer.device.queue,
                    &renderer.texture_cache.bind_group_layout,
                );
            }

            str_names.sort();
            str_names.dedup();
            for name in &str_names {
                self.str_effects.load(
                    name,
                    &[],
                    grf,
                    &mut renderer.texture_cache,
                    &renderer.device.device,
                    &renderer.device.queue,
                );
            }

            for aliases in ragnarok_game::effect::effect_str_names() {
                self.str_effects.load(
                    aliases[0],
                    &aliases[1..],
                    grf,
                    &mut renderer.texture_cache,
                    &renderer.device.device,
                    &renderer.device.queue,
                );
            }
        }

        if let Some(renderer) = &mut self.renderer
            && let Some(gat) = &self.game.session.gat
        {
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

        if let Some(track) = bgm_track {
            self.play_bgm_track(&track);
        }
    }

    fn position_camera_at(&mut self, cell_x: f32, cell_y: f32) {
        self.aim_camera_at(cell_x, cell_y, false);
    }

    /// Jump the camera onto a cell without the usual glide: on map entry and
    /// warps the old target is meaningless.
    fn warp_camera_to(&mut self, cell_x: f32, cell_y: f32) {
        self.aim_camera_at(cell_x, cell_y, true);
    }

    fn aim_camera_at(&mut self, cell_x: f32, cell_y: f32, snap: bool) {
        if let (Some(coords), Some(renderer)) = (&self.game.session.map_coords, &mut self.renderer)
        {
            input::position_camera_at(
                &mut renderer.camera,
                self.game.session.gat.as_ref(),
                coords,
                cell_x,
                cell_y,
            );
            if snap {
                renderer.camera.snap_target();
            }
        }
    }

    fn hovered_cell(&self) -> Option<(i32, i32)> {
        let (renderer, coords, screen_w, screen_h) = self.screen_dims()?;
        input::hovered_cell(
            self.input.mouse_position,
            &renderer.camera,
            screen_w,
            screen_h,
            coords,
            self.game.session.gat.as_ref(),
        )
    }

    pub(crate) fn is_gm_account(&self) -> bool {
        let account_id = self
            .game
            .session
            .login_session
            .as_ref()
            .map_or(0, |s| s.account_id);
        self.config.is_gm_account(account_id)
    }

    fn spawn_network(&mut self) {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        self.channel.cmd_tx = Some(cmd_tx);
        self.channel.event_rx = Some(event_rx);

        let packetver = self.active_packetver;
        let debug_delay_ms = self.config.debug_network_delay_ms;
        let start_time = self.start_time;
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
                start_time,
            ));
        });
    }

    fn build_ui(&mut self, elapsed: f32) -> (UiOutput, Vec<GameEvent>) {
        ragnarok_profiling::profile_function!();
        let now_ms = self.start_time.elapsed().as_millis() as u64;
        if let Some(ui_ctx) = &mut self.ui_context {
            ui_ctx.now_ms = now_ms;
        }
        let account_bg = self.account_background.clone();
        match self.game.session.app_state {
            AppState::LoginServerSelect => {
                if let (Some(ui_ctx), Some(renderer), Some(server_win)) = (
                    &self.ui_context,
                    &self.renderer,
                    &mut self.login_server_list_window,
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
                    ragnarok_ui_component::account::draw_background(&mut ui, account_bg.as_deref());
                    if self.account_dialog.state.is_some() {
                        ui.block_keyboard();
                    }
                    let events = server_win.build(&mut ui);
                    self.account_dialog.build(&mut ui);
                    (ui.finish(), events)
                } else {
                    (UiOutput::default(), Vec::new())
                }
            }
            AppState::Login => {
                if let (Some(ui_ctx), Some(renderer)) = (&self.ui_context, &self.renderer) {
                    let initial_focus = match self.login_window.focus {
                        LoginFocus::Username => Some(USERNAME_ID),
                        LoginFocus::Password => Some(PASSWORD_ID),
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
                    ragnarok_ui_component::account::draw_background(&mut ui, account_bg.as_deref());
                    if self.account_dialog.state.is_some() {
                        ui.block_keyboard();
                    }
                    let events = self.login_window.build(&mut ui);
                    self.account_dialog.build(&mut ui);
                    (ui.finish(), events)
                } else {
                    (UiOutput::default(), Vec::new())
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
                    ragnarok_ui_component::account::draw_background(&mut ui, account_bg.as_deref());
                    if self.account_dialog.state.is_some() {
                        ui.block_keyboard();
                    }
                    let events = server_win.build(&mut ui);
                    self.account_dialog.build(&mut ui);
                    (ui.finish(), events)
                } else {
                    (UiOutput::default(), Vec::new())
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
                    ragnarok_ui_component::account::draw_background(&mut ui, account_bg.as_deref());
                    if self.account_dialog.state.is_some() {
                        ui.block_keyboard();
                    }
                    let events = char_win.build(&mut ui);
                    self.account_dialog.build(&mut ui);
                    (ui.finish(), events)
                } else {
                    (UiOutput::default(), Vec::new())
                }
            }
            AppState::CharacterCreate => {
                if let (Some(ui_ctx), Some(renderer), Some(create_win)) = (
                    &self.ui_context,
                    &self.renderer,
                    &mut self.char_create_window,
                ) {
                    let mut ui = UiFrame::new(
                        ui_ctx,
                        &renderer.font_atlas,
                        &mut self.ui_state_cache,
                        elapsed,
                        create_win.has_grf_textures,
                        None,
                        &self.saved_window_positions,
                    );
                    ragnarok_ui_component::account::draw_background(&mut ui, account_bg.as_deref());
                    if self.account_dialog.state.is_some() {
                        ui.block_keyboard();
                    }
                    let events = create_win.build(&mut ui);
                    self.account_dialog.build(&mut ui);
                    (ui.finish(), events)
                } else {
                    (UiOutput::default(), Vec::new())
                }
            }
            AppState::InGame => {
                let render_list = self.compute_render_list();
                if let (Some(ui_ctx), Some(renderer)) = (&self.ui_context, &self.renderer) {
                    let mut ui = UiFrame::new(
                        ui_ctx,
                        &renderer.font_atlas,
                        &mut self.ui_state_cache,
                        elapsed,
                        self.windows.system_menu.has_grf_textures,
                        None,
                        &self.saved_window_positions,
                    );
                    let events = crate::ui::build_in_game_ui(
                        &mut self.game,
                        &mut self.windows,
                        &mut ui,
                        &|name| renderer.texture_cache.texture_size(name),
                        &render_list,
                    );

                    let mut overlay_lines: Vec<String> = Vec::new();
                    if self.game.show_fps {
                        overlay_lines.push(format!("fps: {:.0}", self.fps_smoothed));
                    }
                    if self.game.show_ping {
                        let local_ms = self.start_time.elapsed().as_millis() as u32;
                        let st = &self.game.session.server_time;
                        let est = st.estimated_server_tick(local_ms);
                        let offset = est as i64 - local_ms as i64;
                        overlay_lines.push(format!(
                            "net sync: {}",
                            if st.is_synced() { "yes" } else { "no" }
                        ));
                        overlay_lines.push(format!(
                            "rtt: {} ms (avg {:.0})",
                            st.rtt(),
                            st.rtt_avg()
                        ));
                        overlay_lines.push(format!("server tick est: {est}"));
                        overlay_lines.push(format!("offset: {offset} ms"));
                    }
                    if !overlay_lines.is_empty() {
                        const MINIMAP_LEFT_MARGIN: f32 = 130.0;
                        const PADDING: f32 = 10.0;
                        let color = [1.0, 0.9, 0.25, 1.0];
                        let shadow = [0.0, 0.0, 0.0, 0.9];
                        let right_x =
                            (ui.ctx.screen_width - MINIMAP_LEFT_MARGIN - PADDING).max(0.0);
                        for (i, line) in overlay_lines.iter().enumerate() {
                            let y = 10.0 + i as f32 * 16.0;
                            let tw = ui.atlas.measure_text(line);
                            let x = right_x - tw;
                            ui.text(x + 1.0, y + 1.0, line, shadow);
                            ui.text(x, y, line, color);
                        }
                    }

                    (ui.finish(), events)
                } else {
                    (UiOutput::default(), Vec::new())
                }
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
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.screen_width,
                self.config.screen_height,
            ));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        if self.config.fullscreen {
            window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
        }
        let os_scale = window.scale_factor() as f32;
        let dpi_scale = if self.config.dpi_scale > 0.0 {
            self.config.dpi_scale / 100.0
        } else {
            os_scale
        };
        let mut renderer = block_on(Renderer::new(
            window.clone(),
            self.config.font_px_height(),
            dpi_scale,
        ));
        renderer.set_fog_scale(self.config.custom.fog_scale);

        let physical_size = window.inner_size();
        self.window = Some(window);
        self.renderer = Some(renderer);
        let mut ui_ctx = UiContext::new(
            physical_size.width as f32 / dpi_scale,
            physical_size.height as f32 / dpi_scale,
        );
        ui_ctx.dpi_scale = dpi_scale;
        ui_ctx.lock_account_windows = true;
        self.ui_context = Some(ui_ctx);

        if !self.config.grf_paths.is_empty() {
            let data_dir = self.config.data_dir.as_deref().map(Path::new);
            match GrfArchive::open_layered(&self.config.grf_paths, data_dir) {
                Ok(grf) => {
                    println!(
                        "GRF loaded: {} ({} files, {} overlay archive(s), data_dir: {})",
                        self.config.grf_paths[0],
                        grf.file_count(),
                        self.config.grf_paths.len() - 1,
                        self.config.data_dir.as_deref().unwrap_or("none"),
                    );

                    if let Some(renderer) = &mut self.renderer {
                        renderer.try_load_grf_font(&grf);
                        events::preload_window(&mut self.login_window, renderer, &grf);
                        events::preload_window(&mut self.account_dialog, renderer, &grf);
                        self.account_background = pick_account_background(
                            &self.config.account_backgrounds,
                        )
                        .and_then(|path| {
                            renderer
                                .preload_textures(&[path.as_str()], &grf)
                                .then_some(path)
                        });
                    }
                    self.login_window.keep_id = self.config.keep_login_id;
                    if self.config.keep_login_id {
                        self.login_window.username.text = self.config.saved_username.clone();
                        self.login_window.focus = LoginFocus::Password;
                    }

                    self.load_cursor_sprite(&grf);
                    self.load_emotion_sprite(&grf);
                    self.load_status_overlay_sprites(&grf);
                    self.load_damage_sprites(&grf);
                    self.game.data_table.accessory = Some(AccessoryTable::load_from_grf(&grf));
                    self.game.data_table.name = Some(NameTable::load(&grf));
                    self.game.data_table.item_name = Some(ItemNameTable::load(&grf));
                    self.game.data_table.item_resource = Some(ItemResourceTable::load(&grf));
                    self.game.data_table.item_slot_count = Some(ItemSlotCountTable::load(&grf));
                    self.game.data_table.card_name = Some(CardNameTable::load(&grf));
                    self.game.data_table.card_illustration =
                        Some(CardIllustrationTable::load(&grf));
                    self.game.data_table.item_description = Some(ItemDescriptionTable::load(&grf));
                    self.game.data_table.skill_name = Some(SkillNameTable::load(&grf));
                    self.game.data_table.skill_description =
                        Some(SkillDescriptionTable::load(&grf));
                    self.game.data_table.skill_tree = Some(SkillTreeTable::load(&grf));
                    let mut skill_use_level = SkillUseLevelTable::load(&grf);
                    if self.config.custom.skill.al_teleport.separate_lvl {
                        skill_use_level.force_level_select(SkillEnum::AlTeleport);
                    }
                    self.game.data_table.skill_use_level = Some(skill_use_level);
                    self.game.data_table.quest_display = Some(
                        ragnarok_game::data_table::quest_display_table::QuestDisplayTable::load(
                            &grf,
                        ),
                    );
                    self.game.data_table.msg_string = Some(
                        ragnarok_game::data_table::msg_string_table::MsgStringTable::load(&grf),
                    );
                    self.game.data_table.map_position = Some(
                        ragnarok_game::data_table::map_position_table::MapPositionTable::load(&grf),
                    );
                    self.game.data_table.map_name =
                        Some(ragnarok_game::data_table::map_name_table::MapNameTable::load(&grf));
                    if let Ok(bytes) = grf.read_file(ragnarok_resources::table::PET_TALK) {
                        self.game.data_table.pet_talk =
                            Some(ragnarok_formats::pettalk::PetTalkTable::parse(&bytes));
                    }
                    if let Ok(bytes) = grf.read_file(ragnarok_resources::table::MP3_NAME) {
                        let text = String::from_utf8_lossy(&bytes);
                        self.bgm_table =
                            ragnarok_game::sound::bgm_table::parse_mp3_name_table(&text);
                    }
                    self.grf = Some(grf);
                }
                Err(e) => {
                    tracing::error!("Failed to open GRF {}: {e}", self.config.grf_paths[0]);
                }
            }
        }

        self.setup_initial_screen();
        self.spawn_network();
        self.play_bgm_track("01.mp3");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(ui_ctx) = &mut self.ui_context {
            ui_ctx.handle_event(&event);
        }

        match event {
            WindowEvent::CloseRequested => self.handle_close_requested(event_loop),
            WindowEvent::Resized(size) => self.handle_resize(size),
            WindowEvent::Focused(focused) => self.handle_focus_changed(focused),
            WindowEvent::MouseInput { state, button, .. } => self.handle_mouse_input(state, button),
            WindowEvent::CursorMoved { position, .. } => self.handle_cursor_moved(position),
            WindowEvent::MouseWheel { delta, .. } => self.handle_mouse_wheel(delta),
            WindowEvent::KeyboardInput { event, .. } => self.handle_keyboard_input(event),
            WindowEvent::ModifiersChanged(modifiers) => self.handle_modifiers_changed(modifiers),
            WindowEvent::RedrawRequested => {
                self.profiler.new_frame();
                ragnarok_profiling::profile_scope!("frame");
                let elapsed = self.start_time.elapsed().as_secs_f32();

                self.handle_game_events(event_loop);

                let (
                    UiOutput {
                        draw_calls: ui_draw_calls,
                        tooltip_draw_calls,
                        any_hovered: ui_any_hovered,
                        any_interactive_hovered: ui_any_interactive,
                    },
                    ui_events,
                ) = self.build_ui(elapsed);
                self.input.ui_hovered = ui_any_hovered;
                if let Some(dirty) = self.windows.hotkey_config_window.take_dirty_bindings() {
                    self.config.keybindings = dirty.interface;
                    self.config.emotion_keys = dirty.emotion;
                    self.config.save("config.json");
                }
                let mut queued = std::mem::take(&mut self.pending_events);
                queued.extend(ui_events);
                self.handle_ui_events(queued, event_loop);

                if self.game.session.pending_disconnect_exit {
                    event_loop.exit();
                }
                let now = Instant::now();
                let raw_delta = now.duration_since(self.last_frame_instant).as_secs_f32();
                self.last_frame_instant = now;
                if raw_delta > 0.0 {
                    let instant_fps = 1.0 / raw_delta;
                    self.fps_smoothed = if self.fps_smoothed == 0.0 {
                        instant_fps
                    } else {
                        self.fps_smoothed * 0.9 + instant_fps * 0.1
                    };
                }
                let delta = raw_delta.min(0.1);
                self.run_game_updates(delta, elapsed);
                self.drain_sound_queue(delta);

                let hovered = self.update_grid_hover();
                let render_list = self.compute_render_list();
                let floor_item_render_list = self.compute_floor_item_render_list(elapsed);
                let mut cart_render_list = self.compute_cart_render_list();
                cart_render_list.extend(self.compute_falcon_render_list());
                // A stealthed actor the local player can't see is not hoverable or
                // attackable: drop it before hit-testing (self stays out of picking
                // regardless, so its shadow-only self view never enters here).
                let pick_render_list: Vec<RenderEntry> = render_list
                    .iter()
                    .filter(|entry| {
                        self.game.world.entities.get(entry.id).is_none_or(|e| {
                            hidden_render(
                                e.effect_state,
                                self.hidden_viewer_for(entry.id),
                                self.player_clairvoyant(),
                            ) != HiddenRender::Skip
                        })
                    })
                    .copied()
                    .collect();
                self.prune_companion_targets();
                let hover = self.resolve_hover(
                    hovered,
                    &pick_render_list,
                    &floor_item_render_list,
                    ui_any_hovered,
                    ui_any_interactive,
                );
                let cursor = cursor_type_from_hover(
                    &hover,
                    CursorInput {
                        in_game: self.game.session.app_state == AppState::InGame,
                        right_mouse_down: self.input.right_mouse_down,
                        ui_any_hovered,
                        ui_any_interactive_hovered: ui_any_interactive,
                        item_drag_active: ragnarok_ui::frame::drag_active(&self.ui_state_cache),
                    },
                    CursorPending {
                        companion_target_armed: self
                            .game
                            .companions
                            .companion_attack_target
                            .iter()
                            .any(Option::is_some),
                        pending_companion_skill: self
                            .game
                            .pending_casts
                            .pending_companion_skill
                            .is_some(),
                        pending_companion_patrol: self
                            .game
                            .pending_casts
                            .pending_companion_patrol
                            .is_some(),
                        capture_targeting: self.game.companions.capture_targeting,
                        pending_skill: self.game.pending_casts.pending_skill_target.is_some(),
                        marriage_targeting: self.game.pending_casts.marriage_targeting,
                    },
                );
                self.game.assets.cursor_animation.set_cursor_type(cursor);
                self.game.hover = hover;

                let hovered_named_id = self
                    .game
                    .hover
                    .target_id()
                    .or(self.game.hover.hovered_player_id)
                    .or(self.game.hover.hovered_self_id);
                let hovered_floor_item_id = self.game.hover.hovered_floor_item_id;
                if let Some(entity_id) = hovered_named_id
                    && let Some(entity) = self.game.world.entities.get_mut(entity_id)
                    && !entity.name_requested
                {
                    entity.name_requested = true;
                    self.channel
                        .send_packet(build_reqname_packet(entity_id, self.active_packetver));
                }

                let cursor_clips =
                    self.build_cursor_sprite_clips(delta, &render_list, &floor_item_render_list);
                let lock_cursor_clips = self.build_lock_cursor_clips(delta, &render_list);

                let world_overlay_calls = self.build_world_overlays(
                    &render_list,
                    &floor_item_render_list,
                    hovered_named_id,
                    hovered_floor_item_id,
                );
                let skill_level_calls = self.build_skill_overlay();

                self.compose_and_render(
                    &render_list,
                    &floor_item_render_list,
                    &cart_render_list,
                    elapsed,
                    delta,
                    cursor_clips,
                    lock_cursor_clips,
                    world_overlay_calls,
                    skill_level_calls,
                    ui_draw_calls,
                    tooltip_draw_calls,
                );

                if let Some(ui_ctx) = &mut self.ui_context {
                    ui_ctx.begin_frame();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_frame {
            self.next_frame = now + FRAME_INTERVAL;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }
}

fn pick_account_background(paths: &[String]) -> Option<String> {
    if paths.is_empty() {
        return None;
    }
    let idx = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0)
        % paths.len();
    Some(paths[idx].clone())
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::load_or_default("config.json");
    ragnarok_profiling::debug::init(
        config.debug.trace_packet,
        config.debug.trace_effects,
        config.debug.trace_input,
        config.debug.trace_texture_load,
    );

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(config);
    event_loop.run_app(&mut app).unwrap();
}
