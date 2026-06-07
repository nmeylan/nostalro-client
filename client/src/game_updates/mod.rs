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
        self.update_arrows(delta);
        self.check_pending_pickup();
        self.check_pending_attack(delta);
        self.check_pending_skill();
        self.load_missing_entity_sprites();
        self.update_sprite_animation(delta);
        self.update_fades(delta);
        self.game.effects.update(delta);

        self.effect_holder.drain_queue(&mut self.effect_queue);
        // Live caster facing for direction-oriented effects: RO body direction
        // (0..7) maps to world yaw at 45° per step (yaw = dir * 45). The
        // effect adds the original game's per-handler offset on top.
        let entities = &self.game.entities;
        let resolve_caster_yaw =
            |id: u32| entities.get(id).map(|e| e.direction as f32 * (std::f32::consts::TAU / 8.0));
        self.effect_holder.update(
            &EffectUpdateCtx { delta, camera_target: None, caster_yaw: None },
            &resolve_caster_yaw,
        );
        // Apply any active screen-shake to the camera so the whole
        // view trembles while an effect's shake is live.
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.camera.shake_offset = self.effect_holder.camera_shake_offset().into();
        }
    }
}
