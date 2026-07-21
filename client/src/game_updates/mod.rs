mod animation;
mod combat;
mod companion;
mod items;
mod movement;

use crate::App;
use models::enums::EnumWithStringValue;
use ragnarok_formats::act::SpriteAnimationState;
use ragnarok_game::app_state::AppState;
use ragnarok_game::damage_number::DamageNumber;
use ragnarok_renderer::effect::EffectUpdateCtx;

/// Reserved sprite key for the (single) character-creation preview.
pub(crate) const CREATE_PREVIEW_GID: u32 = u32::MAX;

impl App {
    fn update_account_sprites(&mut self, _delta: f32, elapsed: f32) {
        if self.game.session.app_state != AppState::CharacterCreate {
            return;
        }
        let appearance = match &self.char_create_window {
            Some(w) => (w.hair_style, w.hair_color),
            None => return,
        };
        if self.char_create_built_appearance != Some(appearance) {
            let sex = self.game.session.login_session.as_ref().map(|s| s.sex).unwrap_or(0);
            self.load_player_sprite(
                CREATE_PREVIEW_GID,
                0,
                sex,
                appearance.0,
                appearance.1,
                0,
                None,
                0,
                0,
                0,
                0,
            );
            self.char_create_built_appearance = Some(appearance);
            self.account_anims
                .insert(CREATE_PREVIEW_GID, SpriteAnimationState::new(0));
        }
        let dir = ((elapsed / 0.5) as u32 % 8) as u8;
        if let Some(anim) = self.account_anims.get_mut(&CREATE_PREVIEW_GID) {
            anim.set_direction(dir);
        }
    }

    pub(crate) fn run_game_updates(&mut self, delta: f32, elapsed: f32) {
        self.update_account_sprites(delta, elapsed);
        let now_ms = self.start_time.elapsed().as_millis() as u64;
        self.game.character.prune_expired(now_ms);
        self.update_movement(delta, elapsed);
        self.process_continuous_walk(delta);
        self.update_entity_state(delta);
        self.update_companion_ai(delta);
        self.game.combat.damage_numbers.update(delta);
        self.process_scheduled_hits();
        self.process_caster_replays();
        self.update_floor_items(elapsed);
        self.update_arrows(delta);
        self.check_pending_pickup();
        self.check_pending_attack(delta);
        self.check_pending_skill();
        self.check_pending_ground_skill();
        self.load_missing_entity_sprites();
        self.update_sprite_animation(delta);
        self.update_gr2_models(elapsed);
        self.update_running_footprints(delta);
        self.update_cart_animations(delta);
        self.update_falcon_visuals(delta);
        self.update_pet_roulette(delta);
        self.update_fades(delta);
        self.game.day_night.tick(delta);
        if self.game.day_night.take_dirty()
            && let Some(renderer) = &mut self.renderer
        {
            renderer.set_day_night(
                self.game.day_night.world_diffuse(),
                self.game.day_night.sprite_light(),
            );
        }
        let camera = self.renderer.as_ref().map(|r| &r.camera);
        let is_visible = |pos: [f32; 3]| {
            camera.is_some_and(|c| c.is_world_pos_visible(pos[0], pos[1], pos[2], 0.25))
        };
        self.game
            .ambient_effects
            .update(delta, &is_visible, &mut self.effect_queue);

        let entities = &self.game.entities;
        let resolve_caster_yaw = |id: u32| {
            entities
                .get(id)
                .map(|e| e.direction as f32 * (std::f32::consts::TAU / 8.0))
        };
        let gat = self.game.session.gat.as_ref();
        let map_coords = self.game.session.map_coords.as_ref();
        let resolve_entity_pos = |id: u32| {
            let (gat, coords) = (gat?, map_coords?);
            let (cx, cy) = entities.get(id)?.movement.position();
            let (wx, _, wz) = coords.cell_to_world(cx + 0.5, cy + 0.5);
            Some([wx, gat.get_height(cx + 0.5, cy + 0.5), wz])
        };
        if !self.effect_queue.pending.is_empty() {
            let t = self.start_time.elapsed().as_millis();
            for req in &self.effect_queue.pending {
                tracing::info!(
                    "[effect-timing t={t}ms] queue drain -> spawn {} (attach={:?})",
                    req.effect_id.as_str(),
                    req.attach,
                );
            }
        }
        self.game
            .repeat_sounds
            .update(delta, &resolve_entity_pos, &mut self.sound_queue);
        self.effect_holder
            .drain_queue(&mut self.effect_queue, &resolve_entity_pos);
        self.effect_holder.update(
            &EffectUpdateCtx {
                delta,
                camera_target: None,
                caster_yaw: None,
            },
            &resolve_caster_yaw,
            &resolve_entity_pos,
        );
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.camera.shake_offset = self.effect_holder.camera_shake_offset().into();
        }

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

        for (entity_id, req) in self.effect_holder.drain_number_requests() {
            self.game.combat.damage_numbers.add(DamageNumber::effect_number(
                entity_id, req.value, req.color, 0,
            ));
        }

        self.sync_trap_models();
    }

    /// Build/remove the deployed-trap RSM models to match `game.trap_units`.
    fn sync_trap_models(&mut self) {
        let (Some(renderer), Some(grf)) = (self.renderer.as_mut(), self.grf.as_ref()) else {
            return;
        };
        let live: std::collections::HashSet<u32> = self.game.trap_units.keys().copied().collect();
        renderer.retain_skill_unit_models(&live);
        if self.game.trap_units.is_empty() {
            return;
        }
        let scale_factor = self
            .game
            .session.map_coords
            .as_ref()
            .map(|c| c.zoom() / 10.0)
            .unwrap_or(1.0);
        for (&aid, &(unit_id, world)) in &self.game.trap_units {
            if renderer.has_skill_unit_model(aid) {
                continue;
            }
            let Some(name) = ragnarok_game::effect::trap_model_name(unit_id) else {
                continue;
            };
            let path = format!("data\\model\\{name}");
            match grf.read_file(&path) {
                Ok(bytes) => match ragnarok_formats::rsm::RsmFile::parse(&bytes) {
                    Ok(rsm) => renderer.add_skill_unit_model(aid, &rsm, grf, world, scale_factor),
                    Err(e) => tracing::warn!("trap model parse failed {path}: {e}"),
                },
                Err(e) => tracing::warn!("trap model read failed {path}: {e}"),
            }
        }
    }

}
