# UI System

This document covers how the UI system works, how to implement new widgets, and how textures flow from GRF files to the screen.

## Architecture overview

The UI uses an **immediate mode** pattern (similar to egui/Dear ImGui). There is no retained widget tree — every frame, all UI is rebuilt from scratch by calling widget methods on `UiFrame`.

```
                      lib/ui                              lib/renderer
                 ┌─────────────────┐              ┌─────────────────────┐
  winit events → │ UiContext       │              │ Renderer            │
                 │  (input state)  │              │  ├─ UiRenderer      │
                 └────────┬────────┘              │  ├─ TextureCache    │
                          │                       │  ├─ FontAtlas       │
                          ▼                       │  └─ white/font bind │
                 ┌─────────────────┐              │     groups          │
                 │ UiFrame         │              └────────┬────────────┘
                 │  .button()      │─── produces ──→ Vec<UiDrawCall>
                 │  .text_input()  │                       │
                 │  .text()        │                       ▼
                 └─────────────────┘              Renderer::render()
                                                   resolves textures
                                                   batches by bind group
                                                   GPU draw
```

The separation is intentional: `lib/ui` knows nothing about wgpu. It produces `DrawCall` structs with abstract texture references (`TextureRef` enum). The renderer resolves those references to actual GPU bind groups at render time.

## UiContext lifecycle

`UiContext` acts as a **frame-scoped input accumulator**. It has two categories of state:

- **Persistent state** (survives across frames): mouse position, mouse_down, screen dimensions. Never cleared by `begin_frame()`.
- **One-frame pulses** (cleared every frame): mouse_clicked, typed_chars, key flags (backspace, enter, tab, escape, arrows, delete). These fire once and must be consumed before `begin_frame()` clears them.

`UiContext` collects raw winit events into these flags via `handle_event()`. Widgets read them directly — there is no event queue or dispatch system.

## Rendering cycle

Each frame goes through four steps driven by `RedrawRequested`:

```
┌──────────────────────────────────────────────────────────────────┐
│ Between frames: winit delivers events                            │
│   UiContext.handle_event() accumulates input state               │
└──────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────┐
│ Step 1: Build UI (CPU)                                           │
│                                                                  │
│   UiFrame created with &UiContext, &FontAtlas, &mut StateCache,  │
│     elapsed_secs                                                 │
│                                                                  │
│   Compound widgets call ui.button(), ui.text_input(), ui.text()  │
│   Each widget reads input, computes interaction, appends         │
│     DrawCalls, returns a response struct                         │
│                                                                  │
│   Output: ui.draw_calls: Vec<DrawCall>                           │
└──────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────┐
│ Step 2: Render 3D scene (GPU)                                    │
│   RenderPass with LoadOp::Clear (sky color), depth testing on    │
└──────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────┐
│ Step 3: Render UI overlay (GPU)                                  │
│                                                                  │
│   Renderer resolves TextureRef → wgpu::BindGroup                │
│   UiRenderer batches draw calls by texture, draws with           │
│     LoadOp::Load (composites over 3D), no depth, alpha blending  │
└──────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────┐
│ Step 4: Reset pulses                                             │
│   UiContext.begin_frame() clears one-frame flags                 │
│   Persistent state (mouse_down, mouse_x/y, screen size) kept    │
└──────────────────────────────────────────────────────────────────┘
                             │
                             ▼
                    request next redraw
```

### Why this ordering matters

- **Build before render**: widgets produce geometry, the renderer consumes it. There is no retained scene graph.
- **Reset after render, not before build**: if pulses were cleared before building, widgets would never see clicks. The pulse must survive from `handle_event()` through `build()`, then get cleared.
- **`mouse_clicked` vs `mouse_down`**: a button uses `mouse_clicked` (one-frame) for discrete actions ("user clicked"), and `mouse_down` (persistent) for visual feedback ("button looks pressed while held"). That's why `button()` checks `hovered && mouse_clicked` for the response but `hovered && (mouse_clicked || mouse_down)` for the pressed visual state.

## UiFrame — the widget builder

`UiFrame` is created once per frame and borrowed by compound widgets (like `LoginWindow`) to build their UI.

Key fields:
- `draw_calls: Vec<DrawCall>` — widgets append to this
- `state: &mut StateCache` — type-erased cross-frame state keyed by `(WidgetId, TypeId)`, owned by `App`
- `focus: Option<WidgetId>` — tracks which widget has keyboard focus
- `has_grf_textures: bool` — widgets use this to decide between textured or fallback rendering

## Implementing a widget

### The pattern

Every widget is a method on `UiFrame` that:
1. Reads input from `self.ctx`
2. Computes interaction state (hovered, clicked, focused, etc.)
3. Generates vertices and appends `DrawCall`s to `self.draw_calls`
4. Returns a response struct

### Response struct

All widgets return a unified `Response` struct with `clicked()`, `hovered()`, and `has_focus()` getters. Callers check responses immediately after the widget call. Private fields leave room to add fields later without breaking callers.

### `interact()` — the fundamental interaction primitive

`UiFrame::interact(id, rect)` performs hit testing and focus management for any rectangular area. It returns a `Response` and is the building block for all interactive widgets. Use it directly for custom interactive regions (e.g., list rows) instead of duplicating hover/click logic.

### Widget IDs

`WidgetId(u32)` identifies widgets for focus tracking and state cache lookups. IDs must be unique within a screen/window. Define them as constants. All interactive widgets (`button`, `text_input`, `interact`) use IDs for focus management.

### Dual rendering: textured + fallback

Widgets should support both GRF-textured and plain fallback rendering by checking `self.has_grf_textures`. Textured mode draws quads with `TextureRef::Named(path)`, fallback mode draws solid color + border with `TextureRef::White`. This ensures the client is usable without GRF data files.

## Draw calls and geometry

### DrawCall

A `DrawCall` contains vertices (screen-space positions + UV + color), indices (triangle indices local to the call, starting from 0), and a `TextureRef`. The renderer re-indexes when batching.

### TextureRef

- `White` + vertex color = solid colored rectangle (all fallback UI)
- `FontAtlas` + vertex color = tinted text (atlas has white-on-transparent glyphs)
- `Named(path)` = GRF texture, resolved at render time

### Vertex format

`UiVertex` has `position: [f32; 2]` (screen-space pixels, top-left origin), `tex_coord: [f32; 2]`, and `color: [f32; 4]` (multiplied with texture sample). The vertex shader converts pixel coordinates to NDC.

### Geometry helpers (`draw.rs`)

- `quad_vertices` — solid colored rectangle with full-texture UV
- `quad_vertices_uv` — rectangle with custom UV range (for atlas sub-regions)
- `text_vertices` — text string to quads using font atlas
- `text_vertices_clipped` — text with horizontal clipping (adjusts UV proportionally for partial glyphs at field edges)

## Textures

### Loading from GRF

Textures are loaded lazily by `TextureCache` on first access. The cache reads from GRF, decodes with the `image` crate, converts magenta (#FF00FF) to transparent (Ragnarok convention for BMPs), creates a wgpu texture + bind group, and stores in a HashMap.

### Preloading for UI screens

UI screens expose their texture paths via a `grf_texture_paths()` method. At startup, all paths are preloaded via `texture_cache.get_or_load()` so the first frame doesn't stall. If all load successfully, `has_grf_textures` is set to true.

### Texture sizing

GRF textures have arbitrary dimensions. After loading, query sizes from the texture cache via `set_texture_sizes()` so layout adapts to actual asset dimensions. Fallback constants are used when GRF textures aren't available.

### Filtering

- BMP textures → `Nearest` filtering (pixel-perfect for UI sprites)
- Other formats → `Linear` filtering

### Font atlas

The font atlas renders ASCII 32–126 into a single RGBA texture with shelf-packing. It provides `measure_text()` for layout and per-glyph metrics (UV, offset, size, advance).

Font loading priority: GRF font first (`NanumBarunGothicBold.ttf`), embedded `NotoSans-Regular.ttf` fallback.

## Composing compound widgets

Compound widgets (windows, dialogs) are structs holding retained state (`TextInput` fields, focus enum, `has_grf_textures`) with a `build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent>` method. They compute layout, draw backgrounds directly via `draw::quad_vertices`, call child widget methods on `UiFrame`, and return game events based on responses.

### Focus management

For widgets with multiple focusable children: handle tab cycling by updating an internal focus enum and calling `ui.set_focus()`, then after building all widgets sync focus back from `ui.focused()` (click-to-focus may have changed it).

## Rendering pipeline

The UI renders **after** the 3D scene in a separate render pass:

1. **3D pass** — `LoadOp::Clear` with sky color, depth testing enabled
2. **UI pass** — `LoadOp::Load` (preserves 3D), no depth testing, alpha blending

The `UiRenderer` collects all vertices/indices into contiguous GPU buffers, batches by texture bind group, and draws with `draw_indexed()`. Buffers grow dynamically (power-of-2 sizing) and are reused across frames.

## Testing

Widgets can be tested without a GPU by creating a `UiContext` with fake input, a `StateCache`, and a `FontAtlas` from the embedded font. See existing tests in `login_window.rs` and `server_list_window.rs` for the `make_frame` helper pattern.

Test the **behavior**, not the draw calls. Check that clicks produce the right response, that focus cycles correctly, that events are emitted with proper values.

## GRF texture paths

Ragnarok UI textures live under `data/texture/유저인터페이스/` in the GRF archive. Paths use UTF-8 Korean characters (decoded from EUC-KR by `GrfArchive`). Use forward slashes.

## Summary: adding a new UI screen

1. **Define texture paths** as constants, expose via `grf_texture_paths()` method
2. **Define widget IDs** as constants (`WidgetId(N)`)
3. **Create a struct** holding retained state (`TextInput` fields, focus enum, etc.)
4. **Implement `build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent>`** using widget methods
5. **Preload textures** at app startup via `texture_cache.get_or_load()`
6. **Query texture sizes** and store them for layout
7. **Wire into the main loop**: create `UiFrame`, call `build()`, pass draw calls to renderer
8. **Write tests** using `UiContext` + `StateCache` + `UiFrame` without GPU
