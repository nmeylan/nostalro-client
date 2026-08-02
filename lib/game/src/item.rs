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

/// First card slot of a forged (blacksmith) item; the remaining slots hold the
/// star-crumb count, the element and the forger's char id instead of cards.
pub const CARD0_FORGE: u16 = 0x00ff;
/// Same layout as [`CARD0_FORGE`] but without star crumbs or element.
pub const CARD0_CREATE: u16 = 0x00fe;

/// Flame Heart, Mystic Frozen, Rough Wind and Great Nature.
const FORGE_ELEMENT_ITEMS: std::ops::RangeInclusive<u16> = 994..=997;
pub const STAR_CRUMB: u16 = 1000;

pub fn is_forge_element_item(item_id: u16) -> bool {
    FORGE_ELEMENT_ITEMS.contains(&item_id)
}

/// What a forge accepts in its three optional slots.
pub fn is_forge_material_item(item_id: u16) -> bool {
    item_id == STAR_CRUMB || is_forge_element_item(item_id)
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
            .map(|name| ragnarok_resources::ui::item::icon(name))
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

    pub fn is_forged(&self) -> bool {
        self.slot[0] == CARD0_FORGE
    }

    pub fn is_created(&self) -> bool {
        self.slot[0] == CARD0_CREATE && self.slot[1] == 0
    }

    /// Char id of the blacksmith/alchemist who made this item.
    pub fn producer_char_id(&self) -> Option<u32> {
        (self.is_forged() || self.is_created())
            .then(|| self.slot[2] as u32 | ((self.slot[3] as u32) << 16))
    }

    /// Star crumbs used at forge time, 0..=3.
    pub fn star_crumb_count(&self) -> u8 {
        if !self.is_forged() {
            return 0;
        }
        match self.slot[1] >> 8 {
            5 => 1,
            10 => 2,
            15 => 3,
            _ => 0,
        }
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
