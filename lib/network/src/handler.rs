use packets::packets::*;
use ragnarok_game::event::{CharacterInfo, GameEvent, ServerInfo};
use ragnarok_game::inventory::{EquipmentItemData, NormalItemData};
use tracing::debug;

use crate::helpers::{decode_pos, decode_pos2};

pub fn dispatch_packet(packet: &dyn Packet, packetver: u32) -> Vec<GameEvent> {
    let any = packet.as_any();

    if let Some(p) = any.downcast_ref::<PacketAcAcceptLogin>() {
        let servers = p.server_list.iter().map(ServerInfo::from).collect();
        return vec![GameEvent::LoginAccepted {
            account_id: p.aid,
            login_id1: p.auth_code,
            login_id2: p.user_level,
            sex: p.sex,
            servers,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketAcRefuseLogin>() {
        return vec![GameEvent::LoginRefused {
            error_code: p.error_code,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketHcAcceptEnterNeoUnion>() {
        let characters = p
            .char_info
            .iter()
            .map(|c| CharacterInfo::from_neo_union(c, packetver))
            .collect();
        return vec![GameEvent::CharacterListReceived { characters }];
    }
    if let Some(p) = any.downcast_ref::<PacketHcAcceptEnterNeoUnionHeader>() {
        let characters = p
            .char_info
            .char_info
            .iter()
            .map(|c| CharacterInfo::from_neo_union(c, packetver))
            .collect();
        return vec![GameEvent::CharacterListReceived { characters }];
    }
    if let Some(p) = any.downcast_ref::<PacketHcNotifyZonesvr>() {
        let map_name: String = p.map_name.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::ZoneServerConnectInfo {
            char_id: p.gid,
            map_name,
            ip: p.addr.ip,
            port: p.addr.port,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAcceptEnter>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return vec![GameEvent::MapEntered {
            x,
            y,
            dir,
            tick: p.start_time,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAcceptEnter2>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return vec![GameEvent::MapEntered {
            x,
            y,
            dir,
            tick: p.start_time,
        }];
    }
    if any.downcast_ref::<PacketZcRestartAck>().is_some() {
        return vec![GameEvent::RestartAck];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNpcackMapmove>() {
        let map_name: String = p.map_name.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::MapChanged {
            map_name,
            x: p.x_pos,
            y: p.y_pos,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyPlayermove>() {
        let (x1, y1, x2, y2) = decode_pos2(&p.move_data);
        return vec![GameEvent::PlayerMoved {
            start_x: x1,
            start_y: y1,
            dest_x: x2,
            dest_y: y2,
            start_time: p.move_start_time,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyTime>() {
        return vec![GameEvent::ServerTick { server_tick: p.time, local_send_time_ms: 0 }];
    }

    // Entity spawn packets
    if let Some(p) = any.downcast_ref::<PacketZcNotifyStandentry7>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return vec![GameEvent::EntitySpawned {
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
            body_state: p.body_state,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyStandentry>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return vec![GameEvent::EntitySpawned {
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
            body_state: p.body_state,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyNewentry>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return vec![GameEvent::EntitySpawned {
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
            body_state: p.body_state,
        }];
    }
    // MoveEntry8: entity entering view while already moving — treat as spawn at pos_dir
    if let Some(p) = any.downcast_ref::<PacketZcNotifyMoveentry8>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return vec![GameEvent::EntitySpawned {
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
            body_state: p.body_state,
        }];
    }
    // MoveEntry9: entity entering view while already moving — spawn + start movement
    if let Some(p) = any.downcast_ref::<PacketZcNotifyMoveentry9>() {
        let (x1, y1, x2, y2) = decode_pos2(&p.move_data);
        return vec![
            GameEvent::EntitySpawned {
                gid: p.gid,
                job: p.job as u16,
                speed: p.speed as u16,
                sex: p.sex,
                head: p.head as u16,
                weapon: p.weapon as u16,
                shield: 0,
                head_top: p.accessory2 as u16,
                head_mid: p.accessory3 as u16,
                head_bottom: p.accessory as u16,
                hair_color: p.headpalette as u16,
                x: x1, y: y1, direction: 0,
                body_state: p.body_state,
            },
            GameEvent::EntityMoved {
                gid: p.gid,
                start_x: x1, start_y: y1,
                dest_x: x2, dest_y: y2,
                start_time: p.move_start_time,
            },
        ];
    }

    // Entity movement
    if let Some(p) = any.downcast_ref::<PacketZcNotifyMove>() {
        let (x1, y1, x2, y2) = decode_pos2(&p.move_data);
        return vec![GameEvent::EntityMoved {
            gid: p.gid,
            start_x: x1, start_y: y1,
            dest_x: x2, dest_y: y2,
            start_time: p.move_start_time,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcStopmove>() {
        return vec![GameEvent::EntityStopMove {
            gid: p.aid,
            x: p.x_pos as u16,
            y: p.y_pos as u16,
        }];
    }

    // Entity action (sit, stand, attack, etc.)
    if let Some(p) = any.downcast_ref::<PacketZcNotifyAct>() {
        return vec![GameEvent::EntityAction {
            gid: p.gid,
            target_gid: p.target_gid,
            action: p.action,
            damage: p.damage,
            left_damage: p.left_damage,
            attack_mt: p.attack_mt,
            attacked_mt: p.attacked_mt,
            start_time: p.start_time,
            count: p.count,
        }];
    }

    // Entity direction change (doridori)
    if let Some(p) = any.downcast_ref::<PacketZcChangeDirection>() {
        return vec![GameEvent::EntityDirectionChanged {
            gid: p.aid,
            head_dir: p.head_dir as u8,
            dir: p.dir,
        }];
    }

    // Chat messages
    if let Some(p) = any.downcast_ref::<PacketZcNotifyChat>() {
        let message: String = p.msg.chars().take_while(|c| *c != '\0').collect();
        return vec![GameEvent::ChatMessage { gid: p.gid, message }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyPlayerchat>() {
        let message: String = p.msg.chars().take_while(|c| *c != '\0').collect();
        return vec![GameEvent::OwnChatMessage { message }];
    }

    // Entity name
    if let Some(p) = any.downcast_ref::<PacketZcAckReqname>() {
        let name: String = p.cname.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::EntityNameReceived { gid: p.aid, name }];
    }

    // Entity despawn
    if let Some(p) = any.downcast_ref::<PacketZcNotifyVanish>() {
        return vec![GameEvent::EntityVanished { gid: p.gid }];
    }

    // Character stats & parameters
    if let Some(p) = any.downcast_ref::<PacketZcParChange>() {
        return vec![GameEvent::ParameterChanged { var_id: p.var_id, value: p.count }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcStatusValues>() {
        return vec![GameEvent::StatusChanged {
            status_type: p.status_type,
            base: p.default_status,
            bonus: p.plus_status,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAttackRange>() {
        return vec![GameEvent::AttackRangeChanged { range: p.current_att_range }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcSpriteChange2>() {
        return vec![GameEvent::EntitySpriteChanged {
            gid: p.gid,
            sprite_type: p.atype,
            value: p.value,
            value2: p.value2,
        }];
    }

    // Skill casting & emotions
    if let Some(p) = any.downcast_ref::<PacketZcUseskillAck2>() {
        return vec![GameEvent::SkillCasting {
            gid: p.aid,
            target_gid: p.target_id,
            skill_id: p.skid,
            delay_ms: p.delay_time,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcEmotion>() {
        return vec![GameEvent::EntityEmotion {
            gid: p.gid,
            emotion_type: p.atype,
        }];
    }

    // Party member HP notifications
    if let Some(p) = any.downcast_ref::<PacketZcNotifyHpToGroupm>() {
        return vec![GameEvent::EntityHpChanged {
            gid: p.aid,
            hp: p.hp as u32,
            max_hp: p.maxhp as u32,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyHpToGroupmR2>() {
        return vec![GameEvent::EntityHpChanged {
            gid: p.aid,
            hp: p.hp as u32,
            max_hp: p.maxhp as u32,
        }];
    }

    // NPC dialog
    if let Some(p) = any.downcast_ref::<PacketZcSayDialog>() {
        let text: String = p.msg.chars().take_while(|c| *c != '\0').collect();
        return vec![GameEvent::NpcDialogText { npc_id: p.naid, text }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcWaitDialog>() {
        return vec![GameEvent::NpcDialogNext { npc_id: p.naid }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcCloseDialog>() {
        return vec![GameEvent::NpcDialogClose { npc_id: p.naid }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcMenuList>() {
        let raw_msg: String = p.msg.chars().take_while(|c| *c != '\0').collect();
        let items: Vec<String> = raw_msg.split(':')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        return vec![GameEvent::NpcDialogMenu { npc_id: p.naid, items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcOpenEditdlg>() {
        return vec![GameEvent::NpcInputNumber { npc_id: p.naid }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcOpenEditdlgstr>() {
        return vec![GameEvent::NpcInputString { npc_id: p.naid }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcSelectDealtype>() {
        return vec![GameEvent::NpcDealTypeSelect { npc_id: p.naid }];
    }

    // NPC shop
    if let Some(p) = any.downcast_ref::<PacketZcPcPurchaseItemlist>() {
        let items = p.item_list.iter().map(|item| {
            (item.itid, item.price, item.discountprice, item.atype)
        }).collect();
        return vec![GameEvent::NpcShopBuyList { npc_id: 0, items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPcSellItemlist>() {
        let items = p.item_list.iter().map(|item| {
            (item.index, item.price, item.overchargeprice)
        }).collect();
        return vec![GameEvent::NpcShopSellList { npc_id: 0, items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPcPurchaseResult>() {
        return vec![GameEvent::NpcShopBuyResult { result: p.result }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPcSellResult>() {
        return vec![GameEvent::NpcShopSellResult { result: p.result }];
    }

    // Inventory
    if let Some(p) = any.downcast_ref::<PacketZcNormalItemlist>() {
        let items = p.item_info.iter().map(|i| NormalItemData {
            index: i.index, item_id: i.itid, item_type: i.atype,
            is_identified: i.is_identified, count: i.count, wear_state: i.wear_state,
        }).collect();
        return vec![GameEvent::InventoryNormalItems { items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcEquipmentItemlist>() {
        let items = p.item_info.iter().map(|i| EquipmentItemData {
            index: i.index, item_id: i.itid, item_type: i.atype,
            is_identified: i.is_identified, location: i.location,
            wear_state: i.wear_state, is_damaged: i.is_damaged,
            refining_level: i.refining_level,
            slot: [i.slot.card1, i.slot.card2, i.slot.card3, i.slot.card4],
        }).collect();
        return vec![GameEvent::InventoryEquipmentItems { items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcItemPickupAck>() {
        return vec![GameEvent::InventoryItemPickup {
            index: p.index, item_id: p.itid, count: p.count, item_type: p.atype,
            is_identified: p.is_identified, is_damaged: p.is_damaged,
            refining_level: p.refining_level,
            slot: [p.slot.card1, p.slot.card2, p.slot.card3, p.slot.card4],
            location: p.location, result: p.result,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcItemPickupAck2>() {
        return vec![GameEvent::InventoryItemPickup {
            index: p.index, item_id: p.itid, count: p.count, item_type: p.atype,
            is_identified: p.is_identified, is_damaged: p.is_damaged,
            refining_level: p.refining_level,
            slot: [p.slot.card1, p.slot.card2, p.slot.card3, p.slot.card4],
            location: p.location, result: p.result,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcItemPickupAck3>() {
        return vec![GameEvent::InventoryItemPickup {
            index: p.index, item_id: p.itid, count: p.count, item_type: p.atype,
            is_identified: p.is_identified, is_damaged: p.is_damaged,
            refining_level: p.refining_level,
            slot: [p.slot.card1, p.slot.card2, p.slot.card3, p.slot.card4],
            location: p.location, result: p.result,
        }];
    }
    // Normal item list v2/v3
    if let Some(p) = any.downcast_ref::<PacketZcNormalItemlist2>() {
        let items = p.item_info.iter().map(|i| NormalItemData {
            index: i.index, item_id: i.itid, item_type: i.atype,
            is_identified: i.is_identified, count: i.count, wear_state: i.wear_state,
        }).collect();
        return vec![GameEvent::InventoryNormalItems { items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNormalItemlist3>() {
        let items = p.item_info.iter().map(|i| NormalItemData {
            index: i.index, item_id: i.itid, item_type: i.atype,
            is_identified: i.is_identified, count: i.count, wear_state: i.wear_state,
        }).collect();
        return vec![GameEvent::InventoryNormalItems { items }];
    }
    // Equipment item list v2/v3
    if let Some(p) = any.downcast_ref::<PacketZcEquipmentItemlist2>() {
        let items = p.item_info.iter().map(|i| EquipmentItemData {
            index: i.index, item_id: i.itid, item_type: i.atype,
            is_identified: i.is_identified, location: i.location,
            wear_state: i.wear_state, is_damaged: i.is_damaged,
            refining_level: i.refining_level,
            slot: [i.slot.card1, i.slot.card2, i.slot.card3, i.slot.card4],
        }).collect();
        return vec![GameEvent::InventoryEquipmentItems { items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcEquipmentItemlist3>() {
        let items = p.item_info.iter().map(|i| EquipmentItemData {
            index: i.index, item_id: i.itid, item_type: i.atype,
            is_identified: i.is_identified, location: i.location,
            wear_state: i.wear_state, is_damaged: i.is_damaged,
            refining_level: i.refining_level,
            slot: [i.slot.card1, i.slot.card2, i.slot.card3, i.slot.card4],
        }).collect();
        return vec![GameEvent::InventoryEquipmentItems { items }];
    }
    // Use item ack v1/v2
    if let Some(p) = any.downcast_ref::<PacketZcUseItemAck>() {
        return vec![GameEvent::InventoryUseItemResult {
            index: p.index, count: p.count, success: p.result,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcUseItemAck2>() {
        return vec![GameEvent::InventoryUseItemResult {
            index: p.index, count: p.count, success: p.result,
        }];
    }
    // Arrow/ammo equip notification
    if let Some(p) = any.downcast_ref::<PacketZcEquipArrow>() {
        return vec![GameEvent::InventoryArrowEquipped { index: p.index as u16 }];
    }
    // Equip/unequip ack v1/v2
    if let Some(p) = any.downcast_ref::<PacketZcReqWearEquipAck>() {
        return vec![GameEvent::InventoryEquipResult {
            index: p.index, wear_location: p.wear_location, view_id: p.view_id,
            success: p.result == 1,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcReqWearEquipAck2>() {
        return vec![GameEvent::InventoryEquipResult {
            index: p.index, wear_location: p.wear_location, view_id: p.view_id,
            success: p.result == 0,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcReqTakeoffEquipAck>() {
        return vec![GameEvent::InventoryUnequipResult {
            index: p.index, wear_location: p.wear_location, success: p.result == 0,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcReqTakeoffEquipAck2>() {
        return vec![GameEvent::InventoryUnequipResult {
            index: p.index, wear_location: p.wear_location, success: p.result == 0,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcItemThrowAck>() {
        return vec![GameEvent::InventoryItemRemoved {
            index: p.index, count: p.count,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcDeleteItemFromBody>() {
        return vec![GameEvent::InventoryItemRemoved {
            index: p.index, count: p.count,
        }];
    }

    // Floor items
    if let Some(p) = any.downcast_ref::<PacketZcItemFallEntry>() {
        return vec![GameEvent::FloorItemAppeared {
            id: p.itaid, item_id: p.itid, is_identified: p.is_identified,
            x: p.x_pos, y: p.y_pos, sub_x: p.sub_x, sub_y: p.sub_y,
            count: p.count, is_falling: true,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcItemEntry>() {
        return vec![GameEvent::FloorItemAppeared {
            id: p.itaid, item_id: p.itid, is_identified: p.is_identified,
            x: p.x_pos, y: p.y_pos, sub_x: p.sub_x, sub_y: p.sub_y,
            count: p.count, is_falling: false,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcItemDisappear>() {
        return vec![GameEvent::FloorItemDisappeared { id: p.itaid }];
    }

    // Acknowledged but not yet used (no UI)
    if let Some(p) = any.downcast_ref::<PacketZcAid>() {
        debug!("zone server confirmed AID={}", p.aid);
        return vec![GameEvent::Acknowledged];
    }
    if any.downcast_ref::<PacketHcBlockCharacter>().is_some() {
        return vec![GameEvent::Acknowledged];
    }
    if any.downcast_ref::<PacketPincodeLoginstate>().is_some() {
        return vec![GameEvent::Acknowledged];
    }
    if any.downcast_ref::<PacketZcFriendsList>().is_some() {
        return vec![GameEvent::Acknowledged];
    }
    if any.downcast_ref::<PacketZcSkillinfoList>().is_some() {
        return vec![GameEvent::Acknowledged];
    }
    if any.downcast_ref::<PacketZcShortcutKeyListV2>().is_some() {
        return vec![GameEvent::Acknowledged];
    }
    if any.downcast_ref::<PacketZcNotifyMapproperty>().is_some() {
        return vec![GameEvent::Acknowledged];
    }

    debug!("unhandled packet: {}", packet.name());
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_unknown_packet_returns_empty() {
        let packetver = 20120307;
        let mut pkt = PacketZcLoadConfirm::new(packetver);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert!(result.is_empty());
    }

    #[test]
    fn dispatch_notify_time_returns_server_tick() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyTime::new(packetver);
        pkt.set_time(42000);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::ServerTick { server_tick, .. } => assert_eq!(*server_tick, 42000),
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
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::LoginRefused { error_code } => assert_eq!(*error_code, 1),
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
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::MapChanged { map_name, x, y } => {
                assert_eq!(map_name, "prt_fild08.gat");
                assert_eq!(*x, 150);
                assert_eq!(*y, 200);
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
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::PlayerMoved { start_x, start_y, dest_x, dest_y, start_time } => {
                assert_eq!((*start_x, *start_y), (100, 200));
                assert_eq!((*dest_x, *dest_y), (110, 210));
                assert_eq!(*start_time, 5000);
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
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::MapEntered { x, y, dir, tick } => {
                assert_eq!((*x, *y, *dir, *tick), (100, 200, 3, 1000));
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
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntityVanished { gid } => assert_eq!(*gid, 42),
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
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntityMoved { gid, start_x, start_y, dest_x, dest_y, start_time } => {
                assert_eq!(*gid, 99);
                assert_eq!((*start_x, *start_y), (50, 60));
                assert_eq!((*dest_x, *dest_y), (55, 65));
                assert_eq!(*start_time, 7000);
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
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntityStopMove { gid, x, y } => {
                assert_eq!(*gid, 77);
                assert_eq!((*x, *y), (120, 130));
            }
            other => panic!("expected EntityStopMove, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_notify_act_returns_entity_action() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyAct::new(packetver);
        pkt.set_gid(50);
        pkt.set_target_gid(99);
        pkt.set_action(8);
        pkt.set_damage(42);
        pkt.set_attack_mt(500);
        pkt.set_attacked_mt(300);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntityAction { gid, target_gid, action, damage, attack_mt, attacked_mt, .. } => {
                assert_eq!(*gid, 50);
                assert_eq!(*target_gid, 99);
                assert_eq!(*action, 8);
                assert_eq!(*damage, 42);
                assert_eq!(*attack_mt, 500);
                assert_eq!(*attacked_mt, 300);
            }
            other => panic!("expected EntityAction, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_change_direction_returns_entity_direction_changed() {
        let packetver = 20120307;
        let mut pkt = PacketZcChangeDirection::new(packetver);
        pkt.set_aid(60);
        pkt.set_head_dir(1);
        pkt.set_dir(3);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntityDirectionChanged { gid, head_dir, dir } => {
                assert_eq!(*gid, 60);
                assert_eq!(*head_dir, 1);
                assert_eq!(*dir, 3);
            }
            other => panic!("expected EntityDirectionChanged, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_notify_chat_returns_chat_message() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyChat::new(packetver);
        pkt.set_gid(42);
        pkt.set_msg("Player : Hello".to_string());
        pkt.set_msg_raw("Player : Hello".as_bytes().to_vec());
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::ChatMessage { gid, message } => {
                assert_eq!(*gid, 42);
                assert_eq!(message, "Player : Hello");
            }
            other => panic!("expected ChatMessage, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_notify_playerchat_returns_own_chat() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyPlayerchat::new(packetver);
        pkt.set_msg("Me : Hi there".to_string());
        pkt.set_msg_raw("Me : Hi there".as_bytes().to_vec());
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::OwnChatMessage { message } => {
                assert_eq!(message, "Me : Hi there");
            }
            other => panic!("expected OwnChatMessage, got {other:?}"),
        }
    }

    #[test]
    fn build_action_request_packet_has_correct_format() {
        let raw = crate::sender::build_action_request_packet(0, 2, 20120307);
        assert_eq!(raw.len(), 7);
        // PacketCzRequestAct at packetver>=20120307 uses 0x0885
        assert_eq!(raw[0], 0x85);
        assert_eq!(raw[1], 0x08);
        // target_gid = 0 at offset 2
        assert_eq!(&raw[2..6], &[0, 0, 0, 0]);
        // action = 2 (sit) at offset 6
        assert_eq!(raw[6], 2);
    }

    #[test]
    fn build_chat_packet_has_correct_format() {
        let raw = crate::sender::build_chat_packet("Player : hello", 20120307);
        assert_eq!(raw.len(), 19);
        // rAthena uses 0x00F3 for CZ_REQUEST_CHAT
        assert_eq!(raw[0], 0xF3);
        assert_eq!(raw[1], 0x00);
        let pkt_len = i16::from_le_bytes([raw[2], raw[3]]);
        assert_eq!(pkt_len, 19);
        assert_eq!(&raw[4..], b"Player : hello\0");
    }

    #[test]
    fn build_request_time_packet_contains_client_time() {
        let raw = crate::sender::build_request_time_packet(12345, 20120307);
        assert_eq!(raw.len(), 6);
        let client_time = u32::from_le_bytes([raw[2], raw[3], raw[4], raw[5]]);
        assert_eq!(client_time, 12345);
    }

    #[test]
    fn build_char_ping_packet_contains_account_id() {
        let raw = crate::sender::build_char_ping_packet(200_000, 20120307);
        assert_eq!(raw.len(), 6);
        let aid = u32::from_le_bytes([raw[2], raw[3], raw[4], raw[5]]);
        assert_eq!(aid, 200_000);
    }

    #[test]
    fn build_reqname_packet_has_correct_format() {
        let raw = crate::sender::build_reqname_packet(12345, 20120307);
        assert_eq!(raw.len(), 6);
        let aid = u32::from_le_bytes([raw[2], raw[3], raw[4], raw[5]]);
        assert_eq!(aid, 12345);
    }

    #[test]
    fn dispatch_ack_reqname_returns_entity_name_received() {
        let packetver = 20120307;
        let mut pkt = PacketZcAckReqname::new(packetver);
        pkt.set_aid(42);
        let mut name = ['\0'; 24];
        for (i, c) in "Poring".chars().enumerate() {
            name[i] = c;
        }
        pkt.set_cname(name);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntityNameReceived { gid, name } => {
                assert_eq!(*gid, 42);
                assert_eq!(name, "Poring");
            }
            other => panic!("expected EntityNameReceived, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_par_change_returns_parameter_changed() {
        let packetver = 20120307;
        let mut pkt = PacketZcParChange::new(packetver);
        pkt.set_var_id(5); // HP
        pkt.set_count(441);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::ParameterChanged { var_id, value } => {
                assert_eq!(*var_id, 5);
                assert_eq!(*value, 441);
            }
            other => panic!("expected ParameterChanged, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_sprite_change2_returns_entity_sprite_changed() {
        let packetver = 20120307;
        let mut pkt = PacketZcSpriteChange2::new(packetver);
        pkt.set_gid(150000);
        pkt.set_atype(2); // weapon
        pkt.set_value(1);
        pkt.set_value2(0);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntitySpriteChanged { gid, sprite_type, value, value2 } => {
                assert_eq!(*gid, 150000);
                assert_eq!(*sprite_type, 2);
                assert_eq!(*value, 1);
                assert_eq!(*value2, 0);
            }
            other => panic!("expected EntitySpriteChanged, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_attack_range_returns_attack_range_changed() {
        let packetver = 20120307;
        let mut pkt = PacketZcAttackRange::new(packetver);
        pkt.set_current_att_range(2);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::AttackRangeChanged { range } => assert_eq!(*range, 2),
            other => panic!("expected AttackRangeChanged, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_acknowledged_packets_return_acknowledged() {
        let packetver = 20120307;

        let mut pkt = PacketZcAid::new(packetver);
        pkt.set_aid(200000);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], GameEvent::Acknowledged));

        let mut pkt = PacketZcNotifyMapproperty::new(packetver);
        pkt.set_atype(0);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], GameEvent::Acknowledged));
    }

    #[test]
    fn dispatch_useskill_ack2_returns_skill_casting() {
        let packetver = 20120307;
        let mut pkt = PacketZcUseskillAck2::new(packetver);
        pkt.set_aid(150000);
        pkt.set_target_id(200000);
        pkt.set_skid(10);
        pkt.set_delay_time(2000);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::SkillCasting { gid, target_gid, skill_id, delay_ms } => {
                assert_eq!(*gid, 150000);
                assert_eq!(*target_gid, 200000);
                assert_eq!(*skill_id, 10);
                assert_eq!(*delay_ms, 2000);
            }
            other => panic!("expected SkillCasting, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_notify_hp_to_groupm_returns_entity_hp_changed() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyHpToGroupm::new(packetver);
        pkt.set_aid(42);
        pkt.set_hp(350);
        pkt.set_maxhp(500);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntityHpChanged { gid, hp, max_hp } => {
                assert_eq!(*gid, 42);
                assert_eq!(*hp, 350);
                assert_eq!(*max_hp, 500);
            }
            other => panic!("expected EntityHpChanged, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_emotion_returns_entity_emotion() {
        let packetver = 20120307;
        let mut pkt = PacketZcEmotion::new(packetver);
        pkt.set_gid(42);
        pkt.set_atype(1);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntityEmotion { gid, emotion_type } => {
                assert_eq!(*gid, 42);
                assert_eq!(*emotion_type, 1);
            }
            other => panic!("expected EntityEmotion, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_say_dialog_returns_npc_dialog_text() {
        let packetver = 20120307;
        let mut pkt = PacketZcSayDialog::new(packetver);
        pkt.set_naid(500);
        pkt.set_msg("Hello traveler!\0".to_string());
        pkt.set_msg_raw("Hello traveler!\0".as_bytes().to_vec());
        pkt.set_packet_length((8 + 16) as i16);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::NpcDialogText { npc_id, text } => {
                assert_eq!(*npc_id, 500);
                assert_eq!(text, "Hello traveler!");
            }
            other => panic!("expected NpcDialogText, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_menu_list_splits_items() {
        let packetver = 20120307;
        let mut pkt = PacketZcMenuList::new(packetver);
        pkt.set_naid(500);
        let msg = "Buy:Sell:Cancel\0";
        pkt.set_msg(msg.to_string());
        pkt.set_msg_raw(msg.as_bytes().to_vec());
        pkt.set_packet_length((8 + msg.len()) as i16);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::NpcDialogMenu { npc_id, items } => {
                assert_eq!(*npc_id, 500);
                assert_eq!(items, &["Buy", "Sell", "Cancel"]);
            }
            other => panic!("expected NpcDialogMenu, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_wait_and_close_dialog() {
        let packetver = 20120307;

        let mut pkt = PacketZcWaitDialog::new(packetver);
        pkt.set_naid(500);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        assert!(matches!(&result[0], GameEvent::NpcDialogNext { npc_id: 500 }));

        let mut pkt = PacketZcCloseDialog::new(packetver);
        pkt.set_naid(500);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        assert!(matches!(&result[0], GameEvent::NpcDialogClose { npc_id: 500 }));
    }
}
