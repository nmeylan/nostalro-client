use crate::cooldown::CooldownTracker;
use crate::event::CharacterInfo;
use crate::hotkey::HotkeyBar;
use crate::inventory::{CartData, InventoryData, StorageData};
use crate::mail::MailState;
use crate::skill::SkillList;
use crate::trade::TradeData;
use models::enums::class::JobName;
use models::enums::{EnumWithNumberValue, EnumWithStringValue};

/// Times are local-clock milliseconds; `end_ms` is `None` for permanent statuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveStatus {
    pub efst: i16,
    pub val1: i32,
    pub start_ms: u64,
    pub end_ms: Option<u64>,
    pub icon_loaded: bool,
}

pub struct Character {
    pub inventory: InventoryData,
    pub cart: CartData,
    pub storage: StorageData,
    pub trade: TradeData,
    pub mail: MailState,
    pub skills: SkillList,
    pub hotkeys: HotkeyBar,
    pub cooldowns: CooldownTracker,
    pub name: String,
    pub class: u16,
    pub skill_point: u32,
    pub status_point: u32,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u16,
    pub max_sp: u16,
    pub base_level: u16,
    pub job_level: u32,
    pub base_exp: u32,
    pub job_exp: u32,
    pub next_base_exp: u32,
    pub next_job_exp: u32,
    pub str: u8,
    pub agi: u8,
    pub vit: u8,
    pub int: u8,
    pub dex: u8,
    pub luk: u8,
    pub str_bonus: i16,
    pub agi_bonus: i16,
    pub vit_bonus: i16,
    pub int_bonus: i16,
    pub dex_bonus: i16,
    pub luk_bonus: i16,
    pub str_cost: u16,
    pub agi_cost: u16,
    pub vit_cost: u16,
    pub int_cost: u16,
    pub dex_cost: u16,
    pub luk_cost: u16,
    pub atk1: i32,
    pub atk2: i32,
    pub matk1: i32,
    pub matk2: i32,
    pub def1: i32,
    pub def2: i32,
    pub mdef1: i32,
    pub mdef2: i32,
    pub hit: i32,
    pub flee1: i32,
    pub flee2: i32,
    pub critical: i32,
    pub aspd: i32,
    pub effect_state: i32,
    pub cart_design: Option<u8>,
    pub active_statuses: Vec<ActiveStatus>,
    /// Married partner's name (from ZC_COUPLENAME); empty when unpartnered.
    pub partner_name: String,
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
            cart: CartData::new(),
            storage: StorageData::new(),
            trade: TradeData::new(),
            mail: MailState::new(),
            skills: SkillList::new(),
            hotkeys: HotkeyBar::new(),
            cooldowns: CooldownTracker::new(),
            name: String::new(),
            class: 0,
            skill_point: 0,
            status_point: 0,
            hp: 0,
            max_hp: 0,
            sp: 0,
            max_sp: 0,
            base_level: 0,
            job_level: 0,
            base_exp: 0,
            job_exp: 0,
            next_base_exp: 0,
            next_job_exp: 0,
            str: 0,
            agi: 0,
            vit: 0,
            int: 0,
            dex: 0,
            luk: 0,
            str_bonus: 0,
            agi_bonus: 0,
            vit_bonus: 0,
            int_bonus: 0,
            dex_bonus: 0,
            luk_bonus: 0,
            str_cost: 0,
            agi_cost: 0,
            vit_cost: 0,
            int_cost: 0,
            dex_cost: 0,
            luk_cost: 0,
            atk1: 0,
            atk2: 0,
            matk1: 0,
            matk2: 0,
            def1: 0,
            def2: 0,
            mdef1: 0,
            mdef2: 0,
            hit: 0,
            flee1: 0,
            flee2: 0,
            critical: 0,
            aspd: 0,
            effect_state: 0,
            cart_design: None,
            active_statuses: Vec::new(),
            partner_name: String::new(),
        }
    }

    /// Sentinel the server sends for infinite-duration statuses: any tick `<= 0` is
    /// rewritten to this before transmission, so the icon must show no countdown.
    pub const PERMANENT_STATUS_TICK: u64 = 9999;

    /// `life_ms == 0` or [`PERMANENT_STATUS_TICK`] means permanent (no expiry).
    /// Re-applying an existing status updates timing in place.
    pub fn apply_status(
        &mut self,
        efst: i16,
        val1: i32,
        now_ms: u64,
        life_ms: u64,
        icon_loaded: bool,
    ) {
        let end_ms = if life_ms == 0 || life_ms == Self::PERMANENT_STATUS_TICK {
            None
        } else {
            Some(now_ms + life_ms)
        };
        if let Some(existing) = self.active_statuses.iter_mut().find(|s| s.efst == efst) {
            existing.val1 = val1;
            existing.start_ms = now_ms;
            existing.end_ms = end_ms;
            existing.icon_loaded = icon_loaded;
        } else {
            self.active_statuses.push(ActiveStatus {
                efst,
                val1,
                start_ms: now_ms,
                end_ms,
                icon_loaded,
            });
        }
    }

    pub fn clear_status(&mut self, efst: i16) {
        self.active_statuses.retain(|s| s.efst != efst);
    }

    pub fn prune_expired(&mut self, now_ms: u64) {
        self.active_statuses
            .retain(|s| s.end_ms.is_none_or(|end| now_ms < end));
    }

    pub fn init_from_info(&mut self, info: &CharacterInfo) {
        self.name = info.name.clone();
        self.class = info.class;
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
        self.effect_state = info.effect_state;
        self.inventory.zeny = info.zeny;
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

    pub fn base_exp_percentage(&self) -> f32 {
        if self.next_base_exp > 0 {
            self.base_exp as f32 / self.next_base_exp as f32
        } else {
            0.0
        }
    }

    pub fn job_exp_percentage(&self) -> f32 {
        if self.next_job_exp > 0 {
            self.job_exp as f32 / self.next_job_exp as f32
        } else {
            0.0
        }
    }

    pub fn apply_parameter_changed(&mut self, var_id: u16, value: i32) -> Option<u16> {
        use models::enums::EnumWithNumberValue;
        use models::enums::status::StatusTypes;
        let Ok(status) = StatusTypes::try_from_value(var_id as usize) else {
            return None;
        };
        match status {
            StatusTypes::Speed => return Some(value as u16),
            StatusTypes::Hp => self.hp = value as u32,
            StatusTypes::Maxhp => self.max_hp = value as u32,
            StatusTypes::Sp => self.sp = value as u16,
            StatusTypes::Maxsp => self.max_sp = value as u16,
            StatusTypes::Baseexp => self.base_exp = value as u32,
            StatusTypes::Jobexp => self.job_exp = value as u32,
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
            StatusTypes::Nextbaseexp => self.next_base_exp = value as u32,
            StatusTypes::Nextjobexp => self.next_job_exp = value as u32,
            StatusTypes::StrNextLevelIncreaseCost => self.str_cost = value as u16,
            StatusTypes::AgiNextLevelIncreaseCost => self.agi_cost = value as u16,
            StatusTypes::VitNextLevelIncreaseCost => self.vit_cost = value as u16,
            StatusTypes::IntNextLevelIncreaseCost => self.int_cost = value as u16,
            StatusTypes::DexNextLevelIncreaseCost => self.dex_cost = value as u16,
            StatusTypes::LukNextLevelIncreaseCost => self.luk_cost = value as u16,
            StatusTypes::Atk1 => self.atk1 = value,
            StatusTypes::Atk2 => self.atk2 = value,
            StatusTypes::Matk1 => self.matk1 = value,
            StatusTypes::Matk2 => self.matk2 = value,
            StatusTypes::Def1 => self.def1 = value,
            StatusTypes::Def2 => self.def2 = value,
            StatusTypes::Mdef1 => self.mdef1 = value,
            StatusTypes::Mdef2 => self.mdef2 = value,
            StatusTypes::Hit => self.hit = value,
            StatusTypes::Flee1 => self.flee1 = value,
            StatusTypes::Flee2 => self.flee2 = value,
            StatusTypes::Critical => self.critical = value,
            StatusTypes::Aspd => self.aspd = value,
            _ => {}
        }
        None
    }

    pub fn apply_status_changed(&mut self, status_type: u32, base: i32, bonus: i32) {
        use models::enums::EnumWithNumberValue;
        use models::enums::status::StatusTypes;
        if let Ok(status) = StatusTypes::try_from_value(status_type as usize) {
            match status {
                StatusTypes::Str => {
                    self.str = base as u8;
                    self.str_bonus = bonus as i16;
                }
                StatusTypes::Agi => {
                    self.agi = base as u8;
                    self.agi_bonus = bonus as i16;
                }
                StatusTypes::Vit => {
                    self.vit = base as u8;
                    self.vit_bonus = bonus as i16;
                }
                StatusTypes::Int => {
                    self.int = base as u8;
                    self.int_bonus = bonus as i16;
                }
                StatusTypes::Dex => {
                    self.dex = base as u8;
                    self.dex_bonus = bonus as i16;
                }
                StatusTypes::Luk => {
                    self.luk = base as u8;
                    self.luk_bonus = bonus as i16;
                }
                _ => {}
            }
        }
    }

    pub fn job_class_name(&self) -> &'static str {
        JobName::try_from_value(self.class as usize)
            .map(|j| j.as_str())
            .unwrap_or("Novice")
    }

    pub fn clear(&mut self) {
        self.inventory.clear();
        self.cart.clear();
        self.skills.clear();
        self.hotkeys.clear();
        self.cooldowns.clear();
        self.name.clear();
        self.class = 0;
        self.skill_point = 0;
        self.status_point = 0;
        self.hp = 0;
        self.max_hp = 0;
        self.sp = 0;
        self.max_sp = 0;
        self.base_level = 0;
        self.job_level = 0;
        self.base_exp = 0;
        self.job_exp = 0;
        self.next_base_exp = 0;
        self.next_job_exp = 0;
        self.str = 0;
        self.agi = 0;
        self.vit = 0;
        self.int = 0;
        self.dex = 0;
        self.luk = 0;
        self.str_bonus = 0;
        self.agi_bonus = 0;
        self.vit_bonus = 0;
        self.int_bonus = 0;
        self.dex_bonus = 0;
        self.luk_bonus = 0;
        self.str_cost = 0;
        self.agi_cost = 0;
        self.vit_cost = 0;
        self.int_cost = 0;
        self.dex_cost = 0;
        self.luk_cost = 0;
        self.atk1 = 0;
        self.atk2 = 0;
        self.matk1 = 0;
        self.matk2 = 0;
        self.def1 = 0;
        self.def2 = 0;
        self.mdef1 = 0;
        self.mdef2 = 0;
        self.hit = 0;
        self.flee1 = 0;
        self.flee2 = 0;
        self.critical = 0;
        self.aspd = 0;
        self.effect_state = 0;
        self.cart_design = None;
        self.active_statuses.clear();
        self.partner_name.clear();
    }
}

pub fn job_class_name(class_id: u16) -> &'static str {
    use models::enums::EnumWithNumberValue;
    use models::enums::class::JobName;
    match JobName::try_from_value(class_id as usize) {
        Ok(job) => match job {
            JobName::Novice => "Novice",
            JobName::Swordsman => "Swordsman",
            JobName::Mage => "Mage",
            JobName::Archer => "Archer",
            JobName::Acolyte => "Acolyte",
            JobName::Merchant => "Merchant",
            JobName::Thief => "Thief",
            JobName::Knight => "Knight",
            JobName::Priest => "Priest",
            JobName::Wizard => "Wizard",
            JobName::Blacksmith => "Blacksmith",
            JobName::Hunter => "Hunter",
            JobName::Assassin => "Assassin",
            JobName::Crusader => "Crusader",
            JobName::Monk => "Monk",
            JobName::Sage => "Sage",
            JobName::Rogue => "Rogue",
            JobName::Alchemist => "Alchemist",
            JobName::Bard => "Bard",
            JobName::Dancer => "Dancer",
            JobName::SuperNovice => "Super Novice",
            JobName::NoviceHigh => "Novice High",
            JobName::SwordsmanHigh => "Swordsman High",
            JobName::MageHigh => "Mage High",
            JobName::ArcherHigh => "Archer High",
            JobName::AcolyteHigh => "Acolyte High",
            JobName::MerchantHigh => "Merchant High",
            JobName::ThiefHigh => "Thief High",
            JobName::LordKnight => "Lord Knight",
            JobName::HighPriest => "High Priest",
            JobName::HighWizard => "High Wizard",
            JobName::Whitesmith => "Whitesmith",
            JobName::Sniper => "Sniper",
            JobName::AssassinCross => "Assassin Cross",
            JobName::Paladin => "Paladin",
            JobName::Champion => "Champion",
            JobName::Professor => "Professor",
            JobName::Stalker => "Stalker",
            JobName::Creator => "Creator",
            JobName::Clown => "Clown",
            JobName::Gypsy => "Gypsy",
            _ => "Adventurer",
        },
        Err(_) => "Adventurer",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_changed_updates_stats_and_returns_speed() {
        let mut char = Character::new();

        assert!(char.apply_parameter_changed(5, 500).is_none());
        assert_eq!(char.hp, 500);

        char.apply_parameter_changed(6, 1000);
        assert_eq!(char.max_hp, 1000);
        assert!((char.hp_percentage() - 0.5).abs() < 0.01);

        char.apply_parameter_changed(7, 80);
        assert_eq!(char.sp, 80);

        char.apply_parameter_changed(20, 999999);
        assert_eq!(char.inventory.zeny, 999999);

        let result = char.apply_parameter_changed(0, 150);
        assert_eq!(result, Some(150));

        char.apply_parameter_changed(13, 42);
        assert_eq!(char.str, 42);

        char.apply_parameter_changed(12, 5);
        assert_eq!(char.skill_point, 5);
    }

    #[test]
    fn init_from_info_populates_zeny_from_char_select() {
        let info = CharacterInfo {
            gid: 1,
            name: "Knight".into(),
            class: 7,
            base_level: 50,
            base_exp: 0,
            job_level: 42,
            map: "prontera".into(),
            slot: 0,
            head: 1,
            hair_color: 0,
            weapon: 2,
            head_top: 0,
            head_mid: 0,
            head_bottom: 0,
            shield: 0,
            sex: 1,
            hp: 3000,
            max_hp: 3500,
            sp: 100,
            max_sp: 150,
            str: 50,
            agi: 30,
            vit: 40,
            int: 10,
            dex: 20,
            luk: 10,
            effect_state: 0,
            zeny: 123_456,
        };

        let mut char = Character::new();
        char.init_from_info(&info);
        assert_eq!(char.inventory.zeny, 123_456);
    }

    #[test]
    fn status_changed_updates_base_stats() {
        let mut char = Character::new();
        char.apply_status_changed(13, 50, 5);
        assert_eq!(char.str, 50);
        assert_eq!(char.str_bonus, 5);
        char.apply_status_changed(14, 30, 0);
        assert_eq!(char.agi, 30);
        char.apply_status_changed(17, 99, -2);
        assert_eq!(char.dex, 99);
        assert_eq!(char.dex_bonus, -2);
    }

    #[test]
    fn status_lifecycle_apply_clear_and_prune() {
        let mut char = Character::new();
        char.apply_status(10, 0, 1_000, 30_000, true);
        char.apply_status(2, 0, 1_000, 0, true);
        assert_eq!(char.active_statuses.len(), 2);

        char.apply_status(10, 0, 5_000, 60_000, true);
        assert_eq!(char.active_statuses.len(), 2);
        assert_eq!(char.active_statuses[0].efst, 10);
        assert_eq!(char.active_statuses[0].end_ms, Some(65_000));

        char.clear_status(2);
        assert_eq!(char.active_statuses.len(), 1);

        char.prune_expired(64_000);
        assert_eq!(char.active_statuses.len(), 1);
        char.prune_expired(65_000);
        assert!(char.active_statuses.is_empty());
    }

    #[test]
    fn permanent_tick_sentinel_never_expires() {
        let mut char = Character::new();
        char.apply_status(35, 0, 1_000, Character::PERMANENT_STATUS_TICK, true);
        assert_eq!(char.active_statuses[0].end_ms, None);
        char.prune_expired(u64::MAX);
        assert_eq!(char.active_statuses.len(), 1);
    }
}
