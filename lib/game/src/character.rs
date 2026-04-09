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
    
    pub fn clear(&mut self) {
       self.inventory.clear(); 
    }
}
