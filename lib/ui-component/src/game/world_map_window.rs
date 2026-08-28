use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_sys_button, draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::data_table::map_position_table::WorldMapRect;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const WORLD_MAP_WINDOW_ID: WidgetId = WidgetId(5800);
const CLOSE_BTN_ID: WidgetId = WidgetId(5801);
const MAP_AREA_ID: WidgetId = WidgetId(5802);

pub const WORLD_MAP_TEX: &str = ragnarok_resources::ui::WORLDMAP;
const PLAYER_ARROW_TEX: &str = ragnarok_resources::ui::minimap::MAP_ARROW;
const CLOSE_OFF_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_OFF;
const CLOSE_ON_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_ON;

/// Fallback until the texture is measured; the real image is 1280x1024.
const IMAGE_W: f32 = 1280.0;
const IMAGE_H: f32 = 1024.0;

const TITLE_H: f32 = 20.0;
const SCREEN_MARGIN: f32 = 16.0;
const CLOSE_SIZE: f32 = 11.0;

const PREVIEW_SIZE: f32 = 128.0;
const PREVIEW_LABEL_H: f32 = 14.0;
const PREVIEW_MARGIN: f32 = 6.0;

const ARROW_SIZE: f32 = 12.0;
const PARTY_MARK_SIZE: f32 = 6.0;

const CURRENT_MAP_COLOR: [f32; 4] = [1.0, 0.85, 0.2, 1.0];
const PANEL_BG: [f32; 4] = [0.04, 0.04, 0.06, 0.92];
const PANEL_BORDER: [f32; 4] = [0.55, 0.5, 0.4, 1.0];

pub struct WorldMapWindow {
    pub has_grf_textures: bool,
    open: bool,
    /// Map the local player is on, without extension.
    pub current_map: Option<String>,
    pub map_width: i32,
    pub map_height: i32,
    pub player_position: Option<(f32, f32)>,
    pub player_direction: u8,
    image_size: (f32, f32),
    selected_map: Option<String>,
    /// Texture paths already asked of the client, so a missing file is requested
    /// once rather than every frame.
    requested: Vec<String>,
    /// Of those, the ones the client did load.
    loaded: Vec<String>,
    size: (f32, f32),
}

impl Default for WorldMapWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldMapWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            open: false,
            current_map: None,
            map_width: 1,
            map_height: 1,
            player_position: None,
            player_direction: 0,
            image_size: (IMAGE_W, IMAGE_H),
            selected_map: None,
            requested: Vec::new(),
            loaded: Vec::new(),
            size: (IMAGE_W / 2.0, IMAGE_H / 2.0 + TITLE_H),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.selected_map = None;
    }

    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Client reply to `RequestWorldMapTexture`. `loaded` is false when the GRF
    /// has no such file — most maps have no minimap image.
    pub fn texture_loaded(&mut self, path: &str, loaded: bool) {
        if !self.requested.iter().any(|p| p == path) {
            self.requested.push(path.to_string());
        }
        if loaded && !self.loaded.iter().any(|p| p == path) {
            self.loaded.push(path.to_string());
        }
    }

    fn is_loaded(&self, path: &str) -> bool {
        self.loaded.iter().any(|p| p == path)
    }

    pub fn on_map_changed(&mut self) {
        self.selected_map = None;
    }

    pub fn minimap_texture_path(map: &str) -> String {
        ragnarok_resources::ui::minimap::of(map)
    }

    fn request(&mut self, path: String, events: &mut Vec<GameEvent>) {
        if self.requested.iter().any(|p| *p == path) {
            return;
        }
        self.requested.push(path.clone());
        events.push(GameEvent::RequestWorldMapTexture { path });
    }

    /// Scale from world-map image pixels to on-screen pixels, and the map area.
    fn layout(&self, screen_w: f32, screen_h: f32) -> (f32, f32, f32) {
        let (img_w, img_h) = self.image_size;
        let avail_w = (screen_w - SCREEN_MARGIN * 2.0).max(64.0);
        let avail_h = (screen_h - SCREEN_MARGIN * 2.0 - TITLE_H).max(64.0);
        let scale = (avail_w / img_w).min(avail_h / img_h);
        (scale, (img_w * scale).floor(), (img_h * scale).floor())
    }

    fn rect_on_screen(
        rect: WorldMapRect,
        scale: f32,
        origin_x: f32,
        origin_y: f32,
    ) -> (f32, f32, f32, f32) {
        let x = origin_x + rect.left as f32 * scale;
        let y = origin_y + rect.top as f32 * scale;
        (
            x,
            y,
            (rect.width() as f32 * scale).max(1.0),
            (rect.height() as f32 * scale).max(1.0),
        )
    }

    /// Where a cell position inside `rect`'s map lands on screen. `cell` is in
    /// gat cells, `map` the gat dimensions of that map.
    fn cell_in_rect(
        rect: WorldMapRect,
        cell: (f32, f32),
        map: (i32, i32),
        scale: f32,
        origin_x: f32,
        origin_y: f32,
    ) -> (f32, f32) {
        let (x, y, w, h) = Self::rect_on_screen(rect, scale, origin_x, origin_y);
        let nx = (cell.0 / map.0.max(1) as f32).clamp(0.0, 1.0);
        let ny = 1.0 - (cell.1 / map.1.max(1) as f32).clamp(0.0, 1.0);
        (x + nx * w, y + ny * h)
    }

    /// One stable colour per account id: the original gives each party member a
    /// distinguishable dot, with no table behind it.
    fn party_color(aid: u32) -> [f32; 4] {
        const PALETTE: [[f32; 3]; 6] = [
            [0.30, 0.85, 1.00],
            [1.00, 0.60, 0.20],
            [0.55, 1.00, 0.45],
            [1.00, 0.45, 0.85],
            [0.95, 0.90, 0.35],
            [0.60, 0.60, 1.00],
        ];
        let c = PALETTE[(aid as usize) % PALETTE.len()];
        [c[0], c[1], c[2], 1.0]
    }

    fn fill(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let (v, i) = draw::quad_vertices(x, y, w, h, color);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
    }

    fn outline(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        Self::fill(ui, x, y, w, 1.0, color);
        Self::fill(ui, x, y + h - 1.0, w, 1.0, color);
        Self::fill(ui, x, y, 1.0, h, color);
        Self::fill(ui, x + w - 1.0, y, 1.0, h, color);
    }
}

impl Window for WorldMapWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }

    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(WORLD_MAP_TEX)
            && w > 0
            && h > 0
        {
            self.image_size = (w as f32, h as f32);
        }
    }

    fn window_size(&self) -> (f32, f32) {
        self.size
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            TITLEBAR_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            PLAYER_ARROW_TEX,
        ]
    }
}

impl InGameWindow for WorldMapWindow {
    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.is_open()
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.close();
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        if !self.open {
            return Vec::new();
        }
        let mut events = Vec::new();
        let image_loaded = self.is_loaded(WORLD_MAP_TEX);
        if !image_loaded {
            self.request(WORLD_MAP_TEX.to_string(), &mut events);
        }

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;

        let (scale, map_w, map_h) = self.layout(ui.ctx.screen_width, ui.ctx.screen_height);
        let win_w = map_w;
        let win_h = map_h + TITLE_H;
        self.size = (win_w, win_h);

        let default_x = ((ui.ctx.screen_width - win_w) / 2.0).max(0.0).floor();
        let default_y = ((ui.ctx.screen_height - win_h) / 2.0).max(0.0).floor();
        let win = ui.window_at(
            WORLD_MAP_WINDOW_ID,
            win_w,
            win_h,
            TITLE_H,
            default_x,
            default_y,
        );
        let resp = ui.interact(WORLD_MAP_WINDOW_ID, Rect::new(win.x, win.y, win_w, win_h));
        if resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        let map_x = win.x;
        let map_y = win.y + TITLE_H;

        if image_loaded {
            let (v, i) = draw::quad_vertices_uv(
                map_x,
                map_y,
                map_w,
                map_h,
                [0.0, 0.0],
                [1.0, 1.0],
                [1.0, 1.0, 1.0, 1.0],
            );
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(WORLD_MAP_TEX.to_string()),
            });
        } else {
            Self::fill(ui, map_x, map_y, map_w, map_h, [0.05, 0.06, 0.09, 1.0]);
        }
        Self::outline(ui, map_x, map_y, map_w, map_h, PANEL_BORDER);

        draw_titlebar(ui, win.x, win.y, win_w, TITLE_H, grf);
        let current_name = self
            .current_map
            .as_deref()
            .map(|m| {
                ctx.data
                    .map_name
                    .as_ref()
                    .and_then(|t| t.display_name(m))
                    .unwrap_or(m)
                    .to_string()
            })
            .unwrap_or_default();
        let title = if current_name.is_empty() {
            "World Map".to_string()
        } else {
            format!("World Map - {current_name}")
        };
        ui.text(win.x + 20.0, win.y + TITLE_H - 5.0, &title, text_color(grf));

        let close_rect = Rect::new(
            win.x + win_w - CLOSE_SIZE - 4.0,
            win.y + 4.0,
            CLOSE_SIZE,
            CLOSE_SIZE,
        );
        let close_resp = ui.interact(CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        draw_sys_button(
            ui,
            close_rect,
            (CLOSE_SIZE, CLOSE_SIZE),
            close_resp.hovered(),
            grf,
            CLOSE_ON_TEX,
            CLOSE_OFF_TEX,
            Some('x'),
        );
        if close_resp.clicked() {
            self.close();
            ui.has_grf_textures = prev_grf;
            return events;
        }

        let Some(positions) = ctx.data.map_position.as_ref() else {
            ui.has_grf_textures = prev_grf;
            return events;
        };

        let current_rect = self
            .current_map
            .as_deref()
            .and_then(|map| positions.rect(map));
        if let Some(rect) = current_rect {
            let (x, y, w, h) = Self::rect_on_screen(rect, scale, map_x, map_y);
            Self::outline(ui, x, y, w, h, CURRENT_MAP_COLOR);
            Self::outline(ui, x - 1.0, y - 1.0, w + 2.0, h + 2.0, CURRENT_MAP_COLOR);

            if let Some(pos) = self.player_position {
                let (px, py) = Self::cell_in_rect(
                    rect,
                    pos,
                    (self.map_width, self.map_height),
                    scale,
                    map_x,
                    map_y,
                );
                let angle = crate::game::minimap_window::MinimapWindow::direction_angle(
                    self.player_direction,
                );
                let (v, i) =
                    draw::quad_vertices_rotated(px, py, ARROW_SIZE, angle, [1.0, 1.0, 1.0, 1.0]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(PLAYER_ARROW_TEX.to_string()),
                });
            }
        }

        // Party members, keyed by the map they are on: only same-map members
        // carry live coordinates, everyone else sits at their map's centre.
        let current_key = self.current_map.as_deref().map(ragnarok_game::map_key);
        let mut party_marks: Vec<(f32, f32, String, [f32; 4])> = Vec::new();
        if let Some(party) = ctx.party {
            for member in &party.members {
                if member.aid == ctx.local_aid || !member.online {
                    continue;
                }
                let Some(rect) = positions.rect(&member.map) else {
                    continue;
                };
                let member_key = ragnarok_game::map_key(&member.map);
                let on_our_map = current_key.as_deref() == Some(member_key.as_str());
                let (mx, my) = if on_our_map {
                    Self::cell_in_rect(
                        rect,
                        (member.x as f32, member.y as f32),
                        (self.map_width, self.map_height),
                        scale,
                        map_x,
                        map_y,
                    )
                } else {
                    let (cx, cy) = rect.center();
                    (map_x + cx * scale, map_y + cy * scale)
                };
                party_marks.push((mx, my, member.name.clone(), Self::party_color(member.aid)));
            }
        }
        for (mx, my, _, color) in &party_marks {
            let half = PARTY_MARK_SIZE / 2.0;
            Self::fill(
                ui,
                mx - half,
                my - half,
                PARTY_MARK_SIZE,
                PARTY_MARK_SIZE,
                [1.0, 1.0, 1.0, 1.0],
            );
            Self::fill(
                ui,
                mx - half + 1.0,
                my - half + 1.0,
                PARTY_MARK_SIZE - 2.0,
                PARTY_MARK_SIZE - 2.0,
                *color,
            );
        }

        // Hover and click both work off the rect under the cursor.
        let (mouse_x, mouse_y) = (ui.ctx.mouse_x, ui.ctx.mouse_y);
        let map_resp = ui.interact(MAP_AREA_ID, Rect::new(map_x, map_y, map_w, map_h));
        if map_resp.hovered() {
            let img_x = ((mouse_x - map_x) / scale) as u16;
            let img_y = ((mouse_y - map_y) / scale) as u16;
            if let Some((map, rect)) = positions.at_pixel(img_x, img_y) {
                let (x, y, w, h) = Self::rect_on_screen(rect, scale, map_x, map_y);
                Self::outline(ui, x, y, w, h, [1.0, 1.0, 1.0, 0.8]);

                let display = ctx
                    .data
                    .map_name
                    .as_ref()
                    .and_then(|t| t.display_name(map))
                    .unwrap_or(map);
                let mut tip = display.to_string();
                for (mx, my, name, _) in &party_marks {
                    if *mx >= x && *mx < x + w && *my >= y && *my < y + h {
                        tip.push_str(" / ");
                        tip.push_str(name);
                    }
                }
                ui.tooltip(mouse_x, mouse_y, &tip);

                if map_resp.clicked() {
                    self.selected_map = Some(map.to_string());
                    self.request(Self::minimap_texture_path(map), &mut events);
                }
            }
        }

        if let Some(selected) = self.selected_map.clone() {
            let panel_w = PREVIEW_SIZE + PREVIEW_MARGIN * 2.0;
            let panel_h = PREVIEW_SIZE + PREVIEW_LABEL_H + PREVIEW_MARGIN * 2.0;
            let panel_x = map_x + map_w - panel_w - PREVIEW_MARGIN;
            let panel_y = map_y + map_h - panel_h - PREVIEW_MARGIN;
            Self::fill(ui, panel_x, panel_y, panel_w, panel_h, PANEL_BG);
            Self::outline(ui, panel_x, panel_y, panel_w, panel_h, PANEL_BORDER);

            let img_x = panel_x + PREVIEW_MARGIN;
            let img_y = panel_y + PREVIEW_MARGIN;
            let preview = Self::minimap_texture_path(&selected);
            match self.is_loaded(&preview) {
                true => {
                    let (v, i) = draw::quad_vertices(
                        img_x,
                        img_y,
                        PREVIEW_SIZE,
                        PREVIEW_SIZE,
                        [1.0, 1.0, 1.0, 1.0],
                    );
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: i.to_vec(),
                        texture: TextureRef::Named(preview),
                    });
                }
                false => {
                    Self::fill(
                        ui,
                        img_x,
                        img_y,
                        PREVIEW_SIZE,
                        PREVIEW_SIZE,
                        [0.1, 0.1, 0.12, 1.0],
                    );
                    ui.text_centered(
                        img_x,
                        img_y + PREVIEW_SIZE / 2.0,
                        PREVIEW_SIZE,
                        "no minimap",
                        [0.6, 0.6, 0.6, 1.0],
                    );
                }
            }
            let label = ctx
                .data
                .map_name
                .as_ref()
                .and_then(|t| t.display_name(&selected))
                .unwrap_or(&selected);
            ui.text_centered(
                img_x,
                img_y + PREVIEW_SIZE + PREVIEW_LABEL_H - 2.0,
                PREVIEW_SIZE,
                label,
                [1.0, 1.0, 1.0, 1.0],
            );
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;
    use ragnarok_game::data_table::map_position_table::MapPositionTable;
    use ragnarok_game::party::{Party, PartyMember};

    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_ui::test_support::test_frame;

    fn table() -> MapPositionTable {
        MapPositionTable::parse(
            concat!(
                "0#hugel.rsw#871#0#927#57#\n",
                "8#prontera.rsw#812#587#870#643#\n",
            )
            .as_bytes(),
        )
    }

    fn member(aid: u32, name: &str, map: &str, x: u16, y: u16) -> PartyMember {
        PartyMember {
            aid,
            name: name.to_string(),
            map: map.to_string(),
            leader: false,
            online: true,
            hp: None,
            max_hp: None,
            x,
            y,
            has_live_position: true,
        }
    }

    /// Clicking Prontera's rect selects it and asks the client for its minimap,
    /// and the two party members resolve through the `.gat` suffix the server
    /// sends.
    #[test]
    fn clicking_a_map_selects_it_and_requests_its_minimap() {
        let mut data = DataTable::new();
        data.map_position = Some(table());
        let mut party = Party::new("Adventurers".to_string());
        party.members = vec![
            member(1, "Lidia", "prontera.gat", 200, 200),
            member(2, "Garm", "hugel.gat", 0, 0),
        ];

        let mut win = WorldMapWindow::new();
        win.open();
        win.texture_loaded(WORLD_MAP_TEX, true);
        win.current_map = Some("prontera".to_string());
        win.map_width = 400;
        win.map_height = 400;
        win.player_position = Some((200.0, 200.0));

        let mut character = Character::new();
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(
            IMAGE_W + SCREEN_MARGIN * 2.0,
            IMAGE_H + SCREEN_MARGIN * 2.0 + TITLE_H,
        );
        // Prontera's rect centre, in a 1:1 layout: window origin + image pixel.
        ctx.mouse_x = SCREEN_MARGIN + 841.0;
        ctx.mouse_y = SCREEN_MARGIN + TITLE_H + 615.0;
        ctx.mouse_clicked = true;

        let mut ui = test_frame(&mut ctx, &mut state);
        let mut build_ctx = crate::BuildCtx::test(&mut character, &data);
        build_ctx.party = Some(&party);
        let events = win.build(&mut ui, &mut build_ctx);

        assert_eq!(win.selected_map.as_deref(), Some("prontera"));
        let expected = WorldMapWindow::minimap_texture_path("prontera");
        assert!(
            events.iter().any(|e| matches!(
                e,
                GameEvent::RequestWorldMapTexture { path } if *path == expected
            )),
            "expected a minimap texture request, got {events:?}"
        );
    }

    #[test]
    fn scale_fits_image_inside_the_screen() {
        let win = WorldMapWindow::new();
        let (scale, w, h) = win.layout(683.0, 512.0);
        assert!(w <= 683.0 - 32.0 && h <= 512.0 - 32.0 - TITLE_H);
        assert!((w / h - IMAGE_W / IMAGE_H).abs() < 0.01);
        assert!((scale - (512.0 - 32.0 - TITLE_H) / IMAGE_H).abs() < 0.001);
    }

    #[test]
    fn player_marker_lands_inside_the_current_map_rect() {
        let positions = table();
        let rect = positions.rect("prontera").unwrap();
        let (scale, _, _) = WorldMapWindow::new().layout(1280.0 + 32.0, 1024.0 + 32.0 + TITLE_H);
        assert!((scale - 1.0).abs() < 0.001);

        // Centre of a 400x400 gat maps to the centre of the rect; y is flipped.
        let (x, y) =
            WorldMapWindow::cell_in_rect(rect, (200.0, 200.0), (400, 400), scale, 0.0, 0.0);
        assert!((x - 841.0).abs() < 0.5);
        assert!((y - 615.0).abs() < 0.5);

        let (_, top) =
            WorldMapWindow::cell_in_rect(rect, (200.0, 400.0), (400, 400), scale, 0.0, 0.0);
        assert!((top - rect.top as f32).abs() < 0.5);
    }

    /// A map with no minimap image in the GRF must not be asked for again, and
    /// must not disturb the maps that do have one.
    #[test]
    fn a_missing_texture_is_requested_once() {
        let mut win = WorldMapWindow::new();
        win.texture_loaded(WORLD_MAP_TEX, true);
        assert!(win.is_loaded(WORLD_MAP_TEX));

        let prontera = WorldMapWindow::minimap_texture_path("prontera");
        win.texture_loaded(&prontera, true);
        let missing = WorldMapWindow::minimap_texture_path("prt_in");
        win.texture_loaded(&missing, false);

        assert!(win.is_loaded(&prontera));
        assert!(!win.is_loaded(&missing));

        let mut events = Vec::new();
        win.request(missing, &mut events);
        win.request(prontera, &mut events);
        assert!(events.is_empty(), "already answered, got {events:?}");
    }
}
