use models::enums::EnumWithMaskValueU64;
pub use models::enums::item::EquipmentLocation;
use crate::item::Item;
use crate::item::InventoryTab;
use crate::item_resource_table::ItemResourceTable;

#[derive(Debug)]
pub struct InventoryData {
    items: Vec<Item>,
    pub active_tab: InventoryTab,
    pub weight: i32,
    pub max_weight: i32,
    pub zeny: i32,
    open: bool,
}

impl InventoryData {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            active_tab: InventoryTab::Usable,
            weight: 0,
            max_weight: 0,
            zeny: 0,
            open: false,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn add_item(&mut self, item: Item) {
        if let Some(existing) = self.items.iter_mut().find(|i| i.index == item.index) {
            existing.count = item.count;
            existing.wear_state = item.wear_state;
        } else {
            self.items.push(item);
        }
    }

    pub fn remove_item(&mut self, index: u16) {
        self.items.retain(|i| i.index != index);
    }

    pub fn update_item_count(&mut self, index: u16, count: i16) {
        if count <= 0 {
            self.remove_item(index);
        } else if let Some(item) = self.items.iter_mut().find(|i| i.index == index) {
            item.count = count;
        }
    }

    pub fn subtract_item_count(&mut self, index: u16, amount: i16) {
        if let Some(item) = self.items.iter_mut().find(|i| i.index == index) {
            item.count -= amount;
            if item.count <= 0 {
                self.remove_item(index);
            }
        }
    }

    pub fn update_wear_state(&mut self, index: u16, wear_location: u16) {
        if let Some(item) = self.items.iter_mut().find(|i| i.index == index) {
            item.wear_state = wear_location;
        }
    }

    pub fn clear_wear_state(&mut self, index: u16) {
        if let Some(item) = self.items.iter_mut().find(|i| i.index == index) {
            item.wear_state = 0;
        }
    }

    pub fn get_item(&self, index: u16) -> Option<&Item> {
        self.items.iter().find(|i| i.index == index)
    }

    pub fn filtered_items(&self) -> Vec<&Item> {
        self.items.iter()
            .filter(|item| item.tab() == self.active_tab && !item.is_equipped())
            .collect()
    }

    pub fn equipped_in_slot(&self, slot: EquipmentLocation) -> Option<&Item> {
        let mask = slot.as_flag() as u16;
        self.items.iter().find(|i| i.wear_state & mask != 0)
    }

    pub fn all_items(&self) -> &[Item] {
        &self.items
    }

    pub fn resolve_resource_names(&mut self, table: &ItemResourceTable) {
        for item in &mut self.items {
            item.resolve_resource_name(table);
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

// Data carriers for GameEvent transport (before name/resource resolution)
#[derive(Debug, Clone)]
pub struct NormalItemData {
    pub index: i16,
    pub item_id: u16,
    pub item_type: u8,
    pub is_identified: bool,
    pub count: i16,
    pub wear_state: u16,
}

#[derive(Debug, Clone)]
pub struct EquipmentItemData {
    pub index: i16,
    pub item_id: u16,
    pub item_type: u8,
    pub is_identified: bool,
    pub location: u16,
    pub wear_state: u16,
    pub is_damaged: bool,
    pub refining_level: u8,
    pub slot: [u16; 4],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::item_tab;

    fn make_normal_item(index: u16, item_id: u16, item_type: u8, count: i16) -> Item {
        Item {
            index,
            item_id,
            item_type,
            count,
            is_identified: true,
            is_damaged: false,
            refining_level: 0,
            slot: [0; 4],
            location: 0,
            wear_state: 0,
            name: format!("Item {item_id}"),
            resource_name: Some(format!("item_{item_id}")),
        }
    }

    fn make_equip_item(index: u16, item_id: u16, location: u16) -> Item {
        Item {
            index,
            item_id,
            item_type: 5, // armor
            count: 1,
            is_identified: true,
            is_damaged: false,
            refining_level: 0,
            slot: [0; 4],
            location,
            wear_state: 0,
            name: format!("Equip {item_id}"),
            resource_name: Some(format!("equip_{item_id}")),
        }
    }

    #[test]
    fn inventory_lifecycle() {
        let mut inv = InventoryData::new();
        assert!(!inv.is_open());

        inv.toggle();
        assert!(inv.is_open());
        inv.toggle();
        assert!(!inv.is_open());

        // Add mixed items: healing (type 0), weapon (type 4), card (type 6)
        inv.add_item(make_normal_item(1, 501, 0, 10)); // Red Potion - usable
        inv.add_item(make_normal_item(2, 502, 2, 5));   // Orange Potion - usable
        inv.add_item(make_equip_item(3, 1201, 2));       // Knife - equip
        inv.add_item(make_normal_item(4, 4001, 6, 1));   // Card - etc
        inv.add_item(make_normal_item(5, 7001, 3, 50));  // Etc item

        // Tab filtering
        inv.active_tab = InventoryTab::Usable;
        assert_eq!(inv.filtered_items().len(), 2);

        inv.active_tab = InventoryTab::Equip;
        assert_eq!(inv.filtered_items().len(), 1);
        assert_eq!(inv.filtered_items()[0].item_id, 1201);

        inv.active_tab = InventoryTab::Etc;
        assert_eq!(inv.filtered_items().len(), 2);

        // Equipped items are filtered out
        inv.update_wear_state(3, 2);
        inv.active_tab = InventoryTab::Equip;
        assert_eq!(inv.filtered_items().len(), 0);

        inv.clear();
        assert!(inv.all_items().is_empty());
    }

    #[test]
    fn item_mutations() {
        let mut inv = InventoryData::new();
        inv.add_item(make_normal_item(1, 501, 0, 10));
        inv.add_item(make_equip_item(2, 1201, 2));

        // Use item reduces count
        inv.update_item_count(1, 9);
        assert_eq!(inv.get_item(1).unwrap().count, 9);

        // Use item until 0 removes it
        inv.update_item_count(1, 0);
        assert!(inv.get_item(1).is_none());
        assert_eq!(inv.all_items().len(), 1);

        // Equip / unequip
        inv.update_wear_state(2, 2);
        assert!(inv.get_item(2).unwrap().is_equipped());
        assert_eq!(inv.get_item(2).unwrap().wear_state, 2);

        inv.clear_wear_state(2);
        assert!(!inv.get_item(2).unwrap().is_equipped());

        // Remove item directly
        inv.remove_item(2);
        assert!(inv.all_items().is_empty());
    }

    #[test]
    fn pickup_and_tab_classification() {
        let mut inv = InventoryData::new();

        // Pickup adds to correct tab
        inv.add_item(make_normal_item(10, 501, 0, 1));  // healing -> Usable
        inv.add_item(make_normal_item(11, 1101, 4, 1)); // weapon -> Equip
        inv.add_item(make_normal_item(12, 7001, 3, 1)); // etc -> Etc
        inv.add_item(make_normal_item(13, 1750, 10, 100)); // ammo -> Etc

        assert_eq!(item_tab(0), InventoryTab::Usable);
        assert_eq!(item_tab(2), InventoryTab::Usable);
        assert_eq!(item_tab(11), InventoryTab::Usable);
        assert_eq!(item_tab(4), InventoryTab::Equip);
        assert_eq!(item_tab(5), InventoryTab::Equip);
        assert_eq!(item_tab(1), InventoryTab::Equip);
        assert_eq!(item_tab(3), InventoryTab::Etc);
        assert_eq!(item_tab(6), InventoryTab::Etc);
        assert_eq!(item_tab(10), InventoryTab::Etc);

        // add_item with same index updates existing
        inv.add_item(make_normal_item(10, 501, 0, 5));
        assert_eq!(inv.get_item(10).unwrap().count, 5);
        assert_eq!(inv.all_items().len(), 4);

        // Remove and verify
        inv.remove_item(12);
        assert!(inv.get_item(12).is_none());
        assert_eq!(inv.all_items().len(), 3);
    }

    #[test]
    fn equipped_in_slot_lookup() {
        let mut inv = InventoryData::new();
        inv.add_item(make_equip_item(3, 1201, 2));  // Knife, location=HandRight
        inv.add_item(make_equip_item(5, 2101, 16)); // Armor, location=Armor

        // Nothing equipped yet
        assert!(inv.equipped_in_slot(EquipmentLocation::HandRight).is_none());

        // Equip knife in right hand
        inv.update_wear_state(3, 2);
        let item = inv.equipped_in_slot(EquipmentLocation::HandRight).unwrap();
        assert_eq!(item.index, 3);
        assert!(inv.equipped_in_slot(EquipmentLocation::Armor).is_none());

        // Equip armor
        inv.update_wear_state(5, 16);
        assert!(inv.equipped_in_slot(EquipmentLocation::Armor).is_some());

        // Unequip knife
        inv.clear_wear_state(3);
        assert!(inv.equipped_in_slot(EquipmentLocation::HandRight).is_none());
        assert!(inv.equipped_in_slot(EquipmentLocation::Armor).is_some());
    }

    #[test]
    fn headgear_equipped_in_slot() {
        let mut inv = InventoryData::new();
        // HeadTop mask=256, HeadMid mask=512, HeadLow mask=1
        inv.add_item(make_equip_item(10, 2220, 256));   // Hat → HeadTop
        inv.add_item(make_equip_item(11, 5001, 512));   // Sunglasses → HeadMid
        inv.add_item(make_equip_item(12, 5100, 1));     // Mouth mask → HeadLow

        // Equip all three
        inv.update_wear_state(10, 256);
        inv.update_wear_state(11, 512);
        inv.update_wear_state(12, 1);

        assert_eq!(inv.equipped_in_slot(EquipmentLocation::HeadTop).unwrap().index, 10);
        assert_eq!(inv.equipped_in_slot(EquipmentLocation::HeadMid).unwrap().index, 11);
        assert_eq!(inv.equipped_in_slot(EquipmentLocation::HeadLow).unwrap().index, 12);

        // Multi-slot headgear (HeadTop+HeadMid = 256|512 = 768)
        inv.clear_wear_state(10);
        inv.clear_wear_state(11);
        inv.add_item(make_equip_item(13, 2230, 768));
        inv.update_wear_state(13, 768);

        assert_eq!(inv.equipped_in_slot(EquipmentLocation::HeadTop).unwrap().index, 13);
        assert_eq!(inv.equipped_in_slot(EquipmentLocation::HeadMid).unwrap().index, 13);
        assert_eq!(inv.equipped_in_slot(EquipmentLocation::HeadLow).unwrap().index, 12);
    }

    #[test]
    fn icon_path_construction() {
        let item = make_normal_item(1, 501, 0, 1);
        assert_eq!(
            item.icon_path().unwrap(),
            "data/texture/유저인터페이스/item/item_501.bmp"
        );

        let mut no_resource = make_normal_item(2, 502, 0, 1);
        no_resource.resource_name = None;
        assert!(no_resource.icon_path().is_none());
    }
}
