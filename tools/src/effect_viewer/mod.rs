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

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::effect::{EffectId, EffectQueue};
use ragnarok_renderer::effect::{
    EffectHolder, EffectUpdateCtx, StrEffectCache, StrEmitterInput, build_str_effect_batches,
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

// === FFI types — must match `tools/effect-viewer-hot/src/lib.rs` exactly ===

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ViewerFlags {
    paused: u8,
    _pad0: [u8; 3],
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

const ACTION_NEXT_EFFECT: u32 = 1;
const ACTION_PREV_EFFECT: u32 = 2;
const ACTION_RESPAWN: u32 = 3;
const ACTION_TOGGLE_PAUSE: u32 = 4;
const ACTION_SPEED_UP: u32 = 5;
const ACTION_SPEED_DOWN: u32 = 6;

type HotCreateFn = extern "C" fn() -> *mut ();
type HotDestroyFn = unsafe extern "C" fn(*mut ());
type HotUpdateFn = unsafe extern "C" fn(*mut (), f32);
type HotOnActionFn = unsafe extern "C" fn(*mut (), u32);
type HotGetFlagsFn = unsafe extern "C" fn(*mut (), *mut ViewerFlags);
type HotTakePendingFn = unsafe extern "C" fn(*mut (), *mut PendingSpawn);
type HotBuildOverlayFn =
    unsafe extern "C" fn(*mut (), *const FontAtlas, f32, f32, *mut Vec<UiDrawCall>);

struct HotLib {
    _lib: libloading::Library,
    state: *mut (),
    update_fn: HotUpdateFn,
    on_action_fn: HotOnActionFn,
    get_flags_fn: HotGetFlagsFn,
    take_pending_fn: HotTakePendingFn,
    build_overlay_fn: HotBuildOverlayFn,
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
        let (create, destroy, update, on_action, get_flags, take_pending, build_overlay) = unsafe {
            let c: libloading::Symbol<HotCreateFn> = lib.get(b"hot_create").ok()?;
            let d: libloading::Symbol<HotDestroyFn> = lib.get(b"hot_destroy").ok()?;
            let u: libloading::Symbol<HotUpdateFn> = lib.get(b"hot_update").ok()?;
            let a: libloading::Symbol<HotOnActionFn> = lib.get(b"hot_on_action").ok()?;
            let f: libloading::Symbol<HotGetFlagsFn> = lib.get(b"hot_get_flags").ok()?;
            let t: libloading::Symbol<HotTakePendingFn> = lib.get(b"hot_take_pending_spawn").ok()?;
            let b: libloading::Symbol<HotBuildOverlayFn> = lib.get(b"hot_build_overlay").ok()?;
            (*c, *d, *u, *a, *f, *t, *b)
        };
        let state = (create)();
        Some(Self {
            _lib: lib,
            state,
            update_fn: update,
            on_action_fn: on_action,
            get_flags_fn: get_flags,
            take_pending_fn: take_pending,
            build_overlay_fn: build_overlay,
            destroy_fn: destroy,
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

/// Reverse-lookup a u16 onto an `EffectId` variant. Linear over the variant
/// list — fine for ~20 entries; will be a generated table when `EffectId`
/// scales to ~1000.
fn effect_id_from_u16(value: u16) -> Option<EffectId> {
    use EffectId::*;
    const ALL: &[EffectId] = &[
        Torch,
        ChimneySmoke,
        Bubble,
        GasPush,
        Spring,
        Hit1,
        Hit2,
        Hit3,
        FireBolt,
        ColdBolt,
        LightningBolt,
        IceWall,
        EarthSpike,
        GrimTooth,
        MagnusExorcismus,
        GrandCross,
        LordOfVermillion,
        StormGust,
        Level99,
        Lvup,
        JobLvup,
        RefineOk,
        RefineFail,
        Potion1,
        Potion2,
    ];
    ALL.iter().copied().find(|e| e.as_u16() == value)
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
    last_frame: Instant,
    hot_lib: Option<HotLib>,
    dylib_path: PathBuf,
    last_dylib_mtime: SystemTime,
    reload_counter: u64,
    /// STR file names already requested through the cache (failures included)
    /// so we don't retry every frame.
    attempted_str_files: std::collections::HashSet<String>,
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
            last_frame: Instant::now(),
            hot_lib,
            dylib_path,
            last_dylib_mtime,
            reload_counter: 0,
            attempted_str_files: std::collections::HashSet::new(),
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
        self.effect_queue
            .spawn_at(effect_id, [pos[0], pos[1], pos[2]]);
    }

    fn ensure_str_loaded(&mut self) {
        // Eagerly preload STR files for any EffectSpec::Str entries in the
        // pickable list. Cheap to do once per app start.
        let names: Vec<&'static str> = effect_str_names();
        let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) else {
            return;
        };
        for name in names {
            if self.attempted_str_files.contains(name) {
                continue;
            }
            self.attempted_str_files.insert(name.to_string());
            self.str_effects.load(
                name,
                grf,
                &mut renderer.texture_cache,
                &renderer.device.device,
                &renderer.device.queue,
            );
        }
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

        let Some(renderer) = &mut self.renderer else {
            return;
        };

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
        let sprite_batches = build_str_effect_batches(
            &str_inputs,
            &self.str_effects,
            &renderer.camera,
            screen_w,
            screen_h,
        );

        // cdylib overlay
        let mut ui_calls: Vec<UiDrawCall> = Vec::new();
        if let Some(hot) = &self.hot_lib {
            hot.build_overlay(&renderer.font_atlas, screen_w, screen_h, &mut ui_calls);
        }

        renderer.render(&ui_calls, &sprite_batches, &[], &[], 0.0);
    }
}

/// STR file names referenced by the pickable list in the cdylib. Hand-curated
/// for stage 1; replaced by a code-generated table later.
fn effect_str_names() -> Vec<&'static str> {
    vec![
        "bubble",
        "gaspush",
        "spring",
        "fire_bolt",
        "lightning_bolt",
        "lvup",
        "joblvup",
        "refineok",
        "refinefail",
        "potion1",
    ]
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

        match GrfArchive::open(Path::new(&self.grf_path)) {
            Ok(grf) => self.grf = Some(grf),
            Err(e) => {
                eprintln!("Failed to open GRF {}: {e}", self.grf_path);
                event_loop.exit();
                return;
            }
        }

        self.renderer = Some(renderer);
        self.window = Some(window);
        self.last_frame = Instant::now();
        self.ensure_str_loaded();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.device.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let action = match event.logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) => {
                        event_loop.exit();
                        return;
                    }
                    Key::Named(NamedKey::Space) | Key::Named(NamedKey::ArrowRight) => {
                        Some(ACTION_NEXT_EFFECT)
                    }
                    Key::Named(NamedKey::ArrowLeft) => Some(ACTION_PREV_EFFECT),
                    Key::Character(c) => match c {
                        "r" | "R" => Some(ACTION_RESPAWN),
                        "p" | "P" => Some(ACTION_TOGGLE_PAUSE),
                        "+" | "=" => Some(ACTION_SPEED_UP),
                        "-" | "_" => Some(ACTION_SPEED_DOWN),
                        _ => None,
                    },
                    _ => None,
                };
                if let (Some(code), Some(hot)) = (action, &self.hot_lib) {
                    hot.on_action(code);
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
