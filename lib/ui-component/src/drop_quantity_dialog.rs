use ragnarok_ui::frame::{UiFrame, WidgetId};
use crate::number_input::{NumberInputDialog, NumberInputConfig, NumberInputResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropQuantityResult {
    None,
    Ok(i16),
    Cancel,
}

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

    pub fn set_texture_sizes(&mut self, size_fn: impl Fn(&str) -> Option<(u32, u32)>) {
        self.inner.set_texture_sizes(size_fn);
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> DropQuantityResult {
        self.inner.has_grf_textures = self.has_grf_textures;
        match self.inner.build(ui) {
            NumberInputResult::Submitted => {
                let qty: i16 = self.inner.value_i16().unwrap_or(0);
                if qty > 0 && qty <= self.max_count {
                    DropQuantityResult::Ok(qty)
                } else {
                    DropQuantityResult::Cancel
                }
            }
            NumberInputResult::Cancel => DropQuantityResult::Cancel,
            NumberInputResult::None => DropQuantityResult::None,
        }
    }

    pub fn grf_texture_paths() -> Vec<&'static str> {
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

    #[test]
    fn enter_key_confirms_with_valid_quantity() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        dialog.inner.set_input_text("5");
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), DropQuantityResult::Ok(5));
    }

    #[test]
    fn enter_key_cancels_with_zero() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        dialog.inner.set_input_text("0");
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), DropQuantityResult::Cancel);
    }

    #[test]
    fn enter_key_cancels_with_over_max() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        dialog.inner.set_input_text("11");
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), DropQuantityResult::Cancel);
    }

    #[test]
    fn escape_key_cancels() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), DropQuantityResult::Cancel);
    }

    #[test]
    fn no_input_returns_none() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), DropQuantityResult::None);
    }

    #[test]
    fn initial_text_is_max_count() {
        let dialog = DropQuantityDialog::new(5, 42);
        assert_eq!(dialog.inner.value_str(), "42");
    }
}
