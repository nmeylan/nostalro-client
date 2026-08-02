use crate::cursor::CursorType;
use crate::entity::{Entity, EntityCategory, EntityState, EntityType};
use models::enums::EnumWithMaskValueU64;
use models::enums::map::MapPropertyFlags;
use models::enums::skill::SkillTargetType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MapKind {
    #[default]
    Normal,
    FreePvp,
    EventPvp,
    Agit,
    PkServer,
    PvpServer,
    DenySkill,
}

impl MapKind {
    pub fn from_property(value: i16) -> Self {
        match value {
            1 => MapKind::FreePvp,
            2 => MapKind::EventPvp,
            3 => MapKind::Agit,
            4 => MapKind::PkServer,
            5 => MapKind::PvpServer,
            6 => MapKind::DenySkill,
            _ => MapKind::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MapProperties {
    pub kind: MapKind,
    pub flags: u64,
}

impl MapProperties {
    pub fn from_kind(kind: MapKind) -> Self {
        Self { kind, flags: 0 }
    }

    pub fn with_flags(kind: MapKind, flags: u64) -> Self {
        Self { kind, flags }
    }

    fn has(&self, flag: MapPropertyFlags) -> bool {
        self.flags & flag.as_flag() != 0
    }

    pub fn is_pvp(&self) -> bool {
        matches!(
            self.kind,
            MapKind::FreePvp | MapKind::EventPvp | MapKind::PkServer | MapKind::PvpServer
        ) || self.has(MapPropertyFlags::IsParty)
    }

    pub fn is_gvg(&self) -> bool {
        matches!(self.kind, MapKind::Agit) || self.has(MapPropertyFlags::IsGuild)
    }

    pub fn is_siege(&self) -> bool {
        matches!(self.kind, MapKind::Agit) || self.has(MapPropertyFlags::IsSiege)
    }

    pub fn no_lockon(&self) -> bool {
        self.has(MapPropertyFlags::IsNoLockOn)
    }

    pub fn is_pk_zone(&self) -> bool {
        matches!(self.kind, MapKind::FreePvp | MapKind::PvpServer)
    }

    pub fn count_pk(&self) -> bool {
        self.is_pvp() || self.has(MapPropertyFlags::CountPk)
    }

    pub fn enable_pk(&self) -> bool {
        self.is_pvp() || self.is_gvg()
    }
}

pub const EFFECT_STATE_PINK_NAME: i32 = 0x80000;
pub const EFFECT_STATE_RED_NAME: i32 = 0x100000;

/// Name, guild name and chat color for GM (admin) accounts.
pub const GM_TEXT_COLOR: [f32; 4] = [1.0, 1.0, 0.0, 1.0];

pub fn pk_name_color(effect_state: i32) -> Option<[f32; 4]> {
    if effect_state & EFFECT_STATE_RED_NAME != 0 {
        Some([1.0, 0.0, 0.0, 1.0])
    } else if effect_state & EFFECT_STATE_PINK_NAME != 0 {
        Some([1.0, 0.392, 0.722, 1.0])
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetClass {
    Offensive,
    Supportive,
    SelfOnly,
    Ground,
    /// Cast on a deployed skill unit (a trap), not on an entity or a cell.
    SkillUnit,
}

pub fn skill_target_class(target_type: SkillTargetType) -> TargetClass {
    match target_type {
        SkillTargetType::Trap => TargetClass::SkillUnit,
        SkillTargetType::Ground => TargetClass::Ground,
        SkillTargetType::MySelf | SkillTargetType::Party => TargetClass::SelfOnly,
        SkillTargetType::Friend => TargetClass::Supportive,
        SkillTargetType::Target | SkillTargetType::Passive => TargetClass::Offensive,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relationship {
    Myself,
    Party,
    Guild,
    Other,
}

pub fn relationship(target_id: u32, player_id: Option<u32>) -> Relationship {
    if Some(target_id) == player_id {
        Relationship::Myself
    } else {
        Relationship::Other
    }
}

pub fn can_attack(target: &Entity, map: &MapProperties, player_id: Option<u32>) -> bool {
    match target.entity_type {
        EntityType::Monster => !target.is_pet,
        EntityType::Player => {
            map.enable_pk() && relationship(target.id, player_id) == Relationship::Other
        }
        EntityType::Npc | EntityType::Homunculus | EntityType::Mercenary => false,
    }
}

pub fn hover_cursor(
    target: &Entity,
    map: &MapProperties,
    active_skill: Option<TargetClass>,
    player_id: Option<u32>,
) -> Option<CursorType> {
    if target.is_fading() {
        return None;
    }
    if target.state == EntityState::Dead {
        // Resurrection is cast on a corpse, so a supportive skill keeps a dead
        // player pickable — with the plain cursor, not the lock one.
        return (active_skill == Some(TargetClass::Supportive)
            && target.entity_type == EntityType::Player)
            .then_some(CursorType::Default);
    }
    let category = target.category();
    if matches!(category, EntityCategory::Skill | EntityCategory::Invisible)
        || crate::sprite_path::is_inert_actor(target.job)
    {
        return None;
    }
    if active_skill.is_none() {
        match category {
            EntityCategory::WarpPoint => return Some(CursorType::Warp),
            EntityCategory::Npc => return Some(CursorType::Talk),
            _ => {}
        }
    }
    // Companions are interactable (right-click menu / owner commands) but not
    // attackable — make them pickable with the click cursor when no skill is armed.
    if (matches!(
        target.entity_type,
        EntityType::Homunculus | EntityType::Mercenary
    ) || target.is_pet)
        && active_skill.is_none()
    {
        return Some(CursorType::Click);
    }
    match active_skill {
        Some(TargetClass::Supportive | TargetClass::SelfOnly) => Some(CursorType::Lock),
        Some(TargetClass::Offensive) => {
            can_attack(target, map, player_id).then_some(CursorType::Attack)
        }
        Some(TargetClass::SkillUnit) => None,
        Some(TargetClass::Ground) | None => {
            can_attack(target, map, player_id).then_some(CursorType::Attack)
        }
    }
}

pub fn skill_target_allowed(
    class: TargetClass,
    target: &Entity,
    map: &MapProperties,
    player_id: Option<u32>,
) -> bool {
    if target.state == EntityState::Dead {
        return class == TargetClass::Supportive && target.entity_type == EntityType::Player;
    }
    match class {
        TargetClass::Ground | TargetClass::SkillUnit => false,
        TargetClass::SelfOnly => relationship(target.id, player_id) == Relationship::Myself,
        TargetClass::Supportive => target.entity_type != EntityType::Npc,
        TargetClass::Offensive => can_attack(target, map, player_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_drives_pvp_and_gvg_without_flags() {
        assert!(MapProperties::from_kind(MapKind::FreePvp).is_pvp());
        assert!(MapProperties::from_kind(MapKind::PvpServer).enable_pk());
        assert!(MapProperties::from_kind(MapKind::Agit).is_gvg());
        assert!(MapProperties::from_kind(MapKind::Agit).is_siege());

        let town = MapProperties::from_kind(MapKind::Normal);
        assert!(!town.is_pvp() && !town.is_gvg() && !town.enable_pk());
        assert!(!town.no_lockon());
    }

    #[test]
    fn detailed_flags_are_decoded() {
        let flags = MapPropertyFlags::IsParty.as_flag() | MapPropertyFlags::IsNoLockOn.as_flag();
        let props = MapProperties::with_flags(MapKind::Normal, flags);
        assert!(props.is_pvp());
        assert!(props.no_lockon());
        assert!(!props.is_gvg());
    }

    fn entity(id: u32, entity_type: EntityType, job: u16) -> Entity {
        Entity::new(
            id,
            entity_type,
            job,
            1,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            100,
            100,
            0,
            150,
        )
    }

    #[test]
    fn target_class_maps_from_skill_target_type() {
        assert_eq!(
            skill_target_class(SkillTargetType::Target),
            TargetClass::Offensive
        );
        assert_eq!(
            skill_target_class(SkillTargetType::Friend),
            TargetClass::Supportive
        );
        assert_eq!(
            skill_target_class(SkillTargetType::MySelf),
            TargetClass::SelfOnly
        );
        assert_eq!(
            skill_target_class(SkillTargetType::Ground),
            TargetClass::Ground
        );
        assert_eq!(
            skill_target_class(SkillTargetType::Trap),
            TargetClass::SkillUnit
        );
    }

    #[test]
    fn trap_targeting_never_resolves_to_an_entity() {
        let me = Some(1u32);
        let pvp = MapProperties::from_kind(MapKind::FreePvp);
        let class = skill_target_class(SkillTargetType::Trap);
        for target in [
            entity(10, EntityType::Monster, 1002),
            entity(20, EntityType::Player, 0),
        ] {
            assert!(!skill_target_allowed(class, &target, &pvp, me));
            assert_eq!(hover_cursor(&target, &pvp, Some(class), me), None);
        }
    }

    #[test]
    fn only_supportive_skills_reach_a_corpse() {
        let me = Some(1u32);
        let town = MapProperties::from_kind(MapKind::Normal);
        let mut corpse = entity(20, EntityType::Player, 0);
        corpse.state = EntityState::Dead;
        let mut dead_mob = entity(30, EntityType::Monster, 1002);
        dead_mob.state = EntityState::Dead;

        let support = skill_target_class(SkillTargetType::Friend);
        assert!(skill_target_allowed(support, &corpse, &town, me));
        assert_eq!(
            hover_cursor(&corpse, &town, Some(support), me),
            Some(CursorType::Default)
        );
        assert!(!skill_target_allowed(support, &dead_mob, &town, me));

        let offensive = skill_target_class(SkillTargetType::Target);
        assert!(!skill_target_allowed(offensive, &corpse, &town, me));
        assert_eq!(hover_cursor(&corpse, &town, Some(offensive), me), None);
        assert_eq!(hover_cursor(&corpse, &town, None, me), None);
    }

    #[test]
    fn hover_and_attack_depend_on_pvp_zone() {
        let me = Some(1u32);
        let town = MapProperties::from_kind(MapKind::Normal);
        let pvp = MapProperties::from_kind(MapKind::FreePvp);
        let monster = entity(10, EntityType::Monster, 1002);
        let other = entity(20, EntityType::Player, 0);
        let myself = entity(1, EntityType::Player, 0);

        assert!(can_attack(&monster, &town, me));
        assert_eq!(
            hover_cursor(&monster, &town, None, me),
            Some(CursorType::Attack)
        );

        assert!(!can_attack(&other, &town, me));
        assert_eq!(hover_cursor(&other, &town, None, me), None);
        assert!(can_attack(&other, &pvp, me));
        assert_eq!(
            hover_cursor(&other, &pvp, None, me),
            Some(CursorType::Attack)
        );

        assert!(!can_attack(&myself, &pvp, me));

        assert_eq!(
            hover_cursor(&myself, &town, Some(TargetClass::Supportive), me),
            Some(CursorType::Lock)
        );
        assert_eq!(
            hover_cursor(&other, &town, Some(TargetClass::Offensive), me),
            None
        );
    }

    #[test]
    fn skill_ground_units_are_not_interactive() {
        use crate::sprite_path::{JT_WARPNPC, SKILL_UNIT_JOB_MIN};

        let town = MapProperties::from_kind(MapKind::Normal);
        let me = Some(1u32);
        let sanctuary = entity(10, EntityType::Npc, SKILL_UNIT_JOB_MIN + 5);
        let category = sanctuary.category();

        assert_eq!(category, EntityCategory::Skill);
        assert!(!category.has_name_plate());
        assert!(!category.has_health_bar());
        assert_eq!(hover_cursor(&sanctuary, &town, None, me), None);
        assert_eq!(
            hover_cursor(&sanctuary, &town, Some(TargetClass::Offensive), me),
            None
        );

        let warp = entity(11, EntityType::Npc, JT_WARPNPC);
        assert_eq!(hover_cursor(&warp, &town, None, me), Some(CursorType::Warp));
        assert!(!warp.category().has_health_bar());

        let shopkeeper = entity(12, EntityType::Npc, 60);
        assert_eq!(
            hover_cursor(&shopkeeper, &town, None, me),
            Some(CursorType::Talk)
        );
    }

    #[test]
    fn trigger_actors_are_undrawn_and_only_the_clickable_ones_pick() {
        use crate::sprite_path::{
            JT_EFFECTLAUNCHER, JT_HIDDEN_NPC, JT_HIDDEN_WARP_NPC, JT_WARPNPC, is_undrawn_actor,
        };

        let town = MapProperties::from_kind(MapKind::Normal);
        let me = Some(1u32);
        for job in [
            JT_WARPNPC,
            JT_EFFECTLAUNCHER,
            JT_HIDDEN_NPC,
            JT_HIDDEN_WARP_NPC,
        ] {
            assert!(is_undrawn_actor(job), "job {job}");
        }
        assert!(!is_undrawn_actor(110));

        let hidden_npc = entity(30, EntityType::Npc, JT_HIDDEN_NPC);
        assert_eq!(
            hover_cursor(&hidden_npc, &town, None, me),
            Some(CursorType::Talk)
        );

        for job in [JT_EFFECTLAUNCHER, JT_HIDDEN_WARP_NPC] {
            let inert = entity(31, EntityType::Npc, job);
            assert_eq!(hover_cursor(&inert, &town, None, me), None, "job {job}");
        }
    }

    #[test]
    fn pk_name_color_reads_effect_state_bits() {
        assert_eq!(pk_name_color(0), None);
        assert_eq!(
            pk_name_color(EFFECT_STATE_RED_NAME),
            Some([1.0, 0.0, 0.0, 1.0])
        );
        assert_eq!(
            pk_name_color(EFFECT_STATE_PINK_NAME),
            Some([1.0, 0.392, 0.722, 1.0])
        );
        assert_eq!(
            pk_name_color(EFFECT_STATE_RED_NAME | EFFECT_STATE_PINK_NAME),
            Some([1.0, 0.0, 0.0, 1.0])
        );
    }

    #[test]
    fn skill_validity_rejects_bad_targets() {
        let me = Some(1u32);
        let pvp = MapProperties::from_kind(MapKind::FreePvp);
        let monster = entity(10, EntityType::Monster, 1002);
        let other = entity(20, EntityType::Player, 0);
        let myself = entity(1, EntityType::Player, 0);

        assert!(skill_target_allowed(
            TargetClass::Offensive,
            &monster,
            &pvp,
            me
        ));
        assert!(skill_target_allowed(
            TargetClass::SelfOnly,
            &myself,
            &pvp,
            me
        ));
        assert!(!skill_target_allowed(
            TargetClass::SelfOnly,
            &other,
            &pvp,
            me
        ));
        assert!(skill_target_allowed(
            TargetClass::Supportive,
            &other,
            &pvp,
            me
        ));
        assert!(!skill_target_allowed(
            TargetClass::Ground,
            &monster,
            &pvp,
            me
        ));
    }
}
