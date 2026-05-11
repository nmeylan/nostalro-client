mod animation;
mod combat;
mod items;
mod movement;

use crate::App;
use ragnarok_renderer::effect::EffectUpdateCtx;

impl App {
    pub(crate) fn run_game_updates(&mut self, delta: f32, elapsed: f32) {
        self.update_movement(elapsed);
        self.process_continuous_walk(delta);
        self.update_entity_state(delta);
        self.game.damage_numbers.update(delta);
        self.process_scheduled_hits();
        self.process_caster_replays();
        self.update_floor_items(elapsed);
        self.check_pending_pickup();
        self.check_pending_attack(delta);
        self.check_pending_skill();
        self.load_missing_entity_sprites();
        self.update_sprite_animation(delta);
        self.update_fades(delta);
        self.game.effects.update(delta);

        self.effect_holder.drain_queue(&mut self.effect_queue);
        self.effect_holder.update(&EffectUpdateCtx { dt: delta });
    }
}
