mod adoption;
mod character;
mod chat;
mod companion;
mod config;
mod connection;
mod entity;
mod friends;
mod guild;
mod inventory;
mod login;
mod mail;
pub(crate) mod marriage;
mod npc;
mod party;
mod pet;
mod production;
mod quest;
mod skill;
mod storage;
mod trade;

use crate::App;
use models::enums::EnumWithMaskValueU64;
use ragnarok_formats::act::SpriteActionType;
use ragnarok_game::entity::ForcedAnimation;
use models::enums::skill_enums::SkillEnum;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::chat_room::ChatRoom;
use ragnarok_ui_component::game::chat_room_member_window;
use ragnarok_game::autocounter;
use ragnarok_game::event::GameEvent;
use ragnarok_network::build_npc_close_packet;
use ragnarok_renderer::Renderer;
use ragnarok_ui_component::Window as UiWindow;
use winit::event_loop::ActiveEventLoop;

const BLADE_STOP_GRIP_FRAME: usize = 4;

pub(crate) fn char_create_error_message(error_code: u8) -> &'static str {
    match error_code {
        0x00 => "That name already exists.",
        0x01 => "You are not eligible to create a character.",
        0x02 => "Character creation denied.",
        0x03 => "Character creation is currently disabled.",
        _ => "Character creation failed.",
    }
}

pub(crate) fn char_delete_reserve_error(result: u32) -> &'static str {
    match result {
        3 => "A database error occurred.",
        4 => "Leave your guild to delete this character.",
        5 => "Leave your party to delete this character.",
        _ => "Unable to schedule deletion.",
    }
}

pub(crate) fn char_delete_confirm_error(result: u32) -> &'static str {
    match result {
        2 => "This character cannot be deleted.",
        3 => "A database error occurred.",
        4 => "The deletion delay has not elapsed yet.",
        5 => "The birthdate does not match.",
        _ => "Character deletion failed.",
    }
}

pub(crate) fn preload_window<W: UiWindow>(
    window: &mut W,
    renderer: &mut Renderer,
    grf: &GrfArchive,
) {
    if !window.has_grf_textures() {
        let paths = W::grf_texture_paths();
        let loaded = renderer.preload_textures(&paths, grf);
        window.set_has_grf_textures(loaded);
        if loaded {
            window.set_texture_sizes(&|name| renderer.texture_cache.texture_size(name));
        }
    }
}

impl App {
    pub(crate) fn handle_game_events(&mut self, event_loop: &ActiveEventLoop) {
        let events = self.channel.drain_events();
        for event in events {
            match event {
                GameEvent::LoginAccepted {
                    account_id,
                    login_id1,
                    login_id2,
                    sex,
                    servers,
                } => {
                    self.handle_login_accepted(account_id, login_id1, login_id2, sex, servers);
                }
                GameEvent::LoginRefused { error_code } => {
                    self.handle_login_refused(error_code);
                }
                GameEvent::CharacterListReceived { characters } => {
                    self.handle_character_list_received(characters);
                }
                GameEvent::ZoneServerConnectInfo {
                    char_id,
                    map_name,
                    ip,
                    port,
                } => {
                    self.handle_zone_server_connect_info(char_id, map_name, ip, port);
                }
                GameEvent::RestartAck => {
                    self.handle_restart_ack();
                }
                GameEvent::DisconnectAck { allowed } => {
                    self.handle_disconnect_ack(allowed, event_loop);
                }

                GameEvent::MapEntered { x, y, dir, .. } => {
                    self.handle_map_entered(x, y, dir);
                }
                GameEvent::MapChanged { map_name, x, y } => {
                    self.handle_map_changed(map_name, x, y);
                }
                GameEvent::MapPropertyChanged(properties) => {
                    self.game.combat.damage_numbers.combat_hidden = properties.is_siege();
                    self.game.map_properties = properties;
                }
                GameEvent::PlayerMoved {
                    start_x,
                    start_y,
                    dest_x,
                    dest_y,
                    start_time,
                } => {
                    self.handle_player_moved(start_x, start_y, dest_x, dest_y, start_time);
                }

                GameEvent::EntitySpawned {
                    gid,
                    aid,
                    job,
                    speed,
                    sex,
                    head,
                    weapon,
                    shield,
                    head_top,
                    head_mid,
                    head_bottom,
                    hair_color,
                    x,
                    y,
                    direction,
                    body_state,
                    health_state,
                    effect_state,
                    base_level,
                    is_boss,
                    posture,
                    guild_id,
                    guild_emblem_version,
                    is_new_entry,
                } => {
                    self.handle_entity_spawned(
                        gid,
                        aid,
                        job,
                        speed,
                        sex,
                        head,
                        weapon,
                        shield,
                        head_top,
                        head_mid,
                        head_bottom,
                        hair_color,
                        x,
                        y,
                        direction,
                        body_state,
                        health_state,
                        effect_state,
                        base_level,
                        is_boss,
                        posture,
                        guild_id,
                        guild_emblem_version,
                        is_new_entry,
                    );
                }
                GameEvent::EntityMoved {
                    gid,
                    start_x,
                    start_y,
                    dest_x,
                    dest_y,
                    start_time,
                } => {
                    self.handle_entity_moved(gid, start_x, start_y, dest_x, dest_y, start_time);
                }
                GameEvent::EntityVanished { gid, vanish_type } => {
                    self.handle_entity_vanished(gid, vanish_type);
                }
                GameEvent::EntityStopMove { gid, x, y } => {
                    self.game.entities.apply_entity_stop_move(gid, x, y);
                }
                GameEvent::EntityHighJumped { gid, x, y } => {
                    // By the time this relocate arrives the leap has carried the
                    // caster off-screen (faded), so teleport to the landing cell
                    // straight away — the landing effect drops it back in.
                    self.game.entities.apply_entity_stop_move(gid, x, y);
                }
                GameEvent::EntityAction {
                    gid,
                    target_gid,
                    action,
                    damage,
                    left_damage,
                    attack_mt,
                    attacked_mt,
                    count,
                    start_time,
                    ..
                } => {
                    self.handle_entity_action(
                        gid,
                        target_gid,
                        action,
                        damage,
                        left_damage,
                        attack_mt,
                        attacked_mt,
                        count,
                        start_time,
                    );
                }
                GameEvent::EntityDirectionChanged { gid, head_dir, dir } => {
                    self.game
                        .entities
                        .apply_entity_direction_changed(gid, head_dir, dir);
                }
                GameEvent::EntityNameReceived { gid, name } => {
                    self.game.entities.apply_entity_name_received(gid, name);
                }
                GameEvent::EntityNamesReceived {
                    gid,
                    name,
                    guild_name,
                    position_name,
                } => {
                    self.game.entities.apply_entity_names_received(
                        gid,
                        name,
                        guild_name,
                        position_name,
                    );
                }
                GameEvent::EntityHpChanged { gid, hp, max_hp } => {
                    self.handle_entity_hp_changed(gid, hp, max_hp);
                }
                GameEvent::EntityOptionChanged {
                    gid,
                    body_state,
                    health_state,
                    effect_state,
                } => {
                    self.handle_entity_option_changed(gid, body_state, health_state, effect_state);
                }
                GameEvent::PlayEffectOnEntity {
                    gid,
                    effect_id,
                    value,
                } => {
                    self.handle_play_effect_on_entity(gid, effect_id, value);
                }
                GameEvent::PlayMiscEffectOnEntity { gid, code } => {
                    self.handle_play_misc_effect_on_entity(gid, code);
                }
                GameEvent::SpiritsChanged { gid, count } => {
                    self.handle_spirits_changed(gid, count);
                }
                GameEvent::BladeStop {
                    src_gid,
                    dest_gid,
                    active,
                } => {
                    for gid in [src_gid, dest_gid] {
                        if let Some(entity) = self.game.entities.get_mut(gid) {
                            entity.rooted = active;
                            if active {
                                entity.movement.stop();
                            }
                        }
                    }
                    if let Some(caster) = self.game.entities.get_mut(src_gid) {
                        caster.forced_animation = active.then(|| {
                            ForcedAnimation::held(
                                SpriteActionType::Skill as usize,
                                BLADE_STOP_GRIP_FRAME,
                            )
                        });
                    }
                }
                GameEvent::StatusEffectChanged {
                    gid,
                    efst,
                    active,
                    remain_ms,
                    val1,
                } => {
                    self.handle_status_effect_changed(gid, efst, active, remain_ms, val1);
                }
                GameEvent::SoundEffect {
                    name,
                    act,
                    term_ms,
                    gid,
                } => {
                    self.handle_sound_effect(name, act, term_ms, gid);
                }
                GameEvent::EntityResurrected { gid } => {
                    self.handle_entity_resurrected(gid);
                }
                GameEvent::MvpReward { gid } => {
                    self.handle_mvp_reward(gid);
                }
                GameEvent::EntitySpriteChanged {
                    gid,
                    sprite_type,
                    value,
                    value2,
                } => {
                    self.handle_entity_sprite_changed(gid, sprite_type, value, value2);
                }
                GameEvent::EntityEmotion { gid, emotion_type } => {
                    self.game.entities.apply_entity_emotion(gid, emotion_type);
                }

                GameEvent::NpcDialogText { npc_id, text } => {
                    self.game.npc_dialog.dialog.open_text(npc_id, &text);
                }
                GameEvent::NpcDialogNext { npc_id } => {
                    self.game.npc_dialog.dialog.wait_for_next(npc_id);
                }
                GameEvent::NpcDialogClose { npc_id } => {
                    if self.game.npc_dialog.dialog.has_text() {
                        self.game.npc_dialog.dialog.wait_for_close(npc_id);
                    } else {
                        self.game.npc_dialog.dialog.close();
                        self.game.npc_cutins = [None, None, None];
                        self.channel
                            .send_packet(build_npc_close_packet(npc_id, self.config.packetver));
                    }
                }
                GameEvent::NpcDialogMenu { npc_id, items } => {
                    self.game.npc_dialog.dialog.show_menu(npc_id, items);
                }
                GameEvent::WarpList {
                    skill_id,
                    destinations,
                } => {
                    self.game.warp_list_window.open(skill_id, destinations);
                }
                GameEvent::NpcInputNumber { npc_id } => {
                    self.game.npc_dialog.dialog.wait_for_number_input(npc_id);
                }
                GameEvent::NpcInputString { npc_id } => {
                    self.game.npc_dialog.dialog.wait_for_string_input(npc_id);
                }
                GameEvent::NpcDealTypeSelect { npc_id } => {
                    self.game.npc_dialog.dialog.show_deal_type(npc_id);
                }

                GameEvent::NpcShopBuyList { npc_id, items } => {
                    self.handle_npc_shop_buy_list(npc_id, items);
                }
                GameEvent::NpcShopSellList { npc_id, items } => {
                    self.handle_npc_shop_sell_list(npc_id, items);
                }
                GameEvent::NpcShopBuyResult { result } => {
                    self.handle_npc_shop_buy_result(result);
                }
                GameEvent::NpcShopSellResult { result } => {
                    self.handle_npc_shop_sell_result(result);
                }

                GameEvent::ChatRoomUpsert {
                    owner_aid,
                    room_id,
                    max_count,
                    cur_count,
                    atype,
                    title,
                } => {
                    self.game.chat_rooms.upsert(ChatRoom {
                        room_id,
                        owner_aid,
                        title,
                        cur_count,
                        max_count,
                        atype,
                    });
                }
                GameEvent::ChatRoomDestroy { room_id } => {
                    self.game.chat_rooms.remove(room_id);
                    if self.game.chat_room_member_window.room_id() == room_id {
                        self.game.chat_room_member_window.close();
                    }
                }
                GameEvent::ChatRoomEntered { room_id, members } => {
                    let (title, max_count, public) = self
                        .game
                        .chat_rooms
                        .get(room_id)
                        .map(|r| (r.title.clone(), r.max_count, r.atype != 0))
                        .unwrap_or_default();
                    let local_name = self.game.character.name.clone();
                    self.game.chat_room_member_window.open_joined(
                        room_id,
                        &title,
                        max_count,
                        public,
                        members,
                        &local_name,
                    );
                    self.game.chat_room_member_window.push_message(
                        "You entered the room.".to_string(),
                        chat_room_member_window::JOIN_MSG_COLOR,
                    );
                    self.game
                        .chat_window
                        .add_system("You entered the room.".to_string());
                }
                GameEvent::ChatRoomCreateResult { flag } => {
                    if flag == 0 {
                        if let Some((title, limit, public)) = self.game.pending_chat_room.take() {
                            let local_name = self.game.character.name.clone();
                            self.game.chat_room_member_window.open_created(
                                0, &title, limit, public, &local_name,
                            );
                        }
                        self.game.chat_room_create_window.close();
                    } else {
                        let reason = match flag {
                            1 => "Room limit exceeded.",
                            2 => "A room with that name already exists.",
                            _ => "Could not create the room.",
                        };
                        self.game.chat_window.add_system(reason.to_string());
                    }
                }
                GameEvent::ChatRoomMemberJoined { name, .. } => {
                    self.game.chat_room_member_window.add_member(&name);
                    let msg = format!("{name} has joined the room.");
                    self.game
                        .chat_room_member_window
                        .push_message(msg.clone(), chat_room_member_window::JOIN_MSG_COLOR);
                    self.game.chat_window.add_system(msg);
                }
                GameEvent::ChatRoomMemberLeft { name, kicked, .. } => {
                    let verb = if kicked { "was kicked from" } else { "has left" };
                    let msg = format!("{name} {verb} the room.");
                    if self.game.chat_room_member_window.is_local(&name) {
                        self.game.chat_room_member_window.close();
                    } else {
                        self.game.chat_room_member_window.remove_member(&name);
                        self.game.chat_room_member_window.push_message(
                            msg.clone(),
                            chat_room_member_window::LEAVE_MSG_COLOR,
                        );
                    }
                    self.game.chat_window.add_system(msg);
                }
                GameEvent::ChatRoomOwnerChanged { name } => {
                    self.game.chat_room_member_window.set_owner(&name);
                    let msg = format!("{name} is now the room owner.");
                    self.game
                        .chat_room_member_window
                        .push_message(msg.clone(), chat_room_member_window::SYSTEM_MSG_COLOR);
                    self.game.chat_window.add_system(msg);
                }
                GameEvent::ChatRoomJoinRefused { result } => {
                    let reason = match result {
                        1 => "Room is full.",
                        2 => "Wrong password.",
                        3 => "You have been kicked from this room.",
                        4 => "Not enough Zeny to enter.",
                        5 => "Your level is too low to enter.",
                        6 => "Your level is too high to enter.",
                        _ => "Cannot enter the room.",
                    };
                    self.game.chat_window.add_system(reason.to_string());
                }

                GameEvent::InventoryNormalItems { items } => {
                    self.handle_inventory_normal_items(items);
                }
                GameEvent::InventoryEquipmentItems { items } => {
                    self.handle_inventory_equipment_items(items);
                }
                GameEvent::InventoryItemPickup {
                    index,
                    item_id,
                    count,
                    item_type,
                    is_identified,
                    is_damaged,
                    refining_level,
                    slot,
                    location,
                    result,
                } => {
                    self.handle_inventory_item_pickup(
                        index,
                        item_id,
                        count,
                        item_type,
                        is_identified,
                        is_damaged,
                        refining_level,
                        slot,
                        location,
                        result,
                    );
                }
                GameEvent::InventoryUseItemResult {
                    index,
                    count,
                    success,
                } => {
                    if success {
                        use ragnarok_game::effect::consumable_effects::{
                            consumable_use_effect, is_mercenary_potion,
                        };
                        let item_id = self
                            .game
                            .character
                            .inventory
                            .get_item(index)
                            .map(|item| item.item_id as u32);
                        let used_effect = item_id.and_then(consumable_use_effect);
                        let target_gid = item_id
                            .filter(|id| is_mercenary_potion(*id))
                            .and(self.game.companions.mercenary.as_ref().map(|m| m.gid))
                            .filter(|gid| *gid != 0)
                            .or_else(|| self.game.entities.player_id());
                        if let (Some(effect), Some(gid)) = (used_effect, target_gid) {
                            self.effect_queue.spawn_on(effect, gid);
                        }
                        self.game
                            .character
                            .inventory
                            .update_item_count(index, count);
                    }
                }
                GameEvent::InventoryEquipResult {
                    index,
                    wear_location,
                    view_id,
                    success,
                } => {
                    self.handle_inventory_equip_result(index, wear_location, view_id, success);
                }
                GameEvent::InventoryArrowEquipped { index } => {
                    let ammo_mask =
                        ragnarok_game::inventory::EquipmentLocation::Ammo.as_flag() as u16;
                    self.game
                        .character
                        .inventory
                        .update_wear_state(index, ammo_mask);
                }
                GameEvent::InventoryUnequipResult {
                    index,
                    wear_location,
                    success,
                } => {
                    self.handle_inventory_unequip_result(index, wear_location, success);
                }
                GameEvent::InventoryItemRemoved { index, count } => {
                    self.game
                        .character
                        .inventory
                        .subtract_item_count(index, count);
                    self.game.combat.waiting_item_throw_ack = false;
                }
                GameEvent::CartNormalItems { items } => {
                    self.handle_cart_normal_items(items);
                }
                GameEvent::CartEquipmentItems { items } => {
                    self.handle_cart_equipment_items(items);
                }
                GameEvent::CartItemAdded {
                    index,
                    item_id,
                    count,
                    item_type,
                    is_identified,
                    is_damaged,
                    refining_level,
                    slot,
                } => {
                    self.handle_cart_item_added(
                        index,
                        item_id,
                        count,
                        item_type,
                        is_identified,
                        is_damaged,
                        refining_level,
                        slot,
                    );
                }
                GameEvent::CartItemRemoved { index, count } => {
                    self.handle_cart_item_removed(index, count);
                }
                GameEvent::CartCountInfo {
                    cur_weight,
                    max_weight,
                    cur_count,
                    max_count,
                } => {
                    self.handle_cart_count_info(cur_weight, max_weight, cur_count, max_count);
                }
                GameEvent::CartOff => {
                    self.handle_cart_off();
                }

                GameEvent::StorageNormalItems { items } => {
                    self.handle_storage_normal_items(items);
                }
                GameEvent::StorageEquipItems { items } => {
                    self.handle_storage_equip_items(items);
                }
                GameEvent::StorageOpened { cur, max } => {
                    self.handle_storage_opened(cur, max);
                }
                GameEvent::StorageItemAdded {
                    index,
                    item_id,
                    count,
                    item_type,
                    is_identified,
                    is_damaged,
                    refining_level,
                    slot,
                } => {
                    self.handle_storage_item_added(
                        index,
                        item_id,
                        count,
                        item_type,
                        is_identified,
                        is_damaged,
                        refining_level,
                        slot,
                    );
                }
                GameEvent::StorageItemRemoved { index, amount } => {
                    self.handle_storage_item_removed(index, amount);
                }
                GameEvent::StorageClosed => {
                    self.handle_storage_closed();
                }

                GameEvent::ExchangeRequested { name, gid, level } => {
                    self.handle_exchange_requested(name, gid, level);
                }
                GameEvent::ExchangeAckResult { result, level } => {
                    self.handle_exchange_ack_result(result, level);
                }
                GameEvent::ExchangeItemAdded {
                    item_id,
                    item_type,
                    count,
                    is_identified,
                    is_damaged,
                    refining_level,
                    slot,
                } => {
                    self.handle_exchange_item_added(
                        item_id,
                        item_type,
                        count,
                        is_identified,
                        is_damaged,
                        refining_level,
                        slot,
                    );
                }
                GameEvent::ExchangeAddResult { index, result } => {
                    self.handle_exchange_add_result(index, result);
                }
                GameEvent::ExchangeConcluded { who } => {
                    self.handle_exchange_concluded(who);
                }
                GameEvent::ExchangeCanceled => {
                    self.handle_exchange_canceled();
                }
                GameEvent::ExchangeCompleted { result } => {
                    self.handle_exchange_completed(result);
                }
                GameEvent::ExchangeUndo => {
                    self.handle_exchange_undo();
                }

                GameEvent::MailWindow { open } => {
                    self.handle_mail_window(open);
                }
                GameEvent::MailInboxReceived { entries } => {
                    self.handle_mail_inbox_received(entries);
                }
                GameEvent::MailOpened { mail } => {
                    self.handle_mail_opened(mail);
                }
                GameEvent::MailDeleteAck { mail_id, ok } => {
                    self.handle_mail_delete_ack(mail_id, ok);
                }
                GameEvent::MailGetItemAck { result } => {
                    self.handle_mail_get_item_ack(result);
                }
                GameEvent::MailAddItemAck { index, ok } => {
                    self.handle_mail_add_item_ack(index, ok);
                }
                GameEvent::MailSendAck { ok } => {
                    self.handle_mail_send_ack(ok);
                }
                GameEvent::MailNewReceived {
                    mail_id,
                    title,
                    sender,
                } => {
                    self.handle_mail_new_received(mail_id, title, sender);
                }
                GameEvent::MailReturnAck { mail_id, ok } => {
                    self.handle_mail_return_ack(mail_id, ok);
                }
                GameEvent::ShowSystemMessage { message } => {
                    self.game.chat_window.add_system(message);
                }

                GameEvent::CardInsertItemList { equip_indices, .. } => {
                    self.handle_card_insert_item_list(equip_indices);
                }
                GameEvent::CardInsertResult {
                    equip_index,
                    card_index,
                    result,
                } => {
                    self.handle_card_insert_result(equip_index, card_index, result);
                }

                GameEvent::FloorItemAppeared {
                    id,
                    item_id,
                    is_identified,
                    x,
                    y,
                    sub_x,
                    sub_y,
                    count,
                    is_falling,
                } => {
                    self.handle_floor_item_appeared(
                        id,
                        item_id,
                        is_identified,
                        x,
                        y,
                        sub_x,
                        sub_y,
                        count,
                        is_falling,
                    );
                }
                GameEvent::FloorItemDisappeared { id } => {
                    self.game.floor_items.remove(&id);
                    self.game.floor_item_sprites.remove(&id);
                }

                GameEvent::ChatMessage { gid, message } => {
                    self.handle_chat_message(gid, message);
                }
                GameEvent::OwnChatMessage { message } => {
                    self.handle_own_chat_message(message);
                }
                GameEvent::RankingReceived { title, entries } => {
                    self.handle_ranking_received(title, entries);
                }
                GameEvent::BroadcastMessage {
                    message,
                    color,
                    banner,
                } => {
                    self.handle_broadcast_message(message, color, banner);
                }

                GameEvent::ParameterChanged { var_id, value } => {
                    self.handle_parameter_changed(var_id, value);
                }
                GameEvent::Recovery { var_id, amount } => {
                    self.handle_recovery(var_id, amount);
                }
                GameEvent::ExpGained {
                    aid,
                    amount,
                    is_base,
                    is_quest,
                } => {
                    self.handle_exp_gained(aid, amount, is_base, is_quest);
                }
                GameEvent::StatusChanged {
                    status_type,
                    base,
                    bonus,
                } => {
                    self.game
                        .character
                        .apply_status_changed(status_type, base, bonus);
                }
                GameEvent::AttackRangeChanged { range } => {
                    self.game.combat.attack_range = range;
                }

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
                    if autocounter::is_kn_autocounter(skill_id)
                        && self.game.entities.player_id() == Some(gid)
                    {
                        self.start_autocounter_channel(gid);
                    } else {
                        let display_name =
                            self.game.data_table.skill_name.as_ref().map(|table| {
                                table.get_display_name_or_internal(&skill_name.unwrap_or_default())
                            });
                        self.game.entities.apply_skill_casting(
                            gid, target_gid, skill_id, delay_ms, x, y, display_name,
                        );
                        self.spawn_skill_begin_cast(skill_id, gid, property, delay_ms);
                    }
                }
                GameEvent::SkillListReceived { skills } => {
                    self.handle_skill_list_received(skills);
                }
                GameEvent::SkillUpdated {
                    id,
                    level,
                    sp_cost,
                    attack_range,
                    upgradable,
                } => {
                    self.game.character.skills.update_skill(
                        id,
                        level,
                        sp_cost,
                        attack_range,
                        upgradable,
                    );
                }
                GameEvent::SkillAdded { skill } => {
                    self.handle_skill_added(skill);
                }
                GameEvent::SkillDamage {
                    skill_id,
                    src_gid,
                    target_gid,
                    damage,
                    attack_mt,
                    attacked_mt,
                    count,
                    level,
                    action,
                    start_time,
                } => {
                    self.handle_skill_damage(
                        skill_id,
                        src_gid,
                        target_gid,
                        damage,
                        attack_mt,
                        attacked_mt,
                        count,
                        level,
                        action,
                        start_time,
                    );
                }
                GameEvent::SkillNoDamage {
                    skill_id,
                    src_gid,
                    target_gid,
                    level,
                } => {
                    if autocounter::is_kn_autocounter(skill_id)
                        && self.game.entities.player_id() == Some(src_gid)
                    {
                        self.start_autocounter_channel(src_gid);
                    } else {
                        self.game.entities.apply_skill_no_damage(
                            skill_id,
                            src_gid,
                            target_gid,
                        );
                        self.spawn_skill_no_damage_effects(skill_id, src_gid, target_gid, level);
                    }
                }
                GameEvent::GroundSkill {
                    skill_id,
                    src_gid,
                    level,
                    x,
                    y,
                } => {
                    self.game
                        .entities
                        .apply_ground_skill(skill_id, src_gid, x, y);
                    self.spawn_ground_skill_effects(skill_id, src_gid, level, x, y);
                    let falcon_target = if self.game.falcons.contains_key(&src_gid)
                        && matches!(
                            SkillEnum::from_id(skill_id as u32),
                            SkillEnum::HtDetecting | SkillEnum::SnSight
                        ) {
                        match (self.game.map_coords.as_ref(), self.game.gat.as_ref()) {
                            (Some(coords), Some(gat)) => {
                                let (wx, _, wz) =
                                    coords.cell_to_world(x as f32 + 0.5, y as f32 + 0.5);
                                Some([wx, gat.get_height(x as f32 + 0.5, y as f32 + 0.5), wz])
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    if let Some(target) = falcon_target {
                        self.start_falcon_flight(src_gid, target);
                    }
                }
                GameEvent::SkillUnitEntered {
                    aid,
                    creator_aid,
                    x,
                    y,
                    unit_id,
                    is_visible,
                } => {
                    self.handle_skill_unit_entered(aid, creator_aid, x, y, unit_id, is_visible);
                }
                GameEvent::SkillUnitDisappeared { aid } => {
                    self.handle_skill_unit_disappeared(aid);
                }
                GameEvent::SkillUnitUpdated { aid } => {
                    self.handle_skill_unit_updated(aid);
                }
                GameEvent::SkillCastCancel { gid } => {
                    self.fire_autocounter_on_cancel(gid);
                    self.game.entities.apply_skill_cast_cancel(gid);
                }
                GameEvent::SkillFailed { skill_id, cause } => {
                    self.handle_skill_failed(skill_id, cause);
                }
                GameEvent::SkillPostDelay { skill_id, delay_ms } => {
                    let now = self.start_time.elapsed().as_secs_f32();
                    self.game.character.cooldowns.set_skill_cooldown(
                        skill_id,
                        delay_ms as f32 / 1000.0,
                        now,
                    );
                }
                GameEvent::AfterCastDelay { delay_ms } => {
                    let now = self.start_time.elapsed().as_secs_f32();
                    self.game
                        .character
                        .cooldowns
                        .set_global_cooldown(delay_ms as f32 / 1000.0, now);
                }

                GameEvent::ServerTick {
                    server_tick,
                    local_send_time_ms,
                } => {
                    self.handle_server_tick(server_tick, local_send_time_ms);
                }
                GameEvent::HotkeyListReceived { slots } => {
                    self.game
                        .character
                        .hotkeys
                        .set_from_server(&slots);
                }
                GameEvent::Disconnected(reason) => {
                    self.handle_disconnected(reason, event_loop);
                }
                GameEvent::ActionFailure => {
                    self.game.combat.attack_target_id = None;
                    self.game.entities.apply_action_failure();
                }

                GameEvent::PartyMemberList { name, members } => {
                    self.handle_party_member_list(name, members);
                }
                GameEvent::PartyMemberAdded {
                    aid,
                    name,
                    map,
                    leader,
                    online,
                    x,
                    y,
                } => {
                    self.handle_party_member_added(aid, name, map, leader, online, x, y);
                }
                GameEvent::PartyMemberRemoved { aid, name, result } => {
                    self.handle_party_member_removed(aid, name, result);
                }
                GameEvent::PartyMemberHp { aid, hp, max_hp } => {
                    self.handle_party_member_hp(aid, hp, max_hp);
                }
                GameEvent::PartyMemberPosition { aid, x, y } => {
                    self.handle_party_member_position(aid, x, y);
                }
                GameEvent::PartyExpOptionChanged { exp_option } => {
                    self.handle_party_exp_option_changed(exp_option);
                }
                GameEvent::SelfConfigChanged { kind, enabled } => {
                    self.handle_self_config_changed(kind, enabled);
                }
                GameEvent::PartyInviteReceived {
                    party_grid,
                    party_name,
                } => {
                    self.handle_party_invite_received(party_grid, party_name);
                }
                GameEvent::PartyInviteResult { name, answer } => {
                    self.handle_party_invite_result(name, answer);
                }
                GameEvent::PartyCreateResult { result } => {
                    self.handle_party_create_result(result);
                }
                GameEvent::PartyChatMessage { aid, message } => {
                    self.handle_party_chat_message(aid, message);
                }
                GameEvent::GuildChatMessage { message } => {
                    self.handle_guild_chat_message(message);
                }
                GameEvent::WhisperReceived { sender, message } => {
                    self.handle_whisper_received(sender, message);
                }
                GameEvent::WhisperAck { result } => {
                    self.handle_whisper_ack(result);
                }
                GameEvent::FriendListReceived { friends } => {
                    self.handle_friend_list_received(friends);
                }
                GameEvent::FriendStateChanged { aid, gid, online } => {
                    self.handle_friend_state_changed(aid, gid, online);
                }
                GameEvent::FriendAddResult {
                    result,
                    aid,
                    gid,
                    name,
                } => {
                    self.handle_friend_add_result(result, aid, gid, name);
                }
                GameEvent::FriendRemoved { aid, gid } => {
                    self.handle_friend_removed(aid, gid);
                }
                GameEvent::FriendRequestReceived {
                    req_aid,
                    req_gid,
                    name,
                } => {
                    self.handle_friend_request_received(req_aid, req_gid, name);
                }

                GameEvent::GuildMenuFlag { flag } => {
                    self.handle_guild_menu_flag(flag);
                }
                GameEvent::GuildInfo {
                    gdid,
                    name,
                    level,
                    exp,
                    max_exp,
                    member_num,
                    max_member_num,
                    avg_level,
                    point,
                    honor,
                    virtue,
                    master_name,
                    manage_land,
                    emblem_version,
                } => {
                    self.handle_guild_info(
                        gdid,
                        name,
                        level,
                        exp,
                        max_exp,
                        member_num,
                        max_member_num,
                        avg_level,
                        point,
                        honor,
                        virtue,
                        master_name,
                        manage_land,
                        emblem_version,
                    );
                }
                GameEvent::GuildMembers { members } => {
                    self.handle_guild_members(members);
                }
                GameEvent::GuildPositions { positions } => {
                    self.handle_guild_positions(positions);
                }
                GameEvent::GuildPositionNames { names } => {
                    self.handle_guild_position_names(names);
                }
                GameEvent::GuildMemberPositionsChanged { entries } => {
                    self.handle_guild_member_positions_changed(entries);
                }
                GameEvent::GuildMemberPosition { aid, x, y } => {
                    if let Some(guild) = &mut self.game.guild {
                        guild.set_position(aid, x, y);
                    }
                }
                GameEvent::GuildSkills { point, skills } => {
                    self.handle_guild_skills(point, skills);
                }
                GameEvent::GuildBanList { entries } => {
                    self.handle_guild_ban_list(entries);
                }
                GameEvent::GuildNotice { subject, body } => {
                    self.handle_guild_notice(subject, body);
                }
                GameEvent::GuildOtherList { guilds } => {
                    self.handle_guild_other_list(guilds);
                }
                GameEvent::GuildRelations { relations } => {
                    self.handle_guild_relations(relations);
                }
                GameEvent::GuildEmblem { gdid, version, bmp } => {
                    self.handle_guild_emblem(gdid, version, bmp);
                }
                GameEvent::GuildIdentityUpdated {
                    gdid,
                    emblem_version,
                    right,
                    is_master,
                    name,
                } => {
                    self.handle_guild_identity_updated(gdid, emblem_version, right, is_master, name);
                }
                GameEvent::GuildCreateResult { result } => {
                    self.handle_guild_create_result(result);
                }
                GameEvent::GuildMemberLeft { name, reason } => {
                    self.handle_guild_member_left(name, reason);
                }
                GameEvent::GuildMemberExpelled { name, reason } => {
                    self.handle_guild_member_expelled(name, reason);
                }
                GameEvent::EntityGuildChanged {
                    aid,
                    gdid,
                    emblem_version,
                } => {
                    self.game
                        .entities
                        .apply_entity_guild_changed(aid, gdid, emblem_version);
                    self.request_entity_guild_emblem(gdid, emblem_version);
                }
                GameEvent::GuildDisbandResult { reason } => {
                    self.handle_guild_disband_result(reason);
                }
                GameEvent::GuildInviteReceived { gdid, name } => {
                    self.handle_guild_invite_received(gdid, name);
                }
                GameEvent::GuildAllyRequestReceived { aid, name } => {
                    self.handle_guild_ally_request_received(aid, name);
                }
                GameEvent::GuildAllyResult { answer } => {
                    self.handle_guild_ally_result(answer);
                }
                GameEvent::GuildHostileResult { result } => {
                    self.handle_guild_hostile_result(result);
                }
                GameEvent::GuildJoinResult { answer } => {
                    self.handle_guild_join_result(answer);
                }
                GameEvent::GuildRelationDeleted { gdid, relation } => {
                    self.handle_guild_relation_deleted(gdid, relation);
                }
                GameEvent::GuildRelationAdded {
                    gdid,
                    relation,
                    name,
                } => {
                    self.handle_guild_relation_added(gdid, relation, name);
                }

                GameEvent::AutoCastSkill { skill_id, level } => {
                    self.handle_auto_cast_skill(skill_id, level);
                }
                GameEvent::ItemIdentifyList { indices } => {
                    self.handle_item_identify_list(indices);
                }
                GameEvent::ItemIdentifyResult { index, ok } => {
                    self.handle_item_identify_result(index, ok);
                }
                GameEvent::MakingArrowList { item_ids } => {
                    self.handle_making_arrow_list(item_ids);
                }
                GameEvent::AutoSpellList { skill_ids } => {
                    self.handle_auto_spell_list(skill_ids);
                }
                GameEvent::WeaponRefineList { items } => {
                    self.handle_weapon_refine_list(items);
                }
                GameEvent::WeaponRefineResult { result, item_id } => {
                    self.handle_weapon_refine_result(result, item_id);
                }
                GameEvent::RepairItemList { items } => {
                    let target_aid = self.game.pending_casts.pending_repair_target.take().unwrap_or(0);
                    self.handle_repair_item_list(target_aid, items);
                }
                GameEvent::RepairItemResult { index, ok } => {
                    self.sound_queue.ui(ragnarok_game::sound::tables::ui::REPAIR);
                    self.handle_repair_item_result(index, ok);
                }
                GameEvent::MakableItemList { item_ids } => {
                    self.handle_makable_item_list(item_ids);
                }
                GameEvent::MakingItemResult { result, item_id } => {
                    self.handle_making_item_result(result, item_id);
                }
                GameEvent::OpenVendingSetup { max_items } => {
                    self.handle_open_vending_setup(max_items);
                }
                GameEvent::VendingOwnStock { items } => {
                    self.handle_vending_own_stock(items);
                }
                GameEvent::VendingBoardShown { aid, name } => {
                    self.handle_vending_board_shown(aid, name);
                }
                GameEvent::VendingBoardHidden { aid } => {
                    self.handle_vending_board_hidden(aid);
                }
                GameEvent::VendingShopList {
                    aid,
                    unique_id,
                    items,
                } => {
                    self.handle_vending_shop_list(aid, unique_id, items);
                }
                GameEvent::VendingPurchaseResult {
                    index,
                    curcount,
                    result,
                } => {
                    self.handle_vending_purchase_result(index, curcount, result);
                }
                GameEvent::VendingStockDecrement { index, count } => {
                    self.handle_vending_stock_decrement(index, count);
                }
                GameEvent::VendingOpenResult { result } => {
                    self.handle_vending_open_result(result);
                }

                GameEvent::CharacterCreated { character } => {
                    self.handle_character_created(character);
                }
                GameEvent::CharacterCreateFailed { error_code } => {
                    if let Some(win) = &mut self.char_create_window {
                        win.error_message =
                            Some(char_create_error_message(error_code).to_string());
                    }
                }
                GameEvent::CharacterDeleteReserved {
                    gid,
                    result,
                    delete_reserved_date,
                } => {
                    if let Some(win) = &mut self.char_select_window {
                        if result == 0 || result == 1 {
                            win.open_delete_dialog(gid, delete_reserved_date);
                        } else {
                            win.set_delete_status(char_delete_reserve_error(result).to_string());
                        }
                    }
                }
                GameEvent::CharacterDeleted { gid, result } => {
                    if let Some(win) = &mut self.char_select_window {
                        if result == 1 {
                            win.remove_character(gid);
                            win.close_delete_dialog();
                            self.account_anims.remove(&gid);
                        } else {
                            win.set_delete_dialog_error(
                                char_delete_confirm_error(result).to_string(),
                            );
                        }
                    }
                }
                GameEvent::CharacterDeleteCancelled { gid: _, result } => {
                    if let Some(win) = &mut self.char_select_window {
                        if result == 1 {
                            win.close_delete_dialog();
                        } else {
                            win.set_delete_dialog_error("Failed to cancel deletion.".to_string());
                        }
                    }
                }

                GameEvent::HomunPropertyReceived { property } => {
                    self.handle_homun_property(property);
                }
                GameEvent::CompanionStateChanged { state, gid, data } => {
                    self.handle_companion_state_changed(state, gid, data);
                }
                GameEvent::HomunFeedResult { success, item_id } => {
                    self.handle_homun_feed_result(success, item_id);
                }
                GameEvent::MercenaryInfoReceived { info, is_init } => {
                    self.handle_mercenary_info(info, is_init);
                }
                GameEvent::MercenaryParamChanged { var, value } => {
                    self.handle_mercenary_param_changed(var, value);
                }
                GameEvent::HomunParamChanged { var, value } => {
                    self.handle_homun_param_changed(var, value);
                }
                GameEvent::HomunSkillList { skills } => {
                    self.handle_homun_skill_list(skills);
                }
                GameEvent::HomunSkillUpdate {
                    id,
                    level,
                    sp_cost,
                    attack_range,
                    upgradable,
                } => {
                    self.handle_homun_skill_update(id, level, sp_cost, attack_range, upgradable);
                }
                GameEvent::MercenarySkillList { skills } => {
                    self.handle_mercenary_skill_list(skills);
                }
                GameEvent::MercenarySkillUpdate {
                    id,
                    level,
                    sp_cost,
                    attack_range,
                    upgradable,
                } => {
                    self.handle_mercenary_skill_update(id, level, sp_cost, attack_range, upgradable);
                }

                GameEvent::PetCaptureStart => {
                    self.handle_pet_capture_start();
                }
                GameEvent::PetCaptureResult { ok } => {
                    self.handle_pet_capture_result(ok);
                }
                GameEvent::PetProperty { property } => {
                    self.handle_pet_property(property);
                }
                GameEvent::PetFeedResult { ok, food_item_id } => {
                    self.handle_pet_feed_result(ok, food_item_id);
                }
                GameEvent::PetStateChanged { ty, gid, data } => {
                    self.handle_pet_state_changed(ty, gid, data);
                }
                GameEvent::PetEggList { indices } => {
                    self.handle_pet_egg_list(indices);
                }
                GameEvent::PetAct { gid, data } => {
                    self.handle_pet_act(gid, data);
                }

                GameEvent::QuestListReceived { quests } => {
                    self.handle_quest_list_received(quests);
                }
                GameEvent::QuestMissionsReceived { missions } => {
                    self.handle_quest_missions_received(missions);
                }
                GameEvent::QuestAdded { quest } => {
                    self.handle_quest_added(quest);
                }
                GameEvent::QuestRemoved { quest_id } => {
                    self.handle_quest_removed(quest_id);
                }
                GameEvent::QuestHuntUpdated { entries } => {
                    self.handle_quest_hunt_updated(entries);
                }
                GameEvent::QuestActiveChanged { quest_id, active } => {
                    self.handle_quest_active_changed(quest_id, active);
                }
                GameEvent::QuestNpcMarker {
                    npc_id,
                    x,
                    y,
                    effect,
                    color,
                } => {
                    self.handle_quest_npc_marker(npc_id, x, y, effect, color);
                }

                GameEvent::CoupleNameReceived { name } => {
                    self.game.character.partner_name = name;
                }
                GameEvent::WeddingCelebration { account_id } => {
                    self.handle_wedding_celebration(account_id);
                }
                GameEvent::Divorced { name } => {
                    self.handle_divorced(name);
                }
                GameEvent::NpcCutin { image, position } => {
                    self.handle_npc_cutin(image, position);
                }
                GameEvent::AdoptionRequested {
                    father_aid,
                    mother_aid,
                    name,
                } => {
                    self.handle_adoption_requested(father_aid, mother_aid, name);
                }
                GameEvent::AdoptionMessage { msg_no } => {
                    self.handle_adoption_message(msg_no);
                }

                _ => {}
            }
        }
        self.game.entities.clear_just_spawned_flags();
    }
}
