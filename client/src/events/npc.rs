use crate::App;

impl App {
    pub(super) fn handle_npc_shop_buy_list(
        &mut self,
        npc_id: u32,
        items: Vec<(u16, i32, i32, u8)>,
    ) {
        let fallback_npc_id = self.game.npc_dialog.dialog.npc_id;
        let icon_paths = self.game.npc_shop.shop.apply_buy_list(
            npc_id,
            fallback_npc_id,
            items,
            &self.game.data_table,
        );
        self.game.npc_dialog.dialog.close();
        self.preload_item_icons(icon_paths);
    }

    pub(super) fn handle_npc_shop_sell_list(&mut self, npc_id: u32, items: Vec<(i16, i32, i32)>) {
        let fallback_npc_id = self.game.npc_dialog.dialog.npc_id;
        let icon_paths = self.game.npc_shop.shop.apply_sell_list(
            npc_id,
            fallback_npc_id,
            items,
            &self.game.character.inventory,
        );
        self.game.npc_dialog.dialog.close();
        self.preload_item_icons(icon_paths);
    }

    pub(super) fn handle_npc_shop_buy_result(&mut self, result: u8) {
        let msg = self.game.npc_shop.shop.apply_buy_result(result);
        self.game.chat_window.add_chat(msg.into());
    }

    pub(super) fn handle_npc_shop_sell_result(&mut self, result: u8) {
        let msg = self.game.npc_shop.shop.apply_sell_result(result);
        self.game.chat_window.add_chat(msg.into());
    }
}
