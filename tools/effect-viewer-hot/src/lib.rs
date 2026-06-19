// Force System allocator so Vec/String allocations are interchangeable with
// the host binary across the cdylib boundary.
#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

use models::enums::EnumWithNumberValue;
use models::enums::EnumWithStringValue;
use models::enums::effect_id::EffectId;
use ragnarok_game::effect::spec::EffectAnchor;
use ragnarok_game::effect::{
    Effect as GameEffect, EffectDrawList,
    EffectRenderCtx as GameEffectRenderCtx, EffectSpec, EffectStatus,
    EffectUpdateCtx as GameEffectUpdateCtx, effect_spec, make_effect,
};
use std::collections::HashMap;
use std::sync::Mutex;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::{UiDrawCall, UiTextureRef};
use ragnarok_ui::draw::{quad_vertices, text_vertices};

// === Action codes (FFI contract - host duplicates these) ===
pub const ACTION_NEXT_EFFECT: u32 = 1;
pub const ACTION_PREV_EFFECT: u32 = 2;
pub const ACTION_RESPAWN: u32 = 3;
pub const ACTION_TOGGLE_PAUSE: u32 = 4;
pub const ACTION_SPEED_UP: u32 = 5;
pub const ACTION_SPEED_DOWN: u32 = 6;
pub const ACTION_SHOW_CONTROLS: u32 = 7;
pub const ACTION_CLOSE_INFO_PANEL: u32 = 8;
pub const ACTION_RESET_CAMERA: u32 = 9;
pub const ACTION_PAGE_DOWN: u32 = 10;
pub const ACTION_PAGE_UP: u32 = 11;
pub const ACTION_HOME: u32 = 12;
pub const ACTION_END: u32 = 13;
pub const ACTION_NEXT_FILTER: u32 = 14;
pub const ACTION_PREV_FILTER: u32 = 15;
pub const ACTION_FOV_NARROWER: u32 = 16;
pub const ACTION_FOV_WIDER: u32 = 17;
pub const ACTION_STEP_FRAME: u32 = 18;

// === C-ABI POD types (host duplicates exactly) ===

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ViewerFlags {
    pub paused: u8,
    /// Mirrors `InfoPanel`: 0 = None, 1 = Controls.
    pub show_info: u8,
    pub _pad0: [u8; 2],
    pub speed_x100: u32,
    pub selected_effect_id: u16,
    pub _pad1: [u8; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PendingSpawn {
    pub effect_id: u16,
    pub valid: u8,
    pub _pad: u8,
    pub world_pos: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CameraView {
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    /// Vertical field-of-view in radians. The host applies this each frame so
    /// the effect viewer's preferred wider FOV doesn't bleed across other
    /// tools sharing the renderer's `Camera::default()`.
    pub fov_y: f32,
}

/// Snapshot of every field the host needs to carry across a hot-reload so the
/// new dylib's `State` resumes where the old one left off. `magic` + `version`
/// let the host abandon the restore cleanly if the cdylib bumps the layout
/// between rebuilds.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PersistentState {
    pub magic: u32,
    pub version: u32,
    pub selected_effect_id: u16,
    pub filter_idx: u16,
    pub paused: u8,
    pub show_info: u8,
    pub _pad: [u8; 2],
    pub speed_x100: u32,
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub fov_y: f32,
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

pub const PERSISTENT_STATE_MAGIC: u32 = 0x45565053; // 'EVPS' little-endian
/// Bumped to 2 when the camera profile grew a `fov_y` field. Older
/// snapshots are discarded by the version check in `hot_restore_state`.
pub const PERSISTENT_STATE_VERSION: u32 = 2;

// === Internal state ===

/// Picker draws from `ALL_EFFECT_IDS` directly - every original game `EF_*` ID is
/// selectable (~821 variants today). Names + EF_ aliases come from the
/// generated helpers.

/// How many list entries to skip when paging.
const PAGE_SIZE: usize = 25;

#[derive(Clone, Copy, PartialEq)]
enum InfoPanel {
    None = 0,
    Controls = 1,
}

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    All,
    Str,
    Spr,
    Custom,
}

const FILTERS: &[Filter] = &[Filter::All, Filter::Str, Filter::Spr, Filter::Custom];

fn filter_label(f: Filter) -> &'static str {
    match f {
        Filter::All => "All",
        Filter::Str => "Str",
        Filter::Spr => "Spr",
        Filter::Custom => "Custom",
    }
}

fn filter_matches(f: Filter, id: EffectId) -> bool {
    let Some(spec) = effect_spec(id) else {
        return false;
    };
    if matches!(spec, EffectSpec::Noop) {
        return false;
    }
    if matches!(f, Filter::All) {
        return true;
    }
    matches!(
        (f, spec),
        (Filter::Str, EffectSpec::Str { .. })
            | (Filter::Spr, EffectSpec::Spr { .. })
            | (Filter::Custom, EffectSpec::Custom { .. })
    )
}

fn build_filtered(filter: Filter) -> Vec<EffectId> {
    (0..=2027usize)
        .filter_map(|v| EffectId::try_from_value(v).ok())
        .filter(|id| filter_matches(filter, *id))
        .collect()
}

struct State {
    paused: bool,
    speed: f32,
    /// Index into `filtered_ids`.
    selection: usize,
    filter_idx: usize,
    filtered_ids: Vec<EffectId>,
    /// Set to Some(...) when an effect should be spawned; host reads this
    /// once per frame via `hot_take_pending_spawn`.
    pending_spawn: Option<EffectId>,
    show_info: InfoPanel,
    last_status: u8,
    /// Set by `ACTION_STEP_FRAME` while paused; the host consumes it via
    /// `hot_take_step_request` to advance one 1/60s tick. Not persisted across
    /// hot-reloads — stepping is per-keystroke and shouldn't survive a rebuild.
    step_pending: bool,

    // Orbit camera around the spawn point.
    target: [f32; 3],
    yaw: f32,
    pitch: f32,
    distance: f32,
    /// Vertical field-of-view in radians. The renderer's `Camera::default()`
    /// uses 15° (long telephoto, tuned for the isometric map) which crowds a
    /// single effect at the origin; the viewer overrides this to a wider FOV.
    fov_y: f32,

    /// Hot-reloadable effect registry. The host holds u64 handles only;
    /// `Box<dyn Effect>` ownership stays in this cdylib so it can be torn
    /// down before the dylib unloads.
    effects: Mutex<HashMap<u64, Box<dyn GameEffect>>>,
    next_effect_handle: Mutex<u64>,
}

/// Lower / upper bounds for camera zoom (world units). Range chosen so the
/// smallest effects (radius ~1.5) still fill a sensible fraction of the
/// viewport at `DISTANCE_MIN`, and the largest auras (radius ~26) still leave
/// generous margin at `DISTANCE_MAX`.
const DISTANCE_MIN: f32 = 30.0;
const DISTANCE_MAX: f32 = 800.0;
/// Lower / upper bounds for vertical FOV (radians).
const FOV_MIN: f32 = 15_f32 * std::f32::consts::PI / 180.0;
const FOV_MAX: f32 = 85_f32 * std::f32::consts::PI / 180.0;
/// Right-drag sensitivity (radians per pixel) for orbit yaw/pitch.
const ORBIT_SENSITIVITY: f32 = 0.005;

impl State {
    fn default_camera(&mut self) {
        // Effect-viewer profile: wider FOV + larger distance than the map
        // camera so a typical effect occupies a comfortable fraction of the
        // viewport instead of crowding it. With fov_y = 55° and distance =
        // 120, the visible height at the target is ~125 world units — a
        // 26-unit Aura billboard reads at ~20% of the screen, leaving room
        // around it. Pitch stays low enough (28°) for vertical primitives
        // (cones, pillars) to read as 3D rather than concentric rings.
        self.target = [0.0, 0.0, 0.0];
        self.yaw = 0.0;
        self.pitch = 28_f32.to_radians();
        self.distance = 120.0;
        self.fov_y = 55_f32.to_radians();
    }

    fn adjust_fov(&mut self, factor: f32) {
        self.fov_y = (self.fov_y * factor).clamp(FOV_MIN, FOV_MAX);
    }

    fn orbit(&mut self, dx: f32, dy: f32) {
        use std::f32::consts::FRAC_PI_2;
        self.yaw -= dx * ORBIT_SENSITIVITY;
        // Clamp pitch just shy of poles to avoid the up vector degenerating
        // when the eye crosses through the world's Y axis.
        self.pitch = (self.pitch - dy * ORBIT_SENSITIVITY).clamp(0.05, FRAC_PI_2 - 0.05);
    }
}

impl State {
    fn current_effect(&self) -> Option<EffectId> {
        self.filtered_ids.get(self.selection).copied()
    }

    fn current_filter(&self) -> Filter {
        FILTERS[self.filter_idx]
    }

    fn current_label(&self) -> String {
        let Some(id) = self.current_effect() else {
            return format!(
                "(no effects in {} filter)",
                filter_label(self.current_filter())
            );
        };
        format!(
            "{:?} ({})  [{}/{}]",
            id,
            id.value(),
            self.selection + 1,
            self.filtered_ids.len(),
        )
    }

    fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(DISTANCE_MIN, DISTANCE_MAX);
    }

    fn cycle(&mut self, forward: bool) {
        let n = self.filtered_ids.len();
        if n == 0 {
            return;
        }
        self.selection = if forward {
            (self.selection + 1) % n
        } else {
            (self.selection + n - 1) % n
        };
        self.pending_spawn = self.current_effect();
    }

    fn page(&mut self, forward: bool) {
        let n = self.filtered_ids.len();
        if n == 0 {
            return;
        }
        self.selection = if forward {
            (self.selection + PAGE_SIZE).min(n - 1)
        } else {
            self.selection.saturating_sub(PAGE_SIZE)
        };
        self.pending_spawn = self.current_effect();
    }

    fn jump_home(&mut self) {
        if self.filtered_ids.is_empty() {
            return;
        }
        self.selection = 0;
        self.pending_spawn = self.current_effect();
    }

    fn jump_end(&mut self) {
        if self.filtered_ids.is_empty() {
            return;
        }
        self.selection = self.filtered_ids.len() - 1;
        self.pending_spawn = self.current_effect();
    }

    fn respawn(&mut self) {
        self.pending_spawn = self.current_effect();
    }

    fn cycle_filter(&mut self, forward: bool) {
        let n = FILTERS.len();
        self.filter_idx = if forward {
            (self.filter_idx + 1) % n
        } else {
            (self.filter_idx + n - 1) % n
        };
        self.filtered_ids = build_filtered(self.current_filter());
        self.selection = 0;
        self.pending_spawn = self.current_effect();
    }
}

// === FFI exports ===

#[unsafe(no_mangle)]
pub extern "C" fn hot_create() -> *mut () {
    let filtered_ids = build_filtered(Filter::All);
    let first = filtered_ids.first().copied();
    let mut state = State {
        paused: false,
        speed: 1.0,
        selection: 0,
        filter_idx: 0,
        filtered_ids,
        pending_spawn: first,
        show_info: InfoPanel::None,
        last_status: 0,
        step_pending: false,
        target: [0.0; 3],
        yaw: 0.0,
        pitch: 0.0,
        distance: 0.0,
        fov_y: 0.0,
        effects: Mutex::new(HashMap::new()),
        // Reserve 0 as "invalid handle" so the host can use 0 to signal
        // spawn failure across the FFI boundary.
        next_effect_handle: Mutex::new(1),
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
pub unsafe extern "C" fn hot_update(state_ptr: *mut (), _dt: f32) {
    let _state = unsafe { &mut *(state_ptr as *mut State) };
    // No time-driven state in the cdylib yet - host owns the holder + clock.
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_on_action(state_ptr: *mut (), action_code: u32) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    match action_code {
        ACTION_NEXT_EFFECT => state.cycle(true),
        ACTION_PREV_EFFECT => state.cycle(false),
        ACTION_PAGE_DOWN => state.page(true),
        ACTION_PAGE_UP => state.page(false),
        ACTION_HOME => state.jump_home(),
        ACTION_END => state.jump_end(),
        ACTION_RESPAWN => state.respawn(),
        ACTION_TOGGLE_PAUSE => state.paused = !state.paused,
        ACTION_SPEED_UP => state.speed = (state.speed + 0.25).min(4.0),
        ACTION_SPEED_DOWN => state.speed = (state.speed - 0.25).max(0.1),
        ACTION_SHOW_CONTROLS => {
            state.show_info = match state.show_info {
                InfoPanel::Controls => InfoPanel::None,
                _ => InfoPanel::Controls,
            };
        }
        ACTION_CLOSE_INFO_PANEL => state.show_info = InfoPanel::None,
        ACTION_RESET_CAMERA => state.default_camera(),
        ACTION_NEXT_FILTER => state.cycle_filter(true),
        ACTION_PREV_FILTER => state.cycle_filter(false),
        ACTION_FOV_NARROWER => state.adjust_fov(0.9),
        ACTION_FOV_WIDER => state.adjust_fov(1.1),
        ACTION_STEP_FRAME => {
            if state.paused {
                state.step_pending = true;
            }
        }
        _ => {}
    }
}

/// Mouse wheel: positive `dy` = scroll up = zoom in (closer to target).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_on_mouse_wheel(state_ptr: *mut (), dy: f32) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    if dy > 0.0 {
        state.zoom(0.9);
    } else if dy < 0.0 {
        state.zoom(1.1);
    }
}

/// Right-drag orbit: `dx` and `dy` in pixels since the last cursor sample.
/// `button` is reserved for future left-vs-right disambiguation (the host
/// already filters); the dylib treats every drag as an orbit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_on_mouse_drag(state_ptr: *mut (), dx: f32, dy: f32, _button: u8) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    state.orbit(dx, dy);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_get_camera(state_ptr: *mut (), out: *mut CameraView) {
    let state = unsafe { &*(state_ptr as *const State) };
    let v = CameraView {
        target: state.target,
        yaw: state.yaw,
        pitch: state.pitch,
        distance: state.distance,
        fov_y: state.fov_y,
    };
    unsafe { *out = v };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_get_flags(state_ptr: *mut (), out: *mut ViewerFlags) {
    let state = unsafe { &*(state_ptr as *const State) };
    let f = ViewerFlags {
        paused: state.paused as u8,
        show_info: state.show_info as u8,
        _pad0: [0; 2],
        speed_x100: (state.speed * 100.0) as u32,
        selected_effect_id: state.current_effect().map(|id| id.value() as u16).unwrap_or(u16::MAX),
        _pad1: [0; 2],
    };
    unsafe { *out = f };
}

/// Returns and clears any pending spawn request. Host calls once per frame.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_take_pending_spawn(state_ptr: *mut (), out: *mut PendingSpawn) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    let result = match state.pending_spawn.take() {
        Some(id) => PendingSpawn {
            effect_id: id.value() as u16,
            valid: 1,
            _pad: 0,
            world_pos: [0.0, 0.0, 0.0],
        },
        None => PendingSpawn::default(),
    };
    unsafe { *out = result };
}

/// Pops the pending single-frame step request. Returns 1 if a step was
/// queued (host should advance one 1/60s tick), 0 otherwise.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_take_step_request(state_ptr: *mut ()) -> u8 {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    if state.step_pending {
        state.step_pending = false;
        1
    } else {
        0
    }
}

/// Writes the currently filtered effect IDs (sorted as the dylib stores them)
/// into the host-provided Vec. Used by the host browser overlay to build its
/// item list without duplicating filter logic.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_get_filtered_ids(state_ptr: *mut (), out: *mut Vec<u16>) {
    let state = unsafe { &*(state_ptr as *const State) };
    let out = unsafe { &mut *out };
    out.clear();
    out.extend(state.filtered_ids.iter().map(|id| id.value() as u16));
}

/// Host-initiated selection: jump the picker to the given effect id within the
/// current filter and queue a spawn. No-op if the id isn't in the filter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_set_selected_effect_id(state_ptr: *mut (), effect_id: u16) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    let Some(id) = EffectId::try_from_value(effect_id as usize).ok() else {
        return;
    };
    if let Some(idx) = state.filtered_ids.iter().position(|x| *x == id) {
        state.selection = idx;
        state.pending_spawn = Some(id);
    }
}

/// Writes the dylib's persistent state into `out` so the host can carry it
/// across an unload/load cycle. Host treats the struct as opaque: it copies
/// the bytes out, destroys the old dylib, loads the new one, then hands the
/// bytes to `hot_restore_state`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_snapshot_state(state_ptr: *mut (), out: *mut PersistentState) {
    let state = unsafe { &*(state_ptr as *const State) };
    let snap = PersistentState {
        magic: PERSISTENT_STATE_MAGIC,
        version: PERSISTENT_STATE_VERSION,
        selected_effect_id: state
            .current_effect()
            .map(|id| id.value() as u16)
            .unwrap_or(u16::MAX),
        filter_idx: state.filter_idx as u16,
        paused: state.paused as u8,
        show_info: state.show_info as u8,
        _pad: [0; 2],
        speed_x100: (state.speed * 100.0) as u32,
        target: state.target,
        yaw: state.yaw,
        pitch: state.pitch,
        distance: state.distance,
        fov_y: state.fov_y,
    };
    unsafe { *out = snap };
}

/// Restore a previously snapshotted state. Returns 1 on success, 0 if the
/// snapshot's magic/version don't match (host should leave the fresh state
/// untouched in that case). Re-queues a spawn for the restored effect so the
/// next frame's `poll_pending_spawn` repopulates the holder after the host
/// cleared it for the unload.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_restore_state(state_ptr: *mut (), snap: *const PersistentState) -> u8 {
    if snap.is_null() {
        return 0;
    }
    let snap = unsafe { &*snap };
    if snap.magic != PERSISTENT_STATE_MAGIC || snap.version != PERSISTENT_STATE_VERSION {
        return 0;
    }
    let state = unsafe { &mut *(state_ptr as *mut State) };

    let filter_idx = (snap.filter_idx as usize).min(FILTERS.len().saturating_sub(1));
    state.filter_idx = filter_idx;
    state.filtered_ids = build_filtered(state.current_filter());

    state.selection = match EffectId::try_from_value(snap.selected_effect_id as usize).ok() {
        Some(id) => state
            .filtered_ids
            .iter()
            .position(|x| *x == id)
            .unwrap_or(0),
        None => 0,
    };

    state.paused = snap.paused != 0;
    state.show_info = match snap.show_info {
        1 => InfoPanel::Controls,
        _ => InfoPanel::None,
    };
    state.speed = (snap.speed_x100 as f32 / 100.0).clamp(0.1, 4.0);
    state.target = snap.target;
    state.yaw = snap.yaw;
    state.pitch = snap.pitch;
    state.distance = snap.distance;
    // FOV may legitimately be 0 in a future schema bump; fall back to the
    // viewer's default if so.
    state.fov_y = if snap.fov_y > 0.0 {
        snap.fov_y
    } else {
        35_f32.to_radians()
    };

    state.pending_spawn = state.current_effect();
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_set_last_status(state_ptr: *mut (), status: u8) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    state.last_status = status;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_build_overlay(
    state_ptr: *mut (),
    atlas: *const FontAtlas,
    screen_w: f32,
    screen_h: f32,
    out: *mut Vec<UiDrawCall>,
) {
    if atlas.is_null() {
        return;
    }
    let state = unsafe { &*(state_ptr as *const State) };
    let atlas = unsafe { &*atlas };
    let out = unsafe { &mut *out };

    out.extend(build_status(atlas, screen_w, state));
    out.extend(build_legend(atlas, screen_h));
    if state.show_info == InfoPanel::Controls {
        out.extend(build_controls_panel(atlas, screen_w, screen_h));
    }
}

// === Overlay builders ===

const PADDING: f32 = 8.0;
const LINE_HEIGHT: f32 = 16.0;
const KEY_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.9];
const DESC_COLOR: [f32; 4] = [0.7, 0.7, 0.7, 0.7];
const BG_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.5];

const LEGEND: &[(&str, &str)] = &[
    ("-> / <-", "Next / prev effect"),
    ("Up / Down", "Filter -/+"),
    ("PgDn / PgUp", "Jump 25"),
    ("Home / End", "First / last"),
    ("Tab", "Browser"),
    ("R", "Replay"),
    ("Space", "Pause"),
    ("N", "Step frame"),
    ("+ / -", "Speed"),
    ("Scroll", "Zoom"),
    ("Right drag", "Orbit"),
    ("[ / ]", "FOV -/+"),
    ("C", "Reset camera"),
    ("B", "Toggle background"),
    ("1", "Toggle controls"),
    ("T", "Place trail target"),
    ("X", "Clear trail target"),
    ("Esc", "Quit / close panel"),
];

const CONTROLS_LINES: &[&str] = &[
    "Picker:",
    "  ->           Next effect (wraps)",
    "  <-           Previous effect (wraps)",
    "  PgDn / PgUp  Jump 25 entries forward / back",
    "  Home / End   Jump to first / last effect",
    "  Tab          Open browser (filter by typing, Enter to pick)",
    "  R            Replay current effect at origin",
    "",
    "Filter:",
    "  Down         Next filter (All, Str, Spr, Custom)",
    "  Up           Previous filter",
    "",
    "Playback:",
    "  Space        Pause / resume",
    "  N            Step one frame (1/60s) while paused",
    "  + / -        Speed up / down (0.1x - 4.0x)",
    "",
    "Camera:",
    "  Scroll       Zoom in / out (distance)",
    "  Right drag   Orbit yaw / pitch",
    "  [  /  ]      Narrow / widen FOV",
    "  C            Reset camera to default",
    "",
    "Window:",
    "  B            Toggle background (blue / black)",
    "  1            Toggle this panel",
    "  Esc          Close panel (or quit if none open)",
];

fn build_status(atlas: &FontAtlas, screen_w: f32, state: &State) -> Vec<UiDrawCall> {
    let pause_str = if state.paused { " [PAUSED]" } else { "" };
    let filter = state.current_filter();
    let filter_text = format!(
        "Filter: {} ({})",
        filter_label(filter),
        state.filtered_ids.len()
    );
    let text = format!(
        "Effect: {}  Speed: {:.2}x{}",
        state.current_label(),
        state.speed,
        pause_str
    );
    let (badge_label, badge_color) = status_badge(state.last_status);
    let gap = "  ";
    let text_w = atlas.measure_text(&text);
    let gap_w = atlas.measure_text(gap);
    let badge_w = atlas.measure_text(badge_label);
    let filter_w = atlas.measure_text(&filter_text);
    let line1_w = text_w + gap_w + badge_w;
    let inner_w = line1_w.max(filter_w);
    let box_w = inner_w + PADDING * 2.0;
    let box_h = LINE_HEIGHT * 2.0 + PADDING * 2.0;
    let box_x = (screen_w - box_w) / 2.0;
    let box_y = PADDING;

    let mut calls = Vec::new();
    let (bv, bi) = quad_vertices(box_x, box_y, box_w, box_h, BG_COLOR);
    calls.push(UiDrawCall {
        vertices: bv.to_vec(),
        indices: bi.to_vec(),
        texture: UiTextureRef::White,
    });
    let (tv, ti) = text_vertices(&text, box_x + PADDING, box_y + PADDING, KEY_COLOR, atlas);
    calls.push(UiDrawCall {
        vertices: tv,
        indices: ti,
        texture: UiTextureRef::FontAtlas,
    });
    let badge_x = box_x + PADDING + text_w + gap_w;
    let (cv, ci) = text_vertices(badge_label, badge_x, box_y + PADDING, badge_color, atlas);
    calls.push(UiDrawCall {
        vertices: cv,
        indices: ci,
        texture: UiTextureRef::FontAtlas,
    });
    let (fv, fi) = text_vertices(
        &filter_text,
        box_x + PADDING,
        box_y + PADDING + LINE_HEIGHT,
        FILTER_COLOR,
        atlas,
    );
    calls.push(UiDrawCall {
        vertices: fv,
        indices: fi,
        texture: UiTextureRef::FontAtlas,
    });
    calls
}

const FILTER_COLOR: [f32; 4] = [0.6, 0.85, 1.0, 0.85];

pub const STATUS_UNKNOWN: u8 = 0;
pub const STATUS_RENDERING: u8 = 1;
pub const STATUS_STR_FILE_MISSING: u8 = 2;
pub const STATUS_CUSTOM_NOT_IMPL: u8 = 3;
pub const STATUS_NO_SPEC: u8 = 4;
pub const STATUS_CUSTOM_TEXTURE_MISSING: u8 = 5;

fn status_badge(code: u8) -> (&'static str, [f32; 4]) {
    match code {
        STATUS_RENDERING => ("[OK rendering]", [0.4, 1.0, 0.4, 1.0]),
        STATUS_STR_FILE_MISSING => ("[!! STR file missing]", [1.0, 0.85, 0.2, 1.0]),
        STATUS_CUSTOM_TEXTURE_MISSING => ("[!! texture missing]", [1.0, 0.85, 0.2, 1.0]),
        STATUS_CUSTOM_NOT_IMPL => ("[XX custom not impl]", [1.0, 0.3, 0.3, 1.0]),
        STATUS_NO_SPEC => ("[-- no spec]", [0.6, 0.6, 0.6, 1.0]),
        STATUS_UNKNOWN | _ => ("", [1.0, 1.0, 1.0, 1.0]),
    }
}

fn build_controls_panel(atlas: &FontAtlas, screen_w: f32, screen_h: f32) -> Vec<UiDrawCall> {
    let mut calls = Vec::new();
    let box_w = 360.0;
    let box_h = (CONTROLS_LINES.len() as f32 + 2.0) * LINE_HEIGHT + PADDING * 2.0;
    let box_x = (screen_w - box_w) / 2.0;
    let box_y = (screen_h - box_h) / 2.0;

    let (bv, bi) = quad_vertices(box_x, box_y, box_w, box_h, BG_COLOR);
    calls.push(UiDrawCall {
        vertices: bv.to_vec(),
        indices: bi.to_vec(),
        texture: UiTextureRef::White,
    });

    let (tv, ti) = text_vertices(
        "=== Effect Viewer Controls ===",
        box_x + PADDING,
        box_y + PADDING,
        KEY_COLOR,
        atlas,
    );
    calls.push(UiDrawCall {
        vertices: tv,
        indices: ti,
        texture: UiTextureRef::FontAtlas,
    });

    for (i, line) in CONTROLS_LINES.iter().enumerate() {
        let y = box_y + PADDING + LINE_HEIGHT * 1.5 + i as f32 * LINE_HEIGHT;
        let (lv, li) = text_vertices(line, box_x + PADDING, y, DESC_COLOR, atlas);
        calls.push(UiDrawCall {
            vertices: lv,
            indices: li,
            texture: UiTextureRef::FontAtlas,
        });
    }
    calls
}

fn build_legend(atlas: &FontAtlas, screen_h: f32) -> Vec<UiDrawCall> {
    let mut calls = Vec::new();
    let line_count = LEGEND.len() as f32;
    let box_h = line_count * LINE_HEIGHT + PADDING * 2.0;
    let box_w = 240.0;
    let box_x = PADDING;
    let box_y = screen_h - box_h - PADDING;

    let (bv, bi) = quad_vertices(box_x, box_y, box_w, box_h, BG_COLOR);
    calls.push(UiDrawCall {
        vertices: bv.to_vec(),
        indices: bi.to_vec(),
        texture: UiTextureRef::White,
    });

    let key_col_x = box_x + PADDING;
    let desc_col_x = box_x + 100.0;
    for (i, (key, desc)) in LEGEND.iter().enumerate() {
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

// === Hot-reloadable custom-effect registry ===
//
// The host's `EffectHolder` calls into these symbols whenever the spec is
// `EffectSpec::Custom`, so each effect's update + collect_draws runs out of
// THIS cdylib's text. When the dylib is replaced, the host drops all
// registry handles (via `hot_drop_all_effects`) before unloading.

/// Construct a custom effect by id. Returns 0 if the id has no factory arm.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_spawn_custom_effect(
    state_ptr: *mut (),
    effect_id: u16,
    from_ptr: *const [f32; 3],
    to_ptr: *const [f32; 3],
    hit_count: u8,
    target_w: f32,
    target_h: f32,
) -> u64 {
    let state = unsafe { &*(state_ptr as *const State) };
    let Some(id) = EffectId::try_from_value(effect_id as usize).ok() else {
        return 0;
    };
    let from = if from_ptr.is_null() { [0.0; 3] } else { unsafe { *from_ptr } };
    let to = if to_ptr.is_null() { from } else { unsafe { *to_ptr } };
    let anchor = if from == to {
        EffectAnchor::Point(from)
    } else {
        EffectAnchor::Trail { from, to }
    };
    let hc = if hit_count > 0 { Some(hit_count) } else { None };
    // NaN encodes "no target size" across the C ABI (no `Option<[f32; 2]>`).
    let target_size = if target_w.is_nan() || target_h.is_nan() {
        None
    } else {
        Some([target_w, target_h])
    };
    let Some(effect) = make_effect(id, anchor, hc, target_size) else {
        return 0;
    };
    let mut next = state.next_effect_handle.lock().unwrap();
    let handle = *next;
    *next = next.wrapping_add(1).max(1);
    drop(next);
    state.effects.lock().unwrap().insert(handle, effect);
    handle
}

/// Advance a registered effect by `dt`. Returns 1 if the effect signalled
/// `EffectStatus::Dead` (the host then calls `hot_drop_effect`), 0 if the
/// effect is still running or the handle is unknown.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_update_custom_effect(
    state_ptr: *mut (),
    handle: u64,
    dt: f32,
    caster_yaw: f32,
) -> u8 {
    let state = unsafe { &*(state_ptr as *const State) };
    let mut effects = state.effects.lock().unwrap();
    let Some(effect) = effects.get_mut(&handle) else {
        return 1;
    };
    // NaN means "no caster facing" (the host's C-ABI encoding of `None`).
    let caster_yaw = (!caster_yaw.is_nan()).then_some(caster_yaw);
    let status = effect.update(&GameEffectUpdateCtx { delta: dt, camera_target: None, caster_yaw });
    matches!(status, EffectStatus::Dead) as u8
}

/// Collect primitive draws for a registered effect, appending them to the
/// host-provided draw list. `ctx_ffi` carries the renderer-agnostic camera
/// + screen dims; if null we substitute defaults (good enough for picker
/// previews that don't drive billboard orientation).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EffectRenderCtxFfi {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub screen_w: f32,
    pub screen_h: f32,
    pub elapsed: f32,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_collect_custom_draws(
    state_ptr: *mut (),
    handle: u64,
    ctx_ffi: *const EffectRenderCtxFfi,
    out: *mut EffectDrawList,
) {
    let state = unsafe { &*(state_ptr as *const State) };
    let effects = state.effects.lock().unwrap();
    let Some(effect) = effects.get(&handle) else {
        return;
    };
    let ctx = if ctx_ffi.is_null() {
        GameEffectRenderCtx {
            camera: Default::default(),
            screen_w: 0.0,
            screen_h: 0.0,
            elapsed: 0.0,
        }
    } else {
        let c = unsafe { &*ctx_ffi };
        GameEffectRenderCtx {
            camera: ragnarok_game::effect::CameraView {
                eye: c.eye,
                target: c.target,
                up: c.up,
            },
            screen_w: c.screen_w,
            screen_h: c.screen_h,
            elapsed: c.elapsed,
        }
    };
    let out = unsafe { &mut *out };
    effect.collect_draws(out, &ctx);
}

/// FFI mirror of `ragnarok_game::effect::CameraShake` (must match the host).
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CameraShakeFfi {
    pub amplitude: f32,
    pub duration_ms: u32,
}

/// Drain a one-shot camera-shake request from a registered effect. Returns 1
/// and fills `out` when the effect fired a shake this frame, 0 otherwise.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_take_camera_shake(
    state_ptr: *mut (),
    handle: u64,
    out: *mut CameraShakeFfi,
) -> u8 {
    let state = unsafe { &*(state_ptr as *const State) };
    let mut effects = state.effects.lock().unwrap();
    let Some(effect) = effects.get_mut(&handle) else {
        return 0;
    };
    match effect.take_camera_shake() {
        Some(s) => {
            unsafe {
                *out = CameraShakeFfi {
                    amplitude: s.amplitude,
                    duration_ms: s.duration_ms,
                };
            }
            1
        }
        None => 0,
    }
}

/// Drop a single effect by handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_drop_custom_effect(state_ptr: *mut (), handle: u64) {
    let state = unsafe { &*(state_ptr as *const State) };
    state.effects.lock().unwrap().remove(&handle);
}

/// Drop every registered effect. Called by the host immediately before
/// unloading the dylib, so no in-flight effects survive into the new dylib
/// (their vtables would otherwise dangle).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_drop_all_custom_effects(state_ptr: *mut ()) {
    let state = unsafe { &*(state_ptr as *const State) };
    state.effects.lock().unwrap().clear();
}
