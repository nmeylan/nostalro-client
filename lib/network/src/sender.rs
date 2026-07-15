use crate::session::Session;
use packets::packets::*;

pub fn build_login_packet(username: &str, password: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCaLogin::new(packetver);
    pkt.set_version(20);

    let mut id = [0 as char; 24];
    for (i, c) in username.chars().take(23).enumerate() {
        id[i] = c;
    }
    pkt.set_id(id);

    let mut passwd = [0 as char; 24];
    for (i, c) in password.chars().take(23).enumerate() {
        passwd[i] = c;
    }
    pkt.set_passwd(passwd);
    pkt.set_client_type(0);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_char_enter_packet(session: &Session) -> Vec<u8> {
    let mut pkt = PacketChEnter::new(session.packetver);
    pkt.set_aid(session.account_id);
    pkt.set_auth_code(session.login_id1);
    pkt.set_user_level(session.login_id2);
    pkt.set_client_type(0);
    pkt.set_sex(session.sex);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_select_char_packet(slot: u8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketChSelectChar::new(packetver);
    pkt.set_char_num(slot);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_make_char_packet(
    name: &str,
    slot: u8,
    hair_style: u16,
    hair_color: u16,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketChMakeChar2::new(packetver);
    pkt.set_name(name_to_char24(name));
    pkt.set_char_num(slot);
    pkt.set_head_pal(hair_color as i16);
    pkt.set_head(hair_style as i16);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_make_char_with_stats_packet(
    name: &str,
    stats: [u8; 6],
    slot: u8,
    hair_style: u16,
    hair_color: u16,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketChMakeChar::new(packetver);
    pkt.set_name(name_to_char24(name));
    pkt.set_str(stats[0]);
    pkt.set_agi(stats[1]);
    pkt.set_vit(stats[2]);
    pkt.set_int(stats[3]);
    pkt.set_dex(stats[4]);
    pkt.set_luk(stats[5]);
    pkt.set_char_num(slot);
    pkt.set_head_pal(hair_color as i16);
    pkt.set_head(hair_style as i16);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_delete_char_reserve_packet(gid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketChDeleteChar3Reserved::new(packetver);
    pkt.set_gid(gid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_delete_char_confirm_packet(gid: u32, birthdate: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketChDeleteChar3::new(packetver);
    pkt.set_gid(gid);
    pkt.set_birth(birthdate_to_char6(birthdate));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_delete_char_cancel_packet(gid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketChDeleteChar3Cancel::new(packetver);
    pkt.set_gid(gid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_request_move_packet(dest_x: u16, dest_y: u16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRequestMove::new(packetver);
    pkt.set_dest(crate::helpers::encode_pos(dest_x, dest_y, 0));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_map_loaded_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzNotifyActorinit::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_chat_packet(msg: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzPlayerChat::new(packetver);
    let msg_null = format!("{msg}\0");
    pkt.set_packet_length((4 + msg_null.len()) as i16);
    pkt.set_msg(msg_null);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_action_request_packet(target_gid: u32, action: u8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRequestAct::new(packetver);
    pkt.set_target_gid(target_gid);
    pkt.set_action(action);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_change_direction_packet(head_dir: u8, dir: u8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzChangeDirection::new(packetver);
    if packetver >= 20120307 {
        // Generated new() emits header 0x9008; rathena expects 0x0890 at this packetver.
        pkt.set_packet_id(0x0890);
    }
    pkt.set_head_dir(head_dir as i16);
    pkt.set_dir(dir);
    pkt.fill_raw();
    pkt.raw
}

#[cfg(test)]
mod tests {
    use super::*;

    // rathena @20120307: parseable_packet(0x0890, 5, clif_parse_ChangeDir, 2, 4)
    #[test]
    fn change_direction_wire_layout() {
        let raw = build_change_direction_packet(2, 5, 20120307);
        assert_eq!(raw.len(), 5);
        assert_eq!(u16::from_le_bytes([raw[0], raw[1]]), 0x0890);
        assert_eq!(i16::from_le_bytes([raw[2], raw[3]]), 2);
        assert_eq!(raw[4], 5);
    }

    #[test]
    fn restart_packet_types() {
        assert_eq!(*build_return_savepoint_packet(20120307).last().unwrap(), 0);
        assert_eq!(*build_restart_packet(20120307).last().unwrap(), 1);
    }

    #[test]
    fn standing_resurrection_builds() {
        assert!(!build_standing_resurrection_packet(20120307).is_empty());
    }
}

pub fn build_zone_enter_packet(session: &Session) -> Vec<u8> {
    let mut pkt = PacketCzEnter2::new(session.packetver);
    pkt.set_aid(session.account_id);
    pkt.set_gid(session.char_id);
    pkt.set_auth_code(session.login_id1);
    pkt.set_client_time(0);
    pkt.set_sex(session.sex);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_restart_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRestart::new(packetver);
    pkt.set_atype(1);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_return_savepoint_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRestart::new(packetver);
    pkt.set_atype(0);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_standing_resurrection_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzStandingResurrection::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_disconnect_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqDisconnect::new(packetver);
    pkt.set_atype(0);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_request_time_packet(client_time: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRequestTime::new(packetver);
    pkt.set_client_time(client_time);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_reqname_packet(entity_id: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqname::new(packetver);
    pkt.set_aid(entity_id);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_char_ping_packet(account_id: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzPing::new(packetver);
    pkt.set_aid(account_id);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_contact_npc_packet(npc_id: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzContactnpc::new(packetver);
    pkt.set_naid(npc_id);
    pkt.set_atype(1);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_npc_next_packet(npc_id: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqNextScript::new(packetver);
    pkt.set_naid(npc_id);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_npc_close_packet(npc_id: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzCloseDialog::new(packetver);
    pkt.set_naid(npc_id);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_npc_menu_select_packet(npc_id: u32, choice: u8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzChooseMenu::new(packetver);
    pkt.set_naid(npc_id);
    pkt.set_num(choice);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_npc_input_number_packet(npc_id: u32, value: i32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzInputEditdlg::new(packetver);
    pkt.set_naid(npc_id);
    pkt.set_value(value);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_npc_input_string_packet(npc_id: u32, text: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzInputEditdlgstr::new(packetver);
    let msg = format!("{text}\0");
    pkt.set_packet_length((8 + msg.len()) as i16);
    pkt.set_naid(npc_id);
    pkt.set_msg(msg);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_npc_deal_type_packet(npc_id: u32, deal_type: u8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzAckSelectDealtype::new(packetver);
    pkt.set_naid(npc_id);
    pkt.set_atype(deal_type);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_enter_room_packet(room_id: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqEnterRoom::new(packetver);
    pkt.set_room_id(room_id);
    pkt.set_passwd(['\0'; 8]);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_purchase_item_list_packet(items: &[(i16, u16)], packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzPcPurchaseItemlist::new(packetver);
    let item_list: Vec<CzPurchaseItem> = items
        .iter()
        .map(|(count, item_id)| {
            let mut item = CzPurchaseItem::new(packetver);
            item.set_count(*count);
            item.set_itid(*item_id);
            item.fill_raw();
            item
        })
        .collect();
    pkt.set_packet_length((4 + items.len() * 4) as i16);
    pkt.set_item_list(item_list);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_sell_item_list_packet(items: &[(i16, i16)], packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzPcSellItemlist::new(packetver);
    let item_list: Vec<CzSellItem> = items
        .iter()
        .map(|(index, count)| {
            let mut item = CzSellItem::new(packetver);
            item.set_index(*index);
            item.set_count(*count);
            item.fill_raw();
            item
        })
        .collect();
    pkt.set_packet_length((4 + items.len() * 4) as i16);
    pkt.set_item_list(item_list);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_use_item_packet(index: u16, account_id: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzUseItem::new(packetver);
    pkt.set_index(index);
    pkt.set_aid(account_id);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_equip_item_packet(index: u16, location: u16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqWearEquip::new(packetver);
    pkt.set_index(index);
    pkt.set_wear_location(location);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_unequip_item_packet(index: u16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqTakeoffEquip::new(packetver);
    pkt.set_index(index);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_drop_item_packet(index: u16, count: i16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzItemThrow::new(packetver);
    pkt.set_index(index);
    pkt.set_count(count);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_pickup_item_packet(itaid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzItemPickup::new(packetver);
    pkt.set_itaid(itaid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_move_item_body_to_cart_packet(index: u16, count: i16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMoveItemFromBodyToCart::new(packetver);
    pkt.set_index(index as i16);
    pkt.set_count(count as i32);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_move_item_cart_to_body_packet(index: u16, count: i16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMoveItemFromCartToBody::new(packetver);
    pkt.set_index(index as i16);
    pkt.set_count(count as i32);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_move_item_store_to_cart_packet(index: u16, count: i16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMoveItemFromStoreToCart::new(packetver);
    pkt.set_index(index as i16);
    pkt.set_count(count as i32);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_move_item_cart_to_store_packet(index: u16, count: i16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMoveItemFromCartToStore::new(packetver);
    pkt.set_index(index as i16);
    pkt.set_count(count as i32);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_cartoff_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqCartoff::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_change_cart_packet(num: i16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqChangecart::new(packetver);
    pkt.set_num(num);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_upgrade_skill_packet(skill_id: u16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzUpgradeSkilllevel::new(packetver);
    pkt.set_skid(skill_id);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_stat_change_packet(status_id: u16, amount: u8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzStatusChange::new(packetver);
    pkt.set_status_id(status_id);
    pkt.set_change_amount(amount);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_shortcut_key_change_packet(
    index: u16,
    is_skill: i8,
    id: u32,
    count: i16,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzShortcutKeyChange::new(packetver);
    pkt.set_index(index);
    let mut key = ShortCutKey::new(packetver);
    key.set_is_skill(is_skill);
    key.set_id(id);
    key.set_count(count);
    key.fill_raw();
    pkt.set_short_cut_key(key);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_card_composition_list_packet(card_index: u16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqItemcompositionList::new(packetver);
    pkt.set_card_index(card_index as i16);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_card_composition_packet(card_index: u16, equip_index: u16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqItemcomposition::new(packetver);
    pkt.set_card_index(card_index as i16);
    pkt.set_equip_index(equip_index as i16);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_use_skill_packet(
    skill_id: u16,
    level: i16,
    target_id: u32,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzUseSkill::new(packetver);
    pkt.set_selected_level(level);
    pkt.set_skid(skill_id);
    pkt.set_target_id(target_id);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_use_skill_to_ground_packet(
    skill_id: u16,
    level: i16,
    x: i16,
    y: i16,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzUseSkillToground::new(packetver);
    pkt.set_selected_level(level);
    pkt.set_skid(skill_id);
    pkt.set_x_pos(x);
    pkt.set_y_pos(y);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_select_warppoint_packet(skill_id: u16, map_name: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzSelectWarppoint::new(packetver);
    pkt.set_skid(skill_id);
    let mut bytes = [0u8; 16];
    let src = map_name.as_bytes();
    let n = src.len().min(15);
    bytes[..n].copy_from_slice(&src[..n]);
    pkt.set_map_name(std::array::from_fn(|i| bytes[i] as char));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_remove_option_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqCartoff::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

fn name_to_char24(name: &str) -> [char; 24] {
    let mut buf = [0 as char; 24];
    for (i, c) in name.chars().take(23).enumerate() {
        buf[i] = c;
    }
    buf
}

/// Birthdate as the 6 raw digits (YYMMDD) the server compares against; non-digits
/// are dropped and the century is trimmed, so "2001-05-14", "20010514" and
/// "010514" all send "010514".
fn birthdate_to_char6(birthdate: &str) -> [char; 6] {
    let digits: Vec<char> = birthdate.chars().filter(|c| c.is_ascii_digit()).collect();
    let start = digits.len().saturating_sub(6);
    let mut buf = [0 as char; 6];
    for (i, c) in digits[start..].iter().enumerate() {
        buf[i] = *c;
    }
    buf
}

pub fn build_make_party_packet(name: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMakeGroup::new(packetver);
    pkt.set_group_name(name_to_char24(name));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_join_party_packet(target_aid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqJoinGroup::new(packetver);
    pkt.set_aid(target_aid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_join_party_reply_packet(party_grid: u32, accept: bool, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzJoinGroup::new(packetver);
    pkt.set_grid(party_grid);
    pkt.set_answer(if accept { 1 } else { 0 });
    pkt.fill_raw();
    pkt.raw
}

pub fn build_leave_party_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqLeaveGroup::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_expel_party_member_packet(aid: u32, name: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqExpelGroupMember::new(packetver);
    pkt.set_aid(aid);
    pkt.set_character_name(name_to_char24(name));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_change_party_exp_option_packet(exp_option: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzChangeGroupexpoption::new(packetver);
    pkt.set_exp_option(exp_option);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_make_party2_packet(
    name: &str,
    item_pickup_rule: u8,
    item_division_rule: u8,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzMakeGroup2::new(packetver);
    pkt.set_group_name(name_to_char24(name));
    pkt.set_item_pickup_rule(item_pickup_rule);
    pkt.set_item_division_rule(item_division_rule);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_party_invite_by_name_packet(name: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzPartyJoinReq::new(packetver);
    pkt.set_character_name(name_to_char24(name));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_change_party_leader_packet(aid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzChangeGroupMaster::new(packetver);
    pkt.set_aid(aid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_add_friend_packet(name: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzAddFriends::new(packetver);
    pkt.set_name(name_to_char24(name));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_ack_add_friend_packet(
    req_aid: u32,
    req_gid: u32,
    accept: bool,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzAckReqAddFriends::new(packetver);
    pkt.set_req_aid(req_aid);
    pkt.set_req_gid(req_gid);
    pkt.set_result(if accept { 1 } else { 0 });
    pkt.fill_raw();
    pkt.raw
}

pub fn build_delete_friend_packet(aid: u32, gid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzDeleteFriends::new(packetver);
    pkt.set_aid(aid);
    pkt.set_gid(gid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_party_chat_packet(msg: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRequestChatParty::new(packetver);
    let msg_null = format!("{msg}\0");
    pkt.set_packet_length((4 + msg_null.len()) as i16);
    pkt.set_msg(msg_null);
    pkt.fill_raw();
    pkt.raw
}

// --- Skill-triggered production / selection windows ---

pub fn build_req_itemidentify_packet(index: i16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqItemidentify::new(packetver);
    pkt.set_index(index);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_makingarrow_packet(item_id: u16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqMakingarrow::new(packetver);
    pkt.set_id(item_id);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_makingitem_packet(item_id: u16, materials: [u16; 3], packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqmakingitem::new(packetver);
    let mut info = MakableitemInfo::new(packetver);
    info.set_itid(item_id);
    // The generated `fill_raw` serializes `material_id_raw` (not `material_id`),
    // so pack the three material words into the raw bytes directly.
    let mut mat_raw = [0u8; 6];
    mat_raw[0..2].copy_from_slice(&materials[0].to_le_bytes());
    mat_raw[2..4].copy_from_slice(&materials[1].to_le_bytes());
    mat_raw[4..6].copy_from_slice(&materials[2].to_le_bytes());
    info.set_material_id(materials);
    info.set_material_id_raw(mat_raw);
    info.fill_raw();
    pkt.set_info(info);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_weaponrefine_packet(index: i32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqWeaponrefine::new(packetver);
    pkt.set_index(index);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_itemrepair_packet(
    index: i16,
    item_id: u16,
    refine: u8,
    cards: [u16; 4],
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzReqItemrepair::new(packetver);
    let mut info = RepairitemInfo::new(packetver);
    info.set_index(index);
    info.set_itid(item_id);
    info.set_refining_level(refine);
    let mut slot = EQUIPSLOTINFO::new(packetver);
    slot.set_card1(cards[0]);
    slot.set_card2(cards[1]);
    slot.set_card3(cards[2]);
    slot.set_card4(cards[3]);
    slot.fill_raw();
    info.set_slot(slot);
    info.fill_raw();
    pkt.set_target_item_info(info);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_select_autospell_packet(skill_id: i32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzSelectautospell::new(packetver);
    pkt.set_skid(skill_id);
    pkt.fill_raw();
    pkt.raw
}

// --- Vending ---

pub fn build_req_openstore2_packet(
    shop_name: &str,
    items: &[(i16, i16, i32)],
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzReqOpenstore2::new(packetver);
    let mut name = ['\0'; 80];
    for (i, c) in shop_name.chars().take(79).enumerate() {
        name[i] = c;
    }
    pkt.set_store_name(name);
    pkt.set_result(true);
    let store_list: Vec<StoreItem> = items
        .iter()
        .map(|(index, count, price)| {
            let mut s = StoreItem::new(packetver);
            s.set_index(*index);
            s.set_count(*count);
            s.set_price(*price);
            s.fill_raw();
            s
        })
        .collect();
    // header(2) + len(2) + name(80) + result(1) + N*8
    pkt.set_packet_length((85 + items.len() * 8) as i16);
    pkt.set_store_list(store_list);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_cancel_openstore_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqOpenstore2::new(packetver);
    pkt.set_store_name(['\0'; 80]);
    pkt.set_result(false);
    pkt.set_packet_length(85);
    pkt.set_store_list(Vec::new());
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_closestore_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqClosestore::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_buy_frommc_packet(aid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqBuyFrommc::new(packetver);
    pkt.set_aid(aid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_purchase_frommc2_packet(
    aid: u32,
    unique_id: u32,
    items: &[(i16, i16)],
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzPcPurchaseItemlistFrommc2::new(packetver);
    pkt.set_aid(aid);
    pkt.set_unique_id(unique_id);
    let item_list: Vec<CzPurchaseItemFrommc> = items
        .iter()
        .map(|(count, index)| {
            let mut item = CzPurchaseItemFrommc::new(packetver);
            item.set_count(*count);
            item.set_index(*index);
            item.fill_raw();
            item
        })
        .collect();
    // header(2) + len(2) + aid(4) + uniqueId(4) + N*4
    pkt.set_packet_length((12 + items.len() * 4) as i16);
    pkt.set_item_list(item_list);
    pkt.fill_raw();
    pkt.raw
}

// --- Homunculus / Mercenary ---

pub fn build_companion_move_packet(gid: u32, x: u16, y: u16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRequestMovenpc::new(packetver);
    pkt.set_gid(gid);
    pkt.set_dest(crate::helpers::encode_pos(x, y, 0));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_companion_attack_packet(gid: u32, target_gid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRequestActnpc::new(packetver);
    pkt.set_gid(gid);
    pkt.set_target_gid(target_gid);
    pkt.set_action(0);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_companion_move_to_owner_packet(gid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRequestMovetoowner::new(packetver);
    pkt.set_gid(gid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_config_packet(config: i32, value: i32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzConfig::new(packetver);
    pkt.set_config(config);
    pkt.set_value(value);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_homun_menu_packet(command: i8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzCommandMer::new(packetver);
    pkt.set_atype(0);
    pkt.set_command(command);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_mercenary_command_packet(command: i8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMerCommand::new(packetver);
    pkt.set_command(command);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_rename_homun_packet(name: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRenameMer::new(packetver);
    pkt.set_name(name_to_char24(name));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_guild_menuinterface(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqGuildMenuinterface::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_guild_menu(atype: i32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqGuildMenu::new(packetver);
    pkt.set_atype(atype);
    pkt.fill_raw();
    pkt.raw
}

fn to_char_array<const N: usize>(s: &str) -> [char; N] {
    let mut buf = [0 as char; N];
    for (i, c) in s.chars().take(N - 1).enumerate() {
        buf[i] = c;
    }
    buf
}

pub fn build_guild_notice(gdid: u32, subject: &str, notice: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzGuildNotice::new(packetver);
    pkt.set_gdid(gdid);
    pkt.set_subject(to_char_array::<60>(subject));
    pkt.set_notice(to_char_array::<120>(notice));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_leave_guild(
    gdid: u32,
    aid: i32,
    gid: i32,
    reason: &str,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzReqLeaveGuild::new(packetver);
    pkt.set_gdid(gdid);
    pkt.set_aid(aid);
    pkt.set_gid(gid);
    pkt.set_reason_desc(to_char_array::<40>(reason));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_ban_guild(
    gdid: u32,
    aid: i32,
    gid: i32,
    reason: &str,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzReqBanGuild::new(packetver);
    pkt.set_gdid(gdid);
    pkt.set_aid(aid);
    pkt.set_gid(gid);
    pkt.set_reason_desc(to_char_array::<40>(reason));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_change_memberpos(
    aid: i32,
    gid: i32,
    position_id: i32,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzReqChangeMemberpos::new(packetver);
    let mut row = MemberPositionInfo::new(packetver);
    row.set_aid(aid);
    row.set_gid(gid);
    row.set_position_id(position_id);
    row.fill_raw();
    pkt.set_packet_length((4 + 12) as i16);
    pkt.set_member_info(vec![row]);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_reg_change_guild_positioninfo(
    rows: &[ragnarok_game::guild::GuildPosition],
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzRegChangeGuildPositioninfo::new(packetver);
    let list: Vec<GuildRegPositionInfo> = rows
        .iter()
        .map(|p| {
            let mut r = GuildRegPositionInfo::new(packetver);
            r.set_position_id(p.id);
            r.set_right(p.right);
            r.set_ranking(p.ranking);
            r.set_pay_rate(p.pay_rate);
            r.set_pos_name(to_char_array::<24>(&p.name));
            r.fill_raw();
            r
        })
        .collect();
    pkt.set_packet_length((4 + rows.len() * 40) as i16);
    pkt.set_member_list(list);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_make_guild(gid: u32, name: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqMakeGuild::new(packetver);
    pkt.set_gid(gid);
    pkt.set_gname(name_to_char24(name));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_disorganize_guild(key: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqDisorganizeGuild::new(packetver);
    pkt.set_key(to_char_array::<40>(key));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_guild_emblem_img(gdid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqGuildEmblemImg::new(packetver);
    pkt.set_gdid(gdid as i32);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_register_guild_emblem(bmp: Vec<u8>, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRegisterGuildEmblemImg::new(packetver);
    pkt.set_packet_length((4 + bmp.len()) as i16);
    // fill_raw rebuilds img_raw from the (empty) img string, so append the body after.
    pkt.fill_raw();
    pkt.raw.extend_from_slice(&bmp);
    pkt.raw
}

pub fn build_req_join_guild(target_aid: u32, my_aid: u32, my_gid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqJoinGuild::new(packetver);
    pkt.set_aid(target_aid);
    pkt.set_my_aid(my_aid);
    pkt.set_my_gid(my_gid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_ans_join_guild(gdid: u32, accept: bool, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzJoinGuild::new(packetver);
    pkt.set_gdid(gdid);
    pkt.set_answer(if accept { 1 } else { 0 });
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_ally_guild(target_aid: u32, my_aid: u32, my_gid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqAllyGuild::new(packetver);
    pkt.set_aid(target_aid);
    pkt.set_my_aid(my_aid);
    pkt.set_my_gid(my_gid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_ally_guild(other_aid: u32, accept: bool, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzAllyGuild::new(packetver);
    pkt.set_other_aid(other_aid);
    pkt.set_answer(if accept { 1 } else { 0 });
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_hostile_guild(target_aid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqHostileGuild::new(packetver);
    pkt.set_aid(target_aid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_delete_related_guild(opponent_gdid: u32, relation: i32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqDeleteRelatedGuild::new(packetver);
    pkt.set_opponent_gdid(opponent_gdid);
    pkt.set_relation(relation);
    pkt.fill_raw();
    pkt.raw
}
