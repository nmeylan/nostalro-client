use crate::App;
use ragnarok_game::effect::EffectQueue;
use ragnarok_game::targeting::MapProperties;

/// The session transitions whose reset policy lives in `on_session_change`.
/// Handlers that detect a transition keep their non-reset work (map loading,
/// entity spawning, packet sends) and call in here for the clears.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SessionChange {
    MapChange,
    Logout,
    Death,
    Resurrect,
}

impl App {
    pub(crate) fn on_session_change(&mut self, change: SessionChange) {
        match change {
            SessionChange::MapChange => {
                self.game.reset_effects(&mut self.effect_queue);
                self.effect_holder.clear();
                self.sound_queue.clear();
                self.sound.stop_all_sfx();
                self.game.world.arrows.clear();
                self.game.combat.damage_numbers.clear();
                self.game.character.storage.clear();
                self.game.character.trade.reset();
                self.windows.trade_window.reset_input();
                self.game.pending_confirms.pending_trade_partner = None;
                self.game.session.map_properties = MapProperties::default();
                self.game.combat.damage_numbers.combat_hidden = false;
                self.game.pending_casts.pending_skill_target = None;
                self.game.pending_casts.pending_skill_id = None;
                self.game.pending_casts.pending_skill_level = None;
                self.game.combat.attack_target_id = None;
                self.game.combat.attack_request_sent = false;
                self.game.combat.queued_move = None;
                self.game.npc_cutins = [None, None, None];
                self.game.session.progress_bar = None;
            }
            SessionChange::Logout => {
                self.window_state_restored = false;
                self.game.session.screen_ripple = false;
                self.char_select_window = None;
                self.game.character.clear();
                self.game.world.entities.clear();
                self.game.sprite_caches.sprites.clear();
                self.game.sprite_caches.carts.clear();
                self.game.sprite_caches.falcons.clear();
                self.game.sprite_caches.gr2_models.clear();
                if let Some(renderer) = &mut self.renderer {
                    renderer.gr2_models.clear();
                }
                self.game.sprite_caches.sprite_cache.clear();
                self.game.world.floor_items.clear();
                self.game.world.graffiti.clear();
                self.game.world.cast_marks.clear();
                self.game.assets.floor_item_sprites.clear();
                self.game.chat_rooms.clear();
                self.game.combat.waiting_item_throw_ack = false;
                self.windows.drop_quantity_dialog = None;
                self.windows.guild_expel_dialog = None;
                self.windows.skill_talkbox_dialog = None;
                self.windows.card_insert_dialog = None;
                self.game.pending_casts.pending_card_composition_index = None;
                self.game.pending_casts.pending_pickup_item_id = None;
                self.game.combat.attack_target_id = None;
                self.game.combat.attack_request_sent = false;
                self.game.combat.queued_move = None;
                self.game.companions.homunculus = None;
                self.game.companions.mercenary = None;
                self.game.companions.pet = ragnarok_game::pet::PetState::default();
                self.game.companions.capture_targeting = false;
                self.game.companions.pet_roulette = None;
                self.game.quest_log.clear();
                self.game.quest_markers.clear();
                self.game.minimap_marks.clear();
                self.windows.pet_window.set_visible(false);
                self.game.companions.companion_attack_target = [None; 2];
                self.windows.homunculus_window.set_visible(false);
                self.windows.mercenary_window.set_visible(false);
                self.game.guild = None;
                self.game.sprite_caches.guild_head_sprites.clear();
                self.windows.guild_window.open = false;
                self.windows.world_map_window.close();
                self.game.session.current_map = None;
                self.game.session.progress_bar = None;
                self.game.session.map_coords = None;
                self.game.session.gat = None;
                self.effect_holder.clear();
                self.effect_queue = EffectQueue::new();
                self.game.schedulers.ambient_effects =
                    ragnarok_game::effects::AmbientEffectScheduler::empty();
                self.game.schedulers.ambient_sounds =
                    ragnarok_game::sound::ambient::AmbientSoundScheduler::empty();
                self.game.schedulers.repeat_sounds.clear();
                self.game.effect_keys.status_buff_keys.clear();
                self.game.effect_keys.opt3_keys.clear();
                self.game.effect_keys.next_status_buff_key = 0;
                self.game.effect_keys.level_aura_keys.clear();
                self.game.effect_keys.boss_aura_keys.clear();
                self.game.effect_keys.warp_portal_keys.clear();
                self.game.effect_keys.sight_aura_keys.clear();
                self.game.effect_keys.ruwach_aura_keys.clear();
                self.game.schedulers.day_night.reset();
                if let Some(renderer) = &mut self.renderer {
                    renderer.ground_renderer = None;
                    renderer.model_renderer = None;
                    renderer.water_renderer = None;
                    renderer.grid_selector = None;
                }
                self.play_bgm_track("01.mp3");
            }
            SessionChange::Death => {
                self.game.session.player_dead = true;
                self.game.session.screen_ripple = false;
                self.windows.system_menu.open_dead();
            }
            SessionChange::Resurrect => {
                self.game.session.player_dead = false;
                self.windows.system_menu.close_dead();
            }
        }
    }
}
