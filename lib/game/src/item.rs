use crate::data_table::item_resource_table::ItemResourceTable;
use models::enums::EnumWithMaskValueU64;
use models::enums::item::{EquipmentLocation, ItemType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryTab {
    Usable,
    Equip,
    Etc,
}

pub fn item_tab(item_type: ItemType) -> InventoryTab {
    match item_type {
        ItemType::Healing | ItemType::Usable | ItemType::DelayConsume | ItemType::Cash => {
            InventoryTab::Usable
        }
        ItemType::Unknown | ItemType::Armor | ItemType::Weapon | ItemType::PetArmor => {
            InventoryTab::Equip
        }
        _ => InventoryTab::Etc,
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub index: u16,
    pub item_id: u16,
    pub item_type: ItemType,
    pub count: i16,
    pub is_identified: bool,
    pub is_damaged: bool,
    pub refining_level: u8,
    pub slot: [u16; 4],
    pub location: u16,
    pub wear_state: u16,
    pub name: String,
    pub resource_name: Option<String>,
}

impl Item {
    pub fn tab(&self) -> InventoryTab {
        item_tab(self.item_type)
    }

    pub fn icon_path(&self) -> Option<String> {
        self.resource_name
            .as_ref()
            .map(|name| format!("data/texture/유저인터페이스/item/{name}.bmp"))
    }

    pub fn is_equipment(&self) -> bool {
        self.tab() == InventoryTab::Equip || self.is_ammunition()
    }

    pub fn is_weapon(&self) -> bool {
        self.item_type == ItemType::Weapon
    }

    pub fn is_equipped(&self) -> bool {
        self.wear_state != 0
    }

    pub fn is_ammunition(&self) -> bool {
        self.item_type == ItemType::Ammo
    }

    pub fn is_card(&self) -> bool {
        self.item_type == ItemType::Card
    }

    pub fn equip_location(&self) -> u16 {
        if self.is_ammunition() {
            EquipmentLocation::Ammo.as_flag() as u16
        } else {
            self.location
        }
    }

    pub fn resolve_resource_name(&mut self, table: &ItemResourceTable) {
        if self.resource_name.is_none() {
            self.resource_name = table
                .get_resource_name_for(self.item_id, self.is_identified)
                .map(|s| s.to_string());
        }
    }
}
