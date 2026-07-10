use crate::App;
use ragnarok_game::ailment;
use ragnarok_game::companion::OwnerCommand;
use ragnarok_game::entity::EntityType;
use ragnarok_game::path::try_move_to_range;
use ragnarok_game::targeting::can_attack;
use ragnarok_network::{build_action_request_packet, build_request_move_packet};

impl App {
    pub(crate) fn is_local_player_incapacitated(&self) -> bool {
        self.game.entities.player().is_some_and(|p| {
            ailment::movement_blocked(p.body_state, p.rooted) || p.vending_board.is_some()
        })
    }

    /// True when there is a companion whose AI can accept owner commands.
    pub(crate) fn has_companion(&self) -> bool {
        self.game
            .homunculus
            .as_ref()
            .is_some_and(|h| !h.vaporized && h.gid != 0)
            || self.game.mercenary.as_ref().is_some_and(|m| m.gid != 0)
    }

    /// Alt+right-click: order the companion to attack the hovered target, or move
    /// to the clicked cell. Shift+Alt queues the command as reserved.
    pub(crate) fn issue_owner_command(&mut self) {
        let reserved = self.input.shift_pressed;
        let player_id = self.game.entities.player_id();

        // An attackable target under the cursor becomes an attack order.
        let attack_target = self.input.right_press_target.and_then(|gid| {
            let entity = self.game.entities.get(gid)?;
            let attackable = match entity.entity_type {
                EntityType::Monster => true,
                EntityType::Player => can_attack(entity, &self.game.map_properties, player_id),
                _ => false,
            };
            attackable.then_some(gid)
        });

        let cmd = if let Some(target) = attack_target {
            OwnerCommand::attack(target)
        } else if let Some((x, y)) = self.hovered_cell() {
            OwnerCommand::move_to(x, y)
        } else {
            return;
        };

        self.push_owner_command(cmd, reserved);
    }

    pub(crate) fn push_owner_command(&mut self, cmd: OwnerCommand, reserved: bool) {
        // Homunculus takes precedence when both companions exist.
        let ai = if let Some(h) = self
            .game
            .homunculus
            .as_mut()
            .filter(|h| !h.vaporized && h.gid != 0)
        {
            &mut h.ai
        } else if let Some(m) = self.game.mercenary.as_mut().filter(|m| m.gid != 0) {
            &mut m.ai
        } else {
            return;
        };
        if reserved {
            ai.push_reserved(cmd);
        } else {
            ai.push_command(cmd);
        }
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
