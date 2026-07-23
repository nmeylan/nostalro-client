use crate::item::Item;

pub const TRADE_MAX_SLOTS: usize = 10;

/// Sent as `index` in an add request to mean "this is zeny, not an inventory item".
pub const TRADE_ZENY_INDEX: u16 = 0;

/// `who` values in ZC_CONCLUDE_EXCHANGE_ITEM.
pub const CONCLUDE_ME: u8 = 0;
pub const CONCLUDE_OTHER: u8 = 1;

#[derive(Debug, Default)]
pub struct TradeData {
    active: bool,
    partner_name: String,
    partner_aid: u32,
    partner_level: i16,
    my_level: i16,
    my_items: Vec<Item>,
    other_items: Vec<Item>,
    my_zeny: i64,
    other_zeny: i64,
    my_locked: bool,
    other_locked: bool,
    /// Index+count of an add request awaiting its server ack, so a success can be
    /// reflected onto our own side. Index `TRADE_ZENY_INDEX` means zeny.
    pending_add: Option<(u16, i32)>,
}

impl TradeData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn begin(
        &mut self,
        partner_name: String,
        partner_aid: u32,
        partner_level: i16,
        my_level: i16,
    ) {
        *self = Self {
            active: true,
            partner_name,
            partner_aid,
            partner_level,
            my_level,
            ..Self::default()
        };
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn partner_name(&self) -> &str {
        &self.partner_name
    }
    pub fn partner_aid(&self) -> u32 {
        self.partner_aid
    }
    pub fn partner_level(&self) -> i16 {
        self.partner_level
    }
    pub fn my_level(&self) -> i16 {
        self.my_level
    }
    pub fn my_items(&self) -> &[Item] {
        &self.my_items
    }
    pub fn other_items(&self) -> &[Item] {
        &self.other_items
    }
    pub fn my_zeny(&self) -> i64 {
        self.my_zeny
    }
    pub fn other_zeny(&self) -> i64 {
        self.other_zeny
    }
    pub fn my_locked(&self) -> bool {
        self.my_locked
    }
    pub fn other_locked(&self) -> bool {
        self.other_locked
    }
    pub fn both_locked(&self) -> bool {
        self.my_locked && self.other_locked
    }
    pub fn my_slots_full(&self) -> bool {
        self.my_items.len() >= TRADE_MAX_SLOTS
    }

    pub fn has_my_index(&self, index: u16) -> bool {
        self.my_items.iter().any(|i| i.index == index)
    }

    pub fn add_other_item(&mut self, item: Item) {
        if self.other_items.len() < TRADE_MAX_SLOTS {
            self.other_items.push(item);
        }
    }

    pub fn add_my_item(&mut self, item: Item) {
        if self.my_items.len() < TRADE_MAX_SLOTS {
            self.my_items.push(item);
        }
    }

    pub fn set_other_zeny(&mut self, zeny: i64) {
        self.other_zeny = zeny;
    }

    pub fn add_my_zeny(&mut self, zeny: i64) {
        self.my_zeny += zeny;
    }

    pub fn set_pending_add(&mut self, index: u16, count: i32) {
        self.pending_add = Some((index, count));
    }

    pub fn take_pending_add(&mut self) -> Option<(u16, i32)> {
        self.pending_add.take()
    }

    pub fn lock(&mut self, who: u8) {
        match who {
            CONCLUDE_ME => self.my_locked = true,
            CONCLUDE_OTHER => self.other_locked = true,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Item;
    use models::enums::item::ItemType;

    fn item(index: u16, id: u16, count: i16) -> Item {
        Item {
            index,
            item_id: id,
            item_type: ItemType::Healing,
            count,
            is_identified: true,
            is_damaged: false,
            refining_level: 0,
            slot: [0; 4],
            location: 0,
            wear_state: 0,
            name: format!("Item {id}"),
            resource_name: None,
        }
    }

    #[test]
    fn full_round_trip_completes_and_resets() {
        let mut trade = TradeData::new();
        trade.begin("Bob".into(), 2000, 55, 60);
        assert!(trade.is_active());

        // Other side offers an item and some zeny.
        trade.add_other_item(item(0, 909, 5));
        trade.set_other_zeny(1000);

        // I add an item: request pending, then acked ok reflects onto my side.
        trade.set_pending_add(7, 3);
        let (idx, cnt) = trade.take_pending_add().unwrap();
        assert_eq!((idx, cnt), (7, 3));
        trade.add_my_item(item(7, 501, 3));

        // Both sides lock, only then can the trade execute.
        assert!(!trade.both_locked());
        trade.lock(CONCLUDE_ME);
        trade.lock(CONCLUDE_OTHER);
        assert!(trade.both_locked());

        // Completion clears everything.
        trade.reset();
        assert!(!trade.is_active());
        assert!(trade.my_items().is_empty());
        assert!(trade.other_items().is_empty());
    }

    #[test]
    fn cancel_mid_trade_clears_state() {
        let mut trade = TradeData::new();
        trade.begin("Bob".into(), 2000, 55, 60);
        trade.add_other_item(item(0, 909, 5));
        trade.lock(CONCLUDE_ME);
        trade.reset();
        assert!(!trade.is_active());
        assert!(!trade.my_locked());
    }
}
