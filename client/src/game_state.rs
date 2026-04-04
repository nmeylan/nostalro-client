use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use ragnarok_game::accessory_table::AccessoryTable;
use ragnarok_game::app_state::AppState;
use ragnarok_game::cursor::CursorAnimationState;
use ragnarok_game::entity_collection::EntityCollection;
use ragnarok_game::map_coordinates::MapCoordinates;
use ragnarok_game::item_name_table::ItemNameTable;
use ragnarok_game::item_resource_table::ItemResourceTable;
use ragnarok_game::name_table::NameTable;
use ragnarok_game::event::CharacterInfo;
use ragnarok_game::server_time::ServerTimeClock;
use ragnarok_network::session::Session;
use ragnarok_renderer::{EntitySprite, SpriteTextures};
use ragnarok_ui_component::chat_window::ChatWindow;
use ragnarok_ui_component::npc_dialog::NpcDialog;
use ragnarok_ui_component::npc_shop::NpcShop;
use ragnarok_ui_component::system_menu::SystemMenu;

pub struct GameState {
    pub app_state: AppState,
    pub login_session: Option<Session>,
    pub selected_character: Option<CharacterInfo>,
    pub current_map: Option<String>,
    pub map_coords: Option<MapCoordinates>,
    pub gat: Option<GatFile>,
    pub entities: EntityCollection,
    pub sprites: HashMap<u32, Rc<EntitySprite>>,
    pub sprite_cache: HashMap<String, Rc<EntitySprite>>,
    pub name_table: Option<NameTable>,
    pub accessory_table: Option<AccessoryTable>,
    pub cursor_textures: Option<SpriteTextures>,
    pub cursor_act: Option<ActFile>,
    pub cursor_animation: CursorAnimationState,
    pub emotion_textures: Option<SpriteTextures>,
    pub emotion_act: Option<ActFile>,
    pub item_name_table: Option<ItemNameTable>,
    pub item_resource_table: Option<ItemResourceTable>,
    pub chat_window: ChatWindow,
    pub npc_dialog: NpcDialog,
    pub npc_shop: NpcShop,
    pub system_menu: SystemMenu,
    pub hovered_entity_id: Option<u32>,
    pub failed_sprite_loads: HashSet<u32>,
    pub server_time: ServerTimeClock,
    pub attack_range: i16,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            app_state: AppState::Login,
            login_session: None,
            selected_character: None,
            current_map: None,
            map_coords: None,
            gat: None,
            entities: EntityCollection::new(),
            sprites: HashMap::new(),
            sprite_cache: HashMap::new(),
            name_table: None,
            accessory_table: None,
            cursor_textures: None,
            cursor_act: None,
            cursor_animation: CursorAnimationState::new(),
            emotion_textures: None,
            emotion_act: None,
            item_name_table: None,
            item_resource_table: None,
            chat_window: ChatWindow::new(),
            npc_dialog: NpcDialog::new(),
            npc_shop: NpcShop::new(),
            system_menu: SystemMenu::new(),
            hovered_entity_id: None,
            failed_sprite_loads: HashSet::new(),
            server_time: ServerTimeClock::new(),
            attack_range: 1,
        }
    }
}
