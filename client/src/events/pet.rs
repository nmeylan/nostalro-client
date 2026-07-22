use crate::App;
use ragnarok_game::event::PetProperty;
use ragnarok_ui_component::game::item_list_selection_window::{ListContext, ListRow};

impl App {
    pub(super) fn handle_pet_property(&mut self, property: PetProperty) {
        self.game.companions.pet.apply_property(&property);
        let illust = self.game.companions.pet.illust_path().to_string();
        self.preload_item_icons(vec![illust]);
    }

    pub(super) fn handle_pet_state_changed(&mut self, ty: i8, gid: u32, data: i32) {
        use ragnarok_game::pet::{PET_STATE_ACCESSORY, PET_STATE_PERFORMANCE};
        self.game.companions.pet.apply_state_changed(ty, gid, data);
        match ty {
            PET_STATE_ACCESSORY => {
                let accessory = data as u16;
                let job = self
                    .game
                    .world
                    .entities
                    .get(gid)
                    .map(|e| e.job)
                    .unwrap_or(self.game.companions.pet.job as u16);
                if let Some(entity) = self.game.world.entities.get_mut(gid) {
                    entity.pet_accessory = accessory;
                }
                self.load_pet_sprite(gid, job, accessory);
            }
            PET_STATE_PERFORMANCE => {
                // data 1..=3 → PERF1/2/3 (rows 6/7/8), 4 → SPECIAL (row 5).
                let action = 5 + (data.clamp(1, 4) as usize % 4);
                if let Some(entity) = self.game.world.entities.get_mut(gid) {
                    entity.forced_animation = Some(ragnarok_game::entity::ForcedAnimation::new(
                        action, 0, 800.0,
                    ));
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_pet_feed_result(&mut self, ok: bool, food_item_id: u16) {
        if !ok {
            let name = self
                .game
                .data_table
                .item_name
                .as_ref()
                .map(|t| t.get_name_or_id(food_item_id))
                .unwrap_or_else(|| format!("Item #{food_item_id}"));
            self.windows
                .chat_window
                .add_error(format!("You don't have {name}."));
            return;
        }
        // Feeding uses the hunger band the pet had before this feed lifted it.
        self.emit_pet_act(0);
    }

    /// Owner-side chatter: rolls an emote for the given pet act (PM_*) via the
    /// hunger×intimacy×act table and broadcasts it through CZ_PET_ACT.
    pub(crate) fn emit_pet_act(&mut self, act: usize) {
        if self.game.companions.pet.gid.is_none() {
            return;
        }
        let hunger = self.game.companions.pet.hunger_state().index();
        let friendly = self.game.companions.pet.intimacy_state().index();
        if let Some(emote) = ragnarok_game::pet_tables::pet_emotion(hunger, friendly, act) {
            self.channel.send_packet(ragnarok_network::build_pet_act_packet(
                emote as i32,
                self.active_packetver,
            ));
        }
    }

    pub(super) fn handle_pet_capture_start(&mut self) {
        self.game.companions.pet.capture_pending = true;
        self.game.companions.capture_targeting = true;
    }

    pub(super) fn handle_pet_capture_result(&mut self, ok: bool) {
        self.game.companions.pet.capture_pending = false;
        if let Some(roulette) = &mut self.game.companions.pet_roulette {
            roulette.resolve(ok);
        }
    }

    pub(crate) fn try_pet_modal_click(&mut self) -> bool {
        // Capture roulette is modal: a click confirms the spinning attempt.
        if let Some(roulette) = &mut self.game.companions.pet_roulette {
            if roulette.state == ragnarok_game::pet::RouletteState::Idle && !roulette.sent {
                roulette.sent = true;
                let gid = roulette.target_gid;
                self.channel
                    .send_packet(ragnarok_network::build_trycapture_packet(
                        gid,
                        self.active_packetver,
                    ));
            }
            return true;
        }
        // Capture targeting armed by ZC_START_CAPTURE: a click on a valid mob opens
        // the roulette (players and the caster's own pet are not valid targets).
        if self.game.companions.capture_targeting {
            if let Some(entity_id) = self.game.hover.hovered_entity_id
                && self.game.companions.pet.gid != Some(entity_id)
                && self
                    .game
                    .world
                    .entities
                    .get(entity_id)
                    .is_some_and(|e| {
                        e.entity_type == ragnarok_game::entity::EntityType::Monster && !e.is_pet
                    })
            {
                self.open_capture_roulette(entity_id);
            }
            return true;
        }
        false
    }

    /// Loads the slotmachine sprite and opens the roulette for the picked mob.
    pub(crate) fn open_capture_roulette(&mut self, target_gid: u32) {
        self.game.companions.capture_targeting = false;
        self.game.companions.pet_roulette = Some(ragnarok_game::pet::PetRoulette::new(target_gid));
        if self.roulette_act.is_some() {
            return;
        }
        if let Some(grf) = &self.grf {
            let data = ragnarok_game::sprite_loader::load_sprite_data(
                grf,
                "data/sprite/slotmachine.spr",
                "data/sprite/slotmachine.act",
            );
            if let Some(data) = data
                && let Some(textures) = self.upload_sprite(&data)
            {
                self.roulette_textures = Some(textures);
                self.roulette_act = Some(data.act);
            }
        }
    }

    /// Advances the roulette state machine and auto-closes once the result
    /// animation has played out.
    pub(crate) fn update_pet_roulette(&mut self, dt: f32) {
        let now = self.start_time.elapsed().as_secs_f32();
        let close = self
            .game
            .companions
            .pet_roulette
            .as_ref()
            .and_then(|r| r.close_at)
            .is_some_and(|t| now >= t);
        if close {
            self.game.companions.pet_roulette = None;
            return;
        }
        let Some(act) = &self.roulette_act else {
            return;
        };
        if let Some(roulette) = &mut self.game.companions.pet_roulette {
            roulette.advance(act, dt * 1000.0, now);
        }
    }

    pub(super) fn handle_pet_egg_list(&mut self, indices: Vec<u16>) {
        let rows: Vec<ListRow> = indices
            .iter()
            .map(|&idx| {
                let item = self.game.character.inventory.get_item(idx);
                ListRow {
                    name: item.map(|it| it.name.clone()).unwrap_or_default(),
                    icon: item.and_then(|it| it.icon_path()),
                    index: idx as i16,
                    item_id: item.map(|it| it.item_id).unwrap_or(0),
                    refine: 0,
                    cards: [0; 4],
                    skill_id: 0,
                }
            })
            .collect();
        self.windows
            .item_list_selection_window
            .open("Hatch Pet", ListContext::SelectPetEgg, rows);
    }

    pub(super) fn handle_pet_act(&mut self, gid: u32, data: i32) {
        let Some(code) = ragnarok_game::pet::decode_pet_talk(data) else {
            // Plain emotion id.
            self.game.world.entities.apply_entity_emotion(gid, data as u8);
            return;
        };
        // Talk line: resolve a random sentence from pettalktable.xml, keyed by the
        // pet's real name (lowercased) and the encoded hunger/act.
        let hunger = ragnarok_game::pet::HUNGER_KEYS
            .get(code.hunger)
            .copied()
            .unwrap_or("noting");
        let act = ragnarok_game::pet::ACT_KEYS
            .get(code.act)
            .copied()
            .unwrap_or("normal");
        let mob_key = self
            .game
            .world
            .entities
            .get(gid)
            .and_then(|e| e.name.clone())
            .unwrap_or_default()
            .to_lowercase();
        let line = self
            .game
            .data_table
            .pet_talk
            .as_ref()
            .and_then(|t| t.lines(&mob_key, hunger, act))
            .filter(|lines| !lines.is_empty())
            .map(|lines| {
                let idx = (self.start_time.elapsed().as_nanos() as usize) % lines.len();
                lines[idx].clone()
            });
        if let Some(line) = line {
            if let Some(entity) = self.game.world.entities.get_mut(gid) {
                entity.chat_bubble =
                    Some(ragnarok_game::entity::ChatBubbleState::new(line.clone()));
            }
            let name = self
                .game
                .world
                .entities
                .get(gid)
                .and_then(|e| e.name.clone())
                .unwrap_or_else(|| "Pet".to_string());
            self.windows.chat_window.add_message(
                format!("{name} : {line}"),
                [1.0, 1.0, 1.0, 1.0],
                ragnarok_ui_component::game::chat_window::ChatChannel::Public,
            );
        }
    }
}
