use crate::App;
use models::enums::EnumWithNumberValue;
use models::enums::status::StatusTypes;
use ragnarok_game::damage_number::DamageNumber;

impl App {
    pub(super) fn handle_recovery(&mut self, var_id: u16, amount: i32) {
        let Some(gid) = self.game.world.entities.player_id() else {
            return;
        };
        let color = match StatusTypes::try_from_value(var_id as usize) {
            Ok(StatusTypes::Sp) => [0.0, 0.0, 1.0],
            _ => [0.0, 1.0, 0.0],
        };
        self.game
            .combat
            .damage_numbers
            .add(DamageNumber::effect_number(gid, amount, color, 0));
    }

    pub(super) fn handle_parameter_changed(&mut self, var_id: u16, value: i32) {
        if let Some(speed) = self.game.character.apply_parameter_changed(var_id, value)
            && let Some(entity) = self.game.world.entities.player_mut()
        {
            tracing::info!(
                previous = entity.speed,
                speed,
                "player movement speed from server"
            );
            entity.speed = speed;
            entity.movement.set_speed(speed);
        }
        if let Ok(StatusTypes::Baselevel) = StatusTypes::try_from_value(var_id as usize)
            && let Some(gid) = self.game.world.entities.player_id()
        {
            self.refresh_level_aura(gid);
        }
    }
}
