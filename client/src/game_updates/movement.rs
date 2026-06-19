use crate::App;
use ragnarok_game::app_state::AppState;

impl App {
    pub(crate) fn process_continuous_walk(&mut self, delta: f32) {
        if !self.input.left_mouse_down || self.game.app_state != AppState::InGame {
            return;
        }
        if self.input.ui_dragging {
            return;
        }
        if self.game.attack_target_id.is_some() {
            return;
        }
        if self.game.pending_skill_target.is_some() {
            return;
        }
        if self.game.chat_window.is_active() {
            return;
        }
        if self.game.npc_dialog.dialog.is_open() || self.game.npc_shop.shop.is_open() {
            return;
        }
        if self.game.chat_window.contains_point(
            self.input.mouse_position.0 as f32,
            self.input.mouse_position.1 as f32,
        ) {
            return;
        }
        self.input.walk_packet_cooldown -= delta;
        if self.input.walk_packet_cooldown > 0.0 && !self.input.walk_server_acked {
            return;
        }
        if self.input.walk_packet_cooldown > 0.0 {
            return;
        }
        self.handle_left_click();
        self.input.walk_packet_cooldown = 0.5;
        self.input.walk_server_acked = false;
    }

    pub(crate) fn update_movement(&mut self, delta: f32, elapsed: f32) {
        for entity in self.game.entities.iter_mut() {
            entity.movement.decay_correction(delta);
            if entity.movement.is_moving() {
                entity.movement.update(elapsed);
            }
        }
        if let Some(player) = self.game.entities.player() {
            let (px, py) = player.movement.position();
            self.position_camera_at(px, py);
        }
    }
}
