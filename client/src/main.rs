mod config;
mod events;
mod game_state;
mod game_updates;
mod input;
mod input_action;
mod overlay;
mod scene;
mod sound;
mod sprite;

use config::Config;
use game_state::{GameState, PendingGuildConfirm};
use input::InputState;
use models::enums::skill_enums::SkillEnum;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::app_state::AppState;
use ragnarok_game::cursor::{
    CursorType, PendingCompanionSkill, PendingSkillTarget, RenderEntry, RenderEntryKind,
    cursor_type_for_cell, hovered_entity_cursor_type,
};
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
use ragnarok_game::entity::EntityState;
use ragnarok_game::event::GameEvent;
use ragnarok_game::skill::SkillTargetType;
use ragnarok_game::sound::SoundQueue;
use ragnarok_game::sprite_path::{HiddenRender, hidden_render, hide_allows_skill};
use ragnarok_game::targeting::{TargetClass, skill_target_class};
use ragnarok_game::{map_loader, sprite_loader};
use ragnarok_network::{
    KeepaliveMode, NetworkCommand, build_action_request_packet, build_card_composition_list_packet,
    build_card_composition_packet, build_cartoff_packet, build_change_cart_packet,
    build_change_direction_packet,
    build_char_enter_packet, build_chat_packet, build_contact_npc_packet, build_drop_item_packet,
    build_emotion_packet,
    build_delete_char_cancel_packet, build_delete_char_confirm_packet,
    build_delete_char_reserve_packet,
    build_make_char_packet, build_make_char_with_stats_packet,
    build_equip_item_packet, build_login_packet, build_move_item_body_to_cart_packet,
    build_move_item_cart_to_body_packet, build_move_item_cart_to_store_packet,
    build_move_item_store_to_cart_packet, build_move_item_body_to_store_packet,
    build_move_item_store_to_body_packet, build_close_store_packet,
    build_req_exchange_item_packet,
    build_add_exchange_item_packet, build_conclude_exchange_item_packet,
    build_cancel_exchange_item_packet, build_exec_exchange_item_packet,
    build_mail_get_list_packet, build_mail_open_packet, build_mail_delete_packet,
    build_mail_get_item_packet, build_mail_reset_item_packet, build_mail_add_item_packet,
    build_req_mail_return_packet, build_mail_send_packet,
    build_npc_close_packet, build_npc_deal_type_packet,
    build_npc_input_number_packet, build_npc_input_string_packet, build_npc_menu_select_packet,
    build_npc_next_packet, build_pickup_item_packet, build_purchase_item_list_packet,
    build_change_party_exp_option_packet, build_expel_party_member_packet,
    build_join_party_reply_packet, build_leave_party_packet, build_make_party_packet,
    build_make_party2_packet, build_change_party_leader_packet, build_party_invite_by_name_packet,
    build_add_friend_packet, build_ack_add_friend_packet, build_delete_friend_packet,
    build_req_guild_menu, build_req_guild_menuinterface, build_guild_notice,
    build_req_leave_guild, build_req_ban_guild, build_req_change_memberpos,
    build_reg_change_guild_positioninfo, build_make_guild, build_req_disorganize_guild,
    build_register_guild_emblem, build_req_join_guild, build_ans_join_guild,
    build_req_ally_guild, build_ally_guild, build_req_hostile_guild,
    build_req_delete_related_guild,
    build_party_chat_packet, build_remove_option_packet, build_req_enter_room_packet,
    build_create_chatroom_packet, build_change_chatroom_packet, build_change_chat_owner_packet,
    build_expel_chat_member_packet, build_exit_room_packet,
    build_req_disconnect_packet, build_req_join_party_packet, build_reqname_packet,
    build_restart_packet, build_return_savepoint_packet, build_standing_resurrection_packet,
    build_select_char_packet, build_select_warppoint_packet,
    build_sell_item_list_packet, build_shortcut_key_change_packet, build_stat_change_packet,
    build_unequip_item_packet, build_upgrade_skill_packet, build_use_item_packet,
    build_use_skill_packet, build_req_itemidentify_packet, build_req_makingarrow_packet,
    build_req_makingitem_packet, build_req_weaponrefine_packet, build_req_itemrepair_packet,
    build_select_autospell_packet, build_req_openstore2_packet, build_req_cancel_openstore_packet,
    build_req_buy_frommc_packet, build_purchase_frommc2_packet, ip_u32_to_string, network_loop,
    build_companion_move_packet, build_companion_attack_packet,
    build_companion_move_to_owner_packet, build_homun_menu_packet,
    build_mercenary_command_packet, build_rename_homun_packet, build_config_packet,
    build_trycapture_packet, build_command_pet_packet, build_rename_pet_packet,
    build_select_petegg_packet, build_pet_act_packet,
};
use ragnarok_audio::SoundManager;
use ragnarok_renderer::effect::EffectHolder;
use ragnarok_renderer::{
    EffectSpriteCache, GridSelectorRenderer, Renderer, SpriteVertex, StrEffectCache, UiDrawCall,
    block_on, upload_sprite_textures,
};
use ragnarok_ui::context::UiContext;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::state::StateCache;
use ragnarok_formats::act::SpriteAnimationState;
use ragnarok_ui_component::Window as _;
use ragnarok_ui_component::account::char_create_window::CharCreateWindow;
use ragnarok_ui_component::game::guild_expel_dialog::GuildExpelDialog;
use ragnarok_ui_component::game::party_helper_window::MODE_CREATE;
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
    window: Option<Arc<Window>>,
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
    server_list_window: Option<ServerListWindow>,
    char_select_window: Option<CharSelectWindow>,
    char_create_window: Option<CharCreateWindow>,
    account_anims: HashMap<u32, SpriteAnimationState>,
    char_create_built_appearance: Option<(u16, u16)>,
    roulette_act: Option<ragnarok_formats::act::ActFile>,
    roulette_textures: Option<ragnarok_renderer::SpriteTextures>,
    channel: GameChannel,
    game: GameState,
    sound: SoundManager,
    sound_queue: SoundQueue,
    bgm_table: HashMap<String, String>,
    sfx_rng: u32,
    start_time: Instant,
    last_frame_instant: Instant,
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
        game.sound_options.set_values(
            config.bgm_volume,
            config.sfx_volume,
            config.bgm_enabled,
            config.sfx_enabled,
        );
        game.self_config.refuse_party_invite = config.refuse_party_invite;
        let mut effect_queue = EffectQueue::new();
        effect_queue.set_effects_enabled(config.show_skill_effects);
        let sound =
            SoundManager::new(config.effective_bgm_volume(), config.effective_sfx_volume());
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
            server_list_window: None,
            char_select_window: None,
            char_create_window: None,
            account_anims: HashMap::new(),
            char_create_built_appearance: None,
            roulette_act: None,
            roulette_textures: None,
            channel: GameChannel::new(),
            game,
            sound,
            sound_queue: SoundQueue::new(),
            bgm_table: HashMap::new(),
            sfx_rng: 0x1234_5678,
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
            None => {
                self.game.map_missing_window.show(map_name.to_string());
                return;
            }
        };

        self.game.map_missing_window.hide();
        self.game.map_coords = map_data.coordinates;
        self.game.gat = map_data.gat;
        let was_locked = self.game.camera_locked;
        self.game.camera_locked = map_data.indoor;
        if let Some(renderer) = &mut self.renderer {
            if map_data.indoor {
                if !was_locked {
                    self.game.saved_camera_yaw = Some(renderer.camera.yaw);
                }
                input::lock_indoor_camera(&mut renderer.camera);
            } else if was_locked && let Some(yaw) = self.game.saved_camera_yaw.take() {
                renderer.camera.yaw = yaw;
            }
        }

        self.game.ambient_effects.clear(&mut self.effect_queue);
        self.game.ambient_effects =
            ragnarok_game::effects::AmbientEffectScheduler::from_rsw(&map_data.rsw, &map_data.gnd);
        self.game.ambient_sounds =
            ragnarok_game::sound::ambient::AmbientSoundScheduler::from_rsw(&map_data.rsw, &map_data.gnd);
        self.game.repeat_sounds.clear();

        self.game.day_night.on_map_loaded(
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

        if let Some(track) = bgm_track {
            self.play_bgm_track(&track);
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
                    self.sound_queue.ui(ragnarok_game::sound::tables::ui::LOGIN);
                    let addr = format!("{}:{}", self.config.login_ip, self.config.login_port);
                    self.channel.send_cmd(NetworkCommand::Connect(addr));
                    self.sound_queue.ui(ragnarok_game::sound::tables::ui::BUTTON);
                    self.channel.send_packet(build_login_packet(
                        &username,
                        &password,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestSelectServer { index } => {
                    self.sound_queue.ui(ragnarok_game::sound::tables::ui::BUTTON);
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
                    self.sound_queue.ui(ragnarok_game::sound::tables::ui::BUTTON);
                    if let Some(char_win) = &self.char_select_window {
                        self.game.selected_character = char_win
                            .characters
                            .iter()
                            .find(|c| c.slot == slot as i8)
                            .cloned();
                    }
                    if self.config.last_char_slot != Some(slot) {
                        self.config.last_char_slot = Some(slot);
                        self.config.save("config.json");
                    }
                    self.channel
                        .send_packet(build_select_char_packet(slot, self.config.packetver));
                }
                GameEvent::RequestCreateCharacter { slot } => {
                    self.sound_queue.ui(ragnarok_game::sound::tables::ui::BUTTON);
                    let with_stats = self.config.packetver < 20120307;
                    let mut win = CharCreateWindow::new(slot, with_stats);
                    if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
                        let loaded = renderer.preload_textures(&win.layout_texture_paths(), grf);
                        win.set_has_grf_textures(loaded);
                        if with_stats {
                            let _ = renderer.preload_textures(
                                &CharCreateWindow::stat_arrow_texture_paths(),
                                grf,
                            );
                        }
                    }
                    self.char_create_window = Some(win);
                    self.char_create_built_appearance = None;
                    self.game.app_state = AppState::CharacterCreate;
                }
                GameEvent::RequestMakeCharacter {
                    name,
                    slot,
                    hair_style,
                    hair_color,
                    stats,
                } => {
                    self.sound_queue.ui(ragnarok_game::sound::tables::ui::BUTTON);
                    let packet = if self.config.packetver >= 20120307 {
                        build_make_char_packet(
                            &name,
                            slot,
                            hair_style,
                            hair_color,
                            self.config.packetver,
                        )
                    } else {
                        build_make_char_with_stats_packet(
                            &name,
                            stats,
                            slot,
                            hair_style,
                            hair_color,
                            self.config.packetver,
                        )
                    };
                    self.channel.send_packet(packet);
                }
                GameEvent::CancelCreateCharacter => {
                    self.char_create_window = None;
                    self.game.app_state = AppState::CharacterSelect;
                }
                GameEvent::RequestDeleteCharacterReserve { gid } => {
                    self.sound_queue.ui(ragnarok_game::sound::tables::ui::BUTTON);
                    self.channel
                        .send_packet(build_delete_char_reserve_packet(gid, self.config.packetver));
                }
                GameEvent::RequestDeleteCharacterConfirm { gid, birthdate } => {
                    self.sound_queue.ui(ragnarok_game::sound::tables::ui::BUTTON);
                    self.channel.send_packet(build_delete_char_confirm_packet(
                        gid,
                        &birthdate,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestDeleteCharacterCancel { gid } => {
                    self.sound_queue.ui(ragnarok_game::sound::tables::ui::BUTTON);
                    self.channel
                        .send_packet(build_delete_char_cancel_packet(gid, self.config.packetver));
                }
                GameEvent::BackToServerSelect => {
                    self.game.app_state = AppState::ServerSelect;
                    self.char_select_window = None;
                    self.char_create_window = None;
                    self.account_anims.clear();
                    self.game.system_menu.open = false;
                    self.channel.send_cmd(NetworkCommand::Disconnect);
                }
                GameEvent::BackToLogin => {
                    self.game.app_state = AppState::Login;
                    self.server_list_window = None;
                    self.char_select_window = None;
                    self.char_create_window = None;
                    self.account_anims.clear();
                    self.game.login_session = None;
                    self.game.system_menu.open = false;
                    self.channel.send_cmd(NetworkCommand::Disconnect);
                }
                GameEvent::BackToCharacterSelect => {
                    self.game.system_menu.open = false;
                    self.game.map_missing_window.hide();
                    self.clear_companions();
                    self.channel
                        .send_packet(build_restart_packet(self.config.packetver));
                }
                GameEvent::ReturnToSavePoint => {
                    self.channel
                        .send_packet(build_return_savepoint_packet(self.config.packetver));
                }
                GameEvent::RequestStandingResurrection => {
                    self.channel
                        .send_packet(build_standing_resurrection_packet(self.config.packetver));
                }
                GameEvent::RequestMapRecoveryWarp => {
                    let char_name = self
                        .game
                        .selected_character
                        .as_ref()
                        .map(|c| c.name.as_str())
                        .unwrap_or("Unknown");
                    let full_msg = format!("{char_name} : {}", self.config.map_recovery_command);
                    self.channel
                        .send_packet(build_chat_packet(&full_msg, self.config.packetver));
                }
                GameEvent::QuitGame => {
                    self.game.system_menu.open = false;
                    self.channel
                        .send_packet(build_req_disconnect_packet(self.config.packetver));
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
                GameEvent::ToggleChatRoomCreate => {
                    self.game.chat_room_create_window.toggle();
                }
                GameEvent::RequestCreateChatRoom {
                    title,
                    limit,
                    public,
                    password,
                } => {
                    self.game.pending_chat_room = Some((title.clone(), limit, public));
                    self.channel.send_packet(build_create_chatroom_packet(
                        &title,
                        limit,
                        public,
                        &password,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestChangeChatRoom {
                    title,
                    limit,
                    public,
                    password,
                } => {
                    self.channel.send_packet(build_change_chatroom_packet(
                        &title,
                        limit,
                        public,
                        &password,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestLeaveChatRoom => {
                    self.channel
                        .send_packet(build_exit_room_packet(self.config.packetver));
                    self.game.chat_room_member_window.close();
                }
                GameEvent::RequestEditChatRoomSettings => {
                    let w = &self.game.chat_room_member_window;
                    let (room_id, title, limit, public) =
                        (w.room_id(), w.title().to_string(), w.max_count(), w.public());
                    self.game
                        .chat_room_create_window
                        .open_change(room_id, &title, limit, public);
                }
                GameEvent::RequestKickChatMember { name } => {
                    self.channel
                        .send_packet(build_expel_chat_member_packet(&name, self.config.packetver));
                }
                GameEvent::RequestChangeChatOwner { name } => {
                    self.channel.send_packet(build_change_chat_owner_packet(
                        &name,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestOpenChatMemberMenu { name, x, y } => {
                    use ragnarok_ui_component::game::context_menu::{
                        ContextMenuAction, ContextMenuItem,
                    };
                    let items = vec![
                        ContextMenuItem {
                            label: "Hand Over Chat".to_string(),
                            action: ContextMenuAction::ChangeChatOwner { name: name.clone() },
                        },
                        ContextMenuItem {
                            label: "Kick".to_string(),
                            action: ContextMenuAction::KickFromChatRoom { name },
                        },
                    ];
                    self.game.context_menu.open_at(x, y, items);
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
                        let is_book = self.item_is_book(item.item_id);
                        self.game
                            .item_info_window
                            .show(item, &self.game.data_table, is_book);
                        let tex_paths = self.game.item_info_window.pending_texture_paths();
                        self.preload_item_icons(tex_paths);
                    }
                }
                GameEvent::ShowItemInfoDirect { item } => {
                    let is_book = self.item_is_book(item.item_id);
                    self.game
                        .item_info_window
                        .show(&item, &self.game.data_table, is_book);
                    let tex_paths = self.game.item_info_window.pending_texture_paths();
                    self.preload_item_icons(tex_paths);
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
                GameEvent::ReadBook { item_id } => {
                    if let Some(grf) = self.grf.as_ref()
                        && let Ok(data) = grf.read_file(&format!("data/book/{item_id}.txt"))
                    {
                        let content = ragnarok_game::book::BookContent::parse(&data);
                        self.game.book_window.show(content);
                        self.game.item_info_window.close();
                    }
                }
                GameEvent::RequestUseItem { index } => {
                    if self.player_hidden() {
                        continue;
                    }
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
                    self.channel
                        .send_packet(build_move_item_body_to_cart_packet(
                            index,
                            count,
                            self.config.packetver,
                        ));
                }
                GameEvent::RequestMoveItemCartToBody { index, count } => {
                    self.channel
                        .send_packet(build_move_item_cart_to_body_packet(
                            index,
                            count,
                            self.config.packetver,
                        ));
                }
                GameEvent::RequestMoveItemStoreToCart { index, count } => {
                    self.channel
                        .send_packet(build_move_item_store_to_cart_packet(
                            index,
                            count,
                            self.config.packetver,
                        ));
                }
                GameEvent::RequestMoveItemCartToStore { index, count } => {
                    self.channel
                        .send_packet(build_move_item_cart_to_store_packet(
                            index,
                            count,
                            self.config.packetver,
                        ));
                }
                GameEvent::RequestMoveItemBodyToStore { index, count } => {
                    self.channel
                        .send_packet(build_move_item_body_to_store_packet(
                            index,
                            count,
                            self.config.packetver,
                        ));
                }
                GameEvent::RequestMoveItemStoreToBody { index, count } => {
                    self.channel
                        .send_packet(build_move_item_store_to_body_packet(
                            index,
                            count,
                            self.config.packetver,
                        ));
                }
                GameEvent::RequestCloseStorage => {
                    self.channel
                        .send_packet(build_close_store_packet(self.config.packetver));
                }
                GameEvent::RequestExchangeItem { target_aid } => {
                    let name = self
                        .game
                        .entities
                        .get(target_aid)
                        .and_then(|e| e.name.clone())
                        .unwrap_or_default();
                    self.game.pending_trade_partner = Some((target_aid, name));
                    self.channel
                        .send_packet(build_req_exchange_item_packet(target_aid, self.config.packetver));
                }
                GameEvent::RespondExchangeRequest { accept } => {
                    self.respond_exchange_request(accept);
                }
                GameEvent::RequestAddExchangeItem { index, count } => {
                    self.channel
                        .send_packet(build_add_exchange_item_packet(index, count, self.config.packetver));
                }
                GameEvent::RequestConcludeExchange => {
                    self.channel
                        .send_packet(build_conclude_exchange_item_packet(self.config.packetver));
                }
                GameEvent::RequestCancelExchange => {
                    self.channel
                        .send_packet(build_cancel_exchange_item_packet(self.config.packetver));
                }
                GameEvent::RequestExecExchange => {
                    self.channel
                        .send_packet(build_exec_exchange_item_packet(self.config.packetver));
                }
                GameEvent::RequestMailList => {
                    self.channel
                        .send_packet(build_mail_get_list_packet(self.config.packetver));
                }
                GameEvent::RequestMailOpen { mail_id } => {
                    self.channel
                        .send_packet(build_mail_open_packet(mail_id, self.config.packetver));
                }
                GameEvent::RequestMailDelete { mail_id } => {
                    self.channel
                        .send_packet(build_mail_delete_packet(mail_id, self.config.packetver));
                }
                GameEvent::RequestMailGetItem { mail_id } => {
                    self.channel
                        .send_packet(build_mail_get_item_packet(mail_id, self.config.packetver));
                }
                GameEvent::RequestMailResetItem { ty } => {
                    self.channel
                        .send_packet(build_mail_reset_item_packet(ty, self.config.packetver));
                }
                GameEvent::RequestMailAddItem { index, amount } => {
                    self.channel.send_packet(build_mail_add_item_packet(
                        index,
                        amount,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestMailSend { to, title, body } => {
                    self.channel.send_packet(build_mail_send_packet(
                        &to,
                        &title,
                        &body,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestMailReturn { mail_id, sender } => {
                    self.channel.send_packet(build_req_mail_return_packet(
                        mail_id,
                        &sender,
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
                GameEvent::RequestSetCartPick { .. } => {}
                GameEvent::ToggleCart => {
                    self.game.character.cart.toggle();
                }
                GameEvent::RequestSkillLevelUp { skill_id } => {
                    self.channel
                        .send_packet(build_upgrade_skill_packet(skill_id, self.config.packetver));
                }
                GameEvent::RequestStatChange { status_id, amount } => {
                    self.channel.send_packet(build_stat_change_packet(
                        status_id,
                        amount,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestCompanionMove { gid, x, y } => {
                    self.channel.send_packet(build_companion_move_packet(
                        gid,
                        x as u16,
                        y as u16,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestCompanionAttack { gid, target_gid } => {
                    self.channel.send_packet(build_companion_attack_packet(
                        gid,
                        target_gid,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestCompanionMoveToOwner { gid } => {
                    self.channel.send_packet(build_companion_move_to_owner_packet(
                        gid,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestSetConfig { kind, enabled } => {
                    self.channel.send_packet(build_config_packet(
                        kind.config_id(),
                        enabled as i32,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestHomunMenu { command } => {
                    self.channel
                        .send_packet(build_homun_menu_packet(command as i8, self.config.packetver));
                    if command == 2 {
                        self.clear_homunculus();
                    }
                }
                GameEvent::RequestHomunRest => {
                    let skill_id = SkillEnum::AmRest.id() as u16;
                    if !self.skill_on_cooldown(skill_id) {
                        let target_id = self.game.entities.player_id().unwrap_or(0);
                        self.channel.send_packet(build_use_skill_packet(
                            skill_id,
                            1,
                            target_id,
                            self.config.packetver,
                        ));
                    }
                }
                GameEvent::RequestHomunDelete => {
                    let name = self
                        .game
                        .homunculus
                        .as_ref()
                        .map(|h| h.name.clone())
                        .filter(|n| !n.is_empty())
                        .unwrap_or_else(|| "your homunculus".to_string());
                    self.game.homun_delete_result.set(None);
                    self.game.homun_delete_pending = true;
                    self.game.confirm_dialog.show_with_out(
                        &format!("Delete {name} permanently?"),
                        true,
                        self.game.homun_delete_result.clone(),
                        |_| {},
                    );
                }
                GameEvent::RequestMercenaryCommand { command } => {
                    self.channel.send_packet(build_mercenary_command_packet(
                        command,
                        self.config.packetver,
                    ));
                    if command == 2 {
                        self.clear_mercenary();
                    }
                }
                GameEvent::RequestRenameHomun { name } => {
                    self.channel
                        .send_packet(build_rename_homun_packet(&name, self.config.packetver));
                }
                GameEvent::ToggleHomunculusWindow => {
                    self.game.homunculus_window.toggle();
                }
                GameEvent::ToggleMercenaryWindow => {
                    self.game.mercenary_window.toggle();
                }
                GameEvent::ToggleMercenarySkillWindow => {
                    self.game.mercenary_skill_window.toggle();
                }
                GameEvent::ToggleHomunSkillWindow => {
                    self.game.homun_skill_window.toggle();
                }
                GameEvent::ToggleCompanionAiConfig => {
                    self.game.companion_ai_config_window.toggle();
                }
                GameEvent::SaveCompanionAiConfig => {
                    if let Err(e) = self
                        .game
                        .companion_ai
                        .save(crate::game_state::COMPANION_AI_CONFIG_PATH)
                    {
                        tracing::warn!("failed to save companion AI config: {e}");
                    }
                }
                GameEvent::RevertCompanionAiConfig => {
                    self.game.companion_ai = ragnarok_ai::config::CompanionAiConfig::load_or_default(
                        crate::game_state::COMPANION_AI_CONFIG_PATH,
                    );
                }
                GameEvent::ResetCompanionAiConfig => {
                    self.game.companion_ai = ragnarok_ai::config::CompanionAiConfig::default();
                }
                GameEvent::ToggleCompanionStandby { is_mercenary } => {
                    self.push_owner_command_to(
                        is_mercenary,
                        ragnarok_game::companion::OwnerCommand::follow(),
                        false,
                    );
                }
                GameEvent::HotkeyListReceived { slots } => {
                    self.game
                        .character
                        .hotkeys
                        .set_from_server(&slots);
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
                    if self.player_hidden() && !hide_allows_skill(skill_id) {
                        continue;
                    }
                    if skill_id == SkillEnum::McChangecart.id() as u16 {
                        if self.game.character.cart_design.is_some() {
                            self.preload_cart_previews(&[1, 2, 3, 4, 5]);
                            self.game.cart_select_window.open();
                        }
                        continue;
                    }
                    if skill_id == SkillEnum::AcMakingarrow.id() as u16
                        || skill_id == SkillEnum::SaCreatecon.id() as u16
                    {
                        self.game.pending_list_skill = Some(skill_id);
                    }
                    let skill_target_type = self
                        .game
                        .resolve_cast_skill(skill_id)
                        .map(|(target_type, _)| target_type)
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
                GameEvent::RequestCompanionUseSkill {
                    is_mercenary,
                    skill_id,
                    level,
                } => {
                    // The companion must exist to cast its own skill, so its skill
                    // list (with the target type) is available here even though the
                    // hotkey that triggered this carried only the id.
                    let companion = if is_mercenary {
                        self.game.mercenary.as_ref().map(|m| (m.gid, &m.skills))
                    } else {
                        self.game.homunculus.as_ref().map(|h| (h.gid, &h.skills))
                    };
                    let Some((gid, skills)) = companion else {
                        tracing::info!("RequestCompanionUseSkill: no companion present — dropped");
                        continue;
                    };
                    let target_type = skills
                        .iter()
                        .find(|s| s.id == skill_id)
                        .map(|s| s.skill_target_type)
                        .unwrap_or(SkillTargetType::Target);
                    tracing::info!(
                        "RequestCompanionUseSkill: merc={is_mercenary} skill={skill_id} gid={gid} target_type={target_type:?}"
                    );
                    match target_type {
                        SkillTargetType::Target | SkillTargetType::Friend => {
                            self.game.pending_companion_skill = Some(PendingCompanionSkill {
                                is_mercenary,
                                skill_id,
                                level,
                                is_ground: false,
                            });
                        }
                        SkillTargetType::Ground | SkillTargetType::Trap => {
                            self.game.pending_companion_skill = Some(PendingCompanionSkill {
                                is_mercenary,
                                skill_id,
                                level,
                                is_ground: true,
                            });
                        }
                        _ => {
                            self.push_owner_command_to(
                                is_mercenary,
                                ragnarok_game::companion::OwnerCommand::skill_object(
                                    skill_id, level as u8, gid,
                                ),
                                self.input.shift_pressed,
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
                GameEvent::RequestIdentifyItem { index } => {
                    self.channel
                        .send_packet(build_req_itemidentify_packet(index, self.config.packetver));
                }
                GameEvent::RequestMakingArrow { item_id } => {
                    self.channel
                        .send_packet(build_req_makingarrow_packet(item_id, self.config.packetver));
                }
                GameEvent::RequestMakingItem { item_id, materials } => {
                    self.channel.send_packet(build_req_makingitem_packet(
                        item_id,
                        materials,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestWeaponRefine { index } => {
                    self.channel
                        .send_packet(build_req_weaponrefine_packet(index, self.config.packetver));
                }
                GameEvent::RequestRepairItem {
                    index,
                    item_id,
                    refine,
                    cards,
                } => {
                    self.channel.send_packet(build_req_itemrepair_packet(
                        index,
                        item_id,
                        refine,
                        cards,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestSelectAutoSpell { skill_id } => {
                    self.channel
                        .send_packet(build_select_autospell_packet(skill_id, self.config.packetver));
                }
                GameEvent::RequestOpenStore { shop_name, items } => {
                    self.channel.send_packet(build_req_openstore2_packet(
                        &shop_name,
                        &items,
                        self.config.packetver,
                    ));
                    self.game.pending_shop_name = Some(shop_name);
                }
                GameEvent::RequestCancelVendingSetup => {
                    self.channel
                        .send_packet(build_req_cancel_openstore_packet(self.config.packetver));
                }
                GameEvent::RequestCloseStore => {
                    self.close_own_shop();
                }
                GameEvent::RequestBuyFromVendor { aid } => {
                    self.channel
                        .send_packet(build_req_buy_frommc_packet(aid, self.config.packetver));
                }
                GameEvent::RequestPurchaseFromVendor {
                    aid,
                    unique_id,
                    items,
                } => {
                    self.channel.send_packet(build_purchase_frommc2_packet(
                        aid,
                        unique_id,
                        &items,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestSendChat { message } => {
                    self.run_chat_command(&message);
                }
                GameEvent::ToggleShortcutList => {
                    if !self.game.shortcut_list_window.is_open() {
                        self.game
                            .shortcut_list_window
                            .set_bindings(&self.config.shortcut_commands);
                    }
                    self.game.shortcut_list_window.toggle();
                }
                GameEvent::ShortcutBindingsChanged(commands) => {
                    self.config.shortcut_commands = commands;
                }
                GameEvent::ToggleQuestWindow => {
                    self.game.quest_window.toggle();
                }
                GameEvent::OpenQuestDetail { quest_id } => {
                    self.game.quest_detail_window.open(quest_id);
                }
                GameEvent::RequestToggleQuestActive { quest_id, active } => {
                    self.channel.send_packet(ragnarok_network::build_active_quest_packet(
                        quest_id,
                        active,
                        self.config.packetver,
                    ));
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
                GameEvent::ToggleEmotionWindow => {
                    self.game.emotion_window.toggle();
                }
                GameEvent::RequestEmotion { emote_type } => {
                    self.channel
                        .send_packet(build_emotion_packet(emote_type, self.config.packetver));
                }
                GameEvent::ToggleStatusWindow => {
                    self.game.status_window.toggle();
                }
                GameEvent::ToggleMinimap => {
                    self.game.minimap_window.cycle_visibility();
                }
                GameEvent::ToggleSoundOptions => {
                    self.game.sound_options.set_values(
                        self.config.bgm_volume,
                        self.config.sfx_volume,
                        self.config.bgm_enabled,
                        self.config.sfx_enabled,
                    );
                    self.game.sound_options.toggle();
                }
                GameEvent::SoundSettingsChanged {
                    bgm_volume,
                    sfx_volume,
                    bgm_enabled,
                    sfx_enabled,
                    persist,
                } => {
                    self.config.bgm_volume = bgm_volume;
                    self.config.sfx_volume = sfx_volume;
                    self.config.bgm_enabled = bgm_enabled;
                    self.config.sfx_enabled = sfx_enabled;
                    self.sound.set_volumes(
                        self.config.effective_bgm_volume(),
                        self.config.effective_sfx_volume(),
                    );
                    if persist {
                        self.config.save("config.json");
                    }
                }
                GameEvent::ToggleGraphicOptions => {
                    if !self.game.graphic_options.open {
                        let resolutions = self.available_resolutions();
                        self.game.graphic_options.set_values(
                            resolutions,
                            (self.config.screen_width, self.config.screen_height),
                            self.config.fullscreen,
                            self.config.fog,
                            self.config.show_skill_effects,
                            self.config.display.clone(),
                            self.config.refuse_trade,
                            self.config.refuse_party_invite,
                        );
                    }
                    self.game.graphic_options.toggle();
                }
                GameEvent::GraphicsSettingsChanged {
                    width,
                    height,
                    fullscreen,
                    fog,
                    show_skill_effects,
                    display,
                    refuse_trade,
                    refuse_party_invite,
                    persist,
                } => {
                    self.apply_graphics_settings(
                        width,
                        height,
                        fullscreen,
                        fog,
                        show_skill_effects,
                        display,
                        refuse_trade,
                        refuse_party_invite,
                        persist,
                    );
                }
                GameEvent::TogglePartyWindow => {
                    self.game.party_friends_window.open_party_tab();
                }
                GameEvent::ToggleFriendWindow => {
                    self.game.party_friends_window.open_friend_tab();
                }
                GameEvent::RequestPartyInvite { target_aid } => {
                    let pv = self.config.packetver;
                    if self.game.party.is_none() {
                        // The party is created asynchronously server-side, so the invite must
                        // wait for the create ack — sending it now would be dropped.
                        let party_name = self
                            .game
                            .selected_character
                            .as_ref()
                            .map(|c| c.name.clone())
                            .unwrap_or_else(|| "Party".to_string());
                        self.channel
                            .send_packet(build_make_party_packet(&party_name, pv));
                        self.game.pending_invite_aid = Some(target_aid);
                    } else {
                        self.channel
                            .send_packet(build_req_join_party_packet(target_aid, pv));
                    }
                }
                GameEvent::RespondPartyInvite { party_grid, accept } => {
                    self.channel.send_packet(build_join_party_reply_packet(
                        party_grid,
                        accept,
                        self.config.packetver,
                    ));
                }
                GameEvent::RespondGuildInvite { gdid, accept } => {
                    self.channel
                        .send_packet(build_ans_join_guild(gdid, accept, self.config.packetver));
                }
                GameEvent::RespondGuildAlly { aid, accept } => {
                    self.channel
                        .send_packet(build_ally_guild(aid, accept, self.config.packetver));
                }
                GameEvent::RequestLeaveParty => {
                    self.channel
                        .send_packet(build_leave_party_packet(self.config.packetver));
                }
                GameEvent::RequestExpelMember { aid, name } => {
                    self.channel.send_packet(build_expel_party_member_packet(
                        aid,
                        &name,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestPartyExpOption { exp_share } => {
                    self.channel
                        .send_packet(build_change_party_exp_option_packet(
                            exp_share as u32,
                            self.config.packetver,
                        ));
                }
                GameEvent::SendPartyChat { message } => {
                    self.channel
                        .send_packet(build_party_chat_packet(&message, self.config.packetver));
                }
                GameEvent::ShowPartyHelper { mode } => {
                    let local_aid = self
                        .game
                        .login_session
                        .as_ref()
                        .map(|s| s.account_id)
                        .unwrap_or(0);
                    let is_leader = self
                        .game
                        .party
                        .as_ref()
                        .and_then(|p| p.leader_aid())
                        .map(|aid| aid == local_aid)
                        .unwrap_or(false);
                    let (exp, pickup, division) = self
                        .game
                        .party
                        .as_ref()
                        .map(|p| (p.exp_share, p.item_pickup_rule, p.item_division_rule))
                        .unwrap_or((false, 0, 0));
                    let editable = mode == MODE_CREATE || is_leader;
                    self.game
                        .party_helper_window
                        .open(mode, exp, pickup, division, editable);
                }
                GameEvent::RequestPartyCreate {
                    name,
                    item_pickup_rule,
                    item_division_rule,
                } => {
                    self.channel.send_packet(build_make_party2_packet(
                        &name,
                        item_pickup_rule,
                        item_division_rule,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestPartyInviteByName { name } => {
                    self.channel.send_packet(build_party_invite_by_name_packet(
                        &name,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestChangePartyLeader { aid } => {
                    self.channel
                        .send_packet(build_change_party_leader_packet(aid, self.config.packetver));
                }
                GameEvent::RequestGuildInfoBurst => {
                    let pv = self.config.packetver;
                    self.channel.send_packet(build_req_guild_menuinterface(pv));
                    for atype in 0..=4 {
                        self.channel.send_packet(build_req_guild_menu(atype, pv));
                    }
                }
                GameEvent::RequestGuildMenu { atype } => {
                    self.channel
                        .send_packet(build_req_guild_menu(atype, self.config.packetver));
                }
                GameEvent::ShowGuildMemberMenu { aid, gid, name, x, y } => {
                    use ragnarok_ui_component::game::context_menu::{
                        ContextMenuAction, ContextMenuItem,
                    };
                    let local_gid = self
                        .game
                        .login_session
                        .as_ref()
                        .map(|s| s.account_id)
                        .unwrap_or(0);
                    let mut items = Vec::new();
                    let is_self = gid == local_gid;
                    if !is_self {
                        items.push(ContextMenuItem {
                            label: "Whisper".to_string(),
                            action: ContextMenuAction::Whisper { name: name.clone() },
                        });
                    }
                    if let Some(g) = &self.game.guild {
                        let target_master = g
                            .member_by_gid(gid)
                            .map(|m| m.position_id == 0)
                            .unwrap_or(false);
                        if is_self && !g.is_master(local_gid) {
                            items.push(ContextMenuItem {
                                label: "Leave Guild".to_string(),
                                action: ContextMenuAction::GuildLeave,
                            });
                        }
                        if g.is_master(local_gid) && !target_master {
                            for p in &g.positions {
                                items.push(ContextMenuItem {
                                    label: format!("Set: {}", p.name),
                                    action: ContextMenuAction::ChangeGuildPosition {
                                        aid,
                                        gid,
                                        position_id: p.id,
                                    },
                                });
                            }
                            items.push(ContextMenuItem {
                                label: "Expel".to_string(),
                                action: ContextMenuAction::ExpelFromGuild { aid, gid, name },
                            });
                        }
                    }
                    self.game.context_menu.open_at(x, y, items);
                }
                GameEvent::RequestSetGuildNotice { subject, body } => {
                    if let Some(gdid) = self.game.guild.as_ref().map(|g| g.gdid) {
                        self.channel.send_packet(build_guild_notice(
                            gdid,
                            &subject,
                            &body,
                            self.config.packetver,
                        ));
                    }
                }
                GameEvent::RequestGuildLeave => {
                    let name = self
                        .game
                        .guild
                        .as_ref()
                        .map(|g| g.name.clone())
                        .filter(|n| !n.is_empty())
                        .unwrap_or_else(|| "the guild".to_string());
                    self.game.guild_confirm_result.set(None);
                    self.game.pending_guild_confirm = Some(PendingGuildConfirm::Leave);
                    self.game.confirm_dialog.show_with_out(
                        &format!("Leave {name}?"),
                        true,
                        self.game.guild_confirm_result.clone(),
                        |_| {},
                    );
                }
                GameEvent::RequestGuildExpel { aid, gid, name } => {
                    self.game.guild_expel_dialog = Some(GuildExpelDialog::new(aid, gid, name));
                }
                GameEvent::ConfirmedGuildLeave => {
                    if let Some(g) = &self.game.guild {
                        let aid = self
                            .game
                            .login_session
                            .as_ref()
                            .map(|s| s.account_id)
                            .unwrap_or(0) as i32;
                        self.channel.send_packet(build_req_leave_guild(
                            g.gdid,
                            aid,
                            aid,
                            "",
                            self.config.packetver,
                        ));
                    }
                }
                GameEvent::ConfirmedGuildExpel { aid, gid, name: _, reason } => {
                    if let Some(gdid) = self.game.guild.as_ref().map(|g| g.gdid) {
                        self.channel.send_packet(build_req_ban_guild(
                            gdid,
                            aid as i32,
                            gid as i32,
                            &reason,
                            self.config.packetver,
                        ));
                    }
                }
                GameEvent::RequestChangeMemberPosition { aid, gid, position_id } => {
                    self.channel.send_packet(build_req_change_memberpos(
                        aid as i32,
                        gid as i32,
                        position_id,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestChangePositionInfo { positions } => {
                    self.channel.send_packet(build_reg_change_guild_positioninfo(
                        &positions,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestUpgradeGuildSkill { skid } => {
                    self.channel
                        .send_packet(build_upgrade_skill_packet(skid, self.config.packetver));
                }
                GameEvent::RequestGuildInvite { target_aid } => {
                    let (my_aid, my_gid) = self.local_aid_gid();
                    self.channel.send_packet(build_req_join_guild(
                        target_aid,
                        my_aid,
                        my_gid,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestGuildAlly { target_aid } => {
                    let (my_aid, my_gid) = self.local_aid_gid();
                    self.channel.send_packet(build_req_ally_guild(
                        target_aid,
                        my_aid,
                        my_gid,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestGuildHostile { target_aid } => {
                    self.channel
                        .send_packet(build_req_hostile_guild(target_aid, self.config.packetver));
                }
                GameEvent::RequestDeleteGuildRelation { gdid, relation } => {
                    let msg = if relation == 0 {
                        "Cancel this alliance?"
                    } else {
                        "Cancel this antagonist declaration?"
                    };
                    self.game.guild_confirm_result.set(None);
                    self.game.pending_guild_confirm =
                        Some(PendingGuildConfirm::DeleteRelation { gdid, relation });
                    self.game.confirm_dialog.show_with_out(
                        msg,
                        true,
                        self.game.guild_confirm_result.clone(),
                        |_| {},
                    );
                }
                GameEvent::ConfirmedDeleteGuildRelation { gdid, relation } => {
                    self.channel.send_packet(build_req_delete_related_guild(
                        gdid,
                        relation,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestSelectEmblem => {
                    self.open_emblem_picker();
                }
                GameEvent::RequestUploadEmblem { path } => {
                    self.upload_emblem_file(&path);
                }
                GameEvent::RequestAddFriend { name } => {
                    self.channel
                        .send_packet(build_add_friend_packet(&name, self.config.packetver));
                }
                GameEvent::RequestDeleteFriend { aid, gid } => {
                    self.channel
                        .send_packet(build_delete_friend_packet(aid, gid, self.config.packetver));
                }
                GameEvent::RespondFriendRequest {
                    req_aid,
                    req_gid,
                    accept,
                } => {
                    self.channel.send_packet(build_ack_add_friend_packet(
                        req_aid,
                        req_gid,
                        accept,
                        self.config.packetver,
                    ));
                }
                GameEvent::RequestWhisper { name } => {
                    self.game.chat_window.start_whisper(name);
                }

                GameEvent::RequestTryCapture { gid } => {
                    self.channel
                        .send_packet(build_trycapture_packet(gid, self.config.packetver));
                }
                GameEvent::RequestPetCommand { csub } => {
                    self.channel
                        .send_packet(build_command_pet_packet(csub, self.config.packetver));
                    // Return-to-egg: the pet vanishes and the egg becomes usable again.
                    if csub == 3
                        && let Some(index) = self.game.pet.egg_index.take()
                    {
                        self.game.character.inventory.set_item_damaged(index, false);
                    }
                    // Performance: owner emits a matching emote (PM_PERFORMANCE_S).
                    if csub == 2 {
                        self.emit_pet_act(5);
                    }
                }
                GameEvent::RequestRenamePet { name } => {
                    self.channel
                        .send_packet(build_rename_pet_packet(&name, self.config.packetver));
                }
                GameEvent::RequestSelectPetEgg { index } => {
                    self.channel
                        .send_packet(build_select_petegg_packet(index, self.config.packetver));
                    self.game.pet.egg_index = Some(index);
                    self.game.character.inventory.set_item_damaged(index, true);
                }
                GameEvent::RequestPetAct { data } => {
                    self.channel
                        .send_packet(build_pet_act_packet(data, self.config.packetver));
                }
                GameEvent::RequestPetFeed => {
                    self.game.pet_feed_result.set(None);
                    self.game.pet_feed_pending = true;
                    self.game.confirm_dialog.show_with_out(
                        "Are you sure you want to feed your pet?",
                        true,
                        self.game.pet_feed_result.clone(),
                        |_| {},
                    );
                }
                GameEvent::TogglePetWindow => {
                    self.game.pet_window.toggle();
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
        self.game.app_state = AppState::CharacterSelect;
        true
    }

    fn local_aid_gid(&self) -> (u32, u32) {
        let aid = self
            .game
            .login_session
            .as_ref()
            .map(|s| s.account_id)
            .unwrap_or(0);
        (aid, aid)
    }

    fn open_emblem_picker(&mut self) {
        use ragnarok_ui_component::game::emblem_picker_window::EmblemEntry;

        let dir = std::path::PathBuf::from(&self.config.emblem_path);
        let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bmp")))
            .collect();
        files.sort();

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let mut entries = Vec::new();
        for path in files {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let (valid, verdict) = match ragnarok_game::guild::validate_emblem_bmp(&bytes) {
                Ok(()) => (true, String::new()),
                Err(e) => (false, e),
            };
            let key = format!("__emblem_file_{name}");
            if renderer.texture_cache.texture_size(&key).is_none()
                && let Some(rgba) = ragnarok_renderer::texture::decode_emblem(&bytes)
            {
                let (w, h) = (rgba.width(), rgba.height());
                let bg = ragnarok_renderer::texture::create_texture_bind_group_from_rgba(
                    &renderer.device.device,
                    &renderer.device.queue,
                    rgba.as_raw(),
                    w,
                    h,
                    &renderer.texture_cache.bind_group_layout,
                    &key,
                    ragnarok_renderer::wgpu::FilterMode::Nearest,
                    ragnarok_renderer::wgpu::TextureFormat::Rgba8Unorm,
                    ragnarok_renderer::wgpu::AddressMode::ClampToEdge,
                );
                renderer.texture_cache.insert(&key, bg, w, h);
            }
            entries.push(EmblemEntry {
                name,
                path: path.to_string_lossy().to_string(),
                key,
                valid,
                verdict,
            });
        }
        if entries.is_empty() {
            self.game.chat_window.add_system(format!(
                "No emblem .bmp found in '{}'.",
                dir.display()
            ));
        }
        self.game.emblem_picker_window.open(entries);
    }

    fn upload_emblem_file(&mut self, path: &str) {
        let Ok(bmp) = std::fs::read(path) else {
            self.game
                .chat_window
                .add_system(format!("Failed to read emblem '{path}'."));
            return;
        };
        if let Err(e) = ragnarok_game::guild::validate_emblem_bmp(&bmp) {
            self.game.chat_window.add_system(e);
            return;
        }
        let compressed = ragnarok_formats::zlib_compress(&bmp);
        self.channel
            .send_packet(build_register_guild_emblem(compressed, self.config.packetver));
        if let Some(guild) = &self.game.guild {
            self.channel.send_packet(ragnarok_network::build_req_guild_emblem_img(
                guild.gdid,
                self.config.packetver,
            ));
        }
    }

    fn available_resolutions(&self) -> Vec<(u32, u32)> {
        let Some(monitor) = self.window.as_ref().and_then(|w| w.current_monitor()) else {
            return Vec::new();
        };
        let monitor_size = monitor.size();
        let is_standard_aspect =
            |w: u32, h: u32| w * 3 == h * 4 || w * 9 == h * 16 || w * 10 == h * 16;
        let mut resolutions: Vec<(u32, u32)> = monitor
            .video_modes()
            .map(|m| (m.size().width, m.size().height))
            .filter(|&(w, h)| {
                w > 800
                    && w <= monitor_size.width
                    && h <= monitor_size.height
                    && is_standard_aspect(w, h)
            })
            .collect();
        resolutions.sort();
        resolutions.dedup();
        resolutions
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_graphics_settings(
        &mut self,
        width: u32,
        height: u32,
        fullscreen: bool,
        fog: bool,
        show_skill_effects: bool,
        display: crate::config::DisplayOptions,
        refuse_trade: bool,
        refuse_party_invite: bool,
        persist: bool,
    ) {
        let resolution_changed =
            (width, height) != (self.config.screen_width, self.config.screen_height);
        let fullscreen_changed = fullscreen != self.config.fullscreen;
        let aura_changed = display.show_level_aura != self.config.display.show_level_aura;

        self.config.screen_width = width;
        self.config.screen_height = height;
        self.config.fullscreen = fullscreen;
        self.config.fog = fog;
        self.config.show_skill_effects = show_skill_effects;
        self.config.display = display;
        self.config.refuse_trade = refuse_trade;
        self.config.refuse_party_invite = refuse_party_invite;
        self.game.self_config.refuse_party_invite = refuse_party_invite;

        if let Some(window) = &self.window {
            if fullscreen_changed {
                window.set_fullscreen(
                    fullscreen.then(|| winit::window::Fullscreen::Borderless(None)),
                );
            }
            if resolution_changed && !fullscreen {
                let _ = window.request_inner_size(winit::dpi::LogicalSize::new(width, height));
            }
        }
        if let Some(renderer) = &mut self.renderer {
            renderer.set_fog(if fog { self.map_fog } else { None });
        }
        self.effect_queue.set_effects_enabled(show_skill_effects);
        if aura_changed {
            let gids: Vec<u32> = self.game.entities.iter().map(|e| e.id).collect();
            for gid in gids {
                self.refresh_level_aura(gid);
            }
        }
        if persist {
            self.config.save("config.json");
        }
    }

    fn run_chat_command(&mut self, message: &str) {
        if message.is_empty() {
            return;
        }
        if message.starts_with('/') {
            self.handle_slash_command(message);
        } else if let Some(party_msg) = message.strip_prefix('%') {
            let char_name = self
                .game
                .selected_character
                .as_ref()
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown");
            let full_msg = format!("{char_name} : {}", party_msg.trim_start());
            self.channel
                .send_packet(build_party_chat_packet(&full_msg, self.config.packetver));
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

    fn handle_slash_command(&mut self, command: &str) {
        let cmd = command.split_whitespace().next().unwrap_or("");
        match cmd {
            "/sit" => {
                if self.player_hidden() {
                    return;
                }
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
                    entity.head_dir = if entity.head_dir == 1 { 2 } else { 1 };
                    let (head_dir, dir) = (entity.head_dir, entity.direction);
                    self.channel.send_packet(build_change_direction_packet(
                        head_dir,
                        dir,
                        self.config.packetver,
                    ));
                }
            }
            "/bingbing" | "/bangbang" => {
                if let Some(entity) = self.game.entities.player_mut() {
                    let step = if cmd == "/bingbing" { 1 } else { 7 };
                    entity.direction = (entity.direction + step) % 8;
                    entity.head_dir = 0;
                    let (head_dir, dir) = (entity.head_dir, entity.direction);
                    self.channel.send_packet(build_change_direction_packet(
                        head_dir,
                        dir,
                        self.config.packetver,
                    ));
                }
            }
            "/effect" => {
                let show = !self.config.show_skill_effects;
                self.apply_graphics_settings(
                    self.config.screen_width,
                    self.config.screen_height,
                    self.config.fullscreen,
                    self.config.fog,
                    show,
                    self.config.display.clone(),
                    self.config.refuse_trade,
                    self.config.refuse_party_invite,
                    true,
                );
                let status = if show { "ON" } else { "OFF" };
                self.game
                    .chat_window
                    .add_system(format!("Skill effects: {status}"));
            }
            "/fog" => {
                let fog = !self.config.fog;
                self.apply_graphics_settings(
                    self.config.screen_width,
                    self.config.screen_height,
                    self.config.fullscreen,
                    fog,
                    self.config.show_skill_effects,
                    self.config.display.clone(),
                    self.config.refuse_trade,
                    self.config.refuse_party_invite,
                    true,
                );
                let status = if fog { "ON" } else { "OFF" };
                self.game.chat_window.add_system(format!("Fog: {status}"));
            }
            "/aura" => {
                let mut display = self.config.display.clone();
                display.show_level_aura = !display.show_level_aura;
                let status = if display.show_level_aura { "ON" } else { "OFF" };
                self.apply_graphics_settings(
                    self.config.screen_width,
                    self.config.screen_height,
                    self.config.fullscreen,
                    self.config.fog,
                    self.config.show_skill_effects,
                    display,
                    self.config.refuse_trade,
                    self.config.refuse_party_invite,
                    true,
                );
                self.game
                    .chat_window
                    .add_system(format!("Level aura: {status}"));
            }
            "/notrade" | "/nt" => {
                let refuse = !self.config.refuse_trade;
                self.apply_graphics_settings(
                    self.config.screen_width,
                    self.config.screen_height,
                    self.config.fullscreen,
                    self.config.fog,
                    self.config.show_skill_effects,
                    self.config.display.clone(),
                    refuse,
                    self.config.refuse_party_invite,
                    true,
                );
                let status = if refuse { "ON" } else { "OFF" };
                self.game
                    .chat_window
                    .add_system(format!("Refuse trade requests: {status}"));
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
            "/where" => match (self.game.current_map.as_ref(), self.game.entities.player()) {
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
            },
            "/organize" => {
                let name = command["/organize".len()..].trim();
                if name.is_empty() {
                    self.game
                        .chat_window
                        .add_system("Usage: /organize <party name>".to_string());
                } else if self.game.party.is_some() {
                    self.game
                        .chat_window
                        .add_system("You are already in a party.".to_string());
                } else {
                    self.channel
                        .send_packet(build_make_party_packet(name, self.config.packetver));
                }
            }
            "/guild" => {
                const EMPERIUM_ITEM_ID: u16 = 714;
                let name = command["/guild".len()..].trim();
                let has_emperium = self
                    .game
                    .character
                    .inventory
                    .all_items()
                    .iter()
                    .any(|i| i.item_id == EMPERIUM_ITEM_ID);
                let gid = self
                    .game
                    .login_session
                    .as_ref()
                    .map(|s| s.account_id)
                    .unwrap_or(0);
                if name.is_empty() {
                    self.game
                        .chat_window
                        .add_system("Usage: /guild <guild name>".to_string());
                } else if self.game.guild.is_some() {
                    self.game
                        .chat_window
                        .add_system("You are already in a guild.".to_string());
                } else if !has_emperium {
                    self.game
                        .chat_window
                        .add_system("You need an Emperium to create a guild.".to_string());
                } else {
                    self.channel
                        .send_packet(build_make_guild(gid, name, self.config.packetver));
                }
            }
            "/breakguild" => {
                let name = command["/breakguild".len()..].trim();
                if name.is_empty() {
                    self.game
                        .chat_window
                        .add_system("Usage: /breakguild <guild name>".to_string());
                } else if self.game.guild.is_none() {
                    self.game
                        .chat_window
                        .add_system("You are not in a guild.".to_string());
                } else {
                    self.channel
                        .send_packet(build_req_disorganize_guild(name, self.config.packetver));
                }
            }
            _ => {
                if let Some(emote_type) = ragnarok_game::emotion::emote_type_for_command(cmd) {
                    self.channel
                        .send_packet(build_emotion_packet(emote_type, self.config.packetver));
                } else {
                    self.game
                        .chat_window
                        .add_system(format!("Unknown command: {cmd}"));
                }
            }
        }
    }

    fn build_ui(&mut self, elapsed: f32) -> (Vec<UiDrawCall>, Vec<GameEvent>, bool, bool) {
        let now_ms = self.start_time.elapsed().as_millis() as u64;
        if let Some(ui_ctx) = &mut self.ui_context {
            ui_ctx.now_ms = now_ms;
        }
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
                    let events = create_win.build(&mut ui);
                    let any_hovered = ui.any_hovered;
                    let any_interactive = ui.any_interactive_hovered;
                    (ui.draw_calls, events, any_hovered, any_interactive)
                } else {
                    (Vec::new(), Vec::new(), false, false)
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
                        self.game.system_menu.has_grf_textures,
                        None,
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

    fn hovered_vending_board(&self, render_list: &[RenderEntry]) -> Option<u32> {
        let (mx, my) = self.input.mouse_position;
        let (mx, my) = (mx as f32, my as f32);
        for entry in render_list {
            let is_vendor = self
                .game
                .entities
                .get(entry.id)
                .is_some_and(|e| e.vending_board.is_some());
            if !is_vendor {
                continue;
            }
            let r = crate::overlay::vending_board_rect(entry);
            if mx >= r[0] && mx <= r[2] && my >= r[1] && my <= r[3] {
                return Some(entry.id);
            }
        }
        None
    }

    fn hovered_chat_room(&self, render_list: &[RenderEntry]) -> Option<u32> {
        let (mx, my) = self.input.mouse_position;
        let (mx, my) = (mx as f32, my as f32);
        for room in self.game.chat_rooms.iter() {
            let entry = match render_list.iter().find(|e| e.id == room.owner_aid) {
                Some(e) => e,
                None => continue,
            };
            let r = crate::overlay::chat_room_board_rect(entry);
            if mx >= r[0] && mx <= r[2] && my >= r[1] && my <= r[3] {
                return Some(room.room_id);
            }
        }
        None
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
                    let flat_depth_gradient = input::entity_ground_gradient(
                        entity.movement.position(),
                        self.game.gat.as_ref(),
                        coords,
                        &renderer.camera,
                        renderer.device.surface_config.width as f32 / renderer.dpi_scale,
                        renderer.device.surface_config.height as f32 / renderer.dpi_scale,
                    );
                    render_list.push(RenderEntry {
                        kind: RenderEntryKind::Entity,
                        id: entity.id,
                        screen_anchor,
                        depth,
                        depth_gradient,
                        flat_depth_gradient,
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
                        flat_depth_gradient: depth_gradient,
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

    fn compute_falcon_render_list(&self) -> Vec<RenderEntry> {
        let mut render_list = Vec::new();
        if let (Some(renderer), Some(coords)) = (&self.renderer, &self.game.map_coords) {
            let screen_w = renderer.device.surface_config.width as f32 / renderer.dpi_scale;
            let screen_h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
            for (gid, falcon) in self.game.falcons.iter() {
                if let Some((screen_anchor, depth, camera_dir, sprite_scale, depth_gradient)) =
                    input::project_world_screen(
                        falcon.motion.pos,
                        coords,
                        &renderer.camera,
                        screen_w,
                        screen_h,
                    )
                {
                    render_list.push(RenderEntry {
                        kind: RenderEntryKind::Falcon,
                        id: *gid,
                        screen_anchor,
                        depth,
                        depth_gradient,
                        flat_depth_gradient: depth_gradient,
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
                        flat_depth_gradient: depth_gradient,
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
        for (idx, present) in [self.has_homunculus(), self.has_mercenary()]
            .into_iter()
            .enumerate()
        {
            if let Some(t) = self.game.companion_attack_target[idx] {
                if !present || self.game.entities.get(t).is_none() {
                    self.game.companion_attack_target[idx] = None;
                }
            }
        }
        let companion_target_armed =
            self.game.companion_attack_target.iter().any(Option::is_some);
        self.game.hovered_chat_room = None;
        let (cursor, hovered_entity_id) = if self.game.app_state == AppState::InGame {
            if self.input.right_mouse_down {
                (CursorType::Rotate, None)
            } else if ui_any_interactive_hovered {
                (CursorType::Click, None)
            } else if ui_any_hovered {
                (CursorType::Default, None)
            } else if companion_target_armed {
                (CursorType::Lock, None)
            } else if let Some(pending) = &self.game.pending_companion_skill {
                if pending.is_ground {
                    (CursorType::Lock, None)
                } else {
                    // Resolve the entity under the cursor so the target click can
                    // find it; no class filter, since a companion skill may target
                    // either an enemy or an ally.
                    let hovered = hovered_entity_cursor_type(
                        self.input.mouse_position,
                        &self.game.entities,
                        render_list,
                        &self.game.map_properties,
                        None,
                    );
                    (CursorType::Lock, hovered.map(|(_, id)| id))
                }
            } else if self.game.capture_targeting {
                let hovered = hovered_entity_cursor_type(
                    self.input.mouse_position,
                    &self.game.entities,
                    render_list,
                    &self.game.map_properties,
                    Some(TargetClass::Offensive),
                );
                (CursorType::Lock, hovered.map(|(_, id)| id))
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
                        (CursorType::Lock, hovered.map(|(_, id)| id))
                    }
                    PendingSkillTarget::Ground { .. } => (CursorType::Lock, None),
                }
            } else if let Some(room_id) = self.hovered_chat_room(render_list) {
                self.game.hovered_chat_room = Some(room_id);
                (CursorType::Click, None)
            } else if let Some(vendor_id) = self.hovered_vending_board(render_list) {
                (CursorType::Click, Some(vendor_id))
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
        if self.config.fullscreen {
            window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
        }
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
                    self.game.data_table.skill_use_level = Some(SkillUseLevelTable::load(&grf));
                    self.game.data_table.quest_display =
                        Some(ragnarok_game::data_table::quest_display_table::QuestDisplayTable::load(&grf));
                    if let Ok(bytes) = grf.read_file("data/pettalktable.xml") {
                        self.game.data_table.pet_talk =
                            Some(ragnarok_formats::pettalk::PetTalkTable::parse(&bytes));
                    }
                    if let Ok(bytes) = grf.read_file("data/mp3nametable.txt") {
                        let text = String::from_utf8_lossy(&bytes);
                        self.bgm_table =
                            ragnarok_game::sound::bgm_table::parse_mp3_name_table(&text);
                    }
                    self.grf = Some(grf);
                }
                Err(e) => {
                    tracing::error!("Failed to open GRF {grf_path}: {e}");
                }
            }
        }

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

                if self.game.pending_disconnect_exit {
                    event_loop.exit();
                }
                let now = Instant::now();
                let raw_delta = now.duration_since(self.last_frame_instant).as_secs_f32();
                self.last_frame_instant = now;
                let delta = raw_delta.min(0.1);
                self.run_game_updates(delta, elapsed);
                self.drain_sound_queue(delta);

                let hovered = self.update_grid_hover();
                let render_list = self.compute_render_list();
                let floor_item_render_list = self.compute_floor_item_render_list();
                let mut cart_render_list = self.compute_cart_render_list();
                cart_render_list.extend(self.compute_falcon_render_list());
                // A stealthed actor the local player can't see is not hoverable or
                // attackable: drop it before hit-testing (self stays out of picking
                // regardless, so its shadow-only self view never enters here).
                let pick_render_list: Vec<RenderEntry> = render_list
                    .iter()
                    .filter(|entry| {
                        self.game.entities.get(entry.id).is_none_or(|e| {
                            hidden_render(e.effect_state, self.hidden_viewer_for(entry.id))
                                != HiddenRender::Skip
                        })
                    })
                    .copied()
                    .collect();
                let hovered_entity_id = self.update_cursor_type(
                    hovered,
                    ui_any_hovered,
                    ui_any_interactive,
                    &pick_render_list,
                );
                self.game.hovered_entity_id = hovered_entity_id;
                self.game.hovered_player_id = ragnarok_game::cursor::hovered_player(
                    self.input.mouse_position,
                    &self.game.entities,
                    &pick_render_list,
                );
                let hovered_named_id = hovered_entity_id.or(self.game.hovered_player_id);
                if let Some(entity_id) = hovered_named_id
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
                    hovered_named_id,
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
