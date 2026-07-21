use crate::App;
use crate::ClipData;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::cursor::{CursorType, RenderEntry};
use ragnarok_game::sprite_loader;
use ragnarok_renderer::build_clip_quad;

impl App {
    pub(crate) fn load_cursor_sprite(&mut self, grf: &GrfArchive) {
        if let Some(sprite_data) = sprite_loader::load_cursor_sprite(grf)
            && let Some(textures) = self.upload_sprite(&sprite_data)
        {
            self.game.cursor_textures = Some(textures);
            self.game.cursor_act = Some(sprite_data.act);

            if let Some(window) = &self.window {
                window.set_cursor_visible(false);
            }
        }
    }

    pub(crate) fn build_cursor_sprite_clips(&mut self, dt: f32) -> Vec<ClipData> {
        let cursor_act = match &self.game.cursor_act {
            Some(a) => a,
            None => return Vec::new(),
        };

        self.game.cursor_animation.update(dt, cursor_act);
        let action_idx = self.game.cursor_animation.action_index();
        let action_idx = if action_idx < cursor_act.actions.len() {
            action_idx
        } else {
            0
        };
        let action = &cursor_act.actions[action_idx];
        if action.motions.is_empty() {
            return Vec::new();
        }
        let motion_idx = self.game.cursor_animation.motion_index() % action.motions.len();
        let motion = &action.motions[motion_idx];
        let (mx, my) = self.input.mouse_position;
        let cursor_tex = match &self.game.cursor_textures {
            Some(t) => t,
            None => return Vec::new(),
        };
        let mut clips = Vec::new();
        for clip in &motion.clips {
            if let Some((vertices, indices, tex_idx)) =
                build_clip_quad(clip, cursor_tex, [mx as f32, my as f32], 0.0, [0, 0])
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
        let cursor_act = match &self.game.cursor_act {
            Some(a) => a,
            None => return Vec::new(),
        };
        let cursor_tex = match &self.game.cursor_textures {
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
            .lock_cursor_animation
            .set_cursor_type(CursorType::SemiLock);
        self.game.lock_cursor_animation.update(dt, cursor_act);
        let action_idx = self.game.lock_cursor_animation.action_index();
        let action_idx = if action_idx < cursor_act.actions.len() {
            action_idx
        } else {
            return Vec::new();
        };
        let action = &cursor_act.actions[action_idx];
        if action.motions.is_empty() {
            return Vec::new();
        }
        let motion_idx = self.game.lock_cursor_animation.motion_index() % action.motions.len();
        let motion = &action.motions[motion_idx];

        let mut clips = Vec::new();
        for clip in &motion.clips {
            if let Some((vertices, indices, tex_idx)) =
                build_clip_quad(clip, cursor_tex, [sx, sy], 0.0, [0, 0])
                && tex_idx < cursor_tex.bind_groups.len()
            {
                clips.push((vertices, indices, tex_idx));
            }
        }
        clips
    }
}
