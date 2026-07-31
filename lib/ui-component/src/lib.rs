pub mod account;
pub mod game;
pub mod helper;
pub mod widget_id;

use ragnarok_ai::config::CompanionAiConfig;
use ragnarok_game::character::Character;
use ragnarok_game::companion::{HomunculusState, MercenaryState};
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_game::friends::FriendList;
use ragnarok_game::guild::Guild;
use ragnarok_game::party::Party;
use ragnarok_game::pet::PetState;
use ragnarok_game::quest::QuestLog;
use ragnarok_ui::frame::UiFrame;

/// Live game state passed to every window's `build`. Windows read the fields
/// they need directly each frame (no snapshot, no per-frame clone). Carries
/// only `ragnarok_game` / `ragnarok_ai` data plus primitives: the crate
/// boundary forbids referencing the client's `GameState`/`World`, so state that
/// lives client-side is applied by the caller before `build`.
pub struct BuildCtx<'a> {
    pub character: &'a mut Character,
    pub data: &'a DataTable,
    pub party: Option<&'a Party>,
    pub friends: &'a FriendList,
    pub guild: Option<&'a Guild>,
    pub quest_log: &'a QuestLog,
    pub homunculus: Option<&'a HomunculusState>,
    pub mercenary: Option<&'a MercenaryState>,
    pub pet: &'a PetState,
    pub companion_ai: &'a mut CompanionAiConfig,
    /// Live job of the player entity — the server only ever announces a job
    /// change as a base-look sprite change, so the entity is the only source.
    pub job_class: u16,
    pub local_aid: u32,
    pub local_gid: u32,
}

#[cfg(test)]
impl<'a> BuildCtx<'a> {
    /// Minimal ctx for window unit tests: real `character`/`data`, everything
    /// else defaulted. The defaults are leaked (fine for a test process).
    pub fn test(character: &'a mut Character, data: &'a DataTable) -> Self {
        BuildCtx {
            character,
            data,
            party: None,
            friends: Box::leak(Box::new(FriendList::default())),
            guild: None,
            quest_log: Box::leak(Box::new(QuestLog::default())),
            homunculus: None,
            mercenary: None,
            pet: Box::leak(Box::new(PetState::default())),
            companion_ai: Box::leak(Box::new(CompanionAiConfig::default())),
            job_class: 0,
            local_aid: 0,
            local_gid: 0,
        }
    }
}

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
    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent>;
    /// Whether this window claims Escape right now. The caller offers the key to
    /// the front-most claimant only, so a window that is merely visible without
    /// being dismissable (bars, notifications) keeps the `false` default.
    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        false
    }
    /// Escape reached this window: close it, or leave an inner mode (a nested
    /// dialog, a sub-page) and stay open.
    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        Vec::new()
    }
    /// Whether this window is currently answering Enter — a modal waiting for an
    /// OK, or a nested input dialog. The caller then blocks the key for every
    /// window that reads it through [`UiFrame::enter_pressed`], so one press
    /// cannot both confirm the modal and open the chat line.
    fn owns_keyboard(&self, _ctx: &BuildCtx) -> bool {
        false
    }
}
