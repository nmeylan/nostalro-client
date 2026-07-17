use crate::App;
use models::enums::EnumWithNumberValue;
use models::enums::item::ItemType;
use ragnarok_game::inventory::{EquipmentItemData, NormalItemData};
use ragnarok_game::item::Item;

impl App {
    pub(super) fn handle_storage_normal_items(&mut self, items: Vec<NormalItemData>) {
        let icon_paths = self
            .game
            .character
            .storage
            .buffer_normal_items(items, &self.game.data_table);
        self.preload_item_icons(icon_paths);
    }

    pub(super) fn handle_storage_equip_items(&mut self, items: Vec<EquipmentItemData>) {
        let icon_paths = self
            .game
            .character
            .storage
            .buffer_equipment_items(items, &self.game.data_table);
        self.preload_item_icons(icon_paths);
    }

    pub(super) fn handle_storage_opened(&mut self, cur: i16, max: i16) {
        if self.game.character.storage.is_open() {
            self.game.character.storage.set_counts(cur, max);
        } else {
            self.game.character.storage.open_with_pending(cur, max);
            self.game.character.inventory.open();
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn handle_storage_item_added(
        &mut self,
        index: u16,
        item_id: u16,
        count: i16,
        item_type: u8,
        is_identified: bool,
        is_damaged: bool,
        refining_level: u8,
        slot: [u16; 4],
    ) {
        let resolved_type = if item_type != 0 {
            ItemType::from_value(item_type as usize)
        } else {
            self.game
                .character
                .inventory
                .all_items()
                .iter()
                .find(|i| i.item_id == item_id)
                .map(|i| i.item_type)
                .unwrap_or(ItemType::from_value(item_type as usize))
        };
        let name = self
            .game
            .data_table
            .item_name
            .as_ref()
            .map(|t| t.get_name_or_id_for(item_id, is_identified))
            .unwrap_or_else(|| format!("Item #{item_id}"));
        let resource_name = self.game.data_table.item_resource.as_ref().and_then(|t| {
            t.get_resource_name_for(item_id, is_identified)
                .map(|s| s.to_string())
        });
        self.game.character.storage.add_item(Item {
            index,
            item_id,
            item_type: resolved_type,
            count,
            is_identified,
            is_damaged,
            refining_level,
            slot,
            location: 0,
            wear_state: 0,
            name,
            resource_name,
        });
        let icon_path = self
            .game
            .character
            .storage
            .get_item(index)
            .and_then(|item| item.icon_path());
        if let Some(path) = icon_path {
            self.preload_item_icons(vec![path]);
        }
    }

    pub(super) fn handle_storage_item_removed(&mut self, index: u16, amount: i16) {
        self.game.character.storage.remove(index, amount);
    }

    pub(super) fn handle_storage_closed(&mut self) {
        self.game.character.storage.clear();
    }
}
