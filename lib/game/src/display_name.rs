use crate::char_name::CharNameCache;
use crate::data_table::DataTable;
use crate::data_table::card_name_table::CardNameTable;
use crate::data_table::item_slot_count_table::ItemSlotCountTable;
use crate::item::Item;

const STAR_CRUMB_PREFIX: [&str; 4] = [
    "",
    "Very Strong ",
    "Very Very Strong ",
    "Very Very Very Strong ",
];

const PRODUCER_COLOR: [f32; 4] = [0.251, 0.251, 0.878, 1.0];
const PRODUCER_PENDING_COLOR: [f32; 4] = [0.878, 0.251, 0.251, 1.0];
const MSI_NAMELESS: u16 = 581;
const NAMELESS_FALLBACK: &str = "Nameless";

fn element_postfix(element: u16) -> &'static str {
    match element {
        0 => "'s ",
        1 => "'s Ice ",
        2 => "'s Earth ",
        3 => "'s Fire ",
        4 => "'s Wind ",
        _ => "",
    }
}

fn color_code(color: [f32; 4]) -> String {
    let channel = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "^{:02X}{:02X}{:02X}",
        channel(color[0]),
        channel(color[1]),
        channel(color[2])
    )
}

/// How to render the producer name when the name is wanted in colour.
struct ProducerStyle<'a> {
    /// Colour the rest of the name returns to after the producer segment.
    base: [f32; 4],
    pending_name: &'a str,
}

pub fn format_equipment_display_name(
    item: &Item,
    slot_count_table: Option<&ItemSlotCountTable>,
    card_table: Option<&CardNameTable>,
    producers: &CharNameCache,
) -> String {
    format_display_name(item, slot_count_table, card_table, producers, None)
}

/// Same name, but with `^RRGGBB` codes around the producer segment: the original
/// game tints a known smith's name blue, and shows a red placeholder until the
/// name request comes back. Only windows use this; chat lines and pickup
/// notifications take the plain form.
pub fn format_equipment_display_name_colored(
    item: &Item,
    data: &DataTable,
    producers: &CharNameCache,
    base_color: [f32; 4],
) -> String {
    let pending_name = data
        .msg_string
        .as_ref()
        .and_then(|table| table.get(MSI_NAMELESS))
        .unwrap_or(NAMELESS_FALLBACK);
    format_display_name(
        item,
        data.item_slot_count.as_ref(),
        data.card_name.as_ref(),
        producers,
        Some(ProducerStyle {
            base: base_color,
            pending_name,
        }),
    )
}

fn format_display_name(
    item: &Item,
    slot_count_table: Option<&ItemSlotCountTable>,
    card_table: Option<&CardNameTable>,
    producers: &CharNameCache,
    producer_style: Option<ProducerStyle>,
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

    if let Some(char_id) = item.producer_char_id() {
        result.push_str(STAR_CRUMB_PREFIX[item.star_crumb_count() as usize]);
        let producer = producers.get(char_id);
        match producer_style {
            Some(style) => {
                let (name, color) = match producer {
                    Some(name) => (name, PRODUCER_COLOR),
                    None => (style.pending_name, PRODUCER_PENDING_COLOR),
                };
                result.push_str(&color_code(color));
                result.push_str(name);
                result.push_str(&color_code(style.base));
            }
            None => result.push_str(producer.unwrap_or_default()),
        }
        result.push_str(element_postfix(item.slot[1] & 0xff));
        result.push_str(&item.name);
        return result;
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
    use crate::data_table::msg_string_table::MsgStringTable;
    use crate::item::{CARD0_CREATE, CARD0_FORGE};
    use models::enums::item::ItemType;
    use std::collections::{HashMap, HashSet};

    const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

    fn producers() -> CharNameCache {
        let mut cache = CharNameCache::default();
        cache.insert(0x0004_0002, "Bob".to_string());
        cache
    }

    fn table_with_nameless() -> DataTable {
        DataTable {
            msg_string: Some(MsgStringTable::parse(
                format!("{}Nameless", "#".repeat(MSI_NAMELESS as usize)).as_bytes(),
            )),
            ..DataTable::default()
        }
    }

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
        assert_eq!(
            format_equipment_display_name(&item, None, None, &producers()),
            "Sword"
        );
    }

    #[test]
    fn refining_only() {
        let item = make_item("Sword", 5, [0; 4]);
        assert_eq!(
            format_equipment_display_name(&item, None, None, &producers()),
            "+5 Sword"
        );
    }

    #[test]
    fn slot_count_only() {
        let item = make_item("Sword", 0, [0; 4]);
        let slot_table = make_slot_table(1101, 3);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), None, &producers()),
            "Sword [3]"
        );
    }

    #[test]
    fn refining_and_slot_count() {
        let item = make_item("Sword", 7, [0; 4]);
        let slot_table = make_slot_table(1101, 2);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), None, &producers()),
            "+7 Sword [2]"
        );
    }

    #[test]
    fn single_prefix_card() {
        let table = make_card_table(&[(4001, "Bloody")], &[]);
        let item = make_item("Katana", 5, [4001, 0, 0, 0]);
        let slot_table = make_slot_table(1101, 3);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table), &producers()),
            "+5 Bloody Katana [3]"
        );
    }

    #[test]
    fn single_postfix_card() {
        let table = make_card_table(&[(4002, "of Starlight")], &[4002]);
        let item = make_item("Katana", 0, [4002, 0, 0, 0]);
        let slot_table = make_slot_table(1101, 2);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table), &producers()),
            "Katana of Starlight [2]"
        );
    }

    #[test]
    fn double_prefix_card() {
        let table = make_card_table(&[(4001, "Bloody")], &[]);
        let item = make_item("Katana", 7, [4001, 4001, 0, 0]);
        let slot_table = make_slot_table(1101, 3);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table), &producers()),
            "+7 Double Bloody Katana [3]"
        );
    }

    #[test]
    fn mixed_prefix_and_postfix() {
        let table = make_card_table(&[(4001, "Bloody"), (4002, "of Starlight")], &[4002]);
        let item = make_item("Katana", 7, [4001, 4001, 4002, 0]);
        let slot_table = make_slot_table(1101, 3);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table), &producers()),
            "+7 Double Bloody Katana of Starlight [3]"
        );
    }

    #[test]
    fn unknown_card_id_skipped() {
        let table = make_card_table(&[(4001, "Bloody")], &[]);
        let item = make_item("Katana", 0, [9999, 4001, 0, 0]);
        let slot_table = make_slot_table(1101, 2);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table), &producers()),
            "Bloody Katana [2]"
        );
    }

    #[test]
    fn quadruple_card() {
        let table = make_card_table(&[(4001, "Bloody")], &[]);
        let item = make_item("Katana", 0, [4001, 4001, 4001, 4001]);
        let slot_table = make_slot_table(1101, 4);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table), &producers()),
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
            format_equipment_display_name(&item, Some(&slot_table), Some(&table), &producers()),
            "Unknown Weapon"
        );
    }

    #[test]
    fn forged_weapon_names_its_smith_star_crumbs_and_element() {
        let table = make_card_table(&[(4001, "Bloody")], &[]);
        let slot_table = make_slot_table(1101, 3);
        let item = make_item("Katana", 7, [CARD0_FORGE, (10 << 8) | 3, 2, 4]);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), Some(&table), &producers()),
            "+7 Very Very Strong Bob's Fire Katana"
        );
    }

    #[test]
    fn forged_weapon_without_extras_keeps_bare_possessive() {
        let slot_table = make_slot_table(1101, 3);
        let item = make_item("Katana", 0, [CARD0_FORGE, 0, 2, 4]);
        assert_eq!(
            format_equipment_display_name(&item, Some(&slot_table), None, &producers()),
            "Bob's Katana"
        );
    }

    #[test]
    fn colored_name_tints_a_known_smith_and_returns_to_the_base_colour() {
        let item = make_item("Katana", 7, [CARD0_FORGE, (10 << 8) | 3, 2, 4]);
        assert_eq!(
            format_equipment_display_name_colored(
                &item,
                &table_with_nameless(),
                &producers(),
                BLACK
            ),
            "+7 Very Very Strong ^4040E0Bob^000000's Fire Katana"
        );
    }

    #[test]
    fn colored_name_falls_back_to_a_red_placeholder_until_the_smith_is_known() {
        let item = make_item("Katana", 0, [CARD0_FORGE, 0, 7, 9]);
        assert_eq!(
            format_equipment_display_name_colored(
                &item,
                &table_with_nameless(),
                &CharNameCache::default(),
                BLACK
            ),
            "^E04040Nameless^000000's Katana"
        );
    }

    #[test]
    fn created_item_names_its_maker() {
        let item = make_item("White Potion", 0, [CARD0_CREATE, 0, 2, 4]);
        assert_eq!(
            format_equipment_display_name(&item, None, None, &producers()),
            "Bob's White Potion"
        );
    }
}
