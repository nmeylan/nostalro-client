pub mod browser;
pub mod controls;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::map_loader::{self, MapData};
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::{block_on, UiDrawCall};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::rsw_viewer::browser::{BrowserTab, MapBrowser};
use crate::rsw_viewer::controls::OverlayMode;

pub struct Args {
    pub grf_path: String,
    pub map_name: Option<String>,
}

// === FFI types — must match `tools/rsw-viewer-hot/src/lib.rs` exactly ===

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CameraView {
    target: [f32; 3],
    yaw: f32,
    pitch: f32,
    distance: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ViewerFlags {
    overlay_mode: u8,
    paused: u8,
    show_info: u8,
    hover_on: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MapLightInfo {
    has_sun: u8,
    _pad0: [u8; 3],
    sun_lon: i32,
    sun_lat: i32,
    has_ambient: u8,
    _pad1: [u8; 3],
    ambient: [f32; 3],
    has_diffuse: u8,
    _pad2: [u8; 3],
    diffuse: [f32; 3],
    shadow_alpha: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SceneOverrides {
    override_light: u8,
    _pad0: [u8; 3],
    light_dir: [f32; 3],
    diffuse: [f32; 3],
    ambient: [f32; 3],
}

type HotCreateFn = extern "C" fn() -> *mut ();
type HotDestroyFn = unsafe extern "C" fn(*mut ());
type HotUpdateFn = unsafe extern "C" fn(*mut (), f32);
type HotOnActionFn = unsafe extern "C" fn(*mut (), u32);
type HotOnMouseDragFn = unsafe extern "C" fn(*mut (), f32, f32, u8);
type HotOnMouseWheelFn = unsafe extern "C" fn(*mut (), f32);
type HotSetViewportFn = unsafe extern "C" fn(*mut (), f32, f32);
type HotSetMapInfoFn = unsafe extern "C" fn(
    *mut (),
    *const u8,
    usize,
    i32,
    i32,
    f32,
    i32,
    i32,
    u32,
    u32,
    u8,
    *const MapLightInfo,
);
type HotSetHoverCellFn = unsafe extern "C" fn(*mut (), i32, i32, u8);
type HotGetCameraFn = unsafe extern "C" fn(*mut (), *mut CameraView);
type HotSetCameraFn = unsafe extern "C" fn(*mut (), *const CameraView);
type HotGetFlagsFn = unsafe extern "C" fn(*mut (), *mut ViewerFlags);
type HotGetOverridesFn = unsafe extern "C" fn(*mut (), *mut SceneOverrides);
type HotBuildOverlayFn =
    unsafe extern "C" fn(*mut (), *const FontAtlas, f32, f32, *mut Vec<UiDrawCall>);
type HotSetTargetFn = unsafe extern "C" fn(*mut (), f32, f32, f32);
type HotGetHoverCellFn = unsafe extern "C" fn(*mut (), *mut [i32; 2], *mut u8);

struct HotLib {
    _lib: libloading::Library,
    state: *mut (),
    update_fn: HotUpdateFn,
    on_action_fn: HotOnActionFn,
    on_mouse_drag_fn: HotOnMouseDragFn,
    on_mouse_wheel_fn: HotOnMouseWheelFn,
    set_viewport_fn: HotSetViewportFn,
    set_map_info_fn: HotSetMapInfoFn,
    set_hover_cell_fn: HotSetHoverCellFn,
    get_hover_cell_fn: HotGetHoverCellFn,
    get_camera_fn: HotGetCameraFn,
    set_camera_fn: HotSetCameraFn,
    get_flags_fn: HotGetFlagsFn,
    get_overrides_fn: HotGetOverridesFn,
    build_overlay_fn: HotBuildOverlayFn,
    set_target_fn: HotSetTargetFn,
    destroy_fn: HotDestroyFn,
}

impl HotLib {
    fn load(dylib_path: &Path) -> Option<Self> {
        let lib = match unsafe { libloading::Library::new(dylib_path) } {
            Ok(lib) => lib,
            Err(e) => {
                eprintln!("Failed to load dylib {}: {e}", dylib_path.display());
                return None;
            }
        };

        let (
            create_fn,
            destroy_fn,
            update_fn,
            on_action_fn,
            on_mouse_drag_fn,
            on_mouse_wheel_fn,
            set_viewport_fn,
            set_map_info_fn,
            set_hover_cell_fn,
            get_hover_cell_fn,
            get_camera_fn,
            set_camera_fn,
            get_flags_fn,
            get_overrides_fn,
            build_overlay_fn,
            set_target_fn,
        ) = unsafe {
            let create: libloading::Symbol<HotCreateFn> = lib.get(b"hot_create").ok()?;
            let destroy: libloading::Symbol<HotDestroyFn> = lib.get(b"hot_destroy").ok()?;
            let update: libloading::Symbol<HotUpdateFn> = lib.get(b"hot_update").ok()?;
            let on_action: libloading::Symbol<HotOnActionFn> = lib.get(b"hot_on_action").ok()?;
            let on_drag: libloading::Symbol<HotOnMouseDragFn> =
                lib.get(b"hot_on_mouse_drag").ok()?;
            let on_wheel: libloading::Symbol<HotOnMouseWheelFn> =
                lib.get(b"hot_on_mouse_wheel").ok()?;
            let set_viewport: libloading::Symbol<HotSetViewportFn> =
                lib.get(b"hot_set_viewport").ok()?;
            let set_map_info: libloading::Symbol<HotSetMapInfoFn> =
                lib.get(b"hot_set_map_info").ok()?;
            let set_hover: libloading::Symbol<HotSetHoverCellFn> =
                lib.get(b"hot_set_hover_cell").ok()?;
            let get_hover: libloading::Symbol<HotGetHoverCellFn> =
                lib.get(b"hot_get_hover_cell").ok()?;
            let get_camera: libloading::Symbol<HotGetCameraFn> = lib.get(b"hot_get_camera").ok()?;
            let set_camera: libloading::Symbol<HotSetCameraFn> = lib.get(b"hot_set_camera").ok()?;
            let get_flags: libloading::Symbol<HotGetFlagsFn> = lib.get(b"hot_get_flags").ok()?;
            let get_overrides: libloading::Symbol<HotGetOverridesFn> =
                lib.get(b"hot_get_overrides").ok()?;
            let build_overlay: libloading::Symbol<HotBuildOverlayFn> =
                lib.get(b"hot_build_overlay").ok()?;
            let set_target: libloading::Symbol<HotSetTargetFn> =
                lib.get(b"hot_set_target").ok()?;

            (
                *create,
                *destroy,
                *update,
                *on_action,
                *on_drag,
                *on_wheel,
                *set_viewport,
                *set_map_info,
                *set_hover,
                *get_hover,
                *get_camera,
                *set_camera,
                *get_flags,
                *get_overrides,
                *build_overlay,
                *set_target,
            )
        };

        let state = (create_fn)();
        Some(Self {
            _lib: lib,
            state,
            update_fn,
            on_action_fn,
            on_mouse_drag_fn,
            on_mouse_wheel_fn,
            set_viewport_fn,
            set_map_info_fn,
            set_hover_cell_fn,
            get_hover_cell_fn,
            get_camera_fn,
            set_camera_fn,
            get_flags_fn,
            get_overrides_fn,
            build_overlay_fn,
            set_target_fn,
            destroy_fn,
        })
    }

    fn unload(mut self) {
        if !self.state.is_null() {
            unsafe { (self.destroy_fn)(self.state) };
            self.state = std::ptr::null_mut();
        }
    }

    fn update(&self, dt: f32) {
        unsafe { (self.update_fn)(self.state, dt) };
    }
    fn on_action(&self, code: u32) {
        unsafe { (self.on_action_fn)(self.state, code) };
    }
    fn on_mouse_drag(&self, dx: f32, dy: f32, button: u8) {
        unsafe { (self.on_mouse_drag_fn)(self.state, dx, dy, button) };
    }
    fn on_mouse_wheel(&self, dy: f32) {
        unsafe { (self.on_mouse_wheel_fn)(self.state, dy) };
    }
    fn set_viewport(&self, w: f32, h: f32) {
        unsafe { (self.set_viewport_fn)(self.state, w, h) };
    }
    #[allow(clippy::too_many_arguments)]
    fn set_map_info(
        &self,
        name: &str,
        gnd_w: i32,
        gnd_h: i32,
        gnd_zoom: f32,
        gat_w: i32,
        gat_h: i32,
        model_count: u32,
        object_count: u32,
        has_water: bool,
        light: &MapLightInfo,
    ) {
        let bytes = name.as_bytes();
        unsafe {
            (self.set_map_info_fn)(
                self.state,
                bytes.as_ptr(),
                bytes.len(),
                gnd_w,
                gnd_h,
                gnd_zoom,
                gat_w,
                gat_h,
                model_count,
                object_count,
                has_water as u8,
                light as *const MapLightInfo,
            )
        };
    }
    fn set_hover_cell(&self, cx: i32, cy: i32, valid: bool) {
        unsafe { (self.set_hover_cell_fn)(self.state, cx, cy, valid as u8) };
    }
    fn get_camera(&self) -> CameraView {
        let mut out = CameraView::default();
        unsafe { (self.get_camera_fn)(self.state, &mut out as *mut CameraView) };
        out
    }
    fn set_camera(&self, c: &CameraView) {
        unsafe { (self.set_camera_fn)(self.state, c as *const CameraView) };
    }
    fn get_flags(&self) -> ViewerFlags {
        let mut out = ViewerFlags::default();
        unsafe { (self.get_flags_fn)(self.state, &mut out as *mut ViewerFlags) };
        out
    }
    #[allow(dead_code)]
    fn get_overrides(&self) -> SceneOverrides {
        let mut out = SceneOverrides::default();
        unsafe { (self.get_overrides_fn)(self.state, &mut out as *mut SceneOverrides) };
        out
    }
    fn build_overlay(
        &self,
        atlas: &FontAtlas,
        screen_w: f32,
        screen_h: f32,
        out: &mut Vec<UiDrawCall>,
    ) {
        unsafe {
            (self.build_overlay_fn)(
                self.state,
                atlas as *const FontAtlas,
                screen_w,
                screen_h,
                out as *mut Vec<UiDrawCall>,
            )
        };
    }

    /// Get the current hovered cell from dylib.
    fn get_hover_cell(&self) -> Option<(i32, i32)> {
        let mut out = [0i32, 0];
        let mut valid: u8 = 0;
        unsafe { (self.get_hover_cell_fn)(self.state, &mut out as *mut _, &mut valid as *mut _) };
        if valid != 0 { Some((out[0], out[1])) } else { None }
    }

    /// Click-to-move: set a new camera target (world-space position).
    fn set_target(&self, x: f32, y: f32, z: f32) {
        unsafe { (self.set_target_fn)(self.state, x, y, z) };
    }
}

fn find_dylib() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest_dir.parent().unwrap().join("target").join("debug");

    #[cfg(target_os = "linux")]
    let name = "librsw_viewer_hot.so";
    #[cfg(target_os = "macos")]
    let name = "librsw_viewer_hot.dylib";
    #[cfg(target_os = "windows")]
    let name = "rsw_viewer_hot.dll";

    target_dir.join(name)
}

fn dylib_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// Cached payload so we can replay `hot_set_map_info` after a dylib reload.
#[derive(Clone)]
struct MapInfoCache {
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

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<ragnarok_renderer::Renderer>,

    // Map data
    map_name: Option<String>,
    map_data: Option<MapData>,
    grf_path: String,

    // Browser
    browser: Option<MapBrowser>,
    ctrl_pressed: bool,

    // Mouse tracking (host owns: drag deltas pushed to dylib; raycast for hover)
    mouse_pos: (f32, f32),
    mouse_down_left: bool,
    mouse_down_right: bool,
    last_mouse: (f32, f32),

    // Hot-reload
    hot_lib: Option<HotLib>,
    dylib_path: PathBuf,
    last_dylib_mtime: SystemTime,
    reload_counter: u64,
    cached_map: Option<MapInfoCache>,

    last_frame: Instant,
}

impl App {
    fn new(args: Args) -> Self {
        let dylib_path = find_dylib();
        let last_dylib_mtime = dylib_mtime(&dylib_path).unwrap_or(SystemTime::UNIX_EPOCH);
        let hot_lib = HotLib::load(&dylib_path);

        Self {
            window: None,
            renderer: None,
            map_name: args.map_name.clone(),
            map_data: None,
            grf_path: args.grf_path.clone(),
            browser: None,
            ctrl_pressed: false,
            mouse_pos: (0.0, 0.0),
            mouse_down_left: false,
            mouse_down_right: false,
            last_mouse: (0.0, 0.0),
            hot_lib,
            dylib_path,
            last_dylib_mtime,
            reload_counter: 0,
            cached_map: None,
            last_frame: Instant::now(),
        }
    }

    fn load_map(&mut self) {
        let map_name = match &self.map_name {
            Some(n) => n.clone(),
            None => return,
        };

        let grf = match GrfArchive::open(Path::new(&self.grf_path)) {
            Ok(g) => g,
            Err(e) => {
                tracing::error!("Failed to open GRF '{}': {}", self.grf_path, e);
                return;
            }
        };
        if self.renderer.is_none() {
            return;
        }

        tracing::info!("Loading map: {map_name}");

        if let Some(map_data) = map_loader::load_map_data(&grf, &map_name) {
            let gnd_zoom = map_data.gnd.zoom;
            let gnd_width = map_data.gnd.width;
            let gnd_height = map_data.gnd.height;

            // Build the map info payload before moving map_data.
            let model_count = map_data
                .rsw
                .objects
                .iter()
                .filter(|o| matches!(o, ragnarok_formats::rsw::RswObject::Model(_)))
                .count() as u32;
            let object_count = map_data.rsw.objects.len() as u32;
            let has_water = map_data
                .rsw
                .water
                .water_type
                .map(|t| t > 0)
                .unwrap_or(false);

            let mut light = MapLightInfo::default();
            if let (Some(lon), Some(lat)) =
                (map_data.rsw.light.longitude, map_data.rsw.light.latitude)
            {
                light.has_sun = 1;
                light.sun_lon = lon as i32;
                light.sun_lat = lat as i32;
            }
            if let Some(ambient) = map_data.rsw.light.ambient {
                light.has_ambient = 1;
                light.ambient = ambient;
            }
            if let Some(diffuse) = map_data.rsw.light.diffuse {
                light.has_diffuse = 1;
                light.diffuse = diffuse;
            }
            if let Some(alpha) = map_data.rsw.light.shadow_map_alpha {
                light.shadow_alpha = alpha;
            }

            let (gat_w, gat_h) = map_data
                .gat
                .as_ref()
                .map(|g| (g.width, g.height))
                .unwrap_or((0, 0));

            let cache = MapInfoCache {
                name: map_name.clone(),
                gnd_w: gnd_width,
                gnd_h: gnd_height,
                gnd_zoom,
                gat_w,
                gat_h,
                model_count,
                object_count,
                has_water,
                light,
            };
            self.push_map_info_to_dylib(&cache);
            self.cached_map = Some(cache);

            self.map_data = Some(map_data);

            if let Some(data) = &self.map_data
                && let Some(renderer) = &mut self.renderer
            {
                renderer.load_map(&data.gnd, &data.rsw, &grf, data.fog);
            }

            if let Some(data) = &self.map_data
                && let Some(gat) = data.gat.as_ref()
                && let Some(renderer) = &mut self.renderer
            {
                let wgpu_device = &renderer.device.device;
                if let Some(grid_sel) = &mut renderer.grid_selector {
                    grid_sel.show_grid = true;
                    grid_sel.build_grid_mesh(
                        wgpu_device,
                        gat,
                        gnd_width,
                        gnd_height,
                        gnd_zoom,
                    );
                }
            }

            tracing::info!(
                "Map loaded successfully: {map_name} ({gnd_width}x{gnd_height})"
            );

            if let Some(window) = &self.window {
                window.set_title(&format!("RSW Viewer — {map_name}"));
            }
        } else {
            tracing::error!("Failed to load map '{map_name}'");
        }
    }

    fn push_map_info_to_dylib(&self, cache: &MapInfoCache) {
        if let Some(hot) = &self.hot_lib {
            hot.set_map_info(
                &cache.name,
                cache.gnd_w,
                cache.gnd_h,
                cache.gnd_zoom,
                cache.gat_w,
                cache.gat_h,
                cache.model_count,
                cache.object_count,
                cache.has_water,
                &cache.light,
            );
        }
    }

    fn open_browser(&mut self) {
        let grf = match GrfArchive::open(Path::new(&self.grf_path)) {
            Ok(g) => g,
            Err(e) => {
                tracing::error!("Failed to open GRF '{}': {}", self.grf_path, e);
                return;
            }
        };
        let paths: Vec<String> = grf
            .files_with_extension(".rsw")
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let mut browser = MapBrowser::from_grf_paths(paths);
        if let Some(renderer) = &self.renderer {
            browser.update_visible_rows(renderer.device.surface_config.height as f32);
        }
        self.browser = Some(browser);
    }

    fn handle_browser_select(&mut self) {
        let selected = self
            .browser
            .as_ref()
            .and_then(|b| b.selected_map().map(|s| s.to_string()));
        if let Some(name) = selected {
            self.map_name = Some(name);
            self.browser = None;
            self.load_map();
        }
    }

    fn check_hot_reload(&mut self) {
        let Some(mtime) = dylib_mtime(&self.dylib_path) else {
            return;
        };
        if mtime <= self.last_dylib_mtime {
            return;
        }
        self.last_dylib_mtime = mtime;
        self.reload_counter += 1;
        let ext = format!("hot{}.so", self.reload_counter);
        let tmp_path = self.dylib_path.with_extension(&ext);
        if std::fs::copy(&self.dylib_path, &tmp_path).is_err() {
            eprintln!("Failed to copy dylib to temp file");
            return;
        }

        eprintln!("Reloading dylib...");

        // Snapshot camera before unloading so we can restore it.
        let camera_snapshot = self.hot_lib.as_ref().map(|h| h.get_camera());

        if let Some(old) = self.hot_lib.take() {
            old.unload();
        }

        match HotLib::load(&tmp_path) {
            Some(new_lib) => {
                self.hot_lib = Some(new_lib);
                self.replay_state_after_reload(camera_snapshot);
                eprintln!("Reload complete.");
            }
            None => {
                eprintln!("Failed to load new dylib, falling back to original");
                self.hot_lib = HotLib::load(&self.dylib_path);
                self.replay_state_after_reload(camera_snapshot);
            }
        }

        if self.reload_counter > 1 {
            let prev_ext = format!("hot{}.so", self.reload_counter - 1);
            let prev = self.dylib_path.with_extension(prev_ext);
            let _ = std::fs::remove_file(prev);
        }
    }

    fn replay_state_after_reload(&self, camera: Option<CameraView>) {
        let Some(hot) = &self.hot_lib else { return };
        if let Some(renderer) = &self.renderer {
            hot.set_viewport(
                renderer.device.surface_config.width as f32,
                renderer.device.surface_config.height as f32,
            );
        }
        if let Some(cache) = &self.cached_map {
            self.push_map_info_to_dylib(cache);
        }
        if let Some(c) = camera {
            hot.set_camera(&c);
        }
    }

    fn sync_camera_to_renderer(&mut self) {
        let Some(hot) = &self.hot_lib else { return };
        let view = hot.get_camera();
        if let Some(renderer) = &mut self.renderer {
            renderer.camera.target = glam::Vec3::from(view.target);
            renderer.camera.yaw = view.yaw;
            renderer.camera.pitch = view.pitch;
            renderer.camera.distance = view.distance;
        }
    }

    fn sync_flags_to_renderer(&mut self) {
        let Some(hot) = &self.hot_lib else { return };
        let flags = hot.get_flags();
        let mode = OverlayMode::from_u8(flags.overlay_mode);
        if let Some(renderer) = &mut self.renderer
            && let Some(grid_sel) = &mut renderer.grid_selector
        {
            grid_sel.show_grid = matches!(mode, OverlayMode::Grid | OverlayMode::Full);
            grid_sel.set_hover_visible(flags.hover_on != 0);
        }
    }

    fn update_hover_from_mouse(&mut self, x: f32, y: f32) {
        let (origin, dir, target_y) = if let Some(renderer) = &self.renderer {
            let camera = &renderer.camera;
            let (origin, dir) = camera.screen_to_ray(
                x,
                y,
                renderer.device.surface_config.width as f32,
                renderer.device.surface_config.height as f32,
            );
            (origin, dir, camera.target.y)
        } else {
            return;
        };

        if dir.y.abs() <= 0.001 {
            return;
        }
        let t = (target_y - origin.y) / dir.y;
        if t < 0.0 {
            return;
        }
        let hit_x = origin.x + dir.x * t;
        let hit_z = origin.z + dir.z * t;

        let Some(gat) = self.map_data.as_ref().and_then(|m| m.gat.as_ref()) else {
            return;
        };
        let map = match self.map_data.as_ref() {
            Some(m) => m,
            None => return,
        };
        let zoom = map.gnd.zoom;
        let gnd_w = map.gnd.width;
        let gnd_h = map.gnd.height;
        let gat_w = gat.width;
        let gat_h = gat.height;
        let cell_w = (gnd_w as f32 / gat_w as f32) * zoom;
        let cell_h = (gnd_h as f32 / gat_h as f32) * zoom;

        let cx = (hit_x / cell_w) as i32;
        let cy = (hit_z / cell_h) as i32;

        let valid = cx >= 0 && cy >= 0 && cx < gat_w && cy < gat_h;
        if let Some(hot) = &self.hot_lib {
            hot.set_hover_cell(cx, cy, valid);
        }

        if !valid {
            return;
        }

        let cell_idx = (cy * gat_w + cx) as usize;
        if cell_idx >= gat.cells.len() {
            return;
        }
        let cell = &gat.cells[cell_idx];
        let corners = [
            [cx as f32 * cell_w, cell.height_sw, cy as f32 * cell_h],
            [(cx + 1) as f32 * cell_w, cell.height_se, cy as f32 * cell_h],
            [cx as f32 * cell_w, cell.height_nw, (cy + 1) as f32 * cell_h],
            [
                (cx + 1) as f32 * cell_w,
                cell.height_ne,
                (cy + 1) as f32 * cell_h,
            ],
        ];
        if let Some(renderer) = &mut self.renderer {
            let queue = &renderer.device.queue;
            if let Some(grid_sel) = &mut renderer.grid_selector {
                grid_sel.update_hover(queue, corners);
            }
        }
    }

    fn render_frame(&mut self) {
        self.check_hot_reload();

        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        // Tick dylib (paused state lives there too).
        if let Some(hot) = &self.hot_lib {
            hot.update(dt);
        }

        self.sync_camera_to_renderer();
        self.sync_flags_to_renderer();

        let paused = self
            .hot_lib
            .as_ref()
            .map(|h| h.get_flags().paused != 0)
            .unwrap_or(false);

        let Some(renderer) = &mut self.renderer else {
            return;
        };

        let width = renderer.device.surface_config.width as f32;
        let height = renderer.device.surface_config.height as f32;
        let elapsed = if paused { 0.0 } else { dt };

        let mut draw_calls: Vec<UiDrawCall> = Vec::new();
        if let Some(browser) = &self.browser {
            draw_calls.extend(browser.build_draw_calls(&renderer.font_atlas, width, height));
        } else if let Some(hot) = &self.hot_lib {
            hot.build_overlay(&renderer.font_atlas, width, height, &mut draw_calls);
        }

        renderer.render(&draw_calls, &[], &[], &[], elapsed);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("RSW Viewer")
            .with_inner_size(winit::dpi::PhysicalSize::new(1280u32, 800u32));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());

        let font_px_height = 14.0;
        let dpi_scale = 1.0;
        let mut renderer = block_on(ragnarok_renderer::Renderer::new(
            window.clone(),
            font_px_height,
            dpi_scale,
        ));

        let grf = match GrfArchive::open(Path::new(&self.grf_path)) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("Failed to open GRF '{}': {}", self.grf_path, e);
                event_loop.exit();
                return;
            }
        };

        renderer.try_load_grf_font(&grf);

        let initial_w = renderer.device.surface_config.width as f32;
        let initial_h = renderer.device.surface_config.height as f32;

        self.renderer = Some(renderer);
        self.window = Some(window);

        // Push initial viewport to dylib.
        if let Some(hot) = &self.hot_lib {
            hot.set_viewport(initial_w, initial_h);
        }

        if self.map_name.is_some() {
            self.load_map();
        } else {
            self.open_browser();
        }

        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                if let Some(browser) = &mut self.browser {
                    browser.update_visible_rows(size.height as f32);
                }
                if let Some(hot) = &self.hot_lib {
                    hot.set_viewport(size.width as f32, size.height as f32);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.ctrl_pressed = modifiers.state().control_key();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if self.browser.is_some() {
                    if event.state != winit::event::ElementState::Pressed {
                        return;
                    }
                    match &event.logical_key {
                        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::Tab) => {
                            if self.map_name.is_some() {
                                self.browser = None;
                            } else {
                                event_loop.exit();
                            }
                        }
                        Key::Named(NamedKey::Enter) => {
                            self.handle_browser_select();
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            if let Some(b) = &mut self.browser {
                                b.handle_up();
                            }
                        }
                        Key::Named(NamedKey::ArrowDown) => {
                            if let Some(b) = &mut self.browser {
                                b.handle_down();
                            }
                        }
                        Key::Named(NamedKey::PageUp) => {
                            if let Some(b) = &mut self.browser {
                                b.handle_page_up();
                            }
                        }
                        Key::Named(NamedKey::PageDown) => {
                            if let Some(b) = &mut self.browser {
                                b.handle_page_down();
                            }
                        }
                        Key::Named(NamedKey::Backspace) => {
                            if let Some(b) = &mut self.browser {
                                b.handle_backspace();
                            }
                        }
                        Key::Character(ch) => {
                            if self.ctrl_pressed && ch == "v" {
                                if let Ok(mut clipboard) = arboard::Clipboard::new()
                                    && let Ok(text) = clipboard.get_text()
                                    && let Some(b) = &mut self.browser
                                {
                                    b.handle_paste(&text);
                                }
                            } else if let Some(b) = &mut self.browser {
                                match ch.as_str() {
                                    "1" => b.switch_tab(BrowserTab::Towns),
                                    "2" => b.switch_tab(BrowserTab::Dungeons),
                                    "3" => b.switch_tab(BrowserTab::Fields),
                                    "4" => b.switch_tab(BrowserTab::Other),
                                    _ => {
                                        for c in ch.chars() {
                                            if !c.is_control() {
                                                b.handle_char(c);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    return;
                }

                if event.state == winit::event::ElementState::Pressed
                    && let Key::Character(ch) = &event.logical_key
                    && (ch == "b" || ch == "B")
                {
                    self.open_browser();
                    return;
                }
                if let Some(action) = controls::map_key_press(&event.logical_key, event.state)
                    && let Some(hot) = &self.hot_lib
                {
                    hot.on_action(action as u32);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.browser.is_some() {
                    return;
                }
                let dy = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, dy) => dy,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };
                if let Some(hot) = &self.hot_lib {
                    hot.on_mouse_wheel(dy);
                }
            }
            WindowEvent::MouseInput { button, state, .. } => match button {
                winit::event::MouseButton::Left => {
                    let pressed = state == winit::event::ElementState::Pressed;
                    self.mouse_down_left = pressed;
                    if pressed {
                        self.last_mouse = self.mouse_pos;
                        // Click-to-move: center the camera on the hovered cell.
                        if let Some(hot) = &self.hot_lib
                            && let Some((cx, cy)) = hot.get_hover_cell()
                            && let Some(coords) = self
                                .map_data
                                .as_ref()
                                .and_then(|m| m.coordinates.as_ref())
                        {
                            let (wx, _wy, wz) =
                                coords.cell_to_world(cx as f32 + 0.5, cy as f32 + 0.5);
                            hot.set_target(wx, 0.0, wz);
                        }
                    }
                }
                winit::event::MouseButton::Right => {
                    self.mouse_down_right = state == winit::event::ElementState::Pressed;
                    if self.mouse_down_right {
                        self.last_mouse = self.mouse_pos;
                    }
                }
                _ => {}
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = (position.x as f32, position.y as f32);

                // Hover raycast (host-side; pushes cell to dylib)
                self.update_hover_from_mouse(position.x as f32, position.y as f32);

                // Right-drag = orbit (left-click is reserved for click-to-move).
                if self.mouse_down_right {
                    let dx = self.mouse_pos.0 - self.last_mouse.0;
                    let dy = self.mouse_pos.1 - self.last_mouse.1;
                    if let Some(hot) = &self.hot_lib {
                        hot.on_mouse_drag(dx, dy, 0);
                    }
                    self.last_mouse = self.mouse_pos;
                }
            }
            WindowEvent::RedrawRequested => {
                self.render_frame();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

pub fn run(args: Args) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(args);
    event_loop.run_app(&mut app).unwrap();
}
