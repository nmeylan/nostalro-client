use crate::App;
use crate::ClipData;
use models::enums::skill_enums::SkillEnum;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::cursor::{
    CompanionSkillTarget, CursorType, PendingSkillTarget, RenderEntry, RenderEntryKind, SnapTarget,
};
use ragnarok_game::entity::{EntityCategory, EntityType};
use ragnarok_game::sprite_loader;
use ragnarok_renderer::build_clip_quad;

impl App {
    pub(crate) fn load_cursor_sprite(&mut self, grf: &GrfArchive) {
        if let Some(sprite_data) = sprite_loader::load_cursor_sprite(grf)
            && let Some(textures) = self.upload_sprite(&sprite_data)
        {
            self.game.assets.cursor_textures = Some(textures);
            self.game.assets.cursor_act = Some(sprite_data.act);

            if let Some(window) = &self.window {
                window.set_cursor_visible(false);
            }
        }
    }

    /// Centre of the picked thing the cursor sticks to, or `None` to leave the
    /// cursor on the pointer.
    fn cursor_snap_pos(
        &self,
        render_list: &[RenderEntry],
        floor_item_render_list: &[RenderEntry],
    ) -> Option<[f32; 2]> {
        let skill_armed = matches!(
            self.game.pending_casts.pending_skill_target,
            Some(PendingSkillTarget::Entity { .. })
        ) || self
            .game
            .pending_casts
            .pending_companion_skill
            .is_some_and(|pending| pending.target == CompanionSkillTarget::Entity);

        let aid_potion_armed = self
            .game
            .pending_casts
            .pending_skill_target
            .is_some_and(|pending| pending.skill_id() == SkillEnum::AmPotionpitcher.id() as u16);

        let (target, entry) = if let Some(id) = self.game.hover.hovered_entity_id {
            let entity = self.game.world.entities.get(id)?;
            let target = if aid_potion_armed {
                if !matches!(
                    entity.entity_type,
                    EntityType::Homunculus | EntityType::Mercenary
                ) {
                    return None;
                }
                SnapTarget::Companion
            } else {
                if !matches!(
                    entity.category(),
                    EntityCategory::Monster | EntityCategory::Pet
                ) {
                    return None;
                }
                SnapTarget::Monster
            };
            let entry = render_list
                .iter()
                .find(|e| e.id == id && e.kind == RenderEntryKind::Entity)?;
            (target, entry)
        } else if let Some(id) = self.game.hover.hovered_floor_item_id {
            if aid_potion_armed {
                return None;
            }
            let entry = floor_item_render_list.iter().find(|e| e.id == id)?;
            (SnapTarget::FloorItem, entry)
        } else {
            return None;
        };

        if !self.config.snap.snaps_to(target, skill_armed) {
            return None;
        }
        let [left, top, right, bottom] = entry.pick_bounds;
        Some([(left + right) / 2.0, (top + bottom) / 2.0])
    }

    pub(crate) fn build_cursor_sprite_clips(
        &mut self,
        dt: f32,
        render_list: &[RenderEntry],
        floor_item_render_list: &[RenderEntry],
    ) -> Vec<ClipData> {
        let cursor_act = match &self.game.assets.cursor_act {
            Some(a) => a,
            None => return Vec::new(),
        };

        self.game.assets.cursor_animation.update(dt, cursor_act);
        let action_idx = self.game.assets.cursor_animation.action_index();
        let action_idx = if action_idx < cursor_act.actions.len() {
            action_idx
        } else {
            0
        };
        let action = &cursor_act.actions[action_idx];
        if action.motions.is_empty() {
            return Vec::new();
        }
        let motion_idx = self.game.assets.cursor_animation.motion_index() % action.motions.len();
        let motion = &action.motions[motion_idx];
        let (mx, my) = self.input.mouse_position;
        let origin = self
            .cursor_snap_pos(render_list, floor_item_render_list)
            .unwrap_or([mx as f32, my as f32]);
        let cursor_tex = match &self.game.assets.cursor_textures {
            Some(t) => t,
            None => return Vec::new(),
        };
        let mut clips = Vec::new();
        for clip in &motion.clips {
            if let Some((vertices, indices, tex_idx)) =
                build_clip_quad(clip, cursor_tex, origin, 0.0, [0.0, 0.0])
                && tex_idx < cursor_tex.bind_groups.len()
            {
                clips.push((vertices, indices, tex_idx));
            }
        }
        clips
    }

    pub(crate) fn build_lock_cursor_clips(
        &mut self,
        dt: f32,
        render_list: &[RenderEntry],
    ) -> Vec<ClipData> {
        let target_id = match self.game.combat.attack_target_id {
            Some(id) => id,
            None => return Vec::new(),
        };
        let cursor_act = match &self.game.assets.cursor_act {
            Some(a) => a,
            None => return Vec::new(),
        };
        let cursor_tex = match &self.game.assets.cursor_textures {
            Some(t) => t,
            None => return Vec::new(),
        };

        let screen_pos = render_list
            .iter()
            .find(|e| e.id == target_id)
            .map(|e| e.screen_anchor);
        let [sx, sy] = match screen_pos {
            Some(pos) => pos,
            None => return Vec::new(),
        };

        self.game
            .assets
            .lock_cursor_animation
            .set_cursor_type(CursorType::SemiLock);
        self.game
            .assets
            .lock_cursor_animation
            .update(dt, cursor_act);
        let action_idx = self.game.assets.lock_cursor_animation.action_index();
        let action_idx = if action_idx < cursor_act.actions.len() {
            action_idx
        } else {
            return Vec::new();
        };
        let action = &cursor_act.actions[action_idx];
        if action.motions.is_empty() {
            return Vec::new();
        }
        let motion_idx =
            self.game.assets.lock_cursor_animation.motion_index() % action.motions.len();
        let motion = &action.motions[motion_idx];

        let mut clips = Vec::new();
        for clip in &motion.clips {
            if let Some((vertices, indices, tex_idx)) =
                build_clip_quad(clip, cursor_tex, [sx, sy], 0.0, [0.0, 0.0])
                && tex_idx < cursor_tex.bind_groups.len()
            {
                clips.push((vertices, indices, tex_idx));
            }
        }
        clips
    }
}
