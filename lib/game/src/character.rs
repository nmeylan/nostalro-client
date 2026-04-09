use crate::inventory::InventoryData;

pub struct Character {
    pub inventory: InventoryData,
}

impl Character {
    pub fn new() -> Self {
        Self {
            inventory: InventoryData::new(),
        }
    }
}
