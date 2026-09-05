use crate::App;
use ragnarok_game::event::SelfConfigKind;

impl App {
    pub(super) fn handle_self_config_changed(&mut self, kind: SelfConfigKind, enabled: bool) {
        let config = &mut self.game.prefs.self_config;
        match kind {
            SelfConfigKind::RefusePartyInvite => config.refuse_party_invite = enabled,
            SelfConfigKind::OpenEquipmentWindow => config.open_equipment_window = enabled,
            SelfConfigKind::Call => config.call_enabled = enabled,
            SelfConfigKind::PetAutofeed => config.pet_autofeed = enabled,
            SelfConfigKind::HomunculusAutofeed => config.homun_autofeed = enabled,
        }
    }
}

impl App {
    pub(crate) fn open_sound_options(&mut self) {
        self.windows.sound_options.set_values(
            self.config.bgm_volume,
            self.config.sfx_volume,
            self.config.bgm_enabled,
            self.config.sfx_enabled,
            self.config.custom.sound.stereo,
            self.config.custom.sound.play_when_unfocused,
        );
        self.windows.sound_options.toggle();
    }

    pub(crate) fn open_graphic_options(&mut self) {
        if !self.windows.graphic_options.open {
            self.windows.graphic_options.set_values(
                self.config.dpi_scale,
                self.config.fullscreen,
                self.config.fog,
                self.config.show_skill_effects,
                self.config.display.clone(),
                self.config.snap,
                self.config.refuse_trade,
                self.config.refuse_party_invite,
                self.config.custom.accessibility,
                self.config.custom.filtering.world,
                self.config.custom.filtering.effects,
                self.config.custom.filtering.sprites,
                self.config.custom.filtering.sprite_upscale,
            );
        }
        self.windows.graphic_options.toggle();
    }

    /// Re-derives the sprite upscale factor from the camera and stores it.
    /// Returns the factor in force, which is 1 while the setting is off.
    pub(crate) fn refresh_sprite_upscale(&self) -> u32 {
        let Some(renderer) = self.renderer.as_ref() else {
            return ragnarok_renderer::sprite::upscale();
        };
        let logical_h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
        let ratio = self.game.session.map_coords.map(|coords| {
            ragnarok_renderer::sprite::texel_to_pixel(
                &renderer.camera,
                coords.zoom(),
                renderer.dpi_scale,
                logical_h,
            )
        });
        let factor = match (self.config.custom.filtering.sprite_upscale, ratio) {
            (true, Some(ratio)) => ratio.ceil() as u32,
            _ => 1,
        };
        ragnarok_renderer::sprite::set_upscale(factor);
        if ragnarok_profiling::debug::trace_sprite_scale() {
            tracing::info!(
                "[sprite-scale] texel_to_pixel={:.2} dpi={:.2} camera_distance={:.0} upscale={}",
                ratio.unwrap_or(0.0),
                renderer.dpi_scale,
                renderer.camera.distance,
                ragnarok_renderer::sprite::upscale(),
            );
        }
        ragnarok_renderer::sprite::upscale()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_graphics_settings(
        &mut self,
        ui_scale: f32,
        fullscreen: bool,
        fog: bool,
        show_skill_effects: bool,
        display: crate::config::DisplayOptions,
        snap: crate::config::MouseSnapPrefs,
        refuse_trade: bool,
        refuse_party_invite: bool,
        accessibility: bool,
        filter_world: bool,
        filter_effects: bool,
        filter_sprites: bool,
        sprite_upscale: bool,
        persist: bool,
    ) {
        let fullscreen_changed = fullscreen != self.config.fullscreen;
        let aura_changed = display.show_level_aura != self.config.display.show_level_aura;
        let ui_scale_changed = ui_scale != self.config.dpi_scale;
        let world_filter_changed = filter_world != self.config.custom.filtering.world;
        let effect_filter_changed = filter_effects != self.config.custom.filtering.effects;
        let sprite_filter_changed = filter_sprites != self.config.custom.filtering.sprites;
        let upscale_changed = sprite_upscale != self.config.custom.filtering.sprite_upscale;

        self.config.dpi_scale = ui_scale;
        self.config.fullscreen = fullscreen;
        self.config.fog = fog;
        self.config.show_skill_effects = show_skill_effects;
        self.config.display = display;
        self.config.snap = snap;
        self.config.refuse_trade = refuse_trade;
        self.config.refuse_party_invite = refuse_party_invite;
        self.config.custom.accessibility = accessibility;
        self.config.custom.filtering.world = filter_world;
        self.config.custom.filtering.effects = filter_effects;
        self.config.custom.filtering.sprites = filter_sprites;
        self.config.custom.filtering.sprite_upscale = sprite_upscale;
        self.game.prefs.self_config.refuse_party_invite = refuse_party_invite;

        if let Some(window) = &self.window {
            if fullscreen_changed {
                window.set_fullscreen(
                    fullscreen.then(|| winit::window::Fullscreen::Borderless(None)),
                );
            }
        }
        if ui_scale_changed {
            let new_dpi = ui_scale / 100.0;
            if let Some(renderer) = &mut self.renderer {
                renderer.set_dpi_scale(new_dpi);
                let phys_w = renderer.device.surface_config.width as f32;
                let phys_h = renderer.device.surface_config.height as f32;
                if let Some(ui_ctx) = &mut self.ui_context {
                    ui_ctx.dpi_scale = new_dpi;
                    ui_ctx.screen_width = phys_w / new_dpi;
                    ui_ctx.screen_height = phys_h / new_dpi;
                }
            }
        }
        if let Some(renderer) = &mut self.renderer {
            renderer.set_fog(if fog { self.map_fog } else { None });
            if world_filter_changed {
                renderer.set_world_filtering(filter_world, self.grf.as_deref());
            }
            if effect_filter_changed {
                renderer.set_effect_filtering(filter_effects, self.grf.as_deref());
            }
        }
        if effect_filter_changed {
            self.str_effects.set_filtering(filter_effects);
            self.effect_sprites.set_filtering(filter_effects);
            if let (Some(renderer), Some(grf)) = (&self.renderer, &self.grf) {
                self.effect_sprites.reload(
                    grf,
                    &renderer.device.device,
                    &renderer.device.queue,
                    &renderer.texture_cache.bind_group_layout,
                );
            }
        }
        if sprite_filter_changed {
            ragnarok_renderer::sprite::set_filtering(filter_sprites);
        }
        if sprite_filter_changed || upscale_changed {
            let factor = self.refresh_sprite_upscale();
            // `load_missing_entity_sprites` rebuilds every entity but the player
            // on the next frame.
            self.game.sprite_caches.sprites.clear();
            self.game.sprite_caches.sprite_cache.clear();
            self.game.sprite_caches.guild_head_sprites.clear();
            if let Some(gid) = self.game.world.entities.player_id() {
                self.reload_player_sprite(gid);
            }
            if upscale_changed {
                let message = match (sprite_upscale, filter_sprites) {
                    (true, true) => format!("Sprite upscale: {factor}x"),
                    (true, false) => {
                        "Sprite upscale applies once sprite filtering is on.".to_string()
                    }
                    (false, _) => "Sprite upscale: off".to_string(),
                };
                self.windows.chat_window.add_system(message);
            }
        }
        self.effect_queue.set_effects_enabled(show_skill_effects);
        if aura_changed {
            let gids: Vec<u32> = self.game.world.entities.iter().map(|e| e.id).collect();
            for gid in gids {
                self.refresh_level_aura(gid);
            }
        }
        if persist {
            self.config.save("config.json");
        }
    }
}
