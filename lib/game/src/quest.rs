/// A single hunt objective. `required` is the total the server tracks; it only
/// arrives via ZC_UPDATE_MISSION_HUNT (0x2b5), so it starts at 0 until then.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestObjective {
    pub mob_id: u32,
    pub name: String,
    pub current: i16,
    pub required: i16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Quest {
    pub id: u32,
    pub active: bool,
    /// Unix expiry, `None` when the quest is not time-limited.
    pub end_time: Option<u32>,
    pub objectives: Vec<QuestObjective>,
}

/// One entry of ZC_ALL_QUEST_LIST (0x2b1): quest id + active flag, no objectives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestListEntry {
    pub id: u32,
    pub active: bool,
}

/// A quest with its mission data, from ZC_ALL_QUEST_MISSION (0x2b2) / ZC_ADD_QUEST (0x2b3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestMissionData {
    pub id: u32,
    pub end_time: Option<u32>,
    pub objectives: Vec<QuestObjective>,
}

/// One entry of ZC_UPDATE_MISSION_HUNT (0x2b5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestHuntEntry {
    pub quest_id: u32,
    pub mob_id: u32,
    pub current: i16,
    pub required: i16,
}

/// An over-NPC quest marker (ZC_QUEST_NOTIFY_EFFECT). `effect` selects the
/// emotion-sprite action (81 + effect); `color` drives the minimap dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestMarker {
    pub x: u16,
    pub y: u16,
    pub effect: i16,
    pub color: u8,
}

/// First emotion.act action used by the over-NPC quest markers; the marker
/// `effect` value indexes forward from here.
const MARKER_ACTION_BASE: usize = 81;

/// emotion.act action index for a marker's `effect` value.
pub fn marker_sprite_action(effect: i16) -> usize {
    MARKER_ACTION_BASE + effect.max(0) as usize
}

#[derive(Debug, Clone, Default)]
pub struct QuestLog {
    pub quests: Vec<Quest>,
}

impl QuestLog {
    pub fn clear(&mut self) {
        self.quests.clear();
    }

    fn index_of(&self, id: u32) -> Option<usize> {
        self.quests.iter().position(|q| q.id == id)
    }

    pub fn get(&self, id: u32) -> Option<&Quest> {
        self.index_of(id).map(|i| &self.quests[i])
    }

    /// Sets the active flag from the list packet, creating the row if missing.
    pub fn set_list_entry(&mut self, entry: QuestListEntry) {
        if let Some(i) = self.index_of(entry.id) {
            self.quests[i].active = entry.active;
        } else {
            self.quests.push(Quest {
                id: entry.id,
                active: entry.active,
                end_time: None,
                objectives: Vec::new(),
            });
        }
    }

    /// Applies mission data (objectives + expiry) to an existing row or inserts it.
    pub fn set_mission(&mut self, mission: QuestMissionData) {
        if let Some(i) = self.index_of(mission.id) {
            self.quests[i].end_time = mission.end_time;
            self.quests[i].objectives = mission.objectives;
        } else {
            self.quests.push(Quest {
                id: mission.id,
                active: true,
                end_time: mission.end_time,
                objectives: mission.objectives,
            });
        }
    }

    /// ZC_ADD_QUEST: dedup by id, ignoring duplicates entirely.
    pub fn add(&mut self, mission: QuestMissionData) {
        if self.index_of(mission.id).is_some() {
            return;
        }
        self.quests.push(Quest {
            id: mission.id,
            active: true,
            end_time: mission.end_time,
            objectives: mission.objectives,
        });
    }

    pub fn remove(&mut self, id: u32) -> bool {
        if let Some(i) = self.index_of(id) {
            self.quests.remove(i);
            true
        } else {
            false
        }
    }

    pub fn set_active(&mut self, id: u32, active: bool) {
        if let Some(i) = self.index_of(id) {
            self.quests[i].active = active;
        }
    }

    /// The sole source of required totals: matches by (quest_id, mob_id).
    pub fn update_hunt(&mut self, entry: QuestHuntEntry) {
        if let Some(i) = self.index_of(entry.quest_id) {
            if let Some(obj) = self.quests[i]
                .objectives
                .iter_mut()
                .find(|o| o.mob_id == entry.mob_id)
            {
                obj.current = entry.current;
                obj.required = entry.required;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_burst_builds_quests_with_totals() {
        let mut log = QuestLog::default();
        // 0x2b1: two quests, one active.
        log.set_list_entry(QuestListEntry {
            id: 1000,
            active: true,
        });
        log.set_list_entry(QuestListEntry {
            id: 1001,
            active: false,
        });
        // 0x2b2: mission data with objectives (names + current kills, no totals yet).
        log.set_mission(QuestMissionData {
            id: 1000,
            end_time: None,
            objectives: vec![QuestObjective {
                mob_id: 1002,
                name: "Poring".into(),
                current: 3,
                required: 0,
            }],
        });
        // 0x2b5: the required total.
        log.update_hunt(QuestHuntEntry {
            quest_id: 1000,
            mob_id: 1002,
            current: 3,
            required: 10,
        });

        let quest = log.get(1000).unwrap();
        assert!(quest.active);
        assert_eq!(quest.objectives[0].name, "Poring");
        assert_eq!(quest.objectives[0].current, 3);
        assert_eq!(quest.objectives[0].required, 10);
        assert!(!log.get(1001).unwrap().active);
    }

    #[test]
    fn add_dedups_and_remove_drops() {
        let mut log = QuestLog::default();
        log.add(QuestMissionData {
            id: 2000,
            end_time: Some(123),
            objectives: vec![QuestObjective {
                mob_id: 1113,
                name: "Drops".into(),
                current: 0,
                required: 0,
            }],
        });
        // Duplicate add is ignored.
        log.add(QuestMissionData {
            id: 2000,
            end_time: None,
            objectives: Vec::new(),
        });
        assert_eq!(log.get(2000).unwrap().end_time, Some(123));
        log.update_hunt(QuestHuntEntry {
            quest_id: 2000,
            mob_id: 1113,
            current: 0,
            required: 5,
        });
        assert_eq!(log.get(2000).unwrap().objectives[0].required, 5);
        assert!(log.remove(2000));
        assert!(log.get(2000).is_none());
    }
}
