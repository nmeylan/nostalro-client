mod animation;
mod combat;
mod items;
mod movement;

use crate::App;
use ragnarok_game::damage_number::DamageNumber;
use ragnarok_renderer::effect::EffectUpdateCtx;

impl App {
    pub(crate) fn run_game_updates(&mut self, delta: f32, elapsed: f32) {
        self.update_movement(delta, elapsed);
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
        self.update_cart_animations(delta);
        self.update_falcon_visuals(delta);
        self.update_fades(delta);
        // RSW ambient effects (torch/smoke/bubble/…) spawn through the shared
        // queue, driven only near the camera. Run before `drain_queue` so this
        // frame's spawn/despawn requests are picked up immediately.
        let camera_target = self
            .renderer
            .as_ref()
            .map(|r| r.camera.target.to_array())
            .unwrap_or([0.0; 3]);
        self.game
            .ambient_effects
            .update(delta, camera_target, &mut self.effect_queue);

        // Live caster facing for direction-oriented effects: RO body direction
        // (0..7) maps to world yaw at 45° per step (yaw = dir * 45). The
        // effect adds the original game's per-handler offset on top.
        let entities = &self.game.entities;
        let resolve_caster_yaw =
            |id: u32| entities.get(id).map(|e| e.direction as f32 * (std::f32::consts::TAU / 8.0));
        // Live world position of an entity for entity-anchored effects:
        // interpolated cell → world at the **ground** (`get_height`), matching
        // the sprite feet anchor. Effects that need to sit higher (the Hit-family
        // ring at torso, a tether at chest) apply their own lift from this
        // ground point.
        let gat = self.game.gat.as_ref();
        let map_coords = self.game.map_coords.as_ref();
        let resolve_entity_pos = |id: u32| {
            let (gat, coords) = (gat?, map_coords?);
            let (cx, cy) = entities.get(id)?.movement.position();
            let (wx, _, wz) = coords.cell_to_world(cx + 0.5, cy + 0.5);
            Some([wx, gat.get_height(cx + 0.5, cy + 0.5), wz])
        };
        self.effect_holder.drain_queue(&mut self.effect_queue, &resolve_entity_pos);
        self.effect_holder.update(
            &EffectUpdateCtx { delta, camera_target: None, caster_yaw: None },
            &resolve_caster_yaw,
            &resolve_entity_pos,
        );
        // Apply any active screen-shake to the camera so the whole
        // view trembles while an effect's shake is live.
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.camera.shake_offset = self.effect_holder.camera_shake_offset().into();
        }

        // Lazily load STR files referenced by live effects but not yet cached —
        // chiefly Custom effects' `str_overlay`s (Storm Gust's storm, …), whose
        // names (often per-instance) the map-load preload can't enumerate. The
        // cache short-circuits already-loaded / known-missing names, so this is
        // a cheap per-frame check that runs before the effect first renders.
        if let (Some(renderer), Some(grf)) = (self.renderer.as_mut(), self.grf.as_ref()) {
            for name in self.effect_holder.live_str_names() {
                if self.str_effects.get(&name).is_none() {
                    self.str_effects.load(
                        &name,
                        &[],
                        grf,
                        &mut renderer.texture_cache,
                        &renderer.device.device,
                        &renderer.device.queue,
                    );
                }
            }
        }

        // Floating recoloured numbers (§9b Damage1/Damage12/Damage13) reach the
        // shared damage-number manager through the effect channel: the holder
        // surfaces a one-shot request per entity, same shape as the shake/sfx
        // drains above. EffectNumber has no horizontal drift, so direction is 0.
        for (entity_id, req) in self.effect_holder.drain_number_requests() {
            self.game.damage_numbers.add(DamageNumber::effect_number(
                entity_id, req.value, req.color, 0,
            ));
        }
    }
}
