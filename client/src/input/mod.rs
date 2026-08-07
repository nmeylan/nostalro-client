mod window_events;

use ragnarok_formats::gat::GatFile;
use ragnarok_formats::map_coordinates::MapCoordinates;
use ragnarok_renderer::Camera;

pub struct InputState {
    pub right_mouse_down: bool,
    pub right_dragged: bool,
    pub right_press_entity: Option<u32>,
    /// Attackable entity (incl. monsters) under the cursor at right-press, for
    /// Alt+right-click companion attack orders.
    pub right_press_target: Option<u32>,
    pub left_mouse_down: bool,
    /// When and where the last right-press landed, for the world's own
    /// double-click detection (UI widgets have their own on `Response`).
    pub last_right_press: Option<(std::time::Instant, (f64, f64))>,
    pub last_mouse_pos: Option<(f64, f64)>,
    pub mouse_position: (f64, f64),
    pub walk_packet_cooldown: f32,
    pub walk_server_acked: bool,
    pub alt_pressed: bool,
    pub shift_pressed: bool,
    pub ctrl_pressed: bool,
    pub ui_hovered: bool,
    pub ui_dragging: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            right_mouse_down: false,
            right_dragged: false,
            right_press_entity: None,
            right_press_target: None,
            left_mouse_down: false,
            last_right_press: None,
            last_mouse_pos: None,
            mouse_position: (0.0, 0.0),
            walk_packet_cooldown: 0.0,
            walk_server_acked: true,
            alt_pressed: false,
            shift_pressed: false,
            ctrl_pressed: false,
            ui_hovered: false,
            ui_dragging: false,
        }
    }
}

pub fn position_camera_at(
    camera: &mut Camera,
    gat: Option<&GatFile>,
    coords: &MapCoordinates,
    cell_x: f32,
    cell_y: f32,
) {
    let (wx, _, wz) = coords.cell_to_world(cell_x + 0.5, cell_y + 0.5);
    let wy = gat.map_or(0.0, |gat| gat.get_height(cell_x + 0.5, cell_y + 0.5));
    camera.set_target(wx, wy, wz);
}

pub fn hovered_cell(
    mouse_pos: (f64, f64),
    camera: &Camera,
    surface_w: f32,
    surface_h: f32,
    coords: &MapCoordinates,
    gat: Option<&GatFile>,
) -> Option<(i32, i32)> {
    let (mx, my) = mouse_pos;
    let (origin, dir) = camera.screen_to_ray(mx as f32, my as f32, surface_w, surface_h);

    match gat {
        Some(gat) => march_terrain(origin, dir, camera.far, coords, gat),
        None => flat_ground_cell(origin, dir, coords),
    }
}

fn flat_ground_cell(
    origin: glam::Vec3,
    dir: glam::Vec3,
    coords: &MapCoordinates,
) -> Option<(i32, i32)> {
    if dir.y.abs() < 1e-6 {
        return None;
    }
    let t = -origin.y / dir.y;
    if t < 0.0 {
        return None;
    }
    let hit = origin + dir * t;
    let (cx, cy) = coords.world_to_cell(hit.x, hit.z);
    coords.is_valid_cell(cx, cy).then_some((cx, cy))
}

/// Walks the GAT grid cell by cell from the camera outwards and returns the first
/// cell whose surface the ray meets — either by crossing its top quad or by
/// entering it already underneath, which is what a cliff face is.
fn march_terrain(
    origin: glam::Vec3,
    dir: glam::Vec3,
    far: f32,
    coords: &MapCoordinates,
    gat: &GatFile,
) -> Option<(i32, i32)> {
    let (ox, oy) = coords.world_to_cell_f(origin.x, origin.z);
    let (dx, dy) = coords.world_to_cell_f(dir.x, dir.z);
    let (mut t, t_end) = clip_to_grid(
        (ox, oy),
        (dx, dy),
        (gat.width as f32, gat.height as f32),
        far,
    )?;

    let mut cx = ((ox + dx * t).floor() as i32).clamp(0, gat.width - 1);
    let mut cy = ((oy + dy * t).floor() as i32).clamp(0, gat.height - 1);

    let mut next_x = boundary_t(t, ox + dx * t, dx, cx);
    let mut next_y = boundary_t(t, oy + dy * t, dy, cy);
    let step_t_x = if dx == 0.0 {
        f32::INFINITY
    } else {
        1.0 / dx.abs()
    };
    let step_t_y = if dy == 0.0 {
        f32::INFINITY
    } else {
        1.0 / dy.abs()
    };

    let under = |t: f32, cx: i32, cy: i32| {
        let fx = (ox + dx * t - cx as f32).clamp(0.0, 1.0);
        let fy = (oy + dy * t - cy as f32).clamp(0.0, 1.0);
        let cell = &gat.cells[(cy * gat.width + cx) as usize];
        origin.y + dir.y * t >= cell.interpolate_height(fx, fy)
    };

    for _ in 0..(gat.width + gat.height + 2) {
        let cell_end = next_x.min(next_y).min(t_end);
        if under(t, cx, cy) || under(cell_end, cx, cy) {
            return Some((cx, cy));
        }
        if cell_end >= t_end {
            return None;
        }
        if next_x < next_y {
            cx += if dx > 0.0 { 1 } else { -1 };
            t = next_x;
            next_x += step_t_x;
        } else {
            cy += if dy > 0.0 { 1 } else { -1 };
            t = next_y;
            next_y += step_t_y;
        }
        if cx < 0 || cy < 0 || cx >= gat.width || cy >= gat.height {
            return None;
        }
    }
    None
}

fn boundary_t(t: f32, pos: f32, d: f32, cell: i32) -> f32 {
    if d > 0.0 {
        t + ((cell + 1) as f32 - pos) / d
    } else if d < 0.0 {
        t + (cell as f32 - pos) / d
    } else {
        f32::INFINITY
    }
}

/// The `[enter, exit]` span over which the ray is inside the grid, clamped to the
/// camera's far plane. `None` when the ray never crosses the grid ahead of it.
fn clip_to_grid(
    origin: (f32, f32),
    dir: (f32, f32),
    size: (f32, f32),
    far: f32,
) -> Option<(f32, f32)> {
    let mut enter = 0.0f32;
    let mut exit = far;
    for (o, d, max) in [(origin.0, dir.0, size.0), (origin.1, dir.1, size.1)] {
        if d.abs() < 1e-9 {
            if o < 0.0 || o > max {
                return None;
            }
            continue;
        }
        let (a, b) = ((-o) / d, (max - o) / d);
        enter = enter.max(a.min(b));
        exit = exit.min(a.max(b));
    }
    (enter <= exit).then_some((enter, exit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_formats::gat::GatCell;

    const GAT_DIM: i32 = 24;
    const SCREEN: (f32, f32) = (800.0, 600.0);

    fn map(height_of: impl Fn(i32) -> f32) -> (GatFile, MapCoordinates) {
        let cells = (0..GAT_DIM * GAT_DIM)
            .map(|i| {
                let h = height_of(i / GAT_DIM);
                GatCell {
                    height_sw: h,
                    height_se: h,
                    height_nw: h,
                    height_ne: h,
                    cell_flags: 0,
                }
            })
            .collect();
        (
            GatFile {
                version: (1, 2),
                width: GAT_DIM,
                height: GAT_DIM,
                cells,
            },
            MapCoordinates::new(10.0, GAT_DIM, GAT_DIM, GAT_DIM / 2, GAT_DIM / 2),
        )
    }

    fn camera_over(coords: &MapCoordinates, gat: &GatFile, cell_x: f32, cell_y: f32) -> Camera {
        let mut camera = Camera::default();
        let (wx, _, wz) = coords.cell_to_world(cell_x, cell_y);
        camera.target = glam::Vec3::new(wx, gat.get_height(cell_x, cell_y), wz);
        camera.aspect = SCREEN.0 / SCREEN.1;
        camera
    }

    fn pick_at_surface(
        camera: &Camera,
        coords: &MapCoordinates,
        gat: &GatFile,
        cell_x: f32,
        cell_y: f32,
    ) -> Option<(i32, i32)> {
        let (wx, _, wz) = coords.cell_to_world(cell_x, cell_y);
        let wy = gat.get_height(cell_x, cell_y);
        let (sx, sy) = camera.world_to_screen(wx, wy, wz, SCREEN.0, SCREEN.1)?;
        hovered_cell(
            (sx as f64, sy as f64),
            camera,
            SCREEN.0,
            SCREEN.1,
            coords,
            Some(gat),
        )
    }

    #[test]
    fn each_stair_tread_picks_the_cell_under_the_cursor() {
        let (gat, coords) = map(|cy| -5.0 * (cy - 6).clamp(0, 8) as f32);
        let camera = camera_over(&coords, &gat, 12.5, 4.5);

        for cy in 3..14 {
            assert_eq!(
                pick_at_surface(&camera, &coords, &gat, 12.5, cy as f32 + 0.5),
                Some((12, cy)),
                "tread {cy}"
            );
        }
    }

    #[test]
    fn a_cliff_face_captures_the_pointer_instead_of_the_floor_it_hides() {
        let (gat, coords) = map(|cy| if (10..14).contains(&cy) { -60.0 } else { 0.0 });
        let camera = camera_over(&coords, &gat, 12.5, 4.5);

        assert_eq!(
            pick_at_surface(&camera, &coords, &gat, 12.5, 15.5),
            Some((12, 10))
        );
    }
}

pub use ragnarok_renderer::sprite_projection::cell_world_pos;
pub use ragnarok_renderer::sprite_projection::entity_ground_gradient;
pub use ragnarok_renderer::sprite_projection::project_entity_screen as entity_screen_params;
pub use ragnarok_renderer::sprite_projection::project_world_screen;
