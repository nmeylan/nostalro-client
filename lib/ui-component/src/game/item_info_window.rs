use crate::helper::dialog_container::DialogContainer;
use crate::helper::scrollbar::{self, ScrollbarIds};
use crate::helper::window_chrome::{draw_sys_button, draw_titlebar, text_color};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::char_name::CharNameCache;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::display_name::format_equipment_display_name;
use ragnarok_game::event::GameEvent;
use ragnarok_game::item::Item;
use ragnarok_ui::draw::{self, DrawCall, TextureRef, strip_color_codes, word_wrap};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const ITEM_INFO_WINDOW_ID: WidgetId = WidgetId(1000);
const CLOSE_BTN_ID: WidgetId = WidgetId(1001);
const SCROLL_UP_ID: WidgetId = WidgetId(1002);
const SCROLL_DOWN_ID: WidgetId = WidgetId(1003);
const SCROLL_THUMB_ID: WidgetId = WidgetId(1004);
const CARD_SLOT_BASE_ID: u32 = 1005;
const CARD_INFO_WINDOW_ID: WidgetId = WidgetId(1010);
const CARD_INFO_CLOSE_ID: WidgetId = WidgetId(1011);
const CARD_INFO_SCROLL_UP_ID: WidgetId = WidgetId(1012);
const CARD_INFO_SCROLL_DOWN_ID: WidgetId = WidgetId(1013);
const CARD_INFO_SCROLL_THUMB_ID: WidgetId = WidgetId(1014);
const VIEW_BTN_ID: WidgetId = WidgetId(1015);
const CARD_INFO_VIEW_BTN_ID: WidgetId = WidgetId(1016);
const READ_BTN_ID: WidgetId = WidgetId(1017);
const CARD_ILLUST_WINDOW_ID: WidgetId = WidgetId(1020);
const CARD_ILLUST_CLOSE_ID: WidgetId = WidgetId(1021);

const COLLECTION_X: f32 = 10.0;
const COLLECTION_Y: f32 = 11.0;
const COLLECTION_W: f32 = 75.0;
const COLLECTION_H: f32 = 100.0;
const TITLE_X: f32 = 90.0;
const TITLE_Y: f32 = 9.0;
const DESC_X: f32 = 90.0;
const DESC_Y: f32 = 25.0;
const DESC_W: f32 = 170.0;
const TEXT_LINE_H: f32 = 16.0;
const CARD_SECTION_H: f32 = 30.0;
const CARD_ICON_SIZE: f32 = 24.0;
const CLOSE_SIZE: f32 = 11.0;
const FALLBACK_WIN_W: f32 = 280.0;
const FALLBACK_WIN_H: f32 = 120.0;

const TITLE_H_ILLUS: f32 = 17.0;

const COLLECTION_BG_TEX: &str = ragnarok_resources::ui::basic::COLLECTION_BG;
const EMPTY_SLOT_TEX: &str = ragnarok_resources::ui::EMPTY_CARD_SLOT;
const DISABLED_SLOT_TEX: &str = ragnarok_resources::ui::basic::COPARISON_DISABLE_CARD_SLOT;
const CLOSE_OFF_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_OFF;
const CLOSE_ON_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_ON;

const VIEW_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_VIEW,
    hover: ragnarok_resources::ui::BTN_VIEW_B,
    pressed: ragnarok_resources::ui::BTN_VIEW_A,
};
const VIEW_BTN_W: f32 = 42.0;
const VIEW_BTN_H: f32 = 20.0;
const VIEW_SECTION_H: f32 = 28.0;
const ILLUST_TITLEBAR_H: f32 = 17.0;
const ILLUST_FALLBACK_W: f32 = 306.0;
const ILLUST_FALLBACK_H: f32 = 428.0;

const SLOT_EMPTY: u16 = 0xFFFF;

struct ItemInfoData {
    item_id: u16,
    name: String,
    collection_path: Option<String>,
    is_damaged: bool,
    is_equipment: bool,
    is_card: bool,
    is_book: bool,
    description_lines: Vec<String>,
    slot: [u16; 4],
    slot_count: u8,
    card_icon_paths: [Option<String>; 4],
}

struct CardIllustration {
    item_id: u16,
    name: String,
    texture_path: String,
}

pub struct ItemInfoWindow {
    pub has_grf_textures: bool,
    item: Option<ItemInfoData>,
    wrapped_lines: Vec<String>,
    scroll_offset: usize,
    bg_size: (f32, f32),
    card_section_container: DialogContainer,
    card_info: Option<ItemInfoData>,
    card_wrapped_lines: Vec<String>,
    card_scroll_offset: usize,
    card_illustration: Option<CardIllustration>,
}

impl Default for ItemInfoWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemInfoWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            item: None,
            wrapped_lines: Vec::new(),
            scroll_offset: 0,
            bg_size: (FALLBACK_WIN_W, FALLBACK_WIN_H),
            card_section_container: DialogContainer::new(),
            card_info: None,
            card_wrapped_lines: Vec::new(),
            card_scroll_offset: 0,
            card_illustration: None,
        }
    }

    pub fn show(
        &mut self,
        item: &Item,
        data: &DataTable,
        producers: &CharNameCache,
        is_book: bool,
    ) {
        if let Some(current) = &self.item
            && current.item_id == item.item_id
        {
            self.close();
            return;
        }

        let description_lines = data
            .item_description
            .as_ref()
            .and_then(|table| table.get(item.item_id, item.is_identified))
            .map(|lines| lines.to_vec())
            .unwrap_or_default();

        let slot_count = if item.producer_char_id().is_some() {
            0
        } else {
            data.item_slot_count
                .as_ref()
                .map(|table| table.get_slot_count(item.item_id))
                .unwrap_or(0)
        };

        let collection_path = item
            .resource_name
            .as_ref()
            .map(|name| ragnarok_resources::ui::collection::named(name));

        let mut card_icon_paths: [Option<String>; 4] = [None, None, None, None];
        if item.is_equipment() && item.producer_char_id().is_none() {
            for i in 0..4usize {
                let card_id = item.slot[i];
                if card_id != 0 && card_id != SLOT_EMPTY {
                    card_icon_paths[i] = data
                        .item_resource
                        .as_ref()
                        .and_then(|table| table.item_icon_path(card_id));
                }
            }
        }

        self.item = Some(ItemInfoData {
            item_id: item.item_id,
            name: format_equipment_display_name(
                item,
                data.item_slot_count.as_ref(),
                data.card_name.as_ref(),
                producers,
            ),
            collection_path,
            is_damaged: item.is_damaged,
            is_equipment: item.is_equipment(),
            is_card: item.is_card(),
            is_book,
            description_lines,
            slot: item.slot,
            slot_count,
            card_icon_paths,
        });
        self.wrapped_lines.clear();
        self.scroll_offset = 0;
    }

    pub fn pending_texture_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        if let Some(data) = &self.item {
            if let Some(path) = &data.collection_path {
                paths.push(path.clone());
            }
            for path in data.card_icon_paths.iter().flatten() {
                paths.push(path.clone());
            }
        }
        paths
    }

    pub fn show_card(&mut self, card_id: u16, data: &DataTable) {
        if let Some(current) = &self.card_info
            && current.item_id == card_id
        {
            self.close_card();
            return;
        }

        let name = data
            .item_name
            .as_ref()
            .map(|t| t.get_name_or_id(card_id))
            .unwrap_or_else(|| format!("Item #{card_id}"));

        let resource_name = data
            .item_resource
            .as_ref()
            .and_then(|t| t.get_resource_name(card_id).map(|s| s.to_string()));

        let collection_path = resource_name
            .as_ref()
            .map(|n| ragnarok_resources::ui::collection::named(n));

        let description_lines = data
            .item_description
            .as_ref()
            .and_then(|table| table.get(card_id, true))
            .map(|lines| lines.to_vec())
            .unwrap_or_default();

        self.card_info = Some(ItemInfoData {
            item_id: card_id,
            name,
            collection_path,
            is_damaged: false,
            is_equipment: false,
            is_card: true,
            is_book: false,
            description_lines,
            slot: [0; 4],
            slot_count: 0,
            card_icon_paths: [None, None, None, None],
        });
        self.card_wrapped_lines.clear();
        self.card_scroll_offset = 0;
    }

    pub fn pending_card_texture_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        if let Some(data) = &self.card_info
            && let Some(path) = &data.collection_path
        {
            paths.push(path.clone());
        }
        paths
    }

    pub fn close(&mut self) {
        self.item = None;
        self.wrapped_lines.clear();
        self.scroll_offset = 0;
    }

    pub fn close_card(&mut self) {
        self.card_info = None;
        self.card_wrapped_lines.clear();
        self.card_scroll_offset = 0;
    }

    pub fn show_illustration(&mut self, item_id: u16, name: String, texture_path: String) {
        if let Some(current) = &self.card_illustration
            && current.item_id == item_id
        {
            self.close_illustration();
            return;
        }
        self.card_illustration = Some(CardIllustration {
            item_id,
            name,
            texture_path,
        });
    }

    pub fn close_illustration(&mut self) {
        self.card_illustration = None;
    }

    pub fn pending_illustration_texture_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        if let Some(illust) = &self.card_illustration {
            paths.push(illust.texture_path.clone());
        }
        paths
    }

    pub fn is_open(&self) -> bool {
        self.item.is_some()
    }

    fn view_card_illustration_button(
        ui: &mut UiFrame,
        events: &mut Vec<GameEvent>,
        info_window: InfoWindowResult,
        card_data: &ItemInfoData,
    ) {
        let btn_x = info_window.win_x + 6.0;
        let btn_y = info_window.win_y + 6.0;
        let btn_rect = Rect::new(btn_x, btn_y, VIEW_BTN_W, VIEW_BTN_H);
        let btn_resp = ui.button(CARD_INFO_VIEW_BTN_ID, btn_rect, &VIEW_BTN, "View");
        if btn_resp.clicked() {
            events.push(GameEvent::ShowCardIllustration {
                item_id: card_data.item_id,
            });
        }
    }

    fn read_book_button(
        ui: &mut UiFrame,
        events: &mut Vec<GameEvent>,
        x: f32,
        y: f32,
        item_id: u16,
    ) {
        let rect = Rect::new(x, y, VIEW_BTN_W, VIEW_BTN_H);
        if ui.button(READ_BTN_ID, rect, &VIEW_BTN, "Read").clicked() {
            events.push(GameEvent::ReadBook { item_id });
        }
    }
}

impl Window for ItemInfoWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(COLLECTION_BG_TEX) {
            self.bg_size = (w as f32, h as f32);
        }
        self.card_section_container.set_texture_sizes(size_fn);
    }

    fn window_size(&self) -> (f32, f32) {
        let (w, h) = if self.has_grf_textures && self.bg_size.0 > 0.0 {
            self.bg_size
        } else {
            (FALLBACK_WIN_W, FALLBACK_WIN_H)
        };
        let extra = match &self.item {
            Some(item) if item.is_equipment => CARD_SECTION_H,
            Some(item) if item.is_card => VIEW_SECTION_H,
            _ => 0.0,
        };
        (w, h + extra)
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            COLLECTION_BG_TEX,
            EMPTY_SLOT_TEX,
            DISABLED_SLOT_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            VIEW_BTN.normal,
            VIEW_BTN.hover,
            VIEW_BTN.pressed,
        ];
        paths.extend(scrollbar::grf_texture_paths());
        paths.extend(DialogContainer::grf_texture_paths());
        paths
    }
}

impl InGameWindow for ItemInfoWindow {
    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.is_open()
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.close();
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let _character = &mut *ctx.character;
        let data = ctx.data;
        if self.item.is_none() && self.card_info.is_none() && self.card_illustration.is_none() {
            return Vec::new();
        }

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let bg_size = self.bg_size;
        let mut events = Vec::new();

        if self.item.is_some() {
            if self.wrapped_lines.is_empty() {
                let full_text = self.item.as_ref().unwrap().description_lines.join("\n");
                self.wrapped_lines = word_wrap(
                    &full_text,
                    DESC_W,
                    |t| ui.atlas.measure_text(&strip_color_codes(t)),
                    false,
                );
            }

            let item_data = self.item.as_ref().unwrap();
            let extra_h = if item_data.is_equipment {
                CARD_SECTION_H
            } else if item_data.is_card {
                VIEW_SECTION_H
            } else {
                0.0
            };

            let ids = InfoWindowIds {
                window: ITEM_INFO_WINDOW_ID,
                close: CLOSE_BTN_ID,
                scroll_up: SCROLL_UP_ID,
                scroll_down: SCROLL_DOWN_ID,
                scroll_thumb: SCROLL_THUMB_ID,
            };
            let result = build_info_window(
                ui,
                item_data,
                &self.wrapped_lines,
                self.scroll_offset,
                &ids,
                grf,
                bg_size,
                extra_h,
            );
            self.scroll_offset = result.scroll_offset;

            if result.closed {
                self.close();
            } else if self.item.as_ref().unwrap().is_book {
                let (_cw, ch) = if grf {
                    bg_size
                } else {
                    (FALLBACK_WIN_W, FALLBACK_WIN_H)
                };
                let item_id = self.item.as_ref().unwrap().item_id;
                let bx = result.win_x + COLLECTION_X;
                let by = result.win_y + ch - VIEW_BTN_H - 6.0;
                Self::read_book_button(ui, &mut events, bx, by, item_id);
            } else if self.item.as_ref().unwrap().is_card {
                let card_data = self.item.as_ref().unwrap();

                Self::view_card_illustration_button(ui, &mut events, result, card_data);
            } else if self.item.as_ref().unwrap().is_equipment {
                let item_data = self.item.as_ref().unwrap();
                let (container_w, container_h) = if grf {
                    bg_size
                } else {
                    (FALLBACK_WIN_W, FALLBACK_WIN_H)
                };
                let win_w = container_w;

                let section_y = result.win_y + container_h;

                self.card_section_container.has_grf_textures = grf;
                self.card_section_container.draw(
                    &mut ui.draw_calls,
                    result.win_x,
                    section_y,
                    win_w,
                    CARD_SECTION_H,
                    [1.0, 1.0, 1.0, 1.0],
                );

                let icon_y = section_y + (CARD_SECTION_H - CARD_ICON_SIZE) / 2.0;
                let mut icon_x = result.win_x + 8.0;
                for i in 0..4usize {
                    let card_id = item_data.slot[i];
                    let slot_rect = Rect::new(icon_x, icon_y, CARD_ICON_SIZE, CARD_ICON_SIZE);

                    if i < item_data.slot_count as usize {
                        if card_id != 0 && card_id != SLOT_EMPTY {
                            let tex = item_data.card_icon_paths[i]
                                .as_ref()
                                .map(|p| TextureRef::Named(p.clone()))
                                .unwrap_or_else(|| {
                                    if grf {
                                        TextureRef::Named(EMPTY_SLOT_TEX.to_string())
                                    } else {
                                        TextureRef::White
                                    }
                                });
                            let (v, idx) = draw::quad_vertices(
                                icon_x,
                                icon_y,
                                CARD_ICON_SIZE,
                                CARD_ICON_SIZE,
                                [1.0, 1.0, 1.0, 1.0],
                            );
                            ui.draw_calls.push(DrawCall {
                                vertices: v.to_vec(),
                                indices: idx.to_vec(),
                                texture: tex,
                            });

                            let slot_resp =
                                ui.interact(WidgetId(CARD_SLOT_BASE_ID + i as u32), slot_rect);
                            if slot_resp.hovered() {
                                ui.any_interactive_hovered = true;
                                let card_name = data
                                    .item_name
                                    .as_ref()
                                    .map(|t| t.get_name_or_id(card_id))
                                    .unwrap_or_else(|| format!("Item #{card_id}"));
                                ui.tooltip(icon_x, icon_y - CARD_ICON_SIZE, &card_name);
                            }
                            if slot_resp.right_clicked() {
                                events.push(GameEvent::ShowCardInfo { item_id: card_id });
                            }
                        } else if grf {
                            let (v, idx) = draw::quad_vertices(
                                icon_x,
                                icon_y,
                                CARD_ICON_SIZE,
                                CARD_ICON_SIZE,
                                [1.0, 1.0, 1.0, 1.0],
                            );
                            ui.draw_calls.push(DrawCall {
                                vertices: v.to_vec(),
                                indices: idx.to_vec(),
                                texture: TextureRef::Named(EMPTY_SLOT_TEX.to_string()),
                            });
                        }
                    } else if grf {
                        let (v, idx) = draw::quad_vertices(
                            icon_x,
                            icon_y,
                            CARD_ICON_SIZE,
                            CARD_ICON_SIZE,
                            [1.0, 1.0, 1.0, 1.0],
                        );
                        ui.draw_calls.push(DrawCall {
                            vertices: v.to_vec(),
                            indices: idx.to_vec(),
                            texture: TextureRef::Named(DISABLED_SLOT_TEX.to_string()),
                        });
                    }

                    icon_x += CARD_ICON_SIZE + 2.0;
                }
            }
        }

        if self.card_info.is_some() {
            if self.card_wrapped_lines.is_empty() {
                let full_text = self
                    .card_info
                    .as_ref()
                    .unwrap()
                    .description_lines
                    .join("\n");
                self.card_wrapped_lines = word_wrap(
                    &full_text,
                    DESC_W,
                    |t| ui.atlas.measure_text(&strip_color_codes(t)),
                    false,
                );
            }

            let card_data = self.card_info.as_ref().unwrap();
            let ids = InfoWindowIds {
                window: CARD_INFO_WINDOW_ID,
                close: CARD_INFO_CLOSE_ID,
                scroll_up: CARD_INFO_SCROLL_UP_ID,
                scroll_down: CARD_INFO_SCROLL_DOWN_ID,
                scroll_thumb: CARD_INFO_SCROLL_THUMB_ID,
            };
            let result = build_info_window(
                ui,
                card_data,
                &self.card_wrapped_lines,
                self.card_scroll_offset,
                &ids,
                grf,
                bg_size,
                VIEW_SECTION_H,
            );
            self.card_scroll_offset = result.scroll_offset;

            if result.closed {
                self.close_card();
            } else {
                let card_data = self.card_info.as_ref().unwrap();
                Self::view_card_illustration_button(ui, &mut events, result, card_data);
            }
        }

        if let Some(illust) = &self.card_illustration {
            let illust_w = ILLUST_FALLBACK_W;
            let illust_h = ILLUST_FALLBACK_H;
            let total_h = ILLUST_TITLEBAR_H + illust_h;

            let win = ui.window(CARD_ILLUST_WINDOW_ID, illust_w, total_h, ILLUST_TITLEBAR_H);
            let win_rect = Rect::new(win.x, win.y, illust_w, total_h);
            ui.interact(CARD_ILLUST_WINDOW_ID, win_rect);

            draw_titlebar(ui, win.x, win.y, illust_w, TITLE_H_ILLUS, grf);
            let text_color = text_color(grf);
            ui.text(
                win.x + (17.0),
                win.y + (TITLE_H_ILLUS) - (3.0),
                &illust.name,
                text_color,
            );
            if grf {
                let (_v, _i) =
                    draw::quad_vertices(win.x, win.y, illust_w, total_h, [1.0, 1.0, 1.0, 1.0]);
            } else {
                crate::helper::fallback::window_body(
                    ui,
                    win.x,
                    win.y + TITLE_H_ILLUS,
                    illust_w,
                    illust_h,
                );
            }

            let close_rect = Rect::new(
                win.x + illust_w - CLOSE_SIZE - 3.0,
                win.y + 3.0,
                CLOSE_SIZE,
                CLOSE_SIZE,
            );
            let close_resp = ui.interact(CARD_ILLUST_CLOSE_ID, close_rect);
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
                self.close_illustration();
            } else {
                let img_x = win.x;
                let img_y = win.y + ILLUST_TITLEBAR_H;
                let (v, i) =
                    draw::quad_vertices(img_x, img_y, illust_w, illust_h, [1.0, 1.0, 1.0, 1.0]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(illust.texture_path.clone()),
                });
            }
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

struct InfoWindowIds {
    window: WidgetId,
    close: WidgetId,
    scroll_up: WidgetId,
    scroll_down: WidgetId,
    scroll_thumb: WidgetId,
}

struct InfoWindowResult {
    win_x: f32,
    win_y: f32,
    scroll_offset: usize,
    closed: bool,
}

fn build_info_window(
    ui: &mut UiFrame,
    item_data: &ItemInfoData,
    wrapped_lines: &[String],
    scroll_offset: usize,
    ids: &InfoWindowIds,
    grf: bool,
    bg_size: (f32, f32),
    extra_height: f32,
) -> InfoWindowResult {
    let (container_w, container_h) = if grf {
        bg_size
    } else {
        (FALLBACK_WIN_W, FALLBACK_WIN_H)
    };
    let win_w = container_w;
    let win_h = container_h + extra_height;

    let desc_area_h = container_h - DESC_Y - 4.0;
    let visible_lines = (desc_area_h / TEXT_LINE_H).floor() as usize;
    let total_lines = wrapped_lines.len();
    let max_scroll = total_lines.saturating_sub(visible_lines);
    let needs_scrollbar = max_scroll > 0;

    let default_x = (ui.ctx.screen_width - win_w) / 2.0;
    let default_y = (ui.ctx.screen_height - win_h) / 2.0;
    let win = ui.window_at(ids.window, win_w, win_h, container_h, default_x, default_y);

    let win_rect = Rect::new(win.x, win.y, win_w, win_h);
    ui.interact(ids.window, win_rect);

    if grf {
        let (v, i) =
            draw::quad_vertices(win.x, win.y, container_w, container_h, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(COLLECTION_BG_TEX.to_string()),
        });
    } else {
        let (v, i) = draw::quad_vertices(win.x, win.y, win_w, win_h, [0.15, 0.15, 0.20, 0.95]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let bc = [0.5, 0.5, 0.6, 1.0];
        for (bx, by, bw, bh) in [
            (win.x, win.y, win_w, 1.0),
            (win.x, win.y + win_h - 1.0, win_w, 1.0),
            (win.x, win.y, 1.0, win_h),
            (win.x + win_w - 1.0, win.y, 1.0, win_h),
        ] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, bc);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }
    }

    if let Some(path) = &item_data.collection_path {
        let (v, i) = draw::quad_vertices(
            win.x + COLLECTION_X,
            win.y + COLLECTION_Y,
            COLLECTION_W,
            COLLECTION_H,
            [1.0, 1.0, 1.0, 1.0],
        );
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(path.clone()),
        });
    }

    let close_rect = Rect::new(
        win.x + container_w - CLOSE_SIZE - 3.0,
        win.y + 3.0,
        CLOSE_SIZE,
        CLOSE_SIZE,
    );
    let close_resp = ui.interact(ids.close, close_rect);
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
    let closed = close_resp.clicked();
    if closed {
        return InfoWindowResult {
            win_x: win.x,
            win_y: win.y,
            scroll_offset,
            closed: true,
        };
    }

    let name_color = if item_data.is_damaged {
        [1.0, 0.0, 0.0, 1.0]
    } else if grf {
        [0.0, 0.0, 0.0, 1.0]
    } else {
        [1.0, 1.0, 1.0, 1.0]
    };
    ui.text(
        win.x + TITLE_X,
        win.y + TITLE_Y + ui.atlas.line_height,
        &item_data.name,
        name_color,
    );

    let text_color = if grf {
        [0.0, 0.0, 0.0, 1.0]
    } else {
        [0.9, 0.9, 0.9, 1.0]
    };
    let desc_x = win.x + DESC_X;
    let desc_top = win.y + DESC_Y;
    let start = scroll_offset;
    let end = (start + visible_lines).min(total_lines);
    let mut text_y = desc_top + ui.atlas.line_height;
    for line in &wrapped_lines[start..end] {
        ui.colored_text(desc_x, text_y, line, text_color);
        text_y += TEXT_LINE_H;
    }

    let mut new_scroll = scroll_offset;
    if needs_scrollbar {
        let sb_x = win.x + container_w - scrollbar::SCROLLBAR_W - 1.0;
        let sb_y = desc_top;
        let sb_h = desc_area_h;
        let content_rect = Rect::new(desc_x, desc_top, DESC_W, desc_area_h);
        new_scroll = scrollbar::scrollbar(
            ui,
            ScrollbarIds {
                up: ids.scroll_up,
                down: ids.scroll_down,
                thumb: ids.scroll_thumb,
            },
            scroll_offset,
            visible_lines,
            max_scroll,
            content_rect,
            sb_x,
            sb_y,
            sb_h,
        );
    }

    InfoWindowResult {
        win_x: win.x,
        win_y: win.y,
        scroll_offset: new_scroll,
        closed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::enums::EnumWithNumberValue;
    use models::enums::item::ItemType;
    use ragnarok_game::data_table::DataTable;
    use ragnarok_game::data_table::item_resource_table::ItemResourceTable;

    fn make_data_table() -> DataTable {
        DataTable::new()
    }

    fn make_item(item_id: u16, item_type: u8, slot: [u16; 4]) -> Item {
        Item {
            index: 1,
            item_id,
            item_type: ItemType::from_value(item_type as usize),
            count: 1,
            is_identified: true,
            is_damaged: false,
            refining_level: 0,
            slot,
            location: 0,
            wear_state: 0,
            name: format!("TestItem{item_id}"),
            resource_name: Some("test_resource".to_string()),
        }
    }

    #[test]
    fn show_opens_and_close_closes() {
        let mut win = ItemInfoWindow::new();
        let data = make_data_table();
        let item = make_item(501, 0, [0; 4]);
        win.show(&item, &data, &CharNameCache::default(), false);
        assert!(win.is_open());
        win.close();
        assert!(!win.is_open());
    }

    #[test]
    fn show_same_item_toggles_off() {
        let mut win = ItemInfoWindow::new();
        let data = make_data_table();
        let item = make_item(501, 0, [0; 4]);
        win.show(&item, &data, &CharNameCache::default(), false);
        assert!(win.is_open());
        win.show(&item, &data, &CharNameCache::default(), false);
        assert!(!win.is_open());
    }

    #[test]
    fn damaged_item_sets_flag() {
        let mut win = ItemInfoWindow::new();
        let data = make_data_table();
        let mut item = make_item(1201, 4, [0; 4]);
        item.is_damaged = true;
        win.show(&item, &data, &CharNameCache::default(), false);
        assert!(win.item.as_ref().unwrap().is_damaged);
    }

    #[test]
    fn slot_interpretation() {
        let mut win = ItemInfoWindow::new();
        let data = make_data_table();
        let item = make_item(1201, 4, [4001, SLOT_EMPTY, 0, 0]);
        win.show(&item, &data, &CharNameCache::default(), false);
        let info = win.item.as_ref().unwrap();
        assert!(info.is_equipment);
        assert_eq!(info.slot[0], 4001);
        assert_eq!(info.slot[1], SLOT_EMPTY);
        assert_eq!(info.slot[2], 0);
    }

    #[test]
    fn collection_path_built_from_resource_name() {
        let mut win = ItemInfoWindow::new();
        let data = make_data_table();
        let item = make_item(501, 0, [0; 4]);
        win.show(&item, &data, &CharNameCache::default(), false);
        assert_eq!(
            win.item.as_ref().unwrap().collection_path.as_deref(),
            Some("data/texture/유저인터페이스/collection/test_resource.bmp"),
        );
    }

    #[test]
    fn pending_textures_includes_collection_and_cards() {
        let mut win = ItemInfoWindow::new();
        let data = make_data_table();
        let item = make_item(501, 0, [0; 4]);
        win.show(&item, &data, &CharNameCache::default(), false);
        let paths = win.pending_texture_paths();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].contains("collection/test_resource.bmp"));
    }

    #[test]
    fn equipment_always_shows_card_section() {
        let mut win = ItemInfoWindow::new();
        let data = make_data_table();
        let item = make_item(1201, 4, [0; 4]);
        win.show(&item, &data, &CharNameCache::default(), false);
        let info = win.item.as_ref().unwrap();
        assert!(info.is_equipment);
        assert_eq!(info.slot_count, 0);
    }

    #[test]
    fn show_card_opens_and_toggle_closes() {
        let mut win = ItemInfoWindow::new();
        let data = make_data_table();
        win.show_card(4025, &data);
        assert!(win.card_info.is_some());
        assert_eq!(win.card_info.as_ref().unwrap().item_id, 4025);
        assert!(!win.card_info.as_ref().unwrap().is_equipment);
        win.show_card(4025, &data);
        assert!(win.card_info.is_none());
    }

    #[test]
    fn close_item_does_not_close_card() {
        let mut win = ItemInfoWindow::new();
        let data = make_data_table();
        let item = make_item(1201, 4, [4025, 0, 0, 0]);
        win.show(&item, &data, &CharNameCache::default(), false);
        win.show_card(4025, &data);
        assert!(win.is_open());
        assert!(win.card_info.is_some());
        win.close();
        assert!(!win.is_open());
        assert!(win.card_info.is_some());
    }

    #[test]
    fn pending_card_texture_paths_returns_collection() {
        let mut win = ItemInfoWindow::new();
        use std::collections::HashMap;
        let data = DataTable {
            item_resource: Some(ItemResourceTable::from_entries(
                {
                    let mut m = HashMap::new();
                    m.insert(4025u16, "고블린카드".to_string());
                    m
                },
                HashMap::new(),
            )),
            ..DataTable::new()
        };
        win.show_card(4025, &data);
        let paths = win.pending_card_texture_paths();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].contains("collection/고블린카드.bmp"));
    }

    #[test]
    fn card_item_sets_is_card_flag() {
        let mut win = ItemInfoWindow::new();
        let data = make_data_table();
        let card = make_item(4001, 6, [0; 4]);
        win.show(&card, &data, &CharNameCache::default(), false);
        assert!(win.item.as_ref().unwrap().is_card);
        assert!(!win.item.as_ref().unwrap().is_equipment);

        let weapon = make_item(1201, 4, [0; 4]);
        win.show(&weapon, &data, &CharNameCache::default(), false);
        assert!(!win.item.as_ref().unwrap().is_card);
        assert!(win.item.as_ref().unwrap().is_equipment);
    }

    #[test]
    fn show_illustration_opens_and_toggle_closes() {
        let mut win = ItemInfoWindow::new();
        win.show_illustration(
            4001,
            "Test Card".to_string(),
            "data/texture/cardbmp/test.bmp".to_string(),
        );
        assert!(win.card_illustration.is_some());
        assert_eq!(win.card_illustration.as_ref().unwrap().item_id, 4001);

        win.show_illustration(
            4001,
            "Test Card".to_string(),
            "data/texture/cardbmp/test.bmp".to_string(),
        );
        assert!(win.card_illustration.is_none());
    }

    #[test]
    fn show_illustration_switches_to_different_card() {
        let mut win = ItemInfoWindow::new();
        win.show_illustration(4001, "Card A".to_string(), "a.bmp".to_string());
        assert_eq!(win.card_illustration.as_ref().unwrap().item_id, 4001);

        win.show_illustration(4002, "Card B".to_string(), "b.bmp".to_string());
        assert_eq!(win.card_illustration.as_ref().unwrap().item_id, 4002);
    }

    #[test]
    fn pending_illustration_texture_paths() {
        let mut win = ItemInfoWindow::new();
        assert!(win.pending_illustration_texture_paths().is_empty());

        win.show_illustration(
            4001,
            "Test".to_string(),
            "data/texture/cardbmp/test.bmp".to_string(),
        );
        let paths = win.pending_illustration_texture_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], "data/texture/cardbmp/test.bmp");
    }
}
