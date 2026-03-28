use ragnarok_formats::gat::GatFile;
use ragnarok_game::map_coordinates::MapCoordinates;
use ragnarok_renderer::Camera;

pub struct InputState {
    pub right_mouse_down: bool,
    pub left_mouse_down: bool,
    pub last_mouse_pos: Option<(f64, f64)>,
    pub mouse_position: (f64, f64),
    pub walk_packet_cooldown: f32,
    pub walk_server_acked: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            right_mouse_down: false,
            left_mouse_down: false,
            last_mouse_pos: None,
            mouse_position: (0.0, 0.0),
            walk_packet_cooldown: 0.0,
            walk_server_acked: true,
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
) -> Option<(i32, i32)> {
    let (mx, my) = mouse_pos;
    let (origin, dir) = camera.screen_to_ray(mx as f32, my as f32, surface_w, surface_h);

    if dir.y.abs() < 1e-6 {
        return None;
    }
    let t = -origin.y / dir.y;
    if t < 0.0 {
        return None;
    }
    let hit = origin + dir * t;

    let (cell_x, cell_y) = coords.world_to_cell(hit.x, hit.z);
    if !coords.is_valid_cell(cell_x, cell_y) {
        return None;
    }
    Some((cell_x, cell_y))
}

pub fn handle_camera_drag(camera: &mut Camera, dx: f32, dy: f32, free_camera: bool) {
    camera.yaw += dx * 0.0175;
    if free_camera {
        camera.pitch = (camera.pitch - dy * 0.005)
            .clamp(0.1, std::f32::consts::FRAC_PI_2 - 0.01);
    }
}

pub fn handle_camera_zoom(camera: &mut Camera, scroll: f32) {
    camera.distance = (camera.distance - scroll * 20.0).clamp(50.0, 1500.0);
}

pub fn entity_screen_params(
    pos: (f32, f32),
    gat: Option<&GatFile>,
    coords: &MapCoordinates,
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
) -> Option<([f32; 2], f32, u8, f32)> {
    let (cell_x, cell_y) = pos;
    let (wx, _, wz) = coords.cell_to_world(cell_x + 0.5, cell_y + 0.5);
    let wy = gat.map_or(0.0, |gat| gat.get_height(cell_x + 0.5, cell_y + 0.5));

    let (sx, sy, ndc_z, clip_w) = camera.world_to_screen_with_depth(wx, wy, wz, screen_w, screen_h)?;
    // Scale bias to a constant view-space offset; fixed NDC bias grew to ~450 world units at max zoom
    let ndc_z = ndc_z - camera.near * 4.0 / (clip_w * clip_w);

    let camera_dir = camera.direction_index();
    let ppu = camera.perspective_scale(wx, wy, wz, screen_h);
    let sprite_scale = ppu * coords.zoom() / 75.0;

    Some(([sx, sy], ndc_z, camera_dir, sprite_scale))
}
