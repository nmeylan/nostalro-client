mod cursor;
mod effects;

use crate::App;
use models::enums::weapon::WeaponType;
use ragnarok_game::entity::EntityType;
use ragnarok_game::sprite_loader;
use ragnarok_game::sprite_path::{entity_sprite_base_path, weapon_view_id_to_type};
use ragnarok_renderer::build_entity_sprite;
use std::rc::Rc;
use ragnarok_game::data_table::accessory_table::AccessoryTable;

impl App {
    pub(crate) fn reload_player_sprite(&mut self, gid: u32) {
        let entity = match self.game.entities.get(gid) {
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
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };
        let empty_table = AccessoryTable::empty();
        let accessory_table = self
            .game
            .data_table
            .accessory
            .as_ref()
            .unwrap_or(&empty_table);
        tracing::debug!("load_player_sprite: gid={gid} job={job} sex={sex}");
        let data = match sprite_loader::load_player_sprite_data(
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
        ) {
            Some(d) => d,
            None => {
                tracing::warn!("load_player_sprite: failed to load sprite data for gid={gid} job={job}");
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
        self.game.sprites.insert(gid, sprite);
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
            EntityType::Npc | EntityType::Monster => {
                let name_table = match &self.game.data_table.name {
                    Some(t) => t,
                    None => {
                        tracing::warn!("No name table for job {job}");
                        return;
                    }
                };
                let cache_key = match entity_sprite_base_path(name_table, job) {
                    Some(p) => p,
                    None => {
                        tracing::warn!("No sprite path for job {job}");
                        return;
                    }
                };

                if let Some(cached) = self.game.sprite_cache.get(&cache_key) {
                    self.game.sprites.insert(gid, Rc::clone(cached));
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
                self.game.sprite_cache.insert(cache_key, Rc::clone(&sprite));
                self.game.sprites.insert(gid, sprite);
            }
        }
    }

    pub(crate) fn load_missing_entity_sprites(&mut self) {
        let missing: Vec<_> = self
            .game
            .entities
            .iter()
            .filter(|e| {
                self.game.entities.player_id() != Some(e.id)
                    && !self.game.sprites.contains_key(&e.id)
                    && !self.game.failed_sprite_loads.contains(&e.id)
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
            if !self.game.sprites.contains_key(gid) {
                self.game.failed_sprite_loads.insert(*gid);
            }
        }
    }
}
