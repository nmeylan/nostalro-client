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
            );
        }
        self.windows.graphic_options.toggle();
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
        persist: bool,
    ) {
        let fullscreen_changed = fullscreen != self.config.fullscreen;
        let aura_changed = display.show_level_aura != self.config.display.show_level_aura;
        let ui_scale_changed = ui_scale != self.config.dpi_scale;
        let world_filter_changed = filter_world != self.config.custom.filtering.world;
        let effect_filter_changed = filter_effects != self.config.custom.filtering.effects;
        let sprite_filter_changed = filter_sprites != self.config.custom.filtering.sprites;

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
                renderer.set_world_filtering(filter_world, self.grf.as_ref());
            }
            if effect_filter_changed {
                renderer.set_effect_filtering(filter_effects, self.grf.as_ref());
            }
        }
        if effect_filter_changed {
            self.str_effects.set_filtering(filter_effects);
        }
        if sprite_filter_changed {
            ragnarok_renderer::sprite::set_filtering(filter_sprites);
            // `load_missing_entity_sprites` rebuilds every entity but the player
            // on the next frame.
            self.game.sprite_caches.sprites.clear();
            self.game.sprite_caches.sprite_cache.clear();
            self.game.sprite_caches.guild_head_sprites.clear();
            if let Some(gid) = self.game.world.entities.player_id() {
                self.reload_player_sprite(gid);
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
