use crate::Camera;
use ragnarok_formats::gat::GatFile;
use ragnarok_game::map_coordinates::MapCoordinates;

fn depth_gradient(
    camera: &Camera,
    world: [f32; 3],
    sx0: f32,
    sy0: f32,
    z0: f32,
    screen_w: f32,
    screen_h: f32,
) -> [f32; 2] {
    let [wx, wy, wz] = world;
    let right = camera.right_vector();
    let (Some((sx_up, sy_up, z_up, _)), Some((sx_r, sy_r, z_r, _))) = (
        camera.world_to_screen_with_depth(wx, wy - 1.0, wz, screen_w, screen_h),
        camera.world_to_screen_with_depth(
            wx + right.x,
            wy + right.y,
            wz + right.z,
            screen_w,
            screen_h,
        ),
    ) else {
        return [0.0, 0.0];
    };

    let (a, b, e) = (sx_up - sx0, sy_up - sy0, z_up - z0);
    let (c, d, f) = (sx_r - sx0, sy_r - sy0, z_r - z0);
    let det = a * d - b * c;
    if det.abs() < 1e-9 {
        return [0.0, 0.0];
    }
    [(e * d - b * f) / det, (a * f - e * c) / det]
}

fn ground_depth_gradient(
    camera: &Camera,
    world: [f32; 3],
    sx0: f32,
    sy0: f32,
    z0: f32,
    screen_w: f32,
    screen_h: f32,
) -> [f32; 2] {
    let [wx, wy, wz] = world;
    let right = camera.right_vector();
    let fwd = glam::Vec3::new(right.z, 0.0, -right.x);
    let (Some((sx_f, sy_f, z_f, _)), Some((sx_r, sy_r, z_r, _))) = (
        camera.world_to_screen_with_depth(wx + fwd.x, wy, wz + fwd.z, screen_w, screen_h),
        camera.world_to_screen_with_depth(
            wx + right.x,
            wy + right.y,
            wz + right.z,
            screen_w,
            screen_h,
        ),
    ) else {
        return [0.0, 0.0];
    };

    let (a, b, e) = (sx_f - sx0, sy_f - sy0, z_f - z0);
    let (c, d, f) = (sx_r - sx0, sy_r - sy0, z_r - z0);
    let det = a * d - b * c;
    if det.abs() < 1e-9 {
        return [0.0, 0.0];
    }
    [(e * d - b * f) / det, (a * f - e * c) / det]
}

pub fn project_entity_screen(
    pos: (f32, f32),
    gat: Option<&GatFile>,
    coords: &MapCoordinates,
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
) -> Option<([f32; 2], f32, u8, f32, [f32; 2])> {
    let (cell_x, cell_y) = pos;
    let (wx, _, wz) = coords.cell_to_world(cell_x + 0.5, cell_y + 0.5);
    let wy = gat.map_or(0.0, |gat| gat.get_height(cell_x + 0.5, cell_y + 0.5));

    let (sx, sy, ndc_z_raw, clip_w) =
        camera.world_to_screen_with_depth(wx, wy, wz, screen_w, screen_h)?;
    let ndc_z =
        ndc_z_raw - camera.near * crate::effect_sprite::ENTITY_DEPTH_BIAS_UNITS / (clip_w * clip_w);

    let grad = depth_gradient(camera, [wx, wy, wz], sx, sy, ndc_z_raw, screen_w, screen_h);

    let camera_dir = camera.direction_index();
    let ppu = camera.perspective_scale(wx, wy, wz, screen_h);
    let sprite_scale = ppu * coords.zoom() / 75.0;

    Some(([sx, sy], ndc_z, camera_dir, sprite_scale, grad))
}

pub fn project_world_screen(
    world: [f32; 3],
    coords: &MapCoordinates,
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
) -> Option<([f32; 2], f32, u8, f32, [f32; 2])> {
    let [wx, wy, wz] = world;
    let (sx, sy, ndc_z_raw, clip_w) =
        camera.world_to_screen_with_depth(wx, wy, wz, screen_w, screen_h)?;
    let ndc_z =
        ndc_z_raw - camera.near * crate::effect_sprite::ENTITY_DEPTH_BIAS_UNITS / (clip_w * clip_w);

    let grad = depth_gradient(camera, [wx, wy, wz], sx, sy, ndc_z_raw, screen_w, screen_h);

    let camera_dir = camera.direction_index();
    let ppu = camera.perspective_scale(wx, wy, wz, screen_h);
    let sprite_scale = ppu * coords.zoom() / 75.0;

    Some(([sx, sy], ndc_z, camera_dir, sprite_scale, grad))
}

pub fn entity_ground_gradient(
    pos: (f32, f32),
    gat: Option<&GatFile>,
    coords: &MapCoordinates,
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
) -> [f32; 2] {
    let world = cell_world_pos(pos, gat, coords);
    let [wx, wy, wz] = world;
    let Some((sx, sy, ndc_z_raw, _)) =
        camera.world_to_screen_with_depth(wx, wy, wz, screen_w, screen_h)
    else {
        return [0.0, 0.0];
    };
    ground_depth_gradient(camera, world, sx, sy, ndc_z_raw, screen_w, screen_h)
}

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

    #[test]
    fn depth_gradient_matches_direct_projection_under_yaw() {
        let (screen_w, screen_h) = (800.0, 600.0);
        let coords = fixture_coords();
        let mut camera = Camera::default();
        let (wx, _, wz) = coords.cell_to_world(50.5, 50.5);
        camera.target = glam::Vec3::new(wx, 0.0, wz);
        camera.yaw = 0.6; // rotate so the vertical line is not screen-vertical

        let ([sx0, sy0], _depth, _dir, _scale, grad) =
            project_entity_screen((50.0, 50.0), None, &coords, &camera, screen_w, screen_h)
                .expect("anchor visible");
        let (_, _, z0, _) = camera
            .world_to_screen_with_depth(wx, 0.0, wz, screen_w, screen_h)
            .unwrap();

        let (px, py, pz, _) = camera
            .world_to_screen_with_depth(wx, -30.0, wz, screen_w, screen_h)
            .unwrap();
        let reconstructed = z0 + grad[0] * (px - sx0) + grad[1] * (py - sy0);
        assert!(
            (reconstructed - pz).abs() < 1e-4,
            "reconstructed {reconstructed} vs direct {pz}"
        );
    }

    #[test]
    fn ground_gradient_matches_projection_of_point_along_the_ground() {
        let (screen_w, screen_h) = (800.0, 600.0);
        let coords = fixture_coords();
        let mut camera = Camera::default();
        let (wx, _, wz) = coords.cell_to_world(50.5, 50.5);
        camera.target = glam::Vec3::new(wx, 0.0, wz);
        camera.yaw = 0.6;

        let grad = entity_ground_gradient((50.0, 50.0), None, &coords, &camera, screen_w, screen_h);
        let (sx0, sy0, z0, _) = camera
            .world_to_screen_with_depth(wx, 0.0, wz, screen_w, screen_h)
            .unwrap();

        let right = camera.right_vector();
        let fwd = glam::Vec3::new(right.z, 0.0, -right.x);
        let (px, py, pz, _) = camera
            .world_to_screen_with_depth(wx + fwd.x * 5.0, 0.0, wz + fwd.z * 5.0, screen_w, screen_h)
            .unwrap();
        let reconstructed = z0 + grad[0] * (px - sx0) + grad[1] * (py - sy0);
        assert!(
            (reconstructed - pz).abs() < 1e-4,
            "reconstructed {reconstructed} vs direct {pz}"
        );
    }
}
