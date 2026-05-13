// Force System allocator so Vec/String allocations are interchangeable with
// the host binary across the cdylib boundary.
#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

use ragnarok_game::effect::{
    ALL_EFFECT_IDS, CustomFamily, EffectId, EffectSpec, effect_ef_name, effect_name, effect_spec,
};
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
}

// === Internal state ===

/// Picker draws from `ALL_EFFECT_IDS` directly - every `EF_*` ID is
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
    StrHybrid,
    Spr,
    Bespoke,
    Aura,
    GroundRing,
    CastCircle,
    SpikeRow,
    Wall,
    CylinderPillar,
    CrossBeam,
    SplineProjectile,
    RadialBurst,
    ScreenFlash,
    FlatQuad,
    HealBurst,
    MeleeImpact,
    AirSwirl,
    StatusOrb,
    FloatingSpirit,
    Waterfall,
}

const FILTERS: &[Filter] = &[
    Filter::All,
    Filter::Str,
    Filter::StrHybrid,
    Filter::Spr,
    Filter::Bespoke,
    Filter::Aura,
    Filter::GroundRing,
    Filter::CastCircle,
    Filter::SpikeRow,
    Filter::Wall,
    Filter::CylinderPillar,
    Filter::CrossBeam,
    Filter::SplineProjectile,
    Filter::RadialBurst,
    Filter::ScreenFlash,
    Filter::FlatQuad,
    Filter::HealBurst,
    Filter::MeleeImpact,
    Filter::AirSwirl,
    Filter::StatusOrb,
    Filter::FloatingSpirit,
    Filter::Waterfall,
];

fn filter_label(f: Filter) -> &'static str {
    match f {
        Filter::All => "All",
        Filter::Str => "Str",
        Filter::StrHybrid => "StrHybrid",
        Filter::Spr => "Spr",
        Filter::Bespoke => "Bespoke",
        Filter::Aura => "Aura",
        Filter::GroundRing => "GroundRing",
        Filter::CastCircle => "CastCircle",
        Filter::SpikeRow => "SpikeRow",
        Filter::Wall => "Wall",
        Filter::CylinderPillar => "CylinderPillar",
        Filter::CrossBeam => "CrossBeam",
        Filter::SplineProjectile => "SplineProjectile",
        Filter::RadialBurst => "RadialBurst",
        Filter::ScreenFlash => "ScreenFlash",
        Filter::FlatQuad => "FlatQuad",
        Filter::HealBurst => "HealBurst",
        Filter::MeleeImpact => "MeleeImpact",
        Filter::AirSwirl => "AirSwirl",
        Filter::StatusOrb => "StatusOrb",
        Filter::FloatingSpirit => "FloatingSpirit",
        Filter::Waterfall => "Waterfall",
    }
}

fn filter_to_family(f: Filter) -> Option<CustomFamily> {
    Some(match f {
        Filter::Aura => CustomFamily::Aura,
        Filter::GroundRing => CustomFamily::GroundRing,
        Filter::CastCircle => CustomFamily::CastCircle,
        Filter::SpikeRow => CustomFamily::SpikeRow,
        Filter::Wall => CustomFamily::Wall,
        Filter::CylinderPillar => CustomFamily::CylinderPillar,
        Filter::CrossBeam => CustomFamily::CrossBeam,
        Filter::SplineProjectile => CustomFamily::SplineProjectile,
        Filter::RadialBurst => CustomFamily::RadialBurst,
        Filter::ScreenFlash => CustomFamily::ScreenFlash,
        Filter::FlatQuad => CustomFamily::FlatQuad,
        Filter::HealBurst => CustomFamily::HealBurst,
        Filter::MeleeImpact => CustomFamily::MeleeImpact,
        Filter::AirSwirl => CustomFamily::AirSwirl,
        Filter::StatusOrb => CustomFamily::StatusOrb,
        Filter::FloatingSpirit => CustomFamily::FloatingSpirit,
        Filter::Waterfall => CustomFamily::Waterfall,
        _ => return None,
    })
}

fn filter_matches(f: Filter, id: EffectId) -> bool {
    if matches!(f, Filter::All) {
        return true;
    }
    let Some(spec) = effect_spec(id) else {
        return false;
    };
    let target = filter_to_family(f);
    match (f, spec) {
        (Filter::Str, EffectSpec::Str { .. }) => true,
        (Filter::StrHybrid, EffectSpec::StrHybrid { .. }) => true,
        (Filter::Spr, EffectSpec::Spr { .. }) => true,
        (
            Filter::Bespoke,
            EffectSpec::Custom {
                family: CustomFamily::Bespoke(_),
                ..
            },
        ) => true,
        (_, EffectSpec::Custom { family, .. }) => target == Some(family),
        (_, EffectSpec::StrHybrid { family, .. }) => target == Some(family),
        _ => false,
    }
}

fn build_filtered(filter: Filter) -> Vec<EffectId> {
    ALL_EFFECT_IDS
        .iter()
        .copied()
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

    // Orbit camera around the spawn point.
    target: [f32; 3],
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl State {
    fn default_camera(&mut self) {
        // Match Camera::default() so the initial view is consistent with the
        // renderer's untouched camera.
        self.target = [0.0, 0.0, 0.0];
        self.yaw = 0.0;
        self.pitch = 55_f32.to_radians();
        self.distance = 200.0;
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
            "{} ({})  [{}/{}]",
            effect_name(id),
            effect_ef_name(id),
            self.selection + 1,
            self.filtered_ids.len(),
        )
    }

    fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(20.0, 2000.0);
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
        target: [0.0; 3],
        yaw: 0.0,
        pitch: 0.0,
        distance: 0.0,
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
pub unsafe extern "C" fn hot_get_flags(state_ptr: *mut (), out: *mut ViewerFlags) {
    let state = unsafe { &*(state_ptr as *const State) };
    let f = ViewerFlags {
        paused: state.paused as u8,
        show_info: state.show_info as u8,
        _pad0: [0; 2],
        speed_x100: (state.speed * 100.0) as u32,
        selected_effect_id: state.current_effect().map(|id| id.as_u16()).unwrap_or(u16::MAX),
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
            effect_id: id.as_u16(),
            valid: 1,
            _pad: 0,
            world_pos: [0.0, 0.0, 0.0],
        },
        None => PendingSpawn::default(),
    };
    unsafe { *out = result };
}

/// Writes the currently filtered effect IDs (sorted as the dylib stores them)
/// into the host-provided Vec. Used by the host browser overlay to build its
/// item list without duplicating filter logic.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_get_filtered_ids(state_ptr: *mut (), out: *mut Vec<u16>) {
    let state = unsafe { &*(state_ptr as *const State) };
    let out = unsafe { &mut *out };
    out.clear();
    out.extend(state.filtered_ids.iter().map(|id| id.as_u16()));
}

/// Host-initiated selection: jump the picker to the given effect id within the
/// current filter and queue a spawn. No-op if the id isn't in the filter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_set_selected_effect_id(state_ptr: *mut (), effect_id: u16) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    let Some(id) = EffectId::from_u16(effect_id) else {
        return;
    };
    if let Some(idx) = state.filtered_ids.iter().position(|x| *x == id) {
        state.selection = idx;
        state.pending_spawn = Some(id);
    }
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
    ("+ / -", "Speed"),
    ("Scroll", "Zoom"),
    ("C", "Reset camera"),
    ("1", "Toggle controls"),
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
    "  Down         Next filter (All, Str, StrHybrid, Spr, Bespoke,",
    "               then each CustomFamily)",
    "  Up           Previous filter",
    "",
    "Playback:",
    "  Space        Pause / resume",
    "  + / -        Speed up / down (0.1x - 4.0x)",
    "",
    "Camera:",
    "  Scroll       Zoom in / out",
    "  C            Reset camera to default",
    "",
    "Window:",
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
