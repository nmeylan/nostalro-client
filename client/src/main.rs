mod config;
mod events;
mod game_state;
mod game_updates;
mod input;
mod input_action;
mod overlay;
mod scene;
mod sprite;

use config::Config;
use game_state::GameState;
use input::InputState;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::app_state::AppState;
use ragnarok_game::cursor::{
    CursorType, PendingSkillTarget, RenderEntry, RenderEntryKind, cursor_type_for_cell,
    hovered_entity_cursor_type,
};
use ragnarok_game::entity::EntityState;
use ragnarok_game::event::GameEvent;
use ragnarok_game::skill::SkillTargetType;
use models::enums::skill_enums::SkillEnum;
use ragnarok_game::targeting::{skill_target_class, TargetClass};
use ragnarok_game::{map_loader, sprite_loader};
use ragnarok_network::{
    KeepaliveMode, NetworkCommand, build_action_request_packet, build_card_composition_list_packet,
    build_card_composition_packet, build_char_enter_packet, build_chat_packet,
    build_contact_npc_packet, build_drop_item_packet, build_equip_item_packet, build_login_packet,
    build_npc_close_packet, build_npc_deal_type_packet, build_npc_input_number_packet,
    build_npc_input_string_packet, build_npc_menu_select_packet, build_npc_next_packet,
    build_cartoff_packet, build_change_cart_packet, build_move_item_body_to_cart_packet, build_move_item_cart_to_body_packet,
    build_move_item_cart_to_store_packet, build_move_item_store_to_cart_packet,
    build_pickup_item_packet, build_purchase_item_list_packet, build_remove_option_packet,
    build_req_enter_room_packet, build_reqname_packet, build_restart_packet, build_select_char_packet,
    build_select_warppoint_packet, build_sell_item_list_packet,
    build_shortcut_key_change_packet, build_stat_change_packet,
    build_unequip_item_packet, build_upgrade_skill_packet, build_use_item_packet, build_use_skill_packet,
    ip_u32_to_string, network_loop,
};
use ragnarok_game::effect::EffectQueue;
use ragnarok_renderer::{
    EffectSpriteCache, GridSelectorRenderer, Renderer, SpriteVertex, StrEffectCache, UiDrawCall,
    block_on, upload_sprite_textures,
};
use ragnarok_renderer::effect::EffectHolder;
use ragnarok_ui::context::UiContext;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::state::StateCache;
use ragnarok_ui_component::account::char_select_window::CharSelectWindow;
use ragnarok_ui_component::account::login_window::{LoginFocus, LoginWindow};
use ragnarok_ui_component::account::server_list_window::ServerListWindow;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};
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

type ClipData = (Vec<SpriteVertex>, Vec<u32>, usize);

/// ~60 FPS render cadence; the loop sleeps (`ControlFlow::WaitUntil`) between
/// frames instead of spinning so an idle client doesn't peg a CPU core.
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
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    effect_sprites: EffectSpriteCache,
    str_effects: StrEffectCache,
    /// Runtime store for every live effect — skill/level-up/refining/custom
    /// effects and the RSW ambient effects (torch/smoke/bubble/…), all spawned
    /// through the queue.
    effect_holder: EffectHolder,
    /// Queue triggers push spawn requests into; drained each frame.
    effect_queue: EffectQueue,
    grf: Option<GrfArchive>,
    input: InputState,
    ui_context: Option<UiContext>,
    ui_state_cache: StateCache,
    login_window: LoginWindow,
    server_list_window: Option<ServerListWindow>,
    char_select_window: Option<CharSelectWindow>,
    channel: GameChannel,
    game: GameState,
    start_time: Instant,
    last_frame_instant: Instant,
    /// Deadline for the next redraw; the event loop waits until this point
    /// instead of spinning.
    next_frame: Instant,
}

impl App {
    fn new(config: Config) -> Self {
        let saved_window_positions = config
            .window_state
            .iter()
            .map(|(&id, entry)| (id, entry.position))
            .collect();
        let mut game = GameState::new();
        game.debug_overlay = config.debug_overlay;
        Self {
            config,
            saved_window_positions,
            window: None,
            renderer: None,
            effect_sprites: EffectSpriteCache::new(),
            str_effects: StrEffectCache::new(),
            effect_holder: EffectHolder::new(),
            effect_queue: EffectQueue::new(),
            grf: None,
            input: InputState::new(),
            ui_context: None,
            ui_state_cache: StateCache::new(),
            login_window: LoginWindow::new(),
            server_list_window: None,
            char_select_window: None,
            channel: GameChannel::new(),
            game,
            start_time: Instant::now(),
            last_frame_instant: Instant::now(),
            next_frame: Instant::now(),
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

        // Tear down the previous map's ambient effects, then rebuild from the
        // new RSW. Spawning is driven per-frame by the scheduler (near camera).
        self.game.ambient_effects.clear(&mut self.effect_queue);
        self.game.ambient_effects =
            ragnarok_game::effects::AmbientEffectScheduler::from_rsw(&map_data.rsw, &map_data.gnd);

        // Resolve each RSW ambient effect to its spec so we can preload the
        // SPR/STR assets it will draw through the holder.
        let (mut spr_paths, mut str_names) =
            ragnarok_game::effects::ambient_effect_assets(&map_data.rsw);

        if let Some(renderer) = &mut self.renderer {
            let fog = if self.config.fog { map_data.fog } else { None };
            renderer.load_map(&map_data.gnd, &map_data.rsw, grf, fog);

            let effect_textures = ragnarok_game::effect::effect_texture_paths();
            renderer.preload_effect_textures(&effect_textures, grf);

            // Sprite paths used by Custom-effect modules (Hit's particle1, etc.)
            // — same loader path as the RSW ambient emitter sprites.
            spr_paths.extend(ragnarok_game::effect::custom_effect_sprite_paths());
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

            // Skill / ground-unit STR effects (Fire Wall, casting glyphs, …):
            // preload every STR-rendered effect so a triggered effect's STR is
            // in the cache when it spawns. The ambient list above only covers
            // RSW emitters, so without this skill STRs never draw.
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
            && let Some(gat) = &self.game.gat
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

    fn spawn_network(&mut self) {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        self.channel.cmd_tx = Some(cmd_tx);
        self.channel.event_rx = Some(event_rx);

        let packetver = self.config.packetver;
        let debug_delay_ms = self.config.debug_network_delay_ms;
        let trace_packets_send = self.config.trace_packets_send;
        let trace_packets_recv = self.config.trace_packets_recv;
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
                trace_packets_send,
                trace_packets_recv,
            ));
        });
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
            .data_table
            .item_name
            .as_ref()
            .map(|t| t.get_name_or_id_for(item_id, is_identified))
            .unwrap_or_else(|| format!("Item #{item_id}"));
        let resource_name = self
            .game
            .data_table
            .item_resource
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
        if let Some(res_name) = &resource_name
            && let (Some(grf), Some(renderer)) = (&self.grf, &self.renderer)
        {
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

    fn handle_ui_events(&mut self, events: Vec<GameEvent>, event_loop: &ActiveEventLoop) {
        for event in events {
            match event {
                GameEvent::RequestLogin { username, password } => {
                    let addr = format!("{}:{}", self.config.login_ip, self.config.login_port);
                    self.channel.send_cmd(NetworkCommand::Connect(addr));
                    self.channel.send_packet(build_login_packet(
                        &username,
                        &password,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestSelectServer { index } => {
                    if let Some(server_win) = &self.server_list_window
                        && let Some(server) = server_win.servers.get(index)
                    {
                        let addr = format!("{}:{}", ip_u32_to_string(server.ip), server.port);
                        self.channel.send_cmd(NetworkCommand::Disconnect);
                        self.channel.send_cmd(NetworkCommand::Connect(addr.clone()));
                        if let Some(session) = &mut self.game.login_session {
                            session.char_server_addr = Some(addr);
                            self.channel.send_packet(build_char_enter_packet(session));
                            self.channel.send_cmd(NetworkCommand::SetKeepalive(
                                KeepaliveMode::CharServer {
                                    account_id: session.account_id,
                                },
                            ));
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
                    self.channel
                        .send_packet(build_select_char_packet(slot, self.config.packetver));
                }
                GameEvent::BackToServerSelect => {
                    self.game.app_state = AppState::ServerSelect;
                    self.char_select_window = None;
                    self.game.system_menu.open = false;
                    self.channel.send_cmd(NetworkCommand::Disconnect);
                }
                GameEvent::BackToLogin => {
                    self.game.app_state = AppState::Login;
                    self.server_list_window = None;
                    self.char_select_window = None;
                    self.game.login_session = None;
                    self.game.system_menu.open = false;
                    self.channel.send_cmd(NetworkCommand::Disconnect);
                }
                GameEvent::BackToCharacterSelect => {
                    self.game.system_menu.open = false;
                    self.channel
                        .send_packet(build_restart_packet(self.config.packetver));
                }
                GameEvent::QuitGame => {
                    self.channel.send_cmd(NetworkCommand::Disconnect);
                    event_loop.exit();
                }
                GameEvent::RequestNpcContact { npc_id } => {
                    self.channel
                        .send_packet(build_contact_npc_packet(npc_id, self.config.packetver));
                }
                GameEvent::RequestNpcNext { npc_id } => {
                    self.channel
                        .send_packet(build_npc_next_packet(npc_id, self.config.packetver));
                }
                GameEvent::RequestNpcClose { npc_id } => {
                    self.channel
                        .send_packet(build_npc_close_packet(npc_id, self.config.packetver));
                }
                GameEvent::RequestNpcMenuSelect { npc_id, choice } => {
                    self.channel.send_packet(build_npc_menu_select_packet(
                        npc_id,
                        choice,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestJoinChatRoom { room_id } => {
                    self.channel
                        .send_packet(build_req_enter_room_packet(room_id, self.config.packetver));
                }
                GameEvent::RequestSelectWarppoint { skill_id, map_name } => {
                    self.channel.send_packet(build_select_warppoint_packet(
                        skill_id,
                        &map_name,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestNpcInputNumber { npc_id, value } => {
                    self.channel.send_packet(build_npc_input_number_packet(
                        npc_id,
                        value,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestNpcInputString { npc_id, text } => {
                    self.channel.send_packet(build_npc_input_string_packet(
                        npc_id,
                        &text,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestNpcDealType { npc_id, deal_type } => {
                    self.channel.send_packet(build_npc_deal_type_packet(
                        npc_id,
                        deal_type,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestNpcShopBuy { items } => {
                    self.channel.send_packet(build_purchase_item_list_packet(
                        &items,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestNpcShopSell { items } => {
                    self.channel
                        .send_packet(build_sell_item_list_packet(&items, self.config.packetver));
                }
                GameEvent::RequestNpcShopClose => {
                    match self.game.npc_shop.shop.mode {
                        Some(ragnarok_game::npc_shop::NpcShopMode::Buy) => {
                            self.channel.send_packet(build_purchase_item_list_packet(
                                &[],
                                self.config.packetver,
                            ));
                        }
                        Some(ragnarok_game::npc_shop::NpcShopMode::Sell) => {
                            self.channel.send_packet(build_sell_item_list_packet(
                                &[],
                                self.config.packetver,
                            ));
                        }
                        None => {}
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
                    self.game
                        .item_info_window
                        .show_card(item_id, &self.game.data_table);
                    let tex_paths = self.game.item_info_window.pending_card_texture_paths();
                    self.preload_item_icons(tex_paths);
                }
                GameEvent::ShowCardIllustration { item_id } => {
                    let name = self
                        .game
                        .data_table
                        .item_name
                        .as_ref()
                        .map(|t| t.get_name_or_id(item_id))
                        .unwrap_or_else(|| format!("Item #{item_id}"));
                    let illust_path = self
                        .game
                        .data_table
                        .card_illustration
                        .as_ref()
                        .and_then(|t| t.illustration_path(item_id));
                    if let Some(path) = illust_path {
                        self.game
                            .item_info_window
                            .show_illustration(item_id, name, path);
                        let tex_paths = self
                            .game
                            .item_info_window
                            .pending_illustration_texture_paths();
                        self.preload_item_icons(tex_paths);
                    }
                }
                GameEvent::RequestUseItem { index } => {
                    let account_id = self
                        .game
                        .login_session
                        .as_ref()
                        .map(|s| s.account_id)
                        .unwrap_or(0);
                    self.channel.send_packet(build_use_item_packet(
                        index,
                        account_id,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestEquipItem { index, location } => {
                    self.channel.send_packet(build_equip_item_packet(
                        index,
                        location,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestUnequipItem { index } => {
                    self.channel
                        .send_packet(build_unequip_item_packet(index, self.config.packetver));
                }
                GameEvent::RequestDropItem { index, count } => {
                    self.channel.send_packet(build_drop_item_packet(
                        index,
                        count,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestRemoveOption => {
                    self.channel
                        .send_packet(build_remove_option_packet(self.config.packetver));
                }
                GameEvent::RequestMoveItemBodyToCart { index, count } => {
                    self.channel.send_packet(build_move_item_body_to_cart_packet(
                        index,
                        count,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestMoveItemCartToBody { index, count } => {
                    self.channel.send_packet(build_move_item_cart_to_body_packet(
                        index,
                        count,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestMoveItemStoreToCart { index, count } => {
                    self.channel.send_packet(build_move_item_store_to_cart_packet(
                        index,
                        count,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestMoveItemCartToStore { index, count } => {
                    self.channel.send_packet(build_move_item_cart_to_store_packet(
                        index,
                        count,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestCartOff => {
                    self.channel
                        .send_packet(build_cartoff_packet(self.config.packetver));
                }
                GameEvent::RequestChangeCart { num } => {
                    self.channel
                        .send_packet(build_change_cart_packet(num, self.config.packetver));
                }
                GameEvent::RequestSetCartPick { .. } => {
                    // Cart auto-pickup settings packet is not yet defined on the
                    // server protocol; the toggles are tracked client-side only.
                }
                GameEvent::ToggleCart => {
                    self.game.character.cart.toggle();
                }
                GameEvent::RequestSkillLevelUp { skill_id } => {
                    self.channel
                        .send_packet(build_upgrade_skill_packet(skill_id, self.config.packetver));
                }
                GameEvent::RequestStatChange { status_id, amount } => {
                    self.channel.send_packet(build_stat_change_packet(status_id, amount, self.config.packetver));
                }
                GameEvent::HotkeyListReceived { slots } => {
                    self.game
                        .character
                        .hotkeys
                        .set_from_server(&slots, self.game.character.inventory.all_items());
                }
                GameEvent::RequestHotkeyChange {
                    index,
                    is_skill,
                    id,
                    count,
                } => {
                    let is_skill_i8 = if is_skill { 1i8 } else { 0i8 };
                    self.channel.send_packet(build_shortcut_key_change_packet(
                        index,
                        is_skill_i8,
                        id,
                        count,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestUseSkill { skill_id, level } => {
                    // Change Cart opens the cart-model picker client-side instead
                    // of casting (the server treats the skill itself as a no-op);
                    // the chosen model is sent as its own request.
                    if skill_id == SkillEnum::McChangecart.id() as u16 {
                        if self.game.character.cart_design.is_some() {
                            self.preload_cart_previews(&[1, 2, 3, 4, 5]);
                            self.game.cart_select_window.open();
                        }
                        continue;
                    }
                    let skill_target_type = self
                        .game
                        .character
                        .skills
                        .get_skill(skill_id)
                        .map(|s| s.skill_target_type)
                        .unwrap_or(SkillTargetType::Target);
                    match skill_target_type {
                        SkillTargetType::MySelf => {
                            if !self.skill_on_cooldown(skill_id) {
                                let target_id = self.game.entities.player_id().unwrap_or(0);
                                self.channel.send_packet(build_use_skill_packet(
                                    skill_id,
                                    level,
                                    target_id,
                                    self.config.packetver,
                                ));
                            }
                        }
                        SkillTargetType::Target | SkillTargetType::Friend => {
                            self.game.pending_skill_target =
                                Some(PendingSkillTarget::Entity { skill_id, level });
                            self.game.pending_skill_id = Some(skill_id);
                            self.game.pending_skill_level = Some(level);
                        }
                        SkillTargetType::Ground | SkillTargetType::Trap => {
                            self.game.pending_skill_target =
                                Some(PendingSkillTarget::Ground { skill_id, level });
                        }
                        _ => {
                            tracing::debug!(
                                "Skill target type {:?} not yet supported for skill {skill_id}",
                                skill_target_type
                            );
                        }
                    }
                }
                GameEvent::RequestPickupItem { id } => {
                    self.channel
                        .send_packet(build_pickup_item_packet(id, self.config.packetver));
                    if let Some(entity) = self.game.entities.player_mut() {
                        entity.enter_pickup(0.5);
                    }
                }
                GameEvent::RequestCardInsertList { card_index } => {
                    self.game.pending_card_composition_index = Some(card_index);
                    self.channel.send_packet(build_card_composition_list_packet(
                        card_index,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestCardInsert {
                    card_index,
                    equip_index,
                } => {
                    self.channel.send_packet(build_card_composition_packet(
                        card_index,
                        equip_index,
                        self.config.packetver,
                    ));
                    self.game.pending_card_composition_index = None;
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
                        self.channel
                            .send_packet(build_chat_packet(&full_msg, self.config.packetver));
                    }
                }
                GameEvent::Disconnected(ref reason) if reason == "User exit" => {
                    self.channel.send_cmd(NetworkCommand::Disconnect);
                    event_loop.exit();
                }
                GameEvent::ToggleInventory => {
                    self.game.character.inventory.toggle();
                }
                GameEvent::ToggleEquipment => {
                    self.game.equipment_window.toggle();
                }
                GameEvent::ToggleSkills => {
                    self.game.character.skills.toggle();
                }
                GameEvent::ToggleStatusWindow => {
                    self.game.status_window.toggle();
                }
                GameEvent::ToggleMinimap => {
                    self.game.minimap_window.cycle_visibility();
                }
                _ => {}
            }
        }
    }

    fn reconnect_to_char_server(&mut self) -> bool {
        if self.channel.cmd_tx.is_none() {
            return false;
        }
        let Some(session) = &self.game.login_session else {
            return false;
        };
        let Some(addr) = &session.char_server_addr else {
            return false;
        };
        self.channel.send_cmd(NetworkCommand::Disconnect);
        self.channel.send_cmd(NetworkCommand::Connect(addr.clone()));
        self.channel.send_packet(build_char_enter_packet(session));
        self.channel
            .send_cmd(NetworkCommand::SetKeepalive(KeepaliveMode::CharServer {
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
                    self.channel.send_packet(build_action_request_packet(
                        0,
                        action,
                        self.config.packetver,
                    ));
                }
            }
            "/doridori" => {
                if let Some(entity) = self.game.entities.player_mut() {
                    entity.head_dir = if entity.head_dir == 0 { 1 } else { 0 };
                }
            }
            "/noshift" | "/ns" => {
                self.game.noshift_mode = !self.game.noshift_mode;
                let status = if self.game.noshift_mode { "ON" } else { "OFF" };
                self.game
                    .chat_window
                    .add_system(format!("No-shift mode: {status}"));
            }
            "/noctrl" | "/nc" => {
                self.game.noctrl_mode = !self.game.noctrl_mode;
                let status = if self.game.noctrl_mode { "ON" } else { "OFF" };
                self.game
                    .chat_window
                    .add_system(format!("No-ctrl mode: {status}"));
            }
            "/where" => {
                match (self.game.current_map.as_ref(), self.game.entities.player()) {
                    (Some(map_name), Some(player)) => {
                        let (x, y) = player.movement.cell_position();
                        let message = format!("{map_name}.gat ({x}, {y})");
                        self.game.chat_window.add_system(message);
                    }
                    _ => {
                        self.game
                            .chat_window
                            .add_system("You are not in a map yet.".to_string());
                    }
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
                // Chat-room boxes follow their owner entity, so the UI needs the
                // projected screen anchors. Computed here (read-only) before the
                // mutable UI build borrow.
                let render_list = self.compute_render_list();
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
                        &render_list,
                    );

                    if self.game.debug_overlay {
                        let local_ms = self.start_time.elapsed().as_millis() as u32;
                        let st = &self.game.server_time;
                        let est = st.estimated_server_tick(local_ms);
                        let offset = est as i64 - local_ms as i64;
                        let color = [0.5, 1.0, 0.6, 1.0];
                        let lines = [
                            format!("net sync: {}", if st.is_synced() { "yes" } else { "no" }),
                            format!("rtt: {} ms (avg {:.0})", st.rtt(), st.rtt_avg()),
                            format!("server tick est: {est}"),
                            format!("offset: {offset} ms"),
                        ];
                        for (i, line) in lines.iter().enumerate() {
                            ui.text(10.0, 10.0 + i as f32 * 16.0, line, color);
                        }
                    }

                    let any_hovered = ui.any_hovered;
                    let any_interactive = ui.any_interactive_hovered;
                    (ui.draw_calls, events, any_hovered, any_interactive)
                } else {
                    (Vec::new(), Vec::new(), false, false)
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

        if let Some(renderer) = &mut self.renderer
            && let Some(grid) = &mut renderer.grid_selector
        {
            if let Some(corners) = hover_corners {
                grid.update_hover(&renderer.device.queue, corners);
                grid.set_hover_visible(true);
            } else {
                grid.set_hover_visible(false);
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
                    let (pick_bounds, head_offset) = match self.game.sprites.get(&entity.id) {
                        Some(sprite) => (
                            sprite.compute_pick_bounds(
                                &entity.animation,
                                Some(camera_dir),
                                entity.head_dir,
                                screen_anchor,
                                depth,
                                sprite_scale,
                            ),
                            sprite.compute_head_offset(
                                &entity.animation,
                                Some(camera_dir),
                                entity.head_dir,
                                screen_anchor,
                                depth,
                                sprite_scale,
                            ),
                        ),
                        None => {
                            let half = 50.0;
                            (
                                [
                                    screen_anchor[0] - half,
                                    screen_anchor[1] - 100.0,
                                    screen_anchor[0] + half,
                                    screen_anchor[1],
                                ],
                                100.0,
                            )
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
                        head_offset,
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

    /// Trailing pushcart render entries. Each carted entity gets one entry at a
    /// world position offset behind the owner along the opposite of its facing,
    /// matching the original client's ride-distance trail. Kept separate from
    /// the entity list so carts never participate in mouse picking.
    fn compute_cart_render_list(&self) -> Vec<RenderEntry> {
        let mut render_list = Vec::new();
        if let (Some(renderer), Some(coords)) = (&self.renderer, &self.game.map_coords) {
            let screen_w = renderer.device.surface_config.width as f32 / renderer.dpi_scale;
            let screen_h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
            for entity in self.game.entities.iter() {
                if entity.cart_type.is_none() || !self.game.carts.contains_key(&entity.id) {
                    continue;
                }
                let (px, py) = entity.movement.position();
                let (ox, oy) = crate::sprite::cart::direction_offset(entity.direction);
                let cart_pos = (
                    px - ox * crate::sprite::cart::CART_TRAIL_DISTANCE,
                    py - oy * crate::sprite::cart::CART_TRAIL_DISTANCE,
                );
                if let Some((screen_anchor, depth, camera_dir, sprite_scale, depth_gradient)) =
                    input::entity_screen_params(
                        cart_pos,
                        self.game.gat.as_ref(),
                        coords,
                        &renderer.camera,
                        screen_w,
                        screen_h,
                    )
                {
                    render_list.push(RenderEntry {
                        kind: RenderEntryKind::Cart,
                        id: entity.id,
                        screen_anchor,
                        depth,
                        depth_gradient,
                        camera_dir,
                        sprite_scale,
                        pick_bounds: [0.0; 4],
                        head_offset: 0.0,
                    });
                }
            }
        }
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
                        head_offset: half * 2.0,
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
            } else if let Some(pending) = &self.game.pending_skill_target {
                match pending {
                    PendingSkillTarget::Entity { skill_id, .. } => {
                        let class = self
                            .game
                            .character
                            .skills
                            .get_skill(*skill_id)
                            .map(|s| skill_target_class(s.skill_target_type))
                            .unwrap_or(TargetClass::Offensive);
                        let hovered = hovered_entity_cursor_type(
                            self.input.mouse_position,
                            &self.game.entities,
                            render_list,
                            &self.game.map_properties,
                            Some(class),
                        );
                        // Skill targeting always shows the lock/skill-level cursor;
                        // the hover result only tells us which entity is targeted.
                        (CursorType::Lock, hovered.map(|(_, id)| id))
                    }
                    PendingSkillTarget::Ground { .. } => {
                        // TODO: render effect\magic_target.tga ground overlay at hovered cell for skill area
                        (CursorType::Lock, None)
                    }
                }
            } else if let Some((entity_cursor, entity_id)) = hovered_entity_cursor_type(
                self.input.mouse_position,
                &self.game.entities,
                render_list,
                &self.game.map_properties,
                None,
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
                        events::preload_window(&mut self.login_window, renderer, &grf);
                    }

                    self.load_cursor_sprite(&grf);
                    self.load_emotion_sprite(&grf);
                    self.load_status_overlay_sprites(&grf);
                    self.load_damage_sprites(&grf);
                    self.game.data_table.accessory =
                        Some(AccessoryTable::load_from_grf(&grf));
                    self.game.data_table.name = Some(NameTable::load(&grf));
                    self.game.data_table.item_name =
                        Some(ItemNameTable::load(&grf));
                    self.game.data_table.item_resource = Some(
                        ItemResourceTable::load(&grf),
                    );
                    self.game.data_table.item_slot_count =
                        Some(ItemSlotCountTable::load(&grf));
                    self.game.data_table.card_name =
                        Some(CardNameTable::load(&grf));
                    self.game.data_table.card_illustration = Some(
                        CardIllustrationTable::load(&grf),
                    );
                    self.game.data_table.item_description = Some(
                        ItemDescriptionTable::load(&grf),
                    );
                    self.game.data_table.skill_name =
                        Some(SkillNameTable::load(&grf));
                    self.game.data_table.skill_description = Some(
                        SkillDescriptionTable::load(&grf),
                    );
                    self.game.data_table.skill_tree =
                        Some(SkillTreeTable::load(&grf));
                    self.game.data_table.skill_use_level =
                        Some(SkillUseLevelTable::load(&grf));
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
            WindowEvent::CloseRequested => self.handle_close_requested(event_loop),
            WindowEvent::Resized(size) => self.handle_resize(size),
            WindowEvent::MouseInput { state, button, .. } => self.handle_mouse_input(state, button),
            WindowEvent::CursorMoved { position, .. } => self.handle_cursor_moved(position),
            WindowEvent::MouseWheel { delta, .. } => self.handle_mouse_wheel(delta),
            WindowEvent::KeyboardInput { event, .. } => self.handle_keyboard_input(event),
            WindowEvent::ModifiersChanged(modifiers) => self.handle_modifiers_changed(modifiers),
            WindowEvent::RedrawRequested => {
                let elapsed = self.start_time.elapsed().as_secs_f32();

                self.handle_game_events(event_loop);

                let (ui_draw_calls, ui_events, ui_any_hovered, ui_any_interactive) =
                    self.build_ui(elapsed);
                self.input.ui_hovered = ui_any_hovered;
                self.handle_ui_events(ui_events, event_loop);

                // Check for disconnect dialog exit
                if self.game.pending_disconnect_exit {
                    event_loop.exit();
                }
                let now = Instant::now();
                let raw_delta = now.duration_since(self.last_frame_instant).as_secs_f32();
                self.last_frame_instant = now;
                let delta = raw_delta.min(0.1);
                self.run_game_updates(delta, elapsed);

                let hovered = self.update_grid_hover();
                let render_list = self.compute_render_list();
                let floor_item_render_list = self.compute_floor_item_render_list();
                let cart_render_list = self.compute_cart_render_list();
                let hovered_entity_id = self.update_cursor_type(
                    hovered,
                    ui_any_hovered,
                    ui_any_interactive,
                    &render_list,
                );
                self.game.hovered_entity_id = hovered_entity_id;
                if let Some(entity_id) = hovered_entity_id
                    && let Some(entity) = self.game.entities.get_mut(entity_id)
                    && !entity.name_requested
                {
                    entity.name_requested = true;
                    self.channel
                        .send_packet(build_reqname_packet(entity_id, self.config.packetver));
                }

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
                let lock_cursor_clips = self.build_lock_cursor_clips(delta, &render_list);

                let world_overlay_calls = self.build_world_overlays(
                    &render_list,
                    &floor_item_render_list,
                    hovered_entity_id,
                    hovered_floor_item_id,
                );
                let skill_level_calls = self.build_skill_overlay();

                self.compose_and_render(
                    &render_list,
                    &floor_item_render_list,
                    &cart_render_list,
                    elapsed,
                    cursor_clips,
                    lock_cursor_clips,
                    world_overlay_calls,
                    skill_level_calls,
                    ui_draw_calls,
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
