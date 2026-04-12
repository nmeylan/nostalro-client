use crate::item::Item;

pub const HOTKEY_ROWS: usize = 4;
pub const HOTKEY_COLS: usize = 9;
pub const HOTKEY_TOTAL: usize = HOTKEY_ROWS * HOTKEY_COLS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeySlotContent {
    Empty,
    Skill { skill_id: u16, level: i16 },
    Item { item_id: u16, inventory_index: u16 },
}

pub struct HotkeyBar {
    slots: [HotkeySlotContent; HOTKEY_TOTAL],
    visible_rows: u8,
    battle_mode: bool,
}

impl HotkeyBar {
    pub fn new() -> Self {
        Self {
            slots: [HotkeySlotContent::Empty; HOTKEY_TOTAL],
            visible_rows: 1,
            battle_mode: false,
        }
    }

    pub fn set_from_server(&mut self, keys: &[(i8, u32, i16)], inventory: &[Item]) {
        for (i, &(is_skill, id, count)) in keys.iter().take(HOTKEY_TOTAL).enumerate() {
            self.slots[i] = if is_skill != 0 && id != 0 {
                HotkeySlotContent::Skill {
                    skill_id: id as u16,
                    level: count,
                }
            } else if id != 0 {
                let inventory_index = id as u16;
                let item_id = inventory.iter()
                    .find(|i| i.index == inventory_index)
                    .map(|i| i.item_id)
                    .unwrap_or(0);
                if item_id != 0 {
                    HotkeySlotContent::Item {
                        item_id,
                        inventory_index,
                    }
                } else {
                    HotkeySlotContent::Empty
                }
            } else {
                HotkeySlotContent::Empty
            };
        }
    }

    pub fn set_slot(&mut self, index: usize, content: HotkeySlotContent) {
        if index < HOTKEY_TOTAL {
            self.slots[index] = content;
        }
    }

    pub fn clear_slot(&mut self, index: usize) {
        if index < HOTKEY_TOTAL {
            self.slots[index] = HotkeySlotContent::Empty;
        }
    }

    pub fn get_slot(&self, index: usize) -> HotkeySlotContent {
        if index < HOTKEY_TOTAL {
            self.slots[index]
        } else {
            HotkeySlotContent::Empty
        }
    }

    pub fn cycle_visibility(&mut self) {
        self.visible_rows = (self.visible_rows + 1) % (HOTKEY_ROWS as u8 + 1);
    }

    pub fn visible_rows(&self) -> u8 {
        self.visible_rows
    }

    pub fn set_visible_rows(&mut self, rows: u8) {
        self.visible_rows = rows.min(HOTKEY_ROWS as u8);
    }

    pub fn toggle_battle_mode(&mut self) {
        self.battle_mode = !self.battle_mode;
    }

    pub fn battle_mode(&self) -> bool {
        self.battle_mode
    }

    pub fn set_battle_mode(&mut self, enabled: bool) {
        self.battle_mode = enabled;
    }

    pub fn to_server_format(&self, index: usize) -> (i8, u32, i16) {
        match self.get_slot(index) {
            HotkeySlotContent::Empty => (0, 0, 0),
            HotkeySlotContent::Skill { skill_id, level } => (1, skill_id as u32, level),
            HotkeySlotContent::Item { inventory_index, .. } => (0, inventory_index as u32, 0),
        }
    }

    pub fn clear(&mut self) {
        self.slots = [HotkeySlotContent::Empty; HOTKEY_TOTAL];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(index: u16, item_id: u16) -> Item {
        Item {
            index,
            item_id,
            item_type: 0,
            count: 1,
            is_identified: true,
            is_damaged: false,
            refining_level: 0,
            slot: [0; 4],
            location: 0,
            wear_state: 0,
            name: String::new(),
            resource_name: None,
        }
    }

    #[test]
    fn set_from_server_and_query() {
        let mut bar = HotkeyBar::new();
        let inventory = vec![make_item(3, 501), make_item(7, 501), make_item(12, 502)];
        let server_data = vec![
            (1i8, 28u32, 5i16),   // Skill: id=28, level=5
            (0, 7, 0),            // Item: inventory_index=7 (second bow)
            (0, 0, 0),            // Empty
            (1, 10, 3),           // Skill: id=10, level=3
            (0, 99, 0),           // Item: inventory_index=99 (not in inventory)
        ];
        bar.set_from_server(&server_data, &inventory);

        assert_eq!(bar.get_slot(0), HotkeySlotContent::Skill { skill_id: 28, level: 5 });
        assert_eq!(bar.get_slot(1), HotkeySlotContent::Item { item_id: 501, inventory_index: 7 });
        assert_eq!(bar.get_slot(2), HotkeySlotContent::Empty);
        assert_eq!(bar.get_slot(3), HotkeySlotContent::Skill { skill_id: 10, level: 3 });
        assert_eq!(bar.get_slot(4), HotkeySlotContent::Empty);
    }

    #[test]
    fn cycle_visibility() {
        let mut bar = HotkeyBar::new();
        assert_eq!(bar.visible_rows(), 1);
        bar.cycle_visibility();
        assert_eq!(bar.visible_rows(), 2);
        bar.cycle_visibility();
        assert_eq!(bar.visible_rows(), 3);
        bar.cycle_visibility();
        assert_eq!(bar.visible_rows(), 4);
        bar.cycle_visibility();
        assert_eq!(bar.visible_rows(), 0);
        bar.cycle_visibility();
        assert_eq!(bar.visible_rows(), 1);
    }

    #[test]
    fn set_and_clear_slot() {
        let mut bar = HotkeyBar::new();
        bar.set_slot(5, HotkeySlotContent::Skill { skill_id: 28, level: 10 });
        assert_eq!(bar.get_slot(5), HotkeySlotContent::Skill { skill_id: 28, level: 10 });

        bar.set_slot(5, HotkeySlotContent::Item { item_id: 501, inventory_index: 3 });
        assert_eq!(bar.get_slot(5), HotkeySlotContent::Item { item_id: 501, inventory_index: 3 });

        bar.clear_slot(5);
        assert_eq!(bar.get_slot(5), HotkeySlotContent::Empty);
    }

    #[test]
    fn to_server_format_conversion() {
        let mut bar = HotkeyBar::new();
        bar.set_slot(0, HotkeySlotContent::Skill { skill_id: 28, level: 5 });
        bar.set_slot(1, HotkeySlotContent::Item { item_id: 501, inventory_index: 3 });

        assert_eq!(bar.to_server_format(0), (1, 28, 5));
        assert_eq!(bar.to_server_format(1), (0, 3, 0));
        assert_eq!(bar.to_server_format(2), (0, 0, 0));
    }
}
