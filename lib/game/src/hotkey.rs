use models::enums::skill_enums::SkillEnum;

pub const HOTKEY_ROWS: usize = 4;
pub const HOTKEY_COLS: usize = 9;
pub const HOTKEY_TOTAL: usize = HOTKEY_ROWS * HOTKEY_COLS;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HotkeySlotContent {
    Empty,
    Skill { skill: SkillEnum, level: i16 },
    Item { item_id: u16 },
}

pub struct HotkeyBar {
    slots: [HotkeySlotContent; HOTKEY_TOTAL],
    visible_rows: u8,
    battle_mode: bool,
}

impl Default for HotkeyBar {
    fn default() -> Self {
        Self::new()
    }
}

impl HotkeyBar {
    pub fn new() -> Self {
        Self {
            slots: [HotkeySlotContent::Empty; HOTKEY_TOTAL],
            visible_rows: 1,
            battle_mode: false,
        }
    }

    pub fn set_from_server(&mut self, keys: &[(i8, u32, i16)]) {
        for (i, &(is_skill, id, count)) in keys.iter().take(HOTKEY_TOTAL).enumerate() {
            self.slots[i] = if is_skill != 0 && id != 0 {
                HotkeySlotContent::Skill {
                    skill: SkillEnum::from_id(id),
                    level: count,
                }
            } else if id != 0 {
                HotkeySlotContent::Item { item_id: id as u16 }
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

    /// Realigns slots holding `skill` after the server changed its learned
    /// level, returning the indexes that moved so the caller can persist them.
    ///
    /// A one-level gain carries along the slots that sat at the old learned
    /// level — the only level a skill that cannot be down-ranked can hold — and
    /// leaves a deliberately down-ranked slot where it is. Any other change
    /// (reset, unlearn) only pulls slots back down to what is still castable.
    pub fn apply_skill_level_change(
        &mut self,
        skill: SkillEnum,
        before_level: i16,
        new_level: i16,
    ) -> Vec<usize> {
        let mut changed = Vec::new();
        for index in 0..HOTKEY_TOTAL {
            let HotkeySlotContent::Skill {
                skill: slot_skill,
                level,
            } = self.slots[index]
            else {
                continue;
            };
            if slot_skill != skill {
                continue;
            }
            if new_level == before_level + 1 {
                if level == before_level {
                    self.slots[index] = HotkeySlotContent::Skill {
                        skill,
                        level: new_level,
                    };
                    changed.push(index);
                }
            } else if level > new_level {
                self.slots[index] = if new_level == 0 {
                    HotkeySlotContent::Empty
                } else {
                    HotkeySlotContent::Skill {
                        skill,
                        level: new_level,
                    }
                };
                changed.push(index);
            }
        }
        changed
    }

    pub fn to_server_format(&self, index: usize) -> (i8, u32, i16) {
        match self.get_slot(index) {
            HotkeySlotContent::Empty => (0, 0, 0),
            HotkeySlotContent::Skill { skill, level } => (1, skill.id(), level),
            HotkeySlotContent::Item { item_id } => (0, item_id as u32, 0),
        }
    }

    pub fn clear(&mut self) {
        self.slots = [HotkeySlotContent::Empty; HOTKEY_TOTAL];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_from_server_and_query() {
        let mut bar = HotkeyBar::new();
        let server_data = vec![
            (1i8, 28u32, 5i16), // Skill: id=28, level=5
            (0, 501, 0),        // Item: item_id=501
            (0, 0, 0),          // Empty
            (1, 10, 3),         // Skill: id=10, level=3
        ];
        bar.set_from_server(&server_data);

        assert_eq!(
            bar.get_slot(0),
            HotkeySlotContent::Skill {
                skill: SkillEnum::AlHeal,
                level: 5
            }
        );
        assert_eq!(bar.get_slot(1), HotkeySlotContent::Item { item_id: 501 });
        assert_eq!(bar.get_slot(2), HotkeySlotContent::Empty);
        assert_eq!(
            bar.get_slot(3),
            HotkeySlotContent::Skill {
                skill: SkillEnum::MgSight,
                level: 3
            }
        );
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
        bar.set_slot(
            5,
            HotkeySlotContent::Skill {
                skill: SkillEnum::AlHeal,
                level: 10,
            },
        );
        assert_eq!(
            bar.get_slot(5),
            HotkeySlotContent::Skill {
                skill: SkillEnum::AlHeal,
                level: 10
            }
        );

        bar.set_slot(5, HotkeySlotContent::Item { item_id: 501 });
        assert_eq!(bar.get_slot(5), HotkeySlotContent::Item { item_id: 501 });

        bar.clear_slot(5);
        assert_eq!(bar.get_slot(5), HotkeySlotContent::Empty);
    }

    #[test]
    fn skill_level_change_follows_slots_at_the_learned_level() {
        let mut bar = HotkeyBar::new();
        bar.set_from_server(&[
            (1, 5, 5),   // SM_BASH at the learned level
            (1, 5, 2),   // SM_BASH deliberately down-ranked
            (1, 10, 5),  // another skill, same level
            (0, 501, 0), // an item
        ]);

        assert_eq!(
            bar.apply_skill_level_change(SkillEnum::SmBash, 5, 6),
            vec![0]
        );
        assert_eq!(bar.to_server_format(0), (1, 5, 6));
        assert_eq!(bar.to_server_format(1), (1, 5, 2));
        assert_eq!(bar.to_server_format(2), (1, 10, 5));
        assert_eq!(bar.to_server_format(3), (0, 501, 0));

        // A reset pulls every slot back to what is still castable.
        assert_eq!(
            bar.apply_skill_level_change(SkillEnum::SmBash, 6, 1),
            vec![0, 1]
        );
        assert_eq!(bar.to_server_format(0), (1, 5, 1));
        assert_eq!(bar.to_server_format(1), (1, 5, 1));

        // Unlearning empties them.
        assert_eq!(
            bar.apply_skill_level_change(SkillEnum::SmBash, 1, 0),
            vec![0, 1]
        );
        assert_eq!(bar.get_slot(0), HotkeySlotContent::Empty);
        assert_eq!(bar.get_slot(1), HotkeySlotContent::Empty);
        assert_eq!(bar.to_server_format(2), (1, 10, 5));
    }

    #[test]
    fn to_server_format_conversion() {
        let mut bar = HotkeyBar::new();
        bar.set_slot(
            0,
            HotkeySlotContent::Skill {
                skill: SkillEnum::AlHeal,
                level: 5,
            },
        );
        bar.set_slot(1, HotkeySlotContent::Item { item_id: 501 });

        assert_eq!(bar.to_server_format(0), (1, 28, 5));
        assert_eq!(bar.to_server_format(1), (0, 501, 0));
        assert_eq!(bar.to_server_format(2), (0, 0, 0));
    }
}
