use packets::packets::*;
use ragnarok_game::event::{CharacterInfo, GameEvent, ServerInfo};
use tracing::debug;

use crate::helpers::decode_pos;

pub fn dispatch_packet(packet: &dyn Packet, packetver: u32) -> Option<GameEvent> {
    match packet.name() {
        "PacketAcAcceptLogin" => {
            let p = packet.as_any().downcast_ref::<PacketAcAcceptLogin>()?;
            let servers = p.server_list.iter().map(ServerInfo::from).collect();
            Some(GameEvent::LoginAccepted {
                account_id: p.aid,
                login_id1: p.auth_code,
                login_id2: p.user_level,
                sex: p.sex,
                servers,
            })
        }
        "PacketAcRefuseLogin" => {
            let p = packet.as_any().downcast_ref::<PacketAcRefuseLogin>()?;
            Some(GameEvent::LoginRefused {
                error_code: p.error_code,
            })
        }
        "PacketHcAcceptEnterNeoUnion" => {
            let p = packet.as_any().downcast_ref::<PacketHcAcceptEnterNeoUnion>()?;
            let characters = p
                .char_info
                .iter()
                .map(|c| CharacterInfo::from_neo_union(c, packetver))
                .collect();
            Some(GameEvent::CharacterListReceived { characters })
        }
        "PacketHcAcceptEnterNeoUnionHeader" => {
            let p = packet
                .as_any()
                .downcast_ref::<PacketHcAcceptEnterNeoUnionHeader>()?;
            let characters = p
                .char_info
                .char_info
                .iter()
                .map(|c| CharacterInfo::from_neo_union(c, packetver))
                .collect();
            Some(GameEvent::CharacterListReceived { characters })
        }
        "PacketHcNotifyZonesvr" => {
            let p = packet.as_any().downcast_ref::<PacketHcNotifyZonesvr>()?;
            let map_name: String = p.map_name.iter().take_while(|c| **c != '\0').collect();
            Some(GameEvent::ZoneServerConnectInfo {
                char_id: p.gid,
                map_name,
                ip: p.addr.ip,
                port: p.addr.port,
            })
        }
        "PacketZcAcceptEnter" => {
            let p = packet.as_any().downcast_ref::<PacketZcAcceptEnter>()?;
            let (x, y, dir) = decode_pos(&p.pos_dir);
            Some(GameEvent::MapEntered {
                x,
                y,
                dir,
                tick: p.start_time,
            })
        }
        "PacketZcAcceptEnter2" => {
            let p = packet.as_any().downcast_ref::<PacketZcAcceptEnter2>()?;
            let (x, y, dir) = decode_pos(&p.pos_dir);
            Some(GameEvent::MapEntered {
                x,
                y,
                dir,
                tick: p.start_time,
            })
        }
        "PacketZcNotifyTime" => {
            let p = packet.as_any().downcast_ref::<PacketZcNotifyTime>()?;
            Some(GameEvent::ServerTick { tick: p.time })
        }
        other => {
            debug!("unhandled packet: {other}");
            None
        }
    }
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
}
