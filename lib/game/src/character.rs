use crate::event::CharacterInfo;
use crate::inventory::InventoryData;

pub struct Character {
    pub inventory: InventoryData,
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

impl Character {
    pub fn new() -> Self {
        Self {
            inventory: InventoryData::new(),
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

    pub fn clear(&mut self) {
        self.inventory.clear();
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
