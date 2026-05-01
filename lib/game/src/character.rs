use crate::cooldown::CooldownTracker;
use crate::event::CharacterInfo;
use crate::hotkey::HotkeyBar;
use crate::inventory::InventoryData;
use crate::skill::SkillList;

pub struct Character {
    pub inventory: InventoryData,
    pub skills: SkillList,
    pub hotkeys: HotkeyBar,
    pub cooldowns: CooldownTracker,
    pub skill_point: u32,
    pub status_point: u32,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u16,
    pub max_sp: u16,
    pub base_level: u16,
    pub job_level: u32,
    pub str: u8,
    pub agi: u8,
    pub vit: u8,
    pub int: u8,
    pub dex: u8,
    pub luk: u8,
}

impl Default for Character {
    fn default() -> Self {
        Self::new()
    }
}

impl Character {
    pub fn new() -> Self {
        Self {
            inventory: InventoryData::new(),
            skills: SkillList::new(),
            hotkeys: HotkeyBar::new(),
            cooldowns: CooldownTracker::new(),
            skill_point: 0,
            status_point: 0,
            hp: 0,
            max_hp: 0,
            sp: 0,
            max_sp: 0,
            base_level: 0,
            job_level: 0,
            str: 0,
            agi: 0,
            vit: 0,
            int: 0,
            dex: 0,
            luk: 0,
        }
    }

    pub fn init_from_info(&mut self, info: &CharacterInfo) {
        self.hp = info.hp;
        self.max_hp = info.max_hp;
        self.sp = info.sp;
        self.max_sp = info.max_sp;
        self.base_level = info.base_level;
        self.job_level = info.job_level;
        self.str = info.str;
        self.agi = info.agi;
        self.vit = info.vit;
        self.int = info.int;
        self.dex = info.dex;
        self.luk = info.luk;
    }

    pub fn hp_percentage(&self) -> f32 {
        if self.max_hp > 0 {
            self.hp as f32 / self.max_hp as f32
        } else {
            0.0
        }
    }

    pub fn sp_percentage(&self) -> f32 {
        if self.max_sp > 0 {
            self.sp as f32 / self.max_sp as f32
        } else {
            0.0
        }
    }

    pub fn apply_parameter_changed(&mut self, var_id: u16, value: i32) -> Option<u16> {
        use models::enums::status::StatusTypes;
        use models::enums::EnumWithNumberValue;
        let Ok(status) = StatusTypes::try_from_value(var_id as usize) else {
            return None;
        };
        match status {
            StatusTypes::Speed => return Some(value as u16),
            StatusTypes::Hp => self.hp = value as u32,
            StatusTypes::Maxhp => self.max_hp = value as u32,
            StatusTypes::Sp => self.sp = value as u16,
            StatusTypes::Maxsp => self.max_sp = value as u16,
            StatusTypes::Baselevel => self.base_level = value as u16,
            StatusTypes::Str => self.str = value as u8,
            StatusTypes::Agi => self.agi = value as u8,
            StatusTypes::Vit => self.vit = value as u8,
            StatusTypes::Int => self.int = value as u8,
            StatusTypes::Dex => self.dex = value as u8,
            StatusTypes::Luk => self.luk = value as u8,
            StatusTypes::Joblevel => self.job_level = value as u32,
            StatusTypes::Weight => self.inventory.weight = value,
            StatusTypes::Maxweight => self.inventory.max_weight = value,
            StatusTypes::Zeny => self.inventory.zeny = value,
            StatusTypes::Skillpoint => self.skill_point = value as u32,
            StatusTypes::Statuspoint => self.status_point = value as u32,
            _ => {}
        }
        None
    }

    pub fn apply_status_changed(&mut self, status_type: u32, base: i32) {
        use models::enums::status::StatusTypes;
        use models::enums::EnumWithNumberValue;
        if let Ok(status) = StatusTypes::try_from_value(status_type as usize) {
            match status {
                StatusTypes::Str => self.str = base as u8,
                StatusTypes::Agi => self.agi = base as u8,
                StatusTypes::Vit => self.vit = base as u8,
                StatusTypes::Int => self.int = base as u8,
                StatusTypes::Dex => self.dex = base as u8,
                StatusTypes::Luk => self.luk = base as u8,
                _ => {}
            }
        }
    }

    pub fn clear(&mut self) {
        self.inventory.clear();
        self.skills.clear();
        self.hotkeys.clear();
        self.cooldowns.clear();
        self.skill_point = 0;
        self.status_point = 0;
        self.hp = 0;
        self.max_hp = 0;
        self.sp = 0;
        self.max_sp = 0;
        self.base_level = 0;
        self.job_level = 0;
        self.str = 0;
        self.agi = 0;
        self.vit = 0;
        self.int = 0;
        self.dex = 0;
        self.luk = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_changed_updates_stats_and_returns_speed() {
        let mut char = Character::new();

        // HP (var_id 5)
        assert!(char.apply_parameter_changed(5, 500).is_none());
        assert_eq!(char.hp, 500);

        // Max HP (var_id 6)
        char.apply_parameter_changed(6, 1000);
        assert_eq!(char.max_hp, 1000);
        assert!((char.hp_percentage() - 0.5).abs() < 0.01);

        // SP (var_id 7)
        char.apply_parameter_changed(7, 80);
        assert_eq!(char.sp, 80);

        // Zeny (var_id 20)
        char.apply_parameter_changed(20, 999999);
        assert_eq!(char.inventory.zeny, 999999);

        // Speed (var_id 0) returns Some
        let result = char.apply_parameter_changed(0, 150);
        assert_eq!(result, Some(150));

        // STR (var_id 13)
        char.apply_parameter_changed(13, 42);
        assert_eq!(char.str, 42);

        // Skill points (var_id 12)
        char.apply_parameter_changed(12, 5);
        assert_eq!(char.skill_point, 5);
    }

    #[test]
    fn status_changed_updates_base_stats() {
        let mut char = Character::new();
        char.apply_status_changed(13, 50); // STR
        assert_eq!(char.str, 50);
        char.apply_status_changed(14, 30); // AGI
        assert_eq!(char.agi, 30);
        char.apply_status_changed(17, 99); // DEX
        assert_eq!(char.dex, 99);
    }
}
