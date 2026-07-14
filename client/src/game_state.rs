use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::config::WindowStateEntry;
use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use ragnarok_game::ailment::AilmentOverlay;
use ragnarok_game::app_state::AppState;
use ragnarok_game::arrow::ArrowProjectile;
use ragnarok_game::character::Character;
use ragnarok_game::chat_room::ChatRoomRegistry;
use ragnarok_game::cursor::{
    CursorAnimationState, PendingCompanionSkill, PendingSkillTarget, RenderEntry,
};
use ragnarok_game::damage_number::DamageNumberManager;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::effects::AmbientEffectScheduler;
use ragnarok_game::entity::EntityType;
use ragnarok_game::entity_collection::EntityCollection;
use ragnarok_game::event::{CharacterInfo, GameEvent};
use ragnarok_game::skill::SkillTargetType;
use ragnarok_game::floor_item::FloorItem;
use ragnarok_game::companion::{HomunculusState, MercenaryState};
use ragnarok_game::map_coordinates::MapCoordinates;
use ragnarok_game::party::Party;
use ragnarok_game::server_time::ServerTimeClock;
use ragnarok_game::sprite_path::JT_WARPNPC;
use ragnarok_game::targeting::MapProperties;
use ragnarok_network::session::Session;
use ragnarok_renderer::{EntitySprite, SpriteTextures};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::state::StateCache;
use ragnarok_ui_component::game::basic_info_window::{BASIC_INFO_WINDOW_ID, BasicInfoWindow};
use ragnarok_ui_component::game::card_insert_dialog::CardInsertDialog;
use ragnarok_ui_component::game::cart_select_window::{CART_SELECT_WINDOW_ID, CartSelectWindow};
use ragnarok_ui_component::game::cart_window::{CART_WINDOW_ID, CartWindow};
use ragnarok_ui_component::game::chat_room_window::{ChatRoomPlacement, ChatRoomWindow};
use ragnarok_ui_component::game::chat_window::{self, ChatWindow};
use ragnarok_ui_component::game::confirm_dialog::{ConfirmDialog, ConfirmResult};
use ragnarok_ui_component::game::context_menu::ContextMenu;
use ragnarok_ui_component::game::drop_quantity_dialog::DropQuantityDialog;
use ragnarok_ui_component::game::equipment_window::{EQ_WINDOW_ID, EquipmentWindow};
use ragnarok_ui_component::game::homun_skill_window::{HOMUN_SKILL_WINDOW_ID, HomunSkillWindow};
use ragnarok_ui_component::game::companion_ai_config_window::{
    COMPANION_AI_CONFIG_WINDOW_ID, CompanionAiConfigWindow,
};
use ragnarok_ui_component::game::homun_window::{HOMUN_WINDOW_ID, HomunWindow};
use ragnarok_ui_component::game::mercenary_skill_window::{
    MERCENARY_SKILL_WINDOW_ID, MercenarySkillWindow,
};
use ragnarok_ui_component::game::mercenary_window::{MERCENARY_WINDOW_ID, MercenaryWindow};
use ragnarok_ui_component::game::hotkey_bar::{HOTKEY_BAR_WINDOW_ID, HotkeyBarWindow};
use ragnarok_ui_component::game::inventory_window::{INV_WINDOW_ID, InventoryWindow};
use ragnarok_ui_component::game::book_window::{BOOK_WINDOW_ID, BookWindow};
use ragnarok_ui_component::game::sound_options::{SOUND_OPTIONS_WINDOW_ID, SoundOptionsWindow};
use ragnarok_ui_component::game::item_info_window::ItemInfoWindow;
use ragnarok_ui_component::game::item_pickup_notification::ItemPickupNotification;
use ragnarok_ui_component::game::minimap_window::{MarkerType, MinimapMarker, MinimapWindow};
use ragnarok_ui_component::game::npc_dialog::NpcDialog;
use ragnarok_ui_component::game::npc_shop::NpcShop;
use ragnarok_ui_component::game::emblem_picker_window::{
    EMBLEM_PICKER_WINDOW_ID, EmblemPickerWindow,
};
use ragnarok_ui_component::game::guild_window::{GUILD_WINDOW_ID, GuildWindow};
use ragnarok_ui_component::game::party_friends_window::{PARTY_FRIENDS_WINDOW_ID, PartyFriendsWindow};
use ragnarok_ui_component::game::party_helper_window::{PARTY_HELPER_WINDOW_ID, PartyHelperWindow};
use ragnarok_ui_component::game::skill_tree_window::{SKILL_WINDOW_ID, SkillTreeWindow};
use ragnarok_ui_component::game::levelup_notification_window::{
    LevelUpClick, LevelUpNotificationWindow,
};
use ragnarok_ui_component::game::status_icon_bar::StatusIconBarWindow;
use ragnarok_ui_component::game::status_window::{STATUS_WINDOW_ID, StatusWindow};
use ragnarok_ui_component::game::system_menu::SystemMenu;
use ragnarok_ui_component::game::map_missing_window::MapMissingWindow;
use ragnarok_ui_component::game::item_list_selection_window::ItemListSelectionWindow;
use ragnarok_ui_component::game::make_item_window::{MAKE_ITEM_WINDOW_ID, MakeItemWindow};
use ragnarok_ui_component::game::vending_setup_window::{
    VENDING_AVAILABLE_WINDOW_ID, VENDING_SETUP_WINDOW_ID, VendingSetupWindow,
};
use ragnarok_ui_component::game::vending_shop_window::{VENDING_SHOP_WINDOW_ID, VendingShopWindow};
use ragnarok_ui_component::game::my_shop_window::{MY_SHOP_WINDOW_ID, MyShopWindow};
use ragnarok_ui_component::game::warp_list_window::WarpListWindow;
use ragnarok_ui_component::{InGameWindow, Window};

/// One-shot ice-shatter animation played when a freeze ends, following the
/// entity by gid. `started_at` is lazily set to the render clock on first draw.
pub struct FreezeShatter {
    pub gid: u32,
    pub started_at: Option<f32>,
}

pub enum PendingGuildConfirm {
    Expel { aid: u32, gid: u32, name: String },
    Leave,
    DeleteRelation { gdid: u32, relation: i32 },
}

pub struct GameState {
    pub app_state: AppState,
    pub login_session: Option<Session>,
    pub selected_character: Option<CharacterInfo>,
    pub current_map: Option<String>,
    pub map_properties: MapProperties,
    pub requested_guild_emblems: HashSet<(u32, i32)>,
    pub map_coords: Option<MapCoordinates>,
    pub gat: Option<GatFile>,
    pub entities: EntityCollection,
    pub sprites: HashMap<u32, Rc<EntitySprite>>,
    pub guild_head_sprites: HashMap<u32, Rc<EntitySprite>>,
    pub sprite_cache: HashMap<String, Rc<EntitySprite>>,
    pub carts: HashMap<u32, crate::sprite::CartVisual>,
    pub falcons: HashMap<u32, crate::sprite::FalconVisual>,
    /// Deployed, visible traps keyed by unit AID → (trap unit id, world
    /// position): each shows a ground model and can fire its trigger burst.
    pub trap_units: HashMap<u32, (u8, [f32; 3])>,
    /// Traps placed hidden to us (cast by others); revealed to `trap_units` when
    /// the server sends a skill-unit update (e.g. an ankle snare springs).
    pub hidden_traps: HashMap<u32, (u8, [f32; 3])>,
    pub cart_preview_sprites: HashMap<u8, Rc<EntitySprite>>,
    pub character: Character,
    pub data_table: DataTable,
    pub cursor_textures: Option<SpriteTextures>,
    pub cursor_act: Option<ActFile>,
    pub cursor_animation: CursorAnimationState,
    pub lock_cursor_animation: CursorAnimationState,
    pub emotion_textures: Option<SpriteTextures>,
    pub emotion_act: Option<ActFile>,
    pub status_overlay_sprites: HashMap<AilmentOverlay, (SpriteTextures, ActFile)>,
    pub freeze_shatters: Vec<FreezeShatter>,
    pub chat_window: ChatWindow,
    pub equipment_window: EquipmentWindow,
    pub inventory_window: InventoryWindow,
    pub cart_window: CartWindow,
    pub cart_select_window: CartSelectWindow,
    pub npc_dialog: NpcDialog,
    pub warp_list_window: WarpListWindow,
    pub item_list_selection_window: ItemListSelectionWindow,
    pub make_item_window: MakeItemWindow,
    pub vending_shop_window: VendingShopWindow,
    pub vending_setup_window: VendingSetupWindow,
    pub my_shop_window: MyShopWindow,
    pub confirm_dialog: ConfirmDialog,
    pub npc_shop: NpcShop,
    pub chat_rooms: ChatRoomRegistry,
    pub chat_room_window: ChatRoomWindow,
    pub system_menu: SystemMenu,
    pub map_missing_window: MapMissingWindow,
    pub hovered_entity_id: Option<u32>,
    pub hovered_player_id: Option<u32>,
    pub hovered_floor_item_id: Option<u32>,
    pub failed_sprite_loads: HashSet<u32>,
    pub server_time: ServerTimeClock,
    pub attack_range: i16,
    pub floor_items: HashMap<u32, FloorItem>,
    pub floor_item_sprites: HashMap<u32, (Rc<SpriteTextures>, ActFile)>,
    pub arrows: Vec<ArrowProjectile>,
    pub waiting_item_throw_ack: bool,
    pub drop_dialog_has_grf_textures: bool,
    pub drop_quantity_dialog: Option<DropQuantityDialog>,
    pub card_insert_dialog: Option<CardInsertDialog>,
    pub card_insert_dialog_has_grf_textures: bool,
    pub pending_skill_target: Option<PendingSkillTarget>,
    pub pending_companion_skill: Option<PendingCompanionSkill>,
    pub pending_skill_id: Option<u16>,
    pub pending_skill_level: Option<i16>,
    pub pending_ground_cast: Option<(u16, i16, i16, i16)>,
    pub pending_card_composition_index: Option<u16>,
    /// Skill that opened the shared 0x01ad arrow/converter list, so the reply
    /// can be routed to the right context (the server disambiguates the same way
    /// via menuskill_id).
    pub pending_list_skill: Option<u16>,
    /// AID of the Repair Weapon target, remembered from the cast so the
    /// server's broken-item list can be attributed back to that player.
    pub pending_repair_target: Option<u32>,
    pub pending_pickup_item_id: Option<u32>,
    /// Shop title submitted with CZ_REQ_OPENSTORE2; applied to our own entity on
    /// open since the server sends ZC_STORE_ENTRY to everyone but us.
    pub pending_shop_name: Option<String>,
    pub attack_target_id: Option<u32>,
    pub last_attacked_enemy: Option<u32>,
    pub attack_request_cooldown: f32,
    pub noshift_mode: bool,
    pub noctrl_mode: bool,
    pub attack_is_locked: bool,
    pub item_info_window: ItemInfoWindow,
    pub book_window: BookWindow,
    pub sound_options: SoundOptionsWindow,
    pub item_pickup_notification: ItemPickupNotification,
    pub skill_tree_window: SkillTreeWindow,
    pub basic_info_window: BasicInfoWindow,
    pub status_window: StatusWindow,
    pub hotkey_bar: HotkeyBarWindow,
    pub minimap_window: MinimapWindow,
    pub status_icon_bar: StatusIconBarWindow,
    pub levelup_notification: LevelUpNotificationWindow,
    pub party: Option<Party>,
    pub guild: Option<ragnarok_game::guild::Guild>,
    pub guild_menu_flag: i32,
    pub guild_window: GuildWindow,
    pub emblem_picker_window: EmblemPickerWindow,
    pub friends: ragnarok_game::friends::FriendList,
    pub party_friends_window: PartyFriendsWindow,
    pub party_helper_window: PartyHelperWindow,
    pub pending_friend_request: Option<(u32, u32)>,
    pub friend_request_result: std::rc::Rc<std::cell::Cell<Option<ConfirmResult>>>,
    pub homunculus: Option<HomunculusState>,
    pub mercenary: Option<MercenaryState>,
    pub companion_ai: ragnarok_ai::config::CompanionAiConfig,
    pub companion_ai_config_window: CompanionAiConfigWindow,
    /// Target armed by the first click of the two-click owner attack, confirmed by
    /// the second. Index 0 = homunculus (Alt+right-click), 1 = mercenary (Alt+left-click).
    pub companion_attack_target: [Option<u32>; 2],
    pub homunculus_window: HomunWindow,
    pub mercenary_window: MercenaryWindow,
    pub mercenary_skill_window: MercenarySkillWindow,
    pub homun_skill_window: HomunSkillWindow,
    pub context_menu: ContextMenu,
    pub pending_party_invite: Option<u32>,
    pub party_invite_result: std::rc::Rc<std::cell::Cell<Option<ConfirmResult>>>,
    pub pending_guild_invite: Option<u32>,
    pub guild_invite_result: std::rc::Rc<std::cell::Cell<Option<ConfirmResult>>>,
    pub pending_guild_ally: Option<u32>,
    pub guild_ally_result: std::rc::Rc<std::cell::Cell<Option<ConfirmResult>>>,
    pub pending_guild_confirm: Option<PendingGuildConfirm>,
    pub guild_confirm_result: std::rc::Rc<std::cell::Cell<Option<ConfirmResult>>>,
    pub homun_delete_pending: bool,
    pub homun_delete_result: std::rc::Rc<std::cell::Cell<Option<ConfirmResult>>>,
    pub pending_invite_aid: Option<u32>,
    pub damage_numbers: DamageNumberManager,
    pub damage_number_textures: Option<SpriteTextures>,
    pub damage_number_act: Option<ragnarok_formats::act::ActFile>,
    pub damage_msg_textures: Option<SpriteTextures>,
    pub damage_msg_act: Option<ragnarok_formats::act::ActFile>,
    pub debug_show_pick_bounds: bool,
    pub debug_overlay: bool,
    pub ambient_effects: AmbientEffectScheduler,
    pub ambient_sounds: ragnarok_game::sound::ambient::AmbientSoundScheduler,
    pub repeat_sounds: ragnarok_game::sound::repeat::RepeatSoundScheduler,
    pub status_buff_keys: HashMap<(u32, i16), u32>,
    pub next_status_buff_key: u32,
    pub level_aura_keys: HashMap<u32, u32>,
    pub boss_aura_keys: HashMap<u32, u32>,
    pub warp_portal_keys: HashMap<u32, u32>,
    pub spirit_keys: HashMap<u32, u32>,
    pub sight_aura_keys: HashMap<u32, u32>,
    pub ruwach_aura_keys: HashMap<u32, u32>,
    pub disconnect_dialog_shown: bool,
    pub pending_disconnect_exit: bool,
}

pub const COMPANION_AI_CONFIG_PATH: &str = "companion_ai.json";

const Z_ORDERABLE_WINDOWS: &[WidgetId] = &[
    BASIC_INFO_WINDOW_ID,
    chat_window::CHAT_WINDOW_ID,
    INV_WINDOW_ID,
    CART_WINDOW_ID,
    CART_SELECT_WINDOW_ID,
    MAKE_ITEM_WINDOW_ID,
    VENDING_SHOP_WINDOW_ID,
    VENDING_SETUP_WINDOW_ID,
    VENDING_AVAILABLE_WINDOW_ID,
    MY_SHOP_WINDOW_ID,
    EQ_WINDOW_ID,
    SKILL_WINDOW_ID,
    STATUS_WINDOW_ID,
    PARTY_FRIENDS_WINDOW_ID,
    GUILD_WINDOW_ID,
    EMBLEM_PICKER_WINDOW_ID,
    PARTY_HELPER_WINDOW_ID,
    HOMUN_WINDOW_ID,
    MERCENARY_WINDOW_ID,
    MERCENARY_SKILL_WINDOW_ID,
    HOMUN_SKILL_WINDOW_ID,
    BOOK_WINDOW_ID,
    SOUND_OPTIONS_WINDOW_ID,
    COMPANION_AI_CONFIG_WINDOW_ID,
];

impl GameState {
    pub fn build_in_game_ui(
        &mut self,
        ui: &mut UiFrame,
        texture_size_fn: &dyn Fn(&str) -> Option<(u32, u32)>,
        render_list: &[RenderEntry],
    ) -> Vec<GameEvent> {
        let chat_was_active = self.chat_window.is_active();
        let mut events = Vec::new();

        self.npc_shop.setup_modal(ui);

        let z_order = ui.get_z_order();
        ui.compute_hovered_window(&z_order);
        for &win_id in &z_order {
            self.build_window(win_id, ui, &mut events);
        }
        for &win_id in Z_ORDERABLE_WINDOWS {
            if !z_order.contains(&win_id) {
                self.build_window(win_id, ui, &mut events);
            }
        }

        self.hotkey_bar.chat_is_active = self.chat_window.is_active();
        self.hotkey_bar.companion_skills.clear();
        if let Some(m) = &self.mercenary {
            self.hotkey_bar.companion_skills.extend(m.skills.iter().cloned());
        }
        if let Some(h) = &self.homunculus {
            self.hotkey_bar.companion_skills.extend(h.skills.iter().cloned());
        }
        events.extend(
            self.hotkey_bar
                .build(ui, &mut self.character, &self.data_table),
        );

        if let Some(player) = self.entities.player() {
            self.minimap_window.player_position = Some(player.movement.position());
            self.minimap_window.player_direction = player.direction;
        }
        if let Some(coords) = &self.map_coords {
            self.minimap_window.map_width = coords.gat_width();
            self.minimap_window.map_height = coords.gat_height();
        }
        self.minimap_window.map_name = self.current_map.clone();
        self.minimap_window.entity_markers.clear();
        for entity in self.entities.iter() {
            if Some(entity.id) == self.entities.player_id() {
                continue;
            }
            if entity.entity_type == EntityType::Npc {
                let (ex, ey) = entity.movement.position();
                let marker_type = if entity.job == JT_WARPNPC {
                    MarkerType::WarpPortal
                } else {
                    MarkerType::Npc
                };
                self.minimap_window.entity_markers.push(MinimapMarker {
                    x: ex,
                    y: ey,
                    marker_type,
                });
            }
        }
        if let Some(party) = &self.party {
            let local_aid = self
                .login_session
                .as_ref()
                .map(|s| s.account_id)
                .unwrap_or(0);
            let current_map = self.current_map.as_deref().unwrap_or("");
            for member in &party.members {
                if member.aid == local_aid || !member.online || member.map != current_map {
                    continue;
                }
                self.minimap_window.entity_markers.push(MinimapMarker {
                    x: member.x as f32,
                    y: member.y as f32,
                    marker_type: MarkerType::PartyMember,
                });
            }
        }
        events.extend(
            self.minimap_window
                .build(ui, &mut self.character, &self.data_table),
        );

        events.extend(
            self.status_icon_bar
                .build(ui, &mut self.character, &self.data_table),
        );

        match self.levelup_notification.build(ui) {
            LevelUpClick::Base => self.status_window.open(),
            LevelUpClick::Job => self.character.skills.open(),
            LevelUpClick::None => {}
        }

        self.chat_room_window.placements = self
            .chat_rooms
            .iter()
            .filter_map(|room| {
                let entry = render_list.iter().find(|e| e.id == room.owner_aid)?;
                Some(ChatRoomPlacement {
                    room_id: room.room_id,
                    atype: room.atype,
                    title: room.title.clone(),
                    cur_count: room.cur_count,
                    max_count: room.max_count,
                    anchor_x: entry.screen_anchor[0],
                    anchor_y: entry.screen_anchor[1],
                    head_offset: entry.head_offset,
                })
            })
            .collect();
        events.extend(
            self.chat_room_window
                .build(ui, &mut self.character, &self.data_table),
        );

        let npc_dialog_open = self.npc_dialog.dialog.is_open();
        events.extend(
            self.npc_dialog
                .build(ui, &mut self.character, &self.data_table),
        );
        let shop_open = self.npc_shop.shop.is_open();
        events.extend(
            self.npc_shop
                .build(ui, &mut self.character, &self.data_table),
        );
        let warp_list_open = self.warp_list_window.is_open();
        events.extend(self.warp_list_window.build(ui));
        let item_list_open = self.item_list_selection_window.is_open();
        events.extend(self.item_list_selection_window.build(ui));
        let mut allow_escape = !chat_was_active
            && !npc_dialog_open
            && !shop_open
            && !warp_list_open
            && !item_list_open;
        if allow_escape && ui.ctx.key_escape && self.pending_skill_target.is_some() {
            self.pending_skill_target = None;
            allow_escape = false;
        }
        self.system_menu.allow_escape_toggle = allow_escape;
        events.extend(
            self.system_menu
                .build(ui, &mut self.character, &self.data_table),
        );
        events.extend(
            self.item_info_window
                .build(ui, &mut self.character, &self.data_table),
        );
        events.extend(InGameWindow::build(
            &mut self.item_pickup_notification,
            ui,
            &mut self.character,
            &self.data_table,
        ));

        let had_disconnect_dialog =
            self.disconnect_dialog_shown && self.confirm_dialog.state.is_some();
        self.confirm_dialog.build(ui);
        if had_disconnect_dialog && self.confirm_dialog.state.is_none() {
            self.pending_disconnect_exit = true;
            self.disconnect_dialog_shown = false;
        }

        if let Some(grid) = self.pending_party_invite
            && let Some(result) = self.party_invite_result.take()
        {
            events.push(GameEvent::RespondPartyInvite {
                party_grid: grid,
                accept: result == ConfirmResult::Ok,
            });
            self.pending_party_invite = None;
        }

        if let Some(gdid) = self.pending_guild_invite
            && let Some(result) = self.guild_invite_result.take()
        {
            events.push(GameEvent::RespondGuildInvite {
                gdid,
                accept: result == ConfirmResult::Ok,
            });
            self.pending_guild_invite = None;
        }

        if let Some(aid) = self.pending_guild_ally
            && let Some(result) = self.guild_ally_result.take()
        {
            events.push(GameEvent::RespondGuildAlly {
                aid,
                accept: result == ConfirmResult::Ok,
            });
            self.pending_guild_ally = None;
        }

        if self.pending_guild_confirm.is_some()
            && let Some(result) = self.guild_confirm_result.take()
        {
            let pending = self.pending_guild_confirm.take().unwrap();
            if result == ConfirmResult::Ok {
                match pending {
                    PendingGuildConfirm::Expel { aid, gid, name } => {
                        events.push(GameEvent::ConfirmedGuildExpel { aid, gid, name });
                    }
                    PendingGuildConfirm::Leave => {
                        events.push(GameEvent::ConfirmedGuildLeave);
                    }
                    PendingGuildConfirm::DeleteRelation { gdid, relation } => {
                        events.push(GameEvent::ConfirmedDeleteGuildRelation { gdid, relation });
                    }
                }
            }
        }

        if let Some((req_aid, req_gid)) = self.pending_friend_request
            && let Some(result) = self.friend_request_result.take()
        {
            events.push(GameEvent::RespondFriendRequest {
                req_aid,
                req_gid,
                accept: result == ConfirmResult::Ok,
            });
            self.pending_friend_request = None;
        }

        if self.homun_delete_pending
            && let Some(result) = self.homun_delete_result.take()
        {
            self.homun_delete_pending = false;
            if result == ConfirmResult::Ok {
                events.push(GameEvent::RequestHomunMenu { command: 2 });
            }
        }

        events.extend(self.context_menu.build(ui));

        events.extend(self.map_missing_window.build(ui));

        ui.flush_tooltips();

        if let Some(cancelled) = ui.draw_drag_icon() {
            if cancelled.source_id == HOTKEY_BAR_WINDOW_ID {
                if self.character.hotkeys.get_slot(cancelled.item_index)
                    != ragnarok_game::hotkey::HotkeySlotContent::Empty
                {
                    self.character.hotkeys.clear_slot(cancelled.item_index);
                    events.push(GameEvent::RequestHotkeyChange {
                        index: cancelled.item_index as u16,
                        is_skill: false,
                        id: 0,
                        count: 0,
                    });
                }
            } else if cancelled.source_id == INV_WINDOW_ID {
                if self.waiting_item_throw_ack {
                } else if self.equipment_window.is_visible() {
                    self.chat_window
                        .add_system("Please close the Equipment window.".to_string());
                } else if let Some(item) = self
                    .character
                    .inventory
                    .get_item(cancelled.item_index as u16)
                {
                    if item.count > 1 {
                        let mut dialog = DropQuantityDialog::new(item.index, item.count);
                        dialog.has_grf_textures = self.drop_dialog_has_grf_textures;
                        if dialog.has_grf_textures {
                            dialog.set_texture_sizes(texture_size_fn);
                        }
                        self.drop_quantity_dialog = Some(dialog);
                    } else {
                        events.push(GameEvent::RequestDropItem {
                            index: item.index,
                            count: 1,
                        });
                        self.waiting_item_throw_ack = true;
                    }
                }
            }
        }

        if let Some(dialog) = &mut self.drop_quantity_dialog {
            let dialog_events =
                InGameWindow::build(dialog, ui, &mut self.character, &self.data_table);
            let closed = dialog_events.iter().any(|e| {
                matches!(
                    e,
                    GameEvent::DialogClosed | GameEvent::RequestDropItem { .. }
                )
            });
            if closed {
                if dialog_events
                    .iter()
                    .any(|e| matches!(e, GameEvent::RequestDropItem { .. }))
                {
                    self.waiting_item_throw_ack = true;
                }
                self.drop_quantity_dialog = None;
            }
            events.extend(
                dialog_events
                    .into_iter()
                    .filter(|e| !matches!(e, GameEvent::DialogClosed)),
            );
        }

        if let Some(dialog) = &mut self.card_insert_dialog {
            let dialog_events =
                InGameWindow::build(dialog, ui, &mut self.character, &self.data_table);
            let closed = dialog_events.iter().any(|e| {
                matches!(
                    e,
                    GameEvent::DialogClosed | GameEvent::RequestCardInsert { .. }
                )
            });
            if closed {
                self.card_insert_dialog = None;
            }
            events.extend(
                dialog_events
                    .into_iter()
                    .filter(|e| !matches!(e, GameEvent::DialogClosed)),
            );
        }

        events
    }

    fn build_window(&mut self, win_id: WidgetId, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        match win_id {
            BASIC_INFO_WINDOW_ID => events.extend(self.basic_info_window.build(
                ui,
                &mut self.character,
                &self.data_table,
            )),
            chat_window::CHAT_WINDOW_ID => events.extend(self.chat_window.build(
                ui,
                &mut self.character,
                &self.data_table,
            )),
            INV_WINDOW_ID => events.extend(self.inventory_window.build(
                ui,
                &mut self.character,
                &self.data_table,
            )),
            CART_WINDOW_ID => events.extend(self.cart_window.build(
                ui,
                &mut self.character,
                &self.data_table,
            )),
            MAKE_ITEM_WINDOW_ID => events.extend(self.make_item_window.build(
                ui,
                &mut self.character,
                &self.data_table,
            )),
            VENDING_SHOP_WINDOW_ID => events.extend(self.vending_shop_window.build(
                ui,
                &mut self.character,
                &self.data_table,
            )),
            VENDING_SETUP_WINDOW_ID => events.extend(self.vending_setup_window.build(
                ui,
                &mut self.character,
                &self.data_table,
            )),
            VENDING_AVAILABLE_WINDOW_ID => events.extend(self.vending_setup_window.build_available(
                ui,
                &mut self.character,
                &self.data_table,
            )),
            MY_SHOP_WINDOW_ID => events.extend(self.my_shop_window.build(
                ui,
                &mut self.character,
                &self.data_table,
            )),
            CART_SELECT_WINDOW_ID => events.extend(self.cart_select_window.build(
                ui,
                &mut self.character,
                &self.data_table,
            )),
            EQ_WINDOW_ID => {
                events.extend(self.equipment_window.build(
                    ui,
                    &mut self.character,
                    &self.data_table,
                ));
            }
            SKILL_WINDOW_ID => {
                events.extend(self.skill_tree_window.build(
                    ui,
                    &mut self.character,
                    &self.data_table,
                ));
            }
            STATUS_WINDOW_ID => {
                events.extend(
                    self.status_window
                        .build(ui, &mut self.character, &self.data_table),
                );
            }
            PARTY_FRIENDS_WINDOW_ID => {
                let local_aid = self
                    .login_session
                    .as_ref()
                    .map(|s| s.account_id)
                    .unwrap_or(0);
                // The server never sends our own HP via party, and same-map members' HP only
                // arrives on change — so refresh rows from live state each frame.
                if let Some(party) = &mut self.party {
                    for m in &mut party.members {
                        if m.aid == local_aid {
                            m.hp = Some(self.character.hp);
                            m.max_hp = Some(self.character.max_hp);
                            if let Some(p) = self.entities.player() {
                                (m.x, m.y) = p.movement.cell_position();
                            }
                        } else if let Some(e) = self.entities.get(m.aid) {
                            if let (Some(hp), Some(max_hp)) = (e.hp, e.max_hp) {
                                m.hp = Some(hp);
                                m.max_hp = Some(max_hp);
                            }
                            (m.x, m.y) = e.movement.cell_position();
                        }
                    }
                }
                self.party_friends_window
                    .sync(self.party.as_ref(), &self.friends.friends, local_aid);
                events.extend(self.party_friends_window.build(
                    ui,
                    &mut self.character,
                    &self.data_table,
                ));
            }
            PARTY_HELPER_WINDOW_ID => {
                events.extend(self.party_helper_window.build(
                    ui,
                    &mut self.character,
                    &self.data_table,
                ));
            }
            GUILD_WINDOW_ID => {
                let local_gid = self
                    .login_session
                    .as_ref()
                    .map(|s| s.account_id)
                    .unwrap_or(0);
                self.guild_window
                    .sync(self.guild.as_ref(), local_gid, &self.character.name);
                events.extend(self.guild_window.build(
                    ui,
                    &mut self.character,
                    &self.data_table,
                ));
            }
            EMBLEM_PICKER_WINDOW_ID => {
                events.extend(self.emblem_picker_window.build(
                    ui,
                    &mut self.character,
                    &self.data_table,
                ));
            }
            COMPANION_AI_CONFIG_WINDOW_ID => {
                events.extend(
                    self.companion_ai_config_window
                        .build(ui, &mut self.companion_ai),
                );
            }
            HOMUN_WINDOW_ID => {
                events.extend(self.homunculus_window.build(ui, self.homunculus.as_ref()));
            }
            MERCENARY_WINDOW_ID => {
                events.extend(self.mercenary_window.build(ui, self.mercenary.as_ref()));
            }
            MERCENARY_SKILL_WINDOW_ID => {
                events.extend(self.mercenary_skill_window.build(
                    ui,
                    self.mercenary.as_ref(),
                    &self.data_table,
                ));
            }
            HOMUN_SKILL_WINDOW_ID => {
                events.extend(self.homun_skill_window.build(
                    ui,
                    self.homunculus.as_ref(),
                    &self.data_table,
                ));
            }
            BOOK_WINDOW_ID => {
                events.extend(
                    self.book_window
                        .build(ui, &mut self.character, &self.data_table),
                );
            }
            SOUND_OPTIONS_WINDOW_ID => {
                events.extend(
                    self.sound_options
                        .build(ui, &mut self.character, &self.data_table),
                );
            }
            _ => {}
        }
    }

    /// Resolves a skill's cast metadata `(target type, attack range)` from the
    /// player's skills first, then the mercenary's, then the homunculus'.
    /// Companion skill IDs live in their own ranges, so the lookup order is
    /// unambiguous. The cast packet is identical for all three; the server
    /// attributes companion-range IDs to the companion.
    pub fn resolve_cast_skill(&self, skill_id: u16) -> Option<(SkillTargetType, i16)> {
        if let Some(s) = self.character.skills.get_skill(skill_id) {
            return Some((s.skill_target_type, s.attack_range));
        }
        if let Some(m) = &self.mercenary
            && let Some(s) = m.skills.iter().find(|s| s.id == skill_id)
        {
            return Some((s.skill_target_type, s.attack_range));
        }
        if let Some(h) = &self.homunculus
            && let Some(s) = h.skills.iter().find(|s| s.id == skill_id)
        {
            return Some((s.skill_target_type, s.attack_range));
        }
        None
    }

    pub fn new() -> Self {
        Self {
            app_state: AppState::Login,
            login_session: None,
            selected_character: None,
            current_map: None,
            map_properties: MapProperties::default(),
            requested_guild_emblems: HashSet::new(),
            map_coords: None,
            gat: None,
            entities: EntityCollection::new(),
            sprites: HashMap::new(),
            guild_head_sprites: HashMap::new(),
            carts: HashMap::new(),
            falcons: HashMap::new(),
            trap_units: HashMap::new(),
            hidden_traps: HashMap::new(),
            cart_preview_sprites: HashMap::new(),
            sprite_cache: HashMap::new(),
            character: Character::new(),
            data_table: DataTable::new(),
            cursor_textures: None,
            cursor_act: None,
            cursor_animation: CursorAnimationState::new(),
            lock_cursor_animation: CursorAnimationState::new(),
            emotion_textures: None,
            emotion_act: None,
            status_overlay_sprites: HashMap::new(),
            freeze_shatters: Vec::new(),
            chat_window: ChatWindow::new(),
            equipment_window: EquipmentWindow::new(),
            inventory_window: InventoryWindow::new(),
            cart_window: CartWindow::new(),
            cart_select_window: CartSelectWindow::new(),
            npc_dialog: NpcDialog::new(),
            warp_list_window: WarpListWindow::new(),
            item_list_selection_window: ItemListSelectionWindow::new(),
            make_item_window: MakeItemWindow::new(),
            vending_shop_window: VendingShopWindow::new(),
            vending_setup_window: VendingSetupWindow::new(),
            my_shop_window: MyShopWindow::new(),
            confirm_dialog: ConfirmDialog::new(),
            npc_shop: NpcShop::new(),
            chat_rooms: ChatRoomRegistry::new(),
            chat_room_window: ChatRoomWindow::new(),
            system_menu: SystemMenu::new(),
            map_missing_window: MapMissingWindow::new(),
            hovered_entity_id: None,
            hovered_player_id: None,
            hovered_floor_item_id: None,
            failed_sprite_loads: HashSet::new(),
            server_time: ServerTimeClock::new(),
            attack_range: 1,
            floor_items: HashMap::new(),
            floor_item_sprites: HashMap::new(),
            arrows: Vec::new(),
            waiting_item_throw_ack: false,
            drop_dialog_has_grf_textures: false,
            drop_quantity_dialog: None,
            card_insert_dialog: None,
            card_insert_dialog_has_grf_textures: false,
            pending_skill_target: None,
            pending_companion_skill: None,
            pending_skill_id: None,
            pending_skill_level: None,
            pending_ground_cast: None,
            pending_card_composition_index: None,
            pending_list_skill: None,
            pending_repair_target: None,
            pending_pickup_item_id: None,
            pending_shop_name: None,
            attack_target_id: None,
            last_attacked_enemy: None,
            attack_request_cooldown: 0.0,
            noshift_mode: false,
            noctrl_mode: true,
            attack_is_locked: false,
            basic_info_window: BasicInfoWindow::new(),
            status_window: StatusWindow::new(),
            item_info_window: ItemInfoWindow::new(),
            book_window: BookWindow::new(),
            sound_options: SoundOptionsWindow::new(),
            item_pickup_notification: ItemPickupNotification::new(),
            skill_tree_window: SkillTreeWindow::new(),
            hotkey_bar: HotkeyBarWindow::new(),
            minimap_window: MinimapWindow::new(),
            status_icon_bar: StatusIconBarWindow::new(),
            levelup_notification: LevelUpNotificationWindow::new(),
            party: None,
            guild: None,
            guild_menu_flag: 0,
            guild_window: GuildWindow::new(),
            emblem_picker_window: EmblemPickerWindow::new(),
            friends: ragnarok_game::friends::FriendList::default(),
            party_friends_window: PartyFriendsWindow::new(),
            party_helper_window: PartyHelperWindow::new(),
            pending_friend_request: None,
            friend_request_result: std::rc::Rc::new(std::cell::Cell::new(None)),
            homunculus: None,
            mercenary: None,
            companion_attack_target: [None; 2],
            homunculus_window: HomunWindow::new(),
            mercenary_window: MercenaryWindow::new(),
            companion_ai: ragnarok_ai::config::CompanionAiConfig::load_or_default(
                COMPANION_AI_CONFIG_PATH,
            ),
            companion_ai_config_window: CompanionAiConfigWindow::new(),
            mercenary_skill_window: MercenarySkillWindow::new(),
            homun_skill_window: HomunSkillWindow::new(),
            context_menu: ContextMenu::new(),
            pending_party_invite: None,
            party_invite_result: std::rc::Rc::new(std::cell::Cell::new(None)),
            pending_guild_invite: None,
            guild_invite_result: std::rc::Rc::new(std::cell::Cell::new(None)),
            pending_guild_ally: None,
            guild_ally_result: std::rc::Rc::new(std::cell::Cell::new(None)),
            pending_guild_confirm: None,
            guild_confirm_result: std::rc::Rc::new(std::cell::Cell::new(None)),
            homun_delete_pending: false,
            homun_delete_result: std::rc::Rc::new(std::cell::Cell::new(None)),
            pending_invite_aid: None,
            disconnect_dialog_shown: false,
            pending_disconnect_exit: false,
            damage_numbers: DamageNumberManager::new(),
            damage_number_textures: None,
            damage_number_act: None,
            damage_msg_textures: None,
            damage_msg_act: None,
            debug_show_pick_bounds: false,
            debug_overlay: false,
            ambient_effects: AmbientEffectScheduler::empty(),
            ambient_sounds: ragnarok_game::sound::ambient::AmbientSoundScheduler::empty(),
            repeat_sounds: ragnarok_game::sound::repeat::RepeatSoundScheduler::new(),
            status_buff_keys: HashMap::new(),
            next_status_buff_key: 0,
            level_aura_keys: HashMap::new(),
            boss_aura_keys: HashMap::new(),
            warp_portal_keys: HashMap::new(),
            spirit_keys: HashMap::new(),
            sight_aura_keys: HashMap::new(),
            ruwach_aura_keys: HashMap::new(),
        }
    }

    pub fn apply_window_state(&mut self, window_state: &HashMap<u32, WindowStateEntry>) {
        if let Some(entry) = window_state.get(&INV_WINDOW_ID.0) {
            if entry.open {
                self.character.inventory.open();
            }
            self.inventory_window.set_minimized(entry.collapsed);
        }
        if let Some(entry) = window_state.get(&EQ_WINDOW_ID.0) {
            self.equipment_window.open = entry.open;
            self.equipment_window.set_minimized(entry.collapsed);
        }
        if let Some(entry) = window_state.get(&SKILL_WINDOW_ID.0)
            && entry.open
        {
            self.character.skills.open();
        }
        if let Some(entry) = window_state.get(&chat_window::CHAT_WINDOW_ID.0) {
            let size_index = if !entry.open {
                0
            } else if entry.collapsed {
                1
            } else {
                5
            };
            self.chat_window.set_initial_size_index(size_index);
        }
    }

    pub fn extract_window_state(&self, state_cache: &StateCache) -> HashMap<u32, (bool, bool)> {
        let mut result = HashMap::new();
        result.insert(
            INV_WINDOW_ID.0,
            (
                self.character.inventory.is_open(),
                self.inventory_window.is_minimized(),
            ),
        );
        result.insert(
            EQ_WINDOW_ID.0,
            (
                self.equipment_window.is_open(),
                self.equipment_window.is_minimized(),
            ),
        );
        result.insert(SKILL_WINDOW_ID.0, (self.character.skills.is_open(), false));
        let size_index = self.chat_window.get_size_index(state_cache);
        result.insert(
            chat_window::CHAT_WINDOW_ID.0,
            (size_index > 0, size_index == 1),
        );
        result
    }
}

#[cfg(test)]
mod skill_resolve_tests {
    use super::*;
    use ragnarok_game::companion::MercenaryState;
    use ragnarok_game::event::SkillInfo;

    #[test]
    fn resolves_mercenary_skill_metadata_for_cast() {
        let mut game = GameState::new();
        let mut merc = MercenaryState::new(2000);
        merc.skills = vec![SkillInfo {
            id: 8201,
            name: "MS_BASH".to_string(),
            level: 3,
            sp_cost: 15,
            attack_range: 9,
            upgradable: false,
            skill_target_type: SkillTargetType::Target,
        }];
        game.mercenary = Some(merc);

        assert_eq!(
            game.resolve_cast_skill(8201),
            Some((SkillTargetType::Target, 9))
        );
        assert_eq!(game.resolve_cast_skill(9999), None);
    }
}
