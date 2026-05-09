use crate::{InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const MINIMAP_WINDOW_ID: WidgetId = WidgetId(1500);
const ZOOM_IN_BTN_ID: WidgetId = WidgetId(1501);
const ZOOM_OUT_BTN_ID: WidgetId = WidgetId(1502);

const MAP_ARROW_TEX: &str = "data/texture/유저인터페이스/map/map_arrow.bmp";
const MAP_PLUS_OFF: &str = "data/texture/유저인터페이스/map/map_plus0.bmp";
const MAP_PLUS_ON: &str = "data/texture/유저인터페이스/map/map_plus1.bmp";
const MAP_MINUS_OFF: &str = "data/texture/유저인터페이스/map/map_minus0.bmp";
const MAP_MINUS_ON: &str = "data/texture/유저인터페이스/map/map_minus1.bmp";

const BTN_ZOOM_IN: ButtonTextures = ButtonTextures {
    normal: MAP_PLUS_OFF,
    hover: MAP_PLUS_ON,
    pressed: MAP_PLUS_ON,
};
const BTN_ZOOM_OUT: ButtonTextures = ButtonTextures {
    normal: MAP_MINUS_OFF,
    hover: MAP_MINUS_ON,
    pressed: MAP_MINUS_ON,
};

const MAP_AREA_SIZE: f32 = 128.0;
const ZOOM_BTN_SIZE: f32 = 12.0;
const ARROW_SIZE: f32 = 12.0;

const ZOOM_LEVELS: [f32; 5] = [1.0, 1.5, 2.0, 3.0, 5.0];

const NPC_DOT_SIZE: f32 = 3.0;
const WARP_DOT_SIZE: f32 = 4.0;

const NPC_DOT_COLOR: [f32; 4] = [0.2, 0.8, 0.2, 1.0];
const WARP_DOT_COLOR: [f32; 4] = [0.3, 0.8, 1.0, 1.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinimapVisibility {
    Visible,
    Transparent,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerType {
    Npc,
    WarpPortal,
}

pub struct MinimapMarker {
    pub x: f32,
    pub y: f32,
    pub marker_type: MarkerType,
}

pub struct MinimapWindow {
    pub has_grf_textures: bool,
    pub visibility: MinimapVisibility,
    zoom_level: usize,
    pub map_name: Option<String>,
    pub map_width: i32,
    pub map_height: i32,
    pub player_position: Option<(f32, f32)>,
    pub player_direction: u8,
    pub entity_markers: Vec<MinimapMarker>,
    pub minimap_texture_path: Option<String>,
}

impl MinimapWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            visibility: MinimapVisibility::Visible,
            zoom_level: 0,
            map_name: None,
            map_width: 1,
            map_height: 1,
            player_position: None,
            player_direction: 0,
            entity_markers: Vec::new(),
            minimap_texture_path: None,
        }
    }

    pub fn cycle_visibility(&mut self) {
        self.visibility = match self.visibility {
            MinimapVisibility::Visible => MinimapVisibility::Transparent,
            MinimapVisibility::Transparent => MinimapVisibility::Hidden,
            MinimapVisibility::Hidden => MinimapVisibility::Visible,
        };
    }

    pub fn set_map_texture(&mut self, path: Option<String>) {
        self.minimap_texture_path = path;
    }

    pub fn on_map_changed(&mut self) {
        self.zoom_level = 0;
        self.entity_markers.clear();
    }

    pub fn is_visible(&self) -> bool {
        self.visibility != MinimapVisibility::Hidden
    }

    fn compute_uv_region(&self) -> ([f32; 2], [f32; 2]) {
        let zoom = ZOOM_LEVELS[self.zoom_level];
        let half = 0.5 / zoom;
        if half >= 0.5 {
            return ([0.0, 0.0], [1.0, 1.0]);
        }
        let (px, py) = self.player_position.unwrap_or((0.0, 0.0));
        let nx = px / self.map_width.max(1) as f32;
        let ny = 1.0 - (py / self.map_height.max(1) as f32);
        let span = 2.0 * half;
        let u_min = (nx - half).clamp(0.0, 1.0 - span);
        let v_min = (ny - half).clamp(0.0, 1.0 - span);
        ([u_min, v_min], [u_min + span, v_min + span])
    }

    fn map_to_screen(
        &self,
        cell_x: f32,
        cell_y: f32,
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        map_area_x: f32,
        map_area_y: f32,
    ) -> Option<(f32, f32)> {
        let nx = cell_x / self.map_width.max(1) as f32;
        let ny = 1.0 - (cell_y / self.map_height.max(1) as f32);
        let u_span = uv_max[0] - uv_min[0];
        let v_span = uv_max[1] - uv_min[1];
        if u_span <= 0.0 || v_span <= 0.0 {
            return None;
        }
        let rel_x = (nx - uv_min[0]) / u_span;
        let rel_y = (ny - uv_min[1]) / v_span;
        if rel_x < 0.0 || rel_x > 1.0 || rel_y < 0.0 || rel_y > 1.0 {
            return None;
        }
        Some((
            map_area_x + rel_x * MAP_AREA_SIZE,
            map_area_y + rel_y * MAP_AREA_SIZE,
        ))
    }

    fn direction_angle(dir: u8) -> f32 {
        // RO directions go counter-clockwise (N=0,NW=1,W=2,...), reverse to match screen rotation
        ((12u8.wrapping_sub(dir) % 8) as f32) * std::f32::consts::FRAC_PI_4
            + std::f32::consts::PI
    }

    fn draw_dot(ui: &mut UiFrame, cx: f32, cy: f32, size: f32, color: [f32; 4]) {
        let half = size / 2.0;
        let (v, i) = draw::quad_vertices(cx - half, cy - half, size, size, color);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
    }
}

impl Window for MinimapWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }

    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        vec![MAP_ARROW_TEX, MAP_PLUS_OFF, MAP_PLUS_ON, MAP_MINUS_OFF, MAP_MINUS_ON]
    }
}

impl InGameWindow for MinimapWindow {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        _character: &mut Character,
        _data: &DataTable,
    ) -> Vec<GameEvent> {
        if self.visibility == MinimapVisibility::Hidden {
            return Vec::new();
        }

        let alpha = match self.visibility {
            MinimapVisibility::Visible => 1.0,
            MinimapVisibility::Transparent => 0.5,
            MinimapVisibility::Hidden => return Vec::new(),
        };

        let default_x = (ui.ctx.screen_width - MAP_AREA_SIZE - 2.0).max(0.0);
        let default_y = 2.0;
        let win = ui.window_at(
            MINIMAP_WINDOW_ID,
            MAP_AREA_SIZE,
            MAP_AREA_SIZE,
            0.0,
            default_x,
            default_y,
        );

        let x = win.x;
        let y = win.y;

        let win_rect = Rect::new(x, y, MAP_AREA_SIZE, MAP_AREA_SIZE);
        let resp = ui.interact(MINIMAP_WINDOW_ID, win_rect);
        if resp.hovered() {
            ui.any_interactive_hovered = true;
        }

        // Map texture
        let (uv_min, uv_max) = self.compute_uv_region();
        if let Some(tex_path) = &self.minimap_texture_path {
            let tex_color = [1.0, 1.0, 1.0, alpha];
            let (v, i) = draw::quad_vertices_uv(
                x,
                y,
                MAP_AREA_SIZE,
                MAP_AREA_SIZE,
                uv_min,
                uv_max,
                tex_color,
            );
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(tex_path.clone()),
            });
        } else {
            let placeholder = [0.05, 0.05, 0.08, alpha];
            let (v, i) =
                draw::quad_vertices(x, y, MAP_AREA_SIZE, MAP_AREA_SIZE, placeholder);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }

        // Entity markers
        for marker in &self.entity_markers {
            let (color, size) = match marker.marker_type {
                MarkerType::Npc => (
                    [NPC_DOT_COLOR[0], NPC_DOT_COLOR[1], NPC_DOT_COLOR[2], alpha],
                    NPC_DOT_SIZE,
                ),
                MarkerType::WarpPortal => (
                    [
                        WARP_DOT_COLOR[0],
                        WARP_DOT_COLOR[1],
                        WARP_DOT_COLOR[2],
                        alpha,
                    ],
                    WARP_DOT_SIZE,
                ),
            };
            if let Some((sx, sy)) =
                self.map_to_screen(marker.x, marker.y, uv_min, uv_max, x, y)
            {
                Self::draw_dot(ui, sx, sy, size, color);
            }
        }

        // Player arrow
        if let Some((px, py)) = self.player_position {
            if let Some((sx, sy)) = self.map_to_screen(px, py, uv_min, uv_max, x, y) {
                let angle = Self::direction_angle(self.player_direction);
                let color = [1.0, 1.0, 1.0, alpha];
                let (v, i) = draw::quad_vertices_rotated(sx, sy, ARROW_SIZE, angle, color);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(MAP_ARROW_TEX.to_string()),
                });
            }
        }

        // Zoom +/- buttons overlaid on top-right corner of map
        let zoom_x = x + MAP_AREA_SIZE - ZOOM_BTN_SIZE;
        let zoom_in_rect = Rect::new(zoom_x, y, ZOOM_BTN_SIZE, ZOOM_BTN_SIZE);
        let zoom_in_resp = ui.button(ZOOM_IN_BTN_ID, zoom_in_rect, &BTN_ZOOM_IN, "+");
        if zoom_in_resp.clicked() && self.zoom_level < ZOOM_LEVELS.len() - 1 {
            self.zoom_level += 1;
        }

        let zoom_out_rect =
            Rect::new(zoom_x, y + ZOOM_BTN_SIZE, ZOOM_BTN_SIZE, ZOOM_BTN_SIZE);
        let zoom_out_resp =
            ui.button(ZOOM_OUT_BTN_ID, zoom_out_rect, &BTN_ZOOM_OUT, "-");
        if zoom_out_resp.clicked() && self.zoom_level > 0 {
            self.zoom_level -= 1;
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_cycles_through_all_states() {
        let mut minimap = MinimapWindow::new();
        assert_eq!(minimap.visibility, MinimapVisibility::Visible);

        minimap.cycle_visibility();
        assert_eq!(minimap.visibility, MinimapVisibility::Transparent);

        minimap.cycle_visibility();
        assert_eq!(minimap.visibility, MinimapVisibility::Hidden);

        minimap.cycle_visibility();
        assert_eq!(minimap.visibility, MinimapVisibility::Visible);
    }

    #[test]
    fn uv_region_full_map_at_low_zoom() {
        let mut minimap = MinimapWindow::new();
        minimap.map_width = 200;
        minimap.map_height = 200;
        minimap.player_position = Some((100.0, 100.0));
        minimap.zoom_level = 0;

        let (uv_min, uv_max) = minimap.compute_uv_region();
        assert_eq!(uv_min, [0.0, 0.0]);
        assert_eq!(uv_max, [1.0, 1.0]);
    }

    #[test]
    fn uv_region_zoomed_in_centered_on_player() {
        let mut minimap = MinimapWindow::new();
        minimap.map_width = 200;
        minimap.map_height = 200;
        minimap.player_position = Some((100.0, 100.0));
        minimap.zoom_level = 1;

        let (uv_min, uv_max) = minimap.compute_uv_region();
        let span = uv_max[0] - uv_min[0];
        assert!((span - 2.0 / 3.0).abs() < 0.01);
        let center_u = (uv_min[0] + uv_max[0]) / 2.0;
        let center_v = (uv_min[1] + uv_max[1]) / 2.0;
        assert!((center_u - 0.5).abs() < 0.01);
        assert!((center_v - 0.5).abs() < 0.01);
    }

    #[test]
    fn uv_region_clamped_at_map_edge() {
        let mut minimap = MinimapWindow::new();
        minimap.map_width = 200;
        minimap.map_height = 200;
        minimap.player_position = Some((0.0, 0.0));
        minimap.zoom_level = 4;

        let (uv_min, uv_max) = minimap.compute_uv_region();
        assert!(uv_min[0] >= 0.0);
        assert!(uv_min[1] >= 0.0);
        assert!(uv_max[0] <= 1.0);
        assert!(uv_max[1] <= 1.0);
    }

    #[test]
    fn map_to_screen_player_at_center() {
        let mut minimap = MinimapWindow::new();
        minimap.map_width = 200;
        minimap.map_height = 200;
        minimap.player_position = Some((100.0, 100.0));
        minimap.zoom_level = 0;

        let uv_min = [0.0, 0.0];
        let uv_max = [1.0, 1.0];
        let result = minimap.map_to_screen(100.0, 100.0, uv_min, uv_max, 0.0, 0.0);
        assert!(result.is_some());
        let (sx, sy) = result.unwrap();
        assert!((sx - MAP_AREA_SIZE / 2.0).abs() < 0.01);
        assert!((sy - MAP_AREA_SIZE / 2.0).abs() < 0.01);
    }

    #[test]
    fn on_map_changed_resets_zoom() {
        let mut minimap = MinimapWindow::new();
        minimap.zoom_level = 3;
        minimap.entity_markers.push(MinimapMarker {
            x: 10.0,
            y: 10.0,
            marker_type: MarkerType::Npc,
        });

        minimap.on_map_changed();
        assert_eq!(minimap.zoom_level, 0);
        assert!(minimap.entity_markers.is_empty());
    }
}
