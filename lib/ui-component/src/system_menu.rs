use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

use crate::confirm_dialog::{ConfirmDialog, ConfirmResult};

const BG_ID: WidgetId = WidgetId(510);
const RESUME_ID: WidgetId = WidgetId(500);
const OPTION_ID: WidgetId = WidgetId(501);
const CHARSELECT_ID: WidgetId = WidgetId(502);
const QUIT_ID: WidgetId = WidgetId(503);

const MENU_W: f32 = 140.0;
const BTN_W: f32 = 120.0;
const BTN_H: f32 = 24.0;
const BTN_SPACING: f32 = 4.0;
const PADDING_TOP: f32 = 12.0;
const PADDING_BOTTOM: f32 = 12.0;
const MENU_H: f32 = PADDING_TOP + 4.0 * BTN_H + 3.0 * BTN_SPACING + PADDING_BOTTOM;

// Fallback button textures (no-op, buttons use fallback labels)
const DUMMY_BTN: ragnarok_ui::frame::ButtonTextures = ragnarok_ui::frame::ButtonTextures {
    normal: "", hover: "", pressed: "",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingConfirm {
    None,
    CharacterSelect,
    QuitGame,
}

pub struct SystemMenu {
    pub open: bool,
    pub has_grf_textures: bool,
    pending_confirm: PendingConfirm,
    confirm_dialog: ConfirmDialog,
}

impl SystemMenu {
    pub fn new() -> Self {
        Self {
            open: false,
            has_grf_textures: false,
            pending_confirm: PendingConfirm::None,
            confirm_dialog: ConfirmDialog::new("Are you sure?"),
        }
    }

    pub fn build(&mut self, ui: &mut UiFrame, allow_escape_toggle: bool) -> Vec<GameEvent> {
        let mut events = Vec::new();

        // Toggle menu on Escape (only when no confirm dialog is showing)
        if allow_escape_toggle && ui.ctx.key_escape && self.pending_confirm == PendingConfirm::None {
            self.open = !self.open;
            if !self.open {
                return events;
            }
        }

        if !self.open {
            return events;
        }

        // Full-screen interact to block game input
        let screen = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);
        ui.interact(BG_ID, screen);

        // Handle confirmation dialog if pending
        if self.pending_confirm != PendingConfirm::None {
            match self.confirm_dialog.build(ui) {
                ConfirmResult::Ok => {
                    match self.pending_confirm {
                        PendingConfirm::CharacterSelect => events.push(GameEvent::BackToCharacterSelect),
                        PendingConfirm::QuitGame => events.push(GameEvent::QuitGame),
                        PendingConfirm::None => {}
                    }
                    self.pending_confirm = PendingConfirm::None;
                    self.open = false;
                }
                ConfirmResult::Cancel => {
                    self.pending_confirm = PendingConfirm::None;
                }
                ConfirmResult::None => {}
            }
            return events;
        }

        // Menu panel centered on screen
        let mx = ((ui.ctx.screen_width - MENU_W) / 2.0).floor();
        let my = ((ui.ctx.screen_height - MENU_H) / 2.0).floor();

        // Background
        let (v, i) = draw::quad_vertices(mx, my, MENU_W, MENU_H, [0.2, 0.2, 0.28, 0.95]);
        ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });
        let border_color = [0.5, 0.5, 0.6, 1.0];
        for (bx, by, bw, bh) in [
            (mx, my, MENU_W, 1.0),
            (mx, my + MENU_H - 1.0, MENU_W, 1.0),
            (mx, my, 1.0, MENU_H),
            (mx + MENU_W - 1.0, my, 1.0, MENU_H),
        ] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, border_color);
            ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });
        }

        // Buttons
        let btn_x = mx + (MENU_W - BTN_W) / 2.0;
        let btn_y = |idx: usize| my + PADDING_TOP + idx as f32 * (BTN_H + BTN_SPACING);

        let resume = ui.button(RESUME_ID, Rect::new(btn_x, btn_y(0), BTN_W, BTN_H), &DUMMY_BTN, "Resume");
        let option = ui.button(OPTION_ID, Rect::new(btn_x, btn_y(1), BTN_W, BTN_H), &DUMMY_BTN, "Option");
        let charselect = ui.button(CHARSELECT_ID, Rect::new(btn_x, btn_y(2), BTN_W, BTN_H), &DUMMY_BTN, "Character Select");
        let quit = ui.button(QUIT_ID, Rect::new(btn_x, btn_y(3), BTN_W, BTN_H), &DUMMY_BTN, "Quit Game");

        if resume.clicked() {
            self.open = false;
        }
        // Option is a no-op for now
        let _ = option;
        if charselect.clicked() {
            self.pending_confirm = PendingConfirm::CharacterSelect;
        }
        if quit.clicked() {
            self.pending_confirm = PendingConfirm::QuitGame;
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_renderer::font_atlas::FontAtlas;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0);
        let atlas = Box::leak(Box::new(atlas));
        UiFrame::new(ctx, atlas, state, 0.0, false, None)
    }

    #[test]
    fn escape_toggles_menu() {
        let mut menu = SystemMenu::new();
        let mut state = StateCache::new();
        assert!(!menu.open);

        // Escape opens the menu
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);
        menu.build(&mut ui, true);
        assert!(menu.open);

        // Escape again closes it
        let mut ui = make_frame(&ctx, &mut state);
        menu.build(&mut ui, true);
        assert!(!menu.open);
    }

    #[test]
    fn escape_blocked_when_not_allowed() {
        let mut menu = SystemMenu::new();
        let mut state = StateCache::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);
        menu.build(&mut ui, false);
        assert!(!menu.open);
    }

    #[test]
    fn resume_closes_menu() {
        let mut menu = SystemMenu::new();
        menu.open = true;
        let mut state = StateCache::new();

        // Click on the Resume button area
        // Menu is centered at (330, 230) for 800x600 screen, resume btn at y=230+12=242
        let mut ctx = UiContext::new(800.0, 600.0);
        let btn_x = ((800.0 - MENU_W) / 2.0).floor() + (MENU_W - BTN_W) / 2.0;
        let btn_y = ((600.0 - MENU_H) / 2.0).floor() + PADDING_TOP;
        ctx.mouse_x = btn_x + BTN_W / 2.0;
        ctx.mouse_y = btn_y + BTN_H / 2.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        menu.build(&mut ui, true);
        assert!(!menu.open);
    }

    #[test]
    fn charselect_opens_confirm_then_emits_event() {
        let mut menu = SystemMenu::new();
        menu.open = true;
        let mut state = StateCache::new();

        // Click Character Select button (index 2)
        let mut ctx = UiContext::new(800.0, 600.0);
        let btn_x = ((800.0 - MENU_W) / 2.0).floor() + (MENU_W - BTN_W) / 2.0;
        let btn_y = ((600.0 - MENU_H) / 2.0).floor() + PADDING_TOP + 2.0 * (BTN_H + BTN_SPACING);
        ctx.mouse_x = btn_x + BTN_W / 2.0;
        ctx.mouse_y = btn_y + BTN_H / 2.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = menu.build(&mut ui, true);
        assert!(events.is_empty());
        assert_eq!(menu.pending_confirm, PendingConfirm::CharacterSelect);

        // Press Enter to confirm
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = menu.build(&mut ui, true);
        assert!(events.iter().any(|e| matches!(e, GameEvent::BackToCharacterSelect)));
        assert!(!menu.open);
    }

    #[test]
    fn quit_opens_confirm_then_emits_event() {
        let mut menu = SystemMenu::new();
        menu.open = true;
        let mut state = StateCache::new();

        // Click Quit button (index 3)
        let mut ctx = UiContext::new(800.0, 600.0);
        let btn_x = ((800.0 - MENU_W) / 2.0).floor() + (MENU_W - BTN_W) / 2.0;
        let btn_y = ((600.0 - MENU_H) / 2.0).floor() + PADDING_TOP + 3.0 * (BTN_H + BTN_SPACING);
        ctx.mouse_x = btn_x + BTN_W / 2.0;
        ctx.mouse_y = btn_y + BTN_H / 2.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = menu.build(&mut ui, true);
        assert!(events.is_empty());
        assert_eq!(menu.pending_confirm, PendingConfirm::QuitGame);

        // Press Enter to confirm
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = menu.build(&mut ui, true);
        assert!(events.iter().any(|e| matches!(e, GameEvent::QuitGame)));
        assert!(!menu.open);
    }

    #[test]
    fn confirm_cancel_returns_to_menu() {
        let mut menu = SystemMenu::new();
        menu.open = true;
        menu.pending_confirm = PendingConfirm::CharacterSelect;
        let mut state = StateCache::new();

        // Press Escape to cancel the confirm dialog
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = menu.build(&mut ui, true);
        assert!(events.is_empty());
        assert_eq!(menu.pending_confirm, PendingConfirm::None);
        assert!(menu.open);
    }
}
