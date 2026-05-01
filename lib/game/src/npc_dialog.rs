#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcDialogState {
    Idle,
    DisplayingText,
    WaitingForNext,
    WaitingForClose,
    WaitingForMenu,
    WaitingForNumberInput,
    WaitingForStringInput,
    WaitingForDealType,
}

#[derive(Debug)]
pub struct NpcDialogData {
    pub state: NpcDialogState,
    pub npc_id: u32,
    pub text: String,
    pub menu_items: Vec<String>,
    pub selected_menu_index: usize,
    pub menu_scroll_offset: usize,
}

impl NpcDialogData {
    pub fn new() -> Self {
        Self {
            state: NpcDialogState::Idle,
            npc_id: 0,
            text: String::new(),
            menu_items: Vec::new(),
            selected_menu_index: 0,
            menu_scroll_offset: 0,
        }
    }

    pub fn is_open(&self) -> bool {
        self.state != NpcDialogState::Idle
    }

    pub fn open_text(&mut self, npc_id: u32, text: &str) {
        if self.state == NpcDialogState::Idle || self.npc_id != npc_id {
            self.text.clear();
        }
        self.npc_id = npc_id;
        if !self.text.is_empty() {
            self.text.push('\n');
        }
        self.text.push_str(text);
        self.state = NpcDialogState::DisplayingText;
    }

    pub fn wait_for_next(&mut self, npc_id: u32) {
        self.npc_id = npc_id;
        self.state = NpcDialogState::WaitingForNext;
    }

    pub fn wait_for_close(&mut self, npc_id: u32) {
        self.npc_id = npc_id;
        self.state = NpcDialogState::WaitingForClose;
    }

    pub fn show_menu(&mut self, npc_id: u32, items: Vec<String>) {
        self.npc_id = npc_id;
        self.menu_items = items;
        self.selected_menu_index = 0;
        self.menu_scroll_offset = 0;
        self.state = NpcDialogState::WaitingForMenu;
    }

    pub fn wait_for_number_input(&mut self, npc_id: u32) {
        self.npc_id = npc_id;
        self.state = NpcDialogState::WaitingForNumberInput;
    }

    pub fn wait_for_string_input(&mut self, npc_id: u32) {
        self.npc_id = npc_id;
        self.state = NpcDialogState::WaitingForStringInput;
    }

    pub fn show_deal_type(&mut self, npc_id: u32) {
        self.npc_id = npc_id;
        self.state = NpcDialogState::WaitingForDealType;
    }

    pub fn advance_next(&mut self) {
        self.text.clear();
        self.state = NpcDialogState::DisplayingText;
    }

    pub fn close(&mut self) {
        self.state = NpcDialogState::Idle;
        self.npc_id = 0;
        self.text.clear();
        self.menu_items.clear();
        self.selected_menu_index = 0;
        self.menu_scroll_offset = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_lifecycle() {
        let mut dialog = NpcDialogData::new();
        assert!(!dialog.is_open());

        // NPC sends text
        dialog.open_text(100, "Hello adventurer!");
        assert!(dialog.is_open());
        assert_eq!(dialog.state, NpcDialogState::DisplayingText);
        assert_eq!(dialog.text, "Hello adventurer!");

        // Server sends wait (Next button)
        dialog.wait_for_next(100);
        assert_eq!(dialog.state, NpcDialogState::WaitingForNext);

        // Player clicks Next => text cleared
        dialog.advance_next();
        assert_eq!(dialog.state, NpcDialogState::DisplayingText);
        assert!(dialog.text.is_empty());

        // More text arrives
        dialog.open_text(100, "Choose wisely.");
        assert_eq!(dialog.text, "Choose wisely.");

        // Server sends menu
        dialog.show_menu(100, vec!["Buy".into(), "Sell".into(), "Cancel".into()]);
        assert_eq!(dialog.state, NpcDialogState::WaitingForMenu);
        assert_eq!(dialog.menu_items.len(), 3);

        // Player selects, server sends close
        dialog.wait_for_close(100);
        assert_eq!(dialog.state, NpcDialogState::WaitingForClose);

        // Player clicks close
        dialog.close();
        assert!(!dialog.is_open());
    }

    #[test]
    fn text_accumulates_within_same_npc() {
        let mut dialog = NpcDialogData::new();
        dialog.open_text(100, "Line 1");
        dialog.open_text(100, "Line 2");
        assert_eq!(dialog.text, "Line 1\nLine 2");
    }
}
