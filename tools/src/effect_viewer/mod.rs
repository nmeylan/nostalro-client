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
use ragnarok_game::effect::{
    EffectId, EffectQueue, EffectSpec, effect_ef_name, effect_name, effect_spec,
    effect_texture_paths, str_aliases,
};

use crate::sprite_viewer::browser::SpriteBrowser;
use ragnarok_renderer::effect::{
    EffectDrawList, EffectHolder, EffectRenderCtx, EffectUpdateCtx, SpawnStatus, StrEffectCache,
    StrEmitterInput, build_billboard_batches, build_str_effect_batches,
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

type HotCreateFn = extern "C" fn() -> *mut ();
type HotDestroyFn = unsafe extern "C" fn(*mut ());
type HotUpdateFn = unsafe extern "C" fn(*mut (), f32);
type HotOnActionFn = unsafe extern "C" fn(*mut (), u32);
type HotOnMouseWheelFn = unsafe extern "C" fn(*mut (), f32);
type HotGetFlagsFn = unsafe extern "C" fn(*mut (), *mut ViewerFlags);
type HotGetCameraFn = unsafe extern "C" fn(*mut (), *mut CameraView);
type HotTakePendingFn = unsafe extern "C" fn(*mut (), *mut PendingSpawn);
type HotSetLastStatusFn = unsafe extern "C" fn(*mut (), u8);
type HotBuildOverlayFn =
    unsafe extern "C" fn(*mut (), *const FontAtlas, f32, f32, *mut Vec<UiDrawCall>);
type HotGetFilteredIdsFn = unsafe extern "C" fn(*mut (), *mut Vec<u16>);
type HotSetSelectedEffectIdFn = unsafe extern "C" fn(*mut (), u16);

const STATUS_UNKNOWN: u8 = 0;
const STATUS_RENDERING: u8 = 1;
const STATUS_STR_FILE_MISSING: u8 = 2;
const STATUS_CUSTOM_NOT_IMPL: u8 = 3;
const STATUS_NO_SPEC: u8 = 4;
const STATUS_CUSTOM_TEXTURE_MISSING: u8 = 5;

fn spawn_status_to_u8(status: SpawnStatus) -> u8 {
    match status {
        SpawnStatus::Rendering => STATUS_RENDERING,
        SpawnStatus::StrFileMissing => STATUS_STR_FILE_MISSING,
        SpawnStatus::CustomTextureMissing => STATUS_CUSTOM_TEXTURE_MISSING,
        SpawnStatus::CustomNotImpl => STATUS_CUSTOM_NOT_IMPL,
        SpawnStatus::NoSpec => STATUS_NO_SPEC,
    }
}

struct HotLib {
    _lib: libloading::Library,
    state: *mut (),
    update_fn: HotUpdateFn,
    on_action_fn: HotOnActionFn,
    on_mouse_wheel_fn: HotOnMouseWheelFn,
    get_flags_fn: HotGetFlagsFn,
    get_camera_fn: HotGetCameraFn,
    take_pending_fn: HotTakePendingFn,
    set_last_status_fn: HotSetLastStatusFn,
    build_overlay_fn: HotBuildOverlayFn,
    get_filtered_ids_fn: HotGetFilteredIdsFn,
    set_selected_effect_id_fn: HotSetSelectedEffectIdFn,
    destroy_fn: HotDestroyFn,
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
        let (
            create,
            destroy,
            update,
            on_action,
            on_wheel,
            get_flags,
            get_camera,
            take_pending,
            set_last_status,
            build_overlay,
            get_filtered_ids,
            set_selected_effect_id,
        ) = unsafe {
            let c: libloading::Symbol<HotCreateFn> = lib.get(b"hot_create").ok()?;
            let d: libloading::Symbol<HotDestroyFn> = lib.get(b"hot_destroy").ok()?;
            let u: libloading::Symbol<HotUpdateFn> = lib.get(b"hot_update").ok()?;
            let a: libloading::Symbol<HotOnActionFn> = lib.get(b"hot_on_action").ok()?;
            let w: libloading::Symbol<HotOnMouseWheelFn> = lib.get(b"hot_on_mouse_wheel").ok()?;
            let f: libloading::Symbol<HotGetFlagsFn> = lib.get(b"hot_get_flags").ok()?;
            let cam: libloading::Symbol<HotGetCameraFn> = lib.get(b"hot_get_camera").ok()?;
            let t: libloading::Symbol<HotTakePendingFn> = lib.get(b"hot_take_pending_spawn").ok()?;
            let s: libloading::Symbol<HotSetLastStatusFn> = lib.get(b"hot_set_last_status").ok()?;
            let b: libloading::Symbol<HotBuildOverlayFn> = lib.get(b"hot_build_overlay").ok()?;
            let gf: libloading::Symbol<HotGetFilteredIdsFn> =
                lib.get(b"hot_get_filtered_ids").ok()?;
            let ss: libloading::Symbol<HotSetSelectedEffectIdFn> =
                lib.get(b"hot_set_selected_effect_id").ok()?;
            (*c, *d, *u, *a, *w, *f, *cam, *t, *s, *b, *gf, *ss)
        };
        let state = (create)();
        Some(Self {
            _lib: lib,
            state,
            update_fn: update,
            on_action_fn: on_action,
            on_mouse_wheel_fn: on_wheel,
            get_flags_fn: get_flags,
            get_camera_fn: get_camera,
            take_pending_fn: take_pending,
            set_last_status_fn: set_last_status,
            build_overlay_fn: build_overlay,
            get_filtered_ids_fn: get_filtered_ids,
            set_selected_effect_id_fn: set_selected_effect_id,
            destroy_fn: destroy,
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

    fn get_filtered_ids(&self) -> Vec<u16> {
        let mut out: Vec<u16> = Vec::new();
        unsafe { (self.get_filtered_ids_fn)(self.state, &mut out) };
        out
    }

    fn set_selected_effect_id(&self, id: u16) {
        unsafe { (self.set_selected_effect_id_fn)(self.state, id) };
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

/// Reverse-lookup a u16 onto an `EffectId` variant. Delegates to the
/// generated table (821 variants, sparse discriminants up to 2027).
fn effect_id_from_u16(value: u16) -> Option<EffectId> {
    EffectId::from_u16(value)
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
}

impl App {
    fn new(args: Args) -> Self {
        let dylib_path = find_dylib();
        let last_dylib_mtime = dylib_mtime(&dylib_path).unwrap_or(SystemTime::UNIX_EPOCH);
        let hot_lib = HotLib::load(&dylib_path);
        Self {
            window: None,
            renderer: None,
            grf: None,
            grf_path: args.grf_path,
            str_effects: StrEffectCache::new(),
            effect_holder: EffectHolder::new(),
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
            let label = format!("{} ({}) [{}]", effect_name(id), effect_ef_name(id), raw);
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
            hot.set_selected_effect_id(id.as_u16());
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
        if let Some(old) = self.hot_lib.take() {
            old.unload();
        }
        match HotLib::load(&tmp) {
            Some(new) => {
                self.hot_lib = Some(new);
                eprintln!("Reload complete.");
            }
            None => {
                eprintln!("Failed to load new dylib; falling back to original.");
                self.hot_lib = HotLib::load(&self.dylib_path);
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

    /// If `effect_id` resolves to an `EffectSpec::Str` or `StrHybrid`, make
    /// sure its STR file is in the cache. Tries the primary name first, then
    /// auto-derived aliases. Failures are remembered so we don't retry
    /// every cycle.
    fn ensure_str_loaded_for(&mut self, id: EffectId) {
        let file = match effect_spec(id) {
            Some(EffectSpec::Str { file, .. }) => file,
            Some(EffectSpec::StrHybrid { file, .. }) => file,
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
        let scaled_dt = if paused { 0.0 } else { dt * speed };

        if let Some(hot) = &self.hot_lib {
            hot.update(scaled_dt);
        }

        // Drain spawn requests into the holder, then tick it.
        self.effect_holder.drain_queue(&mut self.effect_queue);
        self.effect_holder.update(&EffectUpdateCtx { dt: scaled_dt });

        let status_code = self
            .effect_holder
            .last_spawn_status(
                |name| self.str_effects.get(name).is_some(),
                |name| {
                    self.renderer
                        .as_ref()
                        .map(|r| r.texture_cache.get(name).is_some())
                        .unwrap_or(false)
                },
            )
            .map(spawn_status_to_u8)
            .unwrap_or(STATUS_UNKNOWN);
        if let Some(hot) = &self.hot_lib {
            hot.set_last_status(status_code);
        }

        let Some(renderer) = &mut self.renderer else {
            return;
        };

        // Sync cdylib camera state onto the renderer's camera.
        if let Some(hot) = &self.hot_lib {
            let v = hot.camera();
            renderer.camera.target = glam::Vec3::from(v.target);
            renderer.camera.yaw = v.yaw;
            renderer.camera.pitch = v.pitch;
            renderer.camera.distance = v.distance;
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
            camera: &renderer.camera,
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
                        "+" | "=" => Some(ACTION_SPEED_UP),
                        "-" | "_" => Some(ACTION_SPEED_DOWN),
                        "c" | "C" => Some(ACTION_RESET_CAMERA),
                        "1" => Some(ACTION_SHOW_CONTROLS),
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
