//! Projection of world-space entities and effects into the screen-space quads
//! the sprite pass draws.
//!
//! A sprite is a flat billboard, but it has to sort against the 3D world (ground,
//! water, models) using the same depth buffer. This module turns a map cell or a
//! world point into everything the sprite pass needs: a screen anchor, a depth
//! value at that anchor, the facing frame to pick, a scale factor, and a depth
//! gradient so the billboard can slope across its face instead of sitting at one
//! flat depth.
//!
//! # Coordinate conventions
//!
//! We use native Ragnarok world coordinates throughout, matching the rest of the
//! renderer. The important consequence here is that negative Y is up, so a point
//! one unit above the ground is `wy - 1.0`, not `wy + 1.0`. Ground positions come
//! from `MapCoordinates::cell_to_world`, which only fills in the X and Z axes
//! (it always returns `wy = 0.0`); the real height is read separately from the
//! GAT. Cells are addressed by their centre (`cell + 0.5`) so an entity standing
//! on a cell projects to the middle of it rather than a corner.
//!
//! Screen coordinates are pixels with the origin at the top-left and Y pointing
//! down, as produced by `Camera::world_to_screen_with_depth`. The depth value we
//! carry is the NDC z that `world_to_screen_with_depth` returns (near plane at 0,
//! far plane at 1 under the wgpu perspective convention).
//!
//! # Depth at the anchor
//!
//! We take the raw NDC z from the projection and pull the sprite a fixed number
//! of world units toward the camera:
//!
//! ```text
//! ndc_z = ndc_z_raw - near * ENTITY_DEPTH_BIAS_UNITS / clip_w^2
//! ```
//!
//! `clip_w` is the view-space distance to the point, so the bias is largest up
//! close and fades with distance, which is what we want: it keeps a sprite from
//! z-fighting with the ground cell it stands on without noticeably reordering
//! sprites that are far away.
//!
//! # Depth gradient
//!
//! A billboard occupies a range of screen pixels, and each pixel should carry the
//! depth of the world point that pixel covers, otherwise the quad reads as a
//! single flat plane and sorts wrong against sloped geometry. We approximate the
//! depth across the quad as a plane in screen space:
//!
//! ```text
//! z(sx, sy) ~= z0 + grad.x * (sx - sx0) + grad.y * (sy - sy0)
//! ```
//!
//! To find `grad` we project two nearby world points, measure how screen position
//! and depth change along each, and solve the resulting 2x2 system. For a standing
//! sprite the two directions are "up" (`wy - 1.0`) and camera-right, so the
//! gradient follows the upright billboard. For a ground-lying effect the two
//! directions both stay on the ground plane (camera-right, and camera-forward
//! projected onto the ground), so depth interpolates across the floor.
//!
//! With screen deltas `(a, b)` and depth delta `e` along the first direction, and
//! `(c, d)`, `f` along the second, Cramer's rule gives:
//!
//! ```text
//! det    = a*d - b*c
//! grad.x = (e*d - b*f) / det
//! grad.y = (a*f - e*c) / det
//! ```
//!
//! A near-zero determinant means the two directions collapsed to one on screen
//! (edge-on), and we fall back to a flat `[0, 0]` gradient.
//!
//! # Scale
//!
//! `sprite_scale` converts a sprite authored in its own pixel units into world
//! pixels. `Camera::perspective_scale` gives pixels-per-world-unit at the point's
//! depth; multiplying by the map zoom and dividing by the reference sprite pixel
//! size (75) lands the sprite at the right on-screen size for the current camera
//! distance.

use crate::Camera;
use ragnarok_formats::gat::GatFile;
use ragnarok_formats::map_coordinates::MapCoordinates;

/// Depth gradient for an upright billboard, using "up" (`wy - 1.0`) and
/// camera-right as the two screen-space basis directions.
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

/// Depth gradient for a ground-lying effect. Both basis directions stay on the
/// ground plane (camera-right, and camera-forward projected onto the ground), so
/// depth interpolates across the floor instead of up the sprite.
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

/// Upright billboard for an effect anchored at a world point: screen anchor,
/// biased depth, pixels-per-world-unit and the upright depth gradient. Effects
/// standing in a prop (a torch in its brasero) need the same front/back treatment
/// as entity sprites, or the prop's own geometry eats the lower half of the quad.
pub fn project_effect_billboard(
    world: [f32; 3],
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
) -> Option<([f32; 2], f32, f32, [f32; 2])> {
    let [wx, wy, wz] = world;
    let (sx, sy, ndc_z_raw, clip_w) =
        camera.world_to_screen_with_depth(wx, wy, wz, screen_w, screen_h)?;
    let ndc_z =
        ndc_z_raw - camera.near * crate::effect_sprite::ENTITY_DEPTH_BIAS_UNITS / (clip_w * clip_w);
    let grad = depth_gradient(camera, [wx, wy, wz], sx, sy, ndc_z_raw, screen_w, screen_h);
    let ppu = camera.perspective_scale(wx, wy, wz, screen_h);

    Some(([sx, sy], ndc_z, ppu, grad))
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

/// Screen position and sprite scale for a point offset from a cell's origin, in
/// world units. Objects that sit near an actor without being part of its sprite,
/// such as floating numbers, have their own world position and project from it.
pub fn project_cell_offset(
    cell: (f32, f32),
    offset: [f32; 3],
    gat: Option<&GatFile>,
    coords: &MapCoordinates,
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
) -> Option<(f32, f32, f32)> {
    let base = cell_world_pos(cell, gat, coords);
    let world = [
        base[0] + offset[0],
        base[1] + offset[1],
        base[2] + offset[2],
    ];
    let ([sx, sy], _, _, sprite_scale, _) =
        project_world_screen(world, coords, camera, screen_w, screen_h)?;
    Some((sx, sy, sprite_scale))
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
