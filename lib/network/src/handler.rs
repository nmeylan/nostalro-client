use packets::packets::*;
use ragnarok_game::event::{CharacterInfo, GameEvent, ServerInfo};
use tracing::debug;

use crate::helpers::{decode_pos, decode_pos2};

pub fn dispatch_packet(packet: &dyn Packet, packetver: u32) -> Option<GameEvent> {
    let any = packet.as_any();

    if let Some(p) = any.downcast_ref::<PacketAcAcceptLogin>() {
        let servers = p.server_list.iter().map(ServerInfo::from).collect();
        return Some(GameEvent::LoginAccepted {
            account_id: p.aid,
            login_id1: p.auth_code,
            login_id2: p.user_level,
            sex: p.sex,
            servers,
        });
    }
    if let Some(p) = any.downcast_ref::<PacketAcRefuseLogin>() {
        return Some(GameEvent::LoginRefused {
            error_code: p.error_code,
        });
    }
    if let Some(p) = any.downcast_ref::<PacketHcAcceptEnterNeoUnion>() {
        let characters = p
            .char_info
            .iter()
            .map(|c| CharacterInfo::from_neo_union(c, packetver))
            .collect();
        return Some(GameEvent::CharacterListReceived { characters });
    }
    if let Some(p) = any.downcast_ref::<PacketHcAcceptEnterNeoUnionHeader>() {
        let characters = p
            .char_info
            .char_info
            .iter()
            .map(|c| CharacterInfo::from_neo_union(c, packetver))
            .collect();
        return Some(GameEvent::CharacterListReceived { characters });
    }
    if let Some(p) = any.downcast_ref::<PacketHcNotifyZonesvr>() {
        let map_name: String = p.map_name.iter().take_while(|c| **c != '\0').collect();
        return Some(GameEvent::ZoneServerConnectInfo {
            char_id: p.gid,
            map_name,
            ip: p.addr.ip,
            port: p.addr.port,
        });
    }
    if let Some(p) = any.downcast_ref::<PacketZcAcceptEnter>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return Some(GameEvent::MapEntered {
            x,
            y,
            dir,
            tick: p.start_time,
        });
    }
    if let Some(p) = any.downcast_ref::<PacketZcAcceptEnter2>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return Some(GameEvent::MapEntered {
            x,
            y,
            dir,
            tick: p.start_time,
        });
    }
    if let Some(p) = any.downcast_ref::<PacketZcNpcackMapmove>() {
        let map_name: String = p.map_name.iter().take_while(|c| **c != '\0').collect();
        return Some(GameEvent::MapChanged {
            map_name,
            x: p.x_pos,
            y: p.y_pos,
        });
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyPlayermove>() {
        let (x1, y1, x2, y2) = decode_pos2(&p.move_data);
        return Some(GameEvent::PlayerMoved {
            start_x: x1,
            start_y: y1,
            dest_x: x2,
            dest_y: y2,
            start_time: p.move_start_time,
        });
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyTime>() {
        return Some(GameEvent::ServerTick { tick: p.time });
    }

    // Entity spawn packets
    if let Some(p) = any.downcast_ref::<PacketZcNotifyStandentry7>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return Some(GameEvent::EntitySpawned {
            gid: p.gid,
            job: p.job as u16,
            speed: p.speed as u16,
            sex: p.sex,
            head: p.head as u16,
            weapon: p.weapon as u16,
            shield: p.shield as u16,
            head_top: p.accessory2,
            head_mid: p.accessory3,
            head_bottom: p.accessory,
            hair_color: p.headpalette,
            x, y, direction: dir,
        });
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyStandentry>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return Some(GameEvent::EntitySpawned {
            gid: p.gid,
            job: p.job as u16,
            speed: p.speed as u16,
            sex: p.sex,
            head: p.head as u16,
            weapon: p.weapon as u16,
            shield: p.shield as u16,
            head_top: p.accessory2 as u16,
            head_mid: p.accessory3 as u16,
            head_bottom: p.accessory as u16,
            hair_color: p.headpalette as u16,
            x, y, direction: dir,
        });
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyNewentry>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return Some(GameEvent::EntitySpawned {
            gid: p.gid,
            job: p.job as u16,
            speed: p.speed as u16,
            sex: p.sex,
            head: p.head as u16,
            weapon: p.weapon as u16,
            shield: p.shield as u16,
            head_top: p.accessory2 as u16,
            head_mid: p.accessory3 as u16,
            head_bottom: p.accessory as u16,
            hair_color: p.headpalette as u16,
            x, y, direction: dir,
        });
    }
    // MoveEntry8: entity entering view while already moving — treat as spawn at pos_dir
    if let Some(p) = any.downcast_ref::<PacketZcNotifyMoveentry8>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return Some(GameEvent::EntitySpawned {
            gid: p.gid,
            job: p.job as u16,
            speed: p.speed as u16,
            sex: p.sex,
            head: p.head as u16,
            weapon: p.weapon as u16,
            shield: p.shield as u16,
            head_top: p.accessory2,
            head_mid: p.accessory3,
            head_bottom: p.accessory,
            hair_color: p.headpalette,
            x, y, direction: dir,
        });
    }

    // Entity movement
    if let Some(p) = any.downcast_ref::<PacketZcNotifyMove>() {
        let (x1, y1, x2, y2) = decode_pos2(&p.move_data);
        return Some(GameEvent::EntityMoved {
            gid: p.gid,
            start_x: x1, start_y: y1,
            dest_x: x2, dest_y: y2,
            start_time: p.move_start_time,
        });
    }
    if let Some(p) = any.downcast_ref::<PacketZcStopmove>() {
        return Some(GameEvent::EntityStopMove {
            gid: p.aid,
            x: p.x_pos as u16,
            y: p.y_pos as u16,
        });
    }

    // Entity despawn
    if let Some(p) = any.downcast_ref::<PacketZcNotifyVanish>() {
        return Some(GameEvent::EntityVanished { gid: p.gid });
    }

    debug!("unhandled packet: {}", packet.name());
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_unknown_packet_returns_none() {
        let packetver = 20120307;
        let mut pkt = PacketZcLoadConfirm::new(packetver);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert!(result.is_none());
    }

    #[test]
    fn dispatch_notify_time_returns_server_tick() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyTime::new(packetver);
        pkt.set_time(42000);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        match result {
            Some(GameEvent::ServerTick { tick }) => assert_eq!(tick, 42000),
            other => panic!("expected ServerTick, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_refuse_login_returns_error_code() {
        let packetver = 20120307;
        let mut pkt = PacketAcRefuseLogin::new(packetver);
        pkt.set_error_code(1);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        match result {
            Some(GameEvent::LoginRefused { error_code }) => assert_eq!(error_code, 1),
            other => panic!("expected LoginRefused, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_mapmove_returns_map_changed() {
        let packetver = 20120307;
        let mut pkt = PacketZcNpcackMapmove::new(packetver);
        let mut map_name = [0 as char; 16];
        for (i, c) in "prt_fild08.gat".chars().enumerate() {
            map_name[i] = c;
        }
        pkt.set_map_name(map_name);
        pkt.set_x_pos(150);
        pkt.set_y_pos(200);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        match result {
            Some(GameEvent::MapChanged { map_name, x, y }) => {
                assert_eq!(map_name, "prt_fild08.gat");
                assert_eq!(x, 150);
                assert_eq!(y, 200);
            }
            other => panic!("expected MapChanged, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_notify_playermove_decodes_positions() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyPlayermove::new(packetver);
        pkt.set_move_start_time(5000);
        // Encode (100, 200) -> (110, 210) into move_data
        let x1: u16 = 100; let y1: u16 = 200;
        let x2: u16 = 110; let y2: u16 = 210;
        let b0 = (x1 >> 2) as u8;
        let b1 = ((x1 << 6) as u8) | ((y1 >> 4) as u8);
        let b2 = ((y1 << 4) as u8) | ((x2 >> 6) as u8);
        let b3 = ((x2 << 2) as u8) | ((y2 >> 8) as u8);
        let b4 = y2 as u8;
        let b5 = 0u8;
        pkt.set_move_data([b0, b1, b2, b3, b4, b5]);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        match result {
            Some(GameEvent::PlayerMoved { start_x, start_y, dest_x, dest_y, start_time }) => {
                assert_eq!((start_x, start_y), (100, 200));
                assert_eq!((dest_x, dest_y), (110, 210));
                assert_eq!(start_time, 5000);
            }
            other => panic!("expected PlayerMoved, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_accept_enter_decodes_position() {
        let packetver = 20120307;
        let mut pkt = PacketZcAcceptEnter::new(packetver);
        pkt.set_start_time(1000);
        // Encode position (100, 200, 3) into pos_dir
        let encoded = crate::helpers::encode_pos(100, 200, 3);
        pkt.set_pos_dir(encoded);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        match result {
            Some(GameEvent::MapEntered { x, y, dir, tick }) => {
                assert_eq!((x, y, dir, tick), (100, 200, 3, 1000));
            }
            other => panic!("expected MapEntered, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_vanish_returns_entity_vanished() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyVanish::new(packetver);
        pkt.set_gid(42);
        pkt.set_atype(0);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        match result {
            Some(GameEvent::EntityVanished { gid }) => assert_eq!(gid, 42),
            other => panic!("expected EntityVanished, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_notify_move_returns_entity_moved() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyMove::new(packetver);
        pkt.set_gid(99);
        pkt.set_move_start_time(7000);
        let x1: u16 = 50; let y1: u16 = 60;
        let x2: u16 = 55; let y2: u16 = 65;
        let b0 = (x1 >> 2) as u8;
        let b1 = ((x1 << 6) as u8) | ((y1 >> 4) as u8);
        let b2 = ((y1 << 4) as u8) | ((x2 >> 6) as u8);
        let b3 = ((x2 << 2) as u8) | ((y2 >> 8) as u8);
        let b4 = y2 as u8;
        let b5 = 0u8;
        pkt.set_move_data([b0, b1, b2, b3, b4, b5]);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        match result {
            Some(GameEvent::EntityMoved { gid, start_x, start_y, dest_x, dest_y, start_time }) => {
                assert_eq!(gid, 99);
                assert_eq!((start_x, start_y), (50, 60));
                assert_eq!((dest_x, dest_y), (55, 65));
                assert_eq!(start_time, 7000);
            }
            other => panic!("expected EntityMoved, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_stopmove_returns_entity_stop_move() {
        let packetver = 20120307;
        let mut pkt = PacketZcStopmove::new(packetver);
        pkt.set_aid(77);
        pkt.set_x_pos(120);
        pkt.set_y_pos(130);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        match result {
            Some(GameEvent::EntityStopMove { gid, x, y }) => {
                assert_eq!(gid, 77);
                assert_eq!((x, y), (120, 130));
            }
            other => panic!("expected EntityStopMove, got {other:?}"),
        }
    }
}
