use crate::App;
use ragnarok_formats::act::{MotionType, SpriteAnimationState};
use ragnarok_game::entity::EntityState;
use ragnarok_game::sprite_path::cart_sprite_path;
use ragnarok_renderer::{EntitySprite, build_entity_sprite};
use ragnarok_game::sprite_loader;
use std::rc::Rc;

/// Cart sprite action indices, matching the original game: action 0 is the
/// resting pose, action 1 the rolling/walk pose used while the owner moves.
const CART_ACTION_IDLE: usize = 0;
const CART_ACTION_MOVE: usize = 1;

/// How far behind the owner (in cells) the cart trails, along the opposite of
/// the owner's facing — mirrors the original client's ride-distance offset.
pub const CART_TRAIL_DISTANCE: f32 = 0.6;

/// The trailing pushcart that follows a merchant. Holds its own sprite and
/// animation state; the animation is driven each frame from the owner's
/// movement/facing so the cart rolls in lockstep with the player.
pub struct CartVisual {
    pub sprite: Rc<EntitySprite>,
    pub animation: SpriteAnimationState,
    pub design: u8,
}

/// Unit offset (cell-space) for each of the 8 facings, matching
/// `direction_from_delta`. Diagonals are normalised so the trail distance is
/// uniform in every direction.
pub fn direction_offset(dir: u8) -> (f32, f32) {
    const D: f32 = std::f32::consts::FRAC_1_SQRT_2;
    match dir % 8 {
        0 => (0.0, 1.0),   // S
        1 => (-D, D),      // SW
        2 => (-1.0, 0.0),  // W
        3 => (-D, -D),     // NW
        4 => (0.0, -1.0),  // N
        5 => (D, -D),      // NE
        6 => (1.0, 0.0),   // E
        _ => (D, D),       // SE
    }
}

impl App {
    /// Loads (or replaces) the trailing cart visual for `owner_gid` using the
    /// given design index. A failed sprite load is non-fatal — the cart simply
    /// renders nothing until the resource is available.
    pub(crate) fn spawn_cart_visual(&mut self, owner_gid: u32, design: u8) {
        if self
            .game
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
            .entities
            .get(owner_gid)
            .map(|e| e.direction)
            .unwrap_or(0);
        self.game.carts.insert(
            owner_gid,
            CartVisual {
                sprite,
                animation: SpriteAnimationState::new(direction),
                design,
            },
        );
    }

    pub(crate) fn despawn_cart_visual(&mut self, owner_gid: u32) {
        self.game.carts.remove(&owner_gid);
    }

    /// Loads the cart sprite for every selectable design into the preview cache
    /// so the change-cart picker can show each model. Already-cached designs are
    /// skipped; a missing resource is non-fatal (that row falls back to text).
    pub(crate) fn preload_cart_previews(&mut self, designs: &[u8]) {
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };
        for &design in designs {
            if self.game.cart_preview_sprites.contains_key(&design) {
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
                None, None, None, None, None, None, None, None,
            ));
            self.game.cart_preview_sprites.insert(design, sprite);
        }
    }

    /// Drives every cart animation from its owner's facing and move-state.
    pub(crate) fn update_cart_animations(&mut self, delta: f32) {
        let camera_dir = self
            .renderer
            .as_ref()
            .map(|r| r.camera.direction_index())
            .unwrap_or(0);
        let owners: Vec<u32> = self.game.carts.keys().copied().collect();
        for gid in owners {
            let Some(entity) = self.game.entities.get(gid) else {
                self.game.carts.remove(&gid);
                continue;
            };
            // The cart only rolls while its owner walks; standing/sitting freezes
            // it on a static frame rather than looping the idle pose.
            let (action, motion) = if entity.state == EntityState::Moving {
                (CART_ACTION_MOVE, MotionType::Loop)
            } else {
                (CART_ACTION_IDLE, MotionType::Static)
            };
            let direction = entity.direction;
            if let Some(cart) = self.game.carts.get_mut(&gid) {
                cart.animation
                    .set_action_clamped(action, motion, &cart.sprite.body_act);
                cart.animation.set_direction(direction);
                cart.animation
                    .update(delta, &cart.sprite.body_act, camera_dir);
            }
        }
    }
}
