use crate::App;
use crate::input;
use ragnarok_formats::act::{MotionType, SpriteAnimationState};
use ragnarok_game::cursor::{RenderEntry, RenderEntryKind};
use ragnarok_game::entity::EntityState;
use ragnarok_game::sprite_loader;
use ragnarok_game::sprite_path::cart_sprite_path;
use ragnarok_renderer::{EntitySprite, build_entity_sprite};
use std::rc::Rc;

const CART_ACTION_IDLE: usize = 0;
const CART_ACTION_MOVE: usize = 1;

pub const CART_TRAIL_DISTANCE: f32 = 0.6;

pub struct CartVisual {
    pub sprite: Rc<EntitySprite>,
    pub animation: SpriteAnimationState,
    pub design: u8,
}

pub fn direction_offset(dir: u8) -> (f32, f32) {
    const D: f32 = std::f32::consts::FRAC_1_SQRT_2;
    match dir % 8 {
        0 => (0.0, 1.0),  // S
        1 => (-D, D),     // SW
        2 => (-1.0, 0.0), // W
        3 => (-D, -D),    // NW
        4 => (0.0, -1.0), // N
        5 => (D, -D),     // NE
        6 => (1.0, 0.0),  // E
        _ => (D, D),      // SE
    }
}

impl App {
    pub(crate) fn spawn_cart_visual(&mut self, owner_gid: u32, design: u8) {
        if self
            .game
            .sprite_caches
            .carts
            .get(&owner_gid)
            .is_some_and(|c| c.design == design)
        {
            return;
        }
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };
        let base = cart_sprite_path(design);
        let body = match sprite_loader::load_sprite_data(
            grf,
            &format!("{base}.spr"),
            &format!("{base}.act"),
        ) {
            Some(d) => d,
            None => {
                tracing::warn!("cart sprite not found: {base}");
                return;
            }
        };
        let sprite = Rc::new(build_entity_sprite(
            &renderer.device.device,
            &renderer.device.queue,
            &renderer.texture_cache.bind_group_layout,
            body,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ));
        let direction = self
            .game
            .world
            .entities
            .get(owner_gid)
            .map(|e| e.direction)
            .unwrap_or(0);
        self.game.sprite_caches.carts.insert(
            owner_gid,
            CartVisual {
                sprite,
                animation: SpriteAnimationState::new(direction),
                design,
            },
        );
    }

    pub(crate) fn despawn_cart_visual(&mut self, owner_gid: u32) {
        self.game.sprite_caches.carts.remove(&owner_gid);
    }

    pub(crate) fn preload_cart_previews(&mut self, designs: &[u8]) {
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };
        for &design in designs {
            if self
                .game
                .sprite_caches
                .cart_preview_sprites
                .contains_key(&design)
            {
                continue;
            }
            let base = cart_sprite_path(design);
            let Some(body) = sprite_loader::load_sprite_data(
                grf,
                &format!("{base}.spr"),
                &format!("{base}.act"),
            ) else {
                continue;
            };
            let sprite = Rc::new(build_entity_sprite(
                &renderer.device.device,
                &renderer.device.queue,
                &renderer.texture_cache.bind_group_layout,
                body,
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
                .cart_preview_sprites
                .insert(design, sprite);
        }
    }

    pub(crate) fn update_cart_animations(&mut self, delta: f32) {
        let camera_dir = self
            .renderer
            .as_ref()
            .map(|r| r.camera.direction_index())
            .unwrap_or(0);
        let owners: Vec<u32> = self.game.sprite_caches.carts.keys().copied().collect();
        for gid in owners {
            let Some(entity) = self.game.world.entities.get(gid) else {
                self.game.sprite_caches.carts.remove(&gid);
                continue;
            };
            let (action, motion) = if entity.state == EntityState::Moving {
                (CART_ACTION_MOVE, MotionType::Loop)
            } else {
                (CART_ACTION_IDLE, MotionType::Static)
            };
            let direction = entity.direction;
            if let Some(cart) = self.game.sprite_caches.carts.get_mut(&gid) {
                cart.animation
                    .set_action_clamped(action, motion, &cart.sprite.body_act);
                cart.animation.set_direction(direction);
                cart.animation
                    .update(delta, &cart.sprite.body_act, camera_dir);
            }
        }
    }
}

impl App {
    pub(crate) fn compute_cart_render_list(&self) -> Vec<RenderEntry> {
        let mut render_list = Vec::new();
        if let Some((renderer, coords, screen_w, screen_h)) = self.screen_dims() {
            for entity in self.game.world.entities.iter() {
                if entity.cart_type.is_none()
                    || !self.game.sprite_caches.carts.contains_key(&entity.id)
                {
                    continue;
                }
                let (px, py) = entity.movement.position();
                let (ox, oy) = crate::sprite::cart::direction_offset(entity.direction);
                let cart_pos = (
                    px - ox * crate::sprite::cart::CART_TRAIL_DISTANCE,
                    py - oy * crate::sprite::cart::CART_TRAIL_DISTANCE,
                );
                let projected = input::entity_screen_params(
                    cart_pos,
                    self.game.session.gat.as_ref(),
                    coords,
                    &renderer.camera,
                    screen_w,
                    screen_h,
                );
                Self::push_projected(
                    &mut render_list,
                    RenderEntryKind::Cart,
                    entity.id,
                    projected,
                    None,
                    None,
                    |_, _, _, _| ([0.0; 4], 0.0),
                );
            }
        }
        render_list
    }
}
