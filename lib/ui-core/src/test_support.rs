//! Frame construction for tests. Not compiled into the client binary.

use std::collections::HashMap;
use std::sync::OnceLock;

use ragnarok_renderer::font_atlas::FontAtlas;

use crate::context::UiContext;
use crate::frame::{UiFrame, WidgetId};
use crate::state::StateCache;

pub fn atlas() -> &'static FontAtlas {
    static ATLAS: OnceLock<FontAtlas> = OnceLock::new();
    ATLAS.get_or_init(|| FontAtlas::from_embedded(14.0, 1.0))
}

fn no_positions() -> &'static HashMap<u32, [f32; 2]> {
    static POSITIONS: OnceLock<HashMap<u32, [f32; 2]>> = OnceLock::new();
    POSITIONS.get_or_init(HashMap::new)
}

pub fn test_frame<'a>(ctx: &'a mut UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
    UiFrame::new(ctx, atlas(), state, 0.0, false, None, no_positions())
}

pub struct TestFrame {
    elapsed: f32,
    focus: Option<WidgetId>,
    positions: &'static HashMap<u32, [f32; 2]>,
}

impl TestFrame {
    pub fn new() -> Self {
        Self {
            elapsed: 0.0,
            focus: None,
            positions: no_positions(),
        }
    }

    pub fn elapsed(mut self, secs: f32) -> Self {
        self.elapsed = secs;
        self
    }

    pub fn focus(mut self, id: WidgetId) -> Self {
        self.focus = Some(id);
        self
    }

    pub fn positions(mut self, positions: HashMap<u32, [f32; 2]>) -> Self {
        self.positions = Box::leak(Box::new(positions));
        self
    }

    pub fn build<'a>(self, ctx: &'a mut UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        UiFrame::new(
            ctx,
            atlas(),
            state,
            self.elapsed,
            false,
            self.focus,
            self.positions,
        )
    }
}
