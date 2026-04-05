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

/// Build a chat packet. `msg` must be in "CharName : text" format.
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

pub fn build_purchase_item_list_packet(items: &[(i16, u16)], packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzPcPurchaseItemlist::new(packetver);
    let item_list: Vec<CzPurchaseItem> = items.iter().map(|(count, item_id)| {
        let mut item = CzPurchaseItem::new(packetver);
        item.set_count(*count);
        item.set_itid(*item_id);
        item.fill_raw();
        item
    }).collect();
    pkt.set_packet_length((4 + items.len() * 4) as i16);
    pkt.set_item_list(item_list);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_sell_item_list_packet(items: &[(i16, i16)], packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzPcSellItemlist::new(packetver);
    let item_list: Vec<CzSellItem> = items.iter().map(|(index, count)| {
        let mut item = CzSellItem::new(packetver);
        item.set_index(*index);
        item.set_count(*count);
        item.fill_raw();
        item
    }).collect();
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
