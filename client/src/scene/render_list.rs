use crate::{App, input};
use ragnarok_game::cursor::{RenderEntry, RenderEntryKind};
use ragnarok_renderer::Renderer;

impl App {
    pub(crate) fn screen_dims(
        &self,
    ) -> Option<(
        &Renderer,
        &ragnarok_formats::map_coordinates::MapCoordinates,
        f32,
        f32,
    )> {
        let renderer = self.renderer.as_ref()?;
        let coords = self.game.session.map_coords.as_ref()?;
        let screen_w = renderer.device.surface_config.width as f32 / renderer.dpi_scale;
        let screen_h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
        Some((renderer, coords, screen_w, screen_h))
    }

    pub(crate) fn push_projected(
        list: &mut Vec<RenderEntry>,
        kind: RenderEntryKind,
        id: u32,
        projected: Option<([f32; 2], f32, u8, f32, [f32; 2])>,
        flat_depth_gradient: Option<[f32; 2]>,
        camera_dir: Option<u8>,
        bounds: impl FnOnce([f32; 2], f32, u8, f32) -> ([f32; 4], f32),
    ) {
        let Some((screen_anchor, depth, projected_dir, sprite_scale, depth_gradient)) = projected
        else {
            return;
        };
        let (pick_bounds, head_offset) = bounds(screen_anchor, depth, projected_dir, sprite_scale);
        list.push(RenderEntry {
            kind,
            id,
            screen_anchor,
            depth,
            depth_gradient,
            flat_depth_gradient: flat_depth_gradient.unwrap_or(depth_gradient),
            camera_dir: camera_dir.unwrap_or(projected_dir),
            sprite_scale,
            pick_bounds,
            head_offset,
        });
    }

    pub(crate) fn compute_render_list(&self) -> Vec<RenderEntry> {
        ragnarok_profiling::profile_function!();
        let mut render_list = Vec::new();
        if let Some((renderer, coords, screen_w, screen_h)) = self.screen_dims() {
            for entity in self.game.world.entities.iter() {
                let projected = input::entity_screen_params(
                    entity.movement.position(),
                    self.game.session.gat.as_ref(),
                    coords,
                    &renderer.camera,
                    screen_w,
                    screen_h,
                );
                let flat_depth_gradient = input::entity_ground_gradient(
                    entity.movement.position(),
                    self.game.session.gat.as_ref(),
                    coords,
                    &renderer.camera,
                    screen_w,
                    screen_h,
                );
                Self::push_projected(
                    &mut render_list,
                    RenderEntryKind::Entity,
                    entity.id,
                    projected,
                    Some(flat_depth_gradient),
                    None,
                    |screen_anchor, depth, camera_dir, sprite_scale| match self
                        .game
                        .sprite_caches
                        .sprites
                        .get(&entity.id)
                    {
                        Some(sprite) => (
                            sprite.compute_pick_bounds(
                                &entity.animation,
                                Some(camera_dir),
                                entity.head_dir,
                                screen_anchor,
                                depth,
                                sprite_scale,
                            ),
                            sprite.compute_head_offset(
                                &entity.animation,
                                Some(camera_dir),
                                entity.head_dir,
                                screen_anchor,
                                depth,
                                sprite_scale,
                            ),
                        ),
                        None => {
                            let half = 50.0;
                            (
                                [
                                    screen_anchor[0] - half,
                                    screen_anchor[1] - 100.0,
                                    screen_anchor[0] + half,
                                    screen_anchor[1],
                                ],
                                100.0,
                            )
                        }
                    },
                );
            }
        }
        render_list.sort_by(|a, b| {
            b.depth
                .partial_cmp(&a.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        render_list
    }
}
