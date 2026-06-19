use crate::data_table::card_name_table::CardNameTable;
use crate::data_table::item_slot_count_table::ItemSlotCountTable;
use crate::item::Item;

pub fn format_equipment_display_name(
    item: &Item,
    slot_count_table: Option<&ItemSlotCountTable>,
    card_table: Option<&CardNameTable>,
) -> String {
    if !item.is_identified {
        return item.name.clone();
    }

    let slot_count = slot_count_table
        .map(|t| t.get_slot_count(item.item_id))
        .unwrap_or(0);

    let mut result = String::new();

    if item.refining_level > 0 {
        result.push_str(&format!("+{} ", item.refining_level));
    }

    let (prefix, postfix) = build_card_affixes(&item.slot, card_table);
    if !prefix.is_empty() {
        result.push_str(&prefix);
        result.push(' ');
    }

    result.push_str(&item.name);

    if !postfix.is_empty() {
        result.push(' ');
        result.push_str(&postfix);
    }

    if slot_count > 0 {
        result.push_str(&format!(" [{slot_count}]"));
    }

    result
}

fn build_card_affixes(slots: &[u16; 4], card_table: Option<&CardNameTable>) -> (String, String) {
    let Some(table) = card_table else {
        return (String::new(), String::new());
    };

    // Count occurrences preserving insertion order
    let mut card_counts: Vec<(u16, usize)> = Vec::new();
    for &card_id in slots {
        if card_id == 0 {
            continue;
        }
        if let Some(entry) = card_counts.iter_mut().find(|(id, _)| *id == card_id) {
            entry.1 += 1;
        } else {
            card_counts.push((card_id, 1));
        }
    }

    let mut prefix_parts = Vec::new();
    let mut postfix_parts = Vec::new();

    for (card_id, count) in card_counts {
        let Some(name) = table.get_card_name(card_id) else {
            continue;
        };
        let multiplier = match count {
            2 => "Double ",
            3 => "Triple ",
            4 => "Quadruple ",
            _ => "",
        };
        let full = format!("{multiplier}{name}");

        if table.is_postfix(card_id) {
            postfix_parts.push(full);
        } else {
            prefix_parts.push(full);
        }
    }

    (prefix_parts.join(" "), postfix_parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::enums::item::ItemType;
    use std::collections::{HashMap, HashSet};

    fn make_item(name: &str, refining: u8, slots: [u16; 4]) -> Item {
        Item {
            index: 1,
            item_id: 1101,
            item_type: ItemType::Weapon,
            count: 1,
            is_identified: true,
            is_damaged: false,
            refining_level: refining,
            slot: slots,
            location: 2,
            wear_state: 2,
            name: name.to_string(),
            resource_name: None,
        }
    }

    fn make_card_table(prefixes: &[(u16, &str)], postfix_ids: &[u16]) -> CardNameTable {
        let prefix_names: HashMap<u16, String> = prefixes
            .iter()
            .map(|(id, name)| (*id, name.to_string()))
            .collect();
        let postfix_ids: HashSet<u16> = postfix_ids.iter().copied().collect();
        CardNameTable::from_data(prefix_names, postfix_ids)
    }

    fn make_slot_table(item_id: u16, count: u8) -> ItemSlotCountTable {
        let mut entries = HashMap::new();
        if count > 0 {
            entries.insert(item_id, count);
        }
        ItemSlotCountTable::from_entries(entries)
    }

    #[test]
    fn plain_item_name() {
        let item = make_item("Sword", 0, [0; 4]);
        assert_eq!(format_equipment_display_name(&item, None, None), "Sword");
    }

    #[test]
    fn refining_only() {
        let item = make_item("Sword", 5, [0; 4]);
        assert_eq!(format_equipment_display_name(&item, None, None), "+5 Sword");
    }

    #[test]
    fn slot_count_only() {
        let item = make_item("Sword", 0, [0; 4]);
        let slot_table = make_slot_table(1101, 3);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), None),
            "Sword [3]"
        );
    }

    #[test]
    fn refining_and_slot_count() {
        let item = make_item("Sword", 7, [0; 4]);
        let slot_table = make_slot_table(1101, 2);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), None),
            "+7 Sword [2]"
        );
    }

    #[test]
    fn single_prefix_card() {
        let table = make_card_table(&[(4001, "Bloody")], &[]);
        let item = make_item("Katana", 5, [4001, 0, 0, 0]);
        let slot_table = make_slot_table(1101, 3);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table)),
            "+5 Bloody Katana [3]"
        );
    }

    #[test]
    fn single_postfix_card() {
        let table = make_card_table(&[(4002, "of Starlight")], &[4002]);
        let item = make_item("Katana", 0, [4002, 0, 0, 0]);
        let slot_table = make_slot_table(1101, 2);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table)),
            "Katana of Starlight [2]"
        );
    }

    #[test]
    fn double_prefix_card() {
        let table = make_card_table(&[(4001, "Bloody")], &[]);
        let item = make_item("Katana", 7, [4001, 4001, 0, 0]);
        let slot_table = make_slot_table(1101, 3);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table)),
            "+7 Double Bloody Katana [3]"
        );
    }

    #[test]
    fn mixed_prefix_and_postfix() {
        let table = make_card_table(&[(4001, "Bloody"), (4002, "of Starlight")], &[4002]);
        let item = make_item("Katana", 7, [4001, 4001, 4002, 0]);
        let slot_table = make_slot_table(1101, 3);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table)),
            "+7 Double Bloody Katana of Starlight [3]"
        );
    }

    #[test]
    fn unknown_card_id_skipped() {
        let table = make_card_table(&[(4001, "Bloody")], &[]);
        let item = make_item("Katana", 0, [9999, 4001, 0, 0]);
        let slot_table = make_slot_table(1101, 2);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table)),
            "Bloody Katana [2]"
        );
    }

    #[test]
    fn quadruple_card() {
        let table = make_card_table(&[(4001, "Bloody")], &[]);
        let item = make_item("Katana", 0, [4001, 4001, 4001, 4001]);
        let slot_table = make_slot_table(1101, 4);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table)),
            "Quadruple Bloody Katana [4]"
        );
    }

    #[test]
    fn unidentified_item_returns_plain_name() {
        let mut item = make_item("Unknown Weapon", 7, [4001, 0, 0, 0]);
        item.is_identified = false;
        let table = make_card_table(&[(4001, "Bloody")], &[]);
        let slot_table = make_slot_table(1101, 3);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table)),
            "Unknown Weapon"
        );
    }
}
