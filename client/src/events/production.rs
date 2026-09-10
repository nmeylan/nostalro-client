use crate::App;
use models::enums::effect_id::EffectId;
use models::enums::skill_enums::SkillEnum;
use ragnarok_game::data_table::skill_name_table::format_skill_display_name;
use ragnarok_game::entity::EntityState;
use ragnarok_game::event::{RefineItemRow, VendorItem};
use ragnarok_game::skill::{ItemSkill, SkillTargetType, skill_icon_path};
use ragnarok_ui_component::game::item_list_selection_window::{ListContext, ListRow};
use ragnarok_ui_component::helper::colors::{CYAN, RED};

const MSI_CANT_MAKE_ITEM: u16 = 0x1a8;
const MSI_ITEM_IDENTIFY_SUCCEESS: u16 = 0x1eb;
const MSI_ITEM_IDENTIFY_FAIL: u16 = 0x1ec;
const MSI_ITEM_REFINING_SUCCEESS: u16 = 0x1f2;
const MSI_ITEM_REFINING_FAIL: u16 = 0x1f3;
const MSI_ITEM_REPAIR_SUCCEESS: u16 = 0x32d;
const MSI_ITEM_REPAIR_FAIL: u16 = 0x32e;
const MSI_ITEM_REFINE_SUCCEESS: u16 = 0x38f;
const MSI_ITEM_REFINE_FAIL: u16 = 0x390;
const MSI_ITEM_REFINE_FAIL_LEVEL: u16 = 0x391;
const MSI_ITEM_REFINE_FAIL_MATERIAL: u16 = 0x392;

const REFINE_FAILED_COLOR: [f32; 4] = [0.0, 205.0 / 255.0, 205.0 / 255.0, 1.0];
const REFINE_REFUSED_COLOR: [f32; 4] = [1.0, 200.0 / 255.0, 200.0 / 255.0, 1.0];

fn weapon_refine_notice(result: i32) -> Option<(u16, [f32; 4])> {
    match result {
        0 => Some((MSI_ITEM_REFINE_SUCCEESS, CYAN)),
        1 => Some((MSI_ITEM_REFINE_FAIL, REFINE_FAILED_COLOR)),
        2 => Some((MSI_ITEM_REFINE_FAIL_LEVEL, REFINE_REFUSED_COLOR)),
        3 => Some((MSI_ITEM_REFINE_FAIL_MATERIAL, REFINE_REFUSED_COLOR)),
        _ => None,
    }
}

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
            .map(|res| ragnarok_resources::ui::item::icon(res));
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
            skill: None,
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
            skill: None,
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
                    skill: None,
                }
            })
            .collect();
        self.windows
            .item_list_selection_window
            .open("Identify", ListContext::Identify, rows);
    }

    pub(crate) fn handle_item_identify_result(&mut self, index: i16, ok: bool) {
        if ok {
            let icon_path = self
                .game
                .character
                .inventory
                .apply_identify(index as u16, &self.game.data_table);
            if let Some(path) = icon_path {
                self.preload_item_icons(vec![path]);
            }
            self.add_msg_string_line(MSI_ITEM_IDENTIFY_SUCCEESS, &[], CYAN);
        } else {
            self.add_msg_string_line(MSI_ITEM_IDENTIFY_FAIL, &[], RED);
        }
    }

    pub(crate) fn handle_auto_cast_skill(
        &mut self,
        skill: SkillEnum,
        level: i16,
        sp_cost: i16,
        attack_range: i16,
        skill_target_type: SkillTargetType,
    ) {
        self.game.character.item_skills.insert(
            skill,
            ItemSkill {
                level,
                sp_cost,
                attack_range,
                skill_target_type,
            },
        );
        self.handle_item_use_skill(skill, level);
    }

    pub(crate) fn handle_making_arrow_list(&mut self, item_ids: Vec<u16>) {
        let converter = self.game.pending_casts.pending_list_skill == Some(SkillEnum::SaCreatecon);
        self.game.pending_casts.pending_list_skill = None;
        let rows: Vec<ListRow> = item_ids.iter().map(|&id| self.simple_row(id)).collect();
        let (title, context) = if converter {
            ("Elemental Converter", ListContext::ElementalConverter)
        } else {
            ("Make Arrow", ListContext::MakingArrow)
        };
        self.windows
            .item_list_selection_window
            .open(title, context, rows);
    }

    pub(crate) fn handle_auto_spell_list(&mut self, skills: Vec<SkillEnum>) {
        if skills.is_empty() {
            return;
        }
        let rows: Vec<ListRow> = skills
            .iter()
            .map(|&skill| {
                let name =
                    format_skill_display_name(&skill, self.game.data_table.skill_name.as_ref())
                        .to_string();
                ListRow {
                    name,
                    icon: Some(skill_icon_path(skill)),
                    index: 0,
                    item_id: 0,
                    refine: 0,
                    cards: [0; 4],
                    skill: Some(skill),
                }
            })
            .collect();
        self.windows
            .item_list_selection_window
            .open("Auto Spell", ListContext::AutoSpell, rows);
    }

    pub(crate) fn handle_weapon_refine_list(&mut self, items: Vec<RefineItemRow>) {
        if items.is_empty() {
            self.show_cant_make_item();
            return;
        }
        let rows: Vec<ListRow> = items.iter().map(|r| self.refine_row(r)).collect();
        self.windows.item_list_selection_window.open(
            "Refine Weapon",
            ListContext::WeaponRefine,
            rows,
        );
    }

    pub(crate) fn handle_weapon_refine_result(&mut self, result: i32, item_id: u16) {
        let Some((msg_id, color)) = weapon_refine_notice(result) else {
            return;
        };
        let (name, _) = self.resolve_name_icon(item_id, true);
        self.add_msg_string_line(msg_id, &[&name], color);
    }

    pub(crate) fn handle_item_refining_result(&mut self, index: u16, refine: u8, result: i16) {
        match result {
            0 => {
                self.spawn_effect_on_player(EffectId::Refineok);
                self.add_msg_string_line(MSI_ITEM_REFINING_SUCCEESS, &[], CYAN);
            }
            1 => {
                self.spawn_effect_on_player(EffectId::Refinefail);
                self.add_msg_string_line(MSI_ITEM_REFINING_FAIL, &[], CYAN);
            }
            _ => {}
        }
        self.game.character.inventory.set_refine(index, refine);
    }

    pub(crate) fn handle_repair_item_list(&mut self, target_aid: u32, items: Vec<RefineItemRow>) {
        if items.is_empty() {
            self.show_cant_make_item();
            return;
        }
        let rows: Vec<ListRow> = items.iter().map(|r| self.refine_row(r)).collect();
        self.windows.item_list_selection_window.open(
            "Repair Weapon",
            ListContext::RepairWeapon { target_aid },
            rows,
        );
    }

    pub(crate) fn handle_repair_item_result(&mut self, _index: i16, ok: bool) {
        let (msg_id, color) = if ok {
            (MSI_ITEM_REPAIR_SUCCEESS, CYAN)
        } else {
            (MSI_ITEM_REPAIR_FAIL, RED)
        };
        self.add_msg_string_line(msg_id, &[], color);
    }

    pub(crate) fn handle_makable_item_list(&mut self, item_ids: Vec<u16>) {
        if item_ids.is_empty() {
            self.show_cant_make_item();
            return;
        }
        let rows: Vec<(u16, String, Option<String>)> = item_ids
            .iter()
            .map(|&id| {
                let (name, icon) = self.resolve_name_icon(id, true);
                (id, name, icon)
            })
            .collect();
        // Producible items are not necessarily in the inventory, so their icons
        // are not preloaded — do it here or the make window renders blank icons.
        self.preload_item_icons(
            rows.iter()
                .filter_map(|(_, _, icon)| icon.clone())
                .collect(),
        );
        self.windows.make_item_window.open(rows);
    }

    pub(crate) fn handle_making_item_result(&mut self, result: i16) {
        self.spawn_effect_on_player(match result {
            0 => EffectId::Refineok,
            1 => EffectId::Refinefail,
            2 => EffectId::PharmacyOk,
            _ => EffectId::PharmacyFail,
        });
    }

    fn spawn_effect_on_player(&mut self, effect: EffectId) {
        if let Some(player_id) = self.game.world.entities.player_id() {
            self.effect_queue.spawn_on(effect, player_id);
        }
    }

    fn show_cant_make_item(&mut self) {
        let Some(message) = self
            .game
            .data_table
            .msg_string
            .as_ref()
            .and_then(|t| t.get(MSI_CANT_MAKE_ITEM))
            .map(str::to_string)
        else {
            return;
        };
        self.windows.confirm_dialog.show(&message, false, |_| {});
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
        let icon_paths: Vec<String> = rows
            .iter()
            .filter_map(|(_, _, icon)| icon.clone())
            .collect();
        self.preload_item_icons(icon_paths);
        let title = self
            .game
            .world
            .entities
            .get(aid)
            .and_then(|e| e.vending_board.clone())
            .unwrap_or_default();
        self.windows
            .vending_shop_window
            .open(aid, unique_id, title, rows);
    }

    pub(crate) fn handle_open_vending_setup(&mut self, max_items: i16) {
        self.windows
            .vending_setup_window
            .open(max_items.max(0) as usize);
    }

    pub(crate) fn handle_vending_board_shown(&mut self, aid: u32, name: String) {
        if let Some(entity) = self.game.world.entities.get_mut(aid) {
            entity.vending_board = Some(name);
            entity.state = EntityState::Sitting;
        }
    }

    pub(crate) fn handle_vending_board_hidden(&mut self, aid: u32) {
        if let Some(entity) = self.game.world.entities.get_mut(aid) {
            entity.vending_board = None;
            if entity.state == EntityState::Sitting {
                entity.state = EntityState::Standing;
            }
        }
    }

    pub(crate) fn handle_vending_own_stock(&mut self, items: Vec<VendorItem>) {
        self.windows
            .chat_window
            .add_system(format!("Your shop is open ({} items).", items.len()));
        let shop_name = self
            .game
            .pending_casts
            .pending_shop_name
            .take()
            .unwrap_or_default();

        let rows: Vec<(VendorItem, String, Option<String>)> = items
            .into_iter()
            .map(|it| {
                let (name, icon) = self.resolve_name_icon(it.item_id, it.is_identified);
                (it, name, icon)
            })
            .collect();
        self.windows.my_shop_window.open(shop_name.clone(), rows);

        self.windows.vending_setup_window.close();

        if let Some(pid) = self.game.world.entities.player_id()
            && let Some(entity) = self.game.world.entities.get_mut(pid)
        {
            entity.vending_board = Some(shop_name);
            entity.state = EntityState::Sitting;
        }
    }

    pub(crate) fn close_own_shop(&mut self) {
        self.channel
            .send_packet(ragnarok_network::build_req_closestore_packet(
                self.active_packetver,
            ));
        self.game.pending_casts.pending_shop_name = None;
        self.windows.my_shop_window.close();
        if let Some(pid) = self.game.world.entities.player_id()
            && let Some(entity) = self.game.world.entities.get_mut(pid)
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
        self.windows.chat_window.add_system(msg.to_string());
        if result == 0 && self.windows.vending_shop_window.is_open() {
            self.windows
                .vending_shop_window
                .record_sale(index, curcount);
        }
    }

    pub(crate) fn handle_vending_stock_decrement(&mut self, index: i16, count: i16) {
        self.windows.my_shop_window.record_sale(index, count);
        self.windows
            .chat_window
            .add_system("An item was sold from your shop.".to_string());
    }

    pub(crate) fn handle_vending_open_result(&mut self, result: u8) {
        if result != 0 {
            self.game.pending_casts.pending_shop_name = None;
            self.windows
                .chat_window
                .add_system("Failed to open your shop.".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_refine_notice_colors_the_refused_results_apart_from_the_attempted_ones() {
        assert_eq!(weapon_refine_notice(0), Some((0x38f, CYAN)));
        assert_eq!(weapon_refine_notice(1), Some((0x390, REFINE_FAILED_COLOR)));
        assert_eq!(weapon_refine_notice(2), Some((0x391, REFINE_REFUSED_COLOR)));
        assert_eq!(weapon_refine_notice(3), Some((0x392, REFINE_REFUSED_COLOR)));
        assert_eq!(weapon_refine_notice(4), None);
    }
}
