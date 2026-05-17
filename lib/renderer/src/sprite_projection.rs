use crate::Camera;
use ragnarok_formats::gat::GatFile;
use ragnarok_game::map_coordinates::MapCoordinates;

/// Project a world-space entity at cell `pos` to the parameters
/// `EntitySprite::build_batches()` consumes: screen anchor, NDC depth,
/// camera direction, sprite scale, per-pixel depth gradient.
///
/// Returns `None` when the world point is behind/off-screen the camera.
pub fn project_entity_screen(
    pos: (f32, f32),
    gat: Option<&GatFile>,
    coords: &MapCoordinates,
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
) -> Option<([f32; 2], f32, u8, f32, f32)> {
    let (cell_x, cell_y) = pos;
    let (wx, _, wz) = coords.cell_to_world(cell_x + 0.5, cell_y + 0.5);
    let wy = gat.map_or(0.0, |gat| gat.get_height(cell_x + 0.5, cell_y + 0.5));

    let (sx, sy, ndc_z_raw, clip_w) =
        camera.world_to_screen_with_depth(wx, wy, wz, screen_w, screen_h)?;
    let ndc_z = ndc_z_raw - camera.near * 4.0 / (clip_w * clip_w);

    let depth_gradient = camera
        .world_to_screen_with_depth(wx, wy - 1.0, wz, screen_w, screen_h)
        .map(|(_, sy_above, ndc_z_above, _)| {
            let dy = sy_above - sy;
            if dy.abs() > 1e-6 {
                (ndc_z_above - ndc_z_raw) / dy
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);

    let camera_dir = camera.direction_index();
    let ppu = camera.perspective_scale(wx, wy, wz, screen_h);
    let sprite_scale = ppu * coords.zoom() / 75.0;

    Some(([sx, sy], ndc_z, camera_dir, sprite_scale, depth_gradient))
}

/// World position (with GAT height) of the center of `cell`. Mirrors the
/// position `project_entity_screen` uses internally so effect anchors and
/// sprite anchors stay in sync.
pub fn cell_world_pos(
    cell: (f32, f32),
    gat: Option<&GatFile>,
    coords: &MapCoordinates,
) -> [f32; 3] {
    let (cell_x, cell_y) = cell;
    let (wx, _, wz) = coords.cell_to_world(cell_x + 0.5, cell_y + 0.5);
    let wy = gat.map_or(0.0, |gat| gat.get_height(cell_x + 0.5, cell_y + 0.5));
    [wx, wy, wz]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_coords() -> MapCoordinates {
        MapCoordinates::new(10.0, 200, 200, 100, 100)
    }

    #[test]
    fn projects_target_cell_near_screen_center() {
        let coords = fixture_coords();
        let mut camera = Camera::default();
        let (wx, _, wz) = coords.cell_to_world(50.5, 50.5);
        camera.target = glam::Vec3::new(wx, 0.0, wz);

        let result = project_entity_screen((50.0, 50.0), None, &coords, &camera, 800.0, 600.0);
        let (anchor, _depth, _dir, scale, _grad) =
            result.expect("character should be visible at camera target");
        assert!((anchor[0] - 400.0).abs() < 30.0, "anchor.x = {}", anchor[0]);
        assert!(scale > 0.0, "scale should be positive");
    }

    #[test]
    fn cell_world_pos_returns_zero_height_without_gat() {
        let coords = fixture_coords();
        let pos = cell_world_pos((10.0, 10.0), None, &coords);
        assert_eq!(pos[1], 0.0);
    }
}
