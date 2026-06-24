use crate::helper::dialog_container::DialogContainer;
use crate::{InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const CHAT_ROOM_WINDOW_ID: WidgetId = WidgetId(1700);

const CHAT_OPEN_TEX: &str = "data/texture/유저인터페이스/chat_open.bmp";
const CHAT_CLOSE_TEX: &str = "data/texture/유저인터페이스/chat_close.bmp";

/// Fixed box width over the owner, matching the original client's title bar.
const BOX_W: f32 = 140.0;
const PADDING: f32 = 4.0;
/// Icon footprint used until the GRF texture is loaded (chat_open/close are 24×24).
const ICON_SIZE: (f32, f32) = (24.0, 24.0);
/// Vertical gap between the owner's head and the bottom of the box.
const HEAD_GAP: f32 = 5.0;
/// Longest title (before the occupancy suffix) that fits the fixed-width box.
const TITLE_MAX_CHARS: usize = 12;

/// One room to draw this frame: its data plus the owner's projected screen anchor.
/// The caller fills `placements` each frame from the chat-room registry joined
/// with the world render list, since the box position follows the owner entity.
#[derive(Clone)]
pub struct ChatRoomPlacement {
    pub room_id: u32,
    /// 0 = private, 1 = public, 2 = arena, 3 = pk zone.
    pub atype: u8,
    pub title: String,
    pub cur_count: i16,
    pub max_count: i16,
    /// Owner's feet in screen pixels.
    pub anchor_x: f32,
    pub anchor_y: f32,
    /// Pixels from the feet up to the top of the idle pose.
    pub head_offset: f32,
}

/// Floating "waitingroom" box(es) drawn above the owning NPC/player. Unlike the
/// other in-game windows this is not draggable, not unique, and its position is
/// driven by the owner's world position rather than a saved layout.
pub struct ChatRoomWindow {
    pub has_grf_textures: bool,
    pub container: DialogContainer,
    pub placements: Vec<ChatRoomPlacement>,
    open_icon_size: (f32, f32),
    close_icon_size: (f32, f32),
}

impl Default for ChatRoomWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatRoomWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            container: DialogContainer::new(),
            placements: Vec::new(),
            open_icon_size: ICON_SIZE,
            close_icon_size: ICON_SIZE,
        }
    }

    /// Private/password rooms use the "closed" door icon; public/arena/pk "open".
    fn icon_texture(atype: u8) -> &'static str {
        if atype == 0 {
            CHAT_CLOSE_TEX
        } else {
            CHAT_OPEN_TEX
        }
    }

    /// Caption: title plus occupancy, except pk-zone rooms which have no limit.
    /// The title is clipped so the caption stays within the fixed-width box.
    fn label(p: &ChatRoomPlacement) -> String {
        let title = if p.title.chars().count() > TITLE_MAX_CHARS {
            let kept: String = p.title.chars().take(TITLE_MAX_CHARS).collect();
            format!("{kept}…")
        } else {
            p.title.clone()
        };
        if p.atype == 3 {
            title
        } else {
            format!("{title} ({}/{})", p.cur_count, p.max_count)
        }
    }

    fn icon_size(&self, atype: u8) -> (f32, f32) {
        if atype == 0 {
            self.close_icon_size
        } else {
            self.open_icon_size
        }
    }

    /// Fixed-width box centred over the owner's head, sitting above it.
    fn box_rect(p: &ChatRoomPlacement) -> Rect {
        let box_h = PADDING + ICON_SIZE.1 + PADDING;
        Rect::new(
            p.anchor_x - BOX_W / 2.0,
            p.anchor_y - p.head_offset - HEAD_GAP - box_h,
            BOX_W,
            box_h,
        )
    }
}

impl Window for ChatRoomWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }

    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
        self.container.has_grf_textures = value;
    }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        self.container.has_grf_textures = true;
        self.container.set_texture_sizes(size_fn);
        if let Some((w, h)) = size_fn(CHAT_OPEN_TEX) {
            self.open_icon_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(CHAT_CLOSE_TEX) {
            self.close_icon_size = (w as f32, h as f32);
        }
    }

    fn grf_texture_paths() -> Vec<&'static str>
    where
        Self: Sized,
    {
        let mut paths = DialogContainer::grf_texture_paths();
        paths.push(CHAT_OPEN_TEX);
        paths.push(CHAT_CLOSE_TEX);
        paths
    }
}

impl InGameWindow for ChatRoomWindow {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        _character: &mut Character,
        _data: &DataTable,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();
        self.container.has_grf_textures = self.has_grf_textures;

        for (i, p) in self.placements.iter().enumerate() {
            let rect = Self::box_rect(p);
            // Per-room id, stable within a frame, so each box hit-tests independently.
            let id = WidgetId(CHAT_ROOM_WINDOW_ID.0 + 1 + i as u32);
            let resp = ui.interact(id, rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }

            // Sysbox nine-slice background, the same frame the dialog boxes use.
            self.container
                .draw(&mut ui.draw_calls, rect.x, rect.y, rect.w, rect.h, [1.0; 4]);

            // Open/closed door icon at the left, drawn at its native size.
            let (icon_w, icon_h) = self.icon_size(p.atype);
            let icon_rect = Rect::new(
                rect.x + PADDING,
                rect.y + (rect.h - icon_h) / 2.0,
                icon_w,
                icon_h,
            );
            if self.has_grf_textures {
                let (v, idx) =
                    draw::quad_vertices(icon_rect.x, icon_rect.y, icon_w, icon_h, [1.0; 4]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::Named(Self::icon_texture(p.atype).to_string()),
                });
            }

            // Title left-aligned after the icon, coloured to suit the background.
            let text_x = icon_rect.x + icon_w + PADDING;
            let text_y = rect.y + (rect.h + ui.atlas.line_height) / 2.0;
            ui.text(text_x, text_y, &Self::label(p), self.container.text_color());

            if resp.clicked() {
                events.push(GameEvent::RequestJoinChatRoom { room_id: p.room_id });
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placement(atype: u8) -> ChatRoomPlacement {
        ChatRoomPlacement {
            room_id: 7,
            atype,
            title: "Arena".to_string(),
            cur_count: 3,
            max_count: 20,
            anchor_x: 100.0,
            anchor_y: 200.0,
            head_offset: 50.0,
        }
    }

    #[test]
    fn box_sits_centered_above_owner_head() {
        let rect = ChatRoomWindow::box_rect(&placement(2));
        assert!((rect.x + rect.w / 2.0 - 100.0).abs() < f32::EPSILON);
        assert_eq!(rect.w, BOX_W);
        // Box is fully above the feet (the anchor), not overlapping them.
        assert!(rect.y + rect.h < 200.0);
    }

    #[test]
    fn icon_and_label_follow_room_type() {
        assert_eq!(ChatRoomWindow::icon_texture(2), CHAT_OPEN_TEX); // arena
        assert_eq!(ChatRoomWindow::icon_texture(0), CHAT_CLOSE_TEX); // private
        assert_eq!(ChatRoomWindow::label(&placement(2)), "Arena (3/20)");
        // pk zone: no occupancy shown
        assert_eq!(ChatRoomWindow::label(&placement(3)), "Arena");
    }

    #[test]
    fn long_title_is_clipped() {
        let mut p = placement(1);
        p.title = "A Very Long Room Title".to_string();
        let label = ChatRoomWindow::label(&p);
        assert!(label.starts_with("A Very Long …") || label.starts_with("A Very Long…"));
        assert!(label.ends_with("(3/20)"));
    }
}
