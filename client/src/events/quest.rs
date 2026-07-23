use crate::App;
use ragnarok_game::quest::{QuestHuntEntry, QuestListEntry, QuestMarker, QuestMissionData};
use ragnarok_ui_component::game::chat_window::ChatChannel;

const QUEST_MARKER_NONE: i16 = 9999;

impl App {
    pub(super) fn handle_quest_list_received(&mut self, quests: Vec<QuestListEntry>) {
        self.game.quest_log.clear();
        for entry in quests {
            self.game.quest_log.set_list_entry(entry);
        }
    }

    pub(super) fn handle_quest_missions_received(&mut self, missions: Vec<QuestMissionData>) {
        for mission in missions {
            self.game.quest_log.set_mission(mission);
        }
    }

    pub(super) fn handle_quest_added(&mut self, quest: QuestMissionData) {
        let title = self
            .game
            .data_table
            .quest_display
            .as_ref()
            .map(|t| t.title(quest.id))
            .unwrap_or_else(|| "Unknown Quest".to_string());
        self.game.quest_log.add(quest);
        self.windows.chat_window.add_message(
            format!("Quest added : {title}"),
            [1.0, 1.0, 1.0, 1.0],
            ChatChannel::Public,
        );
    }

    pub(super) fn handle_quest_removed(&mut self, quest_id: u32) {
        self.game.quest_log.remove(quest_id);
    }

    pub(super) fn handle_quest_hunt_updated(&mut self, entries: Vec<QuestHuntEntry>) {
        for entry in entries {
            let before = self
                .game
                .quest_log
                .get(entry.quest_id)
                .and_then(|q| q.objectives.iter().find(|o| o.mob_id == entry.mob_id))
                .map(|o| (o.current, o.name.clone()));
            self.game.quest_log.update_hunt(entry);
            if let Some((old_current, name)) = before
                && entry.current > old_current
            {
                let title = self
                    .game
                    .data_table
                    .quest_display
                    .as_ref()
                    .map(|t| t.title(entry.quest_id))
                    .unwrap_or_default();
                self.windows.chat_window.add_message(
                    format!(
                        "Mission [{title}], you killed {name}. ({}/{})",
                        entry.current, entry.required
                    ),
                    [1.0, 1.0, 1.0, 1.0],
                    ChatChannel::Public,
                );
            }
        }
    }

    pub(super) fn handle_quest_active_changed(&mut self, quest_id: u32, active: bool) {
        self.game.quest_log.set_active(quest_id, active);
    }

    pub(super) fn handle_quest_npc_marker(
        &mut self,
        npc_id: u32,
        x: u16,
        y: u16,
        effect: i16,
        color: u8,
    ) {
        if color == 0 || effect == QUEST_MARKER_NONE {
            self.game.quest_markers.remove(&npc_id);
        } else {
            self.game.quest_markers.insert(
                npc_id,
                QuestMarker {
                    x,
                    y,
                    effect,
                    color,
                },
            );
        }
    }
}
