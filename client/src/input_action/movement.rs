use crate::App;
use models::enums::EnumWithNumberValue;
use models::enums::action::ActionType;
use models::enums::skill_enums::SkillEnum;
use ragnarok_game::ailment;
use ragnarok_game::autocounter;
use ragnarok_game::companion::OwnerCommand;
use ragnarok_game::entity::EntityType;
use ragnarok_game::path::{in_attack_range, try_move_to_range};
use ragnarok_game::sprite_path::{OPTION_HIDE, hide_blocks_move};
use ragnarok_game::targeting::can_attack;
use ragnarok_network::{build_action_request_packet, build_request_move_packet};

impl App {
    pub(crate) fn is_local_player_incapacitated(&self) -> bool {
        self.game.world.entities.player().is_some_and(|p| {
            ailment::movement_blocked(p.body_state, p.rooted) || p.vending_board.is_some()
        })
    }

    /// The local player is Hiding (not cloaking) without Tunnel Drive: the
    /// server drops any walk request, so we never send one.
    pub(crate) fn player_hide_move_blocked(&self) -> bool {
        let effect_state = self
            .game
            .world
            .entities
            .player()
            .map(|p| p.effect_state)
            .unwrap_or(0);
        let knows_tunnel_drive = self
            .game
            .character
            .skills
            .get_skill(SkillEnum::RgTunneldrive.id() as u16)
            .is_some();
        hide_blocks_move(effect_state, knows_tunnel_drive)
    }

    /// The local player is Hiding: attack, pickup, sit/stand, item use and most
    /// skills are blocked while the bit is set.
    pub(crate) fn player_hidden(&self) -> bool {
        self.game
            .world
            .entities
            .player()
            .is_some_and(|p| p.effect_state & OPTION_HIDE != 0)
    }

    pub(crate) fn has_homunculus(&self) -> bool {
        self.game
            .companions
            .homunculus
            .as_ref()
            .is_some_and(|h| !h.vaporized && h.gid != 0)
    }

    pub(crate) fn has_mercenary(&self) -> bool {
        self.game
            .companions
            .mercenary
            .as_ref()
            .is_some_and(|m| m.gid != 0)
    }

    pub(crate) fn issue_owner_command(&mut self, is_mercenary: bool, hovered_target: Option<u32>) {
        let reserved = self.input.shift_pressed;
        let player_id = self.game.world.entities.player_id();

        // An attackable target under the cursor becomes an attack order; anything
        // else is a move order to the hovered cell. Shift queues the command
        // behind the current action instead of replacing it.
        let attack_target = hovered_target.and_then(|gid| {
            let entity = self.game.world.entities.get(gid)?;
            let attackable = match entity.entity_type {
                EntityType::Monster => true,
                EntityType::Player => {
                    can_attack(entity, &self.game.session.map_properties, player_id)
                }
                _ => false,
            };
            attackable.then_some(gid)
        });

        if let Some(target) = attack_target {
            self.push_owner_command_to(is_mercenary, OwnerCommand::attack(target), reserved);
        } else if let Some((x, y)) = self.hovered_cell() {
            self.push_owner_command_to(is_mercenary, OwnerCommand::move_to(x, y), reserved);
        }
    }

    /// Send a command to one companion (homunculus when `is_mercenary` is false).
    pub(crate) fn push_owner_command_to(
        &mut self,
        is_mercenary: bool,
        cmd: OwnerCommand,
        reserved: bool,
    ) {
        let ai = if is_mercenary {
            self.game
                .companions
                .mercenary
                .as_mut()
                .filter(|m| m.gid != 0)
                .map(|m| &mut m.ai)
        } else {
            self.game
                .companions
                .homunculus
                .as_mut()
                .filter(|h| !h.vaporized && h.gid != 0)
                .map(|h| &mut h.ai)
        };
        let Some(ai) = ai else {
            tracing::info!(
                "push_owner_command_to: no companion AI (is_mercenary={is_mercenary}) — dropped"
            );
            return;
        };
        tracing::info!("push_owner_command_to: is_mercenary={is_mercenary} reserved={reserved}");
        if reserved {
            ai.push_reserved(cmd);
        } else {
            ai.push_command(cmd);
        }
    }

    pub(crate) fn companion_is_patrolling(&self, is_mercenary: bool) -> bool {
        if is_mercenary {
            self.game
                .companions
                .mercenary
                .as_ref()
                .is_some_and(|m| m.ai.is_patrolling())
        } else {
            self.game
                .companions
                .homunculus
                .as_ref()
                .is_some_and(|h| h.ai.is_patrolling())
        }
    }

    pub(crate) fn initiate_attack(&mut self, target_id: u32) {
        let locked = self.game.prefs.noctrl_mode || self.input.ctrl_pressed;
        self.begin_attack(target_id, locked);
    }

    /// Re-enters the attack path after the server reported the target out of
    /// range, keeping the lock the attack was started with. Drops the intent when
    /// neither a swing nor a walk can be started, so the retry cannot loop.
    pub(crate) fn resume_attack(&mut self, target_id: u32) {
        let locked = self.game.combat.attack_is_locked;
        let request_outstanding = self.game.combat.attack_request_sent;
        if self.begin_attack(target_id, locked) {
            return;
        }
        self.game.combat.attack_request_sent = request_outstanding;
        self.stop_attacking();
    }

    /// Returns whether the target ended up engaged: a swing went out, or a walk
    /// toward it started.
    fn begin_attack(&mut self, target_id: u32, locked: bool) -> bool {
        if self.player_hidden() {
            return false;
        }
        self.game.pending_casts.pending_pickup_item_id = None;
        self.game.combat.queued_move = None;
        self.game.combat.attack_is_locked = locked;
        self.game.combat.attack_request_sent = false;

        let target_pos = match self.game.world.entities.get(target_id) {
            Some(e) => e.movement.cell_position(),
            None => return false,
        };
        let (px, py) = self
            .game
            .world
            .entities
            .player()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));

        let range = self.game.combat.attack_range as i32;
        if in_attack_range(
            px as i32,
            py as i32,
            target_pos.0 as i32,
            target_pos.1 as i32,
            range,
        ) {
            self.send_attack_packet(target_id);
            self.game.combat.attack_target_id = Some(target_id);
            self.game.combat.attack_request_cooldown = 0.3;
            true
        } else if self.try_move_toward(target_pos.0 as i32, target_pos.1 as i32, px, py, range) {
            self.game.combat.attack_target_id = Some(target_id);
            true
        } else {
            false
        }
    }

    pub(crate) fn send_attack_packet(&mut self, target_id: u32) {
        self.game.combat.last_attacked_enemy = Some(target_id);
        self.game.combat.attack_request_sent = true;
        self.channel.send_packet(build_action_request_packet(
            target_id,
            ActionType::AttackRepeat.value() as u8,
            self.active_packetver,
        ));
    }

    /// Drops the local attack target and tells the server to stop the repeat
    /// attack it is still running on our behalf.
    pub(crate) fn stop_attacking(&mut self) {
        self.game.combat.attack_target_id = None;
        if !self.game.combat.attack_request_sent {
            return;
        }
        self.game.combat.attack_request_sent = false;
        self.channel
            .send_packet(ragnarok_network::build_cancel_lockon_packet(
                self.active_packetver,
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
        if autocounter::player_in_autocounter(&self.game.world.entities) {
            self.dispel_autocounter();
            return false;
        }
        if self.is_local_player_incapacitated()
            || self.player_hide_move_blocked()
            || self
                .game
                .world
                .entities
                .player()
                .is_some_and(|p| p.is_move_locked())
        {
            return false;
        }
        let gat = match &self.game.session.gat {
            Some(g) => g,
            None => return false,
        };
        if let Some(move_action) = try_move_to_range(gat, px, py, target_x, target_y, range) {
            let is_moving = self
                .game
                .world
                .entities
                .player()
                .is_some_and(|p| p.movement.is_moving());
            if is_moving {
                let dest_changed = self
                    .game
                    .world
                    .entities
                    .player()
                    .and_then(|p| p.movement.destination())
                    .is_none_or(|(dx, dy)| dx != move_action.dest_x || dy != move_action.dest_y);
                if dest_changed {
                    self.channel.send_packet(build_request_move_packet(
                        move_action.dest_x,
                        move_action.dest_y,
                        self.active_packetver,
                    ));
                }
                return true;
            }
            self.channel.send_packet(build_request_move_packet(
                move_action.dest_x,
                move_action.dest_y,
                self.active_packetver,
            ));
            true
        } else {
            false
        }
    }
}
