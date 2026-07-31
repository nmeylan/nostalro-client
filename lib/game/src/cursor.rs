use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use serde::{Deserialize, Serialize};

use crate::entity_collection::EntityCollection;
use crate::targeting::{MapProperties, TargetClass, hover_cursor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorType {
    #[default]
    Default = 0,
    Talk = 1,
    Click = 2,
    SemiLock = 3,
    Rotate = 4,
    Attack = 5,
    Warp = 7,
    NoWalk = 8,
    Pick = 9,
    Lock = 10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingSkillTarget {
    Entity { skill_id: u16, level: i16 },
    Ground { skill_id: u16, level: i16 },
}

impl PendingSkillTarget {
    pub fn skill_id(&self) -> u16 {
        match self {
            Self::Entity { skill_id, .. } | Self::Ground { skill_id, .. } => *skill_id,
        }
    }

    pub fn level(&self) -> i16 {
        match self {
            Self::Entity { level, .. } | Self::Ground { level, .. } => *level,
        }
    }
}

/// A companion skill awaiting its target click. Unlike [`PendingSkillTarget`],
/// resolving this issues an AI order to the companion (which moves itself into
/// range and casts) rather than casting from the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingCompanionSkill {
    pub is_mercenary: bool,
    pub skill_id: u16,
    pub level: i16,
    pub is_ground: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderEntryKind {
    Entity,
    FloorItem,
    /// A trailing pushcart; `id` is its owner entity's id.
    Cart,
    /// A hovering falcon companion; `id` is its owner entity's id.
    Falcon,
}

#[derive(Clone, Copy)]
pub struct RenderEntry {
    pub kind: RenderEntryKind,
    pub id: u32,
    pub screen_anchor: [f32; 2],
    pub depth: f32,
    pub depth_gradient: [f32; 2],
    /// Ground-lying depth gradient (sprite laid flat on the terrain) used for
    /// dead bodies so the death frame isn't clipped by the floor.
    pub flat_depth_gradient: [f32; 2],
    pub camera_dir: u8,
    pub sprite_scale: f32,
    /// [left, top, right, bottom] in screen pixels.
    pub pick_bounds: [f32; 4],
    /// Screen pixels from feet to the top of action 0 motion 0; anchors floating elements.
    pub head_offset: f32,
}

/// What the drawn cursor may stick to. The OS pointer and hit-testing are
/// unaffected; only the sprite moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapTarget {
    Monster,
    FloorItem,
    /// A homunculus or mercenary while Aid Potion is armed. Always snaps — the
    /// original game gives this case no toggle.
    Companion,
}

/// `/snap`, `/skillsnap` and `/itemsnap`. Monsters get two toggles because the
/// original game snaps to them under different rules while a skill is armed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MouseSnapPrefs {
    pub monster_no_skill: bool,
    pub monster_skill: bool,
    pub item: bool,
}

impl Default for MouseSnapPrefs {
    fn default() -> Self {
        Self {
            monster_no_skill: false,
            monster_skill: true,
            item: false,
        }
    }
}

impl MouseSnapPrefs {
    /// `skill_armed`: an entity-targeted skill is waiting for its click.
    pub fn snaps_to(&self, target: SnapTarget, skill_armed: bool) -> bool {
        match target {
            SnapTarget::Monster if skill_armed => self.monster_skill,
            SnapTarget::Monster => self.monster_no_skill,
            SnapTarget::FloorItem => self.item,
            SnapTarget::Companion => true,
        }
    }
}

pub fn cursor_type_for_cell(gat: &GatFile, cell: Option<(i32, i32)>) -> CursorType {
    match cell {
        Some((cx, cy)) => {
            if gat.is_walkable(cx, cy) {
                CursorType::Default
            } else {
                CursorType::NoWalk
            }
        }
        None => CursorType::Default,
    }
}

const MIN_PICK_SIZE: f32 = 100.0;

/// `render_list` is sorted far-to-near (painter order): iterate in reverse for front-to-back.
pub fn hovered_entity_cursor_type(
    mouse_pos: (f64, f64),
    entities: &EntityCollection,
    render_list: &[RenderEntry],
    map: &MapProperties,
    active_skill: Option<TargetClass>,
) -> Option<(CursorType, u32)> {
    let (mx, my) = (mouse_pos.0 as f32, mouse_pos.1 as f32);
    let player_id = entities.player_id();

    let mut best: Option<(CursorType, u32, f32)> = None;

    for entry in render_list.iter().rev() {
        let [mut left, mut top, mut right, mut bottom] = entry.pick_bounds;
        let dx = MIN_PICK_SIZE - (right - left);
        if dx > 0.0 {
            left -= dx / 2.0;
            right += dx / 2.0;
        }
        let dy = MIN_PICK_SIZE - (bottom - top);
        if dy > 0.0 {
            top -= dy / 2.0;
            bottom += dy / 2.0;
        }

        if mx >= left && mx <= right && my >= top && my <= bottom {
            let entity = match entities.get(entry.id) {
                Some(e) => e,
                None => continue,
            };
            let cursor = match hover_cursor(entity, map, active_skill, player_id) {
                Some(c) => c,
                None => continue,
            };
            let dx = mx - entry.screen_anchor[0];
            let dy = my - entry.screen_anchor[1];
            let dist_sq = dx * dx + dy * dy;
            if best.as_ref().is_none_or(|b| dist_sq < b.2) {
                best = Some((cursor, entry.id, dist_sq));
            }
        }
    }

    best.map(|(cursor, id, _)| (cursor, id))
}

/// Topmost player under the cursor, excluding self, ignoring attackability.
/// Used to target friendly players (party invite etc.) on non-PvP maps where
/// `hovered_entity_cursor_type` returns nothing because they can't be attacked.
pub fn hovered_player(
    mouse_pos: (f64, f64),
    entities: &EntityCollection,
    render_list: &[RenderEntry],
) -> Option<u32> {
    use crate::entity::{EntityState, EntityType};
    let (mx, my) = (mouse_pos.0 as f32, mouse_pos.1 as f32);
    let player_id = entities.player_id();
    let mut best: Option<(u32, f32)> = None;

    for entry in render_list.iter().rev() {
        if Some(entry.id) == player_id {
            continue;
        }
        let [mut left, mut top, mut right, mut bottom] = entry.pick_bounds;
        let dx = MIN_PICK_SIZE - (right - left);
        if dx > 0.0 {
            left -= dx / 2.0;
            right += dx / 2.0;
        }
        let dy = MIN_PICK_SIZE - (bottom - top);
        if dy > 0.0 {
            top -= dy / 2.0;
            bottom += dy / 2.0;
        }
        if mx < left || mx > right || my < top || my > bottom {
            continue;
        }
        let Some(entity) = entities.get(entry.id) else {
            continue;
        };
        if entity.entity_type != EntityType::Player
            || entity.state == EntityState::Dead
            || entity.is_fading()
        {
            continue;
        }
        let ddx = mx - entry.screen_anchor[0];
        let ddy = my - entry.screen_anchor[1];
        let dist_sq = ddx * ddx + ddy * ddy;
        if best.as_ref().is_none_or(|b| dist_sq < b.1) {
            best = Some((entry.id, dist_sq));
        }
    }
    best.map(|(id, _)| id)
}

pub struct CursorAnimationState {
    cursor_type: CursorType,
    motion_index: usize,
    accumulated_ms: f32,
}

impl Default for CursorAnimationState {
    fn default() -> Self {
        Self::new()
    }
}

impl CursorAnimationState {
    pub fn new() -> Self {
        Self {
            cursor_type: CursorType::Default,
            motion_index: 0,
            accumulated_ms: 0.0,
        }
    }

    pub fn set_cursor_type(&mut self, ty: CursorType) {
        if self.cursor_type != ty {
            self.cursor_type = ty;
            self.motion_index = 0;
            self.accumulated_ms = 0.0;
        }
    }

    pub fn cursor_type(&self) -> CursorType {
        self.cursor_type
    }

    pub fn action_index(&self) -> usize {
        self.cursor_type as usize
    }

    pub fn motion_index(&self) -> usize {
        self.motion_index
    }

    pub fn update(&mut self, dt_secs: f32, act: &ActFile) {
        let action_idx = self.action_index();
        if action_idx >= act.actions.len() {
            return;
        }
        let motion_count = act.actions[action_idx].motions.len();
        if motion_count == 0 {
            return;
        }

        let delay_ms = if action_idx < act.delays.len() {
            let d = act.delays[action_idx] * 25.0;
            if d > 0.0 { d } else { 150.0 }
        } else {
            150.0
        };

        self.accumulated_ms += dt_secs * 1000.0;
        while self.accumulated_ms >= delay_ms {
            self.accumulated_ms -= delay_ms;
            self.motion_index = (self.motion_index + 1) % motion_count;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Entity, EntityType};
    use ragnarok_formats::act::{ActFile, Action, Motion};
    use ragnarok_formats::gat::GatFile;

    fn build_gat_bytes(width: i32, height: i32, walkable: &[bool]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"GRAT");
        data.push(1);
        data.push(2);
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&height.to_le_bytes());
        for &w in walkable {
            for _ in 0..4 {
                data.extend_from_slice(&0.0_f32.to_le_bytes());
            }
            let cell_type: i32 = if w { 0 } else { 1 };
            data.extend_from_slice(&cell_type.to_le_bytes());
        }
        data
    }

    fn make_cursor_act(action_count: usize, motions_per_action: usize) -> ActFile {
        let actions: Vec<Action> = (0..action_count)
            .map(|_| Action {
                motions: (0..motions_per_action)
                    .map(|_| Motion {
                        range1: [0; 4],
                        range2: [0; 4],
                        clips: Vec::new(),
                        event_id: -1,
                        attach_points: Vec::new(),
                    })
                    .collect(),
            })
            .collect();
        ActFile {
            version: (2, 5),
            actions,
            events: Vec::new(),
            delays: vec![4.0; action_count],
        }
    }

    #[test]
    fn action_index_maps_to_cursor_type_value() {
        let mut anim = CursorAnimationState::new();
        assert_eq!(anim.action_index(), 0);

        anim.set_cursor_type(CursorType::NoWalk);
        assert_eq!(anim.action_index(), 8);

        anim.set_cursor_type(CursorType::Attack);
        assert_eq!(anim.action_index(), 5);
    }

    #[test]
    fn set_cursor_type_resets_on_change() {
        let act = make_cursor_act(14, 4);
        let mut anim = CursorAnimationState::new();
        anim.update(0.5, &act);
        assert!(anim.motion_index > 0);

        anim.set_cursor_type(CursorType::Talk);
        assert_eq!(anim.motion_index(), 0);
        assert_eq!(anim.accumulated_ms, 0.0);

        anim.update(0.25, &act);
        let idx = anim.motion_index();
        anim.set_cursor_type(CursorType::Talk);
        assert_eq!(anim.motion_index(), idx);
    }

    #[test]
    fn update_advances_motion_frames() {
        let act = make_cursor_act(1, 3);
        let mut anim = CursorAnimationState::new();
        anim.update(0.25, &act);
        assert_eq!(anim.motion_index(), 2);
    }

    #[test]
    fn cursor_type_for_cell_walkable_and_unwalkable() {
        let walkable = vec![true, false];
        let data = build_gat_bytes(2, 1, &walkable);
        let gat = GatFile::parse(&data).unwrap();

        assert_eq!(
            cursor_type_for_cell(&gat, Some((0, 0))),
            CursorType::Default
        );
        assert_eq!(cursor_type_for_cell(&gat, Some((1, 0))), CursorType::NoWalk);
        assert_eq!(cursor_type_for_cell(&gat, None), CursorType::Default);
    }

    fn make_entity(id: u32, entity_type: EntityType, job: u16) -> Entity {
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

    fn default_pick_bounds(cx: f32, cy: f32) -> [f32; 4] {
        [cx - 50.0, cy - 100.0, cx + 50.0, cy]
    }

    fn entry(id: u32, cx: f32, cy: f32, depth: f32, scale: f32) -> RenderEntry {
        let bounds = default_pick_bounds(cx, cy);
        RenderEntry {
            kind: RenderEntryKind::Entity,
            id,
            screen_anchor: [cx, cy],
            depth,
            depth_gradient: [0.0, 0.0],
            flat_depth_gradient: [0.0, 0.0],
            camera_dir: 0,
            sprite_scale: scale,
            pick_bounds: bounds,
            head_offset: bounds[3] - bounds[1],
        }
    }

    #[test]
    fn entity_hover_returns_none_on_empty_list() {
        let entities = EntityCollection::new();
        assert_eq!(
            hovered_entity_cursor_type(
                (400.0, 300.0),
                &entities,
                &[],
                &MapProperties::default(),
                None
            ),
            None
        );
    }

    #[test]
    fn entity_hover_returns_attack_for_monster() {
        let mut entities = EntityCollection::new();
        entities.insert(make_entity(10, EntityType::Monster, 1002));
        let list = vec![entry(10, 400.0, 350.0, 0.5, 1.0)];
        assert_eq!(
            hovered_entity_cursor_type(
                (400.0, 310.0),
                &entities,
                &list,
                &MapProperties::default(),
                None
            ),
            Some((CursorType::Attack, 10)),
        );
    }

    #[test]
    fn entity_hover_returns_talk_for_npc() {
        let mut entities = EntityCollection::new();
        entities.insert(make_entity(20, EntityType::Npc, 100));
        let list = vec![entry(20, 400.0, 350.0, 0.5, 1.0)];
        assert_eq!(
            hovered_entity_cursor_type(
                (400.0, 310.0),
                &entities,
                &list,
                &MapProperties::default(),
                None
            ),
            Some((CursorType::Talk, 20)),
        );
    }

    #[test]
    fn entity_hover_returns_warp_for_job_45() {
        let mut entities = EntityCollection::new();
        entities.insert(make_entity(30, EntityType::Npc, 45));
        let list = vec![entry(30, 400.0, 350.0, 0.5, 1.0)];
        assert_eq!(
            hovered_entity_cursor_type(
                (400.0, 310.0),
                &entities,
                &list,
                &MapProperties::default(),
                None
            ),
            Some((CursorType::Warp, 30)),
        );
    }

    #[test]
    fn entity_hover_skips_local_player() {
        let mut entities = EntityCollection::new();
        entities.set_player_id(1);
        entities.insert(make_entity(1, EntityType::Player, 0));
        let list = vec![entry(1, 400.0, 350.0, 0.5, 1.0)];
        assert_eq!(
            hovered_entity_cursor_type(
                (400.0, 310.0),
                &entities,
                &list,
                &MapProperties::default(),
                None
            ),
            None
        );
    }

    #[test]
    fn entity_hover_picks_closest_anchor_among_overlapping() {
        let mut entities = EntityCollection::new();
        entities.insert(make_entity(10, EntityType::Monster, 1002));
        entities.insert(make_entity(20, EntityType::Npc, 100));
        // Both at same screen anchor - closest anchor distance is equal, first candidate wins
        let list = vec![
            entry(10, 400.0, 350.0, 0.8, 1.0),
            entry(20, 400.0, 350.0, 0.3, 1.0),
        ];
        assert_eq!(
            hovered_entity_cursor_type(
                (400.0, 310.0),
                &entities,
                &list,
                &MapProperties::default(),
                None
            ),
            Some((CursorType::Talk, 20)),
        );
    }

    #[test]
    fn entity_hover_picks_closest_anchor_when_bounds_overlap() {
        let mut entities = EntityCollection::new();
        entities.insert(make_entity(10, EntityType::Monster, 1002));
        entities.insert(make_entity(20, EntityType::Monster, 1002));
        // Interior mob at (400, 330), front-row mob at (400, 370)
        // Both have default 100x100 pick bounds that overlap in the 270-330 y range
        let list = vec![
            entry(10, 400.0, 330.0, 0.8, 1.0),
            entry(20, 400.0, 370.0, 0.3, 1.0),
        ];
        // Mouse at (400, 300) - closer to interior mob anchor (330) than front mob (370)
        assert_eq!(
            hovered_entity_cursor_type(
                (400.0, 300.0),
                &entities,
                &list,
                &MapProperties::default(),
                None
            ),
            Some((CursorType::Attack, 10)),
        );
    }

    #[test]
    fn entity_hover_returns_none_when_outside_bounds() {
        let mut entities = EntityCollection::new();
        entities.insert(make_entity(10, EntityType::Monster, 1002));
        let list = vec![entry(10, 400.0, 350.0, 0.5, 1.0)];
        // Mouse far from entity center
        assert_eq!(
            hovered_entity_cursor_type(
                (100.0, 100.0),
                &entities,
                &list,
                &MapProperties::default(),
                None
            ),
            None
        );
    }

    #[test]
    fn dead_monster_is_not_hoverable() {
        let mut entities = EntityCollection::new();
        let mut monster = make_entity(10, EntityType::Monster, 1002);
        monster.enter_dead();
        entities.insert(monster);
        let list = vec![entry(10, 400.0, 350.0, 0.5, 1.0)];
        assert_eq!(
            hovered_entity_cursor_type(
                (400.0, 310.0),
                &entities,
                &list,
                &MapProperties::default(),
                None
            ),
            None
        );
    }

    #[test]
    fn fading_entity_is_not_hoverable() {
        let mut entities = EntityCollection::new();
        let mut monster = make_entity(10, EntityType::Monster, 1002);
        monster.start_vanish_fade();
        entities.insert(monster);
        let list = vec![entry(10, 400.0, 350.0, 0.5, 1.0)];
        assert_eq!(
            hovered_entity_cursor_type(
                (400.0, 310.0),
                &entities,
                &list,
                &MapProperties::default(),
                None
            ),
            None
        );
    }

    #[test]
    fn small_bounds_inflated_to_minimum_during_hit_test() {
        let mut entities = EntityCollection::new();
        entities.insert(make_entity(10, EntityType::Monster, 1002));
        // Small 30x30 stored bounds centered on (400, 350)
        let list = vec![RenderEntry {
            kind: RenderEntryKind::Entity,
            id: 10,
            screen_anchor: [400.0, 350.0],
            depth: 0.5,
            depth_gradient: [0.0, 0.0],
            flat_depth_gradient: [0.0, 0.0],
            camera_dir: 0,
            sprite_scale: 1.0,
            pick_bounds: [385.0, 335.0, 415.0, 365.0],
            head_offset: 30.0,
        }];
        assert_eq!(
            hovered_entity_cursor_type(
                (400.0, 310.0),
                &entities,
                &list,
                &MapProperties::default(),
                None
            ),
            Some((CursorType::Attack, 10)),
        );
    }

    #[test]
    fn monster_snap_has_a_separate_toggle_per_skill_state() {
        let prefs = MouseSnapPrefs::default();
        assert!(!prefs.snaps_to(SnapTarget::Monster, false));
        assert!(prefs.snaps_to(SnapTarget::Monster, true));
        assert!(!prefs.snaps_to(SnapTarget::FloorItem, false));
        assert!(!prefs.snaps_to(SnapTarget::FloorItem, true));

        let all_on = MouseSnapPrefs {
            monster_no_skill: true,
            monster_skill: false,
            item: true,
        };
        assert!(all_on.snaps_to(SnapTarget::Monster, false));
        assert!(!all_on.snaps_to(SnapTarget::Monster, true));
        assert!(all_on.snaps_to(SnapTarget::FloorItem, true));

        let all_off = MouseSnapPrefs {
            monster_no_skill: false,
            monster_skill: false,
            item: false,
        };
        assert!(all_off.snaps_to(SnapTarget::Companion, true));
    }
}
