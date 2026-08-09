use std::collections::HashMap;

use crate::entity::{EmotionState, Entity, EntityState, EntityType, ForcedAnimation};
use crate::mob_info::MobInfo;
use crate::movement::direction_from_positions;
use crate::skill_action::skill_pose;
use models::enums::skill_enums::SkillEnum;

/// How long a caster stays in its skill motion for a ground-placed skill.
pub const GROUND_SKILL_EXEC_SECS: f32 = 0.6;
const NO_DAMAGE_SKILL_EXEC_SECS: f32 = 0.6;

pub struct EntityCollection {
    entities: HashMap<u32, Entity>,
    player_id: Option<u32>,
    /// Maps a player's account id (the actor id used by name/guild packets) to
    /// the entity key. For players the two differ; for other actors they match.
    account_to_key: HashMap<u32, u32>,
}

impl Default for EntityCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityCollection {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            player_id: None,
            account_to_key: HashMap::new(),
        }
    }

    pub fn register_account_id(&mut self, account_id: u32, key: u32) {
        if account_id != key {
            self.account_to_key.insert(account_id, key);
        }
    }

    /// Resolves an actor id from a name/guild packet to an entity key, following
    /// the account-id bridge when the id is a player's account id.
    pub fn resolve_key(&self, id: u32) -> u32 {
        self.account_to_key.get(&id).copied().unwrap_or(id)
    }

    /// Inverse of [`Self::resolve_key`]: the account id a CZ request must carry
    /// to address the entity behind `key`. Every player-targeted request (trade,
    /// party/guild invite, adoption, manner points) is keyed by account id, not
    /// by the char id the entity is stored under.
    pub fn account_id_of(&self, key: u32) -> u32 {
        self.account_to_key
            .iter()
            .find(|(_, k)| **k == key)
            .map(|(account_id, _)| *account_id)
            .unwrap_or(key)
    }

    pub fn set_player_id(&mut self, id: u32) {
        self.player_id = Some(id);
    }

    pub fn player_id(&self) -> Option<u32> {
        self.player_id
    }

    pub fn player(&self) -> Option<&Entity> {
        self.player_id.and_then(|id| self.entities.get(&id))
    }

    pub fn player_mut(&mut self) -> Option<&mut Entity> {
        self.player_id.and_then(|id| self.entities.get_mut(&id))
    }

    pub fn player_job(&self) -> u16 {
        self.player().map_or(0, |e| e.job)
    }

    pub fn insert(&mut self, entity: Entity) {
        self.entities.insert(entity.id, entity);
    }

    pub fn remove(&mut self, id: u32) -> Option<Entity> {
        self.account_to_key.retain(|_, key| *key != id);
        self.entities.remove(&id)
    }

    pub fn get(&self, id: u32) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Entity> {
        self.entities.values_mut()
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.account_to_key.clear();
        self.player_id = None;
    }

    pub fn is_player(&self, id: u32) -> bool {
        self.player_id == Some(id)
    }

    pub fn clear_just_spawned_flags(&mut self) {
        for entity in self.entities.values_mut() {
            entity.just_spawned = false;
        }
    }

    pub fn clear_non_player(&mut self) {
        if let Some(pid) = self.player_id {
            self.entities.retain(|&id, _| id == pid);
            self.account_to_key.retain(|_, key| *key == pid);
        } else {
            self.entities.clear();
            self.account_to_key.clear();
        }
    }

    pub fn apply_entity_stop_move(&mut self, gid: u32, x: u16, y: u16) {
        if let Some(entity) = self.entities.get_mut(&gid) {
            entity.movement.set_position(x as f32, y as f32);
            if entity.state == EntityState::Moving {
                entity.state = EntityState::Standing;
            }
        }
    }

    pub fn apply_entity_direction_changed(&mut self, gid: u32, head_dir: u8, dir: u8) {
        if let Some(entity) = self.entities.get_mut(&gid) {
            entity.head_dir = head_dir;
            entity.set_facing(dir);
        }
    }

    pub fn apply_entity_name_received(&mut self, gid: u32, name: String) {
        let key = self.resolve_key(gid);
        if let Some(entity) = self.entities.get_mut(&key) {
            entity.name = Some(name);
        }
    }

    pub fn apply_entity_guild_changed(&mut self, aid: u32, gdid: u32, emblem_version: i32) {
        let key = self.resolve_key(aid);
        if let Some(entity) = self.entities.get_mut(&key) {
            entity.guild_id = gdid;
            entity.guild_emblem_version = emblem_version;
            if gdid < 1 {
                entity.guild_name = None;
            }
        }
    }

    pub fn apply_entity_names_received(
        &mut self,
        gid: u32,
        name: String,
        party_name: String,
        guild_name: String,
        position_name: String,
    ) {
        let key = self.resolve_key(gid);
        if let Some(entity) = self.entities.get_mut(&key) {
            entity.name = Some(name);
            entity.guild_name = (!guild_name.is_empty()).then_some(guild_name);
            entity.position_name = (!position_name.is_empty()).then_some(position_name);
            if entity.entity_type == EntityType::Monster {
                entity.mob_info = MobInfo::parse(&party_name);
            }
        }
    }

    pub fn apply_entity_emotion(&mut self, gid: u32, emotion_type: u8, duration: f32) {
        if let Some(entity) = self.entities.get_mut(&gid) {
            entity.emotion = Some(EmotionState::new(emotion_type, duration));
        }
    }

    pub fn apply_skill_casting(
        &mut self,
        gid: u32,
        target_gid: u32,
        skill_id: u16,
        delay_ms: u32,
        x: i16,
        y: i16,
        skill_name: Option<String>,
    ) {
        self.show_skill_chat_bubble(gid, skill_name);
        let target_pos = if target_gid != 0 {
            self.entities
                .get(&target_gid)
                .map(|e| e.movement.cell_position())
        } else if x != 0 || y != 0 {
            Some((x as u16, y as u16))
        } else {
            None
        };
        if let Some(entity) = self.entities.get_mut(&gid) {
            if let Some(tp) = target_pos {
                let sp = entity.movement.cell_position();
                if let Some(dir) = direction_from_positions(sp.0, sp.1, tp.0, tp.1) {
                    entity.set_facing(dir);
                }
            }
            let duration = delay_ms as f32 / 1000.0;
            if duration > 0.0 {
                entity.enter_casting(duration, skill_id);
            } else {
                entity.enter_skill_exec(0.3, skill_id, 1);
            }
        }
    }

    pub fn apply_autocounter_channel(
        &mut self,
        gid: u32,
        face_gid: Option<u32>,
        skill_id: u16,
        duration_secs: f32,
    ) {
        let face_pos =
            face_gid.and_then(|g| self.entities.get(&g).map(|e| e.movement.cell_position()));
        if let Some(entity) = self.entities.get_mut(&gid) {
            if let Some((tx, ty)) = face_pos {
                let (sx, sy) = entity.movement.cell_position();
                if let Some(dir) = direction_from_positions(sx, sy, tx, ty) {
                    entity.set_facing(dir);
                }
            }
            entity.enter_casting(duration_secs, skill_id);
        }
    }

    pub fn apply_skill_no_damage(&mut self, skill_id: u16, src_gid: u32, target_gid: u32) {
        if skill_id == SkillEnum::TkRun.id() as u16
            && !self.entities.get(&src_gid).is_some_and(|e| e.is_running)
        {
            return;
        }
        let target_pos = self
            .entities
            .get(&target_gid)
            .map(|e| e.movement.cell_position());
        if let Some(entity) = self.entities.get_mut(&src_gid) {
            if let Some(dst) = target_pos {
                let src = entity.movement.cell_position();
                if let Some(dir) = direction_from_positions(src.0, src.1, dst.0, dst.1) {
                    entity.set_facing(dir);
                }
            }
            let pose = skill_pose(skill_id);
            let duration = pose.map_or(NO_DAMAGE_SKILL_EXEC_SECS, |p| p.hold_secs);
            entity.enter_skill_exec(duration, skill_id, 1);
            if let Some(pose) = pose {
                entity.forced_animation = Some(ForcedAnimation::held_for(
                    pose.action,
                    pose.frame,
                    pose.hold_secs * 1000.0,
                ));
            }
        }
    }

    fn show_skill_chat_bubble(&mut self, gid: u32, skill_name: Option<String>) {
        if let Some(name) = skill_name
            && let Some(entity) = self.entities.get_mut(&gid)
        {
            if entity.entity_type != EntityType::Monster {
                entity.chat_bubble =
                    Some(crate::entity::ChatBubbleState::new(format!("{} !!", name)));
            }
        }
    }

    pub fn apply_ground_skill(&mut self, skill_id: u16, src_gid: u32, x: i16, y: i16) {
        if let Some(entity) = self.entities.get_mut(&src_gid) {
            let sp = entity.movement.cell_position();
            if let Some(dir) = direction_from_positions(sp.0, sp.1, x as u16, y as u16) {
                entity.set_facing(dir);
            }
            entity.enter_skill_exec(GROUND_SKILL_EXEC_SECS, skill_id, 1);
        }
    }

    pub fn apply_skill_cast_cancel(&mut self, gid: u32) {
        let target_gid = if gid == 0 {
            self.player_id.unwrap_or(0)
        } else {
            gid
        };
        if let Some(entity) = self.entities.get_mut(&target_gid) {
            entity.state = EntityState::Standing;
            entity.state_timer = 0.0;
            entity.clear_cast();
            entity.active_skill_id = None;
        }
    }

    pub fn apply_action_failure(&mut self) {
        if let Some(entity) = self.player_mut() {
            entity.state = EntityState::Standing;
            entity.state_timer = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autocounter::channel_params;
    use crate::character::{Character, job_class_name};
    use crate::entity::EntityType;
    use crate::skill::{SkillData, SkillTargetType};
    use models::enums::EnumWithNumberValue;
    use models::enums::class::JobName;

    fn make_entity(id: u32) -> Entity {
        Entity::new(
            id,
            EntityType::Player,
            0,
            1,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            150,
        )
    }

    #[test]
    fn player_job_follows_base_look_change() {
        let mut col = EntityCollection::new();
        col.set_player_id(100);
        let mut player = make_entity(100);
        player.job = JobName::Swordsman.value() as u16;
        col.insert(player);
        col.insert(make_entity(200));

        assert_eq!(job_class_name(col.player_job()), "Swordsman");

        col.get_mut(100)
            .unwrap()
            .apply_sprite_change(0, JobName::Knight.value() as u16);

        assert_eq!(job_class_name(col.player_job()), "Knight");
    }

    #[test]
    fn insert_get_remove_and_player() {
        let mut col = EntityCollection::new();
        col.set_player_id(100);
        col.insert(make_entity(100));
        col.insert(make_entity(200));
        col.insert(make_entity(300));

        assert!(col.player().is_some());
        assert_eq!(col.player().unwrap().id, 100);
        assert!(col.get(200).is_some());
        assert!(col.get(999).is_none());

        col.remove(200);
        assert!(col.get(200).is_none());

        col.clear_non_player();
        assert!(col.player().is_some());
        assert!(col.get(300).is_none());
    }

    #[test]
    fn names_resolve_through_account_id_bridge() {
        let mut col = EntityCollection::new();
        col.insert(make_entity(2000100));
        col.register_account_id(2000101, 2000100);

        col.apply_entity_names_received(
            2000101,
            "Stalker".to_string(),
            String::new(),
            "rg".to_string(),
            "GuildMaster".to_string(),
        );

        let e = col.get(2000100).unwrap();
        assert_eq!(e.name.as_deref(), Some("Stalker"));
        assert_eq!(e.guild_name.as_deref(), Some("rg"));
        assert_eq!(e.position_name.as_deref(), Some("GuildMaster"));

        col.remove(2000100);
        col.insert(make_entity(2000101));
        col.apply_entity_name_received(2000101, "Other".to_string());
        assert_eq!(col.get(2000101).unwrap().name.as_deref(), Some("Other"));
    }

    #[test]
    fn picked_entity_resolves_back_to_the_account_id_requests_are_addressed_to() {
        let mut col = EntityCollection::new();
        col.insert(make_entity(2000100));
        col.register_account_id(2000101, 2000100);
        col.insert(make_entity(110000));

        assert_eq!(col.account_id_of(2000100), 2000101);
        assert_eq!(col.resolve_key(col.account_id_of(2000100)), 2000100);
        assert_eq!(col.account_id_of(110000), 110000);
    }

    #[test]
    fn option_change_under_account_id_cloaks_another_player() {
        use crate::sprite_path::{HiddenRender, HiddenViewer, OPTION_CLOAK, hidden_render};

        let mut col = EntityCollection::new();
        col.insert(make_entity(2000100));
        col.register_account_id(2000101, 2000100);

        let key = col.resolve_key(2000101);
        col.get_mut(key).unwrap().effect_state = OPTION_CLOAK;

        let e = col.get(2000100).unwrap();
        assert_eq!(
            hidden_render(e.effect_state, HiddenViewer::Other),
            HiddenRender::Skip
        );
    }

    #[test]
    fn mob_info_string_is_parsed_for_monsters_only() {
        let mut col = EntityCollection::new();
        let mut monster = make_entity(100);
        monster.entity_type = EntityType::Monster;
        col.insert(monster);
        col.insert(make_entity(200));

        for id in [100, 200] {
            col.apply_entity_names_received(
                id,
                "Poring".to_string(),
                "HP: 100/200".to_string(),
                String::new(),
                String::new(),
            );
        }

        let monster = col.get(100).unwrap();
        assert_eq!(monster.mob_info.as_ref().unwrap().hp_ratio(), Some(0.5));
        assert_eq!(monster.hp, None);

        let player = col.get(200).unwrap();
        assert!(player.mob_info.is_none());
        assert_eq!(player.name.as_deref(), Some("Poring"));
    }

    #[test]
    fn stop_move_updates_position_and_state() {
        let mut col = EntityCollection::new();
        let mut entity = make_entity(100);
        entity.state = EntityState::Moving;
        col.insert(entity);

        col.apply_entity_stop_move(100, 10, 20);
        let e = col.get(100).unwrap();
        assert_eq!(e.movement.cell_position(), (10, 20));
        assert_eq!(e.state, EntityState::Standing);
    }

    #[test]
    fn stop_move_ignores_non_moving_state() {
        let mut col = EntityCollection::new();
        let mut entity = make_entity(100);
        entity.state = EntityState::Attacking;
        col.insert(entity);

        col.apply_entity_stop_move(100, 5, 5);
        let e = col.get(100).unwrap();
        assert_eq!(e.movement.cell_position(), (5, 5));
        assert_eq!(e.state, EntityState::Attacking);
    }

    #[test]
    fn skill_cast_cancel_resets_casting_state() {
        let mut col = EntityCollection::new();
        let mut entity = make_entity(100);
        entity.enter_casting(2.0, 42);
        assert_eq!(entity.state, EntityState::Casting);
        col.insert(entity);

        col.apply_skill_cast_cancel(100);
        let e = col.get(100).unwrap();
        assert_eq!(e.state, EntityState::Standing);
        assert_eq!(e.state_timer, 0.0);
        assert!(e.active_skill_id.is_none());
    }

    #[test]
    fn guild_changed_sets_emblem_and_clears_tag_on_leave() {
        let mut col = EntityCollection::new();
        let mut entity = make_entity(100);
        entity.guild_name = Some("Knights".to_string());
        col.insert(entity);

        col.apply_entity_guild_changed(100, 7, 42);
        let e = col.get(100).unwrap();
        assert_eq!(e.guild_id, 7);
        assert_eq!(e.guild_emblem_version, 42);
        assert_eq!(e.guild_name.as_deref(), Some("Knights"));

        col.apply_entity_guild_changed(100, 0, 0);
        let e = col.get(100).unwrap();
        assert_eq!(e.guild_id, 0);
        assert!(e.guild_name.is_none());
    }

    #[test]
    fn skill_cast_cancel_zero_targets_player() {
        let mut col = EntityCollection::new();
        col.set_player_id(100);
        let mut entity = make_entity(100);
        entity.enter_casting(1.5, 10);
        col.insert(entity);

        col.apply_skill_cast_cancel(0);
        assert_eq!(col.player().unwrap().state, EntityState::Standing);
    }

    fn make_entity_at(id: u32, x: u16, y: u16) -> Entity {
        Entity::new(
            id,
            EntityType::Player,
            0,
            1,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            x,
            y,
            0,
            150,
        )
    }

    #[test]
    fn autocounter_channel_freezes_caster_facing_the_enemy() {
        let mut col = EntityCollection::new();
        col.set_player_id(100);
        col.insert(make_entity_at(100, 10, 10));
        col.insert(make_entity_at(200, 10, 15));

        col.apply_autocounter_channel(100, Some(200), 61, 1.5);

        let e = col.get(100).unwrap();
        assert_eq!(e.state, EntityState::Casting);
        assert_eq!(e.state_timer, 1.5);
        assert_eq!(e.cast_total_duration, 1.5);
        assert_eq!(e.active_skill_id, Some(61));
        assert_eq!(
            e.direction,
            direction_from_positions(10, 10, 10, 15).unwrap()
        );

        let mut character = Character::default();
        character.skills.set_skills(vec![SkillData {
            id: 61,
            name: "KN_AUTOCOUNTER".to_string(),
            level: 3,
            selected_level: 3,
            sp_cost: 0,
            attack_range: 0,
            upgradable: false,
            skill_target_type: SkillTargetType::Target,
        }]);
        let params = channel_params(&character, true, Some(200), None);
        assert_eq!(params.duration, 3.0 * 0.4);
        assert_eq!(params.face, Some(200));
        assert_eq!(
            channel_params(&character, false, Some(200), Some(300)).face,
            None
        );
    }

    #[test]
    fn action_failure_resets_player_only() {
        let mut col = EntityCollection::new();
        col.set_player_id(100);
        let mut player = make_entity(100);
        player.state = EntityState::Attacking;
        player.state_timer = 1.0;
        col.insert(player);

        let mut other = make_entity(200);
        other.state = EntityState::Attacking;
        col.insert(other);

        col.apply_action_failure();
        assert_eq!(col.player().unwrap().state, EntityState::Standing);
        assert_eq!(col.player().unwrap().state_timer, 0.0);
        assert_eq!(col.get(200).unwrap().state, EntityState::Attacking);
    }

    #[test]
    fn direction_and_name_and_emotion() {
        let mut col = EntityCollection::new();
        col.insert(make_entity(100));

        col.apply_entity_direction_changed(100, 2, 5);
        let e = col.get(100).unwrap();
        assert_eq!(e.head_dir, 2);
        assert_eq!(e.direction, 5);

        col.apply_entity_name_received(100, "Poring".to_string());
        assert_eq!(col.get(100).unwrap().name.as_deref(), Some("Poring"));

        col.apply_entity_emotion(100, 3, 1.4);
        assert!(col.get(100).unwrap().emotion.is_some());
        assert_eq!(
            col.get(100).unwrap().emotion.as_ref().unwrap().emotion_type,
            3
        );
    }

    #[test]
    fn skill_no_damage_faces_target() {
        let mut col = EntityCollection::new();
        let mut caster = Entity::new(
            1,
            EntityType::Player,
            0,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            5,
            5,
            0,
            150,
        );
        caster.set_facing(0);
        col.insert(caster);
        col.insert(Entity::new(
            2,
            EntityType::Monster,
            1002,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            8,
            5,
            0,
            200,
        ));

        col.apply_skill_no_damage(10, 1, 2);
        let e = col.get(1).unwrap();
        assert_eq!(e.direction, 6);
        assert_eq!(e.state, EntityState::SkillExec);
    }

    #[test]
    fn run_toggle_plays_motion_on_start_but_not_when_stopping() {
        let run_id = SkillEnum::TkRun.id() as u16;

        // Start: the EFST_RUN change already flipped is_running on, so the skill
        // packet plays the run motion.
        let mut col = EntityCollection::new();
        let mut starter = make_entity(1);
        starter.is_running = true;
        col.insert(starter);
        col.apply_skill_no_damage(run_id, 1, 0);
        assert_eq!(col.get(1).unwrap().state, EntityState::SkillExec);

        // Stop: is_running already flipped off, so the skill packet is ignored
        // and the character does not re-enter a walk motion in place.
        let mut col = EntityCollection::new();
        let mut runner = make_entity(1);
        runner.state = EntityState::Standing;
        col.insert(runner);
        col.apply_skill_no_damage(run_id, 1, 0);
        assert_eq!(col.get(1).unwrap().state, EntityState::Standing);
    }

    #[test]
    fn prepare_kick_freezes_the_caster_on_its_stance_frame() {
        let mut col = EntityCollection::new();
        col.insert(make_entity(1));
        col.apply_skill_no_damage(SkillEnum::TkReadyturn.id() as u16, 1, 0);

        let e = col.get(1).unwrap();
        assert_eq!(e.state, EntityState::SkillExec);
        assert_eq!(e.action_index(), 12);
        assert_eq!(e.skill_exec_start_frame(), 3);
        let mut forced = e.forced_animation.expect("stance poses the body");
        assert!(forced.hold);
        assert_eq!((forced.action, forced.start_frame), (12, 3));
        assert!(forced.tick_hold(1.0), "held for two seconds");
        assert!(!forced.tick_hold(1.5));

        // A skill without a stance leaves the body animating.
        col.insert(make_entity(2));
        col.apply_skill_no_damage(SkillEnum::AlHeal.id() as u16, 2, 0);
        assert!(col.get(2).unwrap().forced_animation.is_none());
    }
}
