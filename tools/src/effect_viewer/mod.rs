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

pub mod gif_export;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use models::enums::EnumWithNumberValue;
use models::enums::EnumWithStringValue;
use models::enums::effect_id::EffectId;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::effect::spec::EffectAnchor;
use ragnarok_game::effect::{
    EffectQueue, EffectSpec, effect_spec, effect_texture_paths, is_count_point_effect,
    is_link_effect, is_trail_effect, str_aliases,
};

use crate::sprite_viewer::browser::SpriteBrowser;
use crate::sprite_viewer::shader_watcher::ShaderWatcher;
use crate::stress::{StressTick, stress_label};
use ragnarok_game::effect::EffectRenderCtx as GameEffectRenderCtx;
use ragnarok_renderer::effect::{
    EffectDrawList, EffectHolder, EffectRenderCtx, EffectUpdateCtx, ExternalCustomBackend,
    SpawnStatus, StrEffectCache, StrEmitterInput, build_str_effect_batches,
};
use ragnarok_renderer::effect_sprite::{
    EffectSpriteCache, SpriteEffectEmitter, build_emitter_batches, collect_sprite_effect_draws,
};
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::{Camera, Renderer, UiDrawCall, UiTextureRef, block_on};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

pub struct Args {
    pub grf_path: String,
}

/// Batch-mode GIF export: render one effect at the cdylib's default camera
/// and exit when the GIF is fully written. Window is created hidden.
pub struct BatchExport {
    pub effect_id: EffectId,
    pub out_path: PathBuf,
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

/// Representative target sprite footprint `[width, height]` in world units,
/// used to demo target-sized effects (lock-on reticle) in the viewer where
/// there is no live entity to measure.
const DEMO_TARGET_SIZE: [f32; 2] = [16.0, 16.0];

// Effect-registry FFI (handle = 0 = invalid / spawn failed).
// `target_w`/`target_h` carry the target sprite size in world units; NaN means
// "no target size" (the C ABI has no `Option`), mirroring the `caster_yaw` NaN
// convention in the update FFI.
type HotSpawnCustomEffectFn =
    unsafe extern "C" fn(*mut (), u16, *const [f32; 3], *const [f32; 3], u8, f32, f32) -> u64;
type HotUpdateCustomEffectFn = unsafe extern "C" fn(*mut (), u64, f32, f32) -> u8;
type HotCollectCustomDrawsFn =
    unsafe extern "C" fn(*mut (), u64, *const EffectRenderCtxFfi, *mut EffectDrawList);
type HotDropCustomEffectFn = unsafe extern "C" fn(*mut (), u64);
type HotDropAllCustomEffectsFn = unsafe extern "C" fn(*mut ());
/// Drains a one-shot camera-shake from a cdylib effect. Returns 1 and fills
/// `out` when present, 0 otherwise.
type HotTakeCameraShakeFn = unsafe extern "C" fn(*mut (), u64, *mut CameraShakeFfi) -> u8;

/// FFI mirror of `ragnarok_game::effect::CameraShake`. Must match the cdylib.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CameraShakeFfi {
    pub amplitude: f32,
    pub duration_ms: u32,
}

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

/// Which renderer(s) a given shader file feeds. Used by `App` to dispatch a
/// `ShaderWatcher` reload to the right `recreate_pipelines` call.
#[derive(Clone, Copy)]
enum ShaderTarget {
    /// `effect_frustum.wgsl` — shared by `FrustumRenderer` and `QuadHornRenderer`.
    EffectFrustum,
    /// `effect_ground_disc.wgsl` — `GroundDiscRenderer`.
    EffectGroundDisc,
    /// `sprite.wgsl` — main `SpriteRenderer` and `effect_sprite_renderer`.
    Sprite,
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
    take_camera_shake_fn: HotTakeCameraShakeFn,
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
    take_camera_shake_fn: HotTakeCameraShakeFn,
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
    fn spawn(
        &self,
        effect_id: u16,
        from: [f32; 3],
        to: [f32; 3],
        hit_count: u8,
        target_size: Option<[f32; 2]>,
    ) -> u64 {
        let [tw, th] = target_size.unwrap_or([f32::NAN, f32::NAN]);
        let handle = unsafe {
            (self.spawn_fn)(
                self.state,
                effect_id,
                &from as *const _,
                &to as *const _,
                hit_count,
                tw,
                th,
            )
        };
        if handle != 0 {
            self.handle_ids.lock().unwrap().insert(handle, effect_id);
        }
        handle
    }

    fn update(&self, handle: u64, dt: f32, caster_yaw: Option<f32>) -> bool {
        // NaN encodes "no caster facing" across the C ABI (no Option<f32>).
        let yaw = caster_yaw.unwrap_or(f32::NAN);
        let dead = unsafe { (self.update_fn)(self.state, handle, dt, yaw) };
        dead == 0
    }

    fn collect(&self, handle: u64, ctx: &GameEffectRenderCtx, out: &mut EffectDrawList) {
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
            EffectAnchor::Point([0.0, 0.0, 0.0]),
            None,
            None,
            None,
        )?;
        probe.str_overlay().map(|s| s.to_string())
    }

    fn take_camera_shake(&self, handle: u64) -> Option<ragnarok_game::effect::CameraShake> {
        let mut out = CameraShakeFfi::default();
        let present =
            unsafe { (self.take_camera_shake_fn)(self.state, handle, &mut out as *mut _) };
        (present != 0).then_some(ragnarok_game::effect::CameraShake {
            amplitude: out.amplitude,
            duration_ms: out.duration_ms,
        })
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
            let on_mouse_wheel_fn = *lib.get::<HotOnMouseWheelFn>(b"hot_on_mouse_wheel").ok()?;
            let on_mouse_drag_fn = *lib.get::<HotOnMouseDragFn>(b"hot_on_mouse_drag").ok()?;
            let get_flags_fn = *lib.get::<HotGetFlagsFn>(b"hot_get_flags").ok()?;
            let get_camera_fn = *lib.get::<HotGetCameraFn>(b"hot_get_camera").ok()?;
            let take_pending_fn = *lib
                .get::<HotTakePendingFn>(b"hot_take_pending_spawn")
                .ok()?;
            let take_step_request_fn = *lib
                .get::<HotTakeStepRequestFn>(b"hot_take_step_request")
                .ok()?;
            let set_last_status_fn = *lib.get::<HotSetLastStatusFn>(b"hot_set_last_status").ok()?;
            let build_overlay_fn = *lib.get::<HotBuildOverlayFn>(b"hot_build_overlay").ok()?;
            let get_filtered_ids_fn = *lib
                .get::<HotGetFilteredIdsFn>(b"hot_get_filtered_ids")
                .ok()?;
            let set_selected_effect_id_fn = *lib
                .get::<HotSetSelectedEffectIdFn>(b"hot_set_selected_effect_id")
                .ok()?;
            let snapshot_state_fn = *lib.get::<HotSnapshotStateFn>(b"hot_snapshot_state").ok()?;
            let restore_state_fn = *lib.get::<HotRestoreStateFn>(b"hot_restore_state").ok()?;
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
            let take_camera_shake_fn = *lib
                .get::<HotTakeCameraShakeFn>(b"hot_take_camera_shake")
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
                take_camera_shake_fn,
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
            take_camera_shake_fn: self.take_camera_shake_fn,
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
            (self.build_overlay_fn)(
                self.state,
                atlas as *const FontAtlas,
                screen_w,
                screen_h,
                out,
            )
        };
    }
}

fn effect_id_from_u16(value: u16) -> Option<EffectId> {
    EffectId::try_from_value(value as usize).ok()
}

/// Build a demo (caster, target) world-coord pair for trail-shaped
/// effects (e.g. Frost Diver). `world` is the user-picked spawn point;
/// we treat it as the caster's feet and project the target a fixed
/// distance along world +Z so the projectile trail is fully visible
/// inside the GIF exporter's default camera framing. Real spawn callers
/// supply actual caster→target positions and bypass this.
fn demo_trail_endpoints(world: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    const TRAIL_DEMO_LEN: f32 = 22.0;
    // Convention: `from` = source, `to` = target. The clicked point is the
    // target so impact rings / directional sparks (which anchor on `to`)
    // preview at the click; the source sits one trail-length behind it.
    let from = [world[0], world[1], world[2] - TRAIL_DEMO_LEN];
    let to = world;
    (from, to)
}

fn effect_duration_ms(id: EffectId) -> Option<u32> {
    match effect_spec(id) {
        Some(EffectSpec::Custom { duration_ms }) => Some(duration_ms),
        Some(EffectSpec::Str { duration_ms, .. }) => Some(duration_ms),
        Some(EffectSpec::Spr { duration_ms, .. }) => Some(duration_ms),
        Some(EffectSpec::SprBurst { duration_ms, .. }) => Some(duration_ms),
        Some(EffectSpec::Noop) | None => None,
    }
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
    effect_sprites: EffectSpriteCache,
    /// SPR paths already attempted; we don't retry every frame.
    attempted_spr_files: std::collections::HashSet<String>,
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
    trail_target_override: Option<[f32; 3]>,
    placing_target: bool,
    mouse_pos: (f32, f32),
    last_mouse: (f32, f32),
    mouse_down_right: bool,
    /// Set after the first triage log line so the markdown table header is
    /// printed exactly once per viewer session.
    triage_header_emitted: bool,
    /// Active GIF export, when present. Set by the `E` key handler in
    /// interactive mode or by `run_batch_export` for headless capture.
    gif_session: Option<gif_export::GifSession>,
    /// Batch-export configuration. When set, the app starts a GIF session
    /// on first frame and exits when capture completes.
    batch: Option<BatchExport>,
    /// Set by the batch path once the GIF is finalized; the event loop
    /// observes this on the next `RedrawRequested` and exits.
    should_exit: bool,
    /// Hit count for multi-bolt trail effects (Soul Strike). Keys 1-5
    /// cycle the count; the viewer re-spawns with the new value.
    demo_hit_count: u8,
    /// One watcher per `.wgsl` file the effect viewer renders with. Each
    /// fires independently; `render_frame` polls them and rebuilds the
    /// matching pipelines on change.
    shader_watchers: Vec<(ShaderWatcher, ShaderTarget)>,
    /// Stress test (G opens the set browser, K stops). Spawns many effects at
    /// random visible ground positions to profile the effect path under load.
    stress_sets: Vec<crate::stress::StressSet>,
    stress: crate::stress::StressRunner,
    stress_browser: Option<SpriteBrowser>,
    fps: ragnarok_renderer::Fps,
    /// Earliest instant the next interactive frame may render. The event loop
    /// sleeps (`ControlFlow::WaitUntil`) until this point instead of spinning,
    /// keeping CPU near-idle. Bypassed while a GIF export is active so headless
    /// capture runs as fast as it can.
    next_frame: Instant,
}

/// Interactive redraw cadence. Effects are authored for a 60 fps display, so
/// there's nothing to gain from rendering faster — the extra frames only burn
/// CPU (the preferred `Mailbox` present mode never blocks to throttle us).
const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);

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
            effect_sprites: EffectSpriteCache::new(),
            attempted_spr_files: std::collections::HashSet::new(),
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
            trail_target_override: None,
            placing_target: false,
            mouse_pos: (0.0, 0.0),
            last_mouse: (0.0, 0.0),
            mouse_down_right: false,
            triage_header_emitted: false,
            gif_session: None,
            batch: None,
            should_exit: false,
            demo_hit_count: 5,
            shader_watchers: Vec::new(),
            next_frame: Instant::now(),
            stress_sets: crate::stress::stress_sets(),
            stress: crate::stress::StressRunner::new(),
            stress_browser: None,
            fps: ragnarok_renderer::Fps::new(),
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

    fn stress_browser_is_open(&self) -> bool {
        self.stress_browser.as_ref().is_some_and(|b| b.open)
    }

    fn open_stress_browser(&mut self) {
        let items: Vec<String> = self.stress_sets.iter().map(stress_label).collect();
        let mut browser = SpriteBrowser::new(items, "stress tests");
        if let Some(renderer) = &self.renderer {
            let h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
            browser.update_visible_rows(h);
        }
        self.stress_browser = Some(browser);
    }

    fn handle_stress_browser_key(&mut self, key: &Key) {
        let Some(browser) = &mut self.stress_browser else {
            return;
        };
        match key.as_ref() {
            Key::Named(NamedKey::Escape) => browser.open = false,
            Key::Named(NamedKey::Enter) => self.handle_stress_browser_select(),
            Key::Named(NamedKey::ArrowUp) => browser.handle_up(),
            Key::Named(NamedKey::ArrowDown) => browser.handle_down(),
            Key::Named(NamedKey::PageUp) => browser.handle_page_up(),
            Key::Named(NamedKey::PageDown) => browser.handle_page_down(),
            Key::Named(NamedKey::Backspace) => browser.handle_backspace(),
            Key::Character(ch) => {
                for c in ch.chars() {
                    if !c.is_control() {
                        browser.handle_char(c);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_stress_browser_select(&mut self) {
        let Some(browser) = &mut self.stress_browser else {
            return;
        };
        let Some(selected) = browser.selected_item().map(|s| s.to_string()) else {
            return;
        };
        browser.open = false;
        if let Some(idx) = self
            .stress_sets
            .iter()
            .position(|s| stress_label(s) == selected)
        {
            self.effect_holder.clear();
            self.stress.launch(idx);
        }
    }

    fn stop_stress(&mut self) {
        self.stress.stop();
        self.effect_holder.clear();
    }

    /// Clear the holder and re-spawn stress set `idx` at fresh random visible
    /// ground positions (the ground-proxy plane is `y = 0`).
    fn reseed_stress(&mut self, idx: usize) {
        self.effect_holder.clear();
        let entries = match self.stress_sets.get(idx) {
            Some(s) => s.entries.clone(),
            None => return,
        };
        let Some(renderer) = &self.renderer else {
            return;
        };
        let screen_w = renderer.device.surface_config.width as f32;
        let screen_h = renderer.device.surface_config.height as f32;
        let mut rng = crate::stress::Rng::new(self.stress.next_seed());
        for (id, count) in entries {
            self.ensure_str_loaded_for(id);
            self.ensure_spr_loaded_for(id);
            let positions = {
                let camera = &self.renderer.as_ref().unwrap().camera;
                crate::stress::random_visible_ground_positions(
                    camera, screen_w, screen_h, 0.0, count, &mut rng,
                )
            };
            for pos in positions {
                crate::stress::enqueue_effect(&mut self.effect_queue, id, pos, None);
            }
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
        self.emit_triage_row(effect_id);
        // Viewer convention: each picker spawn is the user asking to see
        // *just* this effect. Clear anything still alive so persistent
        // effects (Aura) don't pile up as we cycle through the list.
        self.effect_holder.clear();
        // Lazy-load the STR file for this effect, if any, before pushing
        // the spawn. The holder won't actually render anything until its
        // bind groups are present in the cache.
        self.ensure_str_loaded_for(effect_id);
        self.ensure_spr_loaded_for(effect_id);
        let world = [pos[0], pos[1], pos[2]];
        // Link effects (Linelink) have no entity table in the viewer, so the
        // green-cross target stands in as a static second actor — drive them
        // through the same caster→target spawn path as trail effects.
        if is_trail_effect(effect_id) || is_link_effect(effect_id) {
            let to = self
                .trail_target_override
                .unwrap_or_else(|| demo_trail_endpoints(world).1);
            if effect_id == EffectId::Soulstrike {
                self.effect_queue
                    .spawn_trail_with_count(effect_id, world, to, self.demo_hit_count);
            } else {
                self.effect_queue.spawn_trail(effect_id, world, to);
            }
        } else if is_count_point_effect(effect_id) {
            self.effect_queue
                .spawn_at_with_count(effect_id, world, self.demo_hit_count);
        } else if effect_id == EffectId::Lockon {
            // No entity table in the viewer; stand in a representative target
            // sprite footprint so the reticle scales to it (the in-game path
            // will pass the real target size).
            self.effect_queue
                .spawn_at_with_size(effect_id, world, DEMO_TARGET_SIZE);
        } else {
            self.effect_queue.spawn_at(effect_id, world);
        }
    }

    fn try_pick_world_position(&self) -> Option<[f32; 3]> {
        let renderer = self.renderer.as_ref()?;
        crate::viewer_common::screen_to_ground(
            &renderer.camera,
            self.mouse_pos.0,
            self.mouse_pos.1,
            renderer.device.surface_config.width as f32,
            renderer.device.surface_config.height as f32,
            0.0,
        )
    }

    /// Stderr a markdown table row describing the effect being spawned, so
    /// the user can grep `EFFECT_TRIAGE` lines out of the viewer log and
    /// paste them into a triage doc. Header row prints once per session.
    fn emit_triage_row(&mut self, id: EffectId) {
        if !self.triage_header_emitted {
            eprintln!(
                "EFFECT_TRIAGE | id | name | bucket | class | impl | str | dur_ms | primitive"
            );
            eprintln!("EFFECT_TRIAGE | --- | --- | --- | --- | --- | --- | --- | ---");
            self.triage_header_emitted = true;
        }
        let id_num = id.value() as u32;
        let bucket_start = (id_num / 50) * 50;
        let bucket = format!("{}-{}", bucket_start, bucket_start + 50);
        let class = if ragnarok_game::effect::buckets::is_hybrid(id) {
            "Hybrid"
        } else if ragnarok_game::effect::buckets::is_custom_bucket(id) {
            "Custom"
        } else if ragnarok_game::effect::buckets::is_noop_bucket(id) {
            "Noop"
        } else {
            "DefaultStr"
        };
        let impl_flag = if ragnarok_game::effect::is_real_impl(id) {
            "yes"
        } else {
            "no"
        };
        let str_field = {
            let aliases = str_aliases(id);
            if aliases.is_empty() {
                "-".to_string()
            } else {
                aliases.join(",")
            }
        };
        let dur_ms = match effect_spec(id) {
            Some(EffectSpec::Custom { duration_ms }) => duration_ms.to_string(),
            Some(EffectSpec::Str { duration_ms, .. }) => duration_ms.to_string(),
            Some(EffectSpec::Spr { duration_ms, .. }) => duration_ms.to_string(),
            Some(EffectSpec::SprBurst { duration_ms, .. }) => duration_ms.to_string(),
            Some(EffectSpec::Noop) | None => "-".to_string(),
        };
        eprintln!(
            "EFFECT_TRIAGE | {} | EffectId::{:?} | {} | {} | {} | {} | {} | ?",
            id_num, id, bucket, class, impl_flag, str_field, dur_ms,
        );
    }

    /// Make sure any STR file the effect needs is in the cache. Covers two
    /// cases:
    ///   * `EffectSpec::Str` — load the named file directly.
    ///   * `EffectSpec::Custom` — query the factory for a throwaway
    ///     instance; if it declares an `str_overlay()`, load that file too
    ///     (hybrid effects like Stormgust). The throwaway instance is
    ///     dropped immediately; the real spawn happens via the cdylib path.
    /// Failures are remembered so we don't retry every cycle.
    /// Lazy-load the SPR billboard(s) needed by this effect. For
    /// `EffectSpec::Spr` / `SprBurst` it's the spec's `sprite` path;
    /// for `EffectSpec::Custom` it's whatever sprite paths the effect
    /// declares in its module's `SPRITES` constant (e.g. Hit's
    /// particle1.spr). The viewer accumulates loaded paths so repeated
    /// spawns don't retry parse failures.
    fn ensure_spr_loaded_for(&mut self, id: EffectId) {
        let mut sprites: Vec<&'static str> = Vec::new();
        match effect_spec(id) {
            Some(EffectSpec::Spr { sprite, .. }) => sprites.push(sprite),
            Some(EffectSpec::SprBurst { sprite, .. }) => sprites.push(sprite),
            Some(EffectSpec::Custom { .. }) => {
                // Custom effects can also drive sprite billboards via
                // SpriteParticle. Preload the aggregated paths so the
                // first frame after spawn isn't silently empty.
                sprites.extend(ragnarok_game::effect::custom_effect_sprite_paths());
            }
            _ => return,
        }
        for sprite in sprites {
            if self.attempted_spr_files.contains(sprite) {
                continue;
            }
            self.attempted_spr_files.insert(sprite.to_string());
            let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) else {
                return;
            };
            self.effect_sprites.load(
                sprite,
                grf,
                &renderer.device.device,
                &renderer.device.queue,
                &renderer.texture_cache.bind_group_layout,
            );
        }
    }

    fn ensure_str_loaded_for(&mut self, id: EffectId) {
        let file: &'static str = match effect_spec(id) {
            Some(EffectSpec::Str { file, .. }) => file,
            Some(EffectSpec::Custom { .. }) => {
                let probe = ragnarok_game::effect::factory::make_effect(
                    id,
                    EffectAnchor::Point([0.0, 0.0, 0.0]),
                    None,
                    None,
                    None,
                );
                let Some(probe) = probe else { return };
                let Some(overlay) = probe.str_overlay() else {
                    return;
                };
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

    /// Begin a GIF export for `effect_id`, writing to `out_path`. Clears
    /// the current effect holder and respawns the effect at the world
    /// origin so the recording starts from frame 0.
    fn start_gif_export(&mut self, effect_id: EffectId, out_path: PathBuf) -> bool {
        if self.gif_session.is_some() {
            eprintln!("[gif] export already in progress, ignoring request");
            return false;
        }
        let Some(renderer) = &self.renderer else {
            eprintln!("[gif] renderer not initialised yet");
            return false;
        };
        let format = renderer.device.surface_format;
        let session = match gif_export::GifSession::begin(
            &renderer.device.device,
            format,
            effect_id,
            effect_duration_ms(effect_id),
            out_path.clone(),
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[gif] failed to begin export: {e}");
                return false;
            }
        };
        eprintln!(
            "[gif] recording effect {} ({:?}) → {} ({} frames @ {} fps)",
            effect_id.value(),
            effect_id,
            out_path.display(),
            session.frames_total,
            gif_export::GIF_FPS,
        );
        self.effect_holder.clear();
        self.ensure_str_loaded_for(effect_id);
        self.ensure_spr_loaded_for(effect_id);
        if is_trail_effect(effect_id) || is_link_effect(effect_id) {
            let origin = [0.0, 0.0, 0.0];
            let to = self
                .trail_target_override
                .unwrap_or_else(|| demo_trail_endpoints(origin).1);
            if effect_id == EffectId::Soulstrike {
                self.effect_queue.spawn_trail_with_count(
                    effect_id,
                    origin,
                    to,
                    self.demo_hit_count,
                );
            } else {
                self.effect_queue.spawn_trail(effect_id, origin, to);
            }
        } else if is_count_point_effect(effect_id) {
            self.effect_queue
                .spawn_at_with_count(effect_id, [0.0, 0.0, 0.0], self.demo_hit_count);
        } else if effect_id == EffectId::Lockon {
            self.effect_queue
                .spawn_at_with_size(effect_id, [0.0, 0.0, 0.0], DEMO_TARGET_SIZE);
        } else {
            self.effect_queue.spawn_at(effect_id, [0.0, 0.0, 0.0]);
        }
        self.gif_session = Some(session);
        true
    }

    /// Default output path for an interactive (E-key) export.
    fn default_gif_out_path(effect_id: EffectId) -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        PathBuf::from(format!("gif_export/{}_{}.gif", effect_id.value(), ts))
    }

    /// Spawn one `ShaderWatcher` per shader file used by the effect viewer.
    /// Failure to set up a watcher is logged but non-fatal — the viewer
    /// still runs, just without hot reload for that file.
    fn init_shader_watchers(&mut self) {
        let shader_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../lib/renderer/src/shaders");
        let targets: &[(&str, ShaderTarget)] = &[
            ("effect_frustum.wgsl", ShaderTarget::EffectFrustum),
            ("effect_ground_disc.wgsl", ShaderTarget::EffectGroundDisc),
            ("sprite.wgsl", ShaderTarget::Sprite),
        ];
        for (filename, target) in targets {
            match ShaderWatcher::new(&shader_dir, filename) {
                Ok(w) => self.shader_watchers.push((w, *target)),
                Err(e) => tracing::warn!("Shader watcher unavailable for {}: {e}", filename),
            }
        }
    }

    /// Poll every watcher; if any fired, read its file from disk and rebuild
    /// the matching renderer's pipelines.
    fn poll_shader_reload(&mut self) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        for (watcher, target) in &self.shader_watchers {
            let Some(source) = watcher.check_and_reload() else {
                continue;
            };
            let device = &renderer.device.device;
            let surface_format = renderer.device.surface_format;
            let camera_bgl = &renderer.global_uniforms.bind_group_layout;
            let texture_bgl = &renderer.texture_cache.bind_group_layout;
            match target {
                ShaderTarget::EffectFrustum => {
                    renderer.effect_frustum_renderer.recreate_pipelines(
                        device,
                        surface_format,
                        camera_bgl,
                        texture_bgl,
                        &source,
                    );
                    renderer.effect_quad_horn_renderer.recreate_pipelines(
                        device,
                        surface_format,
                        camera_bgl,
                        texture_bgl,
                        &source,
                    );
                }
                ShaderTarget::EffectGroundDisc => {
                    renderer.effect_ground_disc_renderer.recreate_pipelines(
                        device,
                        surface_format,
                        camera_bgl,
                        texture_bgl,
                        &source,
                    );
                }
                ShaderTarget::Sprite => {
                    renderer.sprite_renderer.recreate_pipeline(
                        device,
                        surface_format,
                        texture_bgl,
                        &source,
                    );
                    renderer.effect_sprite_renderer.recreate_pipeline(
                        device,
                        surface_format,
                        texture_bgl,
                        &source,
                    );
                }
            }
        }
    }

    fn render_frame(&mut self) {
        self.check_hot_reload();
        self.poll_shader_reload();
        // During an active GIF export the spawn comes from `start_gif_export`,
        // not from the cdylib's picker. Drain any pending spawn so the next
        // frame doesn't pick it up, but otherwise leave the holder alone —
        // letting `poll_pending_spawn` fire would clear our explicit spawn
        // and replace it with the cdylib's currently-selected effect.
        if self.gif_session.is_some() {
            if let Some(hot) = &self.hot_lib {
                let _ = hot.take_pending_spawn();
            }
        } else {
            self.poll_pending_spawn();
        }

        let now = Instant::now();
        let raw_dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        let dt = raw_dt.min(0.1);

        let flags = self.hot_lib.as_ref().map(|h| h.flags()).unwrap_or_default();
        let speed = (flags.speed_x100 as f32 / 100.0).max(0.0);
        let paused = flags.paused != 0;
        let step = paused && self.hot_lib.as_ref().is_some_and(|h| h.take_step_request());
        // When a GIF export is active, the simulation is driven by a fixed
        // tick so the recording is deterministic regardless of vsync jitter
        // or the user's pause/speed flags. The viewer also rebuilds the same
        // frame for the surface below, so the user sees what is being
        // captured in real time.
        let scaled_dt = if self.gif_session.is_some() {
            1.0 / gif_export::SIM_TICK_HZ
        } else if paused {
            if step { 1.0 / 60.0 } else { 0.0 }
        } else {
            dt * speed
        };

        if let Some(hot) = &self.hot_lib {
            hot.update(scaled_dt);
        }

        self.fps.tick(dt);
        // Continuous stress test: re-seed the active set on its cadence before
        // the queue is drained below.
        if let StressTick::Reseed(idx) = self.stress.tick(scaled_dt) {
            self.reseed_stress(idx);
        }

        // Drain spawn requests into the holder, then tick it. Camera target
        // lets camera-anchored SprBurst effects (Snow, etc.) follow the view.
        self.effect_holder
            .drain_queue(&mut self.effect_queue, &|_| None);
        let camera_target = self.renderer.as_ref().map(|r| r.camera.target.to_array());
        // Reuse the projectile crosshair to aim direction-oriented effects
        // (AttackEnergy comet, AttackEnergy2 rings, Guard shell): the caster
        // sits at the origin, so the crosshair gives a world facing yaw
        // (`dx.atan2(dz)`, the +Z = 0 heading convention). No crosshair → a
        // fixed front.
        let caster_yaw = self.trail_target_override.map(|t| t[0].atan2(t[2]));
        self.effect_holder.update(
            &EffectUpdateCtx {
                delta: scaled_dt,
                camera_target,
                caster_yaw,
            },
            &|_| None,
            // Viewer has no entity table; link effects render as a static
            // tether to the green-cross fake entity via their spawn anchor.
            &|_| None,
        );
        // Apply any active screen-shake from quake effects to the camera.
        if let Some(r) = self.renderer.as_mut() {
            r.camera.shake_offset = self.effect_holder.camera_shake_offset().into();
        }

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
                repeat: s.repeat,
            })
            .collect();
        let mut effect_batches = build_str_effect_batches(
            &str_inputs,
            &self.str_effects,
            &renderer.camera,
            screen_w,
            screen_h,
            10.0,
        );

        // SPR-billboard snapshots → emitter inputs (Torch and the rest of the
        // Tier-A spec entries). Mirrors the RSW ambient path.
        let spr_snapshots = self.effect_holder.collect_spr_emitters(&|_| None);
        let burst_snapshots = self.effect_holder.collect_spr_burst_emitters(&|_| None);
        let mut spr_inputs: Vec<SpriteEffectEmitter<'_>> = spr_snapshots
            .iter()
            .map(|s| SpriteEffectEmitter::Spr {
                sprite_path: &s.sprite,
                duration_ms: s.duration_ms,
                position: s.position,
                color: s.tint,
                size_scale: s.size_scale,
                anim_speed: s.anim_speed,
                repeat: s.repeat,
                anim_time: s.anim_time,
                action_index: s.action_index,
            })
            .collect();
        spr_inputs.extend(
            burst_snapshots
                .iter()
                .map(|b| SpriteEffectEmitter::ParticleBurst {
                    sprite_path: &b.sprite,
                    alpha_max: b.alpha_max,
                    color: [1.0, 1.0, 1.0, 1.0],
                    size_scale: b.size_scale,
                    anim_speed: b.anim_speed,
                    size_shrink: b.size_shrink,
                    twinkle: b.twinkle,
                    particles: b.particles.clone(),
                }),
        );
        let spr_draws = collect_sprite_effect_draws(
            &spr_inputs,
            &self.effect_sprites,
            &renderer.camera,
            screen_w,
            screen_h,
        );
        effect_batches.extend(build_emitter_batches(&spr_draws));

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
        // Custom effects can emit `SpriteParticle` primitives for
        // per-particle SPR billboards (Hit's debris). The renderer's
        // unified effect dispatch consumes these as `DrawRecord`s so they
        // depth-sort against Billboard / 3D primitives from the same
        // EffectDrawList.
        let sprite_particle_records = ragnarok_renderer::prepare_sprite_particle_records(
            &effect_draws,
            &self.effect_sprites,
            &renderer.camera,
            screen_w,
            screen_h,
        );

        // cdylib overlay (status + legend + controls). Browser, when open,
        // is drawn on top by the host.
        let mut ui_calls: Vec<UiDrawCall> = Vec::new();
        if let Some(hot) = &self.hot_lib {
            hot.build_overlay(&renderer.font_atlas, screen_w, screen_h, &mut ui_calls);
        }
        ui_calls.extend(crate::viewer_common::build_fps(
            &renderer.font_atlas,
            self.fps.get(),
            self.effect_holder.len(),
        ));
        if let Some(browser) = &self.browser
            && browser.open
        {
            ui_calls.extend(browser.build_draw_calls(&renderer.font_atlas, screen_w, screen_h));
        }
        if let Some(browser) = &self.stress_browser
            && browser.open
        {
            ui_calls.extend(browser.build_draw_calls(&renderer.font_atlas, screen_w, screen_h));
        }
        if let Some(target) = self.trail_target_override {
            ui_calls.extend(build_target_crosshair(
                &renderer.camera,
                target,
                screen_w,
                screen_h,
            ));
        }
        if self.placing_target {
            let label = "[Click to place trail target]";
            let tw = renderer.font_atlas.measure_text(label);
            let (bv, bi) = ragnarok_ui::draw::quad_vertices(
                (screen_w - tw - 16.0) / 2.0,
                screen_h - 32.0,
                tw + 16.0,
                24.0,
                [0.0, 0.0, 0.0, 0.6],
            );
            ui_calls.push(UiDrawCall {
                vertices: bv.to_vec(),
                indices: bi.to_vec(),
                texture: UiTextureRef::White,
            });
            let (tv, ti) = ragnarok_ui::draw::text_vertices(
                label,
                (screen_w - tw) / 2.0,
                screen_h - 28.0,
                [0.2, 1.0, 0.2, 0.9],
                &renderer.font_atlas,
            );
            ui_calls.push(UiDrawCall {
                vertices: tv,
                indices: ti,
                texture: UiTextureRef::FontAtlas,
            });
        }

        renderer.render(
            &ui_calls,
            &effect_batches,
            &effect_draws,
            sprite_particle_records,
            &[],
            &[],
            &[],
            &[],
            0.0,
        );

        // GIF capture path: after the surface frame is presented, render the
        // same simulation state into the offscreen capture target at 256x256
        // with a black background and no HUD, then read it back as one GIF
        // frame. The renderer's camera aspect is reset by the next surface
        // render(), so we don't need to restore it here.
        if let Some(session) = self.gif_session.as_mut() {
            let capture_now = session.tick_should_capture();
            if capture_now {
                renderer.camera.aspect = 1.0;
                let cap_w = gif_export::GIF_W as f32;
                let cap_h = gif_export::GIF_H as f32;
                let mut capture_batches = build_str_effect_batches(
                    &str_inputs,
                    &self.str_effects,
                    &renderer.camera,
                    cap_w,
                    cap_h,
                    10.0,
                );
                let spr_draws_capture = collect_sprite_effect_draws(
                    &spr_inputs,
                    &self.effect_sprites,
                    &renderer.camera,
                    cap_w,
                    cap_h,
                );
                capture_batches.extend(build_emitter_batches(&spr_draws_capture));
                let sprite_particle_capture = ragnarok_renderer::prepare_sprite_particle_records(
                    &effect_draws,
                    &self.effect_sprites,
                    &renderer.camera,
                    cap_w,
                    cap_h,
                );
                let color_view = session.target.color_view.clone();
                let depth_view = session.target.depth_view.clone();
                renderer.render_into(
                    &color_view,
                    &depth_view,
                    gif_export::GIF_W,
                    gif_export::GIF_H,
                    wgpu::Color::BLACK,
                    &[],
                    &capture_batches,
                    &effect_draws,
                    sprite_particle_capture,
                    &[],
                    &[],
                    &[],
                    &[],
                    0.0,
                );
                session.write_current_frame(&renderer.device.device, &renderer.device.queue);
            }
            if session.is_complete() {
                let out = session.out_path().clone();
                eprintln!("[gif] export complete: {}", out.display());
                self.gif_session = None;
                if self.batch.is_some() {
                    self.should_exit = true;
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let batch_mode = self.batch.is_some();
        let attrs = WindowAttributes::default()
            .with_title("Effect Viewer")
            .with_inner_size(winit::dpi::PhysicalSize::new(1024u32, 768u32))
            .with_visible(!batch_mode);
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
        // Effect viewer never loads a real map, so install a debug checker
        // floor at y=0. Effect primitives use depth-read against it so their
        // lower halves (e.g. magnum break's sphere) get clipped at the
        // ground plane the same way as in-game.
        renderer.enable_ground_proxy();
        renderer.set_background_mode(ragnarok_renderer::BackgroundMode::GroundProxy);

        self.renderer = Some(renderer);
        self.white_bind_group = Some(white_bind_group);
        self.window = Some(window);
        self.last_frame = Instant::now();
        self.init_shader_watchers();

        // Batch mode: kick off the GIF export immediately so the first
        // render_frame call (triggered by winit's initial RedrawRequested)
        // already has a session running and ticks at gif-deterministic dt.
        if let Some(batch) = self.batch.as_ref() {
            let effect_id = batch.effect_id;
            let out_path = batch.out_path.clone();
            if !self.start_gif_export(effect_id, out_path) {
                self.should_exit = true;
            }
        }
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
                if self.stress_browser_is_open() {
                    self.handle_stress_browser_key(&event.logical_key);
                    return;
                }
                match event.logical_key.as_ref() {
                    Key::Character("g") | Key::Character("G") => {
                        self.open_stress_browser();
                        return;
                    }
                    Key::Character("k") | Key::Character("K") => {
                        self.stop_stress();
                        return;
                    }
                    _ => {}
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
                if matches!(
                    event.logical_key.as_ref(),
                    Key::Character("e") | Key::Character("E")
                ) {
                    let id_u16 = self
                        .hot_lib
                        .as_ref()
                        .map(|h| h.snapshot_state().selected_effect_id)
                        .unwrap_or(u16::MAX);
                    if let Some(id) = effect_id_from_u16(id_u16) {
                        let out = Self::default_gif_out_path(id);
                        self.start_gif_export(id, out);
                    } else {
                        eprintln!("[gif] no effect currently selected");
                    }
                    return;
                }
                if matches!(
                    event.logical_key.as_ref(),
                    Key::Character("t") | Key::Character("T")
                ) {
                    self.placing_target = !self.placing_target;
                    return;
                }
                if matches!(
                    event.logical_key.as_ref(),
                    Key::Character("x") | Key::Character("X")
                ) {
                    self.trail_target_override = None;
                    self.placing_target = false;
                    if let Some(hot) = &self.hot_lib {
                        hot.on_action(ACTION_RESPAWN);
                    }
                    return;
                }
                if matches!(
                    event.logical_key.as_ref(),
                    Key::Character("b") | Key::Character("B")
                ) {
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
                if let Key::Character(c) = event.logical_key.as_ref() {
                    if let Some(n) = c.chars().next()
                        && n.is_ascii_digit()
                    {
                        let count = (n as u8 - b'0').clamp(1, 5);
                        self.demo_hit_count = count;
                        eprintln!("[viewer] demo hit_count = {count}");
                        if let Some(hot) = &self.hot_lib {
                            hot.on_action(ACTION_RESPAWN);
                        }
                        return;
                    }
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
                let pressed = state == ElementState::Pressed;
                match button {
                    winit::event::MouseButton::Left => {
                        if pressed && self.placing_target {
                            if let Some(world_pos) = self.try_pick_world_position() {
                                self.trail_target_override = Some(world_pos);
                                self.placing_target = false;
                                if let Some(hot) = &self.hot_lib {
                                    hot.on_action(ACTION_RESPAWN);
                                }
                            }
                        }
                    }
                    winit::event::MouseButton::Right => {
                        self.mouse_down_right = pressed;
                        if pressed {
                            self.last_mouse = self.mouse_pos;
                        }
                    }
                    _ => {}
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
                if self.should_exit {
                    event_loop.exit();
                    return;
                }
                // GIF export drives its own deterministic tick and should run
                // as fast as possible (especially headless batch mode), so
                // re-arm immediately. The interactive cadence is paced by
                // `about_to_wait` instead.
                if self.gif_session.is_some()
                    && let Some(window) = &self.window
                {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // GIF export re-arms its own redraw in `RedrawRequested`; don't pace it.
        if self.gif_session.is_some() {
            event_loop.set_control_flow(ControlFlow::Poll);
            return;
        }
        let now = Instant::now();
        if now >= self.next_frame {
            self.next_frame = now + FRAME_INTERVAL;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
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

/// Batch GIF export. Creates a hidden window so wgpu can initialise its
/// surface (the renderer's pipelines are baked against the surface format),
/// loads the cdylib so the default camera matches what `C` would reset to
/// in the interactive viewer, and exits when the GIF is fully written.
pub fn run_batch_export(args: Args, effect_id: EffectId, out_path: PathBuf) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(args);
    app.batch = Some(BatchExport {
        effect_id,
        out_path,
    });
    event_loop.run_app(&mut app).unwrap();
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(hot) = self.hot_lib.take() {
            hot.unload();
        }
    }
}

fn build_target_crosshair(
    camera: &Camera,
    target: [f32; 3],
    screen_w: f32,
    screen_h: f32,
) -> Vec<UiDrawCall> {
    let Some((sx, sy)) =
        camera.world_to_screen(target[0], target[1], target[2], screen_w, screen_h)
    else {
        return Vec::new();
    };
    const SIZE: f32 = 10.0;
    const THICK: f32 = 2.0;
    const COLOR: [f32; 4] = [0.2, 1.0, 0.2, 0.9];
    let mut calls = Vec::new();
    let (hv, hi) =
        ragnarok_ui::draw::quad_vertices(sx - SIZE, sy - THICK * 0.5, SIZE * 2.0, THICK, COLOR);
    calls.push(UiDrawCall {
        vertices: hv.to_vec(),
        indices: hi.to_vec(),
        texture: UiTextureRef::White,
    });
    let (vv, vi) =
        ragnarok_ui::draw::quad_vertices(sx - THICK * 0.5, sy - SIZE, THICK, SIZE * 2.0, COLOR);
    calls.push(UiDrawCall {
        vertices: vv.to_vec(),
        indices: vi.to_vec(),
        texture: UiTextureRef::White,
    });
    calls
}
