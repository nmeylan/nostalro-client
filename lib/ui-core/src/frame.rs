use crate::context::UiContext;
use crate::draw::{self, DrawCall, TextureRef};
use crate::rect::Rect;
use crate::state::StateCache;
use crate::text_input::TextInput;
use ragnarok_renderer::font_atlas::FontAtlas;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;

#[derive(Clone, Copy)]
pub enum TextInputBg<'a> {
    Default,
    Texture(&'a str),
    Transparent,
    Gray,
}

pub struct UiFrame<'a> {
    pub ctx: &'a UiContext,
    pub atlas: &'a FontAtlas,
    pub state: &'a mut StateCache,
    pub elapsed_secs: f32,
    pub has_grf_textures: bool,
    pub draw_calls: Vec<DrawCall>,
    pub tooltip_draw_calls: Vec<DrawCall>,
    pub any_hovered: bool,
    pub any_interactive_hovered: bool,
    focus: Option<WidgetId>,
    saved_positions: &'a HashMap<u32, [f32; 2]>,
    drag_started_this_frame: Option<WidgetId>,
    current_window: Option<WidgetId>,
    hovered_window: Option<WidgetId>,
    z_order_snapshot: Vec<WidgetId>,
    modal_layers: Vec<WidgetId>,
    in_popup_layer: bool,
    keyboard_blocked: bool,
    escape_consumed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u32);

impl Display for WidgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Default)]
pub struct WindowState {
    pub x: f32,
    pub y: f32,
    initialized: bool,
    dragging: bool,
    drag_offset_x: f32,
    drag_offset_y: f32,
}

pub struct ButtonTextures {
    pub normal: &'static str,
    pub hover: &'static str,
    pub pressed: &'static str,
}

pub struct CheckboxTextures {
    pub off: &'static str,
    pub on: &'static str,
}

pub struct Response {
    clicked: bool,
    double_clicked: bool,
    right_clicked: bool,
    hovered: bool,
    has_focus: bool,
}

impl Response {
    pub fn clicked(&self) -> bool {
        self.clicked
    }
    pub fn double_clicked(&self) -> bool {
        self.double_clicked
    }
    pub fn right_clicked(&self) -> bool {
        self.right_clicked
    }
    pub fn hovered(&self) -> bool {
        self.hovered
    }
    pub fn has_focus(&self) -> bool {
        self.has_focus
    }
}

pub const RESIZE_HANDLE_TEX: &str = ragnarok_resources::ui::BTN_RESIZE;

const DRAG_STATE_ID: WidgetId = WidgetId(u32::MAX);
pub const Z_ORDER_STATE_ID: WidgetId = WidgetId(u32::MAX - 1);
const WINDOW_RECTS_STATE_ID: WidgetId = WidgetId(u32::MAX - 2);
const FOCUS_STATE_ID: WidgetId = WidgetId(u32::MAX - 3);
const POPUP_BLOCKER_STATE_ID: WidgetId = WidgetId(u32::MAX - 4);
const WINDOW_DRAG_STATE_ID: WidgetId = WidgetId(u32::MAX - 5);
const DRAG_THRESHOLD: f32 = 5.0;

#[derive(Default, Clone, Copy)]
struct WindowDragOwner(Option<WidgetId>);

/// The dragged window is tracked here as well as in its own `WindowState` so the
/// drag can be released without the window being built: a window closed mid-drag
/// would otherwise stay armed and jump to the cursor when reopened.
fn release_window_drag_when_mouse_is_up(state: &mut StateCache, mouse_down: bool) {
    if mouse_down {
        return;
    }
    let owner = state.get_or_default::<WindowDragOwner>(WINDOW_DRAG_STATE_ID);
    if let Some(id) = owner.0.take() {
        state.get_or_default::<WindowState>(id).dragging = false;
    }
}

#[derive(Default, Clone, Copy)]
struct FocusState(Option<WidgetId>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragCancelledInfo {
    pub source_id: WidgetId,
    pub item_index: usize,
}

#[derive(Clone)]
pub struct DragState {
    pending: bool,
    pub active: bool,
    start_x: f32,
    start_y: f32,
    pub source_id: WidgetId,
    pub item_index: usize,
    pub icon_texture: Option<String>,
    pub icon_size: (f32, f32),
}

/// Whether an item drag is in flight, readable outside a `UiFrame` (the frame is
/// already dropped by the time the world decides which cursor to show).
pub fn drag_active(state: &StateCache) -> bool {
    state
        .get::<DragState>(DRAG_STATE_ID)
        .is_some_and(|drag| drag.active)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WindowOrder {
    Middle,
    Foreground,
    Tooltip,
}

#[derive(Default, Clone)]
pub struct ZOrder {
    pub order: Vec<(WidgetId, WindowOrder)>,
    pending_front: Option<WidgetId>,
}

impl Default for WindowOrder {
    fn default() -> Self {
        WindowOrder::Middle
    }
}

impl ZOrder {
    pub fn is_topmost(&self, id: WidgetId) -> bool {
        self.order.last().map(|(wid, _)| *wid) == Some(id)
    }

    fn sorted_ids(&self) -> Vec<WidgetId> {
        let mut middle = Vec::new();
        let mut foreground = Vec::new();
        let mut tooltip = Vec::new();
        for &(id, order) in &self.order {
            match order {
                WindowOrder::Middle => middle.push(id),
                WindowOrder::Foreground => foreground.push(id),
                WindowOrder::Tooltip => tooltip.push(id),
            }
        }
        middle.extend(foreground);
        middle.extend(tooltip);
        middle
    }
}

#[derive(Default)]
struct WindowRects {
    prev_rects: HashMap<WidgetId, Rect>,
    current_rects: HashMap<WidgetId, Rect>,
    non_interactable: HashSet<WidgetId>,
    prev_non_interactable: HashSet<WidgetId>,
}

#[derive(Default)]
struct PopupBlockers {
    prev: Vec<Rect>,
    current: Vec<Rect>,
}

impl Default for DragState {
    fn default() -> Self {
        Self {
            pending: false,
            active: false,
            start_x: 0.0,
            start_y: 0.0,
            source_id: WidgetId(0),
            item_index: 0,
            icon_texture: None,
            icon_size: (24.0, 24.0),
        }
    }
}

#[derive(Default)]
struct ResizeDragState {
    dragging: bool,
    start_mouse_x: f32,
    start_mouse_y: f32,
}

pub struct DragResponse {
    pub delta_x: f32,
    pub delta_y: f32,
    pub started: bool,
    pub dragging: bool,
    pub hovered: bool,
}

#[derive(Default)]
struct SliderState {
    was_dragging: bool,
}

pub struct SliderResponse {
    pub changed: bool,
    pub released: bool,
}

impl<'a> UiFrame<'a> {
    pub fn new(
        ctx: &'a UiContext,
        atlas: &'a FontAtlas,
        state: &'a mut StateCache,
        elapsed_secs: f32,
        has_grf_textures: bool,
        initial_focus: Option<WidgetId>,
        saved_positions: &'a HashMap<u32, [f32; 2]>,
    ) -> Self {
        let focus =
            initial_focus.or_else(|| state.get::<FocusState>(FOCUS_STATE_ID).and_then(|f| f.0));
        release_window_drag_when_mouse_is_up(state, ctx.mouse_down);
        Self {
            ctx,
            atlas,
            state,
            elapsed_secs,
            has_grf_textures,
            draw_calls: Vec::new(),
            tooltip_draw_calls: Vec::new(),
            any_hovered: false,
            any_interactive_hovered: false,
            focus,
            saved_positions,
            drag_started_this_frame: None,
            current_window: None,
            hovered_window: None,
            z_order_snapshot: Vec::new(),
            modal_layers: Vec::new(),
            in_popup_layer: false,
            keyboard_blocked: false,
            escape_consumed: false,
        }
    }

    /// Suppresses keyboard-driven window actions this frame (e.g. an open modal
    /// dialog owns Enter/Escape). Windows must query [`UiFrame::enter_pressed`]
    /// / [`UiFrame::escape_pressed`] rather than reading `ctx` directly.
    pub fn block_keyboard(&mut self) {
        self.keyboard_blocked = true;
    }

    pub fn enter_pressed(&self) -> bool {
        self.ctx.key_enter && !self.keyboard_blocked
    }

    pub fn escape_pressed(&self) -> bool {
        self.ctx.key_escape && !self.keyboard_blocked && !self.escape_consumed
    }

    /// Claims the Escape key for this frame. Escape has exactly one consumer:
    /// the first caller gets `true`, every later caller sees nothing.
    pub fn take_escape(&mut self) -> bool {
        if !self.escape_pressed() {
            return false;
        }
        self.escape_consumed = true;
        true
    }

    pub fn get_z_order(&mut self) -> Vec<WidgetId> {
        let z = self.state.get_or_default::<ZOrder>(Z_ORDER_STATE_ID);
        if let Some(front_id) = z.pending_front.take() {
            if let Some(pos) = z.order.iter().position(|&(id, _)| id == front_id) {
                let entry = z.order.remove(pos);
                let insert_pos = z
                    .order
                    .iter()
                    .rposition(|&(_, o)| o == entry.1)
                    .map(|p| p + 1)
                    .unwrap_or(z.order.len());
                z.order.insert(insert_pos, entry);
            }
        }
        z.sorted_ids()
    }

    pub fn bring_to_front(&mut self, id: WidgetId) {
        let z = self.state.get_or_default::<ZOrder>(Z_ORDER_STATE_ID);
        if !z.order.iter().any(|&(wid, _)| wid == id) {
            z.order.push((id, WindowOrder::Middle));
        }
        z.pending_front = Some(id);
    }

    pub fn ensure_in_z_order(&mut self, id: WidgetId) {
        self.ensure_in_z_order_with(id, WindowOrder::Middle);
    }

    pub fn ensure_in_z_order_with(&mut self, id: WidgetId, order: WindowOrder) {
        let z = self.state.get_or_default::<ZOrder>(Z_ORDER_STATE_ID);
        if !z.order.iter().any(|&(wid, _)| wid == id) {
            z.order.push((id, order));
        }
    }

    pub fn compute_hovered_window(&mut self, z_order: &[WidgetId]) {
        self.z_order_snapshot = z_order.to_vec();

        let pb = self
            .state
            .get_or_default::<PopupBlockers>(POPUP_BLOCKER_STATE_ID);
        pb.prev = std::mem::take(&mut pb.current);

        let wr = self
            .state
            .get_or_default::<WindowRects>(WINDOW_RECTS_STATE_ID);
        wr.prev_rects = std::mem::take(&mut wr.current_rects);
        wr.prev_non_interactable = std::mem::take(&mut wr.non_interactable);

        self.hovered_window = None;
        for &id in z_order.iter().rev() {
            if wr.prev_non_interactable.contains(&id) {
                continue;
            }
            if let Some(rect) = wr.prev_rects.get(&id) {
                if rect.contains(self.ctx.mouse_x, self.ctx.mouse_y) {
                    self.hovered_window = Some(id);
                    break;
                }
            }
        }
    }

    pub fn enter_window(&mut self, id: WidgetId, rect: Rect) {
        self.current_window = Some(id);
        let wr = self
            .state
            .get_or_default::<WindowRects>(WINDOW_RECTS_STATE_ID);
        wr.current_rects.insert(id, rect);
    }

    pub fn enter_window_passthrough(&mut self, id: WidgetId, rect: Rect) {
        self.current_window = Some(id);
        let wr = self
            .state
            .get_or_default::<WindowRects>(WINDOW_RECTS_STATE_ID);
        wr.current_rects.insert(id, rect);
        wr.non_interactable.insert(id);
    }

    pub fn set_modal(&mut self, ids: &[WidgetId]) {
        self.modal_layers = ids.to_vec();
    }

    fn is_window_occluded(&self, id: WidgetId) -> bool {
        if !self.modal_layers.is_empty() && !self.modal_layers.contains(&id) {
            return true;
        }
        // Only apply z-order occlusion to windows present in the snapshot.
        // Windows built after compute_hovered_window (e.g. always-on-top npc_shop)
        // are not in the snapshot and must not be occluded by z-order.
        match self.hovered_window {
            Some(hw) if hw != id => self.z_order_snapshot.contains(&id),
            _ => false,
        }
    }

    pub fn is_current_window_occluded(&self) -> bool {
        match self.current_window {
            Some(cw) => self.is_window_occluded(cw),
            None => false,
        }
    }

    /// The top-most window under the pointer, from the previous frame's rects.
    /// `None` means the pointer is over the game world, not any window.
    pub fn hovered_window(&self) -> Option<WidgetId> {
        self.hovered_window
    }

    /// Begin a top-most popup layer (context menu, dropdown list). Widgets drawn
    /// while a layer is active are exempt from window occlusion and from being
    /// blocked by any popup; every other widget under `rect` is blocked from the
    /// next frame on (occlusion works off the previous frame's snapshot).
    pub fn begin_popup_layer(&mut self, rect: Rect) {
        self.in_popup_layer = true;
        self.state
            .get_or_default::<PopupBlockers>(POPUP_BLOCKER_STATE_ID)
            .current
            .push(rect);
    }

    pub fn end_popup_layer(&mut self) {
        self.in_popup_layer = false;
    }

    fn pointer_blocked_by_popup(&self) -> bool {
        if self.in_popup_layer {
            return false;
        }
        match self.state.get::<PopupBlockers>(POPUP_BLOCKER_STATE_ID) {
            Some(pb) => pb
                .prev
                .iter()
                .any(|r| r.contains(self.ctx.mouse_x, self.ctx.mouse_y)),
            None => false,
        }
    }

    pub fn window(&mut self, id: WidgetId, w: f32, h: f32, title_bar_h: f32) -> Rect {
        self.window_at(
            id,
            w,
            h,
            title_bar_h,
            ((self.ctx.screen_width - w) / 2.0).floor(),
            ((self.ctx.screen_height - h) / 2.0).floor(),
        )
    }

    pub fn window_at(
        &mut self,
        id: WidgetId,
        w: f32,
        h: f32,
        title_bar_h: f32,
        default_x: f32,
        default_y: f32,
    ) -> Rect {
        let state = self.state.get_or_default::<WindowState>(id);
        if !state.initialized {
            if let Some(pos) = self.saved_positions.get(&id.0) {
                state.x = pos[0];
                state.y = pos[1];
            } else {
                state.x = default_x;
                state.y = default_y;
            }
            state.initialized = true;
        }

        let title_bar = Rect::new(state.x, state.y, w, title_bar_h);
        let wants_drag =
            self.ctx.mouse_clicked && title_bar.contains(self.ctx.mouse_x, self.ctx.mouse_y);
        let drag_offset = if wants_drag {
            Some((self.ctx.mouse_x - state.x, self.ctx.mouse_y - state.y))
        } else {
            None
        };

        if state.dragging {
            if self.ctx.mouse_down {
                state.x = self.ctx.mouse_x - state.drag_offset_x;
                state.y = self.ctx.mouse_y - state.drag_offset_y;
            } else {
                state.dragging = false;
            }
        }

        state.x = state.x.clamp(0.0, (self.ctx.screen_width - w).max(0.0));
        state.y = state.y.clamp(0.0, (self.ctx.screen_height - h).max(0.0));
        let rect = Rect::new(state.x, state.y, w, h);

        self.ensure_in_z_order(id);
        self.enter_window(id, rect);

        let is_occluded = self.is_window_occluded(id);
        if !is_occluded
            && self.ctx.mouse_clicked
            && rect.contains(self.ctx.mouse_x, self.ctx.mouse_y)
        {
            self.bring_to_front(id);
        }

        if !is_occluded {
            if let Some((ox, oy)) = drag_offset {
                if let Some(prev_id) = self.drag_started_this_frame {
                    self.state.get_or_default::<WindowState>(prev_id).dragging = false;
                }
                let state = self.state.get_or_default::<WindowState>(id);
                state.dragging = true;
                state.drag_offset_x = ox;
                state.drag_offset_y = oy;
                self.state
                    .get_or_default::<WindowDragOwner>(WINDOW_DRAG_STATE_ID)
                    .0 = Some(id);
                self.drag_started_this_frame = Some(id);
            }
        }

        rect
    }

    pub fn window_fixed(&mut self, id: WidgetId, w: f32, h: f32, x: f32, y: f32) -> Rect {
        let rect = Rect::new(x, y, w, h);
        self.ensure_in_z_order(id);
        self.enter_window(id, rect);
        let is_occluded = self.is_window_occluded(id);
        if !is_occluded
            && self.ctx.mouse_clicked
            && rect.contains(self.ctx.mouse_x, self.ctx.mouse_y)
        {
            self.bring_to_front(id);
        }
        rect
    }

    pub fn window_centered(&mut self, id: WidgetId, w: f32, h: f32) -> Rect {
        let x = ((self.ctx.screen_width - w) / 2.0).floor();
        let y = ((self.ctx.screen_height - h) / 2.0).floor();
        self.window_fixed(id, w, h, x, y)
    }

    pub fn window_account(&mut self, id: WidgetId, w: f32, h: f32, title_bar_h: f32) -> Rect {
        if self.ctx.lock_account_windows {
            self.window_centered(id, w, h)
        } else {
            self.window(id, w, h, title_bar_h)
        }
    }

    pub fn cancel_window_drag(&mut self, id: WidgetId) {
        self.state.get_or_default::<WindowState>(id).dragging = false;
        let owner = self
            .state
            .get_or_default::<WindowDragOwner>(WINDOW_DRAG_STATE_ID);
        if owner.0 == Some(id) {
            owner.0 = None;
        }
    }

    /// Place a window at (x, y) the first time it is seen. Once the window has
    /// been positioned (here, from a saved layout, or dragged by the user) this
    /// is a no-op, so it never fights a drag.
    pub fn seed_window_position(&mut self, id: WidgetId, x: f32, y: f32) {
        let state = self.state.get_or_default::<WindowState>(id);
        if !state.initialized {
            state.x = x;
            state.y = y;
            state.initialized = true;
        }
    }

    /// Move a window to (x, y) unconditionally, overriding any dragged or
    /// previously seeded position. Use to re-apply a computed layout (e.g. on
    /// resize) rather than to place a window once.
    pub fn set_window_position(&mut self, id: WidgetId, x: f32, y: f32) {
        let state = self.state.get_or_default::<WindowState>(id);
        state.x = x;
        state.y = y;
        state.initialized = true;
    }

    pub fn interact(&mut self, id: WidgetId, rect: Rect) -> Response {
        let in_rect = rect.contains(self.ctx.mouse_x, self.ctx.mouse_y);
        let hovered = in_rect
            && (self.in_popup_layer || !self.is_current_window_occluded())
            && !self.pointer_blocked_by_popup();
        if hovered {
            self.any_hovered = true;
        }
        let clicked = hovered && self.ctx.mouse_clicked;
        let double_clicked = hovered && self.ctx.mouse_double_clicked;
        let right_clicked = hovered && self.ctx.mouse_right_clicked;
        if clicked {
            self.set_focus(id);
        }
        let has_focus = self.focus == Some(id);
        Response {
            clicked,
            double_clicked,
            right_clicked,
            hovered,
            has_focus,
        }
    }

    pub fn button(
        &mut self,
        id: WidgetId,
        rect: Rect,
        textures: &ButtonTextures,
        fallback_label: &str,
    ) -> Response {
        let response = self.interact(id, rect);
        if response.hovered {
            self.any_interactive_hovered = true;
        }
        let pressed = response.hovered && (self.ctx.mouse_clicked || self.ctx.mouse_down);

        if self.has_grf_textures {
            let tex = if pressed {
                textures.pressed
            } else if response.hovered {
                textures.hover
            } else {
                textures.normal
            };
            let (verts, indices) =
                draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, [1.0, 1.0, 1.0, 1.0]);
            self.draw_calls.push(DrawCall {
                vertices: verts.to_vec(),
                indices: indices.to_vec(),
                texture: TextureRef::Named(tex.to_string()),
            });
        } else {
            crate::theme::fallback_button(self, rect, response.hovered, pressed, fallback_label);
        }

        response
    }

    pub fn checkbox(
        &mut self,
        id: WidgetId,
        rect: Rect,
        checked: &mut bool,
        textures: &CheckboxTextures,
    ) -> Response {
        let response = self.interact(id, rect);
        if response.hovered {
            self.any_interactive_hovered = true;
        }
        if response.clicked {
            *checked = !*checked;
        }
        self.draw_checkbox(rect, *checked, textures);
        response
    }

    /// Draw a checkbox glyph without registering interaction, for read-only
    /// displays reflecting state the user cannot toggle here.
    pub fn checkbox_display(&mut self, rect: Rect, checked: bool, textures: &CheckboxTextures) {
        self.draw_checkbox(rect, checked, textures);
    }

    fn draw_checkbox(&mut self, rect: Rect, checked: bool, textures: &CheckboxTextures) {
        if self.has_grf_textures {
            let tex = if checked { textures.on } else { textures.off };
            let (verts, indices) =
                draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, [1.0, 1.0, 1.0, 1.0]);
            self.draw_calls.push(DrawCall {
                vertices: verts.to_vec(),
                indices: indices.to_vec(),
                texture: TextureRef::Named(tex.to_string()),
            });
        } else {
            let mut fill = |x: f32, y: f32, w: f32, h: f32, color: [f32; 4]| {
                let (verts, indices) = draw::quad_vertices(x, y, w, h, color);
                self.draw_calls.push(DrawCall {
                    vertices: verts.to_vec(),
                    indices: indices.to_vec(),
                    texture: TextureRef::White,
                });
            };
            fill(rect.x, rect.y, rect.w, rect.h, [0.6, 0.6, 0.65, 1.0]);
            fill(
                rect.x + 1.0,
                rect.y + 1.0,
                rect.w - 2.0,
                rect.h - 2.0,
                [1.0, 1.0, 1.0, 1.0],
            );
            if checked {
                fill(
                    rect.x + 2.0,
                    rect.y + 2.0,
                    rect.w - 4.0,
                    rect.h - 4.0,
                    [0.286, 0.4, 0.7, 1.0],
                );
            }
        }
    }

    /// A labeled button drawn procedurally regardless of GRF textures — for
    /// buttons whose label has no dedicated texture (e.g. Apply/Revert/Add).
    pub fn text_button(&mut self, id: WidgetId, rect: Rect, label: &str) -> Response {
        let response = self.interact(id, rect);
        if response.hovered {
            self.any_interactive_hovered = true;
        }
        let pressed = response.hovered && (self.ctx.mouse_clicked || self.ctx.mouse_down);
        crate::theme::fallback_button(self, rect, response.hovered, pressed, label);
        response
    }

    pub fn text_input(
        &mut self,
        id: WidgetId,
        rect: Rect,
        state: &mut TextInput,
        bg: TextInputBg,
    ) -> Response {
        let response = self.interact(id, rect);
        if response.hovered {
            self.any_interactive_hovered = true;
        }

        if response.has_focus {
            state.process_keys(self.ctx);
        }

        if response.clicked {
            let text = state.display_text();
            let padding = 4.0;
            let available_w = rect.w - padding * 2.0;
            let cur_text = &text[..state.display_cursor_offset()];
            let scroll = (self.atlas.measure_text(cur_text) - available_w).max(0.0);
            let click_rel = self.ctx.mouse_x - (rect.x + padding) + scroll;
            let mut acc = 0.0;
            let mut best_pos = 0;
            for (i, ch) in text.chars().enumerate() {
                let advance = self.atlas.glyph(ch).advance;
                if click_rel < acc + advance * 0.5 {
                    break;
                }
                acc += advance;
                best_pos = i + 1;
            }
            state.cursor_pos = best_pos;
        }

        match bg {
            TextInputBg::Texture(tex_name) => {
                let (verts, indices) =
                    draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, [1.0, 1.0, 1.0, 1.0]);
                self.draw_calls.push(DrawCall {
                    vertices: verts.to_vec(),
                    indices: indices.to_vec(),
                    texture: TextureRef::Named(tex_name.to_string()),
                });
            }
            TextInputBg::Default => {
                crate::theme::fallback_text_input(self, rect, response.has_focus);
            }
            TextInputBg::Gray => {
                let bg_color = [0.15, 0.15, 0.2, 0.3];
                let (verts, indices) =
                    draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, bg_color);
                self.draw_calls.push(DrawCall {
                    vertices: verts.to_vec(),
                    indices: indices.to_vec(),
                    texture: TextureRef::White,
                });
            }
            TextInputBg::Transparent => {}
        }

        let text = state.display_text();
        let padding = 4.0;
        let available_w = rect.w - padding * 2.0;
        let text_y = rect.y - 2.0 + self.atlas.line_height;
        let is_multiline = rect.h > 2.0 * self.atlas.line_height;

        let cursor_text = &text[..state.display_cursor_offset()];
        let cursor_px = self.atlas.measure_text(cursor_text);
        let scroll = (cursor_px - available_w).max(0.0);
        let text_x = rect.x + padding - scroll;

        let clip_left = rect.x + padding;
        let clip_right = rect.x + rect.w - padding;

        if !text.is_empty() {
            let text_color = [0.0, 0.0, 0.0, 1.0];
            let (verts, indices) = draw::text_vertices_clipped(
                &text, text_x, text_y, text_color, self.atlas, clip_left, clip_right,
            );
            if !verts.is_empty() {
                self.draw_calls.push(DrawCall {
                    vertices: verts,
                    indices,
                    texture: TextureRef::FontAtlas,
                });
            }
        }

        if response.has_focus && (self.elapsed_secs % 1.0) < 0.5 {
            let cursor_x = (text_x + cursor_px).clamp(clip_left, clip_right);
            let caret_y = if is_multiline {
                text_y - self.atlas.ascent
            } else {
                rect.y + (rect.h - self.atlas.ascent) / 2.0
            };
            let caret_color = [0.0, 0.0, 0.0, 1.0];
            let (v, i) =
                draw::quad_vertices(cursor_x, caret_y, 1.0, self.atlas.ascent, caret_color);
            self.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }

        response
    }

    pub fn text(&mut self, x: f32, y: f32, content: &str, color: [f32; 4]) {
        let (v, i) = draw::text_vertices(content, x, y, color, self.atlas);
        if !v.is_empty() {
            self.draw_calls.push(DrawCall {
                vertices: v,
                indices: i,
                texture: TextureRef::FontAtlas,
            });
        }
    }

    pub fn text_bold(&mut self, x: f32, y: f32, content: &str, color: [f32; 4]) {
        let bold: String = content
            .chars()
            .map(ragnarok_renderer::font_atlas::bold_char)
            .collect();
        self.text(x, y, &bold, color);
    }

    pub fn text_with_shadow(
        &mut self,
        x: f32,
        y: f32,
        content: &str,
        color: [f32; 4],
        shadow: Option<[f32; 4]>,
    ) {
        if let Some(shadow) = shadow {
            self.text(x + 1.0, y, content, shadow);
        }
        self.text(x, y, &content, color);
    }

    pub fn text_centered(&mut self, x: f32, y: f32, width: f32, content: &str, color: [f32; 4]) {
        let tw = self.atlas.measure_text(content);
        let cx = x + (width - tw) * 0.5;
        self.text(cx, y, content, color);
    }

    pub fn text_right(&mut self, right_x: f32, y: f32, content: &str, color: [f32; 4]) {
        let tw = self.atlas.measure_text(content);
        self.text(right_x - tw, y, content, color);
    }

    pub fn colored_text(&mut self, x: f32, y: f32, content: &str, default_color: [f32; 4]) {
        let (v, i) = draw::colored_text_vertices(content, x, y, default_color, self.atlas);
        if !v.is_empty() {
            self.draw_calls.push(DrawCall {
                vertices: v,
                indices: i,
                texture: TextureRef::FontAtlas,
            });
        }
    }

    pub fn tooltip(&mut self, anchor_x: f32, anchor_y: f32, text: &str) {
        let tw = self.atlas.measure_text(text);
        let th = self.atlas.line_height;
        let pad = 4.0;
        let tx = anchor_x + 12.0;
        let ty = anchor_y + 8.0;
        let (v, idx) = draw::quad_vertices(
            tx - pad,
            ty,
            tw + pad * 2.0,
            th + pad * 2.0,
            [0.0, 0.0, 0.0, 0.85],
        );
        self.tooltip_draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: idx.to_vec(),
            texture: TextureRef::White,
        });
        let (v, i) = draw::text_vertices(text, tx, ty + th, [1.0, 1.0, 1.0, 1.0], self.atlas);
        if !v.is_empty() {
            self.tooltip_draw_calls.push(DrawCall {
                vertices: v,
                indices: i,
                texture: TextureRef::FontAtlas,
            });
        }
    }

    pub fn flush_tooltips(&mut self) {
        self.draw_calls.append(&mut self.tooltip_draw_calls);
    }

    pub fn set_focus(&mut self, id: WidgetId) {
        self.focus = Some(id);
        self.state
            .set::<FocusState>(FOCUS_STATE_ID, FocusState(Some(id)));
    }

    pub fn focused(&self) -> Option<WidgetId> {
        self.focus
    }

    pub fn drag_handle(&mut self, id: WidgetId, rect: Rect, enabled: bool) -> DragResponse {
        let hovered = rect.contains(self.ctx.mouse_x, self.ctx.mouse_y)
            && (self.in_popup_layer || !self.is_current_window_occluded())
            && !self.pointer_blocked_by_popup();
        if hovered {
            self.any_hovered = true;
            self.any_interactive_hovered = true;
        }

        let state = self.state.get_or_default::<ResizeDragState>(id);
        let mut started = false;

        if enabled && hovered && self.ctx.mouse_clicked && !state.dragging {
            state.dragging = true;
            state.start_mouse_x = self.ctx.mouse_x;
            state.start_mouse_y = self.ctx.mouse_y;
            started = true;
        }
        if !self.ctx.mouse_down {
            state.dragging = false;
        }

        let (delta_x, delta_y) = if state.dragging {
            (
                self.ctx.mouse_x - state.start_mouse_x,
                self.ctx.mouse_y - state.start_mouse_y,
            )
        } else {
            (0.0, 0.0)
        };
        let dragging = state.dragging;

        DragResponse {
            delta_x,
            delta_y,
            started,
            dragging,
            hovered,
        }
    }

    /// Horizontal slider. Dragging (or clicking) the track sets `*value` in
    /// `[min, max]`. `changed` is true on any frame the value moved; `released`
    /// is true on the frame the drag ended (use it to persist).
    pub fn slider(
        &mut self,
        id: WidgetId,
        rect: Rect,
        value: &mut f32,
        min: f32,
        max: f32,
    ) -> SliderResponse {
        let resp = self.drag_handle(id, rect, true);
        let mut changed = false;
        if resp.dragging && rect.w > 0.0 {
            let t = ((self.ctx.mouse_x - rect.x) / rect.w).clamp(0.0, 1.0);
            let new_val = min + t * (max - min);
            if (new_val - *value).abs() > f32::EPSILON {
                *value = new_val;
                changed = true;
            }
        }
        let released = {
            let st = self.state.get_or_default::<SliderState>(id);
            let released = st.was_dragging && !resp.dragging;
            st.was_dragging = resp.dragging;
            released
        };

        let span = (max - min).abs().max(f32::EPSILON);
        let t = ((*value - min) / span).clamp(0.0, 1.0);
        let track_h = 4.0;
        let track_y = rect.y + (rect.h - track_h) * 0.5;
        let knob_w = 8.0;
        let knob_x = rect.x + t * (rect.w - knob_w);

        let push_rect = |ui: &mut Self, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]| {
            let (v, i) = draw::quad_vertices(x, y, w, h, c);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        };
        push_rect(
            self,
            rect.x,
            track_y,
            rect.w,
            track_h,
            [0.2, 0.2, 0.28, 1.0],
        );
        push_rect(
            self,
            rect.x,
            track_y,
            knob_x - rect.x,
            track_h,
            [0.45, 0.55, 0.8, 1.0],
        );
        let knob_c = if resp.hovered || resp.dragging {
            [0.85, 0.9, 1.0, 1.0]
        } else {
            [0.65, 0.7, 0.85, 1.0]
        };
        push_rect(self, knob_x, rect.y, knob_w, rect.h, knob_c);

        SliderResponse { changed, released }
    }

    pub fn resize_handle(&mut self, id: WidgetId, rect: Rect) -> DragResponse {
        let resp = self.drag_handle(id, rect, true);

        if self.has_grf_textures {
            let (v, i) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, [1.0, 1.0, 1.0, 1.0]);
            self.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(RESIZE_HANDLE_TEX.to_string()),
            });
        } else {
            let c = if resp.hovered {
                [0.7, 0.7, 0.8, 1.0]
            } else {
                [0.4, 0.4, 0.5, 1.0]
            };
            let (v, i) = draw::quad_vertices(
                rect.x + rect.w * 0.5,
                rect.y + rect.h * 0.5,
                rect.w * 0.5,
                rect.h * 0.5,
                c,
            );
            self.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }

        resp
    }

    pub fn drag_source(
        &mut self,
        source_id: WidgetId,
        item_index: usize,
        icon_texture: Option<String>,
        icon_size: (f32, f32),
    ) {
        let drag = self.state.get_or_default::<DragState>(DRAG_STATE_ID);
        drag.pending = true;
        drag.active = false;
        drag.start_x = self.ctx.mouse_x;
        drag.start_y = self.ctx.mouse_y;
        drag.source_id = source_id;
        drag.item_index = item_index;
        drag.icon_texture = icon_texture;
        drag.icon_size = icon_size;
    }

    pub fn is_dragging(&mut self) -> bool {
        self.state.get_or_default::<DragState>(DRAG_STATE_ID).active
    }

    pub fn drag_info(&mut self) -> Option<(WidgetId, usize)> {
        let drag = self.state.get_or_default::<DragState>(DRAG_STATE_ID);
        if drag.active {
            Some((drag.source_id, drag.item_index))
        } else {
            None
        }
    }

    pub fn drop_zone(&mut self, rect: Rect) -> Option<(WidgetId, usize)> {
        let hovered = rect.contains(self.ctx.mouse_x, self.ctx.mouse_y)
            && (self.in_popup_layer || !self.is_current_window_occluded())
            && !self.pointer_blocked_by_popup();
        let drag = self.state.get_or_default::<DragState>(DRAG_STATE_ID);
        if drag.active && !self.ctx.mouse_down && hovered {
            let result = (drag.source_id, drag.item_index);
            drag.active = false;
            drag.pending = false;
            Some(result)
        } else {
            None
        }
    }

    pub fn draw_drag_icon(&mut self) -> Option<DragCancelledInfo> {
        let drag = self.state.get_or_default::<DragState>(DRAG_STATE_ID);
        if !drag.pending && !drag.active {
            return None;
        }
        if !self.ctx.mouse_down {
            let cancelled = if drag.active {
                Some(DragCancelledInfo {
                    source_id: drag.source_id,
                    item_index: drag.item_index,
                })
            } else {
                None
            };
            drag.active = false;
            drag.pending = false;
            return cancelled;
        }
        if drag.pending && !drag.active {
            let dx = self.ctx.mouse_x - drag.start_x;
            let dy = self.ctx.mouse_y - drag.start_y;
            if (dx * dx + dy * dy).sqrt() >= DRAG_THRESHOLD {
                drag.active = true;
                drag.pending = false;
            }
        }
        if drag.active {
            let (w, h) = drag.icon_size;
            let icon = drag.icon_texture.clone();
            let x = self.ctx.mouse_x - w / 2.0;
            let y = self.ctx.mouse_y - h / 2.0;
            if let Some(tex) = icon {
                let (v, i) = draw::quad_vertices(x, y, w, h, [1.0, 1.0, 1.0, 0.8]);
                self.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(tex),
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::UiContext;
    use crate::state::StateCache;
    use ragnarok_renderer::font_atlas::FontAtlas;

    fn make_frame<'a>(
        ctx: &'a UiContext,
        atlas: &'a FontAtlas,
        state: &'a mut StateCache,
        saved_positions: &'a HashMap<u32, [f32; 2]>,
    ) -> UiFrame<'a> {
        UiFrame::new(ctx, atlas, state, 0.0, false, None, saved_positions)
    }

    #[test]
    fn only_the_first_caller_gets_escape() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut state = StateCache::new();
        let positions = HashMap::new();
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);

        assert!(ui.escape_pressed());
        assert!(ui.take_escape());
        assert!(!ui.take_escape());
        assert!(!ui.escape_pressed());
    }

    #[test]
    fn window_centers_on_first_call() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let ctx = UiContext::new(800.0, 600.0);
        let mut state = StateCache::new();
        let positions = HashMap::new();
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);

        let rect = ui.window(WidgetId(999), 200.0, 100.0, 25.0);
        assert_eq!(rect.x, 300.0);
        assert_eq!(rect.y, 250.0);
        assert_eq!(rect.w, 200.0);
        assert_eq!(rect.h, 100.0);
    }

    #[test]
    fn window_account_locked_is_centered_and_ignores_drag() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let positions = HashMap::new();
        let id = WidgetId(999);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.lock_account_windows = true;
        ctx.mouse_x = 350.0;
        ctx.mouse_y = 260.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window_account(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (300.0, 250.0));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.lock_account_windows = true;
        ctx.mouse_x = 400.0;
        ctx.mouse_y = 280.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window_account(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (300.0, 250.0));
    }

    #[test]
    fn window_account_unlocked_is_draggable() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let positions = HashMap::new();
        let id = WidgetId(999);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 350.0;
        ctx.mouse_y = 260.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_account(id, 200.0, 100.0, 25.0);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 400.0;
        ctx.mouse_y = 280.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window_account(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (350.0, 270.0));
    }

    #[test]
    fn window_drag_moves_position() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id = WidgetId(999);
        let positions = HashMap::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (300.0, 250.0));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 350.0;
        ctx.mouse_y = 260.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window(id, 200.0, 100.0, 25.0);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 400.0;
        ctx.mouse_y = 280.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (350.0, 270.0));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 400.0;
        ctx.mouse_y = 280.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (350.0, 270.0));
    }

    #[test]
    fn seed_window_position_places_once_and_survives_drag() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id = WidgetId(999);
        let positions = HashMap::new();

        // Seeded position overrides the window's own default.
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.seed_window_position(id, 40.0, 60.0);
        let rect = ui.window_at(id, 200.0, 100.0, 25.0, 0.0, 0.0);
        assert_eq!((rect.x, rect.y), (40.0, 60.0));

        // Drag it elsewhere.
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 70.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id, 200.0, 100.0, 25.0, 0.0, 0.0);
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 120.0;
        ctx.mouse_y = 140.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window_at(id, 200.0, 100.0, 25.0, 0.0, 0.0);
        assert_eq!((rect.x, rect.y), (110.0, 130.0));

        // A later seed must not yank it back.
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.seed_window_position(id, 40.0, 60.0);
        let rect = ui.window_at(id, 200.0, 100.0, 25.0, 0.0, 0.0);
        assert_eq!((rect.x, rect.y), (110.0, 130.0));
    }

    #[test]
    fn slider_click_sets_value_and_release_flags() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id = WidgetId(42);
        let positions = HashMap::new();
        let rect = Rect::new(100.0, 100.0, 200.0, 20.0);
        let mut value = 0.0f32;

        // Click at 75% across the track (x = 100 + 150 = 250).
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 250.0;
        ctx.mouse_y = 108.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let resp = ui.slider(id, rect, &mut value, 0.0, 1.0);
        assert!(resp.changed);
        assert!(!resp.released);
        assert!((value - 0.75).abs() < 0.01);

        // Release: not down this frame → released flag set once.
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let resp = ui.slider(id, rect, &mut value, 0.0, 1.0);
        assert!(resp.released);
    }

    #[test]
    fn stacked_windows_only_topmost_drags() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_a = WidgetId(1);
        let id_b = WidgetId(2);
        let positions = HashMap::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 100.0, 25.0, 100.0, 100.0);
        ui.window_at(id_b, 200.0, 100.0, 25.0, 100.0, 100.0);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 150.0;
        ctx.mouse_y = 110.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 100.0, 25.0, 100.0, 100.0);
        ui.window_at(id_b, 200.0, 100.0, 25.0, 100.0, 100.0);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 200.0;
        ctx.mouse_y = 150.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect_a = ui.window_at(id_a, 200.0, 100.0, 25.0, 100.0, 100.0);
        let rect_b = ui.window_at(id_b, 200.0, 100.0, 25.0, 100.0, 100.0);

        assert_eq!((rect_a.x, rect_a.y), (100.0, 100.0));
        assert_eq!((rect_b.x, rect_b.y), (150.0, 140.0));
    }

    #[test]
    fn window_restores_saved_position() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let ctx = UiContext::new(800.0, 600.0);
        let mut state = StateCache::new();
        let mut positions = HashMap::new();
        positions.insert(999, [50.0, 75.0]);

        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window(WidgetId(999), 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (50.0, 75.0));
    }

    #[test]
    fn interact_hover_click_and_focus() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_a = WidgetId(50);
        let id_b = WidgetId(51);
        let rect_a = Rect::new(10.0, 10.0, 100.0, 30.0);
        let rect_b = Rect::new(10.0, 50.0, 100.0, 30.0);
        let positions = HashMap::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 25.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let r = ui.interact(id_a, rect_a);
        assert!(r.hovered());
        assert!(!r.clicked());
        assert!(!r.has_focus());

        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let r = ui.interact(id_a, rect_a);
        assert!(r.clicked());
        assert!(r.has_focus());

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 65.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let ra = ui.interact(id_a, rect_a);
        let rb = ui.interact(id_b, rect_b);
        assert!(!ra.hovered());
        assert!(ra.has_focus()); // focus persists across frames via the state cache
        assert!(rb.hovered());
        assert!(!rb.clicked());

        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let ra = ui.interact(id_a, rect_a);
        let rb = ui.interact(id_b, rect_b);
        assert!(ra.has_focus()); // still on id_a until the id_b click below moves it
        assert!(rb.has_focus());
        assert!(rb.clicked());

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 200.0;
        ctx.mouse_y = 200.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let ra = ui.interact(id_a, rect_a);
        let rb = ui.interact(id_b, rect_b);
        assert!(!ra.hovered());
        assert!(!ra.clicked());
        assert!(!rb.hovered());
        assert!(!rb.clicked());
    }

    #[test]
    fn text_input_click_focuses_then_receives_typing_next_frame() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let positions = HashMap::new();
        let id = WidgetId(70);
        let rect = Rect::new(10.0, 10.0, 100.0, 20.0);
        let mut input = TextInput::new(24, false);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 15.0;
        ctx.mouse_clicked = true;
        {
            let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
            let r = ui.text_input(id, rect, &mut input, TextInputBg::Default);
            assert!(r.has_focus());
        }

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.typed_chars = vec!['h', 'i'];
        {
            let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
            ui.text_input(id, rect, &mut input, TextInputBg::Default);
        }
        assert_eq!(input.text, "hi");
    }

    #[test]
    fn window_click_outside_title_bar_does_not_drag() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id = WidgetId(999);
        let positions = HashMap::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window(id, 200.0, 100.0, 25.0);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 350.0;
        ctx.mouse_y = 290.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window(id, 200.0, 100.0, 25.0);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 500.0;
        ctx.mouse_y = 400.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (300.0, 250.0));
    }

    #[test]
    fn a_window_closed_from_its_title_bar_reopens_where_it_was() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let positions = HashMap::new();
        let id = WidgetId(4242);

        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = 760.0;
        ctx.mouse_y = 305.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id, 280.0, 200.0, 17.0, 500.0, 300.0);

        for down in [true, false] {
            let mut ctx = UiContext::new(1024.0, 768.0);
            ctx.mouse_down = down;
            make_frame(&ctx, &atlas, &mut state, &positions);
        }

        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = 120.0;
        ctx.mouse_y = 640.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window_at(id, 280.0, 200.0, 17.0, 500.0, 300.0);
        assert_eq!((rect.x, rect.y), (500.0, 300.0));
    }

    #[test]
    fn any_hovered_tracks_widget_hover() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let rect = Rect::new(10.0, 10.0, 100.0, 30.0);
        let positions = HashMap::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.interact(WidgetId(1), rect);
        assert!(!ui.any_hovered);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 25.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.interact(WidgetId(1), rect);
        assert!(ui.any_hovered);
    }

    #[test]
    fn interactive_hovered_false_for_plain_interact() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let rect = Rect::new(10.0, 10.0, 100.0, 30.0);
        let positions = HashMap::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 25.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.interact(WidgetId(1), rect);
        assert!(ui.any_hovered);
        assert!(!ui.any_interactive_hovered);
    }

    #[test]
    fn button_fallback_draws_procedural_geometry_not_named_texture() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let positions = HashMap::new();
        let textures = ButtonTextures {
            normal: "n",
            hover: "h",
            pressed: "p",
        };
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);

        ui.button(
            WidgetId(1),
            Rect::new(10.0, 10.0, 60.0, 20.0),
            &textures,
            "OK",
        );

        assert!(!ui.draw_calls.is_empty());
        assert!(
            !ui.draw_calls
                .iter()
                .any(|c| matches!(c.texture, TextureRef::Named(_))),
            "fallback button must not reference GRF textures"
        );
        assert!(
            ui.draw_calls
                .iter()
                .any(|c| matches!(c.texture, TextureRef::White)),
            "fallback button must draw a procedural rounded face"
        );
    }

    #[test]
    fn interactive_hovered_true_for_button() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let rect = Rect::new(10.0, 10.0, 100.0, 30.0);
        let textures = ButtonTextures {
            normal: "n",
            hover: "h",
            pressed: "p",
        };
        let positions = HashMap::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 25.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.button(WidgetId(1), rect, &textures, "Test");
        assert!(ui.any_hovered);
        assert!(ui.any_interactive_hovered);
    }

    #[test]
    fn interactive_hovered_true_for_text_input() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let rect = Rect::new(10.0, 10.0, 200.0, 30.0);
        let mut input = TextInput::new(100, false);
        let positions = HashMap::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 25.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.text_input(WidgetId(1), rect, &mut input, TextInputBg::Default);
        assert!(ui.any_hovered);
        assert!(ui.any_interactive_hovered);
    }

    #[test]
    fn drag_and_drop_lifecycle() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let source = WidgetId(10);
        let drop_rect = Rect::new(200.0, 0.0, 200.0, 100.0);
        let positions = HashMap::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.drag_source(source, 3, None, (24.0, 24.0));
        assert!(!ui.is_dragging());
        ui.draw_drag_icon();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 60.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.draw_drag_icon();
        assert!(ui.is_dragging());

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 300.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_down = false;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let result = ui.drop_zone(drop_rect);
        assert_eq!(result, Some((source, 3)));
        assert!(!ui.is_dragging());
    }

    #[test]
    fn drag_cancelled_on_release_outside_drop_zone() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let source = WidgetId(10);
        let positions = HashMap::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.drag_source(source, 0, None, (24.0, 24.0));
        ui.draw_drag_icon();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 60.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.draw_drag_icon();
        assert!(ui.is_dragging());

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let cancelled = ui.draw_drag_icon();
        assert!(!ui.is_dragging());
        assert_eq!(
            cancelled,
            Some(DragCancelledInfo {
                source_id: source,
                item_index: 0
            })
        );
    }

    #[test]
    fn draw_drag_icon_returns_none_when_drop_zone_consumed() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let source = WidgetId(10);
        let drop_rect = Rect::new(200.0, 0.0, 200.0, 100.0);
        let positions = HashMap::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.drag_source(source, 5, None, (24.0, 24.0));
        ui.draw_drag_icon();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 60.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.draw_drag_icon();
        assert!(ui.is_dragging());

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 300.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_down = false;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let drop_result = ui.drop_zone(drop_rect);
        assert_eq!(drop_result, Some((source, 5)));
        let cancelled = ui.draw_drag_icon();
        assert_eq!(cancelled, None);
    }

    #[test]
    fn click_on_window_brings_to_front() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_a = WidgetId(100);
        let id_b = WidgetId(200);
        let positions = HashMap::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);
        let z = ui.get_z_order();
        assert_eq!(z.len(), 2);
        assert_eq!(z[0], id_a);
        assert_eq!(z[1], id_b);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 70.0;
        ctx.mouse_y = 120.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let z = ui.get_z_order();
        assert_eq!(z[0], id_b);
        assert_eq!(z[1], id_a);
    }

    #[test]
    fn clicking_topmost_window_keeps_z_order() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_a = WidgetId(100);
        let id_b = WidgetId(200);
        let positions = HashMap::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 200.0;
        ctx.mouse_y = 150.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let z = ui.get_z_order();
        assert_eq!(z[0], id_a);
        assert_eq!(z[1], id_b);
    }

    #[test]
    fn interact_blocked_by_overlapping_topmost_window() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_a = WidgetId(100);
        let id_b = WidgetId(200);
        let positions = HashMap::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 150.0;
        ctx.mouse_y = 120.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let z = ui.get_z_order();
        ui.compute_hovered_window(&z);

        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        let widget_in_a = WidgetId(101);
        let r = ui.interact(widget_in_a, Rect::new(100.0, 100.0, 100.0, 50.0));
        assert!(
            !r.hovered(),
            "widget in background window should not be hovered"
        );
        assert!(
            !r.clicked(),
            "widget in background window should not be clicked"
        );

        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);
        let widget_in_b = WidgetId(201);
        let r = ui.interact(widget_in_b, Rect::new(100.0, 100.0, 100.0, 50.0));
        assert!(r.hovered(), "widget in topmost window should be hovered");
        assert!(r.clicked(), "widget in topmost window should be clicked");
    }

    #[test]
    fn interact_not_blocked_in_non_overlapping_area() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_a = WidgetId(100);
        let id_b = WidgetId(200);
        let positions = HashMap::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 200.0, 80.0);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 70.0;
        ctx.mouse_y = 120.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let z = ui.get_z_order();
        ui.compute_hovered_window(&z);

        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        let widget_in_a = WidgetId(101);
        let r = ui.interact(widget_in_a, Rect::new(60.0, 100.0, 80.0, 50.0));
        assert!(
            r.hovered(),
            "widget in non-overlapping area should be hovered"
        );
        assert!(
            r.clicked(),
            "widget in non-overlapping area should be clicked"
        );
    }

    #[test]
    fn bring_to_front_blocked_when_occluded() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_a = WidgetId(100);
        let id_b = WidgetId(200);
        let positions = HashMap::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 150.0;
        ctx.mouse_y = 120.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let z = ui.get_z_order();
        ui.compute_hovered_window(&z);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let z = ui.get_z_order();
        assert_eq!(z[0], id_a);
        assert_eq!(z[1], id_b);
    }

    #[test]
    fn foreground_window_wins_over_middle() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_mid = WidgetId(100);
        let id_fg = WidgetId(200);
        let positions = HashMap::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.ensure_in_z_order(id_mid);
        ui.enter_window(id_mid, Rect::new(50.0, 50.0, 200.0, 150.0));
        ui.ensure_in_z_order_with(id_fg, WindowOrder::Foreground);
        ui.enter_window(id_fg, Rect::new(50.0, 50.0, 200.0, 150.0));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 100.0;
        ctx.mouse_y = 100.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let z = ui.get_z_order();
        ui.compute_hovered_window(&z);

        assert_eq!(z[0], id_mid);
        assert_eq!(z[1], id_fg);

        ui.enter_window(id_mid, Rect::new(50.0, 50.0, 200.0, 150.0));
        let r = ui.interact(WidgetId(101), Rect::new(60.0, 60.0, 100.0, 50.0));
        assert!(
            !r.clicked(),
            "widget in Middle window should be blocked by Foreground"
        );

        ui.enter_window(id_fg, Rect::new(50.0, 50.0, 200.0, 150.0));
        let r = ui.interact(WidgetId(201), Rect::new(60.0, 60.0, 100.0, 50.0));
        assert!(
            r.clicked(),
            "widget in Foreground window should be clickable"
        );
    }

    #[test]
    fn popup_layer_blocks_widgets_behind_but_not_its_own() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let positions = HashMap::new();
        let popup = Rect::new(100.0, 100.0, 80.0, 60.0);

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.compute_hovered_window(&[]);
        ui.begin_popup_layer(popup);
        ui.end_popup_layer();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 120.0;
        ctx.mouse_y = 120.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.compute_hovered_window(&[]);

        let behind = ui.interact(WidgetId(1), popup);
        assert!(!behind.clicked(), "widget behind popup must not be clicked");
        assert!(!behind.hovered(), "widget behind popup must not be hovered");

        ui.begin_popup_layer(popup);
        let own = ui.interact(WidgetId(2), popup);
        ui.end_popup_layer();
        assert!(own.clicked(), "popup's own widget must be clickable");
    }

    #[test]
    fn passthrough_window_does_not_block_interaction() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_win = WidgetId(100);
        let id_tooltip = WidgetId(200);
        let positions = HashMap::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.ensure_in_z_order(id_win);
        ui.enter_window(id_win, Rect::new(50.0, 50.0, 200.0, 150.0));
        ui.ensure_in_z_order_with(id_tooltip, WindowOrder::Tooltip);
        ui.enter_window_passthrough(id_tooltip, Rect::new(80.0, 80.0, 100.0, 30.0));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 100.0;
        ctx.mouse_y = 90.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let z = ui.get_z_order();
        ui.compute_hovered_window(&z);

        ui.enter_window(id_win, Rect::new(50.0, 50.0, 200.0, 150.0));
        let r = ui.interact(WidgetId(101), Rect::new(80.0, 80.0, 100.0, 30.0));
        assert!(
            r.hovered(),
            "widget below passthrough window should be hovered"
        );
        assert!(
            r.clicked(),
            "widget below passthrough window should be clickable"
        );
    }

    #[test]
    fn modal_blocks_all_non_modal_windows() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_bg = WidgetId(100);
        let id_modal = WidgetId(200);
        let positions = HashMap::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.ensure_in_z_order(id_bg);
        ui.enter_window(id_bg, Rect::new(50.0, 50.0, 100.0, 100.0));
        ui.ensure_in_z_order(id_modal);
        ui.enter_window(id_modal, Rect::new(300.0, 300.0, 100.0, 100.0));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 80.0;
        ctx.mouse_y = 80.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.set_modal(&[id_modal]);
        let z = ui.get_z_order();
        ui.compute_hovered_window(&z);

        ui.enter_window(id_bg, Rect::new(50.0, 50.0, 100.0, 100.0));
        let r = ui.interact(WidgetId(101), Rect::new(60.0, 60.0, 50.0, 50.0));
        assert!(!r.clicked(), "widget in non-modal window should be blocked");

        ui.enter_window(id_modal, Rect::new(300.0, 300.0, 100.0, 100.0));
        let r = ui.interact(WidgetId(201), Rect::new(310.0, 310.0, 50.0, 50.0));
        assert!(
            !r.hovered(),
            "modal widget not under mouse should not be hovered"
        );
    }

    #[test]
    fn bring_to_front_respects_category_boundary() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_a = WidgetId(100);
        let id_b = WidgetId(200);
        let id_fg = WidgetId(300);
        let positions = HashMap::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.ensure_in_z_order(id_a);
        ui.ensure_in_z_order(id_b);
        ui.ensure_in_z_order_with(id_fg, WindowOrder::Foreground);

        let z = ui.get_z_order();
        assert_eq!(z, vec![id_a, id_b, id_fg]);

        ui.bring_to_front(id_a);
        let z = ui.get_z_order();
        assert_eq!(z, vec![id_b, id_a, id_fg]);
    }
}
