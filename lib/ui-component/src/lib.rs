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
    /// Nominal outer size (width, height) in pixels. The default `(0.0, 0.0)`
    /// marks a window that positions itself (bars, dialogs, full-screen
    /// screens); draggable windows override it so the gallery can lay them out
    /// without overlap.
    fn window_size(&self) -> (f32, f32) {
        (0.0, 0.0)
    }
    fn grf_texture_paths() -> Vec<&'static str>
    where
        Self: Sized;
}

pub trait InGameWindow: Window {
    fn setup_modal(&self, _ui: &mut UiFrame) {}
    fn build(
        &mut self,
        ui: &mut UiFrame,
        character: &mut Character,
        data: &DataTable,
    ) -> Vec<GameEvent>;
}
