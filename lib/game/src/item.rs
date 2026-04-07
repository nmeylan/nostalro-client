use crate::item_resource_table::ItemResourceTable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryTab {
    Usable,
    Equip,
    Etc,
}

pub fn item_tab(item_type: u8) -> InventoryTab {
    match item_type {
        0 | 2 | 11 | 18 => InventoryTab::Usable,
        1 | 4 | 5 => InventoryTab::Equip,
        _ => InventoryTab::Etc,
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub index: u16,
    pub item_id: u16,
    pub item_type: u8,
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
        self.resource_name.as_ref()
            .map(|name| format!("data/texture/유저인터페이스/item/{name}.bmp"))
    }

    pub fn is_equipment(&self) -> bool {
        self.tab() == InventoryTab::Equip
    }

    pub fn is_equipped(&self) -> bool {
        self.wear_state != 0
    }

    pub fn resolve_resource_name(&mut self, table: &ItemResourceTable) {
        if self.resource_name.is_none() {
            self.resource_name = table.get_resource_name(self.item_id).map(|s| s.to_string());
        }
    }
}
