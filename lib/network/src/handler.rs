use models::enums::EnumWithNumberValue;
use models::enums::action::ActionType;
use models::enums::skill::SkillTargetType;
use models::enums::skill_enums::SkillEnum;
use models::enums::status::StatusTypes;
use models::enums::vanish::VanishType;
use packets::packets::*;
use ragnarok_game::banner::BannerKind;
use ragnarok_game::event::{
    CharacterInfo, FriendData, GameEvent, HomunculusProperty, MercenaryInfo, PartyMemberData,
    SelfConfigKind, ServerInfo, SkillInfo,
};
use ragnarok_game::guild::{
    GuildBanEntry, GuildMember, GuildPosition, GuildRelation, GuildSkill, OtherGuild,
};
use ragnarok_game::inventory::{EquipmentItemData, NormalItemData};
use ragnarok_game::targeting::{MapKind, MapProperties};
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
    if let Some(p) = any.downcast_ref::<PacketHcAcceptMakecharNeoUnion>() {
        let character = CharacterInfo::from_neo_union(&p.charinfo, packetver);
        return vec![GameEvent::CharacterCreated { character }];
    }
    if let Some(p) = any.downcast_ref::<PacketHcRefuseMakechar>() {
        return vec![GameEvent::CharacterCreateFailed {
            error_code: p.error_code,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketHcDeleteChar3Reserved>() {
        return vec![GameEvent::CharacterDeleteReserved {
            gid: p.gid,
            result: p.result as u32,
            delete_reserved_date: p.delete_reserved_date,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketHcDeleteChar3>() {
        return vec![GameEvent::CharacterDeleted {
            gid: p.gid,
            result: p.result as u32,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketHcDeleteChar3Cancel>() {
        return vec![GameEvent::CharacterDeleteCancelled {
            gid: p.gid,
            result: p.result as u32,
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
    if let Some(p) = any.downcast_ref::<PacketZcAckReqDisconnect>() {
        return vec![GameEvent::DisconnectAck {
            allowed: p.result == 0,
        }];
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
        return vec![GameEvent::ServerTick {
            server_tick: p.time,
            local_send_time_ms: 0,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcHighjump>() {
        return vec![GameEvent::EntityHighJumped {
            gid: p.aid,
            x: p.x_pos as u16,
            y: p.y_pos as u16,
        }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcNotifyStandentry7>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return vec![GameEvent::EntitySpawned {
            gid: p.gid,
            aid: p.aid,
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
            x,
            y,
            direction: dir,
            body_state: p.body_state,
            health_state: p.health_state,
            effect_state: p.effect_state,
            base_level: p.clevel,
            is_boss: p.is_boss,
            posture: p.state,
            guild_id: p.guid,
            guild_emblem_version: p.gemblem_ver as i32,
            is_new_entry: false,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyNewentry7>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return vec![GameEvent::EntitySpawned {
            gid: p.gid,
            aid: p.aid,
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
            x,
            y,
            direction: dir,
            body_state: p.body_state,
            health_state: p.health_state,
            effect_state: p.effect_state,
            base_level: p.clevel,
            is_boss: p.is_boss,
            posture: 0,
            guild_id: p.guid,
            guild_emblem_version: p.gemblem_ver as i32,
            is_new_entry: true,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyStandentry>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return vec![GameEvent::EntitySpawned {
            gid: p.gid,
            aid: p.gid,
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
            x,
            y,
            direction: dir,
            body_state: p.body_state,
            health_state: p.health_state,
            effect_state: p.effect_state as i32,
            base_level: p.clevel,
            is_boss: false,
            posture: p.state,
            guild_id: p.guid,
            guild_emblem_version: p.gemblem_ver as i32,
            is_new_entry: false,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyNewentry>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return vec![GameEvent::EntitySpawned {
            gid: p.gid,
            aid: p.gid,
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
            x,
            y,
            direction: dir,
            body_state: p.body_state,
            health_state: p.health_state,
            effect_state: p.effect_state as i32,
            base_level: p.clevel,
            is_boss: false,
            posture: 0,
            guild_id: p.guid,
            guild_emblem_version: p.gemblem_ver as i32,
            is_new_entry: true,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyMoveentry8>() {
        let (x, y, dir) = decode_pos(&p.pos_dir);
        return vec![GameEvent::EntitySpawned {
            gid: p.gid,
            aid: p.aid,
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
            x,
            y,
            direction: dir,
            body_state: p.body_state,
            health_state: p.health_state,
            effect_state: p.effect_state,
            base_level: p.clevel,
            is_boss: p.is_boss,
            posture: 0,
            guild_id: p.guid,
            guild_emblem_version: p.gemblem_ver as i32,
            is_new_entry: false,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyMoveentry9>() {
        let (x1, y1, x2, y2) = decode_pos2(&p.move_data);
        let direction =
            ragnarok_game::movement::direction_from_positions(x1, y1, x2, y2).unwrap_or(0);
        return vec![
            GameEvent::EntitySpawned {
                gid: p.gid,
                aid: p.gid,
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
                x: x1,
                y: y1,
                direction,
                body_state: p.body_state,
                health_state: p.health_state,
                effect_state: p.effect_state as i32,
                base_level: p.clevel,
                is_boss: p.is_boss,
                posture: 0,
                guild_id: p.guid,
                guild_emblem_version: p.gemblem_ver as i32,
                is_new_entry: false,
            },
            GameEvent::EntityMoved {
                gid: p.gid,
                start_x: x1,
                start_y: y1,
                dest_x: x2,
                dest_y: y2,
                start_time: p.move_start_time,
            },
        ];
    }

    if let Some(p) = any.downcast_ref::<PacketZcNotifyMove>() {
        let (x1, y1, x2, y2) = decode_pos2(&p.move_data);
        return vec![GameEvent::EntityMoved {
            gid: p.gid,
            start_x: x1,
            start_y: y1,
            dest_x: x2,
            dest_y: y2,
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
    if let Some(p) = any.downcast_ref::<PacketZcFastmove>() {
        return vec![GameEvent::EntityStopMove {
            gid: p.aid,
            x: p.x_pos as u16,
            y: p.y_pos as u16,
        }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcNotifyAct>() {
        return vec![GameEvent::EntityAction {
            gid: p.gid,
            target_gid: p.target_gid,
            action: ActionType::from_value(p.action as usize),
            damage: p.damage as i32,
            left_damage: p.left_damage as i32,
            attack_mt: p.attack_mt,
            attacked_mt: p.attacked_mt,
            start_time: p.start_time,
            count: p.count,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyAct2>() {
        return vec![GameEvent::EntityAction {
            gid: p.gid,
            target_gid: p.target_gid,
            action: ActionType::from_value(p.action as usize),
            damage: p.damage,
            left_damage: p.left_damage,
            attack_mt: p.attack_mt,
            attacked_mt: p.attacked_mt,
            start_time: p.start_time,
            count: p.count,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyAct3>() {
        return vec![GameEvent::EntityAction {
            gid: p.gid,
            target_gid: p.target_gid,
            action: ActionType::from_value(p.action as usize),
            damage: p.damage,
            left_damage: p.left_damage,
            attack_mt: p.attack_mt,
            attacked_mt: p.attacked_mt,
            start_time: p.start_time,
            count: p.count,
        }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcChangeDirection>() {
        return vec![GameEvent::EntityDirectionChanged {
            gid: p.aid,
            head_dir: p.head_dir as u8,
            dir: p.dir,
        }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcNotifyChat>() {
        let message: String = p.msg.chars().take_while(|c| *c != '\0').collect();
        return vec![GameEvent::ChatMessage {
            gid: p.gid,
            message,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyPlayerchat>() {
        let message: String = p.msg.chars().take_while(|c| *c != '\0').collect();
        return vec![GameEvent::OwnChatMessage { message }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcBroadcast>() {
        let raw: String = p.msg.chars().take_while(|c| *c != '\0').collect();
        let (message, color) = parse_broadcast(&raw);
        let (message, banner) = classify_banner(message);
        return vec![GameEvent::BroadcastMessage {
            message,
            color,
            banner,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcBroadcast2>() {
        let message: String = p.msg.chars().take_while(|c| *c != '\0').collect();
        return vec![GameEvent::BroadcastMessage {
            message,
            color: rgb_u32_to_rgba(p.font_color),
            banner: BannerKind::None,
        }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcAckReqname>() {
        let name: String = p.cname.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::EntityNameReceived { gid: p.aid, name }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcAckReqnameall>() {
        let name: String = p.cname.iter().take_while(|c| **c != '\0').collect();
        let guild_name: String = p.gname.iter().take_while(|c| **c != '\0').collect();
        let position_name: String = p.rname.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::EntityNamesReceived {
            gid: p.aid,
            name,
            guild_name,
            position_name,
        }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcNotifyVanish>() {
        return vec![GameEvent::EntityVanished {
            gid: p.gid,
            vanish_type: VanishType::from_value(p.atype as usize),
        }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcNotifyEffect2>() {
        return vec![GameEvent::PlayEffectOnEntity {
            gid: p.aid,
            effect_id: p.effect_id,
            value: None,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyEffect3>() {
        return vec![GameEvent::PlayEffectOnEntity {
            gid: p.aid,
            effect_id: p.effect_id,
            value: Some(p.numdata),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyEffect>() {
        return vec![GameEvent::PlayMiscEffectOnEntity {
            gid: p.aid,
            code: p.effect_id as u8,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcSpirits>() {
        return vec![GameEvent::SpiritsChanged {
            gid: p.aid,
            count: p.num.max(0) as u8,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcSpirits2>() {
        return vec![GameEvent::SpiritsChanged {
            gid: p.aid,
            count: p.num.max(0) as u8,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcBladestop>() {
        return vec![GameEvent::BladeStop {
            src_gid: p.src_aid,
            dest_gid: p.dest_aid,
            active: p.flag != 0,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcResurrection>() {
        return vec![GameEvent::EntityResurrected { gid: p.aid }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcRecovery>() {
        return vec![GameEvent::Recovery {
            var_id: p.var_id as u16,
            amount: p.amount as i32,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcSound>() {
        let name: String = p.file_name.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::SoundEffect {
            name,
            act: p.act,
            term_ms: p.term,
            gid: p.naid,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcMvp>() {
        return vec![GameEvent::MvpReward { gid: p.aid }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcParChange>() {
        return vec![GameEvent::ParameterChanged {
            var_id: p.var_id,
            value: p.count,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcLongparChange>() {
        return vec![GameEvent::ParameterChanged {
            var_id: p.var_id,
            value: p.amount,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyExp>() {
        return vec![GameEvent::ExpGained {
            aid: p.aid,
            amount: p.amount,
            is_base: p.var_id as usize == StatusTypes::Baseexp.value(),
            is_quest: p.exp_type == 1,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcStatusValues>() {
        return vec![GameEvent::StatusChanged {
            status_type: p.status_type,
            base: p.default_status,
            bonus: p.plus_status,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcStatus>() {
        return vec![GameEvent::ParameterChanged {
            var_id: 9, // StatusTypes::Statuspoint
            value: p.point as i32,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAttackRange>() {
        return vec![GameEvent::AttackRangeChanged {
            range: p.current_att_range,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcSpriteChange2>() {
        return vec![GameEvent::EntitySpriteChanged {
            gid: p.gid,
            sprite_type: p.atype,
            value: p.value,
            value2: p.value2,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNpcspriteChange>() {
        return vec![GameEvent::EntitySpriteChanged {
            gid: p.gid,
            sprite_type: 0,
            value: p.value as u16,
            value2: 0,
        }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcUseskillAck2>() {
        let name = SkillEnum::from_id(p.skid as u32).to_name().to_string();
        return vec![GameEvent::SkillCasting {
            gid: p.aid,
            target_gid: p.target_id,
            skill_id: p.skid,
            property: p.property,
            delay_ms: p.delay_time,
            x: p.x_pos,
            y: p.y_pos,
            skill_name: Some(name),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcUseskillAck>() {
        let name = SkillEnum::from_id(p.skid as u32).to_name().to_string();
        return vec![GameEvent::SkillCasting {
            gid: p.aid,
            target_gid: p.target_id,
            skill_id: p.skid,
            property: p.property,
            delay_ms: p.delay_time,
            x: p.x_pos,
            y: p.y_pos,
            skill_name: Some(name),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckTouseskill>() {
        if !p.result {
            return vec![GameEvent::SkillFailed {
                skill_id: p.skid,
                cause: p.cause,
            }];
        }
        return vec![GameEvent::Acknowledged];
    }
    if let Some(p) = any.downcast_ref::<PacketZcSkillPostdelay>() {
        return vec![GameEvent::SkillPostDelay {
            skill_id: p.skid,
            delay_ms: p.delay_tm,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcMsgStateChange2>() {
        if p.index == 46 && p.state {
            return vec![GameEvent::AfterCastDelay {
                delay_ms: p.remain_ms,
            }];
        }
        return vec![GameEvent::StatusEffectChanged {
            gid: p.aid,
            efst: p.index,
            active: p.state,
            remain_ms: p.remain_ms,
            val1: p.val[0],
        }];
    }
    // 0x196: always the status-OFF packet (flag=0); without this arm buffs never clear on early removal.
    if let Some(p) = any.downcast_ref::<PacketZcMsgStateChange>() {
        return vec![GameEvent::StatusEffectChanged {
            gid: p.aid,
            efst: p.index,
            active: p.state,
            remain_ms: 0,
            val1: 0,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcStateChange3>() {
        return vec![GameEvent::EntityOptionChanged {
            gid: p.aid,
            body_state: p.body_state,
            health_state: p.health_state,
            effect_state: p.effect_state,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifySkill>() {
        return vec![GameEvent::SkillDamage {
            skill_id: p.skid,
            src_gid: p.aid,
            target_gid: p.target_id,
            damage: p.damage as i32,
            attack_mt: p.attack_mt,
            attacked_mt: p.attacked_mt,
            count: p.count,
            level: p.level,
            action: ActionType::from_value(p.action as usize),
            start_time: p.start_time,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifySkill2>() {
        return vec![GameEvent::SkillDamage {
            skill_id: p.skid,
            src_gid: p.aid,
            target_gid: p.target_id,
            damage: p.damage,
            attack_mt: p.attack_mt,
            attacked_mt: p.attacked_mt,
            count: p.count,
            level: p.level,
            action: ActionType::from_value(p.action as usize),
            start_time: p.start_time,
        }];
    }
    if let Some(_p) = any.downcast_ref::<PacketZcProgressCancel>() {
        return vec![GameEvent::SkillCastCancel { gid: 0 }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcDispel>() {
        return vec![GameEvent::SkillCastCancel { gid: p.aid }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcUseSkill>() {
        if p.result {
            return vec![GameEvent::SkillNoDamage {
                skill_id: p.skid,
                src_gid: p.src_aid,
                target_gid: p.target_aid,
                level: p.level,
            }];
        }
        return vec![GameEvent::Acknowledged];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyGroundskill>() {
        return vec![GameEvent::GroundSkill {
            skill_id: p.skid,
            src_gid: p.aid,
            level: p.level,
            x: p.x_pos,
            y: p.y_pos,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcSkillEntry>() {
        return vec![GameEvent::SkillUnitEntered {
            aid: p.aid,
            creator_aid: p.creator_aid,
            x: p.x_pos,
            y: p.y_pos,
            unit_id: p.job,
            is_visible: p.is_visible,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcSkillDisappear>() {
        return vec![GameEvent::SkillUnitDisappeared { aid: p.aid }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcSkillUpdate>() {
        return vec![GameEvent::SkillUnitUpdated { aid: p.aid }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcEmotion>() {
        return vec![GameEvent::EntityEmotion {
            gid: p.gid,
            emotion_type: p.atype,
        }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcNotifyHpToGroupm>() {
        return vec![
            GameEvent::EntityHpChanged {
                gid: p.aid,
                hp: p.hp as u32,
                max_hp: p.maxhp as u32,
            },
            GameEvent::PartyMemberHp {
                aid: p.aid,
                hp: p.hp as u32,
                max_hp: p.maxhp as u32,
            },
        ];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyHpToGroupmR2>() {
        return vec![
            GameEvent::EntityHpChanged {
                gid: p.aid,
                hp: p.hp as u32,
                max_hp: p.maxhp as u32,
            },
            GameEvent::PartyMemberHp {
                aid: p.aid,
                hp: p.hp as u32,
                max_hp: p.maxhp as u32,
            },
        ];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyPositionToGroupm>() {
        return vec![GameEvent::PartyMemberPosition {
            aid: p.aid,
            x: p.x_pos as u16,
            y: p.y_pos as u16,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyPositionToGuildm>() {
        return vec![GameEvent::GuildMemberPosition {
            aid: p.aid,
            x: p.x_pos as u16,
            y: p.y_pos as u16,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckMakeGroup>() {
        return vec![GameEvent::PartyCreateResult { result: p.result }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcGroupList>() {
        let name: String = p.group_name.iter().take_while(|c| **c != '\0').collect();
        let members = p
            .group_info
            .iter()
            .map(|m| PartyMemberData {
                aid: m.aid,
                name: m.character_name.iter().take_while(|c| **c != '\0').collect(),
                map: m.map_name.iter().take_while(|c| **c != '\0').collect(),
                leader: m.role == 0,
                online: m.state == 0,
            })
            .collect();
        return vec![GameEvent::PartyMemberList { name, members }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAddMemberToGroup2>() {
        let name: String = p
            .character_name
            .iter()
            .take_while(|c| **c != '\0')
            .collect();
        let map: String = p.map_name.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::PartyMemberAdded {
            aid: p.aid,
            name,
            map,
            leader: p.role == 0,
            online: p.state == 0,
            x: p.x_pos as u16,
            y: p.y_pos as u16,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAddMemberToGroup>() {
        let name: String = p
            .character_name
            .iter()
            .take_while(|c| **c != '\0')
            .collect();
        let map: String = p.map_name.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::PartyMemberAdded {
            aid: p.aid,
            name,
            map,
            leader: p.role == 0,
            online: p.state == 0,
            x: p.x_pos as u16,
            y: p.y_pos as u16,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcDeleteMemberFromGroup>() {
        let name: String = p
            .character_name
            .iter()
            .take_while(|c| **c != '\0')
            .collect();
        return vec![GameEvent::PartyMemberRemoved {
            aid: p.aid,
            name,
            result: p.result,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPartyConfig>() {
        return vec![GameEvent::SelfConfigChanged {
            kind: SelfConfigKind::RefusePartyInvite,
            enabled: p.b_refuse_join_msg,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcConfigNotify>() {
        return vec![GameEvent::SelfConfigChanged {
            kind: SelfConfigKind::OpenEquipmentWindow,
            enabled: p.b_open_equipment_win,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcConfig>() {
        let kind = match p.config {
            0 => SelfConfigKind::OpenEquipmentWindow,
            1 => SelfConfigKind::Call,
            2 => SelfConfigKind::PetAutofeed,
            3 => SelfConfigKind::HomunculusAutofeed,
            other => {
                debug!("unknown ZC_CONFIG type: {other}");
                return vec![];
            }
        };
        return vec![GameEvent::SelfConfigChanged {
            kind,
            enabled: p.value != 0,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPartyJoinReq>() {
        let party_name: String = p.group_name.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::PartyInviteReceived {
            party_grid: p.grid,
            party_name,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcReqJoinGroup>() {
        let party_name: String = p.group_name.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::PartyInviteReceived {
            party_grid: p.grid,
            party_name,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPartyJoinReqAck>() {
        let name: String = p
            .character_name
            .iter()
            .take_while(|c| **c != '\0')
            .collect();
        return vec![GameEvent::PartyInviteResult {
            name,
            answer: p.answer as u8,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckReqJoinGroup>() {
        let name: String = p
            .character_name
            .iter()
            .take_while(|c| **c != '\0')
            .collect();
        return vec![GameEvent::PartyInviteResult {
            name,
            answer: p.answer,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcGroupinfoChange>() {
        return vec![GameEvent::PartyExpOptionChanged {
            exp_option: p.exp_option,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyChatParty>() {
        let message: String = p.msg.chars().take_while(|c| *c != '\0').collect();
        return vec![GameEvent::PartyChatMessage {
            aid: p.aid,
            message,
        }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcAckGuildMenuinterface>() {
        return vec![GameEvent::GuildMenuFlag {
            flag: p.guild_memu_flag,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcGuildInfo2>() {
        return vec![GameEvent::GuildInfo {
            gdid: p.gdid as u32,
            name: cstr(&p.guildname),
            level: p.level,
            exp: p.exp,
            max_exp: p.max_exp,
            member_num: p.user_num,
            max_member_num: p.max_user_num,
            avg_level: p.user_average_level,
            point: p.point,
            honor: p.honor,
            virtue: p.virtue,
            master_name: cstr(&p.master_name),
            manage_land: cstr(&p.manage_land),
            emblem_version: p.emblem_version,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcGuildInfo>() {
        return vec![GameEvent::GuildInfo {
            gdid: p.gdid as u32,
            name: cstr(&p.guildname),
            level: p.level,
            exp: p.exp,
            max_exp: p.max_exp,
            member_num: p.user_num,
            max_member_num: p.max_user_num,
            avg_level: p.user_average_level,
            point: p.point,
            honor: p.honor,
            virtue: p.virtue,
            master_name: cstr(&p.master_name),
            manage_land: cstr(&p.manage_land),
            emblem_version: p.emblem_version,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcMembermgrInfo>() {
        let members = p
            .member_info
            .iter()
            .map(|m| GuildMember {
                aid: m.aid,
                gid: m.gid,
                name: cstr(&m.char_name),
                job: m.job,
                level: m.level,
                head: m.head_type,
                head_palette: m.head_palette,
                sex: m.sex,
                position_id: m.gposition_id,
                position_name: String::new(),
                contribution_exp: m.member_exp,
                online: m.current_state != 0,
                note: cstr(&m.memo),
                cur_map: String::new(),
                last_offline: 0,
                x: 0,
                y: 0,
                has_live_position: false,
            })
            .collect();
        return vec![GameEvent::GuildMembers { members }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPositionInfo>() {
        let positions = p
            .member_info
            .iter()
            .map(|pos| GuildPosition {
                id: pos.position_id,
                name: String::new(),
                right: pos.right,
                ranking: pos.ranking,
                pay_rate: pos.pay_rate,
            })
            .collect();
        return vec![GameEvent::GuildPositions { positions }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckChangeGuildPositioninfo>() {
        let positions = p
            .member_list
            .iter()
            .map(|pos| GuildPosition {
                id: pos.position_id,
                name: String::new(),
                right: pos.right,
                ranking: pos.ranking,
                pay_rate: pos.pay_rate,
            })
            .collect();
        let names = p
            .member_list
            .iter()
            .map(|pos| (pos.position_id, cstr(&pos.pos_name)))
            .collect();
        return vec![
            GameEvent::GuildPositions { positions },
            GameEvent::GuildPositionNames { names },
        ];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckReqChangeMembers>() {
        let entries = p
            .member_info
            .iter()
            .map(|m| (m.aid as u32, m.gid as u32, m.position_id))
            .collect();
        return vec![GameEvent::GuildMemberPositionsChanged { entries }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPositionIdNameInfo>() {
        let names = p
            .member_list
            .iter()
            .map(|pos| (pos.position_id, cstr(&pos.pos_name)))
            .collect();
        return vec![GameEvent::GuildPositionNames { names }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcGuildSkillinfo>() {
        let skills = parse_skill_info_list(&p.skill_list)
            .into_iter()
            .map(|s| GuildSkill {
                skid: s.id,
                name: s.name,
                level: s.level,
                sp_cost: s.sp_cost,
                attack_range: s.attack_range,
                upgradable: s.upgradable,
                passive: matches!(s.skill_target_type, SkillTargetType::Passive),
            })
            .collect();
        return vec![GameEvent::GuildSkills {
            point: p.skill_point,
            skills,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcBanList>() {
        return vec![GameEvent::GuildBanList {
            entries: parse_ban_list(&p.raw),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcGuildNotice>() {
        return vec![GameEvent::GuildNotice {
            subject: cstr(&p.subject),
            body: cstr(&p.notice),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcOtherGuildList>() {
        let guilds = p
            .guild_list
            .iter()
            .map(|g| OtherGuild {
                name: cstr(&g.guildname),
                level: g.guild_level,
                member_size: g.guild_member_size,
                ranking: g.guild_ranking,
            })
            .collect();
        return vec![GameEvent::GuildOtherList { guilds }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcMyguildBasicInfo>() {
        let relations = p
            .related_guild_list
            .iter()
            .map(|r| GuildRelation {
                // wire order is <relation>.L <gdid>.L; the generated struct swaps them
                gdid: r.relation,
                name: cstr(&r.guild_name),
                relation: r.gdid,
            })
            .collect();
        return vec![GameEvent::GuildRelations { relations }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcGuildEmblemImg>() {
        return vec![GameEvent::GuildEmblem {
            gdid: p.gdid as u32,
            version: p.emblem_version,
            bmp: p.img_raw.clone(),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcUpdateGdid>() {
        return vec![GameEvent::GuildIdentityUpdated {
            gdid: p.gdid,
            emblem_version: p.emblem_version,
            right: p.right,
            is_master: p.is_master,
            name: cstr(&p.gname),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcResultMakeGuild>() {
        return vec![GameEvent::GuildCreateResult { result: p.result }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckLeaveGuild>() {
        return vec![GameEvent::GuildMemberLeft {
            name: cstr(&p.char_name),
            reason: cstr(&p.reason_desc),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckBanGuild>() {
        return vec![GameEvent::GuildMemberExpelled {
            name: cstr(&p.char_name),
            reason: cstr(&p.reason_desc),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckBanGuildSso>() {
        return vec![GameEvent::GuildMemberExpelled {
            name: cstr(&p.char_name),
            reason: cstr(&p.reason_desc),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcChangeGuild>() {
        return vec![GameEvent::EntityGuildChanged {
            aid: p.aid,
            gdid: p.gdid,
            emblem_version: p.emblem_version as i32,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckDisorganizeGuildResult>() {
        return vec![GameEvent::GuildDisbandResult { reason: p.reason }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcReqJoinGuild>() {
        return vec![GameEvent::GuildInviteReceived {
            gdid: p.gdid,
            name: cstr(&p.guild_name),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcReqAllyGuild>() {
        return vec![GameEvent::GuildAllyRequestReceived {
            aid: p.other_aid,
            name: cstr(&p.guild_name),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckReqAllyGuild>() {
        return vec![GameEvent::GuildAllyResult { answer: p.answer }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckReqHostileGuild>() {
        return vec![GameEvent::GuildHostileResult { result: p.result }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckReqJoinGuild>() {
        return vec![GameEvent::GuildJoinResult { answer: p.answer }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcDeleteRelatedGuild>() {
        return vec![GameEvent::GuildRelationDeleted {
            gdid: p.opponent_gdid,
            relation: p.relation,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAddRelatedGuild>() {
        return vec![GameEvent::GuildRelationAdded {
            gdid: p.info.gdid as u32,
            relation: p.info.relation,
            name: cstr(&p.info.guildname),
        }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcSayDialog>() {
        let text: String = p.msg.chars().take_while(|c| *c != '\0').collect();
        return vec![GameEvent::NpcDialogText {
            npc_id: p.naid,
            text,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcWaitDialog>() {
        return vec![GameEvent::NpcDialogNext { npc_id: p.naid }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcCloseDialog>() {
        return vec![GameEvent::NpcDialogClose { npc_id: p.naid }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcMenuList>() {
        let raw_msg: String = p.msg.chars().take_while(|c| *c != '\0').collect();
        let items: Vec<String> = raw_msg
            .split(':')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        return vec![GameEvent::NpcDialogMenu {
            npc_id: p.naid,
            items,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcRoomNewentry>() {
        let title: String = p.title.chars().take_while(|c| *c != '\0').collect();
        return vec![GameEvent::ChatRoomUpsert {
            owner_aid: p.aid,
            room_id: p.room_id,
            max_count: p.maxcount,
            cur_count: p.curcount,
            atype: p.atype,
            title,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcChangeChatroom>() {
        let title: String = p.title.chars().take_while(|c| *c != '\0').collect();
        return vec![GameEvent::ChatRoomUpsert {
            owner_aid: p.aid,
            room_id: p.room_id,
            max_count: p.maxcount,
            cur_count: p.curcount,
            atype: p.atype,
            title,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcDestroyRoom>() {
        return vec![GameEvent::ChatRoomDestroy { room_id: p.room_id }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcEnterRoom>() {
        return vec![GameEvent::ChatRoomEntered { room_id: p.room_id }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcRefuseEnterRoom>() {
        return vec![GameEvent::ChatRoomJoinRefused { result: p.result }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcWarplist>() {
        // Parse raw to stay independent of the generated array field shape: id(2)+SKID(2)+4×16 bytes.
        let raw = p.raw();
        let mut destinations = Vec::new();
        for i in 0..4 {
            let start = 4 + i * 16;
            if start + 16 > raw.len() {
                break;
            }
            let name: String = raw[start..start + 16]
                .iter()
                .take_while(|&&b| b != 0)
                .map(|&b| b as char)
                .collect();
            if !name.is_empty() {
                destinations.push(name);
            }
        }
        if destinations.is_empty() {
            return vec![];
        }
        return vec![GameEvent::WarpList {
            skill_id: p.skid,
            destinations,
        }];
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

    if let Some(p) = any.downcast_ref::<PacketZcPcPurchaseItemlist>() {
        let items = p
            .item_list
            .iter()
            .map(|item| (item.itid, item.price, item.discountprice, item.atype))
            .collect();
        return vec![GameEvent::NpcShopBuyList { npc_id: 0, items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPcSellItemlist>() {
        let items = p
            .item_list
            .iter()
            .map(|item| (item.index, item.price, item.overchargeprice))
            .collect();
        return vec![GameEvent::NpcShopSellList { npc_id: 0, items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPcPurchaseResult>() {
        return vec![GameEvent::NpcShopBuyResult { result: p.result }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPcSellResult>() {
        return vec![GameEvent::NpcShopSellResult { result: p.result }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcNormalItemlist>() {
        let items = p
            .item_info
            .iter()
            .map(|i| NormalItemData {
                index: i.index,
                item_id: i.itid,
                item_type: i.atype,
                is_identified: i.is_identified,
                count: i.count,
                wear_state: i.wear_state,
            })
            .collect();
        return vec![GameEvent::InventoryNormalItems { items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcEquipmentItemlist>() {
        let items = p
            .item_info
            .iter()
            .map(|i| EquipmentItemData {
                index: i.index,
                item_id: i.itid,
                item_type: i.atype,
                is_identified: i.is_identified,
                location: i.location,
                wear_state: i.wear_state,
                is_damaged: i.is_damaged,
                refining_level: i.refining_level,
                slot: [i.slot.card1, i.slot.card2, i.slot.card3, i.slot.card4],
            })
            .collect();
        return vec![GameEvent::InventoryEquipmentItems { items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcItemPickupAck>() {
        return vec![GameEvent::InventoryItemPickup {
            index: p.index,
            item_id: p.itid,
            count: p.count,
            item_type: p.atype,
            is_identified: p.is_identified,
            is_damaged: p.is_damaged,
            refining_level: p.refining_level,
            slot: [p.slot.card1, p.slot.card2, p.slot.card3, p.slot.card4],
            location: p.location,
            result: p.result,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcItemPickupAck2>() {
        return vec![GameEvent::InventoryItemPickup {
            index: p.index,
            item_id: p.itid,
            count: p.count,
            item_type: p.atype,
            is_identified: p.is_identified,
            is_damaged: p.is_damaged,
            refining_level: p.refining_level,
            slot: [p.slot.card1, p.slot.card2, p.slot.card3, p.slot.card4],
            location: p.location,
            result: p.result,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcItemPickupAck3>() {
        return vec![GameEvent::InventoryItemPickup {
            index: p.index,
            item_id: p.itid,
            count: p.count,
            item_type: p.atype,
            is_identified: p.is_identified,
            is_damaged: p.is_damaged,
            refining_level: p.refining_level,
            slot: [p.slot.card1, p.slot.card2, p.slot.card3, p.slot.card4],
            location: p.location,
            result: p.result,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNormalItemlist2>() {
        let items = p
            .item_info
            .iter()
            .map(|i| NormalItemData {
                index: i.index,
                item_id: i.itid,
                item_type: i.atype,
                is_identified: i.is_identified,
                count: i.count,
                wear_state: i.wear_state,
            })
            .collect();
        return vec![GameEvent::InventoryNormalItems { items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNormalItemlist3>() {
        let items = p
            .item_info
            .iter()
            .map(|i| NormalItemData {
                index: i.index,
                item_id: i.itid,
                item_type: i.atype,
                is_identified: i.is_identified,
                count: i.count,
                wear_state: i.wear_state,
            })
            .collect();
        return vec![GameEvent::InventoryNormalItems { items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcEquipmentItemlist2>() {
        let items = p
            .item_info
            .iter()
            .map(|i| EquipmentItemData {
                index: i.index,
                item_id: i.itid,
                item_type: i.atype,
                is_identified: i.is_identified,
                location: i.location,
                wear_state: i.wear_state,
                is_damaged: i.is_damaged,
                refining_level: i.refining_level,
                slot: [i.slot.card1, i.slot.card2, i.slot.card3, i.slot.card4],
            })
            .collect();
        return vec![GameEvent::InventoryEquipmentItems { items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcEquipmentItemlist3>() {
        let items = p
            .item_info
            .iter()
            .map(|i| EquipmentItemData {
                index: i.index,
                item_id: i.itid,
                item_type: i.atype,
                is_identified: i.is_identified,
                location: i.location,
                wear_state: i.wear_state,
                is_damaged: i.is_damaged,
                refining_level: i.refining_level,
                slot: [i.slot.card1, i.slot.card2, i.slot.card3, i.slot.card4],
            })
            .collect();
        return vec![GameEvent::InventoryEquipmentItems { items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcUseItemAck>() {
        return vec![GameEvent::InventoryUseItemResult {
            index: p.index,
            count: p.count,
            success: p.result,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcUseItemAck2>() {
        return vec![GameEvent::InventoryUseItemResult {
            index: p.index,
            count: p.count,
            success: p.result,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcEquipArrow>() {
        return vec![GameEvent::InventoryArrowEquipped {
            index: p.index as u16,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcReqWearEquipAck>() {
        return vec![GameEvent::InventoryEquipResult {
            index: p.index,
            wear_location: p.wear_location,
            view_id: p.view_id,
            success: p.result == 1,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcReqWearEquipAck2>() {
        return vec![GameEvent::InventoryEquipResult {
            index: p.index,
            wear_location: p.wear_location,
            view_id: p.view_id,
            success: p.result == 0,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcReqTakeoffEquipAck>() {
        return vec![GameEvent::InventoryUnequipResult {
            index: p.index,
            wear_location: p.wear_location,
            success: p.result == 0,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcReqTakeoffEquipAck2>() {
        return vec![GameEvent::InventoryUnequipResult {
            index: p.index,
            wear_location: p.wear_location,
            success: p.result == 0,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcItemThrowAck>() {
        return vec![GameEvent::InventoryItemRemoved {
            index: p.index,
            count: p.count,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcDeleteItemFromBody>() {
        return vec![GameEvent::InventoryItemRemoved {
            index: p.index,
            count: p.count,
        }];
    }

    macro_rules! cart_normal_items {
        ($p:expr) => {{
            let items = $p
                .item_info
                .iter()
                .map(|i| NormalItemData {
                    index: i.index,
                    item_id: i.itid,
                    item_type: i.atype,
                    is_identified: i.is_identified,
                    count: i.count,
                    wear_state: i.wear_state,
                })
                .collect();
            return vec![GameEvent::CartNormalItems { items }];
        }};
    }
    macro_rules! cart_equip_items {
        ($p:expr) => {{
            let items = $p
                .item_info
                .iter()
                .map(|i| EquipmentItemData {
                    index: i.index,
                    item_id: i.itid,
                    item_type: i.atype,
                    is_identified: i.is_identified,
                    location: i.location,
                    wear_state: i.wear_state,
                    is_damaged: i.is_damaged,
                    refining_level: i.refining_level,
                    slot: [i.slot.card1, i.slot.card2, i.slot.card3, i.slot.card4],
                })
                .collect();
            return vec![GameEvent::CartEquipmentItems { items }];
        }};
    }
    if let Some(p) = any.downcast_ref::<PacketZcCartNormalItemlist>() {
        cart_normal_items!(p);
    }
    if let Some(p) = any.downcast_ref::<PacketZcCartNormalItemlist2>() {
        cart_normal_items!(p);
    }
    if let Some(p) = any.downcast_ref::<PacketZcCartNormalItemlist3>() {
        cart_normal_items!(p);
    }
    if let Some(p) = any.downcast_ref::<PacketZcCartEquipmentItemlist>() {
        cart_equip_items!(p);
    }
    if let Some(p) = any.downcast_ref::<PacketZcCartEquipmentItemlist2>() {
        cart_equip_items!(p);
    }
    if let Some(p) = any.downcast_ref::<PacketZcCartEquipmentItemlist3>() {
        cart_equip_items!(p);
    }
    if let Some(p) = any.downcast_ref::<PacketZcAddItemToCart2>() {
        return vec![GameEvent::CartItemAdded {
            index: p.index as u16,
            item_id: p.itid,
            count: p.count as i16,
            item_type: p.atype,
            is_identified: p.is_identified,
            is_damaged: p.is_damaged,
            refining_level: p.refining_level,
            slot: [p.slot.card1, p.slot.card2, p.slot.card3, p.slot.card4],
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAddItemToCart>() {
        return vec![GameEvent::CartItemAdded {
            index: p.index as u16,
            item_id: p.itid,
            count: p.count as i16,
            item_type: 0,
            is_identified: p.is_identified,
            is_damaged: p.is_damaged,
            refining_level: p.refining_level,
            slot: [p.slot.card1, p.slot.card2, p.slot.card3, p.slot.card4],
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcDeleteItemFromCart>() {
        return vec![GameEvent::CartItemRemoved {
            index: p.index as u16,
            count: p.count as i16,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyCartitemCountinfo>() {
        return vec![GameEvent::CartCountInfo {
            cur_weight: p.cur_weight,
            max_weight: p.max_weight,
            cur_count: p.cur_count,
            max_count: p.max_count,
        }];
    }
    if any.downcast_ref::<PacketZcCartoff>().is_some() {
        return vec![GameEvent::CartOff];
    }

    if let Some(p) = any.downcast_ref::<PacketZcItemcompositionList>() {
        return vec![GameEvent::CardInsertItemList {
            card_index: 0,
            equip_indices: p.itidlist.clone(),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckItemcomposition>() {
        return vec![GameEvent::CardInsertResult {
            equip_index: p.equip_index as u16,
            card_index: p.card_index as u16,
            result: p.result,
        }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcItemFallEntry>() {
        return vec![GameEvent::FloorItemAppeared {
            id: p.itaid,
            item_id: p.itid,
            is_identified: p.is_identified,
            x: p.x_pos,
            y: p.y_pos,
            sub_x: p.sub_x,
            sub_y: p.sub_y,
            count: p.count,
            is_falling: true,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcItemEntry>() {
        return vec![GameEvent::FloorItemAppeared {
            id: p.itaid,
            item_id: p.itid,
            is_identified: p.is_identified,
            x: p.x_pos,
            y: p.y_pos,
            sub_x: p.sub_x,
            sub_y: p.sub_y,
            count: p.count,
            is_falling: false,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcItemDisappear>() {
        return vec![GameEvent::FloorItemDisappeared { id: p.itaid }];
    }

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
    if let Some(p) = any.downcast_ref::<PacketZcFriendsList>() {
        let friends = p
            .friend_list
            .iter()
            .map(|f| FriendData {
                aid: f.aid,
                gid: f.gid,
                name: f.name.iter().take_while(|c| **c != '\0').collect(),
                online: true,
            })
            .collect();
        return vec![GameEvent::FriendListReceived { friends }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcFriendsState>() {
        return vec![GameEvent::FriendStateChanged {
            aid: p.aid,
            gid: p.gid,
            online: !p.state,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAddFriendsList>() {
        let name: String = p.name.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::FriendAddResult {
            result: p.result as u8,
            aid: p.aid,
            gid: p.gid,
            name,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcReqAddFriends>() {
        let name: String = p.name.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::FriendRequestReceived {
            req_aid: p.req_aid,
            req_gid: p.req_gid,
            name,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcDeleteFriends>() {
        return vec![GameEvent::FriendRemoved {
            aid: p.aid,
            gid: p.gid,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcSkillinfoList>() {
        let skills = p
            .skill_list
            .iter()
            .map(|s| {
                let name: String = s.skill_name.iter().take_while(|c| **c != '\0').collect();
                SkillInfo {
                    id: s.skid as u16,
                    name,
                    level: s.level,
                    sp_cost: s.spcost,
                    attack_range: s.attack_range,
                    upgradable: s.upgradable != 0,
                    skill_target_type: SkillTargetType::from_value(s.atype as usize),
                }
            })
            .collect();
        return vec![GameEvent::SkillListReceived { skills }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcSkillinfoUpdate>() {
        return vec![GameEvent::SkillUpdated {
            id: p.skid,
            level: p.level,
            sp_cost: p.spcost,
            attack_range: p.attack_range,
            upgradable: p.upgradable,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcSkillinfoUpdate2>() {
        return vec![GameEvent::SkillUpdated {
            id: p.skid,
            level: p.level,
            sp_cost: p.spcost,
            attack_range: p.attack_range,
            upgradable: p.upgradable,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAddSkill>() {
        let name: String = p
            .data
            .skill_name
            .iter()
            .take_while(|c| **c != '\0')
            .collect();
        return vec![GameEvent::SkillAdded {
            skill: SkillInfo {
                id: p.data.skid as u16,
                name,
                level: p.data.level,
                sp_cost: p.data.spcost,
                attack_range: p.data.attack_range,
                upgradable: p.data.upgradable != 0,
                skill_target_type: SkillTargetType::from_value(p.data.atype as usize),
            },
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcShortcutKeyListV2>() {
        let slots: Vec<(i8, u32, i16)> = p
            .short_cut_key
            .iter()
            .map(|k| (k.is_skill, k.id, k.count))
            .collect();
        return vec![GameEvent::HotkeyListReceived { slots }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcShortcutKeyList>() {
        let slots: Vec<(i8, u32, i16)> = p
            .short_cut_key
            .iter()
            .map(|k| (k.is_skill, k.id, k.count))
            .collect();
        return vec![GameEvent::HotkeyListReceived { slots }];
    }
    if any.downcast_ref::<PacketZcActionFailure>().is_some() {
        return vec![GameEvent::ActionFailure];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyMapproperty>() {
        let kind = MapKind::from_property(p.atype);
        return vec![GameEvent::MapPropertyChanged(MapProperties::from_kind(
            kind,
        ))];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyMapproperty2>() {
        let kind = MapKind::from_property(p.atype);
        return vec![GameEvent::MapPropertyChanged(MapProperties::with_flags(
            kind,
            p.flags as u64,
        ))];
    }

    if let Some(p) = any.downcast_ref::<PacketZcAutorunSkill>() {
        return vec![GameEvent::AutoCastSkill {
            skill_id: p.data.skid as u16,
            level: p.data.level,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcItemidentifyList>() {
        return vec![GameEvent::ItemIdentifyList {
            indices: p.itidlist.clone(),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckItemidentify>() {
        return vec![GameEvent::ItemIdentifyResult {
            index: p.index,
            ok: p.result == 0,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcMakingarrowList>() {
        let item_ids = p.arrow_list.iter().map(|a| a.index as u16).collect();
        return vec![GameEvent::MakingArrowList { item_ids }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcMakableitemlist>() {
        let item_ids = p.info.iter().map(|e| e.itid).collect();
        return vec![GameEvent::MakableItemList { item_ids }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckReqmakingitem>() {
        return vec![GameEvent::MakingItemResult {
            result: p.result,
            item_id: p.itid,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcNotifyWeaponitemlist>() {
        let items = p.item_list.iter().map(refine_row_from).collect();
        return vec![GameEvent::WeaponRefineList { items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckWeaponrefine>() {
        return vec![GameEvent::WeaponRefineResult {
            result: p.msg,
            item_id: p.itid,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcRepairitemlist>() {
        let items = p.item_list.iter().map(refine_row_from).collect();
        return vec![GameEvent::RepairItemList { items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckItemrepair>() {
        return vec![GameEvent::RepairItemResult {
            index: p.index,
            ok: p.result == 0,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAutospelllist>() {
        let skill_ids = p.skid.iter().copied().filter(|&s| s != 0).collect();
        return vec![GameEvent::AutoSpellList { skill_ids }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcOpenstore>() {
        return vec![GameEvent::OpenVendingSetup {
            max_items: p.itemcount,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcAckOpenstore2>() {
        return vec![GameEvent::VendingOpenResult { result: p.result }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcStoreEntry>() {
        let name: String = p.store_name.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::VendingBoardShown {
            aid: p.maker_aid,
            name,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcDisappearEntry>() {
        return vec![GameEvent::VendingBoardHidden { aid: p.maker_aid }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPcPurchaseItemlistFrommc2>() {
        let items = p
            .item_list
            .iter()
            .map(|it| ragnarok_game::event::VendorItem {
                index: it.index,
                item_id: it.itid,
                amount: it.count,
                price: it.price,
                refine: it.refining_level,
                is_identified: it.is_identified != 0,
                is_damaged: it.is_damaged != 0,
                item_type: it.atype,
            })
            .collect();
        return vec![GameEvent::VendingShopList {
            aid: p.aid,
            unique_id: p.unique_id,
            items,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPcPurchaseMyitemlist>() {
        let items = p
            .item_list
            .iter()
            .map(|it| ragnarok_game::event::VendorItem {
                index: it.index,
                item_id: it.itid,
                amount: it.count,
                price: it.price,
                refine: it.refining_level,
                is_identified: it.is_identified != 0,
                is_damaged: it.is_damaged != 0,
                item_type: it.atype,
            })
            .collect();
        return vec![GameEvent::VendingOwnStock { items }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcPcPurchaseResultFrommc>() {
        return vec![GameEvent::VendingPurchaseResult {
            index: p.index,
            curcount: p.curcount,
            result: p.result,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcDeleteitemFromMcstore>() {
        return vec![GameEvent::VendingStockDecrement {
            index: p.index,
            count: p.count,
        }];
    }

    if let Some(p) = any.downcast_ref::<PacketZcPropertyHomun>() {
        let name: String = p.sz_name.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::HomunPropertyReceived {
            property: HomunculusProperty {
                name,
                renamed: p.b_modified & 0x1 != 0,
                vaporized: p.b_modified & 0x2 != 0,
                level: p.n_level,
                hunger: p.n_fullness,
                intimacy: p.n_relationship,
                accessory: p.itid,
                atk: p.atk,
                matk: p.matk,
                hit: p.hit,
                critical: p.critical,
                def: p.def,
                mdef: p.mdef,
                flee: p.flee,
                aspd: p.aspd,
                hp: p.hp as u32,
                max_hp: p.max_hp as u32,
                sp: p.sp as u32,
                max_sp: p.max_sp as u32,
                exp: p.exp,
                max_exp: p.max_exp,
                skill_points: p.skpoint,
                atk_range: p.atkrange,
            },
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcChangestateMer>() {
        return vec![GameEvent::CompanionStateChanged {
            state: p.state,
            gid: p.gid as u32,
            data: p.data,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcFeedMer>() {
        return vec![GameEvent::HomunFeedResult {
            success: p.c_ret != 0,
            item_id: p.itid,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcMerInit>() {
        let name: String = p.name.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::MercenaryInfoReceived {
            is_init: true,
            info: MercenaryInfo {
                gid: p.aid as u32,
                name,
                level: p.level,
                atk: p.atk,
                matk: p.matk,
                hit: p.hit,
                critical: p.critical,
                def: p.def,
                mdef: p.mdef,
                flee: p.flee,
                aspd: p.aspd,
                atk_range: p.atkrange,
                hp: p.hp as u32,
                max_hp: p.max_hp as u32,
                sp: p.sp as u32,
                max_sp: p.max_sp as u32,
                expire_date: p.expire_date,
                faith: p.faith,
                calls: p.toal_call_num,
                kills: p.approval_monster_kill_counter,
            },
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcMerProperty>() {
        let name: String = p.name.iter().take_while(|c| **c != '\0').collect();
        return vec![GameEvent::MercenaryInfoReceived {
            is_init: false,
            info: MercenaryInfo {
                gid: 0,
                name,
                level: p.level,
                atk: p.atk,
                matk: p.matk,
                hit: p.hit,
                critical: p.critical,
                def: p.def,
                mdef: p.mdef,
                flee: p.flee,
                aspd: p.aspd,
                atk_range: 0,
                hp: p.hp as u32,
                max_hp: p.max_hp as u32,
                sp: p.sp as u32,
                max_sp: p.max_sp as u32,
                expire_date: p.expire_date,
                faith: p.faith,
                calls: p.toal_call_num,
                kills: p.approval_monster_kill_counter,
            },
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcMerParChange>() {
        return vec![GameEvent::MercenaryParamChanged {
            var: p.var,
            value: p.value,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcHoParChange>() {
        return vec![GameEvent::HomunParamChanged {
            var: p.var,
            value: p.value,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcHoskillinfoList>() {
        return vec![GameEvent::HomunSkillList {
            skills: parse_skill_info_list(&p.skill_list),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcHoskillinfoUpdate>() {
        return vec![GameEvent::HomunSkillUpdate {
            id: p.skid,
            level: p.level,
            sp_cost: p.spcost,
            attack_range: p.attack_range,
            upgradable: p.upgradable,
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcMerSkillinfoList>() {
        return vec![GameEvent::MercenarySkillList {
            skills: parse_skill_info_list(&p.skill_list),
        }];
    }
    if let Some(p) = any.downcast_ref::<PacketZcMerSkillinfoUpdate>() {
        return vec![GameEvent::MercenarySkillUpdate {
            id: p.skid,
            level: p.level,
            sp_cost: p.spcost,
            attack_range: p.attack_range,
            upgradable: p.upgradable,
        }];
    }

    debug!("unhandled packet: {}", packet.name());
    vec![]
}

fn cstr(chars: &[char]) -> String {
    chars.iter().take_while(|c| **c != '\0').collect()
}

fn rgb_u32_to_rgba(rgb: u32) -> [f32; 4] {
    [
        ((rgb >> 16) & 0xff) as f32 / 255.0,
        ((rgb >> 8) & 0xff) as f32 / 255.0,
        (rgb & 0xff) as f32 / 255.0,
        1.0,
    ]
}

const BROADCAST_YELLOW: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
const BROADCAST_BLUE: [f32; 4] = [0.0, 1.0, 1.0, 1.0];

fn parse_broadcast(msg: &str) -> (String, [f32; 4]) {
    if let Some(rest) = msg.strip_prefix("tool")
        && rest.len() >= 6
        && let Ok(rgb) = u32::from_str_radix(&rest[..6], 16)
    {
        return (rest[6..].to_string(), rgb_u32_to_rgba(rgb));
    }
    if let Some(rest) = msg.strip_prefix("blue") {
        return (rest.to_string(), BROADCAST_BLUE);
    }
    if let Some(rest) = msg.strip_prefix("ssss") {
        return (rest.to_string(), BROADCAST_YELLOW);
    }
    (msg.to_string(), BROADCAST_YELLOW)
}

fn classify_banner(msg: String) -> (String, BannerKind) {
    if let Some(rest) = msg.strip_prefix('@') {
        return (rest.to_string(), BannerKind::Once);
    }
    if let Some(rest) = msg.strip_prefix("$$")
        && let Some((count, tail)) = rest.split_once('$')
        && let Ok(count) = count.parse::<u16>()
    {
        let text = tail.strip_prefix('$').unwrap_or(tail);
        let text = text.split_once('$').map_or(text, |(_, m)| m);
        return (text.to_string(), BannerKind::Repeat(count));
    }
    (msg, BannerKind::None)
}

fn raw_cstr(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// ZC_BAN_LIST (0x0163) elements for this packetver are `char_name[24] +
/// message[40]`; the generated struct carries an extra account field, so parse
/// the wire bytes directly.
fn parse_ban_list(raw: &[u8]) -> Vec<GuildBanEntry> {
    const ELEM: usize = 64;
    let mut entries = Vec::new();
    if raw.len() < 4 {
        return entries;
    }
    let len = u16::from_le_bytes([raw[2], raw[3]]) as usize;
    let end = len.min(raw.len());
    let mut off = 4;
    while off + ELEM <= end {
        entries.push(GuildBanEntry {
            char_name: raw_cstr(&raw[off..off + 24]),
            account: String::new(),
            reason: raw_cstr(&raw[off + 24..off + 64]),
        });
        off += ELEM;
    }
    entries
}

fn parse_skill_info_list(list: &[packets::packets::SKILLINFO]) -> Vec<SkillInfo> {
    list.iter()
        .map(|s| {
            let name: String = s.skill_name.iter().take_while(|c| **c != '\0').collect();
            SkillInfo {
                id: s.skid as u16,
                name,
                level: s.level,
                sp_cost: s.spcost,
                attack_range: s.attack_range,
                upgradable: s.upgradable != 0,
                skill_target_type: SkillTargetType::from_value(s.atype as usize),
            }
        })
        .collect()
}

fn refine_row_from(info: &RepairitemInfo) -> ragnarok_game::event::RefineItemRow {
    ragnarok_game::event::RefineItemRow {
        index: info.index,
        item_id: info.itid,
        refine: info.refining_level,
        cards: [
            info.slot.card1,
            info.slot.card2,
            info.slot.card3,
            info.slot.card4,
        ],
    }
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
    fn dispatch_self_config_packets_map_to_kinds() {
        let packetver = 20120307;

        let mut party = PacketZcPartyConfig::new(packetver);
        party.set_b_refuse_join_msg(true);
        party.fill_raw();
        assert!(matches!(
            dispatch_packet(&party, packetver).as_slice(),
            [GameEvent::SelfConfigChanged {
                kind: SelfConfigKind::RefusePartyInvite,
                enabled: true
            }]
        ));

        let mut notify = PacketZcConfigNotify::new(packetver);
        notify.set_b_open_equipment_win(true);
        notify.fill_raw();
        assert!(matches!(
            dispatch_packet(&notify, packetver).as_slice(),
            [GameEvent::SelfConfigChanged {
                kind: SelfConfigKind::OpenEquipmentWindow,
                enabled: true
            }]
        ));

        let mut homun = PacketZcConfig::new(packetver);
        homun.set_config(3);
        homun.set_value(1);
        homun.fill_raw();
        assert!(matches!(
            dispatch_packet(&homun, packetver).as_slice(),
            [GameEvent::SelfConfigChanged {
                kind: SelfConfigKind::HomunculusAutofeed,
                enabled: true
            }]
        ));

        let mut unknown = PacketZcConfig::new(packetver);
        unknown.set_config(99);
        unknown.fill_raw();
        assert!(dispatch_packet(&unknown, packetver).is_empty());
    }

    #[test]
    fn dispatch_broadcast_packets_carry_message_and_color() {
        let packetver = 20120307;

        let mut styled = PacketZcBroadcast2::new(packetver);
        styled.set_font_color(0x00ff00);
        styled.set_msg("Server restart soon".to_string());
        styled.fill_raw();
        assert!(matches!(
            dispatch_packet(&styled, packetver).as_slice(),
            [GameEvent::BroadcastMessage { message, color, banner: BannerKind::None }]
                if message == "Server restart soon" && *color == [0.0, 1.0, 0.0, 1.0]
        ));

        let mut plain = PacketZcBroadcast::new(packetver);
        plain.set_msg("Welcome".to_string());
        plain.fill_raw();
        assert!(matches!(
            dispatch_packet(&plain, packetver).as_slice(),
            [GameEvent::BroadcastMessage { message, color, banner: BannerKind::None }]
                if message == "Welcome" && *color == BROADCAST_YELLOW
        ));

        let mut blue = PacketZcBroadcast::new(packetver);
        blue.set_msg("blueGvG starts".to_string());
        blue.fill_raw();
        assert!(matches!(
            dispatch_packet(&blue, packetver).as_slice(),
            [GameEvent::BroadcastMessage { message, color, banner: BannerKind::None }]
                if message == "GvG starts" && *color == BROADCAST_BLUE
        ));

        let mut tool = PacketZcBroadcast::new(packetver);
        tool.set_msg("toolff0000red alert".to_string());
        tool.fill_raw();
        assert!(matches!(
            dispatch_packet(&tool, packetver).as_slice(),
            [GameEvent::BroadcastMessage { message, color, banner: BannerKind::None }]
                if message == "red alert" && *color == [1.0, 0.0, 0.0, 1.0]
        ));

        let mut banner = PacketZcBroadcast::new(packetver);
        banner.set_msg("@Server maintenance".to_string());
        banner.fill_raw();
        assert!(matches!(
            dispatch_packet(&banner, packetver).as_slice(),
            [GameEvent::BroadcastMessage { message, banner: BannerKind::Once, .. }]
                if message == "Server maintenance"
        ));
    }

    #[test]
    fn dispatch_guild_emblem_img_carries_blob() {
        let packetver = 20120307;
        let blob = vec![0x78, 0x9c, 0x01, 0x02, 0x03, 0x04, 0x05];
        let mut pkt = PacketZcGuildEmblemImg::new(packetver);
        pkt.set_gdid(42);
        pkt.set_emblem_version(7);
        pkt.set_img_raw(blob.clone());
        let result = dispatch_packet(&pkt, packetver);
        let [GameEvent::GuildEmblem { gdid, version, bmp }] = result.as_slice() else {
            panic!("expected GuildEmblem, got {result:?}");
        };
        assert_eq!((*gdid, *version), (42, 7));
        assert_eq!(*bmp, blob);
    }

    #[test]
    fn dispatch_change_members_ack_yields_position_changes() {
        let packetver = 20120307;
        let mut row = MemberPositionInfo::new(packetver);
        row.set_aid(2000000);
        row.set_gid(150001);
        row.set_position_id(3);
        row.fill_raw();
        let mut pkt = PacketZcAckReqChangeMembers::new(packetver);
        pkt.set_packet_length(4 + 12);
        pkt.set_member_info(vec![row]);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        let [GameEvent::GuildMemberPositionsChanged { entries }] = result.as_slice() else {
            panic!("expected GuildMemberPositionsChanged, got {result:?}");
        };
        assert_eq!(entries.as_slice(), &[(2000000, 150001, 3)]);
    }

    #[test]
    fn dispatch_cart_normal_itemlist3_yields_cart_items() {
        let packetver = 20120307;
        let mut pkt = PacketZcCartNormalItemlist3::new(packetver);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert!(matches!(
            result.as_slice(),
            [GameEvent::CartNormalItems { .. }]
        ));
    }

    #[test]
    fn disconnect_ack_maps_result_and_request_carries_quit_type() {
        let packetver = 20120307;

        let mut allowed = PacketZcAckReqDisconnect::new(packetver);
        allowed.set_result(0);
        allowed.fill_raw();
        assert!(matches!(
            dispatch_packet(&allowed, packetver).as_slice(),
            [GameEvent::DisconnectAck { allowed: true }]
        ));

        let mut refused = PacketZcAckReqDisconnect::new(packetver);
        refused.set_result(1);
        refused.fill_raw();
        assert!(matches!(
            dispatch_packet(&refused, packetver).as_slice(),
            [GameEvent::DisconnectAck { allowed: false }]
        ));

        let raw = crate::sender::build_req_disconnect_packet(packetver);
        let parsed = packets::packets_parser::parse(&raw, packetver);
        let request = parsed
            .as_any()
            .downcast_ref::<PacketCzReqDisconnect>()
            .expect("parsed as disconnect request");
        assert_eq!(request.atype, 0);
    }

    #[test]
    fn change_cart_packet_carries_model_number() {
        let packetver = 20120307;
        let raw = crate::sender::build_change_cart_packet(3, packetver);
        let parsed = packets::packets_parser::parse(&raw, packetver);
        let p = parsed
            .as_any()
            .downcast_ref::<PacketCzReqChangecart>()
            .expect("parsed as change-cart request");
        assert_eq!(p.num, 3);
    }

    #[test]
    fn dispatch_skill_unit_entry_and_disappear_round_trip() {
        let packetver = 20120307;
        let mut entry = PacketZcSkillEntry::new(packetver);
        entry.set_aid(7001);
        entry.set_creator_aid(42);
        entry.set_x_pos(150);
        entry.set_y_pos(200);
        entry.set_job(0x83);
        entry.set_is_visible(true);
        entry.fill_raw();
        match &dispatch_packet(&entry, packetver)[..] {
            [
                GameEvent::SkillUnitEntered {
                    aid,
                    x,
                    y,
                    unit_id,
                    is_visible,
                    ..
                },
            ] => {
                assert_eq!(*aid, 7001);
                assert_eq!((*x, *y), (150, 200));
                assert_eq!(*unit_id, 0x83);
                assert!(*is_visible);
            }
            other => panic!("expected SkillUnitEntered, got {other:?}"),
        }

        // 0x011f wire layout at this packetver is `job · isVisible` (16 bytes total).
        let wire: Vec<u8> = vec![
            0x1f, 0x01, // packet id
            0x59, 0x1b, 0x00, 0x00, // aid
            0x2a, 0x00, 0x00, 0x00, // creator aid
            0x96, 0x00, // x
            0xc8, 0x00, // y
            0x90, // job (UNT_SKIDTRAP)
            0x01, // isVisible
        ];
        let parsed = PacketZcSkillEntry::from(&wire, packetver);
        assert_eq!(parsed.job, 0x90);
        assert!(parsed.is_visible);

        let mut gone = PacketZcSkillDisappear::new(packetver);
        gone.set_aid(7001);
        gone.fill_raw();
        match &dispatch_packet(&gone, packetver)[..] {
            [GameEvent::SkillUnitDisappeared { aid }] => assert_eq!(*aid, 7001),
            other => panic!("expected SkillUnitDisappeared, got {other:?}"),
        }
    }

    #[test]
    fn makable_item_list_decodes_all_entries() {
        let packetver = 20120307;
        // id(0x8d,0x01) + len(20) + two 8-byte entries {itid.W, mat[3].W}
        let mut buf: Vec<u8> = vec![0x8d, 0x01, 20, 0x00];
        buf.extend_from_slice(&501u16.to_le_bytes());
        buf.extend_from_slice(&[0u8; 6]);
        buf.extend_from_slice(&502u16.to_le_bytes());
        buf.extend_from_slice(&[0u8; 6]);
        let pkt = packets::packets_parser::parse(&buf, packetver);
        match &dispatch_packet(pkt.as_ref(), packetver)[..] {
            [GameEvent::MakableItemList { item_ids }] => {
                assert_eq!(item_ids, &vec![501u16, 502u16]);
            }
            other => panic!("expected MakableItemList, got {other:?}"),
        }
    }

    #[test]
    fn production_cz_builders_round_trip() {
        let packetver = 20120307;

        let raw = crate::sender::build_req_itemidentify_packet(7, packetver);
        let parsed = packets::packets_parser::parse(&raw, packetver);
        assert_eq!(
            parsed
                .as_any()
                .downcast_ref::<PacketCzReqItemidentify>()
                .unwrap()
                .index,
            7
        );

        let raw = crate::sender::build_req_makingarrow_packet(1750, packetver);
        let parsed = packets::packets_parser::parse(&raw, packetver);
        assert_eq!(
            parsed
                .as_any()
                .downcast_ref::<PacketCzReqMakingarrow>()
                .unwrap()
                .id,
            1750
        );

        let raw = crate::sender::build_req_weaponrefine_packet(42, packetver);
        let parsed = packets::packets_parser::parse(&raw, packetver);
        assert_eq!(
            parsed
                .as_any()
                .downcast_ref::<PacketCzReqWeaponrefine>()
                .unwrap()
                .index,
            42
        );

        let raw = crate::sender::build_req_itemrepair_packet(3, 1201, 4, [10, 20, 0, 0], packetver);
        let parsed = packets::packets_parser::parse(&raw, packetver);
        let p = parsed
            .as_any()
            .downcast_ref::<PacketCzReqItemrepair>()
            .unwrap();
        assert_eq!(p.target_item_info.index, 3);
        assert_eq!(p.target_item_info.itid, 1201);
        assert_eq!(p.target_item_info.refining_level, 4);
        assert_eq!(p.target_item_info.slot.card1, 10);
    }

    // CZ_REQMAKINGITEM (0x018e): the generated `MakableitemInfo` parser is
    // internally inconsistent (base_len 5 vs an 8-byte body), so re-parsing panics.
    // Assert the outgoing wire bytes directly: id.W, itid.W, mat[3].W = 10 bytes.
    #[test]
    fn making_item_builder_wire_bytes() {
        let packetver = 20120307;
        let raw = crate::sender::build_req_makingitem_packet(501, [1000, 990, 5], packetver);
        assert_eq!(raw.len(), 10);
        assert_eq!(u16::from_le_bytes([raw[0], raw[1]]), 0x018e);
        assert_eq!(u16::from_le_bytes([raw[2], raw[3]]), 501);
        assert_eq!(u16::from_le_bytes([raw[4], raw[5]]), 1000);
        assert_eq!(u16::from_le_bytes([raw[6], raw[7]]), 990);
        assert_eq!(u16::from_le_bytes([raw[8], raw[9]]), 5);
    }

    #[test]
    fn vending_open_store_builder_sets_packet_length() {
        let packetver = 20120307;
        let raw = crate::sender::build_req_openstore2_packet(
            "Cheap Potions",
            &[(5, 10, 1000), (6, 1, 5000)],
            packetver,
        );
        // header(2) + len(2) + name(80) + result(1) + 2*8 = 101
        let len = u16::from_le_bytes([raw[2], raw[3]]);
        assert_eq!(len, 101);
        let parsed = packets::packets_parser::parse(&raw, packetver);
        let p = parsed
            .as_any()
            .downcast_ref::<PacketCzReqOpenstore2>()
            .unwrap();
        assert_eq!(p.store_list.len(), 2);
        assert_eq!(p.store_list[0].price, 1000);
    }

    #[test]
    fn vending_cancel_builder_clears_result_with_empty_list() {
        let packetver = 20120307;
        let raw = crate::sender::build_req_cancel_openstore_packet(packetver);
        let len = u16::from_le_bytes([raw[2], raw[3]]);
        assert_eq!(len, 85);
        let parsed = packets::packets_parser::parse(&raw, packetver);
        let p = parsed
            .as_any()
            .downcast_ref::<PacketCzReqOpenstore2>()
            .unwrap();
        assert!(!p.result);
        assert!(p.store_list.is_empty());
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
    fn dispatch_notify_exp_returns_base_exp_gain() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyExp::new(packetver);
        pkt.set_aid(2000000);
        pkt.set_amount(1500);
        pkt.set_var_id(StatusTypes::Baseexp.value() as u16);
        pkt.set_exp_type(0);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::ExpGained {
                aid,
                amount,
                is_base,
                is_quest,
            } => {
                assert_eq!(*aid, 2000000);
                assert_eq!(*amount, 1500);
                assert!(*is_base);
                assert!(!*is_quest);
            }
            other => panic!("expected ExpGained, got {other:?}"),
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
        let x1: u16 = 100;
        let y1: u16 = 200;
        let x2: u16 = 110;
        let y2: u16 = 210;
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
            GameEvent::PlayerMoved {
                start_x,
                start_y,
                dest_x,
                dest_y,
                start_time,
            } => {
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
    fn dispatch_spawn_unit_returns_standing_entity_spawn() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyNewentry7::new(packetver);
        pkt.set_gid(123456);
        pkt.set_job(1002);
        pkt.set_pos_dir(crate::helpers::encode_pos(100, 200, 3));
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntitySpawned {
                gid,
                job,
                x,
                y,
                direction,
                posture,
                is_new_entry,
                ..
            } => {
                assert_eq!(*gid, 123456);
                assert_eq!(*job, 1002);
                assert_eq!((*x, *y, *direction), (100, 200, 3));
                assert_eq!(*posture, 0, "a freshly spawned entity is standing");
                assert!(*is_new_entry, "newentry marks a fresh appearance");
            }
            other => panic!("expected EntitySpawned, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_standentry_is_not_a_new_entry() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyStandentry7::new(packetver);
        pkt.set_gid(123456);
        pkt.set_job(1002);
        pkt.set_pos_dir(crate::helpers::encode_pos(100, 200, 3));
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        match &result[0] {
            GameEvent::EntitySpawned { is_new_entry, .. } => {
                assert!(
                    !is_new_entry,
                    "an already-present entity entering view is not a new entry"
                );
            }
            other => panic!("expected EntitySpawned, got {other:?}"),
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
            GameEvent::EntityVanished { gid, vanish_type } => {
                assert_eq!(*gid, 42);
                assert!(matches!(vanish_type, VanishType::OutOfSight));
            }
            other => panic!("expected EntityVanished, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_notify_move_returns_entity_moved() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyMove::new(packetver);
        pkt.set_gid(99);
        pkt.set_move_start_time(7000);
        let x1: u16 = 50;
        let y1: u16 = 60;
        let x2: u16 = 55;
        let y2: u16 = 65;
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
            GameEvent::EntityMoved {
                gid,
                start_x,
                start_y,
                dest_x,
                dest_y,
                start_time,
            } => {
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
            GameEvent::EntityAction {
                gid,
                target_gid,
                action,
                damage,
                attack_mt,
                attacked_mt,
                ..
            } => {
                assert_eq!(*gid, 50);
                assert_eq!(*target_gid, 99);
                assert_eq!(*action, ActionType::AttackMultiple);
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
        assert_eq!(raw[0], 0x85);
        assert_eq!(raw[1], 0x08);
        assert_eq!(&raw[2..6], &[0, 0, 0, 0]);
        assert_eq!(raw[6], 2);
    }

    #[test]
    fn build_chat_packet_has_correct_format() {
        let raw = crate::sender::build_chat_packet("Player : hello", 20120307);
        assert_eq!(raw.len(), 19);
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
    fn dispatch_ack_reqnameall_returns_entity_names_received() {
        let packetver = 20120307;
        let mut pkt = PacketZcAckReqnameall::new(packetver);
        pkt.set_aid(42);
        let fill = |s: &str| {
            let mut buf = ['\0'; 24];
            for (i, c) in s.chars().enumerate() {
                buf[i] = c;
            }
            buf
        };
        pkt.set_cname(fill("Alice"));
        pkt.set_gname(fill("Knights"));
        pkt.set_rname(fill("Leader"));
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntityNamesReceived {
                gid,
                name,
                guild_name,
                position_name,
            } => {
                assert_eq!(*gid, 42);
                assert_eq!(name, "Alice");
                assert_eq!(guild_name, "Knights");
                assert_eq!(position_name, "Leader");
            }
            other => panic!("expected EntityNamesReceived, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_par_change_returns_parameter_changed() {
        let packetver = 20120307;
        let mut pkt = PacketZcParChange::new(packetver);
        pkt.set_var_id(5);
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
        pkt.set_atype(2);
        pkt.set_value(1);
        pkt.set_value2(0);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntitySpriteChanged {
                gid,
                sprite_type,
                value,
                value2,
            } => {
                assert_eq!(*gid, 150000);
                assert_eq!(*sprite_type, 2);
                assert_eq!(*value, 1);
                assert_eq!(*value2, 0);
            }
            other => panic!("expected EntitySpriteChanged, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_npcsprite_change_returns_class_sprite_change() {
        let packetver = 20120307;
        let mut pkt = PacketZcNpcspriteChange::new(packetver);
        pkt.set_gid(150000);
        pkt.set_atype(0);
        pkt.set_value(1160);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntitySpriteChanged {
                gid,
                sprite_type,
                value,
                value2,
            } => {
                assert_eq!(*gid, 150000);
                assert_eq!(*sprite_type, 0);
                assert_eq!(*value, 1160);
                assert_eq!(*value2, 0);
            }
            other => panic!("expected EntitySpriteChanged, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_notify_effect2_returns_play_effect_on_entity() {
        let packetver = 20120307;
        let mut pkt = PacketZcNotifyEffect2::new(packetver);
        pkt.set_aid(150000);
        pkt.set_effect_id(28);
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::PlayEffectOnEntity {
                gid,
                effect_id,
                value,
            } => {
                assert_eq!(*gid, 150000);
                assert_eq!(*effect_id, 28);
                assert_eq!(*value, None);
            }
            other => panic!("expected PlayEffectOnEntity, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_spirits_returns_spirits_changed() {
        let packetver = 20120307;
        let mut pkt = PacketZcSpirits::new(packetver);
        pkt.set_aid(150000);
        pkt.set_num(5);
        match &dispatch_packet(&pkt, packetver)[0] {
            GameEvent::SpiritsChanged { gid, count } => {
                assert_eq!(*gid, 150000);
                assert_eq!(*count, 5);
            }
            other => panic!("expected SpiritsChanged, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_bladestop_toggles_root_by_flag() {
        let packetver = 20120307;
        let mut pkt = PacketZcBladestop::new(packetver);
        pkt.set_src_aid(150000);
        pkt.set_dest_aid(160000);
        pkt.set_flag(1);
        match &dispatch_packet(&pkt, packetver)[0] {
            GameEvent::BladeStop {
                src_gid,
                dest_gid,
                active,
            } => {
                assert_eq!(*src_gid, 150000);
                assert_eq!(*dest_gid, 160000);
                assert!(*active);
            }
            other => panic!("expected BladeStop, got {other:?}"),
        }
        pkt.set_flag(0);
        match &dispatch_packet(&pkt, packetver)[0] {
            GameEvent::BladeStop { active, .. } => assert!(!*active),
            other => panic!("expected BladeStop, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_resurrection_and_mvp_return_lifecycle_events() {
        let packetver = 20120307;
        let mut res = PacketZcResurrection::new(packetver);
        res.set_aid(150000);
        match &dispatch_packet(&res, packetver)[0] {
            GameEvent::EntityResurrected { gid } => assert_eq!(*gid, 150000),
            other => panic!("expected EntityResurrected, got {other:?}"),
        }

        let mut mvp = PacketZcMvp::new(packetver);
        mvp.set_aid(150001);
        match &dispatch_packet(&mvp, packetver)[0] {
            GameEvent::MvpReward { gid } => assert_eq!(*gid, 150001),
            other => panic!("expected MvpReward, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_recovery_returns_recovery_event() {
        let packetver = 20120307;
        let mut pkt = PacketZcRecovery::new(packetver);
        pkt.set_var_id(5);
        pkt.set_amount(42);
        match &dispatch_packet(&pkt, packetver)[0] {
            GameEvent::Recovery { var_id, amount } => {
                assert_eq!(*var_id, 5);
                assert_eq!(*amount, 42);
            }
            other => panic!("expected Recovery, got {other:?}"),
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
            GameEvent::SkillCasting {
                gid,
                target_gid,
                skill_id,
                property,
                delay_ms,
                x,
                y,
                skill_name,
            } => {
                assert_eq!(*gid, 150000);
                assert_eq!(*target_gid, 200000);
                assert_eq!(*skill_id, 10);
                assert_eq!(*property, 0);
                assert_eq!(*delay_ms, 2000);
                assert_eq!(*x, 0);
                assert_eq!(*y, 0);
                assert_eq!(skill_name.as_deref(), Some("MG_SIGHT"));
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
        match result.as_slice() {
            [
                GameEvent::EntityHpChanged { gid, hp, max_hp },
                GameEvent::PartyMemberHp {
                    aid,
                    hp: php,
                    max_hp: pmax,
                },
            ] => {
                assert_eq!((*gid, *hp, *max_hp), (42, 350, 500));
                assert_eq!((*aid, *php, *pmax), (42, 350, 500));
            }
            other => panic!("expected EntityHpChanged + PartyMemberHp, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_autorun_skill_returns_auto_cast() {
        let packetver = 20120307;
        let mut pkt = PacketZcAutorunSkill::new(packetver);
        pkt.data.skid = SkillEnum::McIdentify.id() as i16;
        pkt.data.level = 1;
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::AutoCastSkill { skill_id, level } => {
                assert_eq!(*skill_id, SkillEnum::McIdentify.id() as u16);
                assert_eq!(*level, 1);
            }
            other => panic!("expected AutoCastSkill, got {other:?}"),
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
        assert!(matches!(
            &result[0],
            GameEvent::NpcDialogNext { npc_id: 500 }
        ));

        let mut pkt = PacketZcCloseDialog::new(packetver);
        pkt.set_naid(500);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        assert!(matches!(
            &result[0],
            GameEvent::NpcDialogClose { npc_id: 500 }
        ));
    }

    #[test]
    fn dispatch_state_change3_returns_entity_option_changed() {
        let packetver = 20120307;
        let mut pkt = PacketZcStateChange3::new(packetver);
        pkt.set_aid(150000);
        pkt.set_body_state(3);
        pkt.set_health_state(0x1);
        pkt.set_effect_state(0x20);
        pkt.set_is_pkmode_on(false);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::EntityOptionChanged {
                gid,
                body_state,
                health_state,
                effect_state,
            } => {
                assert_eq!(*gid, 150000);
                assert_eq!(*body_state, 3);
                assert_eq!(*health_state, 0x1);
                assert_eq!(*effect_state, 0x20);
            }
            other => panic!("expected EntityOptionChanged, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_msg_state_change_routes_buff_and_keeps_postdelay() {
        let packetver = 20120307;

        let mut pkt = PacketZcMsgStateChange2::new(packetver);
        pkt.set_index(192);
        pkt.set_aid(150000);
        pkt.set_state(true);
        pkt.set_remain_ms(60000);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::StatusEffectChanged {
                gid,
                efst,
                active,
                remain_ms,
                ..
            } => {
                assert_eq!(*gid, 150000);
                assert_eq!(*efst, 192);
                assert!(*active);
                assert_eq!(*remain_ms, 60000);
            }
            other => panic!("expected StatusEffectChanged, got {other:?}"),
        }

        let mut pkt = PacketZcMsgStateChange2::new(packetver);
        pkt.set_index(46);
        pkt.set_aid(150000);
        pkt.set_state(true);
        pkt.set_remain_ms(500);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert!(matches!(
            &result[0],
            GameEvent::AfterCastDelay { delay_ms: 500 }
        ));

        let mut pkt = PacketZcMsgStateChange::new(packetver);
        pkt.set_index(192);
        pkt.set_aid(150000);
        pkt.set_state(false);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::StatusEffectChanged {
                gid,
                efst,
                active,
                remain_ms,
                ..
            } => {
                assert_eq!(*gid, 150000);
                assert_eq!(*efst, 192);
                assert!(!*active, "0x196 off-packet must deactivate");
                assert_eq!(*remain_ms, 0);
            }
            other => panic!("expected StatusEffectChanged, got {other:?}"),
        }
    }

    #[test]
    fn party_group_list_decodes_members_and_invite_round_trips() {
        let packetver = 20120307;

        let mut name = [0u8; 24];
        name[.."Adventurers".len()].copy_from_slice(b"Adventurers");
        let member = |aid: u32, nick: &str, map: &str, role: u8, state: u8| {
            let mut buf = Vec::with_capacity(46);
            buf.extend_from_slice(&aid.to_le_bytes());
            let mut nb = [0u8; 24];
            nb[..nick.len()].copy_from_slice(nick.as_bytes());
            buf.extend_from_slice(&nb);
            let mut mb = [0u8; 16];
            mb[..map.len()].copy_from_slice(map.as_bytes());
            buf.extend_from_slice(&mb);
            buf.push(role);
            buf.push(state);
            buf
        };
        let m0 = member(101, "Leader", "prontera.gat", 0, 0);
        let m1 = member(102, "Buddy", "payon.gat", 1, 1);

        let mut raw = Vec::new();
        raw.extend_from_slice(&[0xFB, 0x00]);
        let len = (4 + 24 + m0.len() + m1.len()) as i16;
        raw.extend_from_slice(&len.to_le_bytes());
        raw.extend_from_slice(&name);
        raw.extend_from_slice(&m0);
        raw.extend_from_slice(&m1);

        let parsed = packets::packets_parser::parse(&raw, packetver);
        match &dispatch_packet(parsed.as_ref(), packetver)[..] {
            [GameEvent::PartyMemberList { name, members }] => {
                assert_eq!(name, "Adventurers");
                assert_eq!(members.len(), 2);
                assert_eq!(members[0].aid, 101);
                assert_eq!(members[0].name, "Leader");
                assert!(members[0].leader && members[0].online);
                assert_eq!(members[1].name, "Buddy");
                assert!(!members[1].leader && !members[1].online);
            }
            other => panic!("expected PartyMemberList, got {other:?}"),
        }

        let invite = crate::sender::build_req_join_party_packet(101, packetver);
        assert_eq!(invite[0], 0xFC);
        let aid = u32::from_le_bytes([invite[2], invite[3], invite[4], invite[5]]);
        assert_eq!(aid, 101);
    }

    #[test]
    fn dispatch_room_newentry_returns_chat_room_upsert() {
        let packetver = 20120307;
        let mut pkt = PacketZcRoomNewentry::new(packetver);
        pkt.set_aid(150000);
        pkt.set_room_id(7);
        pkt.set_maxcount(20);
        pkt.set_curcount(3);
        pkt.set_atype(2);
        let title = "Arena Entrance\0";
        pkt.set_title(title.to_string());
        pkt.set_title_raw(title.as_bytes().to_vec());
        pkt.set_packet_length((17 + title.len()) as i16);
        pkt.fill_raw();
        let result = dispatch_packet(&pkt, packetver);
        assert_eq!(result.len(), 1);
        match &result[0] {
            GameEvent::ChatRoomUpsert {
                owner_aid,
                room_id,
                max_count,
                cur_count,
                atype,
                title,
            } => {
                assert_eq!(*owner_aid, 150000);
                assert_eq!(*room_id, 7);
                assert_eq!(*max_count, 20);
                assert_eq!(*cur_count, 3);
                assert_eq!(*atype, 2);
                assert_eq!(title, "Arena Entrance");
            }
            other => panic!("expected ChatRoomUpsert, got {other:?}"),
        }
    }

    #[test]
    fn guild_membermgr_decodes_online_and_offline_members() {
        let packetver = 20120307;

        let member = |aid: u32, gid: u32, name: &str, job: i16, level: i16, position: i32,
                      state: i32| {
            let mut buf = Vec::with_capacity(110);
            buf.extend_from_slice(&aid.to_le_bytes());
            buf.extend_from_slice(&gid.to_le_bytes());
            buf.extend_from_slice(&0i16.to_le_bytes()); // head
            buf.extend_from_slice(&0i16.to_le_bytes()); // head palette
            buf.extend_from_slice(&0i16.to_le_bytes()); // sex
            buf.extend_from_slice(&job.to_le_bytes());
            buf.extend_from_slice(&level.to_le_bytes());
            buf.extend_from_slice(&500i32.to_le_bytes()); // contribution exp
            buf.extend_from_slice(&state.to_le_bytes());
            buf.extend_from_slice(&position.to_le_bytes());
            buf.extend_from_slice(&[0u8; 50]); // memo
            let mut nb = [0u8; 24];
            nb[..name.len()].copy_from_slice(name.as_bytes());
            buf.extend_from_slice(&nb);
            buf
        };
        let m0 = member(101, 201, "Master", 4008, 99, 0, 1);
        let m1 = member(102, 202, "Grunt", 1, 40, 2, 0);

        let mut raw = Vec::new();
        raw.extend_from_slice(&[0x54, 0x01]);
        let len = (4 + m0.len() + m1.len()) as i16;
        raw.extend_from_slice(&len.to_le_bytes());
        raw.extend_from_slice(&m0);
        raw.extend_from_slice(&m1);

        let parsed = packets::packets_parser::parse(&raw, packetver);
        match &dispatch_packet(parsed.as_ref(), packetver)[..] {
            [GameEvent::GuildMembers { members }] => {
                assert_eq!(members.len(), 2);
                assert_eq!((members[0].gid, members[0].name.as_str()), (201, "Master"));
                assert!(members[0].online && members[0].position_id == 0);
                assert_eq!((members[1].gid, members[1].name.as_str()), (202, "Grunt"));
                assert!(!members[1].online && members[1].position_id == 2);
            }
            other => panic!("expected GuildMembers, got {other:?}"),
        }
    }

    #[test]
    fn guild_ban_list_decodes_charname_and_reason() {
        let entry = |name: &str, reason: &str| {
            let mut buf = vec![0u8; 64];
            buf[..name.len()].copy_from_slice(name.as_bytes());
            buf[24..24 + reason.len()].copy_from_slice(reason.as_bytes());
            buf
        };
        let e0 = entry("Traitor", "Left mid-WoE");
        let e1 = entry("Spy", "Enemy alt");
        let mut raw = vec![0x63, 0x01];
        let len = (4 + e0.len() + e1.len()) as u16;
        raw.extend_from_slice(&len.to_le_bytes());
        raw.extend_from_slice(&e0);
        raw.extend_from_slice(&e1);

        let entries = parse_ban_list(&raw);
        assert_eq!(entries.len(), 2);
        assert_eq!((entries[0].char_name.as_str(), entries[0].reason.as_str()), ("Traitor", "Left mid-WoE"));
        assert_eq!((entries[1].char_name.as_str(), entries[1].reason.as_str()), ("Spy", "Enemy alt"));
    }

    #[test]
    fn guild_positioninfo_decodes_rights_bits() {
        let packetver = 20120307;

        let position = |id: i32, right: i32, ranking: i32, pay: i32| {
            let mut buf = Vec::with_capacity(16);
            buf.extend_from_slice(&id.to_le_bytes());
            buf.extend_from_slice(&right.to_le_bytes());
            buf.extend_from_slice(&ranking.to_le_bytes());
            buf.extend_from_slice(&pay.to_le_bytes());
            buf
        };
        // Master rank: invite (0x1) + expel (0x10) + storage (0x100); grunt rank: none.
        let p0 = position(0, 0x111, 0, 50);
        let p1 = position(1, 0x000, 1, 0);

        let mut raw = Vec::new();
        raw.extend_from_slice(&[0x60, 0x01]);
        let len = (4 + p0.len() + p1.len()) as i16;
        raw.extend_from_slice(&len.to_le_bytes());
        raw.extend_from_slice(&p0);
        raw.extend_from_slice(&p1);

        let parsed = packets::packets_parser::parse(&raw, packetver);
        match &dispatch_packet(parsed.as_ref(), packetver)[..] {
            [GameEvent::GuildPositions { positions }] => {
                assert_eq!(positions.len(), 2);
                assert_eq!(positions[0].id, 0);
                assert_eq!(positions[0].right, 0x111);
                assert_eq!(positions[0].pay_rate, 50);
                assert_eq!(positions[1].right, 0);
            }
            other => panic!("expected GuildPositions, got {other:?}"),
        }
    }
}
