use crate::App;
use models::enums::effect_id::EffectId;
use ragnarok_game::app_state::AppState;
use ragnarok_game::entity::{EntityState, JOB_STAR_GLADIATOR_UNION};
use ragnarok_game::path::try_move_to;
use ragnarok_game::sprite_path::OPTION_CHASEWALK;
use ragnarok_network::build_request_move_packet;

impl App {
    /// Running and Chase Walking characters stamp alternating left/right
    /// footprints on the ground, oriented to their facing, at a fixed cadence
    /// while moving. Running leaves the wide dust prints; Chase Walk leaves the
    /// dark hidden-tread prints at a slower cadence. The prints linger and fade
    /// on their own; the running stop puff is spawned separately when the
    /// running status ends.
    pub(crate) fn update_running_footprints(&mut self, delta: f32) {
        const RUN_STEP_INTERVAL: f32 = 7.0 / 60.0;
        const CHASEWALK_STEP_INTERVAL: f32 = 12.0 / 60.0;
        // Cell-space offset for each of the 8 facings (matches direction_from_positions).
        const DIR_OFFSET: [(f32, f32); 8] = [
            (0.0, 1.0),
            (-1.0, 1.0),
            (-1.0, 0.0),
            (-1.0, -1.0),
            (0.0, -1.0),
            (1.0, -1.0),
            (1.0, 0.0),
            (1.0, 1.0),
        ];
        let (Some(gat), Some(coords)) = (
            self.game.session.gat.as_ref(),
            self.game.session.map_coords.as_ref(),
        ) else {
            return;
        };
        let mut prints: Vec<(EffectId, [f32; 3], [f32; 3])> = Vec::new();
        for entity in self.game.world.entities.iter_mut() {
            let chasewalk = entity.effect_state & OPTION_CHASEWALK != 0;
            if !entity.is_running && !chasewalk {
                continue;
            }
            if entity.state != EntityState::Moving {
                entity.footstep_timer = 0.0;
                continue;
            }
            let interval = if chasewalk {
                CHASEWALK_STEP_INTERVAL
            } else {
                RUN_STEP_INTERVAL
            };
            entity.footstep_timer -= delta;
            if entity.footstep_timer > 0.0 {
                continue;
            }
            entity.footstep_timer = interval;
            entity.footstep_left = !entity.footstep_left;
            let (cx, cy) = entity.movement.position();
            let (wx, _, wz) = coords.cell_to_world(cx + 0.5, cy + 0.5);
            let from = [wx, gat.get_height(cx + 0.5, cy + 0.5), wz];
            let (ox, oy) = DIR_OFFSET[(entity.direction & 7) as usize];
            let (tx, _, tz) = coords.cell_to_world(cx + 0.5 + ox, cy + 0.5 + oy);
            let to = [tx, from[1], tz];
            let id = match (chasewalk, entity.footstep_left) {
                (true, true) => EffectId::Foot,
                (true, false) => EffectId::Foot2,
                (false, true) => EffectId::Foot3,
                (false, false) => EffectId::Foot4,
            };
            prints.push((id, from, to));
        }
        for (id, from, to) in prints {
            self.effect_queue.spawn_trail(id, from, to);
        }
    }

    /// A Star Gladiator in Union form floats above the ground, bobbing slowly
    /// and rising higher while seated.
    pub(crate) fn update_hover(&mut self, delta: f32) {
        let warm: Vec<u32> = self
            .game
            .world
            .entities
            .iter()
            .filter(|e| e.job == JOB_STAR_GLADIATOR_UNION)
            .map(|e| e.id)
            .filter(|id| self.effect_holder.has_red_body_flash(*id))
            .collect();
        for entity in self.game.world.entities.iter_mut() {
            let warm = warm.contains(&entity.id);
            entity.tick_hover(delta, warm);
        }
    }

    /// A move clicked during a swing or a pickup leaves on the next state
    /// change, whatever that state turns out to be, rather than waiting for the
    /// motion to play out.
    pub(crate) fn flush_queued_move(&mut self) {
        if self.game.combat.queued_move.is_none() {
            return;
        }
        let state = self.game.world.entities.player().map(|p| p.state);
        if state == self.game.combat.queued_move_state {
            return;
        }
        self.send_queued_move();
    }

    pub(crate) fn send_queued_move(&mut self) {
        self.game.combat.queued_move_state = None;
        let Some((dest_x, dest_y)) = self.game.combat.queued_move.take() else {
            return;
        };
        let Some(player) = self.game.world.entities.player() else {
            return;
        };
        if player.state == EntityState::Dead {
            return;
        }
        let (src_x, src_y) = player.movement.cell_position();
        let Some(gat) = &self.game.session.gat else {
            return;
        };
        if let Some(move_action) = try_move_to(gat, src_x, src_y, dest_x, dest_y) {
            self.channel.send_packet(build_request_move_packet(
                move_action.dest_x,
                move_action.dest_y,
                self.active_packetver,
            ));
        }
    }

    pub(crate) fn process_continuous_walk(&mut self, delta: f32) {
        if !self.input.left_mouse_down || self.game.session.app_state != AppState::InGame {
            return;
        }
        if self.input.ui_dragging {
            return;
        }
        if self.game.combat.attack_target_id.is_some() {
            return;
        }
        if self.game.pending_casts.pending_skill_target.is_some() {
            return;
        }
        if self.windows.chat_window.is_active() {
            return;
        }
        if self.windows.npc_dialog.dialog.is_open() || self.windows.npc_shop.shop.is_open() {
            return;
        }
        if self.windows.chat_window.contains_point(
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
        for entity in self.game.world.entities.iter_mut() {
            entity.movement.decay_correction(delta);
            if entity.movement.is_moving() {
                entity.movement.update(elapsed);
            }
        }
        if let Some(player) = self.game.world.entities.player() {
            let (px, py) = player.movement.position();
            self.position_camera_at(px, py);
        }
    }

    pub(crate) fn update_camera(&mut self, delta: f32) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        renderer.camera.interpolate(delta);
        renderer.camera.eye_floor = self
            .game
            .session
            .map_coords
            .as_ref()
            .zip(self.game.session.gat.as_ref())
            .and_then(|(coords, gat)| {
                let eye = renderer.camera.eye_unclamped();
                let (cell_x, cell_y) = coords.world_to_cell_f(eye.x, eye.z);
                let on_map = cell_x >= 0.0
                    && cell_y >= 0.0
                    && coords.is_valid_cell(cell_x as i32, cell_y as i32);
                on_map.then(|| gat.get_height(cell_x, cell_y))
            });
    }
}
