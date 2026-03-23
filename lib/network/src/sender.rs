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
