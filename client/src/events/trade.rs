use crate::App;
use models::enums::EnumWithNumberValue;
use models::enums::item::ItemType;
use ragnarok_game::item::Item;
use ragnarok_game::trade::TRADE_ZENY_INDEX;

impl App {
    pub(super) fn handle_exchange_requested(&mut self, name: String, gid: u32, _level: i16) {
        if self
            .game
            .begin_trade_request(&mut self.windows, name, gid, self.config.refuse_trade)
        {
            self.respond_exchange_request(false);
        }
    }

    pub(crate) fn respond_exchange_request(&mut self, accept: bool) {
        self.game.pending_confirms.pending_trade_request = None;
        if !accept {
            self.game.pending_confirms.pending_trade_partner = None;
        }
        let result = if accept { 3 } else { 4 };
        self.channel
            .send_packet(ragnarok_network::build_ack_exchange_item_packet(
                result,
                self.active_packetver,
            ));
    }

    pub(super) fn handle_exchange_ack_result(&mut self, result: u8, level: i16) {
        match result {
            3 => {
                let (aid, name) = self
                    .game
                    .pending_confirms
                    .pending_trade_partner
                    .take()
                    .unwrap_or((0, String::new()));
                let my_level = self.game.character.base_level as i16;
                self.game.character.trade.begin(name, aid, level, my_level);
                self.game.character.inventory.open();
            }
            0 => self
                .windows
                .chat_window
                .add_error("The character is too far away to trade.".to_string()),
            1 => self
                .windows
                .chat_window
                .add_error("The character does not exist.".to_string()),
            4 => self
                .windows
                .chat_window
                .add_error("The deal was rejected.".to_string()),
            5 => self
                .windows
                .chat_window
                .add_error("The other player is busy dealing.".to_string()),
            _ => self
                .windows
                .chat_window
                .add_error("Trade has failed.".to_string()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn handle_exchange_item_added(
        &mut self,
        item_id: u16,
        item_type: u8,
        count: i32,
        is_identified: bool,
        is_damaged: bool,
        refining_level: u8,
        slot: [u16; 4],
    ) {
        if item_id == 0 {
            self.game.character.trade.set_other_zeny(count as i64);
            return;
        }
        let item = self.build_trade_item(
            0,
            item_id,
            count as i16,
            item_type,
            is_identified,
            is_damaged,
            refining_level,
            slot,
        );
        if let Some(path) = item.icon_path() {
            self.preload_item_icons(vec![path]);
        }
        self.game.character.trade.add_other_item(item);
    }

    pub(super) fn handle_exchange_add_result(&mut self, _index: u16, result: u8) {
        let pending = self.game.character.trade.take_pending_add();
        match result {
            0 => {
                if let Some((idx, cnt)) = pending {
                    if idx == TRADE_ZENY_INDEX {
                        self.game.character.trade.add_my_zeny(cnt as i64);
                        self.game.character.inventory.zeny =
                            (self.game.character.inventory.zeny - cnt).max(0);
                    } else if let Some(src) = self.game.character.inventory.get_item(idx) {
                        let mut item = src.clone();
                        item.count = cnt as i16;
                        self.game.character.trade.add_my_item(item);
                    }
                }
            }
            1 => self
                .windows
                .chat_window
                .add_error("You are over your weight limit.".to_string()),
            4 => self
                .windows
                .chat_window
                .add_error("That amount cannot be traded.".to_string()),
            _ => self
                .windows
                .chat_window
                .add_error("The item cannot be added to the deal.".to_string()),
        }
    }

    pub(super) fn handle_exchange_concluded(&mut self, who: u8) {
        self.game.character.trade.lock(who);
    }

    pub(super) fn handle_exchange_canceled(&mut self) {
        self.game.character.trade.reset();
        self.windows.trade_window.reset_input();
        self.game.pending_confirms.pending_trade_partner = None;
        self.windows
            .chat_window
            .add_system("The deal has been canceled.".to_string());
    }

    pub(super) fn handle_exchange_completed(&mut self, result: u8) {
        if result == 0 {
            self.windows
                .chat_window
                .add_system("Deal successful.".to_string());
        } else {
            self.windows
                .chat_window
                .add_error("The deal has failed.".to_string());
        }
        self.game.character.trade.reset();
        self.windows.trade_window.reset_input();
        self.game.pending_confirms.pending_trade_partner = None;
    }

    pub(super) fn handle_exchange_undo(&mut self) {
        self.game.character.trade.take_pending_add();
    }

    #[allow(clippy::too_many_arguments)]
    fn build_trade_item(
        &self,
        index: u16,
        item_id: u16,
        count: i16,
        item_type: u8,
        is_identified: bool,
        is_damaged: bool,
        refining_level: u8,
        slot: [u16; 4],
    ) -> Item {
        let resolved_type = if item_type != 0 {
            ItemType::from_value(item_type as usize)
        } else {
            ItemType::from_value(0)
        };
        let name = self
            .game
            .data_table
            .item_name
            .as_ref()
            .map(|t| t.get_name_or_id_for(item_id, is_identified))
            .unwrap_or_else(|| format!("Item #{item_id}"));
        let resource_name = self.game.data_table.item_resource.as_ref().and_then(|t| {
            t.get_resource_name_for(item_id, is_identified)
                .map(|s| s.to_string())
        });
        Item {
            index,
            item_id,
            item_type: resolved_type,
            count,
            is_identified,
            is_damaged,
            refining_level,
            slot,
            location: 0,
            wear_state: 0,
            name,
            resource_name,
        }
    }
}
