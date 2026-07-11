use crate::helper::window_chrome::text_color;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

const MENU_BASE_ID: u32 = 2100;
const ITEM_W: f32 = 120.0;
const ITEM_H: f32 = 18.0;

#[derive(Clone, Copy)]
pub enum ContextMenuAction {
    InviteToParty { target_aid: u32 },
    CompanionShowInfo { is_mercenary: bool },
    CompanionFeed,
    CompanionStandby { is_mercenary: bool },
}

pub struct ContextMenuItem {
    pub label: String,
    pub action: ContextMenuAction,
}

#[derive(Default)]
pub struct ContextMenu {
    open: bool,
    x: f32,
    y: f32,
    items: Vec<ContextMenuItem>,
}

impl ContextMenu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_at(&mut self, x: f32, y: f32, items: Vec<ContextMenuItem>) {
        if items.is_empty() {
            return;
        }
        self.open = true;
        self.x = x;
        self.y = y;
        self.items = items;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.items.clear();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        if !self.open {
            return Vec::new();
        }
        let tc = text_color(false);
        let mut events = Vec::new();

        let menu_h = self.items.len() as f32 * ITEM_H;
        let panel = Rect::new(self.x, self.y, ITEM_W, menu_h);

        let (v, i) = draw::quad_vertices(panel.x, panel.y, panel.w, panel.h, [0.1, 0.1, 0.15, 0.96]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let bc = [0.45, 0.45, 0.55, 1.0];
        for (bx, by, bw, bh) in [
            (panel.x, panel.y, panel.w, 1.0),
            (panel.x, panel.y, 1.0, panel.h),
            (panel.x + panel.w - 1.0, panel.y, 1.0, panel.h),
            (panel.x, panel.y + panel.h - 1.0, panel.w, 1.0),
        ] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, bc);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }

        let mut clicked_item = None;
        let mut any_hovered = false;
        for (idx, item) in self.items.iter().enumerate() {
            let iy = self.y + idx as f32 * ITEM_H;
            let rect = Rect::new(self.x, iy, ITEM_W, ITEM_H);
            let resp = ui.interact(WidgetId(MENU_BASE_ID + idx as u32), rect);
            if resp.hovered() {
                any_hovered = true;
                ui.any_interactive_hovered = true;
                let (v, i) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, [0.3, 0.3, 0.45, 1.0]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }
            ui.text(rect.x + 6.0, rect.y + 13.0, &item.label, tc);
            if resp.clicked() {
                clicked_item = Some(item.action);
            }
        }

        if let Some(action) = clicked_item {
            match action {
                ContextMenuAction::InviteToParty { target_aid } => {
                    events.push(GameEvent::RequestPartyInvite { target_aid });
                }
                ContextMenuAction::CompanionShowInfo { is_mercenary } => {
                    events.push(if is_mercenary {
                        GameEvent::ToggleMercenaryWindow
                    } else {
                        GameEvent::ToggleHomunculusWindow
                    });
                }
                ContextMenuAction::CompanionFeed => {
                    events.push(GameEvent::RequestHomunMenu { command: 1 });
                }
                ContextMenuAction::CompanionStandby { is_mercenary } => {
                    events.push(GameEvent::ToggleCompanionStandby { is_mercenary });
                }
            }
            self.close();
        } else if ui.ctx.mouse_clicked && !any_hovered {
            self.close();
        }

        events
    }
}
