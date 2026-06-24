use crate::App;
use models::enums::EnumWithNumberValue;
use models::enums::status::StatusTypes;

impl App {
    pub(super) fn handle_parameter_changed(&mut self, var_id: u16, value: i32) {
        if let Some(speed) = self.game.character.apply_parameter_changed(var_id, value)
            && let Some(entity) = self.game.entities.player_mut()
        {
            entity.speed = speed;
            entity.movement.set_speed(speed);
        }
        // Dinging max level (or losing it) toggles the local player's aura — a
        // derived trigger, not an effect packet.
        if let Ok(StatusTypes::Baselevel) = StatusTypes::try_from_value(var_id as usize)
            && let Some(gid) = self.game.entities.player_id()
        {
            self.refresh_level_aura(gid);
        }
    }
}
