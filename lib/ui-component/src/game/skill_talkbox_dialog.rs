use super::input_dialog::{InputDialog, InputDialogConfig, InputDialogResult};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_game::skill::TALKBOX_MESSAGE_MAX_LEN;
use ragnarok_ui::frame::{UiFrame, WidgetId};

const BASE_ID: WidgetId = WidgetId(430);

pub struct SkillTalkboxDialog {
    pub skill_id: u16,
    pub level: i16,
    pub x: i16,
    pub y: i16,
    pub has_grf_textures: bool,
    inner: InputDialog,
}

impl SkillTalkboxDialog {
    pub fn new(skill_id: u16, level: i16, x: i16, y: i16) -> Self {
        let config = InputDialogConfig {
            label: Some("Message:".to_string()),
            show_cancel: true,
            escape_cancels: true,
            default_value: String::new(),
            max_len: TALKBOX_MESSAGE_MAX_LEN,
            numeric_only: false,
        };
        Self {
            skill_id,
            level,
            x,
            y,
            has_grf_textures: false,
            inner: InputDialog::new(config, BASE_ID),
        }
    }
}

impl InGameWindow for SkillTalkboxDialog {
    fn owns_keyboard(&self, _ctx: &BuildCtx) -> bool {
        true
    }

    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        true
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let _character = &mut *ctx.character;
        let _data = ctx.data;
        self.inner.has_grf_textures = self.has_grf_textures;
        match self.inner.build(ui) {
            // An empty message would place a blank unit, so the cast waits for text.
            InputDialogResult::Submitted if self.inner.value_str().is_empty() => vec![],
            InputDialogResult::Submitted => vec![GameEvent::ConfirmedSkillTalkbox {
                skill_id: self.skill_id,
                level: self.level,
                x: self.x,
                y: self.y,
                message: self.inner.value_str().to_string(),
            }],
            InputDialogResult::Cancel => vec![GameEvent::DialogClosed],
            InputDialogResult::None => vec![],
        }
    }
}

impl Window for SkillTalkboxDialog {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        self.inner.set_texture_sizes(size_fn);
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        InputDialog::grf_texture_paths()
    }
}
