// Force System allocator so Vec/String allocations are interchangeable with
// the host binary across the cdylib boundary.
#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

use ragnarok_game::effect::EffectId;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::{UiDrawCall, UiTextureRef};
use ragnarok_ui::draw::{quad_vertices, text_vertices};

// === Action codes (FFI contract — host duplicates these) ===
pub const ACTION_NEXT_EFFECT: u32 = 1;
pub const ACTION_PREV_EFFECT: u32 = 2;
pub const ACTION_RESPAWN: u32 = 3;
pub const ACTION_TOGGLE_PAUSE: u32 = 4;
pub const ACTION_SPEED_UP: u32 = 5;
pub const ACTION_SPEED_DOWN: u32 = 6;

// === C-ABI POD types (host duplicates exactly) ===

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ViewerFlags {
    pub paused: u8,
    pub _pad0: [u8; 3],
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

// === Internal state ===

/// Hand-curated list of effects available in the picker. Matches the
/// `EffectId` enum in `lib/game/src/effect/id.rs` — when that enum grows
/// (Phase B XML port) this list will be code-generated.
const PICKABLE: &[(EffectId, &str)] = &[
    (EffectId::Bubble, "Bubble (STR)"),
    (EffectId::GasPush, "GasPush (STR)"),
    (EffectId::Spring, "Spring (STR)"),
    (EffectId::FireBolt, "FireBolt (STR)"),
    (EffectId::LightningBolt, "LightningBolt (STR)"),
    (EffectId::Lvup, "Lvup (STR)"),
    (EffectId::JobLvup, "JobLvup (STR)"),
    (EffectId::RefineOk, "RefineOk (STR)"),
    (EffectId::RefineFail, "RefineFail (STR)"),
    (EffectId::Potion1, "Potion1 (STR)"),
    (EffectId::Level99, "Lv99 Aura (Custom)"),
    (EffectId::IceWall, "IceWall (Custom)"),
    (EffectId::GrimTooth, "GrimTooth (Custom)"),
];

struct State {
    paused: bool,
    speed: f32,
    /// Index into PICKABLE.
    selection: usize,
    /// Set to Some(...) when an effect should be spawned; host reads this
    /// once per frame via `hot_take_pending_spawn`.
    pending_spawn: Option<EffectId>,
}

impl State {
    fn current_effect(&self) -> EffectId {
        PICKABLE[self.selection].0
    }

    fn current_label(&self) -> &'static str {
        PICKABLE[self.selection].1
    }

    fn cycle(&mut self, forward: bool) {
        let n = PICKABLE.len();
        self.selection = if forward {
            (self.selection + 1) % n
        } else {
            (self.selection + n - 1) % n
        };
        self.pending_spawn = Some(self.current_effect());
    }

    fn respawn(&mut self) {
        self.pending_spawn = Some(self.current_effect());
    }
}

// === FFI exports ===

#[unsafe(no_mangle)]
pub extern "C" fn hot_create() -> *mut () {
    let state = State {
        paused: false,
        speed: 1.0,
        selection: 0,
        // Spawn the first effect on startup so something is visible immediately.
        pending_spawn: Some(PICKABLE[0].0),
    };
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
    // No time-driven state in the cdylib yet — host owns the holder + clock.
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_on_action(state_ptr: *mut (), action_code: u32) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    match action_code {
        ACTION_NEXT_EFFECT => state.cycle(true),
        ACTION_PREV_EFFECT => state.cycle(false),
        ACTION_RESPAWN => state.respawn(),
        ACTION_TOGGLE_PAUSE => state.paused = !state.paused,
        ACTION_SPEED_UP => state.speed = (state.speed + 0.25).min(4.0),
        ACTION_SPEED_DOWN => state.speed = (state.speed - 0.25).max(0.1),
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_get_flags(state_ptr: *mut (), out: *mut ViewerFlags) {
    let state = unsafe { &*(state_ptr as *const State) };
    let f = ViewerFlags {
        paused: state.paused as u8,
        _pad0: [0; 3],
        speed_x100: (state.speed * 100.0) as u32,
        selected_effect_id: state.current_effect().as_u16(),
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
}

// === Overlay builders ===

const PADDING: f32 = 8.0;
const LINE_HEIGHT: f32 = 16.0;
const KEY_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.9];
const DESC_COLOR: [f32; 4] = [0.7, 0.7, 0.7, 0.7];
const BG_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.5];

const LEGEND: &[(&str, &str)] = &[
    ("→ / Space", "Next effect"),
    ("←", "Previous effect"),
    ("R", "Respawn current"),
    ("P", "Pause / resume"),
    ("+ / -", "Speed up / down"),
    ("Esc", "Quit"),
];

fn build_status(atlas: &FontAtlas, screen_w: f32, state: &State) -> Vec<UiDrawCall> {
    let pause_str = if state.paused { " [PAUSED]" } else { "" };
    let text = format!(
        "Effect: {}  Speed: {:.2}x{}",
        state.current_label(),
        state.speed,
        pause_str
    );
    let text_w = atlas.measure_text(&text);
    let box_w = text_w + PADDING * 2.0;
    let box_h = LINE_HEIGHT + PADDING * 2.0;
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
