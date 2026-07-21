use super::number_input::{NumberInputConfig, NumberInputDialog, NumberInputResult};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_ui::frame::{UiFrame, WidgetId};

const REASON_MAX_LEN: usize = 39;
const BASE_ID: WidgetId = WidgetId(425);

pub struct GuildExpelDialog {
    pub aid: u32,
    pub gid: u32,
    pub name: String,
    pub has_grf_textures: bool,
    inner: NumberInputDialog,
}

impl GuildExpelDialog {
    pub fn new(aid: u32, gid: u32, name: String) -> Self {
        let config = NumberInputConfig {
            label: Some(format!("Reason for expelling {name}:")),
            show_cancel: true,
            escape_cancels: true,
            default_value: String::new(),
            max_len: REASON_MAX_LEN,
        };
        Self {
            aid,
            gid,
            name,
            has_grf_textures: false,
            inner: NumberInputDialog::new(config, BASE_ID),
        }
    }
}

impl InGameWindow for GuildExpelDialog {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        ctx: &mut BuildCtx,
    ) -> Vec<GameEvent> {
        let _character = &mut *ctx.character;
        let _data = ctx.data;
        self.inner.has_grf_textures = self.has_grf_textures;
        match self.inner.build(ui) {
            NumberInputResult::Submitted => vec![GameEvent::ConfirmedGuildExpel {
                aid: self.aid,
                gid: self.gid,
                name: self.name.clone(),
                reason: self.inner.value_str().to_string(),
            }],
            NumberInputResult::Cancel => vec![GameEvent::DialogClosed],
            NumberInputResult::None => vec![],
        }
    }
}

impl Window for GuildExpelDialog {
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
        NumberInputDialog::grf_texture_paths()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    fn build(dialog: &mut GuildExpelDialog, ctx: &UiContext, state: &mut StateCache) -> Vec<GameEvent> {
        let mut ui = make_frame(ctx, state);
        let mut character = Character::new();
        dialog.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &DataTable::default()))
    }

    #[test]
    fn typed_reason_is_sent_on_submit() {
        let mut dialog = GuildExpelDialog::new(11, 22, "Bob".to_string());
        let mut state = StateCache::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.typed_chars = "afk".chars().collect();
        let _ = build(&mut dialog, &ctx, &mut state);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let events = build(&mut dialog, &ctx, &mut state);

        assert!(matches!(
            events.as_slice(),
            [GameEvent::ConfirmedGuildExpel { aid: 11, gid: 22, name, reason }]
                if name == "Bob" && reason == "afk"
        ));
    }

    #[test]
    fn escape_closes_without_expel() {
        let mut dialog = GuildExpelDialog::new(1, 2, "X".to_string());
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let events = build(&mut dialog, &ctx, &mut state);
        assert!(matches!(events.as_slice(), [GameEvent::DialogClosed]));
    }
}
