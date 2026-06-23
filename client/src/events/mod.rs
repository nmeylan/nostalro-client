mod character;
mod chat;
mod connection;
mod entity;
mod inventory;
mod login;
mod npc;
mod skill;

use crate::App;
use models::enums::EnumWithMaskValueU64;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::chat_room::ChatRoom;
use ragnarok_game::event::GameEvent;
use ragnarok_network::build_npc_close_packet;
use ragnarok_renderer::Renderer;
use ragnarok_ui_component::Window as UiWindow;
use winit::event_loop::ActiveEventLoop;

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
                // Login / account
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

                // Map / player
                GameEvent::MapEntered { x, y, dir, .. } => {
                    self.handle_map_entered(x, y, dir);
                }
                GameEvent::MapChanged { map_name, x, y } => {
                    self.handle_map_changed(map_name, x, y);
                }
                GameEvent::MapPropertyChanged(properties) => {
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

                // Entities
                GameEvent::EntitySpawned {
                    gid,
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
                } => {
                    self.handle_entity_spawned(
                        gid,
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
                GameEvent::EntityHpChanged { gid, hp, max_hp } => {
                    self.handle_entity_hp_changed(gid, hp, max_hp);
                }
                GameEvent::EntityOptionChanged {
                    gid,
                    body_state,
                    health_state,
                    effect_state,
                } => {
                    self.handle_entity_option_changed(
                        gid,
                        body_state,
                        health_state,
                        effect_state,
                    );
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
                GameEvent::StatusEffectChanged {
                    gid,
                    efst,
                    active,
                    remain_ms,
                } => {
                    self.handle_status_effect_changed(gid, efst, active, remain_ms);
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

                // NPC dialog
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

                // NPC shop
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

                // Chat room / waitingroom
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
                }
                GameEvent::ChatRoomEntered { .. } => {
                    self.game
                        .chat_window
                        .add_system("You entered the room.".to_string());
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

                // Inventory
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
                    self.game.waiting_item_throw_ack = false;
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

                // Floor items
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

                // Chat
                GameEvent::ChatMessage { gid, message } => {
                    self.handle_chat_message(gid, message);
                }
                GameEvent::OwnChatMessage { message } => {
                    self.handle_own_chat_message(message);
                }

                // Character stats
                GameEvent::ParameterChanged { var_id, value } => {
                    self.handle_parameter_changed(var_id, value);
                }
                GameEvent::StatusChanged {
                    status_type, base, bonus,
                } => {
                    self.game.character.apply_status_changed(status_type, base, bonus);
                }
                GameEvent::AttackRangeChanged { range } => {
                    self.game.attack_range = range;
                }

                // Skills
                GameEvent::SkillCasting {
                    gid,
                    target_gid,
                    skill_id,
                    property,
                    delay_ms,
                    x,
                    y,
                } => {
                    self.game
                        .entities
                        .apply_skill_casting(gid, target_gid, skill_id, delay_ms, x, y);
                    self.spawn_skill_begin_cast(skill_id, gid, property, delay_ms);
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
                    skill_name,
                    start_time,
                } => {
                    let display_name = self.game.data_table.skill_name.as_ref().map(|table| {
                        table.get_display_name_or_internal(&skill_name.clone().unwrap_or_default())
                    });
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
                        display_name,
                        start_time,
                    );
                }
                GameEvent::SkillNoDamage {
                    skill_id,
                    src_gid,
                    target_gid,
                    skill_name,
                } => {
                    let display_name = self.game.data_table.skill_name.as_ref().map(|table| {
                        table.get_display_name_or_internal(&skill_name.clone().unwrap_or_default())
                    });
                    self.game.entities.apply_skill_no_damage(
                        skill_id,
                        src_gid,
                        target_gid,
                        display_name,
                    );
                    self.spawn_skill_no_damage_effects(skill_id, src_gid, target_gid);
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
                    self.spawn_ground_skill_effects(skill_id, level, x, y);
                }
                GameEvent::SkillUnitEntered {
                    aid,
                    creator_aid: _,
                    x,
                    y,
                    unit_id,
                    is_visible,
                } => {
                    self.handle_skill_unit_entered(aid, x, y, unit_id, is_visible);
                }
                GameEvent::SkillUnitDisappeared { aid } => {
                    self.handle_skill_unit_disappeared(aid);
                }
                GameEvent::SkillCastCancel { gid } => {
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

                // Misc
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
                        .set_from_server(&slots, self.game.character.inventory.all_items());
                }
                GameEvent::Disconnected(reason) => {
                    self.handle_disconnected(reason, event_loop);
                }
                GameEvent::ActionFailure => {
                    self.game.attack_target_id = None;
                    self.game.entities.apply_action_failure();
                }

                _ => {}
            }
        }
        self.game.entities.clear_just_spawned_flags();
    }
}
