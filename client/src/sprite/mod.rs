pub(crate) mod cart;
mod cursor;
mod effects;
pub(crate) mod falcon;
mod floor_item;
pub(crate) mod gr2_loader;

pub(crate) use cart::CartVisual;
pub(crate) use falcon::FalconVisual;

use crate::App;
use models::enums::weapon::WeaponType;
use ragnarok_formats::spr::SpriteData;
use ragnarok_game::data_table::accessory_table::AccessoryTable;
use ragnarok_game::entity::EntityType;
use ragnarok_game::gr2_model::{self, Gr2Asset, Gr2ModelInstance};
use ragnarok_game::sprite_loader;
use ragnarok_game::sprite_path::{
    entity_sprite_base_path, is_undrawn_actor, weapon_view_id_to_type,
};
use ragnarok_profiling::profile_function;
use ragnarok_renderer::{
    EntitySprite, Gr2ModelAsset, Gr2ModelDraw, SpriteTextures, build_entity_sprite,
    upload_glyph_textures, upload_sprite_textures,
};
use std::rc::Rc;

impl App {
    pub(crate) fn upload_sprite(&self, data: &SpriteData) -> Option<SpriteTextures> {
        let renderer = self.renderer.as_ref()?;
        Some(upload_sprite_textures(
            &data.images,
            data.indexed_count,
            &renderer.device.device,
            &renderer.device.queue,
            &renderer.texture_cache.bind_group_layout,
        ))
    }

    pub(crate) fn upload_glyph_sprite(&self, data: &SpriteData) -> Option<SpriteTextures> {
        let renderer = self.renderer.as_ref()?;
        Some(upload_glyph_textures(
            &data.images,
            data.indexed_count,
            &renderer.device.device,
            &renderer.device.queue,
            &renderer.texture_cache.bind_group_layout,
        ))
    }

    pub(crate) fn reload_player_sprite(&mut self, gid: u32) {
        let entity = match self.game.world.entities.get(gid) {
            Some(e) => e,
            None => return,
        };
        let job = ragnarok_game::sprite_path::visual_job(entity.job, entity.effect_state);
        let sex = entity.sex;
        let head = entity.head;
        let weapon_type = entity.weapon;
        let weapon_look = entity.weapon_look;
        let shield = entity.shield;
        let head_top = entity.head_top;
        let head_mid = entity.head_mid;
        let head_bottom = entity.head_bottom;
        let hair_color = entity.hair_color;
        let cloth_color = entity.cloth_color;
        self.load_player_sprite(
            gid,
            job,
            sex,
            head,
            hair_color,
            cloth_color,
            weapon_type,
            weapon_look,
            head_top,
            head_mid,
            head_bottom,
            shield,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn load_player_sprite(
        &mut self,
        gid: u32,
        job: u16,
        sex: u8,
        head: u16,
        hair_color: u16,
        cloth_color: u16,
        weapon: Option<WeaponType>,
        weapon_look: u16,
        head_top: u16,
        head_mid: u16,
        head_bottom: u16,
        shield_id: u16,
    ) {
        let (orc_face, is_gm) = self
            .game
            .world
            .entities
            .get(gid)
            .map(|e| {
                (
                    ragnarok_game::sprite_path::is_orcish(e.effect_state),
                    e.is_gm,
                )
            })
            .unwrap_or((false, false));
        if let Some(sprite) = self.build_player_entity_sprite(
            job,
            sex,
            head,
            hair_color,
            cloth_color,
            weapon,
            weapon_look,
            head_top,
            head_mid,
            head_bottom,
            shield_id,
            orc_face,
            is_gm,
        ) {
            self.game.sprite_caches.sprites.insert(gid, sprite);
        } else {
            tracing::warn!(
                "load_player_sprite: failed to load sprite data for gid={gid} job={job}"
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_player_entity_sprite(
        &self,
        job: u16,
        sex: u8,
        head: u16,
        hair_color: u16,
        cloth_color: u16,
        weapon: Option<WeaponType>,
        weapon_look: u16,
        head_top: u16,
        head_mid: u16,
        head_bottom: u16,
        shield_id: u16,
        orc_face: bool,
        is_gm: bool,
    ) -> Option<Rc<EntitySprite>> {
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return None,
        };
        let empty_table = AccessoryTable::empty();
        let accessory_table = self
            .game
            .data_table
            .accessory
            .as_ref()
            .unwrap_or(&empty_table);
        let data = sprite_loader::load_player_sprite_data(
            grf,
            accessory_table,
            job,
            sex,
            head,
            hair_color,
            cloth_color,
            weapon,
            weapon_look,
            head_top,
            head_mid,
            head_bottom,
            shield_id,
            orc_face,
            is_gm,
        )?;
        Some(Rc::new(
            build_entity_sprite(
                &renderer.device.device,
                &renderer.device.queue,
                &renderer.texture_cache.bind_group_layout,
                data.body,
                data.head,
                data.weapon,
                data.weapon_trail,
                data.headgear_top,
                data.headgear_mid,
                data.headgear_bottom,
                data.shield,
                data.shadow,
            )
            .with_layer_order(data.layer_order),
        ))
    }

    /// Load the head/body sprite of every current guild member into a dedicated
    /// cache keyed by member GID, for the face icons in the guild roster. Kept
    /// separate from `game.sprites` so it never collides with on-screen entities.
    pub(crate) fn load_guild_member_sprites(&mut self) {
        self.game.sprite_caches.guild_head_sprites.clear();
        let members: Vec<(u32, u16, u8, u16, u16)> = match &self.game.guild {
            Some(g) => g
                .members
                .iter()
                .map(|m| {
                    let sex = self
                        .game
                        .world
                        .entities
                        .get(m.gid)
                        .or_else(|| {
                            self.game
                                .world
                                .entities
                                .get(self.game.world.entities.resolve_key(m.aid))
                        })
                        .map(|e| e.sex)
                        .unwrap_or_else(|| m.sex.max(0) as u8);
                    (
                        m.gid,
                        m.job.max(0) as u16,
                        sex,
                        m.head.max(0) as u16,
                        m.head_palette.max(0) as u16,
                    )
                })
                .collect(),
            None => return,
        };
        for (gid, job, sex, head, hair_color) in members {
            if let Some(sprite) = self.build_player_entity_sprite(
                job, sex, head, hair_color, 0, None, 0, 0, 0, 0, 0, false, false,
            ) {
                self.game
                    .sprite_caches
                    .guild_head_sprites
                    .insert(gid, sprite);
            }
        }
    }

    pub(crate) fn load_mercenary_sprite(&mut self, gid: u32, job: u16) {
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };
        let name_table = match &self.game.data_table.name {
            Some(t) => t,
            None => return,
        };
        let data = match sprite_loader::load_mercenary_sprite_data(grf, name_table, job) {
            Some(d) => d,
            None => {
                tracing::warn!("load_mercenary_sprite: no sprite data for gid={gid} job={job}");
                return;
            }
        };
        let sprite = Rc::new(
            build_entity_sprite(
                &renderer.device.device,
                &renderer.device.queue,
                &renderer.texture_cache.bind_group_layout,
                data.body,
                data.head,
                data.weapon,
                data.weapon_trail,
                data.headgear_top,
                data.headgear_mid,
                data.headgear_bottom,
                data.shield,
                data.shadow,
            )
            .with_layer_order(data.layer_order),
        );
        self.game.sprite_caches.sprites.insert(gid, sprite);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn load_entity_sprite(
        &mut self,
        gid: u32,
        entity_type: EntityType,
        job: u16,
        sex: u8,
        head: u16,
        weapon: u16,
        shield: u16,
        head_top: u16,
        head_mid: u16,
        head_bottom: u16,
        hair_color: u16,
        _direction: u8,
    ) {
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };

        match entity_type {
            EntityType::Player => {
                let weapon_type = weapon_view_id_to_type(weapon);
                self.load_player_sprite(
                    gid,
                    job,
                    sex,
                    head,
                    hair_color,
                    0,
                    weapon_type,
                    weapon,
                    head_top,
                    head_mid,
                    head_bottom,
                    shield,
                );
            }
            EntityType::Mercenary => {
                self.load_mercenary_sprite(gid, job);
            }
            EntityType::Npc | EntityType::Monster | EntityType::Homunculus => {
                let name_table = match &self.game.data_table.name {
                    Some(t) => t,
                    None => {
                        tracing::warn!("No name table for job {job}");
                        return;
                    }
                };
                let gr2_name = name_table
                    .get_name(job)
                    .filter(|n| gr2_model::is_gr2_name(n))
                    .map(|n| n.to_string());
                if let Some(name) = gr2_name {
                    self.load_gr2_entity_model(gid, &name);
                    return;
                }
                let cache_key = match entity_sprite_base_path(name_table, job) {
                    Some(p) => p,
                    None => {
                        if !is_undrawn_actor(job) {
                            tracing::warn!("No sprite path for id {job}");
                        }
                        return;
                    }
                };

                if let Some(cached) = self.game.sprite_caches.sprite_cache.get(&cache_key) {
                    self.game
                        .sprite_caches
                        .sprites
                        .insert(gid, Rc::clone(cached));
                    return;
                }

                let data = match sprite_loader::load_entity_sprite_data(grf, name_table, job) {
                    Some(d) => d,
                    None => return,
                };
                let sprite = Rc::new(build_entity_sprite(
                    &renderer.device.device,
                    &renderer.device.queue,
                    &renderer.texture_cache.bind_group_layout,
                    data.body,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    data.shadow,
                ));
                self.game
                    .sprite_caches
                    .sprite_cache
                    .insert(cache_key, Rc::clone(&sprite));
                self.game.sprite_caches.sprites.insert(gid, sprite);
            }
        }
    }

    /// Swaps a pet's body sprite to its accessory ACT variant. The accessory is an
    /// ACT-only swap that reuses the mob's base SPR (the accessory frames live in
    /// that SPR). Falls back to the plain mob sprite when the accessory is unset or
    /// its ACT/SPR is missing.
    pub(crate) fn load_pet_sprite(&mut self, gid: u32, job: u16, accessory_view: u16) {
        let load_plain = |app: &mut Self| {
            app.load_entity_sprite(gid, EntityType::Monster, job, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        };
        let Some(act_path) = ragnarok_game::pet_tables::pet_accessory_act(accessory_view) else {
            load_plain(self);
            return;
        };
        if let Some(cached) = self.game.sprite_caches.sprite_cache.get(act_path) {
            let cached = Rc::clone(cached);
            self.game.sprite_caches.sprites.insert(gid, cached);
            return;
        }
        let base_path = self
            .game
            .data_table
            .name
            .as_ref()
            .and_then(|nt| ragnarok_game::sprite_path::entity_sprite_base_path(nt, job));
        let Some(base_path) = base_path else {
            load_plain(self);
            return;
        };
        let spr_path = format!("{base_path}.spr");
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };
        let Some(data) = sprite_loader::load_sprite_data(grf, &spr_path, act_path) else {
            load_plain(self);
            return;
        };
        let sprite = Rc::new(build_entity_sprite(
            &renderer.device.device,
            &renderer.device.queue,
            &renderer.texture_cache.bind_group_layout,
            data,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ));
        self.game
            .sprite_caches
            .sprite_cache
            .insert(act_path.to_string(), Rc::clone(&sprite));
        self.game.sprite_caches.sprites.insert(gid, sprite);
    }

    /// Load a `.gr2` name-table entity (emperium, guardian, guild flag…) as an
    /// animated 3D model instead of a sprite. Geometry, textures, skeleton and
    /// clips are cached per model file; only the transform and bone palette are
    /// per entity.
    pub(crate) fn load_gr2_entity_model(&mut self, gid: u32, model_name: &str) {
        let path = gr2_model::gr2_model_path(model_name);
        if self.game.sprite_caches.gr2_assets.contains_key(&path) {
            self.spawn_gr2_instance(gid, &path);
            return;
        }
        if let Some(loader) = &mut self.gr2_loader {
            loader.request(gid, model_name, &path);
        }
    }

    /// Instance every entity whose model finished reading on the loader thread.
    pub(crate) fn poll_gr2_loads(&mut self) {
        let Some(loader) = &mut self.gr2_loader else {
            return;
        };
        for (path, loaded, gids) in loader.take_ready() {
            let uploaded = match loaded {
                Some(loaded) => self.insert_gr2_asset(path.clone(), loaded),
                None => false,
            };
            for gid in gids {
                if !uploaded {
                    self.game.sprite_caches.failed_sprite_loads.insert(gid);
                    continue;
                }
                if self.game.world.entities.get(gid).is_some() {
                    self.spawn_gr2_instance(gid, &path);
                }
            }
        }
    }

    /// Upload a decoded model and cache it under `path`. False when it produced
    /// nothing drawable.
    fn insert_gr2_asset(&mut self, path: String, loaded: gr2_loader::Gr2LoadedAsset) -> bool {
        let Some(renderer) = &mut self.renderer else {
            return false;
        };
        let Some(asset) = Gr2ModelAsset::from_parts(
            &renderer.device.device,
            &renderer.device.queue,
            &renderer.texture_cache,
            loaded.geometry,
            loaded.textures,
            loaded.emblem_texture_index,
        ) else {
            tracing::warn!("gr2 model {path} produced no renderable geometry");
            return false;
        };
        renderer.gr2_assets.insert(path.clone(), Rc::new(asset));
        self.game.sprite_caches.gr2_assets.insert(
            path,
            Rc::new(Gr2Asset {
                pose: loaded.pose,
                clips: loaded.clips,
            }),
        );
        true
    }

    /// Give `gid` its own transform and bone palette over the cached model.
    fn spawn_gr2_instance(&mut self, gid: u32, path: &str) {
        let Some(cpu_asset) = self.game.sprite_caches.gr2_assets.get(path).cloned() else {
            return;
        };
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let Some(asset) = renderer.gr2_assets.get(path).cloned() else {
            return;
        };
        let draw = Gr2ModelDraw::new(&renderer.device.device, &renderer.gr2_pipeline, asset);
        renderer.gr2_models.insert(gid, draw);
        self.game
            .sprite_caches
            .gr2_models
            .insert(gid, Gr2ModelInstance::new(cpu_asset));
        self.apply_guild_emblem_to_model(gid);
    }

    pub(crate) fn remove_gr2_model(&mut self, gid: u32) {
        self.game.sprite_caches.gr2_models.remove(&gid);
        if let Some(renderer) = &mut self.renderer {
            renderer.gr2_models.remove(&gid);
        }
    }

    pub(crate) fn load_missing_entity_sprites(&mut self) {
        let missing: Vec<_> = self
            .game
            .world
            .entities
            .iter()
            .filter(|e| {
                self.game.world.entities.player_id() != Some(e.id)
                    && !self.game.sprite_caches.sprites.contains_key(&e.id)
                    && !self.game.sprite_caches.gr2_models.contains_key(&e.id)
                    && !self.game.sprite_caches.failed_sprite_loads.contains(&e.id)
                    && !self
                        .gr2_loader
                        .as_ref()
                        .is_some_and(|loader| loader.is_waiting(e.id))
            })
            .map(|e| {
                (
                    e.id,
                    e.entity_type,
                    ragnarok_game::sprite_path::visual_job(e.job, e.effect_state),
                    e.sex,
                    e.head,
                    e.head_top,
                    e.head_mid,
                    e.head_bottom,
                    e.shield,
                    e.hair_color,
                    e.direction,
                )
            })
            .collect();
        for (
            gid,
            entity_type,
            sprite_job,
            sex,
            head,
            head_top,
            head_mid,
            head_bottom,
            shield,
            hair_color,
            direction,
        ) in &missing
        {
            tracing::info!(
                "Retrying sprite load for entity gid={gid} job={sprite_job} type={entity_type:?}"
            );
            self.load_entity_sprite(
                *gid,
                *entity_type,
                *sprite_job,
                *sex,
                *head,
                0,
                *shield,
                *head_top,
                *head_mid,
                *head_bottom,
                *hair_color,
                *direction,
            );
            if !self.game.sprite_caches.sprites.contains_key(gid)
                && !self.game.sprite_caches.gr2_models.contains_key(gid)
                && !self
                    .gr2_loader
                    .as_ref()
                    .is_some_and(|loader| loader.is_waiting(*gid))
            {
                self.game.sprite_caches.failed_sprite_loads.insert(*gid);
            }
        }
    }
}
