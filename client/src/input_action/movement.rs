use crate::App;
use ragnarok_game::ailment;
use ragnarok_game::path::try_move_to_range;
use ragnarok_network::{build_action_request_packet, build_request_move_packet};

impl App {
    pub(crate) fn is_local_player_incapacitated(&self) -> bool {
        self.game.entities.player().is_some_and(|p| {
            ailment::movement_blocked(p.body_state, p.rooted) || p.vending_board.is_some()
        })
    }

    pub(crate) fn initiate_attack(&mut self, target_id: u32) {
        self.game.pending_pickup_item_id = None;
        let locked = self.game.noctrl_mode || self.input.ctrl_pressed;
        self.game.attack_is_locked = locked;

        let target_pos = match self.game.entities.get(target_id) {
            Some(e) => e.movement.cell_position(),
            None => return,
        };
        let (px, py) = self
            .game
            .entities
            .player()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));

        let range = self.game.attack_range as i32;
        let dx = (px as i32 - target_pos.0 as i32).abs();
        let dy = (py as i32 - target_pos.1 as i32).abs();
        let dist = dx.max(dy);

        if dist <= range {
            self.send_attack_packet(target_id);
            self.game.attack_target_id = Some(target_id);
            self.game.attack_request_cooldown = 0.3;
        } else if self.try_move_toward(target_pos.0 as i32, target_pos.1 as i32, px, py, range) {
            self.game.attack_target_id = Some(target_id);
        }
    }

    pub(crate) fn send_attack_packet(&self, target_id: u32) {
        self.channel.send_packet(build_action_request_packet(
            target_id,
            7,
            self.config.packetver,
        ));
    }

    pub(crate) fn try_move_toward(
        &mut self,
        target_x: i32,
        target_y: i32,
        px: u16,
        py: u16,
        range: i32,
    ) -> bool {
        if self.is_local_player_incapacitated() {
            return false;
        }
        let gat = match &self.game.gat {
            Some(g) => g,
            None => return false,
        };
        if let Some(move_action) = try_move_to_range(gat, px, py, target_x, target_y, range) {
            let is_moving = self
                .game
                .entities
                .player()
                .is_some_and(|p| p.movement.is_moving());
            if is_moving {
                let dest_changed = self
                    .game
                    .entities
                    .player()
                    .and_then(|p| p.movement.destination())
                    .is_none_or(|(dx, dy)| dx != move_action.dest_x || dy != move_action.dest_y);
                if dest_changed {
                    self.channel.send_packet(build_request_move_packet(
                        move_action.dest_x,
                        move_action.dest_y,
                        self.config.packetver,
                    ));
                }
                return true;
            }
            self.channel.send_packet(build_request_move_packet(
                move_action.dest_x,
                move_action.dest_y,
                self.config.packetver,
            ));
            let elapsed = self.start_time.elapsed().as_secs_f32();
            if let Some(entity) = self.game.entities.player_mut() {
                entity.movement.start_move(move_action.path, elapsed);
            }
            true
        } else {
            false
        }
    }
}
