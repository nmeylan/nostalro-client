use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use crate::context::UiContext;
use crate::draw::{self, DrawCall, TextureRef};
use crate::rect::Rect;
use crate::state::StateCache;
use crate::text_input::TextInput;
use ragnarok_renderer::font_atlas::FontAtlas;

#[derive(Clone, Copy)]
pub enum TextInputBg<'a> {
    Default,
    Texture(&'a str),
    /// No background drawn; dark text for use over an externally-drawn light bg
    Transparent,
}

pub struct UiFrame<'a> {
    pub ctx: &'a UiContext,
    pub atlas: &'a FontAtlas,
    pub state: &'a mut StateCache,
    pub elapsed_secs: f32,
    pub has_grf_textures: bool,
    pub draw_calls: Vec<DrawCall>,
    pub any_hovered: bool,
    pub any_interactive_hovered: bool,
    focus: Option<WidgetId>,
    saved_positions: &'a HashMap<u32, [f32; 2]>,
    drag_started_this_frame: Option<WidgetId>,
    current_window: Option<WidgetId>,
    hovered_window: Option<WidgetId>,
    z_order_snapshot: Vec<WidgetId>,
    modal_layers: Vec<WidgetId>,
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

pub struct Response {
    clicked: bool,
    double_clicked: bool,
    right_clicked: bool,
    hovered: bool,
    has_focus: bool,
}

impl Response {
    pub fn clicked(&self) -> bool { self.clicked }
    pub fn double_clicked(&self) -> bool { self.double_clicked }
    pub fn right_clicked(&self) -> bool { self.right_clicked }
    pub fn hovered(&self) -> bool { self.hovered }
    pub fn has_focus(&self) -> bool { self.has_focus }
}

pub const RESIZE_HANDLE_TEX: &str = "data/texture/유저인터페이스/btn_resize.bmp";

const DRAG_STATE_ID: WidgetId = WidgetId(u32::MAX);
pub const Z_ORDER_STATE_ID: WidgetId = WidgetId(u32::MAX - 1);
const WINDOW_RECTS_STATE_ID: WidgetId = WidgetId(u32::MAX - 2);
const DRAG_THRESHOLD: f32 = 5.0;

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

    /// Return ids sorted by category: all Middle first, then Foreground, then Tooltip.
    /// Within each category, the original insertion order is preserved.
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
        Self {
            ctx,
            atlas,
            state,
            elapsed_secs,
            has_grf_textures,
            draw_calls: Vec::new(),
            any_hovered: false,
            any_interactive_hovered: false,
            focus: initial_focus,
            saved_positions,
            drag_started_this_frame: None,
            current_window: None,
            hovered_window: None,
            z_order_snapshot: Vec::new(),
            modal_layers: Vec::new(),
        }
    }

    pub fn get_z_order(&mut self) -> Vec<WidgetId> {
        let z = self.state.get_or_default::<ZOrder>(Z_ORDER_STATE_ID);
        if let Some(front_id) = z.pending_front.take() {
            if let Some(pos) = z.order.iter().position(|&(id, _)| id == front_id) {
                let entry = z.order.remove(pos);
                // Move to end of its category
                let insert_pos = z.order.iter().rposition(|&(_, o)| o == entry.1)
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

    /// Pre-compute which z-orderable window is topmost at the mouse position.
    /// Uses rects stored from the previous frame (standard immediate-mode one-frame lag).
    pub fn compute_hovered_window(&mut self, z_order: &[WidgetId]) {
        self.z_order_snapshot = z_order.to_vec();

        let wr = self.state.get_or_default::<WindowRects>(WINDOW_RECTS_STATE_ID);
        wr.prev_rects = std::mem::take(&mut wr.current_rects);
        wr.prev_non_interactable = std::mem::take(&mut wr.non_interactable);

        self.hovered_window = None;
        // Iterate top-to-bottom; skip non-interactable windows (tooltips)
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

    /// Mark the start of a window's widget building and record its rect for hit-testing.
    pub fn enter_window(&mut self, id: WidgetId, rect: Rect) {
        self.current_window = Some(id);
        let wr = self.state.get_or_default::<WindowRects>(WINDOW_RECTS_STATE_ID);
        wr.current_rects.insert(id, rect);
    }

    /// Mark a window as non-interactable (clicks pass through to windows behind it).
    pub fn enter_window_passthrough(&mut self, id: WidgetId, rect: Rect) {
        self.current_window = Some(id);
        let wr = self.state.get_or_default::<WindowRects>(WINDOW_RECTS_STATE_ID);
        wr.current_rects.insert(id, rect);
        wr.non_interactable.insert(id);
    }

    /// Set modal windows — all windows NOT in this group are occluded regardless of mouse position.
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

    pub fn window(&mut self, id: WidgetId, w: f32, h: f32, title_bar_h: f32) -> Rect {
        self.window_at(id, w, h, title_bar_h,
            ((self.ctx.screen_width - w) / 2.0).floor(),
            ((self.ctx.screen_height - h) / 2.0).floor(),
        )
    }

    pub fn window_at(&mut self, id: WidgetId, w: f32, h: f32, title_bar_h: f32, default_x: f32, default_y: f32) -> Rect {
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
        let wants_drag = self.ctx.mouse_clicked && title_bar.contains(self.ctx.mouse_x, self.ctx.mouse_y);
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
        if !is_occluded && self.ctx.mouse_clicked && rect.contains(self.ctx.mouse_x, self.ctx.mouse_y) {
            self.bring_to_front(id);
        }

        // Start drag after releasing the state borrow above.
        // Cancel any earlier window's drag — last (topmost) window wins.
        if !is_occluded {
            if let Some((ox, oy)) = drag_offset {
                if let Some(prev_id) = self.drag_started_this_frame {
                    self.state.get_or_default::<WindowState>(prev_id).dragging = false;
                }
                let state = self.state.get_or_default::<WindowState>(id);
                state.dragging = true;
                state.drag_offset_x = ox;
                state.drag_offset_y = oy;
                self.drag_started_this_frame = Some(id);
            }
        }

        rect
    }

    pub fn interact(&mut self, id: WidgetId, rect: Rect) -> Response {
        let in_rect = rect.contains(self.ctx.mouse_x, self.ctx.mouse_y);
        let hovered = in_rect && !self.is_current_window_occluded();
        if hovered {
            self.any_hovered = true;
        }
        let clicked = hovered && self.ctx.mouse_clicked;
        let double_clicked = hovered && self.ctx.mouse_double_clicked;
        let right_clicked = hovered && self.ctx.mouse_right_clicked;
        if clicked {
            self.focus = Some(id);
        }
        let has_focus = self.focus == Some(id);
        Response { clicked, double_clicked, right_clicked, hovered, has_focus }
    }

    pub fn button(
        &mut self, id: WidgetId, rect: Rect, textures: &ButtonTextures, fallback_label: &str,
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
            let (verts, indices) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, [1.0, 1.0, 1.0, 1.0]);
            self.draw_calls.push(DrawCall {
                vertices: verts.to_vec(),
                indices: indices.to_vec(),
                texture: TextureRef::Named(tex.to_string()),
            });
        } else {
            let bg_color = if pressed {
                [0.15, 0.15, 0.25, 1.0]
            } else if response.hovered {
                [0.35, 0.35, 0.5, 1.0]
            } else {
                [0.25, 0.25, 0.35, 1.0]
            };
            let (v, i) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, bg_color);
            self.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });

            let b = 1.0;
            let bc = [0.5, 0.5, 0.6, 1.0];
            for (bx, by, bw, bh) in [
                (rect.x, rect.y, rect.w, b),
                (rect.x, rect.y + rect.h - b, rect.w, b),
                (rect.x, rect.y, b, rect.h),
                (rect.x + rect.w - b, rect.y, b, rect.h),
            ] {
                let (v, i) = draw::quad_vertices(bx, by, bw, bh, bc);
                self.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }

            let tw = self.atlas.measure_text(fallback_label);
            let tx = rect.x + (rect.w - tw) / 2.0;
            let ty = rect.y + rect.h - (self.atlas.line_height / 2.0);
            let (v, i) = draw::text_vertices(fallback_label, tx, ty, [1.0, 1.0, 1.0, 1.0], self.atlas);
            if !v.is_empty() {
                self.draw_calls.push(DrawCall { vertices: v, indices: i, texture: TextureRef::FontAtlas });
            }
        }

        response
    }

    pub fn text_input(
        &mut self, id: WidgetId, rect: Rect, state: &mut TextInput, bg: TextInputBg,
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

        // Background
        let dark_text = !matches!(bg, TextInputBg::Default);
        match bg {
            TextInputBg::Texture(tex_name) => {
                let (verts, indices) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, [1.0, 1.0, 1.0, 1.0]);
                self.draw_calls.push(DrawCall {
                    vertices: verts.to_vec(),
                    indices: indices.to_vec(),
                    texture: TextureRef::Named(tex_name.to_string()),
                });
            }
            TextInputBg::Default => {
                let bg_color = if response.has_focus {
                    [0.15, 0.15, 0.2, 1.0]
                } else {
                    [0.1, 0.1, 0.15, 1.0]
                };
                let (verts, indices) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, bg_color);
                self.draw_calls.push(DrawCall {
                    vertices: verts.to_vec(),
                    indices: indices.to_vec(),
                    texture: TextureRef::White,
                });

                let border_color = if response.has_focus {
                    [0.5, 0.5, 0.7, 1.0]
                } else {
                    [0.3, 0.3, 0.4, 1.0]
                };
                let border = 1.0;
                for (bx, by, bw, bh) in [
                    (rect.x, rect.y, rect.w, border),
                    (rect.x, rect.y + rect.h - border, rect.w, border),
                    (rect.x, rect.y, border, rect.h),
                    (rect.x + rect.w - border, rect.y, border, rect.h),
                ] {
                    let (v, i) = draw::quad_vertices(bx, by, bw, bh, border_color);
                    self.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: i.to_vec(),
                        texture: TextureRef::White,
                    });
                }
            }
            TextInputBg::Transparent => {}
        }

        // Text
        let text = state.display_text();
        let padding = 4.0;
        let available_w = rect.w - padding * 2.0;
        let text_y = rect.y - 2.0 + self.atlas.line_height;

        // Compute offset so cursor is always visible within the field
        let cursor_text = &text[..state.display_cursor_offset()];
        let cursor_px = self.atlas.measure_text(cursor_text);
        let scroll = (cursor_px - available_w).max(0.0);
        let text_x = rect.x + padding - scroll;

        let clip_left = rect.x + padding;
        let clip_right = rect.x + rect.w - padding;

        if !text.is_empty() {
            let text_color = if dark_text { [0.0, 0.0, 0.0, 1.0] } else { [1.0, 1.0, 1.0, 1.0] };
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

        // Cursor blink
        if response.has_focus && (self.elapsed_secs % 1.0) < 0.5 {
            let cursor_x = (text_x + cursor_px).clamp(clip_left, clip_right);
            let caret_y = rect.y + (rect.h - self.atlas.ascent) / 2.0;
            let caret_color = if dark_text { [0.0, 0.0, 0.0, 1.0] } else { [1.0, 1.0, 1.0, 1.0] };
            let (v, i) = draw::quad_vertices(cursor_x, caret_y, 1.0, self.atlas.ascent, caret_color);
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
            self.draw_calls.push(DrawCall { vertices: v, indices: i, texture: TextureRef::FontAtlas });
        }
    }

    pub fn colored_text(&mut self, x: f32, y: f32, content: &str, default_color: [f32; 4]) {
        let (v, i) = draw::colored_text_vertices(content, x, y, default_color, self.atlas);
        if !v.is_empty() {
            self.draw_calls.push(DrawCall { vertices: v, indices: i, texture: TextureRef::FontAtlas });
        }
    }

    pub fn set_focus(&mut self, id: WidgetId) {
        self.focus = Some(id);
    }

    pub fn focused(&self) -> Option<WidgetId> {
        self.focus
    }

    /// Tracks drag state on a rect, returns pixel delta from drag start.
    /// No drawing — caller renders their own visual. Pass `enabled=false` to prevent new drags.
    pub fn drag_handle(&mut self, id: WidgetId, rect: Rect, enabled: bool) -> DragResponse {
        let hovered = rect.contains(self.ctx.mouse_x, self.ctx.mouse_y) && !self.is_current_window_occluded();
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
            (self.ctx.mouse_x - state.start_mouse_x, self.ctx.mouse_y - state.start_mouse_y)
        } else {
            (0.0, 0.0)
        };
        let dragging = state.dragging;

        DragResponse { delta_x, delta_y, started, dragging, hovered }
    }

    /// Corner resize handle with GRF texture or fallback visual.
    /// Returns pixel delta from drag start.
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
            let c = if resp.hovered { [0.7, 0.7, 0.8, 1.0] } else { [0.4, 0.4, 0.5, 1.0] };
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

    /// Call on mouse_clicked over a draggable widget to begin tracking a potential drag.
    pub fn drag_source(
        &mut self, source_id: WidgetId, item_index: usize,
        icon_texture: Option<String>, icon_size: (f32, f32),
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

    /// Returns true if a drag is currently active (past threshold).
    pub fn is_dragging(&mut self) -> bool {
        self.state.get_or_default::<DragState>(DRAG_STATE_ID).active
    }

    /// Register a drop zone. Returns (source_id, item_index) when a drag is released over this rect.
    pub fn drop_zone(&mut self, rect: Rect) -> Option<(WidgetId, usize)> {
        let hovered = rect.contains(self.ctx.mouse_x, self.ctx.mouse_y) && !self.is_current_window_occluded();
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

    /// Update drag state and render drag icon. Call at end of frame after all widgets.
    /// Returns `Some(DragCancelledInfo)` when a drag was released outside any drop zone.
    pub fn draw_drag_icon(&mut self) -> Option<DragCancelledInfo> {
        let drag = self.state.get_or_default::<DragState>(DRAG_STATE_ID);
        if !drag.pending && !drag.active {
            return None;
        }
        if !self.ctx.mouse_down {
            let cancelled = if drag.active {
                Some(DragCancelledInfo { source_id: drag.source_id, item_index: drag.item_index })
            } else {
                None
            };
            drag.active = false;
            drag.pending = false;
            return cancelled;
        }
        // Promote pending → active once mouse moves past threshold
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
                    vertices: v.to_vec(), indices: i.to_vec(),
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

    fn make_frame<'a>(ctx: &'a UiContext, atlas: &'a FontAtlas, state: &'a mut StateCache, saved_positions: &'a HashMap<u32, [f32; 2]>) -> UiFrame<'a> {
        UiFrame::new(ctx, atlas, state, 0.0, false, None, saved_positions)
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
    fn window_drag_moves_position() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id = WidgetId(999);
        let positions = HashMap::new();

        // Frame 1: initial centering
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (300.0, 250.0));

        // Frame 2: click inside title bar to start drag
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 350.0;
        ctx.mouse_y = 260.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window(id, 200.0, 100.0, 25.0);

        // Frame 3: move mouse while held
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 400.0;
        ctx.mouse_y = 280.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (350.0, 270.0));

        // Frame 4: release mouse — position stays
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 400.0;
        ctx.mouse_y = 280.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (350.0, 270.0));
    }

    #[test]
    fn stacked_windows_only_topmost_drags() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_a = WidgetId(1);
        let id_b = WidgetId(2);
        let positions = HashMap::new();

        // Frame 1: place two windows at same position
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 100.0, 25.0, 100.0, 100.0);
        ui.window_at(id_b, 200.0, 100.0, 25.0, 100.0, 100.0);

        // Frame 2: click in shared title bar area
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 150.0;
        ctx.mouse_y = 110.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 100.0, 25.0, 100.0, 100.0);
        ui.window_at(id_b, 200.0, 100.0, 25.0, 100.0, 100.0);

        // Frame 3: move mouse — only window B (last processed, topmost) should move
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

        // Hover over A without clicking
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 25.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let r = ui.interact(id_a, rect_a);
        assert!(r.hovered());
        assert!(!r.clicked());
        assert!(!r.has_focus());

        // Click on A — should be clicked + focused
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let r = ui.interact(id_a, rect_a);
        assert!(r.clicked());
        assert!(r.has_focus());

        // Next frame: no click, mouse on B — A retains focus, B is hovered
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 65.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let ra = ui.interact(id_a, rect_a);
        let rb = ui.interact(id_b, rect_b);
        assert!(!ra.hovered());
        assert!(!ra.has_focus()); // focus not carried across frames (no initial_focus)
        assert!(rb.hovered());
        assert!(!rb.clicked());

        // Click on B — focus moves to B
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let ra = ui.interact(id_a, rect_a);
        let rb = ui.interact(id_b, rect_b);
        assert!(!ra.has_focus());
        assert!(rb.has_focus());
        assert!(rb.clicked());

        // Click outside both rects — no response
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
    fn window_click_outside_title_bar_does_not_drag() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id = WidgetId(999);
        let positions = HashMap::new();

        // Frame 1: initial centering
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window(id, 200.0, 100.0, 25.0);

        // Frame 2: click inside window body but below title bar (y=250+25=275, click at 290)
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 350.0;
        ctx.mouse_y = 290.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window(id, 200.0, 100.0, 25.0);

        // Frame 3: move mouse — position should not change
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 500.0;
        ctx.mouse_y = 400.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let rect = ui.window(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (300.0, 250.0));
    }

    #[test]
    fn any_hovered_tracks_widget_hover() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let rect = Rect::new(10.0, 10.0, 100.0, 30.0);
        let positions = HashMap::new();

        // Mouse outside — any_hovered stays false
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.interact(WidgetId(1), rect);
        assert!(!ui.any_hovered);

        // Mouse inside — any_hovered becomes true
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
    fn interactive_hovered_true_for_button() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let rect = Rect::new(10.0, 10.0, 100.0, 30.0);
        let textures = ButtonTextures { normal: "n", hover: "h", pressed: "p" };
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

        // Frame 1: click on source item at (50, 50) — starts pending drag
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.drag_source(source, 3, None, (24.0, 24.0));
        assert!(!ui.is_dragging());
        ui.draw_drag_icon();

        // Frame 2: mouse moves past threshold while held — drag becomes active
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 60.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.draw_drag_icon();
        assert!(ui.is_dragging());

        // Frame 3: mouse released over drop zone — drop_zone returns payload
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

        // Start drag
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.drag_source(source, 0, None, (24.0, 24.0));
        ui.draw_drag_icon();

        // Move past threshold
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 60.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.draw_drag_icon();
        assert!(ui.is_dragging());

        // Release mouse — draw_drag_icon returns cancel info
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let cancelled = ui.draw_drag_icon();
        assert!(!ui.is_dragging());
        assert_eq!(cancelled, Some(DragCancelledInfo { source_id: source, item_index: 0 }));
    }

    #[test]
    fn draw_drag_icon_returns_none_when_drop_zone_consumed() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let source = WidgetId(10);
        let drop_rect = Rect::new(200.0, 0.0, 200.0, 100.0);
        let positions = HashMap::new();

        // Start drag
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.drag_source(source, 5, None, (24.0, 24.0));
        ui.draw_drag_icon();

        // Move past threshold
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 60.0;
        ctx.mouse_y = 50.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.draw_drag_icon();
        assert!(ui.is_dragging());

        // Release over drop zone — drop_zone consumes, draw_drag_icon returns None
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

        // Frame 1: place two overlapping windows, A then B (B on top)
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);
        // Both should now be in z-order: [A, B]
        let z = ui.get_z_order();
        assert_eq!(z.len(), 2);
        assert_eq!(z[0], id_a);
        assert_eq!(z[1], id_b);

        // Frame 2: click inside window A's body (in the non-overlapping region)
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 70.0;
        ctx.mouse_y = 120.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);

        // Frame 3: z-order should now be [B, A] (A on top)
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

        // Frame 1: place two windows
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);

        // Frame 2: click on B (already on top)
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 200.0;
        ctx.mouse_y = 150.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);

        // Frame 3: z-order still [A, B]
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

        // Frame 1: place two overlapping windows. A at (50,50), B at (100,80).
        // Overlap region: x 100..250, y 80..200
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);

        // Frame 2: mouse in the overlap region, click.
        // z-order is [A, B] (B on top).
        // Widget in A at the overlap should be blocked.
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 150.0;
        ctx.mouse_y = 120.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let z = ui.get_z_order();
        ui.compute_hovered_window(&z);

        // Build window A first (back), then B (front)
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        let widget_in_a = WidgetId(101);
        let r = ui.interact(widget_in_a, Rect::new(100.0, 100.0, 100.0, 50.0));
        assert!(!r.hovered(), "widget in background window should not be hovered");
        assert!(!r.clicked(), "widget in background window should not be clicked");

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

        // Frame 1: place two windows, B offset so A has a non-overlapping region on the left
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 200.0, 80.0);

        // Frame 2: mouse in A's non-overlapping area (left side)
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
        assert!(r.hovered(), "widget in non-overlapping area should be hovered");
        assert!(r.clicked(), "widget in non-overlapping area should be clicked");
    }

    #[test]
    fn bring_to_front_blocked_when_occluded() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_a = WidgetId(100);
        let id_b = WidgetId(200);
        let positions = HashMap::new();

        // Frame 1: place two overlapping windows
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);
        // z-order: [A, B]

        // Frame 2: click in the overlap region
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 150.0;
        ctx.mouse_y = 120.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let z = ui.get_z_order();
        ui.compute_hovered_window(&z);
        ui.window_at(id_a, 200.0, 150.0, 25.0, 50.0, 50.0);
        ui.window_at(id_b, 200.0, 150.0, 25.0, 100.0, 80.0);

        // Frame 3: z-order should still be [A, B] — A should NOT have been brought to front
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

        // Frame 1: Middle window first, then Foreground at same position
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.ensure_in_z_order(id_mid);
        ui.enter_window(id_mid, Rect::new(50.0, 50.0, 200.0, 150.0));
        ui.ensure_in_z_order_with(id_fg, WindowOrder::Foreground);
        ui.enter_window(id_fg, Rect::new(50.0, 50.0, 200.0, 150.0));

        // Frame 2: mouse in overlap, Foreground should win even though Middle was added first
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 100.0;
        ctx.mouse_y = 100.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let z = ui.get_z_order();
        ui.compute_hovered_window(&z);

        // sorted order: [id_mid, id_fg] (Middle first, Foreground last)
        assert_eq!(z[0], id_mid);
        assert_eq!(z[1], id_fg);

        ui.enter_window(id_mid, Rect::new(50.0, 50.0, 200.0, 150.0));
        let r = ui.interact(WidgetId(101), Rect::new(60.0, 60.0, 100.0, 50.0));
        assert!(!r.clicked(), "widget in Middle window should be blocked by Foreground");

        ui.enter_window(id_fg, Rect::new(50.0, 50.0, 200.0, 150.0));
        let r = ui.interact(WidgetId(201), Rect::new(60.0, 60.0, 100.0, 50.0));
        assert!(r.clicked(), "widget in Foreground window should be clickable");
    }

    #[test]
    fn passthrough_window_does_not_block_interaction() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_win = WidgetId(100);
        let id_tooltip = WidgetId(200);
        let positions = HashMap::new();

        // Frame 1: normal window + passthrough tooltip on top
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.ensure_in_z_order(id_win);
        ui.enter_window(id_win, Rect::new(50.0, 50.0, 200.0, 150.0));
        ui.ensure_in_z_order_with(id_tooltip, WindowOrder::Tooltip);
        ui.enter_window_passthrough(id_tooltip, Rect::new(80.0, 80.0, 100.0, 30.0));

        // Frame 2: click in tooltip area — should pass through to window below
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 100.0;
        ctx.mouse_y = 90.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        let z = ui.get_z_order();
        ui.compute_hovered_window(&z);

        ui.enter_window(id_win, Rect::new(50.0, 50.0, 200.0, 150.0));
        let r = ui.interact(WidgetId(101), Rect::new(80.0, 80.0, 100.0, 30.0));
        assert!(r.hovered(), "widget below passthrough window should be hovered");
        assert!(r.clicked(), "widget below passthrough window should be clickable");
    }

    #[test]
    fn modal_blocks_all_non_modal_windows() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let id_bg = WidgetId(100);
        let id_modal = WidgetId(200);
        let positions = HashMap::new();

        // Frame 1: two non-overlapping windows
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.ensure_in_z_order(id_bg);
        ui.enter_window(id_bg, Rect::new(50.0, 50.0, 100.0, 100.0));
        ui.ensure_in_z_order(id_modal);
        ui.enter_window(id_modal, Rect::new(300.0, 300.0, 100.0, 100.0));

        // Frame 2: set modal on id_modal, click inside id_bg (which is NOT overlapped)
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 80.0;
        ctx.mouse_y = 80.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state, &positions);
        ui.set_modal(&[id_modal]);
        let z = ui.get_z_order();
        ui.compute_hovered_window(&z);

        // hovered_window = id_bg (it's the only one at the mouse), but modal overrides
        ui.enter_window(id_bg, Rect::new(50.0, 50.0, 100.0, 100.0));
        let r = ui.interact(WidgetId(101), Rect::new(60.0, 60.0, 50.0, 50.0));
        assert!(!r.clicked(), "widget in non-modal window should be blocked");

        // Widget in the modal window is still interactable
        ui.enter_window(id_modal, Rect::new(300.0, 300.0, 100.0, 100.0));
        // Mouse is NOT over the modal window, so it shouldn't be hovered
        let r = ui.interact(WidgetId(201), Rect::new(310.0, 310.0, 50.0, 50.0));
        assert!(!r.hovered(), "modal widget not under mouse should not be hovered");
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

        // sorted: [A, B, FG]
        let z = ui.get_z_order();
        assert_eq!(z, vec![id_a, id_b, id_fg]);

        // Bring A to front — should go to end of Middle, not past Foreground
        ui.bring_to_front(id_a);
        let z = ui.get_z_order();
        assert_eq!(z, vec![id_b, id_a, id_fg]);
    }
}
