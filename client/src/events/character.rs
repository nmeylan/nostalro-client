use crate::App;

impl App {
    pub(super) fn handle_parameter_changed(&mut self, var_id: u16, value: i32) {
        if let Some(speed) = self.game.character.apply_parameter_changed(var_id, value) {
            if let Some(entity) = self.game.entities.player_mut() {
                entity.speed = speed;
                entity.movement.set_speed(speed);
            }
        }
    }
}
