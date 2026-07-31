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

pub fn build_select_accessible_map_packet(slot: u8, map_index: u8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketChSelectAccessibleMapname::new(packetver);
    pkt.set_char_num(slot);
    pkt.set_map_list_num(map_index);
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
    pkt.fill_raw_with_packetver(Some(packetver));
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

pub fn build_emotion_packet(emote_type: u8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqEmotion::new(packetver);
    pkt.set_atype(emote_type);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_action_request_packet(target_gid: u32, action: u8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRequestAct::new(packetver);
    pkt.set_target_gid(target_gid);
    pkt.set_action(action);
    pkt.fill_raw_with_packetver(Some(packetver));
    pkt.raw
}

pub fn build_change_direction_packet(head_dir: u8, dir: u8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzChangeDirection::new(packetver);
    pkt.set_head_dir(head_dir as i16);
    pkt.set_dir(dir);
    pkt.fill_raw_with_packetver(Some(packetver));
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

    // WalkToXY body has version-dependent junk padding before the 3-byte position:
    // pad=0 @20120307 (len5, id0x0437), pad=10 @20050718 (len15, id0x00a7), pad=3 @20040705 (len8, id0x0085)
    #[test]
    fn request_move_body_varies_by_packetver() {
        let modern = build_request_move_packet(10, 20, 20120307);
        assert_eq!(modern.len(), 5);
        assert_eq!(u16::from_le_bytes([modern[0], modern[1]]), 0x0437);

        let mid = build_request_move_packet(10, 20, 20050718);
        assert_eq!(mid.len(), 15);
        assert_eq!(u16::from_le_bytes([mid[0], mid[1]]), 0x00a7);

        let e20080910 = build_request_move_packet(10, 20, 20080910);
        assert_eq!(e20080910.len(), 8);
        assert_eq!(u16::from_le_bytes([e20080910[0], e20080910[1]]), 0x00a7);

        let old = build_request_move_packet(10, 20, 20040705);
        assert_eq!(old.len(), 8);
        assert_eq!(u16::from_le_bytes([old[0], old[1]]), 0x0085);
    }

    #[test]
    fn use_item_body_varies_by_packetver() {
        let idx = 10u16;
        let aid = 222u32;

        let modern = build_use_item_packet(idx, aid, 20120307);
        assert_eq!(modern.len(), 8);
        assert_eq!(u16::from_le_bytes([modern[0], modern[1]]), 0x0439);
        assert_eq!(u16::from_le_bytes([modern[2], modern[3]]), idx);
        assert_eq!(
            u32::from_le_bytes([modern[4], modern[5], modern[6], modern[7]]),
            aid
        );

        let e20040705 = build_use_item_packet(idx, aid, 20040705);
        assert_eq!(e20040705.len(), 13);
        assert_eq!(u16::from_le_bytes([e20040705[0], e20040705[1]]), 0x00a7);
        assert_eq!(u16::from_le_bytes([e20040705[5], e20040705[6]]), idx);
        assert_eq!(
            u32::from_le_bytes([e20040705[9], e20040705[10], e20040705[11], e20040705[12]]),
            aid
        );

        let e20040713 = build_use_item_packet(idx, aid, 20040713);
        assert_eq!(e20040713.len(), 17);
        assert_eq!(u16::from_le_bytes([e20040713[0], e20040713[1]]), 0x00a7);
        assert_eq!(u16::from_le_bytes([e20040713[6], e20040713[7]]), idx);
        assert_eq!(
            u32::from_le_bytes([e20040713[13], e20040713[14], e20040713[15], e20040713[16]]),
            aid
        );

        let e20050718 = build_use_item_packet(idx, aid, 20050718);
        assert_eq!(e20050718.len(), 12);
        assert_eq!(u16::from_le_bytes([e20050718[0], e20050718[1]]), 0x009f);
        assert_eq!(u16::from_le_bytes([e20050718[3], e20050718[4]]), idx);
        assert_eq!(
            u32::from_le_bytes([e20050718[8], e20050718[9], e20050718[10], e20050718[11]]),
            aid
        );

        // below the earliest ladder entry falls back to the oldest id, clean layout
        let ancient = build_use_item_packet(idx, aid, 20040101);
        assert_eq!(ancient.len(), 8);
        assert_eq!(u16::from_le_bytes([ancient[0], ancient[1]]), 0x00a7);
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

    // FROMMC2 (with unique_id) only from 20100105; below that the V1 FROMMC is sent.
    #[test]
    fn purchase_frommc_downlevels_below_20100105() {
        let items = [(2i16, 5i16)];

        let modern = build_purchase_frommc_dispatch(111, 222, &items, 20100105);
        assert_eq!(u16::from_le_bytes([modern[0], modern[1]]), 0x0801);
        assert_eq!(modern.len(), 16); // header2+len2+aid4+uid4 + 1*4

        let old = build_purchase_frommc_dispatch(111, 222, &items, 20100104);
        assert_eq!(u16::from_le_bytes([old[0], old[1]]), 0x0134);
        assert_eq!(old.len(), 12); // header2+len2+aid4 + 1*4 (no unique_id)
    }

    #[test]
    fn adopt_reply_round_trips_through_parser() {
        let raw = build_adopt_reply_packet(111, 222, true, 20120307);
        let parsed = packets::packets_parser::parse(&raw, 20120307);
        let pkt = parsed
            .as_any()
            .downcast_ref::<PacketCzJoinBaby>()
            .expect("expected PacketCzJoinBaby");
        assert_eq!(pkt.aid, 111);
        assert_eq!(pkt.gid, 222);
        assert_eq!(pkt.answer, 1);
    }

    /// The written message rides along with the placement, in the wide field the
    /// server reads it from.
    #[test]
    fn talkbox_ground_cast_carries_its_message() {
        let raw =
            build_use_skill_to_ground_with_talkbox_packet(220, 1, 155, 182, "hi there", 20111102);
        assert_eq!(raw.len(), 90);
        assert_eq!(&raw[..2], &[0xad, 0x08]);

        let parsed = packets::packets_parser::parse(&raw, 20111102);
        let pkt = parsed
            .as_any()
            .downcast_ref::<PacketCzUseSkillTogroundWithtalkbox>()
            .expect("expected PacketCzUseSkillTogroundWithtalkbox");
        assert_eq!(pkt.skid, 220);
        assert_eq!(pkt.selected_level, 1);
        assert_eq!((pkt.x_pos, pkt.y_pos), (155, 182));
        let message: String = pkt.contents.iter().take_while(|c| **c != '\0').collect();
        assert_eq!(message, "hi there");
    }
}

pub fn build_zone_enter_packet(session: &Session) -> Vec<u8> {
    let mut pkt = PacketCzEnter2::new(session.packetver);
    pkt.set_aid(session.account_id);
    pkt.set_gid(session.char_id);
    pkt.set_auth_code(session.login_id1);
    pkt.set_client_time(0);
    pkt.set_sex(session.sex);
    pkt.fill_raw_with_packetver(Some(session.packetver));
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

pub fn build_progress_done_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzProgress::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_cancel_lockon_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzCancelLockon::new(packetver);
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
    pkt.fill_raw_with_packetver(Some(packetver));
    pkt.raw
}

pub fn build_reqname_packet(entity_id: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqname::new(packetver);
    pkt.set_aid(entity_id);
    pkt.fill_raw_with_packetver(Some(packetver));
    pkt.raw
}

pub fn build_solve_char_name_packet(char_id: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqnameBygid::new(packetver);
    pkt.set_gid(char_id);
    pkt.fill_raw_with_packetver(Some(packetver));
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

fn passwd_chars(password: &str) -> [char; 8] {
    let mut buf = ['\0'; 8];
    for (i, c) in password.chars().take(8).enumerate() {
        buf[i] = c;
    }
    buf
}

pub fn build_create_chatroom_packet(
    title: &str,
    limit: i16,
    public: bool,
    password: &str,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzCreateChatroom::new(packetver);
    pkt.set_packet_length((15 + title.len()) as i16);
    pkt.set_size(limit);
    pkt.set_atype(public as u8);
    pkt.set_passwd(passwd_chars(password));
    pkt.set_title(title.to_string());
    pkt.fill_raw();
    pkt.raw
}

pub fn build_change_chatroom_packet(
    title: &str,
    limit: i16,
    public: bool,
    password: &str,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzChangeChatroom::new(packetver);
    pkt.set_packet_length((15 + title.len()) as i16);
    pkt.set_size(limit);
    pkt.set_atype(public as u8);
    pkt.set_passwd(passwd_chars(password));
    pkt.set_title(title.to_string());
    pkt.fill_raw();
    pkt.raw
}

fn name_chars(name: &str) -> [char; 24] {
    let mut buf = ['\0'; 24];
    for (i, c) in name.chars().take(24).enumerate() {
        buf[i] = c;
    }
    buf
}

pub fn build_change_chat_owner_packet(name: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqRoleChange::new(packetver);
    pkt.set_role(0);
    pkt.set_name(name_chars(name));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_expel_chat_member_packet(name: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqExpelMember::new(packetver);
    pkt.set_name(name_chars(name));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_exit_room_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzExitRoom::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_remember_warppoint_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRememberWarppoint::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_lesseffect_packet(is_less: bool, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzLesseffect::new(packetver);
    pkt.set_is_less(is_less as i32);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_user_count_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqUserCount::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_give_manner_point_packet(
    target_aid: u32,
    positive: bool,
    point: i16,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzReqGiveMannerPoint::new(packetver);
    pkt.set_other_aid(target_aid);
    pkt.set_atype(if positive {
        ragnarok_game::gm::MANNER_TYPE_PLUS
    } else {
        ragnarok_game::gm::MANNER_TYPE_MINUS
    });
    pkt.set_point(point.clamp(0, 30000));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_give_manner_byname_packet(name: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqGiveMannerByname::new(packetver);
    pkt.set_char_name(name_to_char24(name));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_status_gm_packet(name: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqStatusGm::new(packetver);
    pkt.set_char_name(name_to_char24(name));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_account_name_packet(aid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqAccountname::new(packetver);
    pkt.set_aid(aid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_alchemist_rank_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzAlchemistRank::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_blacksmith_rank_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzBlacksmithRank::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_taekwon_rank_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzTaekwonRank::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_guild_chat_packet(msg: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzGuildChat::new(packetver);
    let msg_null = format!("{msg}\0");
    pkt.set_packet_length((4 + msg_null.len()) as i16);
    pkt.set_msg(msg_null);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_whisper_packet(receiver: &str, msg: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzWhisper::new(packetver);
    pkt.set_receiver(name_to_char24(receiver));
    let msg_null = format!("{msg}\0");
    pkt.set_packet_length((28 + msg_null.len()) as i16);
    pkt.set_msg(msg_null);
    pkt.fill_raw();
    pkt.raw
}

/// `block` = add the player to the ignore list (`/ex`); `false` removes them
/// (`/in`). The wire `type` byte is 0 for add, 1 for remove.
pub fn build_setting_whisper_pc_packet(name: &str, block: bool, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzSettingWhisperPc::new(packetver);
    pkt.set_name(name_to_char24(name));
    pkt.set_atype(if block { 0 } else { 1 });
    pkt.fill_raw();
    pkt.raw
}

/// `block_all` = ignore every incoming whisper (`/exall`); `false` accepts all
/// (`/inall`).
pub fn build_setting_whisper_state_packet(block_all: bool, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzSettingWhisperState::new(packetver);
    pkt.set_atype(if block_all { 0 } else { 1 });
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
    pkt.fill_raw_with_packetver(Some(packetver));
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
    pkt.fill_raw_with_packetver(Some(packetver));
    pkt.raw
}

pub fn build_pickup_item_packet(itaid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzItemPickup::new(packetver);
    pkt.set_itaid(itaid);
    pkt.fill_raw_with_packetver(Some(packetver));
    pkt.raw
}

pub fn build_move_item_body_to_cart_packet(index: u16, count: i16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMoveItemFromBodyToCart::new(packetver);
    pkt.set_index(index as i16);
    pkt.set_count(count as i32);
    pkt.fill_raw_with_packetver(Some(packetver));
    pkt.raw
}

pub fn build_move_item_cart_to_body_packet(index: u16, count: i16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMoveItemFromCartToBody::new(packetver);
    pkt.set_index(index as i16);
    pkt.set_count(count as i32);
    pkt.fill_raw_with_packetver(Some(packetver));
    pkt.raw
}

pub fn build_move_item_store_to_cart_packet(index: u16, count: i16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMoveItemFromStoreToCart::new(packetver);
    pkt.set_index(index as i16);
    pkt.set_count(count as i32);
    pkt.fill_raw_with_packetver(Some(packetver));
    pkt.raw
}

pub fn build_move_item_cart_to_store_packet(index: u16, count: i16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMoveItemFromCartToStore::new(packetver);
    pkt.set_index(index as i16);
    pkt.set_count(count as i32);
    pkt.fill_raw_with_packetver(Some(packetver));
    pkt.raw
}

pub fn build_move_item_body_to_store_packet(index: u16, count: i16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMoveItemFromBodyToStore::new(packetver);
    pkt.set_index(index as i16);
    pkt.set_count(count as i32);
    pkt.fill_raw_with_packetver(Some(packetver));
    pkt.raw
}

pub fn build_move_item_store_to_body_packet(index: u16, count: i16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMoveItemFromStoreToBody::new(packetver);
    pkt.set_index(index as i16);
    pkt.set_count(count as i32);
    pkt.fill_raw_with_packetver(Some(packetver));
    pkt.raw
}

pub fn build_close_store_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzCloseStore::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

/// `CZ_ACK_STORE_PASSWORD.Type`.
const STORE_PASSWORD_CHANGE: i16 = 2;
const STORE_PASSWORD_CHECK: i16 = 3;

pub fn build_ack_store_password_packet(
    change: bool,
    password: &str,
    new_password: &str,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzAckStorePassword::new(packetver);
    pkt.set_atype(if change {
        STORE_PASSWORD_CHANGE
    } else {
        STORE_PASSWORD_CHECK
    });
    pkt.set_password(to_char_array::<16>(password));
    pkt.set_new_password(to_char_array::<16>(new_password));
    pkt.fill_raw_with_packetver(Some(packetver));
    pkt.raw
}

pub fn build_req_exchange_item_packet(target_aid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqExchangeItem::new(packetver);
    pkt.set_aid(target_aid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_ack_exchange_item_packet(result: u8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzAckExchangeItem::new(packetver);
    pkt.set_result(result);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_add_exchange_item_packet(index: u16, count: i32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzAddExchangeItem::new(packetver);
    pkt.set_index(index as i16);
    pkt.set_count(count);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_conclude_exchange_item_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzConcludeExchangeItem::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_cancel_exchange_item_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzCancelExchangeItem::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_exec_exchange_item_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzExecExchangeItem::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_mail_get_list_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMailGetList::new(packetver);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_mail_open_packet(mail_id: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMailOpen::new(packetver);
    pkt.set_mail_id(mail_id as i32);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_mail_delete_packet(mail_id: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMailDelete::new(packetver);
    pkt.set_mail_id(mail_id as i32);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_mail_get_item_packet(mail_id: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMailGetItem::new(packetver);
    pkt.set_mail_id(mail_id as i32);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_mail_reset_item_packet(ty: u8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMailResetItem::new(packetver);
    pkt.set_atype(ty as i16);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_mail_add_item_packet(index: u16, amount: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzMailAddItem::new(packetver);
    pkt.set_index(index as i16);
    pkt.set_count(amount as i32);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_mail_return_packet(mail_id: u32, receiver: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqMailReturn::new(packetver);
    pkt.set_mail_id(mail_id as i32);
    pkt.set_receive_name(name_chars(receiver));
    pkt.fill_raw();
    pkt.raw
}

/// 0x248 <len>.W <recipient>.24B <title>.40B <body len>.B <body>.?B
///
/// Hand-encoded: the generated `PacketCzMailSend` types `msg_len` as a u32, but
/// the wire field is a single byte.
pub fn build_mail_send_packet(to: &str, title: &str, body: &str, _packetver: u32) -> Vec<u8> {
    let mut cut = body.len().min(ragnarok_game::mail::MAIL_BODY_MAX);
    while cut > 0 && !body.is_char_boundary(cut) {
        cut -= 1;
    }
    let body_bytes = &body.as_bytes()[..cut];

    let pkt_len = 69 + body_bytes.len();
    let mut buf = Vec::with_capacity(pkt_len);
    buf.extend_from_slice(&0x248i16.to_le_bytes());
    buf.extend_from_slice(&(pkt_len as i16).to_le_bytes());

    let mut recv = [0u8; 24];
    for (i, b) in to.as_bytes().iter().take(23).enumerate() {
        recv[i] = *b;
    }
    buf.extend_from_slice(&recv);

    let mut header = [0u8; 40];
    for (i, b) in title.as_bytes().iter().take(39).enumerate() {
        header[i] = *b;
    }
    buf.extend_from_slice(&header);

    buf.push(body_bytes.len() as u8);
    buf.extend_from_slice(body_bytes);
    buf
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
    pkt.fill_raw_with_packetver(Some(packetver));
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
    pkt.fill_raw_with_packetver(Some(packetver));
    pkt.raw
}

pub fn build_use_skill_to_ground_with_talkbox_packet(
    skill_id: u16,
    level: i16,
    x: i16,
    y: i16,
    message: &str,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzUseSkillTogroundWithtalkbox::new(packetver);
    pkt.set_selected_level(level);
    pkt.set_skid(skill_id);
    pkt.set_x_pos(x);
    pkt.set_y_pos(y);
    let mut bytes = [0u8; 80];
    let src = message.as_bytes();
    let n = src.len().min(79);
    bytes[..n].copy_from_slice(&src[..n]);
    pkt.set_contents(std::array::from_fn(|i| bytes[i] as char));
    pkt.fill_raw_with_packetver(Some(packetver));
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

pub fn build_doridori_packet(packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzDoridori::new(packetver);
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

pub fn build_adopt_request_packet(target_aid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqJoinBaby::new(packetver);
    pkt.set_aid(target_aid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_adopt_reply_packet(
    father_aid: u32,
    mother_aid: u32,
    accept: bool,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzJoinBaby::new(packetver);
    pkt.set_aid(father_aid);
    pkt.set_gid(mother_aid);
    pkt.set_answer(if accept { 1 } else { 0 });
    pkt.fill_raw();
    pkt.raw
}

pub fn build_marry_request_packet(target_aid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqJoinCouple::new(packetver);
    pkt.set_aid(target_aid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_marry_reply_packet(
    proposer_aid: u32,
    proposer_gid: u32,
    accept: bool,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzJoinCouple::new(packetver);
    pkt.set_aid(proposer_aid);
    pkt.set_gid(proposer_gid);
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

pub fn build_agree_star_place_packet(which: i8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzAgreeStarplace::new(packetver);
    pkt.set_which(which);
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

/// Buy from a vending merchant. `CZ_PC_PURCHASE_ITEMLIST_FROMMC2` carries a
/// `unique_id` and is only registered server-side from 20100105; below that the
/// V1 `CZ_PC_PURCHASE_ITEMLIST_FROMMC` (no `unique_id`) must be sent.
pub fn build_purchase_frommc_dispatch(
    aid: u32,
    unique_id: u32,
    items: &[(i16, i16)],
    packetver: u32,
) -> Vec<u8> {
    if packetver >= 20100105 {
        build_purchase_frommc2_packet(aid, unique_id, items, packetver)
    } else {
        build_purchase_frommc_packet(aid, items, packetver)
    }
}

pub fn build_purchase_frommc_packet(aid: u32, items: &[(i16, i16)], packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzPcPurchaseItemlistFrommc::new(packetver);
    pkt.set_aid(aid);
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
    // header(2) + len(2) + aid(4) + N*4
    pkt.set_packet_length((8 + items.len() * 4) as i16);
    pkt.set_item_list(item_list);
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

pub fn build_trycapture_packet(gid: u32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzTrycaptureMonster::new(packetver);
    pkt.set_target_aid(gid);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_command_pet_packet(csub: i8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzCommandPet::new(packetver);
    pkt.set_c_sub(csub);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_rename_pet_packet(name: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRenamePet::new(packetver);
    pkt.set_sz_name(name_to_char24(name));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_select_petegg_packet(index: u16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzSelectPetegg::new(packetver);
    pkt.set_index(index as i16);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_pet_act_packet(data: i32, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzPetAct::new(packetver);
    pkt.set_data(data);
    pkt.fill_raw();
    pkt.raw
}

pub fn build_active_quest_packet(quest_id: u32, active: bool, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzActiveQuest::new(packetver);
    pkt.set_quest_id(quest_id);
    pkt.set_active(active);
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

pub fn build_req_ban_guild(gdid: u32, aid: i32, gid: i32, reason: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzReqBanGuild::new(packetver);
    pkt.set_gdid(gdid);
    pkt.set_aid(aid);
    pkt.set_gid(gid);
    pkt.set_reason_desc(to_char_array::<40>(reason));
    pkt.fill_raw();
    pkt.raw
}

pub fn build_req_change_memberpos(aid: i32, gid: i32, position_id: i32, packetver: u32) -> Vec<u8> {
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

pub fn build_req_delete_related_guild(
    opponent_gdid: u32,
    relation: i32,
    packetver: u32,
) -> Vec<u8> {
    let mut pkt = PacketCzReqDeleteRelatedGuild::new(packetver);
    pkt.set_opponent_gdid(opponent_gdid);
    pkt.set_relation(relation);
    pkt.fill_raw();
    pkt.raw
}
