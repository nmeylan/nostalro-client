use std::collections::HashMap;

/// Re-requesting the same char id is throttled to this interval.
const REQUEST_INTERVAL_MS: u64 = 2500;

/// Names resolved from a char id (CZ_REQNAME_BYGID). Forged and created items
/// carry their maker's char id in their card slots, and that char is usually
/// not on the map, so the entity list cannot answer for them.
#[derive(Debug, Default)]
pub struct CharNameCache {
    names: HashMap<u32, String>,
    requested_at: HashMap<u32, u64>,
}

impl CharNameCache {
    pub fn get(&self, char_id: u32) -> Option<&str> {
        self.names.get(&char_id).map(|n| n.as_str())
    }

    pub fn insert(&mut self, char_id: u32, name: String) {
        self.requested_at.remove(&char_id);
        self.names.insert(char_id, name);
    }

    pub fn should_request(&mut self, char_id: u32, now_ms: u64) -> bool {
        if char_id == 0 || self.names.contains_key(&char_id) {
            return false;
        }
        let due = match self.requested_at.get(&char_id) {
            Some(sent) => now_ms.saturating_sub(*sent) >= REQUEST_INTERVAL_MS,
            None => true,
        };
        if due {
            self.requested_at.insert(char_id, now_ms);
        }
        due
    }

    pub fn clear(&mut self) {
        self.names.clear();
        self.requested_at.clear();
    }
}
