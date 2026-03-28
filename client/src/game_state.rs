use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use ragnarok_game::accessory_table::AccessoryTable;
use ragnarok_game::app_state::AppState;
use ragnarok_game::cursor::CursorAnimationState;
use ragnarok_game::entity_collection::EntityCollection;
use ragnarok_game::map_coordinates::MapCoordinates;
use ragnarok_game::name_table::NameTable;
use ragnarok_game::event::CharacterInfo;
use ragnarok_game::server_time::ServerTimeClock;
use ragnarok_network::session::Session;
use ragnarok_renderer::{EntitySprite, SpriteTextures};
use ragnarok_ui_component::chat_window::ChatWindow;

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
    pub chat_window: ChatWindow,
    pub failed_sprite_loads: HashSet<u32>,
    pub server_time: ServerTimeClock,
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
            chat_window: ChatWindow::new(),
            failed_sprite_loads: HashSet::new(),
            server_time: ServerTimeClock::new(),
        }
    }
}
