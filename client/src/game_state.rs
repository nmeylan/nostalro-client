use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use ragnarok_game::accessory_table::AccessoryTable;
use ragnarok_game::app_state::AppState;
use ragnarok_game::cursor::CursorAnimationState;
use ragnarok_game::entity::Entity;
use ragnarok_game::event::CharacterInfo;
use ragnarok_game::map_coordinates::MapCoordinates;
use ragnarok_network::session::Session;
use ragnarok_renderer::{EntitySprite, SpriteTextures};

pub struct GameState {
    pub app_state: AppState,
    pub login_session: Option<Session>,
    pub selected_character: Option<CharacterInfo>,
    pub current_map: Option<String>,
    pub map_coords: Option<MapCoordinates>,
    pub gat: Option<GatFile>,
    pub player_entity: Option<Entity>,
    pub player_sprite: Option<EntitySprite>,
    pub accessory_table: Option<AccessoryTable>,
    pub cursor_textures: Option<SpriteTextures>,
    pub cursor_act: Option<ActFile>,
    pub cursor_animation: CursorAnimationState,
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
            player_entity: None,
            player_sprite: None,
            accessory_table: None,
            cursor_textures: None,
            cursor_act: None,
            cursor_animation: CursorAnimationState::new(),
        }
    }
}
