use std::collections::HashMap;

use crate::cursor::RenderEntry;

/// A chat room (waitingroom) anchored to an owner entity. NPC-owned rooms are
/// how arena entrances present themselves: a clickable box floating over the NPC.
#[derive(Debug, Clone)]
pub struct ChatRoom {
    pub room_id: u32,
    pub owner_aid: u32,
    pub title: String,
    pub cur_count: i16,
    pub max_count: i16,
    /// 0 = private, 1 = public, 2 = arena, 3 = pk zone.
    pub atype: u8,
}

#[derive(Default)]
pub struct ChatRoomRegistry {
    rooms: HashMap<u32, ChatRoom>,
}

impl ChatRoomRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&mut self, room: ChatRoom) {
        self.rooms.insert(room.room_id, room);
    }

    pub fn remove(&mut self, room_id: u32) {
        self.rooms.remove(&room_id);
    }

    pub fn clear(&mut self) {
        self.rooms.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &ChatRoom> {
        self.rooms.values()
    }
}

impl ChatRoom {
    /// Single-line label shown in the floating box: title plus occupancy.
    pub fn box_label(&self) -> String {
        format!("{} ({}/{})", self.title, self.cur_count, self.max_count)
    }
}

/// Padding around the room title inside its box, in screen pixels.
pub const BOX_PADDING: f32 = 4.0;
/// Vertical gap between the owner's head and the bottom of the box.
pub const BOX_GAP: f32 = 5.0;

/// Screen-space rect `[left, top, right, bottom]` of the room box floating above
/// `entry`'s head. Shared by the renderer and the click hit-test so they agree
/// pixel-for-pixel; the caller supplies the box size measured from font metrics.
pub fn room_box_rect(entry: &RenderEntry, box_w: f32, box_h: f32) -> [f32; 4] {
    let left = entry.screen_anchor[0] - box_w / 2.0;
    let top = entry.screen_anchor[1] - entry.head_offset - BOX_GAP - box_h;
    [left, top, left + box_w, top + box_h]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::{RenderEntry, RenderEntryKind};

    fn contains(rect: [f32; 4], x: f32, y: f32) -> bool {
        x >= rect[0] && x <= rect[2] && y >= rect[1] && y <= rect[3]
    }

    #[test]
    fn box_sits_centered_above_owner_head() {
        let entry = RenderEntry {
            kind: RenderEntryKind::Entity,
            id: 150000,
            screen_anchor: [100.0, 200.0],
            depth: 0.0,
            depth_gradient: 0.0,
            camera_dir: 0,
            sprite_scale: 1.0,
            pick_bounds: [80.0, 120.0, 120.0, 200.0],
            head_offset: 50.0,
        };
        let (box_w, box_h) = (80.0, 20.0);
        let rect = room_box_rect(&entry, box_w, box_h);

        // Horizontally centred on the anchor, sitting above the head (gap + box height).
        let center_x = (rect[0] + rect[2]) / 2.0;
        let center_y = (rect[1] + rect[3]) / 2.0;
        assert!((center_x - 100.0).abs() < f32::EPSILON);
        assert!(contains(rect, center_x, center_y));
        // The owner's feet (the anchor) are well below the box.
        assert!(!contains(rect, 100.0, 200.0));
        assert!(rect[3] < 200.0);
    }
}
