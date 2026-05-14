//! Effect viewer host. Owns the window, wgpu device, GRF, renderer, and the
//! `EffectHolder`. Delegates effect *selection* and *playback control* to a
//! hot-reloadable cdylib at `tools/effect-viewer-hot/`.
//!
//! Stage 1 (this iteration):
//!   * Window + wgpu init via `ragnarok_renderer::Renderer`
//!   * GRF + STR cache loading
//!   * cdylib loaded via libloading
//!   * cdylib's overlay (status + legend) renders each frame
//!   * Hot-reload on dylib mtime change
//!   * `hot_take_pending_spawn` → `EffectHolder::spawn` → STR rendered
//!     through the existing pipeline
//!
//! Stage 2 (later):
//!   * Time slider, speed control wired
//!   * Custom-effect primitive rendering once `lib/renderer/src/effect/fx/*`
//!     exists
//!   * Sidebar picker UI (instead of cycle key)
//!   * Camera orbit controls

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use ragnarok_formats::grf::GrfArchive;
use models::enums::EnumWithNumberValue;
use models::enums::EnumWithStringValue;
use models::enums::effect_id::EffectId;
use ragnarok_game::effect::{
    Attach, EffectQueue, EffectSpec, effect_spec, effect_texture_paths, str_aliases,
};

use crate::sprite_viewer::browser::SpriteBrowser;
use ragnarok_game::effect::EffectRenderCtx as GameEffectRenderCtx;
use ragnarok_renderer::effect::{
    EffectDrawList, EffectHolder, EffectRenderCtx, EffectUpdateCtx, ExternalCustomBackend,
    SpawnStatus, StrEffectCache, StrEmitterInput, build_billboard_batches,
    build_str_effect_batches,
};
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::{Renderer, UiDrawCall, block_on};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

pub struct Args {
    pub grf_path: String,
}

// === FFI types - must match `tools/effect-viewer-hot/src/lib.rs` exactly ===

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ViewerFlags {
    paused: u8,
    show_info: u8,
    _pad0: [u8; 2],
    speed_x100: u32,
    selected_effect_id: u16,
    _pad1: [u8; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct PendingSpawn {
    effect_id: u16,
    valid: u8,
    _pad: u8,
    world_pos: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CameraView {
    target: [f32; 3],
    yaw: f32,
    pitch: f32,
    distance: f32,
    fov_y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PersistentState {
    magic: u32,
    version: u32,
    selected_effect_id: u16,
    filter_idx: u16,
    paused: u8,
    show_info: u8,
    _pad: [u8; 2],
    speed_x100: u32,
    target: [f32; 3],
    yaw: f32,
    pitch: f32,
    distance: f32,
    fov_y: f32,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            magic: 0,
            version: 0,
            selected_effect_id: u16::MAX,
            filter_idx: 0,
            paused: 0,
            show_info: 0,
            _pad: [0; 2],
            speed_x100: 100,
            target: [0.0; 3],
            yaw: 0.0,
            pitch: 0.0,
            distance: 0.0,
            fov_y: 0.0,
        }
    }
}

const ACTION_NEXT_EFFECT: u32 = 1;
const ACTION_PREV_EFFECT: u32 = 2;
const ACTION_RESPAWN: u32 = 3;
const ACTION_TOGGLE_PAUSE: u32 = 4;
const ACTION_SPEED_UP: u32 = 5;
const ACTION_SPEED_DOWN: u32 = 6;
const ACTION_SHOW_CONTROLS: u32 = 7;
const ACTION_CLOSE_INFO_PANEL: u32 = 8;
const ACTION_RESET_CAMERA: u32 = 9;
const ACTION_PAGE_DOWN: u32 = 10;
const ACTION_PAGE_UP: u32 = 11;
const ACTION_HOME: u32 = 12;
const ACTION_END: u32 = 13;
const ACTION_NEXT_FILTER: u32 = 14;
const ACTION_PREV_FILTER: u32 = 15;
const ACTION_FOV_NARROWER: u32 = 16;
const ACTION_FOV_WIDER: u32 = 17;
const ACTION_STEP_FRAME: u32 = 18;

type HotCreateFn = extern "C" fn() -> *mut ();
type HotDestroyFn = unsafe extern "C" fn(*mut ());
type HotUpdateFn = unsafe extern "C" fn(*mut (), f32);
type HotOnActionFn = unsafe extern "C" fn(*mut (), u32);
type HotOnMouseWheelFn = unsafe extern "C" fn(*mut (), f32);
type HotOnMouseDragFn = unsafe extern "C" fn(*mut (), f32, f32, u8);
type HotGetFlagsFn = unsafe extern "C" fn(*mut (), *mut ViewerFlags);
type HotGetCameraFn = unsafe extern "C" fn(*mut (), *mut CameraView);
type HotTakePendingFn = unsafe extern "C" fn(*mut (), *mut PendingSpawn);
type HotTakeStepRequestFn = unsafe extern "C" fn(*mut ()) -> u8;
type HotSetLastStatusFn = unsafe extern "C" fn(*mut (), u8);
type HotBuildOverlayFn =
    unsafe extern "C" fn(*mut (), *const FontAtlas, f32, f32, *mut Vec<UiDrawCall>);
type HotGetFilteredIdsFn = unsafe extern "C" fn(*mut (), *mut Vec<u16>);
type HotSetSelectedEffectIdFn = unsafe extern "C" fn(*mut (), u16);
type HotSnapshotStateFn = unsafe extern "C" fn(*mut (), *mut PersistentState);
type HotRestoreStateFn = unsafe extern "C" fn(*mut (), *const PersistentState) -> u8;

// Effect-registry FFI (handle = 0 = invalid / spawn failed).
type HotSpawnCustomEffectFn =
    unsafe extern "C" fn(*mut (), u16, *const [f32; 3]) -> u64;
type HotUpdateCustomEffectFn = unsafe extern "C" fn(*mut (), u64, f32) -> u8;
type HotCollectCustomDrawsFn =
    unsafe extern "C" fn(*mut (), u64, *const EffectRenderCtxFfi, *mut EffectDrawList);
type HotDropCustomEffectFn = unsafe extern "C" fn(*mut (), u64);
type HotDropAllCustomEffectsFn = unsafe extern "C" fn(*mut ());

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct EffectRenderCtxFfi {
    eye: [f32; 3],
    target: [f32; 3],
    up: [f32; 3],
    screen_w: f32,
    screen_h: f32,
    elapsed: f32,
}

const STATUS_UNKNOWN: u8 = 0;
const STATUS_RENDERING: u8 = 1;
const STATUS_STR_FILE_MISSING: u8 = 2;
const STATUS_CUSTOM_NOT_IMPL: u8 = 3;
const STATUS_NO_SPEC: u8 = 4;
const STATUS_NOOP: u8 = 5;

fn spawn_status_to_u8(status: SpawnStatus) -> u8 {
    match status {
        SpawnStatus::Rendering => STATUS_RENDERING,
        SpawnStatus::StrFileMissing => STATUS_STR_FILE_MISSING,
        SpawnStatus::CustomNotImpl => STATUS_CUSTOM_NOT_IMPL,
        SpawnStatus::NoSpec => STATUS_NO_SPEC,
        SpawnStatus::Noop => STATUS_NOOP,
    }
}

struct HotLib {
    lib: Arc<libloading::Library>,
    state: *mut (),
    update_fn: HotUpdateFn,
    on_action_fn: HotOnActionFn,
    on_mouse_wheel_fn: HotOnMouseWheelFn,
    on_mouse_drag_fn: HotOnMouseDragFn,
    get_flags_fn: HotGetFlagsFn,
    get_camera_fn: HotGetCameraFn,
    take_pending_fn: HotTakePendingFn,
    take_step_request_fn: HotTakeStepRequestFn,
    set_last_status_fn: HotSetLastStatusFn,
    build_overlay_fn: HotBuildOverlayFn,
    get_filtered_ids_fn: HotGetFilteredIdsFn,
    set_selected_effect_id_fn: HotSetSelectedEffectIdFn,
    snapshot_state_fn: HotSnapshotStateFn,
    restore_state_fn: HotRestoreStateFn,
    destroy_fn: HotDestroyFn,
    spawn_custom_effect_fn: HotSpawnCustomEffectFn,
    update_custom_effect_fn: HotUpdateCustomEffectFn,
    collect_custom_draws_fn: HotCollectCustomDrawsFn,
    drop_custom_effect_fn: HotDropCustomEffectFn,
    drop_all_custom_effects_fn: HotDropAllCustomEffectsFn,
}

/// `ExternalCustomBackend` impl wired to a loaded `HotLib`. The Arc'd
/// `libloading::Library` is held here too so the dylib stays mapped for as
/// long as anything holds a reference to this backend — `EffectHolder::clear`
/// drops the backend before the host unloads the new dylib instance.
struct HotLibEffectBackend {
    _lib: Arc<libloading::Library>,
    state: *mut (),
    spawn_fn: HotSpawnCustomEffectFn,
    update_fn: HotUpdateCustomEffectFn,
    collect_fn: HotCollectCustomDrawsFn,
    drop_fn: HotDropCustomEffectFn,
    drop_all_fn: HotDropAllCustomEffectsFn,
    /// Tracks the `EffectId` each live cdylib handle was spawned with so
    /// `str_overlay` can be answered without FFI. `str_overlay` is a static
    /// property of the effect type — the in-process factory returns the same
    /// value as the cdylib's copy of the same source.
    handle_ids: std::sync::Mutex<std::collections::HashMap<u64, u16>>,
}

// SAFETY: every cdylib FFI entrypoint takes the `state` pointer and we never
// share &mut access across threads; the cdylib's internal effect registry is
// guarded by `Mutex<HashMap<...>>`. Both the host and the cdylib run their
// shared types under the same `#[global_allocator] = System` (the cdylib
// forces it).
unsafe impl Send for HotLibEffectBackend {}
unsafe impl Sync for HotLibEffectBackend {}

impl ExternalCustomBackend for HotLibEffectBackend {
    fn spawn(&self, effect_id: u16, world_pos: [f32; 3]) -> u64 {
        let handle = unsafe { (self.spawn_fn)(self.state, effect_id, &world_pos as *const _) };
        if handle != 0 {
            self.handle_ids
                .lock()
                .unwrap()
                .insert(handle, effect_id);
        }
        handle
    }

    fn update(&self, handle: u64, dt: f32) -> bool {
        let dead = unsafe { (self.update_fn)(self.state, handle, dt) };
        dead == 0
    }

    fn collect(
        &self,
        handle: u64,
        ctx: &GameEffectRenderCtx,
        out: &mut EffectDrawList,
    ) {
        let ffi = EffectRenderCtxFfi {
            eye: ctx.camera.eye,
            target: ctx.camera.target,
            up: ctx.camera.up,
            screen_w: ctx.screen_w,
            screen_h: ctx.screen_h,
            elapsed: ctx.elapsed,
        };
        unsafe {
            (self.collect_fn)(
                self.state,
                handle,
                &ffi as *const EffectRenderCtxFfi,
                out as *mut EffectDrawList,
            )
        };
    }

    fn str_overlay(&self, handle: u64) -> Option<String> {
        // Translate handle → effect_id → factory probe → str_overlay name.
        // str_overlay is a static property of the effect type, so the
        // in-process factory's answer matches the cdylib's.
        let effect_id_u16 = *self.handle_ids.lock().unwrap().get(&handle)?;
        let effect_id = EffectId::try_from_value(effect_id_u16 as usize).ok()?;
        let probe = ragnarok_game::effect::factory::make_effect(
            effect_id,
            Attach::WorldPos([0.0, 0.0, 0.0]),
        )?;
        probe.str_overlay().map(|s| s.to_string())
    }

    fn drop_handle(&self, handle: u64) {
        self.handle_ids.lock().unwrap().remove(&handle);
        unsafe { (self.drop_fn)(self.state, handle) };
    }

    fn drop_all(&self) {
        self.handle_ids.lock().unwrap().clear();
        unsafe { (self.drop_all_fn)(self.state) };
    }
}

impl HotLib {
    fn load(dylib_path: &Path) -> Option<Self> {
        let lib = match unsafe { libloading::Library::new(dylib_path) } {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to load dylib {}: {e}", dylib_path.display());
                return None;
            }
        };
        unsafe {
            let create_fn = *lib.get::<HotCreateFn>(b"hot_create").ok()?;
            let destroy_fn = *lib.get::<HotDestroyFn>(b"hot_destroy").ok()?;
            let update_fn = *lib.get::<HotUpdateFn>(b"hot_update").ok()?;
            let on_action_fn = *lib.get::<HotOnActionFn>(b"hot_on_action").ok()?;
            let on_mouse_wheel_fn =
                *lib.get::<HotOnMouseWheelFn>(b"hot_on_mouse_wheel").ok()?;
            let on_mouse_drag_fn =
                *lib.get::<HotOnMouseDragFn>(b"hot_on_mouse_drag").ok()?;
            let get_flags_fn = *lib.get::<HotGetFlagsFn>(b"hot_get_flags").ok()?;
            let get_camera_fn = *lib.get::<HotGetCameraFn>(b"hot_get_camera").ok()?;
            let take_pending_fn =
                *lib.get::<HotTakePendingFn>(b"hot_take_pending_spawn").ok()?;
            let take_step_request_fn = *lib
                .get::<HotTakeStepRequestFn>(b"hot_take_step_request")
                .ok()?;
            let set_last_status_fn =
                *lib.get::<HotSetLastStatusFn>(b"hot_set_last_status").ok()?;
            let build_overlay_fn =
                *lib.get::<HotBuildOverlayFn>(b"hot_build_overlay").ok()?;
            let get_filtered_ids_fn =
                *lib.get::<HotGetFilteredIdsFn>(b"hot_get_filtered_ids").ok()?;
            let set_selected_effect_id_fn = *lib
                .get::<HotSetSelectedEffectIdFn>(b"hot_set_selected_effect_id")
                .ok()?;
            let snapshot_state_fn =
                *lib.get::<HotSnapshotStateFn>(b"hot_snapshot_state").ok()?;
            let restore_state_fn =
                *lib.get::<HotRestoreStateFn>(b"hot_restore_state").ok()?;
            let spawn_custom_effect_fn = *lib
                .get::<HotSpawnCustomEffectFn>(b"hot_spawn_custom_effect")
                .ok()?;
            let update_custom_effect_fn = *lib
                .get::<HotUpdateCustomEffectFn>(b"hot_update_custom_effect")
                .ok()?;
            let collect_custom_draws_fn = *lib
                .get::<HotCollectCustomDrawsFn>(b"hot_collect_custom_draws")
                .ok()?;
            let drop_custom_effect_fn = *lib
                .get::<HotDropCustomEffectFn>(b"hot_drop_custom_effect")
                .ok()?;
            let drop_all_custom_effects_fn = *lib
                .get::<HotDropAllCustomEffectsFn>(b"hot_drop_all_custom_effects")
                .ok()?;
            let state = (create_fn)();
            Some(Self {
                lib: Arc::new(lib),
                state,
                update_fn,
                on_action_fn,
                on_mouse_wheel_fn,
                on_mouse_drag_fn,
                get_flags_fn,
                get_camera_fn,
                take_pending_fn,
                take_step_request_fn,
                set_last_status_fn,
                build_overlay_fn,
                get_filtered_ids_fn,
                set_selected_effect_id_fn,
                snapshot_state_fn,
                restore_state_fn,
                destroy_fn,
                spawn_custom_effect_fn,
                update_custom_effect_fn,
                collect_custom_draws_fn,
                drop_custom_effect_fn,
                drop_all_custom_effects_fn,
            })
        }
    }

    /// Build an `ExternalCustomBackend` that routes custom-effect dispatch
    /// through this dylib. The returned Arc holds a reference to the
    /// `libloading::Library` so the dylib stays mapped for the backend's
    /// lifetime — drop the Arc before unloading.
    fn make_effect_backend(&self) -> Arc<HotLibEffectBackend> {
        Arc::new(HotLibEffectBackend {
            _lib: Arc::clone(&self.lib),
            state: self.state,
            spawn_fn: self.spawn_custom_effect_fn,
            update_fn: self.update_custom_effect_fn,
            collect_fn: self.collect_custom_draws_fn,
            drop_fn: self.drop_custom_effect_fn,
            drop_all_fn: self.drop_all_custom_effects_fn,
            handle_ids: std::sync::Mutex::new(std::collections::HashMap::new()),
        })
    }

    fn set_last_status(&self, status: u8) {
        unsafe { (self.set_last_status_fn)(self.state, status) };
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

    fn on_mouse_wheel(&self, dy: f32) {
        unsafe { (self.on_mouse_wheel_fn)(self.state, dy) };
    }

    fn on_mouse_drag(&self, dx: f32, dy: f32, button: u8) {
        unsafe { (self.on_mouse_drag_fn)(self.state, dx, dy, button) };
    }

    fn get_filtered_ids(&self) -> Vec<u16> {
        let mut out: Vec<u16> = Vec::new();
        unsafe { (self.get_filtered_ids_fn)(self.state, &mut out) };
        out
    }

    fn set_selected_effect_id(&self, id: u16) {
        unsafe { (self.set_selected_effect_id_fn)(self.state, id) };
    }

    fn snapshot_state(&self) -> PersistentState {
        let mut out = PersistentState::default();
        unsafe { (self.snapshot_state_fn)(self.state, &mut out) };
        out
    }

    fn restore_state(&self, snap: &PersistentState) -> bool {
        let ok = unsafe { (self.restore_state_fn)(self.state, snap as *const _) };
        ok != 0
    }

    fn camera(&self) -> CameraView {
        let mut out = CameraView::default();
        unsafe { (self.get_camera_fn)(self.state, &mut out) };
        out
    }

    fn flags(&self) -> ViewerFlags {
        let mut out = ViewerFlags::default();
        unsafe { (self.get_flags_fn)(self.state, &mut out) };
        out
    }

    fn take_step_request(&self) -> bool {
        unsafe { (self.take_step_request_fn)(self.state) != 0 }
    }

    fn take_pending_spawn(&self) -> Option<(EffectId, [f32; 3])> {
        let mut out = PendingSpawn::default();
        unsafe { (self.take_pending_fn)(self.state, &mut out) };
        if out.valid == 0 {
            return None;
        }
        let id = effect_id_from_u16(out.effect_id)?;
        Some((id, out.world_pos))
    }

    fn build_overlay(
        &self,
        atlas: &FontAtlas,
        screen_w: f32,
        screen_h: f32,
        out: &mut Vec<UiDrawCall>,
    ) {
        unsafe {
            (self.build_overlay_fn)(self.state, atlas as *const FontAtlas, screen_w, screen_h, out)
        };
    }
}

fn effect_id_from_u16(value: u16) -> Option<EffectId> {
    EffectId::try_from_value(value as usize).ok()
}

fn find_dylib() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest_dir.parent().unwrap().join("target").join("debug");
    #[cfg(target_os = "linux")]
    let name = "libeffect_viewer_hot.so";
    #[cfg(target_os = "macos")]
    let name = "libeffect_viewer_hot.dylib";
    #[cfg(target_os = "windows")]
    let name = "effect_viewer_hot.dll";
    target_dir.join(name)
}

fn dylib_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    grf: Option<GrfArchive>,
    grf_path: String,
    str_effects: StrEffectCache,
    effect_holder: EffectHolder,
    effect_queue: EffectQueue,
    /// 1x1 white bind group, owned by App (not the renderer) so it can be
    /// referenced from sprite batches without conflicting with the mutable
    /// borrow taken by `Renderer::render`.
    white_bind_group: Option<wgpu::BindGroup>,
    last_frame: Instant,
    hot_lib: Option<HotLib>,
    dylib_path: PathBuf,
    last_dylib_mtime: SystemTime,
    reload_counter: u64,
    /// STR file names already requested through the cache (failures included)
    /// so we don't retry every frame.
    attempted_str_files: std::collections::HashSet<String>,
    /// Effect picker overlay reusing the sprite-viewer browser UI. Populated
    /// each time the user opens it (Tab) from the dylib's current filter, so
    /// it always reflects whatever family is active.
    browser: Option<SpriteBrowser>,
    /// Maps the browser's displayed strings back to `EffectId` so a pick
    /// resolves to the original effect (browser sorts items alphabetically).
    browser_lookup: HashMap<String, EffectId>,
    ctrl_pressed: bool,
    mouse_pos: (f32, f32),
    last_mouse: (f32, f32),
    mouse_down_right: bool,
}

impl App {
    fn new(args: Args) -> Self {
        let dylib_path = find_dylib();
        let last_dylib_mtime = dylib_mtime(&dylib_path).unwrap_or(SystemTime::UNIX_EPOCH);
        let hot_lib = HotLib::load(&dylib_path);
        let mut effect_holder = EffectHolder::new();
        // Route custom-effect dispatch through the dylib so an edit to
        // `effects/*.rs` only requires rebuilding `effect-viewer-hot` to take
        // effect (the host's `make_effect` symbol is unused once the backend
        // is installed).
        if let Some(hot) = &hot_lib {
            effect_holder.set_external_backend(Some(hot.make_effect_backend()));
        }
        Self {
            window: None,
            renderer: None,
            grf: None,
            grf_path: args.grf_path,
            str_effects: StrEffectCache::new(),
            effect_holder,
            effect_queue: EffectQueue::new(),
            white_bind_group: None,
            last_frame: Instant::now(),
            hot_lib,
            dylib_path,
            last_dylib_mtime,
            reload_counter: 0,
            attempted_str_files: std::collections::HashSet::new(),
            browser: None,
            browser_lookup: HashMap::new(),
            ctrl_pressed: false,
            mouse_pos: (0.0, 0.0),
            last_mouse: (0.0, 0.0),
            mouse_down_right: false,
        }
    }

    fn browser_is_open(&self) -> bool {
        self.browser.as_ref().is_some_and(|b| b.open)
    }

    fn open_browser(&mut self) {
        let Some(hot) = &self.hot_lib else { return };
        let ids = hot.get_filtered_ids();
        self.browser_lookup.clear();
        let mut items: Vec<String> = Vec::with_capacity(ids.len());
        for raw in ids {
            let Some(id) = effect_id_from_u16(raw) else {
                continue;
            };
            let label = format!("{:?} ({}) [{}]", id, id.as_str(), raw);
            self.browser_lookup.insert(label.clone(), id);
            items.push(label);
        }
        let mut browser = SpriteBrowser::new(items, "effects");
        if let Some(renderer) = &self.renderer {
            let h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
            browser.update_visible_rows(h);
        }
        self.browser = Some(browser);
    }

    fn handle_browser_key(&mut self, key: &Key) {
        let ctrl = self.ctrl_pressed;
        let Some(browser) = &mut self.browser else {
            return;
        };
        match key.as_ref() {
            Key::Named(NamedKey::Escape) | Key::Named(NamedKey::Tab) => {
                browser.open = false;
            }
            Key::Named(NamedKey::Enter) => {
                self.handle_browser_select();
            }
            Key::Named(NamedKey::ArrowUp) => browser.handle_up(),
            Key::Named(NamedKey::ArrowDown) => browser.handle_down(),
            Key::Named(NamedKey::PageUp) => browser.handle_page_up(),
            Key::Named(NamedKey::PageDown) => browser.handle_page_down(),
            Key::Named(NamedKey::Backspace) => browser.handle_backspace(),
            Key::Character(ch) => {
                if ctrl && ch == "v" {
                    if let Ok(mut clipboard) = arboard::Clipboard::new()
                        && let Ok(text) = clipboard.get_text()
                    {
                        browser.handle_paste(&text);
                    }
                    return;
                }
                for c in ch.chars() {
                    if !c.is_control() {
                        browser.handle_char(c);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_browser_select(&mut self) {
        let Some(browser) = &mut self.browser else {
            return;
        };
        let Some(selected) = browser.selected_item().map(|s| s.to_string()) else {
            return;
        };
        browser.open = false;
        let Some(&id) = self.browser_lookup.get(&selected) else {
            return;
        };
        if let Some(hot) = &self.hot_lib {
            hot.set_selected_effect_id(id.value() as u16);
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
        let tmp = self
            .dylib_path
            .with_extension(format!("hot{}.so", self.reload_counter));
        if std::fs::copy(&self.dylib_path, &tmp).is_err() {
            eprintln!("Failed to copy dylib to temp file");
            return;
        }
        eprintln!("Reloading dylib...");
        let saved_state = self.hot_lib.as_ref().map(|h| h.snapshot_state());
        // Drop every live external-backend effect BEFORE we tear down the old
        // dylib — otherwise the registry's vtables would point at unmapped
        // code. `set_external_backend(None)` also calls `drop_all` on the old
        // backend, so the cdylib's HashMap is empty before we drop the Arc.
        self.effect_holder.set_external_backend(None);
        if let Some(old) = self.hot_lib.take() {
            old.unload();
        }
        match HotLib::load(&tmp) {
            Some(new) => {
                if let Some(snap) = &saved_state {
                    new.restore_state(snap);
                }
                self.effect_holder
                    .set_external_backend(Some(new.make_effect_backend()));
                self.hot_lib = Some(new);
                eprintln!("Reload complete.");
            }
            None => {
                eprintln!("Failed to load new dylib; falling back to original.");
                self.hot_lib = HotLib::load(&self.dylib_path);
                if let Some(hot) = &self.hot_lib {
                    if let Some(snap) = &saved_state {
                        hot.restore_state(snap);
                    }
                    self.effect_holder
                        .set_external_backend(Some(hot.make_effect_backend()));
                }
            }
        }
        if self.reload_counter > 1 {
            let prev = self
                .dylib_path
                .with_extension(format!("hot{}.so", self.reload_counter - 1));
            let _ = std::fs::remove_file(prev);
        }
    }

    fn poll_pending_spawn(&mut self) {
        let Some(hot) = &self.hot_lib else { return };
        let Some((effect_id, pos)) = hot.take_pending_spawn() else {
            return;
        };
        // Viewer convention: each picker spawn is the user asking to see
        // *just* this effect. Clear anything still alive so persistent
        // effects (Aura) don't pile up as we cycle through the list.
        self.effect_holder.clear();
        // Lazy-load the STR file for this effect, if any, before pushing
        // the spawn. The holder won't actually render anything until its
        // bind groups are present in the cache.
        self.ensure_str_loaded_for(effect_id);
        self.effect_queue
            .spawn_at(effect_id, [pos[0], pos[1], pos[2]]);
    }

    /// Make sure any STR file the effect needs is in the cache. Covers two
    /// cases:
    ///   * `EffectSpec::Str` — load the named file directly.
    ///   * `EffectSpec::Custom` — query the factory for a throwaway
    ///     instance; if it declares an `str_overlay()`, load that file too
    ///     (hybrid effects like Stormgust). The throwaway instance is
    ///     dropped immediately; the real spawn happens via the cdylib path.
    /// Tries the primary name first, then known aliases.
    /// Failures are remembered so we don't retry every cycle.
    fn ensure_str_loaded_for(&mut self, id: EffectId) {
        let file: &'static str = match effect_spec(id) {
            Some(EffectSpec::Str { file, .. }) => file,
            Some(EffectSpec::Custom { .. }) => {
                let probe = ragnarok_game::effect::factory::make_effect(
                    id,
                    Attach::WorldPos([0.0, 0.0, 0.0]),
                );
                let Some(probe) = probe else { return };
                let Some(overlay) = probe.str_overlay() else { return };
                overlay
            }
            _ => return,
        };
        if self.attempted_str_files.contains(file) {
            return;
        }
        self.attempted_str_files.insert(file.to_string());
        let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) else {
            return;
        };
        let aliases = str_aliases(id);
        let fallbacks = if aliases.first().copied() == Some(file) {
            &aliases[1..]
        } else {
            aliases
        };
        self.str_effects.load(
            file,
            fallbacks,
            grf,
            &mut renderer.texture_cache,
            &renderer.device.device,
            &renderer.device.queue,
        );
    }

    fn render_frame(&mut self) {
        self.check_hot_reload();
        self.poll_pending_spawn();

        let now = Instant::now();
        let raw_dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        let dt = raw_dt.min(0.1);

        let flags = self
            .hot_lib
            .as_ref()
            .map(|h| h.flags())
            .unwrap_or_default();
        let speed = (flags.speed_x100 as f32 / 100.0).max(0.0);
        let paused = flags.paused != 0;
        let step = paused
            && self
                .hot_lib
                .as_ref()
                .is_some_and(|h| h.take_step_request());
        let scaled_dt = if paused {
            if step { 1.0 / 60.0 } else { 0.0 }
        } else {
            dt * speed
        };

        if let Some(hot) = &self.hot_lib {
            hot.update(scaled_dt);
        }

        // Drain spawn requests into the holder, then tick it.
        self.effect_holder.drain_queue(&mut self.effect_queue);
        self.effect_holder.update(&EffectUpdateCtx { delta: scaled_dt });

        let status_code = self
            .effect_holder
            .last_spawn_status(|name| self.str_effects.get(name).is_some())
            .map(spawn_status_to_u8)
            .unwrap_or(STATUS_UNKNOWN);
        if let Some(hot) = &self.hot_lib {
            hot.set_last_status(status_code);
        }

        let Some(renderer) = &mut self.renderer else {
            return;
        };

        // Sync cdylib camera state onto the renderer's camera. The effect
        // viewer carries its own `fov_y` so it doesn't inherit the map
        // camera's narrow telephoto default.
        if let Some(hot) = &self.hot_lib {
            let v = hot.camera();
            renderer.camera.target = glam::Vec3::from(v.target);
            renderer.camera.yaw = v.yaw;
            renderer.camera.pitch = v.pitch;
            renderer.camera.distance = v.distance;
            if v.fov_y > 0.0 {
                renderer.camera.fov_y = v.fov_y;
            }
        }

        let screen_w = renderer.device.surface_config.width as f32 / renderer.dpi_scale;
        let screen_h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;

        // STR snapshots → emitter inputs.
        let snapshots = self.effect_holder.collect_str_emitters(&|_| None);
        let str_inputs: Vec<StrEmitterInput<'_>> = snapshots
            .iter()
            .map(|s| StrEmitterInput {
                str_name: &s.name,
                position: s.position,
                anim_time: s.anim_time,
            })
            .collect();
        let mut effect_batches = build_str_effect_batches(
            &str_inputs,
            &self.str_effects,
            &renderer.camera,
            screen_w,
            screen_h,
        );

        // Custom-effect primitives (currently: Aura). Build a draw list from
        // every active custom effect, then turn the list into sprite batches
        // via the billboard primitive renderer.
        let mut effect_draws = EffectDrawList::new();
        let render_ctx = EffectRenderCtx {
            camera: ragnarok_game::effect::CameraView {
                eye: renderer.camera.eye().to_array(),
                target: renderer.camera.target.to_array(),
                up: glam::Vec3::NEG_Y.to_array(),
            },
            screen_w,
            screen_h,
            elapsed: 0.0,
        };
        self.effect_holder
            .collect_custom_draws(&mut effect_draws, &render_ctx);
        if let Some(fallback) = &self.white_bind_group {
            let mut primitive_batches = build_billboard_batches(
                &effect_draws,
                &renderer.camera,
                screen_w,
                screen_h,
                fallback,
                // Named-texture lookup intentionally returns None for now -
                // Aura uses the fallback. Wire up named GRF textures here
                // when fx/* effects need them.
                |_name| None,
            );
            effect_batches.append(&mut primitive_batches);
        }

        // cdylib overlay (status + legend + controls). Browser, when open,
        // is drawn on top by the host.
        let mut ui_calls: Vec<UiDrawCall> = Vec::new();
        if let Some(hot) = &self.hot_lib {
            hot.build_overlay(&renderer.font_atlas, screen_w, screen_h, &mut ui_calls);
        }
        if let Some(browser) = &self.browser
            && browser.open
        {
            ui_calls.extend(browser.build_draw_calls(&renderer.font_atlas, screen_w, screen_h));
        }

        renderer.render(&ui_calls, &effect_batches, &effect_draws, &[], &[], &[], 0.0);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("Effect Viewer")
            .with_inner_size(winit::dpi::PhysicalSize::new(1024u32, 768u32));
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let renderer = block_on(Renderer::new(window.clone(), 14.0, 1.0));

        // App-owned 1x1 white bind group for the billboard primitive
        // fallback. See `App::white_bind_group` doc for why this is on App
        // rather than reusing `renderer.white_bind_group`.
        let white_img = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]));
        let white_bind_group = ragnarok_renderer::texture::create_texture_bind_group(
            &renderer.device.device,
            &renderer.device.queue,
            &white_img,
            &renderer.texture_cache.bind_group_layout,
            "effect_viewer_white",
        );

        match GrfArchive::open(Path::new(&self.grf_path)) {
            Ok(grf) => self.grf = Some(grf),
            Err(e) => {
                eprintln!("Failed to open GRF {}: {e}", self.grf_path);
                event_loop.exit();
                return;
            }
        }

        let mut renderer = renderer;
        if let Some(grf) = &self.grf {
            let paths = effect_texture_paths();
            renderer.preload_effect_textures(&paths, grf);
        }

        self.renderer = Some(renderer);
        self.white_bind_group = Some(white_bind_group);
        self.window = Some(window);
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    // Renderer::resize also resizes the UI + sprite pipelines'
                    // viewports - without it the legend / overlay positions
                    // computed from `screen_h - …` end up off-screen after
                    // winit's initial Resized event.
                    renderer.resize(size.width, size.height);
                }
                if let (Some(browser), Some(renderer)) = (&mut self.browser, &self.renderer) {
                    let h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
                    browser.update_visible_rows(h);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                if self.browser_is_open() {
                    self.handle_browser_key(&event.logical_key);
                    return;
                }
                // Esc closes any open info panel first; only quits if no panel is open.
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    let panel_open = self
                        .hot_lib
                        .as_ref()
                        .map(|h| h.flags().show_info != 0)
                        .unwrap_or(false);
                    if panel_open {
                        if let Some(hot) = &self.hot_lib {
                            hot.on_action(ACTION_CLOSE_INFO_PANEL);
                        }
                    } else {
                        event_loop.exit();
                    }
                    return;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Tab)) {
                    self.open_browser();
                    return;
                }
                if matches!(event.logical_key.as_ref(), Key::Character("b") | Key::Character("B"))
                {
                    if let Some(renderer) = &mut self.renderer {
                        let blue = wgpu::Color {
                            r: 0.392,
                            g: 0.584,
                            b: 0.929,
                            a: 1.0,
                        };
                        renderer.clear_color = if renderer.clear_color == wgpu::Color::BLACK {
                            blue
                        } else {
                            wgpu::Color::BLACK
                        };
                    }
                    return;
                }
                let action = match event.logical_key.as_ref() {
                    Key::Named(NamedKey::Space) => Some(ACTION_TOGGLE_PAUSE),
                    Key::Named(NamedKey::ArrowRight) => Some(ACTION_NEXT_EFFECT),
                    Key::Named(NamedKey::ArrowLeft) => Some(ACTION_PREV_EFFECT),
                    Key::Named(NamedKey::ArrowDown) => Some(ACTION_NEXT_FILTER),
                    Key::Named(NamedKey::ArrowUp) => Some(ACTION_PREV_FILTER),
                    Key::Named(NamedKey::PageDown) => Some(ACTION_PAGE_DOWN),
                    Key::Named(NamedKey::PageUp) => Some(ACTION_PAGE_UP),
                    Key::Named(NamedKey::Home) => Some(ACTION_HOME),
                    Key::Named(NamedKey::End) => Some(ACTION_END),
                    Key::Character(c) => match c {
                        "r" | "R" => Some(ACTION_RESPAWN),
                        "n" | "N" => Some(ACTION_STEP_FRAME),
                        "+" | "=" => Some(ACTION_SPEED_UP),
                        "-" | "_" => Some(ACTION_SPEED_DOWN),
                        "c" | "C" => Some(ACTION_RESET_CAMERA),
                        "1" => Some(ACTION_SHOW_CONTROLS),
                        "[" | "{" => Some(ACTION_FOV_NARROWER),
                        "]" | "}" => Some(ACTION_FOV_WIDER),
                        _ => None,
                    },
                    _ => None,
                };
                if let (Some(code), Some(hot)) = (action, &self.hot_lib) {
                    hot.on_action(code);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.ctrl_pressed = modifiers.state().control_key();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => (p.y as f32) / 20.0,
                };
                if let Some(hot) = &self.hot_lib {
                    hot.on_mouse_wheel(dy);
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if button == winit::event::MouseButton::Right {
                    self.mouse_down_right = state == ElementState::Pressed;
                    if self.mouse_down_right {
                        self.last_mouse = self.mouse_pos;
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = (position.x as f32, position.y as f32);
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

impl Drop for App {
    fn drop(&mut self) {
        if let Some(hot) = self.hot_lib.take() {
            hot.unload();
        }
    }
}
