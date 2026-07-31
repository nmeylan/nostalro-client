pub struct IdBlock {
    pub module: &'static str,
    pub start: u32,
    pub end: u32,
}

const fn block(module: &'static str, start: u32, end: u32) -> IdBlock {
    IdBlock { module, start, end }
}

pub const ID_BLOCKS: &[IdBlock] = &[
    block("account/login_window.rs", 0, 99),
    block("account/server_list_window.rs", 100, 199),
    block("account/char_select_window.rs", 200, 299),
    block("game/chat_window.rs", 300, 399),
    block("game/confirm_dialog.rs", 400, 419),
    block("game/drop_quantity_dialog.rs", 420, 424),
    block("game/guild_expel_dialog.rs", 425, 429),
    block("game/skill_talkbox_dialog.rs", 430, 434),
    block("game/system_menu.rs", 500, 599),
    block("game/npc_dialog.rs", 600, 699),
    block("game/npc_shop.rs", 700, 799),
    block("game/inventory_window.rs", 800, 899),
    block("game/equipment_window.rs", 900, 999),
    block("game/item_info_window.rs", 1000, 1099),
    block("game/card_insert_dialog.rs", 1100, 1119),
    block("game/skill_tree_window.rs", 1120, 1299),
    block("game/hotkey_bar.rs", 1300, 1399),
    block("game/basic_info_window.rs", 1400, 1499),
    block("game/status_window.rs", 1500, 1599),
    block("game/minimap_window.rs", 1600, 1699),
    block("game/chat_room_create_window.rs", 1700, 1799),
    block("game/cart_window.rs", 1800, 1849),
    block("game/cart_select_window.rs", 1850, 1899),
    block("game/status_icon_bar.rs", 1900, 1999),
    block("game/party_friends_window.rs", 2000, 2099),
    block("game/item_list_selection_window.rs", 2100, 2199),
    block("game/make_item_window.rs", 2200, 2299),
    block("game/vending_setup_window.rs", 2300, 2399),
    block("game/vending_shop_window.rs", 2400, 2499),
    block("game/book_window.rs", 2500, 2599),
    block("game/my_shop_window.rs", 2600, 2699),
    block("game/map_missing_window.rs", 2700, 2709),
    block("game/levelup_notification_window.rs", 2710, 2799),
    block("game/sound_options.rs", 2800, 2899),
    block("game/homun_window.rs", 2900, 2909),
    block("game/homun_skill_window.rs", 2910, 2999),
    block("game/mercenary_window.rs", 3000, 3099),
    block("game/mercenary_skill_window.rs", 3100, 3199),
    block("game/companion_ai_config_window.rs", 3200, 3299),
    block("game/party_helper_window.rs", 3300, 3399),
    block("game/guild_window.rs", 3400, 3479),
    block("game/emblem_picker_window.rs", 3480, 3499),
    block("game/emotion_window.rs", 3500, 3599),
    block("game/shortcut_list_window.rs", 3600, 3699),
    block("game/quest_window.rs", 3700, 3799),
    block("game/storage_window.rs", 3800, 3899),
    block("game/mailbox_window.rs", 3900, 3949),
    block("game/read_mail_window.rs", 3950, 3999),
    block("game/pet_window.rs", 4000, 4099),
    block("game/trade_window.rs", 4100, 4199),
    block("game/graphic_options.rs", 4200, 4299),
    block("game/hotkey_config_window.rs", 4300, 4399),
    block("account/login_server_list_window.rs", 4400, 4499),
    block("account/char_create_window.rs", 4500, 4599),
    block("game/warp_list_window.rs", 4600, 4699),
    block("game/context_menu.rs", 4700, 4799),
    block("game/chat_room_member_window.rs", 4800, 4899),
    // Second blocks, for modules whose derived ranges outgrew their first one.
    block("game/vending_setup_window.rs", 4900, 4999),
    block("game/companion_ai_config_window.rs", 5100, 5199),
    block("game/guild_window.rs", 5200, 5799),
    block("game/world_map_window.rs", 5800, 5899),
    block("game/monster_info_window.rs", 5900, 5999),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn src_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
    }

    fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_sources(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// True for const names that hold an absolute id, as opposed to an offset
    /// added to a caller-supplied base.
    fn is_id_name(name: &str) -> bool {
        name.split('_').any(|part| part == "ID" || part == "BASE")
    }

    fn parse_u32_prefix(s: &str) -> Option<u32> {
        let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    }

    /// Every id literal declared outside `#[cfg(test)]`, as
    /// (module, name, line, value).
    fn declared_ids() -> Vec<(String, String, usize, u32)> {
        let root = src_root();
        let mut files = Vec::new();
        rust_sources(&root, &mut files);
        files.sort();

        let mut ids = Vec::new();
        for file in files {
            let module = file
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(&file).unwrap();
            for (i, line) in text.lines().enumerate() {
                if line.starts_with("#[cfg(test)]") {
                    break;
                }
                let decl = line
                    .strip_prefix("pub const ")
                    .or_else(|| line.strip_prefix("const "));
                if let Some(decl) = decl
                    && let Some((name, value)) = decl.split_once(": u32 = ")
                    && is_id_name(name)
                    && let Some(value) = parse_u32_prefix(value)
                {
                    ids.push((module.clone(), name.to_string(), i + 1, value));
                    continue;
                }
                for (idx, _) in line.match_indices("WidgetId(") {
                    let rest = &line[idx + "WidgetId(".len()..];
                    if let Some(value) = parse_u32_prefix(rest) {
                        ids.push((module.clone(), line.trim().to_string(), i + 1, value));
                    }
                }
            }
        }
        ids
    }

    #[test]
    fn id_blocks_are_disjoint() {
        let mut sorted: Vec<&IdBlock> = ID_BLOCKS.iter().collect();
        sorted.sort_by_key(|b| b.start);
        for b in &sorted {
            assert!(b.start <= b.end, "{} has an inverted block", b.module);
        }
        for pair in sorted.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(
                a.end < b.start,
                "blocks overlap: {} [{}..={}] and {} [{}..={}]",
                a.module,
                a.start,
                a.end,
                b.module,
                b.start,
                b.end,
            );
        }
    }

    #[test]
    fn declared_ids_stay_in_their_module_block() {
        for (module, name, line, value) in declared_ids() {
            let owner = ID_BLOCKS
                .iter()
                .find(|b| value >= b.start && value <= b.end);
            match owner {
                Some(block) => assert_eq!(
                    block.module, module,
                    "{module}:{line}: {name} = {value} falls in the block owned by {}",
                    block.module,
                ),
                None => panic!(
                    "{module}:{line}: {name} = {value} is outside every declared block — \
                     claim a free block in ID_BLOCKS"
                ),
            }
        }
    }

    /// A module listed here but no longer declaring ids means the block is free
    /// and should be released rather than left to fence off the range.
    #[test]
    fn every_block_is_used() {
        let ids = declared_ids();
        for b in ID_BLOCKS {
            assert!(
                ids.iter()
                    .any(|(module, _, _, v)| module == b.module && *v >= b.start && *v <= b.end),
                "{} declares no id in [{}..={}]",
                b.module,
                b.start,
                b.end,
            );
        }
    }
}
