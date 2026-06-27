use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ChatRoom {
    pub room_id: u32,
    pub owner_aid: u32,
    pub title: String,
    pub cur_count: i16,
    pub max_count: i16,
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
