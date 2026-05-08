use crate::App;
use ragnarok_game::display_name::format_equipment_display_name;
use ragnarok_game::entity::Entity;
use ragnarok_game::inventory::{EquipmentItemData, NormalItemData};
use ragnarok_game::item::Item;
use ragnarok_ui_component::game::card_insert_dialog::{CardInsertDialog, EligibleItem};
use ragnarok_ui_component::Window as UiWindow;

impl App {
    pub(super) fn handle_inventory_normal_items(&mut self, items: Vec<NormalItemData>) {
        let icon_paths = self
            .game
            .character
            .inventory
            .apply_normal_items(items, &self.game.data_table);
        self.preload_item_icons(icon_paths);
    }

    pub(super) fn handle_inventory_equipment_items(&mut self, items: Vec<EquipmentItemData>) {
        let icon_paths = self
            .game
            .character
            .inventory
            .apply_equipment_items(items, &self.game.data_table);
        self.preload_item_icons(icon_paths);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn handle_inventory_item_pickup(
        &mut self,
        index: u16,
        item_id: u16,
        count: u16,
        item_type: u8,
        is_identified: bool,
        is_damaged: bool,
        refining_level: u8,
        slot: [u16; 4],
        location: u16,
        result: u8,
    ) {
        if result != 0 {
            return;
        }
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
        self.game.character.inventory.add_item(Item {
            index,
            item_id,
            item_type,
            count: count as i16,
            is_identified,
            is_damaged,
            refining_level,
            slot,
            location,
            wear_state: 0,
            name: name.clone(),
            resource_name,
        });
        let icon_path = self
            .game
            .character
            .inventory
            .get_item(index)
            .and_then(|item| item.icon_path());
        let formatted_name = self
            .game
            .character
            .inventory
            .get_item(index)
            .map(|item| {
                format_equipment_display_name(
                    item,
                    self.game.data_table.item_slot_count.as_ref(),
                    self.game.data_table.card_name.as_ref(),
                )
            })
            .unwrap_or(name);
        self.game
            .chat_window
            .add_system(format!("Picked up {formatted_name} x{count}"));
        if let Some(path) = &icon_path {
            self.preload_item_icons(vec![path.clone()]);
        }
        self.game
            .item_pickup_notification
            .show(formatted_name, count, icon_path);
    }

    pub(super) fn handle_inventory_equip_result(
        &mut self,
        index: u16,
        wear_location: u16,
        view_id: u16,
        success: bool,
    ) {
        tracing::debug!(
            "EquipResult: idx={} wear_loc={} view_id={} success={}",
            index,
            wear_location,
            view_id,
            success,
        );
        if success {
            self.game
                .character
                .inventory
                .update_wear_state(index, wear_location);
            let item_type = self.game.character.inventory.get_item(index).map(|i| i.item_type);
            if view_id != 0
                && let Some(sprite_type) =
                    Entity::wear_location_to_sprite_type_for(wear_location, item_type)
                && let Some(player_id) = self.game.entities.player_id()
            {
                if let Some(entity) = self.game.entities.get_mut(player_id) {
                    entity.apply_sprite_change(sprite_type, view_id);
                }
                self.reload_player_sprite(player_id);
            }
        }
    }

    pub(super) fn handle_inventory_unequip_result(
        &mut self,
        index: u16,
        wear_location: u16,
        success: bool,
    ) {
        tracing::debug!(
            "UnequipResult: idx={} wear_loc={} success={}",
            index,
            wear_location,
            success,
        );
        if success {
            let item_type = self
                .game
                .character
                .inventory
                .get_item(index)
                .map(|i| i.item_type);
            self.game.character.inventory.clear_wear_state(index);
            if let Some(sprite_type) =
                Entity::wear_location_to_sprite_type_for(wear_location, item_type)
                && let Some(player_id) = self.game.entities.player_id()
            {
                if let Some(entity) = self.game.entities.get_mut(player_id) {
                    entity.apply_sprite_change(sprite_type, 0);
                }
                self.reload_player_sprite(player_id);
            }
        }
    }

    pub(super) fn handle_card_insert_item_list(&mut self, equip_indices: Vec<u16>) {
        let card_index = match self.game.pending_card_composition_index.take() {
            Some(idx) => idx,
            None => return,
        };
        if equip_indices.is_empty() {
            return;
        }
        let slot_count_table = self.game.data_table.item_slot_count.as_ref();
        let card_name_table = self.game.data_table.card_name.as_ref();
        let eligible: Vec<EligibleItem> = equip_indices
            .iter()
            .filter_map(|&idx| {
                let item = self.game.character.inventory.get_item(idx)?;
                let name = format_equipment_display_name(item, slot_count_table, card_name_table);
                Some(EligibleItem {
                    inventory_index: idx,
                    display_name: name,
                    icon_path: item.icon_path(),
                })
            })
            .collect();
        let card_name = self
            .game
            .character
            .inventory
            .get_item(card_index)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let mut dialog = CardInsertDialog::new();
        dialog.open(card_index, card_name, eligible);
        dialog.has_grf_textures = self.game.card_insert_dialog_has_grf_textures;
        if dialog.has_grf_textures
            && let Some(renderer) = &self.renderer {
                dialog.set_texture_sizes(&|name| renderer.texture_cache.texture_size(name));
            }
        let tex_paths = dialog.pending_texture_paths();
        self.game.card_insert_dialog = Some(dialog);
        self.preload_item_icons(tex_paths);
    }

    pub(super) fn handle_card_insert_result(
        &mut self,
        equip_index: u16,
        card_index: u16,
        result: u8,
    ) {
        self.game.card_insert_dialog = None;
        self.game.pending_card_composition_index = None;
        if result == 0 {
            let card_item_id = self
                .game
                .character
                .inventory
                .get_item(card_index)
                .map(|c| c.item_id)
                .unwrap_or(0);
            self.game
                .character
                .inventory
                .subtract_item_count(card_index, 1);
            if card_item_id != 0 {
                self.game
                    .character
                    .inventory
                    .insert_card(equip_index, card_item_id);
            }
        } else {
            self.game
                .chat_window
                .add_system("Card insertion failed.".to_string());
        }
    }
}
