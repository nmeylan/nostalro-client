use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use crate::{Window, InGameWindow};
use super::number_input::{NumberInputDialog, NumberInputConfig, NumberInputResult};

pub struct DropQuantityDialog {
    pub item_index: u16,
    pub max_count: i16,
    pub has_grf_textures: bool,
    inner: NumberInputDialog,
}

impl DropQuantityDialog {
    pub fn new(item_index: u16, max_count: i16) -> Self {
        let config = NumberInputConfig {
            label: None,
            show_cancel: false,
            escape_cancels: true,
            default_value: max_count.to_string(),
            max_len: 6,
        };
        Self {
            item_index,
            max_count,
            has_grf_textures: false,
            inner: NumberInputDialog::new(config, WidgetId(421)),
        }
    }

}

impl InGameWindow for DropQuantityDialog {
    fn build(&mut self, ui: &mut UiFrame, _character: &mut Character, _data: &DataTable) -> Vec<GameEvent> {
        self.inner.has_grf_textures = self.has_grf_textures;
        match self.inner.build(ui) {
            NumberInputResult::Submitted => {
                let qty: i16 = self.inner.value_i16().unwrap_or(0);
                if qty > 0 && qty <= self.max_count {
                    vec![GameEvent::RequestDropItem { index: self.item_index, count: qty }]
                } else {
                    vec![GameEvent::DialogClosed]
                }
            }
            NumberInputResult::Cancel => vec![GameEvent::DialogClosed],
            NumberInputResult::None => vec![]
        }
    }
}

impl Window for DropQuantityDialog {
    fn has_grf_textures(&self) -> bool { self.has_grf_textures }
    fn set_has_grf_textures(&mut self, value: bool) { self.has_grf_textures = value; }

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
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_renderer::font_atlas::FontAtlas;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> = Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    fn build_dialog(dialog: &mut DropQuantityDialog, ctx: &UiContext, state: &mut StateCache) -> Vec<GameEvent> {
        let mut ui = make_frame(ctx, state);
        let mut character = Character::new();
        dialog.build(&mut ui, &mut character, &DataTable::default())
    }

    #[test]
    fn enter_key_confirms_with_valid_quantity() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        dialog.inner.set_input_text("5");
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let events = build_dialog(&mut dialog, &ctx, &mut state);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], GameEvent::RequestDropItem { index: 0, count: 5 }));
    }

    #[test]
    fn enter_key_cancels_with_zero() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        dialog.inner.set_input_text("0");
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let events = build_dialog(&mut dialog, &ctx, &mut state);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], GameEvent::DialogClosed));
    }

    #[test]
    fn enter_key_cancels_with_over_max() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        dialog.inner.set_input_text("11");
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let events = build_dialog(&mut dialog, &ctx, &mut state);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], GameEvent::DialogClosed));
    }

    #[test]
    fn escape_key_cancels() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let events = build_dialog(&mut dialog, &ctx, &mut state);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], GameEvent::DialogClosed));
    }

    #[test]
    fn no_input_returns_none() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let events = build_dialog(&mut dialog, &ctx, &mut state);
        assert!(events.is_empty());
    }

    #[test]
    fn initial_text_is_max_count() {
        let dialog = DropQuantityDialog::new(5, 42);
        assert_eq!(dialog.inner.value_str(), "42");
    }
}
