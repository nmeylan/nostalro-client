pub(crate) mod cart;
mod cursor;
mod effects;
pub(crate) mod falcon;
mod floor_item;

pub(crate) use cart::CartVisual;
pub(crate) use falcon::FalconVisual;

use crate::App;
use models::enums::weapon::WeaponType;
use ragnarok_formats::gr2::{Gr2Container, Gr2File};
use ragnarok_formats::spr::SpriteData;
use ragnarok_game::data_table::accessory_table::AccessoryTable;
use ragnarok_game::entity::EntityType;
use ragnarok_game::gr2_model::{self, AnimationClip, Gr2Action, Gr2ModelInstance, SkeletonPose};
use ragnarok_game::sprite_loader;
use ragnarok_game::sprite_path::{entity_sprite_base_path, weapon_view_id_to_type};
use ragnarok_renderer::gr2_model::Gr2ModelRenderer;
use ragnarok_renderer::{
    EntitySprite, SpriteTextures, build_entity_sprite, upload_sprite_textures,
};
use std::rc::Rc;

fn parse_gr2_file(bytes: &[u8], path: &str) -> Option<Gr2File> {
    let container = Gr2Container::parse(bytes)
        .map_err(|e| tracing::warn!("gr2 container parse failed for {path}: {e:?}"))
        .ok()?;
    Gr2File::parse(&container)
        .map_err(|e| tracing::warn!("gr2 extract failed for {path}: {e:?}"))
        .ok()
}

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

    pub(crate) fn reload_player_sprite(&mut self, gid: u32) {
        let entity = match self.game.world.entities.get(gid) {
            Some(e) => e,
            None => return,
        };
        let job = ragnarok_game::sprite_path::visual_job(entity.job, entity.effect_state);
        let sex = entity.sex;
        let head = entity.head;
        let weapon_type = entity.weapon;
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
            head_top,
            head_mid,
            head_bottom,
            shield_id,
            orc_face,
            is_gm,
        )?;
        Some(Rc::new(build_entity_sprite(
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
        )))
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
                job, sex, head, hair_color, 0, None, 0, 0, 0, 0, false, false,
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
        let sprite = Rc::new(build_entity_sprite(
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
        ));
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
                        if job != 45 {
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
    /// animated 3D model instead of a sprite: draw resources go into
    /// `Renderer::gr2_models`, animation state into `game.gr2_models`.
    pub(crate) fn load_gr2_entity_model(&mut self, gid: u32, model_name: &str) {
        let (grf, renderer) = match (&self.grf, &mut self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };
        let path = gr2_model::gr2_model_path(model_name);
        let bytes = match grf.read_file(&path) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("cannot read gr2 model {path}: {e}");
                return;
            }
        };
        let Some(file) = parse_gr2_file(&bytes, &path) else {
            return;
        };
        let Some(pose) = SkeletonPose::from_model(&file, 0) else {
            tracing::warn!("gr2 model {path} has no skeleton");
            return;
        };
        let bone_type = gr2_model::bone_type_from_name(model_name);
        let clips: [Option<AnimationClip>; 5] = std::array::from_fn(|i| match Gr2Action::ALL[i] {
            Gr2Action::Stand => AnimationClip::from_gr2(&file, 0),
            action => {
                let anim_path = gr2_model::animation_file_path(bone_type?, action)?;
                let bytes = grf.read_file(&anim_path).ok()?;
                let anim_file = parse_gr2_file(&bytes, &anim_path)?;
                AnimationClip::from_gr2(&anim_file, 0)
            }
        });
        let Some(model_renderer) = Gr2ModelRenderer::from_gr2(
            &file,
            0,
            &renderer.device.device,
            &renderer.device.queue,
            &renderer.global_uniforms,
            &renderer.texture_cache,
            renderer.device.surface_format,
        ) else {
            tracing::warn!("gr2 model {path} produced no renderable geometry");
            return;
        };
        renderer.gr2_models.insert(gid, model_renderer);
        self.game
            .sprite_caches
            .gr2_models
            .insert(gid, Gr2ModelInstance::new(pose, clips));
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
            {
                self.game.sprite_caches.failed_sprite_loads.insert(*gid);
            }
        }
    }
}
