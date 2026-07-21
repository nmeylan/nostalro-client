use crate::App;
use models::enums::skill_enums::SkillEnum;
use ragnarok_game::entity::EntityState;
use ragnarok_game::event::{RefineItemRow, VendorItem};
use ragnarok_ui_component::game::item_list_selection_window::{ListContext, ListRow};

impl App {
    fn resolve_name_icon(&self, item_id: u16, is_identified: bool) -> (String, Option<String>) {
        let name = self
            .game
            .data_table
            .item_name
            .as_ref()
            .map(|t| t.get_name_or_id_for(item_id, is_identified))
            .unwrap_or_else(|| format!("Item #{item_id}"));
        let icon = self
            .game
            .data_table
            .item_resource
            .as_ref()
            .and_then(|t| t.get_resource_name_for(item_id, is_identified))
            .map(|res| format!("data/texture/유저인터페이스/item/{res}.bmp"));
        (name, icon)
    }

    fn simple_row(&self, item_id: u16) -> ListRow {
        let (name, icon) = self.resolve_name_icon(item_id, true);
        ListRow {
            name,
            icon,
            index: 0,
            item_id,
            refine: 0,
            cards: [0; 4],
            skill_id: 0,
        }
    }

    fn refine_row(&self, r: &RefineItemRow) -> ListRow {
        let (base, icon) = self.resolve_name_icon(r.item_id, true);
        let name = if r.refine > 0 {
            format!("+{} {base}", r.refine)
        } else {
            base
        };
        ListRow {
            name,
            icon,
            index: r.index,
            item_id: r.item_id,
            refine: r.refine,
            cards: r.cards,
            skill_id: 0,
        }
    }

    pub(crate) fn handle_item_identify_list(&mut self, indices: Vec<u16>) {
        let rows: Vec<ListRow> = indices
            .iter()
            .map(|&idx| {
                let item = self.game.character.inventory.get_item(idx);
                let (name, icon) = match item {
                    Some(it) => (it.name.clone(), it.icon_path()),
                    None => self.resolve_name_icon(0, false),
                };
                ListRow {
                    name,
                    icon,
                    index: idx as i16,
                    item_id: item.map(|it| it.item_id).unwrap_or(0),
                    refine: 0,
                    cards: [0; 4],
                    skill_id: 0,
                }
            })
            .collect();
        self.game
            .item_list_selection_window
            .open("Identify", ListContext::Identify, rows);
    }

    pub(crate) fn handle_item_identify_result(&mut self, index: i16, ok: bool) {
        let msg = if ok {
            let icon_path = self
                .game
                .character
                .inventory
                .apply_identify(index as u16, &self.game.data_table);
            if let Some(path) = icon_path {
                self.preload_item_icons(vec![path]);
            }
            "Item appraised.".to_string()
        } else {
            "Appraisal failed.".to_string()
        };
        self.game.chat_window.add_system(msg);
    }

    pub(crate) fn handle_auto_cast_skill(&mut self, skill_id: u16, level: i16) {
        let target_id = self.game.entities.player_id().unwrap_or(0);
        self.channel.send_packet(ragnarok_network::build_use_skill_packet(
            skill_id,
            level,
            target_id,
            self.config.packetver,
        ));
    }

    pub(crate) fn handle_making_arrow_list(&mut self, item_ids: Vec<u16>) {
        let converter = self.game.pending_casts.pending_list_skill
            == Some(SkillEnum::SaCreatecon.id() as u16);
        self.game.pending_casts.pending_list_skill = None;
        let rows: Vec<ListRow> = item_ids.iter().map(|&id| self.simple_row(id)).collect();
        let (title, context) = if converter {
            ("Elemental Converter", ListContext::ElementalConverter)
        } else {
            ("Make Arrow", ListContext::MakingArrow)
        };
        self.game
            .item_list_selection_window
            .open(title, context, rows);
    }

    pub(crate) fn handle_auto_spell_list(&mut self, skill_ids: Vec<i32>) {
        let rows: Vec<ListRow> = skill_ids
            .iter()
            .map(|&id| {
                let skill = self.game.character.skills.get_skill(id as u16);
                let (name, icon) = match skill {
                    Some(s) => {
                        let display = self
                            .game
                            .data_table
                            .skill_name
                            .as_ref()
                            .map(|t| t.get_display_name_or_internal(&s.name))
                            .unwrap_or_else(|| s.name.clone());
                        (display, Some(s.icon_path()))
                    }
                    None => (format!("Skill #{id}"), None),
                };
                ListRow {
                    name,
                    icon,
                    index: 0,
                    item_id: 0,
                    refine: 0,
                    cards: [0; 4],
                    skill_id: id,
                }
            })
            .collect();
        self.game
            .item_list_selection_window
            .open("Auto Spell", ListContext::AutoSpell, rows);
    }

    pub(crate) fn handle_weapon_refine_list(&mut self, items: Vec<RefineItemRow>) {
        let rows: Vec<ListRow> = items.iter().map(|r| self.refine_row(r)).collect();
        self.game
            .item_list_selection_window
            .open("Refine Weapon", ListContext::WeaponRefine, rows);
    }

    pub(crate) fn handle_weapon_refine_result(&mut self, result: i32, item_id: u16) {
        let (name, _) = self.resolve_name_icon(item_id, true);
        let msg = match result {
            0 | 1 => format!("{name} was successfully refined."),
            2 => "You need a higher skill level to refine this.".to_string(),
            _ => format!("Failed to refine {name}."),
        };
        self.game.chat_window.add_system(msg);
    }

    pub(crate) fn handle_repair_item_list(&mut self, target_aid: u32, items: Vec<RefineItemRow>) {
        let rows: Vec<ListRow> = items.iter().map(|r| self.refine_row(r)).collect();
        self.game.item_list_selection_window.open(
            "Repair Weapon",
            ListContext::RepairWeapon { target_aid },
            rows,
        );
    }

    pub(crate) fn handle_repair_item_result(&mut self, _index: i16, ok: bool) {
        let msg = if ok {
            "The weapon was repaired.".to_string()
        } else {
            "Repair failed.".to_string()
        };
        self.game.chat_window.add_system(msg);
    }

    pub(crate) fn handle_makable_item_list(&mut self, item_ids: Vec<u16>) {
        let rows: Vec<(u16, String, Option<String>)> = item_ids
            .iter()
            .map(|&id| {
                let (name, icon) = self.resolve_name_icon(id, true);
                (id, name, icon)
            })
            .collect();
        // Producible items are not necessarily in the inventory, so their icons
        // are not preloaded — do it here or the make window renders blank icons.
        self.preload_item_icons(rows.iter().filter_map(|(_, _, icon)| icon.clone()).collect());
        self.game.make_item_window.open(rows);
    }

    pub(crate) fn handle_making_item_result(&mut self, result: i16, item_id: u16) {
        let (name, _) = self.resolve_name_icon(item_id, true);
        let msg = match result {
            0 | 2 => format!("Successfully created {name}."),
            _ => format!("Failed to create {name}."),
        };
        self.game.chat_window.add_system(msg);
    }

    pub(crate) fn handle_vending_shop_list(
        &mut self,
        aid: u32,
        unique_id: u32,
        items: Vec<VendorItem>,
    ) {
        let rows: Vec<(VendorItem, String, Option<String>)> = items
            .into_iter()
            .map(|it| {
                let (name, icon) = self.resolve_name_icon(it.item_id, it.is_identified);
                (it, name, icon)
            })
            .collect();
        let icon_paths: Vec<String> = rows.iter().filter_map(|(_, _, icon)| icon.clone()).collect();
        self.preload_item_icons(icon_paths);
        let title = self
            .game
            .entities
            .get(aid)
            .and_then(|e| e.vending_board.clone())
            .unwrap_or_default();
        self.game
            .vending_shop_window
            .open(aid, unique_id, title, rows);
    }

    pub(crate) fn handle_open_vending_setup(&mut self, max_items: i16) {
        self.game
            .vending_setup_window
            .open(max_items.max(0) as usize);
    }

    pub(crate) fn handle_vending_board_shown(&mut self, aid: u32, name: String) {
        if let Some(entity) = self.game.entities.get_mut(aid) {
            entity.vending_board = Some(name);
            entity.state = EntityState::Sitting;
        }
    }

    pub(crate) fn handle_vending_board_hidden(&mut self, aid: u32) {
        if let Some(entity) = self.game.entities.get_mut(aid) {
            entity.vending_board = None;
            if entity.state == EntityState::Sitting {
                entity.state = EntityState::Standing;
            }
        }
    }

    pub(crate) fn handle_vending_own_stock(&mut self, items: Vec<VendorItem>) {
        self.game
            .chat_window
            .add_system(format!("Your shop is open ({} items).", items.len()));
        let shop_name = self.game.pending_casts.pending_shop_name.take().unwrap_or_default();

        let rows: Vec<(VendorItem, String, Option<String>)> = items
            .into_iter()
            .map(|it| {
                let (name, icon) = self.resolve_name_icon(it.item_id, it.is_identified);
                (it, name, icon)
            })
            .collect();
        self.game.my_shop_window.open(shop_name.clone(), rows);

        self.game.vending_setup_window.close();

        if let Some(pid) = self.game.entities.player_id()
            && let Some(entity) = self.game.entities.get_mut(pid)
        {
            entity.vending_board = Some(shop_name);
            entity.state = EntityState::Sitting;
        }
    }

    pub(crate) fn close_own_shop(&mut self) {
        self.channel.send_packet(ragnarok_network::build_req_closestore_packet(
            self.config.packetver,
        ));
        self.game.pending_casts.pending_shop_name = None;
        self.game.my_shop_window.close();
        if let Some(pid) = self.game.entities.player_id()
            && let Some(entity) = self.game.entities.get_mut(pid)
        {
            entity.vending_board = None;
            if entity.state == EntityState::Sitting {
                entity.state = EntityState::Standing;
            }
        }
    }

    pub(crate) fn handle_vending_purchase_result(&mut self, index: i16, curcount: i16, result: u8) {
        let msg = match result {
            0 => "Purchase complete.",
            1 => "Not enough zeny.",
            2 => "You are overweight.",
            4 => "The item is out of stock.",
            _ => "Purchase failed.",
        };
        self.game.chat_window.add_system(msg.to_string());
        if result == 0 && self.game.vending_shop_window.is_open() {
            self.game.vending_shop_window.record_sale(index, curcount);
        }
    }

    pub(crate) fn handle_vending_stock_decrement(&mut self, index: i16, count: i16) {
        self.game.my_shop_window.record_sale(index, count);
        self.game
            .chat_window
            .add_system("An item was sold from your shop.".to_string());
    }

    pub(crate) fn handle_vending_open_result(&mut self, result: u8) {
        if result != 0 {
            self.game.pending_casts.pending_shop_name = None;
            self.game
                .chat_window
                .add_system("Failed to open your shop.".to_string());
        }
    }
}
