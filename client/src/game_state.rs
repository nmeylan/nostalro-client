use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::config::WindowStateEntry;
use crate::ui::windows::Windows;
use models::enums::effect_id::EffectId;
use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use ragnarok_formats::map_coordinates::MapCoordinates;
use ragnarok_game::ailment::AilmentOverlay;
use ragnarok_game::app_state::AppState;
use ragnarok_game::arrow::ArrowProjectile;
use ragnarok_game::banner::BannerState;
use ragnarok_game::character::Character;
use ragnarok_game::chat_room::ChatRoomRegistry;
use ragnarok_game::companion::{HomunculusState, MercenaryState};
use ragnarok_game::cursor::{
    CursorAnimationState, CursorType, PendingCompanionSkill, PendingSkillTarget,
};
use ragnarok_game::damage_number::DamageNumberManager;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::day_night::DayNightState;
use ragnarok_game::effect::EffectQueue;
use ragnarok_game::effects::AmbientEffectScheduler;
use ragnarok_game::entity_collection::EntityCollection;
use ragnarok_game::event::{CharacterInfo, GameEvent};
use ragnarok_game::floor_item::FloorItem;
use ragnarok_game::gr2_model::Gr2ModelInstance;
use ragnarok_game::party::Party;
use ragnarok_game::pet::PetState;
use ragnarok_game::poptip::PoptipStack;
use ragnarok_game::quest::{QuestLog, QuestMarker};
use ragnarok_game::server_time::ServerTimeClock;
use ragnarok_game::skill::SkillTargetType;
use ragnarok_game::targeting::MapProperties;
use ragnarok_network::session::Session;
use ragnarok_renderer::{EntitySprite, SpriteTextures};
use ragnarok_ui::state::StateCache;
use ragnarok_ui_component::game::chat_window::{self};
use ragnarok_ui_component::game::confirm_dialog::ConfirmResult;
use ragnarok_ui_component::game::equipment_window::EQ_WINDOW_ID;
use ragnarok_ui_component::game::inventory_window::INV_WINDOW_ID;
use ragnarok_ui_component::game::skill_tree_window::SKILL_WINDOW_ID;

/// One-shot ice-shatter animation played when a freeze ends, following the
/// entity by gid. `started_at` is lazily set to the render clock on first draw.
pub struct FreezeShatter {
    pub gid: u32,
    pub started_at: Option<f32>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SelfConfig {
    pub refuse_party_invite: bool,
    pub open_equipment_window: bool,
    pub call_enabled: bool,
    pub pet_autofeed: bool,
    pub homun_autofeed: bool,
}

#[derive(Default)]
pub struct PendingConfirms {
    pub pending_trade_partner: Option<(u32, String)>,
    pub pending_trade_request: Option<u32>,
    pub pending_adopt_request: Option<(u32, u32)>,
    pub pending_invite_aid: Option<u32>,
    active: Option<Box<dyn FnOnce(bool) -> Option<GameEvent>>>,
}

impl PendingConfirms {
    pub(crate) fn dispatch(&mut self, result: ConfirmResult) -> Option<GameEvent> {
        let ctor = self.active.take()?;
        ctor(result == ConfirmResult::Ok)
    }
}

pub struct CombatState {
    pub attack_target_id: Option<u32>,
    pub last_attacked_enemy: Option<u32>,
    pub attack_request_cooldown: f32,
    pub attack_range: i16,
    pub attack_is_locked: bool,
    pub waiting_item_throw_ack: bool,
    pub damage_numbers: DamageNumberManager,
    /// Destination clicked while the swing (or a pickup/hurt motion) held the
    /// character; sent as soon as the motion releases.
    pub queued_move: Option<(i32, i32)>,
}

impl Default for CombatState {
    fn default() -> Self {
        Self::new()
    }
}

impl CombatState {
    pub fn new() -> Self {
        Self {
            attack_target_id: None,
            last_attacked_enemy: None,
            attack_request_cooldown: 0.0,
            attack_range: 1,
            attack_is_locked: false,
            waiting_item_throw_ack: false,
            damage_numbers: DamageNumberManager::new(),
            queued_move: None,
        }
    }
}

#[derive(Default)]
pub struct EffectKeys {
    pub status_buff_keys: HashMap<(u32, i16), u32>,
    pub next_status_buff_key: u32,
    pub level_aura_keys: HashMap<u32, u32>,
    pub boss_aura_keys: HashMap<u32, u32>,
    /// gid -> (effect key, rank the tint was baked from)
    pub toprank_keys: HashMap<u32, (u32, i32)>,
    pub warp_portal_keys: HashMap<u32, u32>,
    pub spirit_keys: HashMap<u32, u32>,
    pub sight_aura_keys: HashMap<u32, u32>,
    pub ruwach_aura_keys: HashMap<u32, u32>,
    pub weather_keys: HashMap<EffectId, u32>,
}

impl EffectKeys {
    pub fn clear(&mut self) {
        self.status_buff_keys.clear();
        self.next_status_buff_key = 0;
        self.level_aura_keys.clear();
        self.boss_aura_keys.clear();
        self.toprank_keys.clear();
        self.warp_portal_keys.clear();
        self.spirit_keys.clear();
        self.sight_aura_keys.clear();
        self.ruwach_aura_keys.clear();
        self.weather_keys.clear();
    }
}

#[derive(Default)]
pub struct HoverState {
    pub hovered_entity_id: Option<u32>,
    pub hovered_player_id: Option<u32>,
    pub hovered_floor_item_id: Option<u32>,
    pub hovered_chat_room: Option<u32>,
    pub hovered_vending: Option<u32>,
    pub hovered_entity_cursor: Option<CursorType>,
    pub cell_cursor: CursorType,
}

impl HoverState {
    /// The id to target for a click: the hovered entity, or the owner of a
    /// hovered vending board (both share the same id).
    pub fn target_id(&self) -> Option<u32> {
        self.hovered_entity_id.or(self.hovered_vending)
    }
}

pub struct CursorInput {
    pub in_game: bool,
    pub right_mouse_down: bool,
    pub ui_any_hovered: bool,
    pub ui_any_interactive_hovered: bool,
    pub item_drag_active: bool,
}

pub struct CursorPending {
    pub companion_target_armed: bool,
    pub pending_companion_skill: bool,
    pub capture_targeting: bool,
    pub pending_skill: bool,
}

pub fn cursor_type_from_hover(
    hover: &HoverState,
    input: CursorInput,
    pending: CursorPending,
) -> CursorType {
    if !input.in_game {
        return if input.ui_any_interactive_hovered {
            CursorType::Click
        } else {
            CursorType::Default
        };
    }
    let base = if input.right_mouse_down {
        CursorType::Rotate
    } else if input.item_drag_active {
        CursorType::Default
    } else if input.ui_any_interactive_hovered {
        CursorType::Click
    } else if input.ui_any_hovered {
        CursorType::Default
    } else if pending.companion_target_armed
        || pending.pending_companion_skill
        || pending.capture_targeting
        || pending.pending_skill
    {
        CursorType::Lock
    } else if hover.hovered_chat_room.is_some() {
        CursorType::Click
    } else if hover.hovered_vending.is_some() {
        CursorType::Click
    } else if let Some(cursor) = hover.hovered_entity_cursor {
        cursor
    } else {
        hover.cell_cursor
    };
    if hover.hovered_floor_item_id.is_some() && !input.item_drag_active {
        CursorType::Pick
    } else {
        base
    }
}

#[derive(Default)]
pub struct PendingCasts {
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
}

pub struct Companions {
    pub homunculus: Option<HomunculusState>,
    pub mercenary: Option<MercenaryState>,
    pub pet: PetState,
    pub companion_ai: ragnarok_ai::config::CompanionAiConfig,
    /// Target armed by the first click of the two-click owner attack, confirmed by
    /// the second. Index 0 = homunculus (Alt+right-click), 1 = mercenary (Alt+left-click).
    pub companion_attack_target: [Option<u32>; 2],
    /// Armed by ZC_START_CAPTURE: the next click on a valid mob opens the roulette.
    pub capture_targeting: bool,
    pub pet_roulette: Option<ragnarok_game::pet::PetRoulette>,
}

pub struct SessionState {
    pub app_state: AppState,
    pub login_session: Option<Session>,
    pub selected_character: Option<CharacterInfo>,
    pub current_map: Option<String>,
    pub map_properties: MapProperties,
    pub map_coords: Option<MapCoordinates>,
    pub gat: Option<GatFile>,
    pub player_dead: bool,
    /// Set on indoor maps; locks the camera rotation to the fixed indoor angle.
    pub camera_locked: bool,
    /// Camera yaw captured when entering an indoor map, restored on exit.
    pub saved_camera_yaw: Option<f32>,
    pub server_time: ServerTimeClock,
    pub disconnect_dialog_shown: bool,
    pub pending_disconnect_exit: bool,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            app_state: AppState::Login,
            login_session: None,
            selected_character: None,
            current_map: None,
            map_properties: MapProperties::default(),
            map_coords: None,
            gat: None,
            player_dead: false,
            camera_locked: false,
            saved_camera_yaw: None,
            server_time: ServerTimeClock::new(),
            disconnect_dialog_shown: false,
            pending_disconnect_exit: false,
        }
    }
}

#[derive(Default)]
pub struct World {
    pub entities: EntityCollection,
    pub floor_items: HashMap<u32, FloorItem>,
    pub arrows: Vec<ArrowProjectile>,
    /// Deployed, visible traps keyed by unit AID → (trap unit id, world
    /// position): each shows a ground model and can fire its trigger burst.
    pub trap_units: HashMap<u32, (u8, [f32; 3])>,
    /// Traps placed hidden to us (cast by others); revealed to `trap_units` when
    /// the server sends a skill-unit update (e.g. an ankle snare springs).
    pub hidden_traps: HashMap<u32, (u8, [f32; 3])>,
    pub freeze_shatters: Vec<FreezeShatter>,
}

#[derive(Default)]
pub struct SpriteCaches {
    pub sprites: HashMap<u32, Rc<EntitySprite>>,
    /// Animation state of GR2 model entities (emperium, guardians…) keyed by
    /// gid; the matching draw resources live in `Renderer::gr2_models`.
    pub gr2_models: HashMap<u32, Gr2ModelInstance>,
    pub guild_head_sprites: HashMap<u32, Rc<EntitySprite>>,
    pub sprite_cache: HashMap<String, Rc<EntitySprite>>,
    pub carts: HashMap<u32, crate::sprite::CartVisual>,
    pub falcons: HashMap<u32, crate::sprite::FalconVisual>,
    pub cart_preview_sprites: HashMap<u8, Rc<EntitySprite>>,
    pub failed_sprite_loads: HashSet<u32>,
}

#[derive(Default)]
pub struct AssetHandles {
    pub cursor_textures: Option<SpriteTextures>,
    pub cursor_act: Option<ActFile>,
    pub cursor_animation: CursorAnimationState,
    pub lock_cursor_animation: CursorAnimationState,
    pub emotion_textures: Option<SpriteTextures>,
    pub emotion_act: Option<ActFile>,
    pub status_overlay_sprites: HashMap<AilmentOverlay, (SpriteTextures, ActFile)>,
    pub floor_item_sprites: HashMap<u32, (Rc<SpriteTextures>, ActFile)>,
    pub damage_number_textures: Option<SpriteTextures>,
    pub damage_number_act: Option<ragnarok_formats::act::ActFile>,
    pub damage_msg_textures: Option<SpriteTextures>,
    pub damage_msg_act: Option<ragnarok_formats::act::ActFile>,
    pub rank_font_textures: Option<SpriteTextures>,
    pub rank_font_act: Option<ragnarok_formats::act::ActFile>,
}

pub struct Schedulers {
    pub ambient_effects: AmbientEffectScheduler,
    pub ambient_sounds: ragnarok_game::sound::ambient::AmbientSoundScheduler,
    pub repeat_sounds: ragnarok_game::sound::repeat::RepeatSoundScheduler,
    pub day_night: DayNightState,
}

impl Default for Schedulers {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedulers {
    pub fn new() -> Self {
        Self {
            ambient_effects: AmbientEffectScheduler::empty(),
            ambient_sounds: ragnarok_game::sound::ambient::AmbientSoundScheduler::empty(),
            repeat_sounds: ragnarok_game::sound::repeat::RepeatSoundScheduler::new(),
            day_night: DayNightState::default(),
        }
    }
}

pub struct Broadcast {
    pub banner: BannerState,
    pub poptip: PoptipStack,
    pub broadcast_last_elapsed: Option<f32>,
}

impl Default for Broadcast {
    fn default() -> Self {
        Self::new()
    }
}

impl Broadcast {
    pub fn new() -> Self {
        Self {
            banner: BannerState::new(),
            poptip: PoptipStack::new(),
            broadcast_last_elapsed: None,
        }
    }
}

pub struct Prefs {
    pub noshift_mode: bool,
    pub noctrl_mode: bool,
    pub show_exp: bool,
    pub hide_public_chat: bool,
    pub show_miss: bool,
    pub equip_open: bool,
    pub blocked_whispers: Vec<String>,
    pub self_config: SelfConfig,
}

impl Default for Prefs {
    fn default() -> Self {
        Self::new()
    }
}

impl Prefs {
    pub fn new() -> Self {
        Self {
            noshift_mode: false,
            noctrl_mode: true,
            show_exp: true,
            hide_public_chat: false,
            show_miss: true,
            equip_open: false,
            blocked_whispers: Vec::new(),
            self_config: SelfConfig::default(),
        }
    }
}

pub struct GameState {
    pub session: SessionState,
    pub requested_guild_emblems: HashSet<(u32, i32)>,
    pub world: World,
    pub sprite_caches: SpriteCaches,
    pub character: Character,
    pub data_table: DataTable,
    pub assets: AssetHandles,
    pub broadcast: Broadcast,
    pub pending_confirms: PendingConfirms,
    /// NPC cutin illustrations by position slot (0 left / 1 middle / 2 right);
    /// GRF base name under `data/texture/유저인터페이스/illust/`.
    pub npc_cutins: [Option<String>; 3],
    pub chat_rooms: ChatRoomRegistry,
    pub pending_chat_room: Option<(String, i16, bool)>,
    pub hover: HoverState,
    pub pending_casts: PendingCasts,
    pub combat: CombatState,
    pub prefs: Prefs,
    pub party: Option<Party>,
    pub guild: Option<ragnarok_game::guild::Guild>,
    pub guild_menu_flag: i32,
    pub friends: ragnarok_game::friends::FriendList,
    pub companions: Companions,
    pub quest_log: QuestLog,
    /// Over-NPC quest markers keyed by NPC block id (account-id space). Cleared
    /// on map change; the server re-sends on load.
    pub quest_markers: std::collections::HashMap<u32, QuestMarker>,
    pub debug_show_pick_bounds: bool,
    pub show_ping: bool,
    pub show_fps: bool,
    pub schedulers: Schedulers,
    pub effect_keys: EffectKeys,
}

pub const COMPANION_AI_CONFIG_PATH: &str = "companion_ai.json";

/// Item id of the Token of Siegfried, which enables standing resurrection.
pub(crate) const TOKEN_OF_SIEGFRIED: u16 = 7621;

impl GameState {
    /// Resolves a skill's cast metadata `(target type, attack range)` from the
    /// player's skills first, then the mercenary's, then the homunculus'.
    /// Companion skill IDs live in their own ranges, so the lookup order is
    /// unambiguous. The cast packet is identical for all three; the server
    /// attributes companion-range IDs to the companion.
    pub fn resolve_cast_skill(&self, skill_id: u16) -> Option<(SkillTargetType, i16)> {
        if let Some(s) = self.character.skills.get_skill(skill_id) {
            return Some((s.skill_target_type, s.attack_range));
        }
        if let Some(m) = &self.companions.mercenary
            && let Some(s) = m.skills.iter().find(|s| s.id == skill_id)
        {
            return Some((s.skill_target_type, s.attack_range));
        }
        if let Some(h) = &self.companions.homunculus
            && let Some(s) = h.skills.iter().find(|s| s.id == skill_id)
        {
            return Some((s.skill_target_type, s.attack_range));
        }
        None
    }

    /// Reads the player's cart design out of the `OPTION_CART` bits carried in
    /// `effect_state` and records it on the entity and character. Returns the
    /// `(player gid, design)` to spawn, or `None` when the player has no cart.
    /// Servers that deliver the cart as a status change instead leave these
    /// bits clear, so this is a no-op there and the status path takes over.
    pub fn player_cart_from_option(&mut self) -> Option<(u32, u8)> {
        let pid = self.world.entities.player_id()?;
        let design = ragnarok_game::sprite_path::cart_design_from_option(
            self.world.entities.get(pid)?.effect_state,
        )?;
        self.character.cart_design = Some(design);
        if let Some(entity) = self.world.entities.get_mut(pid) {
            entity.cart_type = Some(design);
        }
        Some((pid, design))
    }

    /// The queue and the key-maps must be wiped together: a key left behind
    /// points into a now-empty queue and blocks the alive-gated auras from ever
    /// re-spawning.
    pub fn reset_effects(&mut self, queue: &mut EffectQueue) {
        queue.clear();
        self.effect_keys.clear();
    }

    pub fn new() -> Self {
        Self {
            session: SessionState::new(),
            requested_guild_emblems: HashSet::new(),
            world: World::default(),
            sprite_caches: SpriteCaches::default(),
            character: Character::new(),
            data_table: DataTable::new(),
            assets: AssetHandles::default(),
            broadcast: Broadcast::new(),
            pending_confirms: PendingConfirms::default(),
            npc_cutins: [None, None, None],
            chat_rooms: ChatRoomRegistry::new(),
            pending_chat_room: None,
            hover: HoverState::default(),
            pending_casts: PendingCasts::default(),
            combat: CombatState::new(),
            prefs: Prefs::new(),
            party: None,
            guild: None,
            guild_menu_flag: 0,
            friends: ragnarok_game::friends::FriendList::default(),
            companions: Companions {
                homunculus: None,
                mercenary: None,
                pet: PetState::default(),
                companion_ai: ragnarok_ai::config::CompanionAiConfig::load_or_default(
                    COMPANION_AI_CONFIG_PATH,
                ),
                companion_attack_target: [None; 2],
                capture_targeting: false,
                pet_roulette: None,
            },
            quest_log: QuestLog::default(),
            quest_markers: std::collections::HashMap::new(),
            debug_show_pick_bounds: false,
            show_ping: false,
            show_fps: false,
            schedulers: Schedulers::new(),
            effect_keys: EffectKeys::default(),
        }
    }

    pub fn arm_confirm(
        &mut self,
        windows: &mut Windows,
        message: &str,
        ctor: impl FnOnce(bool) -> Option<GameEvent> + 'static,
    ) {
        self.pending_confirms.active = Some(Box::new(ctor));
        windows.confirm_dialog.show_confirm(message);
    }

    /// Returns true when the request must be auto-refused; otherwise records the
    /// pending request and opens the accept dialog.
    pub fn begin_trade_request(
        &mut self,
        windows: &mut Windows,
        name: String,
        gid: u32,
        auto_refuse: bool,
    ) -> bool {
        if auto_refuse {
            return true;
        }
        self.pending_confirms.pending_trade_partner = Some((gid, name.clone()));
        self.pending_confirms.pending_trade_request = Some(gid);
        self.arm_confirm(
            windows,
            &format!("Do you want to trade with {name}?"),
            |accept| Some(GameEvent::RespondExchangeRequest { accept }),
        );
        false
    }

    pub fn apply_window_state(
        &mut self,
        windows: &mut Windows,
        window_state: &HashMap<u32, WindowStateEntry>,
    ) {
        if let Some(entry) = window_state.get(&INV_WINDOW_ID.0) {
            if entry.open {
                self.character.inventory.open();
            }
            windows.inventory_window.set_minimized(entry.collapsed);
        }
        if let Some(entry) = window_state.get(&EQ_WINDOW_ID.0) {
            windows.equipment_window.open = entry.open;
            windows.equipment_window.set_minimized(entry.collapsed);
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
            windows.chat_window.set_initial_size_index(size_index);
        }
    }

    pub fn extract_window_state(
        &self,
        windows: &Windows,
        state_cache: &StateCache,
    ) -> HashMap<u32, (bool, bool)> {
        let mut result = HashMap::new();
        result.insert(
            INV_WINDOW_ID.0,
            (
                self.character.inventory.is_open(),
                windows.inventory_window.is_minimized(),
            ),
        );
        result.insert(
            EQ_WINDOW_ID.0,
            (
                windows.equipment_window.is_open(),
                windows.equipment_window.is_minimized(),
            ),
        );
        result.insert(SKILL_WINDOW_ID.0, (self.character.skills.is_open(), false));
        let size_index = windows.chat_window.get_size_index(state_cache);
        result.insert(
            chat_window::CHAT_WINDOW_ID.0,
            (size_index > 0, size_index == 1),
        );
        result
    }
}

#[cfg(test)]
mod cart_option_tests {
    use super::*;
    use ragnarok_game::entity::Entity;

    fn with_player(effect_state: i32) -> GameState {
        let mut game = GameState::new();
        let mut player = Entity::new_player(1000, 0, 1, 0, 0, 0, 0, 0, 0, 0, 5, 5, 0);
        player.effect_state = effect_state;
        game.world.entities.set_player_id(1000);
        game.world.entities.insert(player);
        game
    }

    #[test]
    fn spawns_player_cart_from_option_bit() {
        let mut game = with_player(0x100);
        assert_eq!(game.player_cart_from_option(), Some((1000, 3)));
        assert_eq!(game.character.cart_design, Some(3));
        assert_eq!(game.world.entities.get(1000).unwrap().cart_type, Some(3));
    }

    #[test]
    fn no_cart_when_option_bit_clear() {
        let mut game = with_player(0);
        assert_eq!(game.player_cart_from_option(), None);
        assert_eq!(game.character.cart_design, None);
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
        game.companions.mercenary = Some(merc);

        assert_eq!(
            game.resolve_cast_skill(8201),
            Some((SkillTargetType::Target, 9))
        );
        assert_eq!(game.resolve_cast_skill(9999), None);
    }
}

#[cfg(test)]
mod trade_request_tests {
    use super::*;

    #[test]
    fn auto_refuse_skips_dialog_and_normal_path_opens_it() {
        let mut game = GameState::new();
        let mut windows = Windows::new();
        assert!(game.begin_trade_request(&mut windows, "Alice".to_string(), 42, true));
        assert!(game.pending_confirms.pending_trade_request.is_none());
        assert!(game.pending_confirms.pending_trade_partner.is_none());
        assert!(windows.confirm_dialog.state.is_none());

        assert!(!game.begin_trade_request(&mut windows, "Alice".to_string(), 42, false));
        assert_eq!(game.pending_confirms.pending_trade_request, Some(42));
        assert_eq!(
            game.pending_confirms.pending_trade_partner,
            Some((42, "Alice".to_string()))
        );
        assert!(windows.confirm_dialog.state.is_some());
    }
}

#[cfg(test)]
mod pending_confirms_tests {
    use super::*;

    #[test]
    fn arm_confirm_dispatches_on_accept_and_clears_on_refuse() {
        let mut game = GameState::new();
        let mut windows = Windows::new();

        game.arm_confirm(&mut windows, "Join party?", |accept| {
            Some(GameEvent::RespondPartyInvite {
                party_grid: 7,
                accept,
            })
        });
        assert!(windows.confirm_dialog.state.is_some());
        assert!(matches!(
            game.pending_confirms.dispatch(ConfirmResult::Ok),
            Some(GameEvent::RespondPartyInvite {
                party_grid: 7,
                accept: true
            })
        ));
        assert!(game.pending_confirms.dispatch(ConfirmResult::Ok).is_none());

        game.arm_confirm(&mut windows, "Feed pet?", |accept| {
            accept.then_some(GameEvent::RequestPetCommand { csub: 1 })
        });
        assert!(
            game.pending_confirms
                .dispatch(ConfirmResult::Cancel)
                .is_none()
        );
    }
}

#[cfg(test)]
mod window_state_persistence_tests {
    use super::*;

    #[test]
    fn closed_skill_window_persists_and_restores_closed() {
        let cache = StateCache::new();
        let mut game = GameState::new();
        let windows = Windows::new();
        game.character.skills.open();
        assert_eq!(
            game.extract_window_state(&windows, &cache)
                .get(&SKILL_WINDOW_ID.0),
            Some(&(true, false))
        );

        game.character.skills.close();
        let captured = game.extract_window_state(&windows, &cache);
        assert_eq!(captured.get(&SKILL_WINDOW_ID.0), Some(&(false, false)));

        let (open, collapsed) = captured[&SKILL_WINDOW_ID.0];
        let mut window_state = HashMap::new();
        window_state.insert(
            SKILL_WINDOW_ID.0,
            WindowStateEntry {
                position: [0.0, 0.0],
                open,
                collapsed,
            },
        );
        let mut next_login = GameState::new();
        let mut next_windows = Windows::new();
        next_login.apply_window_state(&mut next_windows, &window_state);
        assert!(!next_login.character.skills.is_open());
    }
}

#[cfg(test)]
mod cursor_from_hover_tests {
    use super::*;

    fn in_game() -> CursorInput {
        CursorInput {
            in_game: true,
            right_mouse_down: false,
            ui_any_hovered: false,
            ui_any_interactive_hovered: false,
            item_drag_active: false,
        }
    }

    fn no_pending() -> CursorPending {
        CursorPending {
            companion_target_armed: false,
            pending_companion_skill: false,
            capture_targeting: false,
            pending_skill: false,
        }
    }

    #[test]
    fn cursor_follows_hover_priority_chain() {
        let mut hover = HoverState::default();

        hover.cell_cursor = CursorType::NoWalk;
        assert_eq!(
            cursor_type_from_hover(&hover, in_game(), no_pending()),
            CursorType::NoWalk
        );

        hover.hovered_entity_cursor = Some(CursorType::Attack);
        assert_eq!(
            cursor_type_from_hover(&hover, in_game(), no_pending()),
            CursorType::Attack
        );

        hover.hovered_vending = Some(7);
        assert_eq!(
            cursor_type_from_hover(&hover, in_game(), no_pending()),
            CursorType::Click
        );

        hover.hovered_chat_room = Some(3);
        assert_eq!(
            cursor_type_from_hover(&hover, in_game(), no_pending()),
            CursorType::Click
        );

        let pending = CursorPending {
            pending_skill: true,
            ..no_pending()
        };
        assert_eq!(
            cursor_type_from_hover(&hover, in_game(), pending),
            CursorType::Lock
        );

        hover.hovered_floor_item_id = Some(1);
        assert_eq!(
            cursor_type_from_hover(&hover, in_game(), no_pending()),
            CursorType::Pick
        );

        let dragging = CursorInput {
            right_mouse_down: true,
            ..in_game()
        };
        assert_eq!(
            cursor_type_from_hover(&HoverState::default(), dragging, no_pending()),
            CursorType::Rotate
        );

        let out_of_game = CursorInput {
            in_game: false,
            ui_any_interactive_hovered: true,
            ..in_game()
        };
        assert_eq!(
            cursor_type_from_hover(&hover, out_of_game, no_pending()),
            CursorType::Click
        );
    }

    #[test]
    fn item_drag_keeps_default_cursor_over_world_targets() {
        let mut hover = HoverState::default();
        hover.cell_cursor = CursorType::NoWalk;
        hover.hovered_entity_cursor = Some(CursorType::Attack);
        hover.hovered_floor_item_id = Some(1);

        let dragging = CursorInput {
            item_drag_active: true,
            ..in_game()
        };
        assert_eq!(
            cursor_type_from_hover(&hover, dragging, no_pending()),
            CursorType::Default
        );

        let dragging_over_ui = CursorInput {
            item_drag_active: true,
            ui_any_interactive_hovered: true,
            ..in_game()
        };
        assert_eq!(
            cursor_type_from_hover(&hover, dragging_over_ui, no_pending()),
            CursorType::Default
        );

        let rotating = CursorInput {
            item_drag_active: true,
            right_mouse_down: true,
            ..in_game()
        };
        assert_eq!(
            cursor_type_from_hover(&hover, rotating, no_pending()),
            CursorType::Rotate,
            "camera rotation feedback still wins over a drag"
        );
    }
}

#[cfg(test)]
mod effect_reset_tests {
    use super::*;

    #[test]
    fn reset_effects_wipes_queue_and_every_key_map_together() {
        let mut game = GameState::new();
        let mut queue = EffectQueue::new();
        queue.spawn_on_keyed(EffectId::Blessing, 1, 1);
        queue.despawn(2);

        let keys = &mut game.effect_keys;
        keys.status_buff_keys.insert((1, 2), 3);
        keys.next_status_buff_key = 9;
        keys.level_aura_keys.insert(1, 1);
        keys.boss_aura_keys.insert(1, 1);
        keys.toprank_keys.insert(1, (1, 1));
        keys.warp_portal_keys.insert(1, 1);
        keys.spirit_keys.insert(1, 1);
        keys.sight_aura_keys.insert(1, 1);
        keys.ruwach_aura_keys.insert(1, 1);
        keys.weather_keys.insert(EffectId::Snow, 1);

        game.reset_effects(&mut queue);

        assert!(queue.drain().is_empty());
        assert!(queue.drain_despawns().is_empty());
        let keys = &game.effect_keys;
        assert!(keys.status_buff_keys.is_empty());
        assert_eq!(keys.next_status_buff_key, 0);
        assert!(keys.level_aura_keys.is_empty());
        assert!(keys.boss_aura_keys.is_empty());
        assert!(keys.toprank_keys.is_empty());
        assert!(keys.warp_portal_keys.is_empty());
        assert!(keys.spirit_keys.is_empty());
        assert!(keys.sight_aura_keys.is_empty());
        assert!(keys.ruwach_aura_keys.is_empty());
        assert!(keys.weather_keys.is_empty());
    }
}
