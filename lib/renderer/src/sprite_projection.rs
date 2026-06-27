use crate::Camera;
use ragnarok_formats::gat::GatFile;
use ragnarok_game::map_coordinates::MapCoordinates;

/// Screen-space depth gradient `[∂z/∂sx, ∂z/∂sy]` for the camera-facing vertical
/// plane through the anchor. NDC depth is affine in screen coordinates for any
/// flat plane, so a billboard standing at the anchor gets exact per-pixel depth
/// from `z0 + α·Δsx + β·Δsy`. Solving the 2×2 system from a vertical and a
/// horizontal in-plane sample recovers `[α, β]` correctly at any camera yaw —
/// a single vertical-axis slope would conflate the two whenever the camera is
/// rotated, drifting the sprite's upper body behind geometry it stands in front
/// of. Falls back to `[0.0, 0.0]` when a sample is off-screen or degenerate.
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

    // [a b; c d] [α; β] = [e; f]
    let (a, b, e) = (sx_up - sx0, sy_up - sy0, z_up - z0);
    let (c, d, f) = (sx_r - sx0, sy_r - sy0, z_r - z0);
    let det = a * d - b * c;
    if det.abs() < 1e-9 {
        return [0.0, 0.0];
    }
    [(e * d - b * f) / det, (a * f - e * c) / det]
}

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
) -> Option<([f32; 2], f32, u8, f32, [f32; 2])> {
    let (cell_x, cell_y) = pos;
    let (wx, _, wz) = coords.cell_to_world(cell_x + 0.5, cell_y + 0.5);
    let wy = gat.map_or(0.0, |gat| gat.get_height(cell_x + 0.5, cell_y + 0.5));

    let (sx, sy, ndc_z_raw, clip_w) =
        camera.world_to_screen_with_depth(wx, wy, wz, screen_w, screen_h)?;
    let ndc_z = ndc_z_raw
        - camera.near * crate::effect_sprite::ENTITY_DEPTH_BIAS_UNITS / (clip_w * clip_w);

    let grad = depth_gradient(camera, [wx, wy, wz], sx, sy, ndc_z_raw, screen_w, screen_h);

    let camera_dir = camera.direction_index();
    let ppu = camera.perspective_scale(wx, wy, wz, screen_h);
    let sprite_scale = ppu * coords.zoom() / 75.0;

    Some(([sx, sy], ndc_z, camera_dir, sprite_scale, grad))
}

/// Project an arbitrary world-space point (not snapped to ground height) to the
/// same parameters [`project_entity_screen`] returns. Used by hovering sprites —
/// the falcon companion flies above the terrain, so its height is intrinsic to
/// its world position rather than derived from the GAT cell.
///
/// Returns `None` when the point is behind/off-screen the camera.
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
    let ndc_z = ndc_z_raw
        - camera.near * crate::effect_sprite::ENTITY_DEPTH_BIAS_UNITS / (clip_w * clip_w);

    let grad = depth_gradient(camera, [wx, wy, wz], sx, sy, ndc_z_raw, screen_w, screen_h);

    let camera_dir = camera.direction_index();
    let ppu = camera.perspective_scale(wx, wy, wz, screen_h);
    let sprite_scale = ppu * coords.zoom() / 75.0;

    Some(([sx, sy], ndc_z, camera_dir, sprite_scale, grad))
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

    /// The two-axis depth gradient must reproduce the exact NDC depth of the
    /// billboard's vertical plane at any camera yaw. A point a few units up the
    /// vertical line through the cell, reconstructed from the anchor depth plus
    /// `grad·(Δscreen)`, must match its direct projection — a single screen-Y
    /// slope (the previous approach) drifts here because the world-vertical line
    /// projects diagonally under yaw.
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
        // The unbiased anchor depth is the gradient's reference (project returns
        // the camera-biased depth, so recompute the raw value here).
        let (_, _, z0, _) = camera
            .world_to_screen_with_depth(wx, 0.0, wz, screen_w, screen_h)
            .unwrap();

        // Probe a point 30 units up the vertical line through the cell.
        let (px, py, pz, _) = camera
            .world_to_screen_with_depth(wx, -30.0, wz, screen_w, screen_h)
            .unwrap();
        let reconstructed = z0 + grad[0] * (px - sx0) + grad[1] * (py - sy0);
        assert!(
            (reconstructed - pz).abs() < 1e-4,
            "reconstructed {reconstructed} vs direct {pz}"
        );
    }
}
