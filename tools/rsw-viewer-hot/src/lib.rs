// Force System allocator so Vec/String allocations are interchangeable with the
// host binary across the cdylib boundary.
#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::{UiDrawCall, UiTextureRef};
use ragnarok_ui::draw::{quad_vertices, text_vertices};

// === Action codes (FFI contract — host duplicates these as `as u32` casts) ===
pub const ACTION_TOGGLE_PAUSE: u32 = 1;
pub const ACTION_TOGGLE_GRID: u32 = 2;
pub const ACTION_TOGGLE_HOVER: u32 = 3;
pub const ACTION_CYCLE_OVERLAY_MODE: u32 = 4;
pub const ACTION_RESET_CAMERA: u32 = 5;
pub const ACTION_ZOOM_IN: u32 = 6;
pub const ACTION_ZOOM_OUT: u32 = 7;
pub const ACTION_SHOW_CONTROLS: u32 = 8;
pub const ACTION_SHOW_MAP_INFO: u32 = 9;
pub const ACTION_CLOSE_INFO_PANEL: u32 = 10;

// === C-ABI POD types (host duplicates these layouts exactly) ===

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CameraView {
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ViewerFlags {
    /// 0=None 1=Grid 2=Hover 3=Full
    pub overlay_mode: u8,
    pub paused: u8,
    /// 0=None 1=Controls 2=MapInfo
    pub show_info: u8,
    pub hover_on: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct MapLightInfo {
    pub has_sun: u8,
    pub _pad0: [u8; 3],
    pub sun_lon: i32,
    pub sun_lat: i32,
    pub has_ambient: u8,
    pub _pad1: [u8; 3],
    pub ambient: [f32; 3],
    pub has_diffuse: u8,
    pub _pad2: [u8; 3],
    pub diffuse: [f32; 3],
    pub shadow_alpha: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SceneOverrides {
    pub override_light: u8,
    pub _pad0: [u8; 3],
    pub light_dir: [f32; 3],
    pub diffuse: [f32; 3],
    pub ambient: [f32; 3],
}

// === Internal state ===

#[derive(Clone, Copy, PartialEq)]
enum OverlayMode {
    None = 0,
    Grid = 1,
    Hover = 2,
    Full = 3,
}

#[derive(Clone, Copy, PartialEq)]
enum InfoPanel {
    None = 0,
    Controls = 1,
    MapInfo = 2,
}

#[derive(Default)]
struct MapInfoMirror {
    name: String,
    gnd_w: i32,
    gnd_h: i32,
    gnd_zoom: f32,
    gat_w: i32,
    gat_h: i32,
    model_count: u32,
    object_count: u32,
    has_water: bool,
    light: MapLightInfo,
}

struct State {
    // Camera (the iterable bits — sensitivities, defaults, presets all live here)
    yaw: f32,
    pitch: f32,
    distance: f32,
    target: [f32; 3],
    viewport_w: f32,
    viewport_h: f32,

    // Toggles
    overlay_mode: OverlayMode,
    paused: bool,
    show_info: InfoPanel,
    hover_on: bool,

    // Map metadata mirror (pushed by host on map load)
    map: Option<MapInfoMirror>,

    // Hover cell (pushed by host after raycast)
    hover_cell: Option<(i32, i32)>,

    // Click-to-move tween state
    animating: bool,
    target_goal: [f32; 3],
}

impl State {
    fn default_camera(&mut self) {
        self.yaw = std::f32::consts::PI * 0.25;
        self.pitch = std::f32::consts::FRAC_PI_4;
        self.distance = 160.0;
    }

    fn handle_action(&mut self, code: u32) {
        match code {
            ACTION_TOGGLE_PAUSE => self.paused = !self.paused,
            ACTION_TOGGLE_GRID => {
                self.overlay_mode = match self.overlay_mode {
                    OverlayMode::None => OverlayMode::Grid,
                    OverlayMode::Grid => OverlayMode::None,
                    OverlayMode::Hover => OverlayMode::Full,
                    OverlayMode::Full => OverlayMode::Hover,
                };
            }
            ACTION_TOGGLE_HOVER => {
                self.hover_on = !self.hover_on;
                self.overlay_mode = match (self.overlay_mode, self.hover_on) {
                    (OverlayMode::None, true) => OverlayMode::Hover,
                    (OverlayMode::Grid, true) => OverlayMode::Full,
                    (OverlayMode::Hover, false) => OverlayMode::None,
                    (OverlayMode::Full, false) => OverlayMode::Grid,
                    (m, _) => m,
                };
            }
            ACTION_CYCLE_OVERLAY_MODE => {
                self.overlay_mode = match self.overlay_mode {
                    OverlayMode::None => OverlayMode::Grid,
                    OverlayMode::Grid => OverlayMode::Hover,
                    OverlayMode::Hover => OverlayMode::Full,
                    OverlayMode::Full => OverlayMode::None,
                };
                self.hover_on = matches!(self.overlay_mode, OverlayMode::Hover | OverlayMode::Full);
            }
            ACTION_RESET_CAMERA => {
                self.default_camera();
            }
            ACTION_ZOOM_IN => {
                self.distance = (self.distance * 0.8).max(20.0);
            }
            ACTION_ZOOM_OUT => {
                self.distance = (self.distance / 0.8).min(1500.0);
            }
            ACTION_SHOW_CONTROLS => self.show_info = InfoPanel::Controls,
            ACTION_SHOW_MAP_INFO => self.show_info = InfoPanel::MapInfo,
            ACTION_CLOSE_INFO_PANEL => self.show_info = InfoPanel::None,
            _ => {}
        }
    }

    fn handle_mouse_drag(&mut self, dx: f32, dy: f32, _button: u8) {
        const ORBIT_SENSITIVITY: f32 = 0.005;
        self.yaw -= dx * ORBIT_SENSITIVITY;
        self.pitch =
            (self.pitch - dy * ORBIT_SENSITIVITY).clamp(0.1, std::f32::consts::FRAC_PI_2 - 0.01);
    }
}

// === FFI exports ===

#[unsafe(no_mangle)]
pub extern "C" fn hot_create() -> *mut () {
    let mut state = State {
        yaw: 0.0,
        pitch: 0.0,
        distance: 0.0,
        target: [0.0, 0.0, 0.0],
        viewport_w: 1280.0,
        viewport_h: 800.0,
        overlay_mode: OverlayMode::Full,
        paused: false,
        show_info: InfoPanel::None,
        hover_on: true,
        map: None,
        hover_cell: None,
        animating: false,
        target_goal: [0.0, 0.0, 0.0],
    };
    state.default_camera();
    Box::into_raw(Box::new(state)) as *mut ()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_destroy(state_ptr: *mut ()) {
    if !state_ptr.is_null() {
        unsafe { drop(Box::from_raw(state_ptr as *mut State)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_update(state_ptr: *mut (), dt: f32) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    // Camera tween: exponential damping toward goal.
    if state.animating {
        const TWEEN_RATE: f32 = 12.0; // higher = snappier
        const SNAP_THRESHOLD: f32 = 0.5;

        let goal = glam::Vec3::from(state.target_goal);
        let current = glam::Vec3::from(state.target);
        let dist = (goal - current).length();

        if dist < SNAP_THRESHOLD {
            state.target = state.target_goal;
            state.animating = false;
        } else {
            let alpha = 1.0 - (-dt * TWEEN_RATE).exp();
            state.target = current.lerp(goal, alpha).into();
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_on_action(state_ptr: *mut (), action_code: u32) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    state.handle_action(action_code);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_on_mouse_drag(state_ptr: *mut (), dx: f32, dy: f32, button: u8) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    state.handle_mouse_drag(dx, dy, button);
}

/// Click-to-move: set a new camera target goal. The tween runs in hot_update.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_set_target(state_ptr: *mut (), x: f32, y: f32, z: f32) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    state.target_goal = [x, y, z];
    state.animating = true;
}

/// Cancel any in-progress camera tween.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_cancel_tween(state_ptr: *mut ()) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    state.animating = false;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_on_mouse_wheel(state_ptr: *mut (), dy: f32) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    if dy < 0.0 {
        state.distance = (state.distance * 1.1).min(3000.0);
    } else {
        state.distance = (state.distance / 1.1).max(20.0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_set_viewport(state_ptr: *mut (), w: f32, h: f32) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    state.viewport_w = w;
    state.viewport_h = h;
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn hot_set_map_info(
    state_ptr: *mut (),
    name_ptr: *const u8,
    name_len: usize,
    gnd_w: i32,
    gnd_h: i32,
    gnd_zoom: f32,
    gat_w: i32,
    gat_h: i32,
    model_count: u32,
    object_count: u32,
    has_water: u8,
    light: *const MapLightInfo,
) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    let name_bytes = if name_ptr.is_null() {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(name_ptr, name_len) }
    };
    let name = std::str::from_utf8(name_bytes).unwrap_or("").to_string();
    let light_val = if light.is_null() {
        MapLightInfo::default()
    } else {
        unsafe { *light }
    };
    state.map = Some(MapInfoMirror {
        name,
        gnd_w,
        gnd_h,
        gnd_zoom,
        gat_w,
        gat_h,
        model_count,
        object_count,
        has_water: has_water != 0,
        light: light_val,
    });
    // Push camera to map center to mirror Renderer::load_map.
    state.target = [
        gnd_w as f32 * gnd_zoom / 2.0,
        0.0,
        gnd_h as f32 * gnd_zoom / 2.0,
    ];
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_set_hover_cell(state_ptr: *mut (), cx: i32, cy: i32, valid: u8) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    state.hover_cell = if valid != 0 { Some((cx, cy)) } else { None };
}

/// Get the current hovered cell from dylib (set by host each frame).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_get_hover_cell(
    state_ptr: *mut (),
    out: *mut [i32; 2],
    valid_out: *mut u8,
) {
    let state = unsafe { &*(state_ptr as *const State) };
    if let Some((cx, cy)) = state.hover_cell {
        unsafe { *out = [cx, cy] };
        unsafe { *valid_out = 1 };
    } else {
        unsafe { *out = [0, 0] };
        unsafe { *valid_out = 0 };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_get_camera(state_ptr: *mut (), out: *mut CameraView) {
    let state = unsafe { &*(state_ptr as *const State) };
    let v = CameraView {
        target: state.target,
        yaw: state.yaw,
        pitch: state.pitch,
        distance: state.distance,
    };
    unsafe { *out = v };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_set_camera(state_ptr: *mut (), camera: *const CameraView) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    if camera.is_null() {
        return;
    }
    let v = unsafe { *camera };
    state.target = v.target;
    state.yaw = v.yaw;
    state.pitch = v.pitch;
    state.distance = v.distance;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_get_flags(state_ptr: *mut (), out: *mut ViewerFlags) {
    let state = unsafe { &*(state_ptr as *const State) };
    let f = ViewerFlags {
        overlay_mode: state.overlay_mode as u8,
        paused: state.paused as u8,
        show_info: state.show_info as u8,
        hover_on: state.hover_on as u8,
    };
    unsafe { *out = f };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_get_overrides(state_ptr: *mut (), out: *mut SceneOverrides) {
    let _state = unsafe { &*(state_ptr as *const State) };
    // MVP: dylib never overrides lighting. Lighting scenarios land later.
    unsafe { *out = SceneOverrides::default() };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_build_overlay(
    state_ptr: *mut (),
    atlas: *const FontAtlas,
    screen_w: f32,
    screen_h: f32,
    out: *mut Vec<UiDrawCall>,
) {
    let state = unsafe { &*(state_ptr as *const State) };
    if atlas.is_null() {
        return;
    }
    let atlas = unsafe { &*atlas };
    let out = unsafe { &mut *out };

    out.extend(build_status(atlas, screen_w, state));
    out.extend(build_legend(atlas, screen_h));

    match state.show_info {
        InfoPanel::Controls => {
            let lines: &[&str] = &[
                "=== RSW Viewer Controls ===",
                "",
                "Camera:",
                "  Left click   -> Center camera on cell",
                "  Right drag   -> Orbit around map",
                "  Scroll wheel -> Zoom in/out",
                "",
                "Keyboard:",
                "  b/B          -> Open map browser",
                "  g/G          -> Toggle grid overlay",
                "  h/H          -> Toggle hover highlight",
                "  o/O          -> Cycle overlay mode",
                "  r/R          -> Reset camera position",
                "  +/-          -> Zoom in/out",
                "  Space        -> Pause/Resume water",
                "  1            -> Show this panel",
                "  2            -> Show map information",
                "  Esc          -> Close panel",
            ];
            out.extend(build_info_panel(
                atlas, screen_w, screen_h, "Controls", lines,
            ));
        }
        InfoPanel::MapInfo => {
            let lines = build_map_info_lines(state);
            let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
            out.extend(build_info_panel(
                atlas, screen_w, screen_h, "Map Info", &line_refs,
            ));
        }
        InfoPanel::None => {}
    }
}

// === Overlay builders (live in dylib so layout is hot-reloadable) ===

const PADDING: f32 = 8.0;
const LINE_HEIGHT: f32 = 16.0;
const KEY_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.9];
const DESC_COLOR: [f32; 4] = [0.7, 0.7, 0.7, 0.7];
const BG_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.5];

const LEGEND_ENTRIES: &[(&str, &str)] = &[
    ("b/B", "Open map browser"),
    ("g/G", "Toggle grid overlay"),
    ("h/H", "Toggle hover highlight"),
    ("o/O", "Cycle overlay mode"),
    ("r/R", "Reset camera"),
    ("+/-", "Zoom in/out"),
    ("Space", "Pause / Resume water"),
    ("1", "Show controls"),
    ("2", "Show map info"),
    ("Esc", "Close panel"),
];

fn build_legend(atlas: &FontAtlas, screen_h: f32) -> Vec<UiDrawCall> {
    let mut calls = Vec::new();
    let line_count = LEGEND_ENTRIES.len() as f32;
    let box_h = line_count * LINE_HEIGHT + PADDING * 2.0;
    let box_w = 220.0;
    let box_x = PADDING;
    let box_y = screen_h - box_h - PADDING;

    let (bg_verts, bg_idx) = quad_vertices(box_x, box_y, box_w, box_h, BG_COLOR);
    calls.push(UiDrawCall {
        vertices: bg_verts.to_vec(),
        indices: bg_idx.to_vec(),
        texture: UiTextureRef::White,
    });

    let key_col_x = box_x + PADDING;
    let desc_col_x = box_x + 80.0;

    for (i, (key, desc)) in LEGEND_ENTRIES.iter().enumerate() {
        let y = box_y + PADDING + i as f32 * LINE_HEIGHT;
        let (kv, ki) = text_vertices(key, key_col_x, y, KEY_COLOR, atlas);
        calls.push(UiDrawCall {
            vertices: kv,
            indices: ki,
            texture: UiTextureRef::FontAtlas,
        });
        let (dv, di) = text_vertices(desc, desc_col_x, y, DESC_COLOR, atlas);
        calls.push(UiDrawCall {
            vertices: dv,
            indices: di,
            texture: UiTextureRef::FontAtlas,
        });
    }

    calls
}

fn build_status(atlas: &FontAtlas, screen_w: f32, state: &State) -> Vec<UiDrawCall> {
    let map_label: &str = state.map.as_ref().map_or("(no map)", |m| m.name.as_str());
    let pause_str = if state.paused { " [PAUSED]" } else { "" };
    let mode_str = match state.overlay_mode {
        OverlayMode::None => "CLEAN",
        OverlayMode::Grid => "GRID",
        OverlayMode::Hover => "HOVER",
        OverlayMode::Full => "FULL",
    };
    let hover_str = match state.hover_cell {
        Some((cx, cy)) => format!("  Cell: ({cx}, {cy})"),
        None => String::new(),
    };
    let text = format!("Map: {map_label}  Mode: {mode_str}{pause_str}{hover_str}");

    let text_w = atlas.measure_text(&text);
    let box_w = text_w + PADDING * 2.0;
    let box_h = LINE_HEIGHT + PADDING * 2.0;
    let box_x = (screen_w - box_w) / 2.0;
    let box_y = PADDING;

    let mut calls = Vec::new();
    let (bg_verts, bg_idx) = quad_vertices(box_x, box_y, box_w, box_h, BG_COLOR);
    calls.push(UiDrawCall {
        vertices: bg_verts.to_vec(),
        indices: bg_idx.to_vec(),
        texture: UiTextureRef::White,
    });
    let (tv, ti) = text_vertices(&text, box_x + PADDING, box_y + PADDING, KEY_COLOR, atlas);
    calls.push(UiDrawCall {
        vertices: tv,
        indices: ti,
        texture: UiTextureRef::FontAtlas,
    });
    calls
}

fn build_info_panel(
    atlas: &FontAtlas,
    screen_w: f32,
    screen_h: f32,
    title: &str,
    lines: &[&str],
) -> Vec<UiDrawCall> {
    let mut calls = Vec::new();

    let box_w = 400.0;
    let box_h = (lines.len() as f32 + 1.5) * LINE_HEIGHT + PADDING * 2.0;
    let box_x = (screen_w - box_w) / 2.0;
    let box_y = (screen_h - box_h) / 2.0;

    let (bg_verts, bg_idx) = quad_vertices(box_x, box_y, box_w, box_h, BG_COLOR);
    calls.push(UiDrawCall {
        vertices: bg_verts.to_vec(),
        indices: bg_idx.to_vec(),
        texture: UiTextureRef::White,
    });

    let (tv, ti) = text_vertices(title, box_x + PADDING, box_y + PADDING, KEY_COLOR, atlas);
    calls.push(UiDrawCall {
        vertices: tv,
        indices: ti,
        texture: UiTextureRef::FontAtlas,
    });

    for (i, line) in lines.iter().enumerate() {
        let y = box_y + PADDING + LINE_HEIGHT * 1.2 + i as f32 * LINE_HEIGHT;
        let (lv, li) = text_vertices(line, box_x + PADDING, y, DESC_COLOR, atlas);
        calls.push(UiDrawCall {
            vertices: lv,
            indices: li,
            texture: UiTextureRef::FontAtlas,
        });
    }

    calls
}

fn build_map_info_lines(state: &State) -> Vec<String> {
    let mut lines = vec!["=== Map Information ===".to_string()];
    let Some(m) = state.map.as_ref() else {
        lines.push("No map loaded".to_string());
        return lines;
    };
    lines.push(format!("Name: {}", m.name));
    lines.push(format!("Size: {}x{}", m.gnd_w, m.gnd_h));
    lines.push(format!("Zoom: {:.1}", m.gnd_zoom));
    lines.push(format!("Cell size: {:.2}x{:.2}", m.gnd_zoom, m.gnd_zoom));
    if m.gat_w > 0 && m.gat_h > 0 {
        lines.push(format!("GAT resolution: {}x{}", m.gat_w, m.gat_h));
    }
    lines.push(format!("Models: {}", m.model_count));
    lines.push(format!("Objects: {}", m.object_count));
    if m.light.has_sun != 0 {
        lines.push(format!(
            "Sun: lon={} lat={}",
            m.light.sun_lon, m.light.sun_lat
        ));
    }
    if m.light.has_ambient != 0 {
        lines.push(format!(
            "Ambient: [{:.2}, {:.2}, {:.2}]",
            m.light.ambient[0], m.light.ambient[1], m.light.ambient[2]
        ));
    }
    if m.has_water {
        lines.push("Water: Yes".to_string());
    }
    lines
}
