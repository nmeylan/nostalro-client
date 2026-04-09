pub mod account;
pub mod game;
pub mod helper;

use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::frame::UiFrame;

pub trait Window {
    fn has_grf_textures(&self) -> bool;
    fn set_has_grf_textures(&mut self, value: bool);
    fn set_texture_sizes(&mut self, _size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {}
    fn grf_texture_paths() -> Vec<&'static str> where Self: Sized;
}

pub trait InGameWindow: Window {
    fn setup_modal(&self, _ui: &mut UiFrame) {}
    fn build(&mut self, ui: &mut UiFrame, character: &mut Character, data: &DataTable) -> Vec<GameEvent>;
}
