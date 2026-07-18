mod window_events;

use ragnarok_formats::gat::GatFile;
use ragnarok_game::map_coordinates::MapCoordinates;
use ragnarok_renderer::Camera;

pub struct InputState {
    pub right_mouse_down: bool,
    pub right_dragged: bool,
    pub right_press_entity: Option<u32>,
    /// Attackable entity (incl. monsters) under the cursor at right-press, for
    /// Alt+right-click companion attack orders.
    pub right_press_target: Option<u32>,
    pub left_mouse_down: bool,
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

    if dir.y.abs() < 1e-6 {
        return None;
    }

    let mut plane_y = 0.0f32;
    let mut cell = None;

    for _ in 0..5 {
        let t = (plane_y - origin.y) / dir.y;
        if t < 0.0 {
            return None;
        }
        let hit = origin + dir * t;
        let (cx, cy) = coords.world_to_cell(hit.x, hit.z);
        if !coords.is_valid_cell(cx, cy) {
            return None;
        }

        if cell == Some((cx, cy)) {
            break;
        }
        cell = Some((cx, cy));
        plane_y = gat.map_or(0.0, |g| g.get_height(cx as f32 + 0.5, cy as f32 + 0.5));
    }
    cell
}

/// Indoor maps clamp the zoom to a narrow band and forbid rotation, matching the
/// original client's fixed indoor camera.
pub const INDOOR_MIN_DISTANCE: f32 = 150.0;
pub const INDOOR_MAX_DISTANCE: f32 = 300.0;

/// Snap the camera to the fixed indoor view: 45° rotation, zoom clamped into the
/// indoor band.
pub fn lock_indoor_camera(camera: &mut Camera) {
    camera.yaw = -std::f32::consts::FRAC_PI_4;
    camera.distance = camera
        .distance
        .clamp(INDOOR_MIN_DISTANCE, INDOOR_MAX_DISTANCE);
}

pub fn handle_camera_drag(camera: &mut Camera, dx: f32, dy: f32, free_camera: bool, locked: bool) {
    if locked {
        return;
    }
    camera.yaw += dx * 0.0175;
    if free_camera {
        camera.pitch = (camera.pitch - dy * 0.005).clamp(0.1, std::f32::consts::FRAC_PI_2 - 0.01);
    }
}

pub fn handle_camera_zoom(camera: &mut Camera, scroll: f32, locked: bool) {
    let (min, max) = if locked {
        (INDOOR_MIN_DISTANCE, INDOOR_MAX_DISTANCE)
    } else {
        (50.0, 1500.0)
    };
    camera.distance = (camera.distance - scroll * 20.0).clamp(min, max);
}

pub use ragnarok_renderer::sprite_projection::entity_ground_gradient;
pub use ragnarok_renderer::sprite_projection::project_entity_screen as entity_screen_params;
pub use ragnarok_renderer::sprite_projection::project_world_screen;
