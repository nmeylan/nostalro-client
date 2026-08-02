use crate::App;

impl App {
    pub(crate) fn update_floor_items(&mut self, elapsed: f32) {
        for floor_item in self.game.world.floor_items.values_mut() {
            if floor_item.is_falling {
                let t = (elapsed - floor_item.drop_time) * 1000.0 / 24.0;
                let fall_offset = -15.0 + (-0.6 + 0.083 * t as f64) * t as f64;
                if fall_offset >= 0.0 {
                    floor_item.is_falling = false;
                }
            }
        }
    }

    pub(crate) fn request_producer_names(&mut self, now_ms: u64) {
        for char_id in self.game.character.pending_producer_names(now_ms) {
            self.channel
                .send_packet(ragnarok_network::build_solve_char_name_packet(
                    char_id,
                    self.active_packetver,
                ));
        }
    }
}
