use crate::data_table::item_resource_table::ItemResourceTable;
use crate::item::Item;
use models::enums::EnumWithNumberValue;
use models::enums::item::ItemType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcShopMode {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct ShopBuyItem {
    pub item: Item,
    pub price: i32,
    pub discount_price: i32,
}

#[derive(Debug, Clone)]
pub struct ShopSellItem {
    pub item: Item,
    pub price: i32,
    pub overcharge_price: i32,
}

#[derive(Debug, Clone)]
pub struct ShopBasketItem {
    pub source_index: usize,
    pub quantity: i16,
}

#[derive(Debug)]
pub struct NpcShopData {
    pub mode: Option<NpcShopMode>,
    pub npc_id: u32,
    pub buy_items: Vec<ShopBuyItem>,
    pub sell_items: Vec<ShopSellItem>,
    pub basket: Vec<ShopBasketItem>,
    pub selected_index: Option<usize>,
}

impl Default for NpcShopData {
    fn default() -> Self {
        Self::new()
    }
}

impl NpcShopData {
    pub fn new() -> Self {
        Self {
            mode: None,
            npc_id: 0,
            buy_items: Vec::new(),
            sell_items: Vec::new(),
            basket: Vec::new(),
            selected_index: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.mode.is_some()
    }

    pub fn open_buy(&mut self, npc_id: u32, items: Vec<ShopBuyItem>) {
        self.mode = Some(NpcShopMode::Buy);
        self.npc_id = npc_id;
        self.buy_items = items;
        self.sell_items.clear();
        self.basket.clear();
        self.selected_index = None;
    }

    pub fn open_sell(&mut self, npc_id: u32, items: Vec<ShopSellItem>) {
        self.mode = Some(NpcShopMode::Sell);
        self.npc_id = npc_id;
        self.sell_items = items;
        self.buy_items.clear();
        self.basket.clear();
        self.selected_index = None;
    }

    pub fn add_to_basket(&mut self, source_index: usize, quantity: i16) {
        let stackable = self
            .item_at(source_index)
            .map(|i| i.item_type.is_stackable())
            .unwrap_or(true);
        if let Some(existing) = self
            .basket
            .iter_mut()
            .find(|c| c.source_index == source_index)
        {
            if !stackable {
                return;
            }
            existing.quantity += quantity;
        } else {
            self.basket.push(ShopBasketItem {
                source_index,
                quantity,
            });
        }
    }

    pub fn remove_from_basket(&mut self, basket_index: usize) {
        if basket_index < self.basket.len() {
            self.basket.remove(basket_index);
        }
    }

    pub fn basket_total(&self) -> i64 {
        self.basket
            .iter()
            .map(|basket_item| {
                let unit_price = match self.mode {
                    Some(NpcShopMode::Buy) => self
                        .buy_items
                        .get(basket_item.source_index)
                        .map(|i| i.discount_price)
                        .unwrap_or(0),
                    Some(NpcShopMode::Sell) => self
                        .sell_items
                        .get(basket_item.source_index)
                        .map(|i| i.overcharge_price)
                        .unwrap_or(0),
                    None => 0,
                };
                unit_price as i64 * basket_item.quantity as i64
            })
            .sum()
    }

    pub fn item_count(&self) -> usize {
        match self.mode {
            Some(NpcShopMode::Buy) => self.buy_items.len(),
            Some(NpcShopMode::Sell) => self.sell_items.len(),
            None => 0,
        }
    }

    pub fn item_name(&self, index: usize) -> &str {
        match self.mode {
            Some(NpcShopMode::Buy) => self
                .buy_items
                .get(index)
                .map(|i| i.item.name.as_str())
                .unwrap_or(""),
            Some(NpcShopMode::Sell) => self
                .sell_items
                .get(index)
                .map(|i| i.item.name.as_str())
                .unwrap_or(""),
            None => "",
        }
    }

    pub fn item_at(&self, index: usize) -> Option<&Item> {
        match self.mode {
            Some(NpcShopMode::Buy) => self.buy_items.get(index).map(|i| &i.item),
            Some(NpcShopMode::Sell) => self.sell_items.get(index).map(|i| &i.item),
            None => None,
        }
    }

    pub fn item_icon_path(&self, index: usize) -> Option<String> {
        match self.mode {
            Some(NpcShopMode::Buy) => self.buy_items.get(index).and_then(|i| i.item.icon_path()),
            Some(NpcShopMode::Sell) => self.sell_items.get(index).and_then(|i| i.item.icon_path()),
            None => None,
        }
    }

    pub fn item_is_identified(&self, index: usize) -> bool {
        match self.mode {
            Some(NpcShopMode::Buy) => true,
            Some(NpcShopMode::Sell) => self
                .sell_items
                .get(index)
                .map(|i| i.item.is_identified)
                .unwrap_or(true),
            None => true,
        }
    }

    /// Undiscounted/base price. `item_price` returns the discount/overcharge-adjusted
    /// value; the two differ only when the merchant has the relevant skill.
    pub fn item_base_price(&self, index: usize) -> i32 {
        match self.mode {
            Some(NpcShopMode::Buy) => self.buy_items.get(index).map(|i| i.price).unwrap_or(0),
            Some(NpcShopMode::Sell) => self.sell_items.get(index).map(|i| i.price).unwrap_or(0),
            None => 0,
        }
    }

    pub fn item_price(&self, index: usize) -> i32 {
        match self.mode {
            Some(NpcShopMode::Buy) => self
                .buy_items
                .get(index)
                .map(|i| i.discount_price)
                .unwrap_or(0),
            Some(NpcShopMode::Sell) => self
                .sell_items
                .get(index)
                .map(|i| i.overcharge_price)
                .unwrap_or(0),
            None => 0,
        }
    }

    pub fn sell_item_count(&self, index: usize) -> i16 {
        self.sell_items
            .get(index)
            .map(|i| i.item.count)
            .unwrap_or(1)
    }

    pub fn sell_item_remaining(&self, index: usize) -> i16 {
        let total = self.sell_item_count(index);
        let in_basket: i16 = self
            .basket
            .iter()
            .filter(|c| c.source_index == index)
            .map(|c| c.quantity)
            .sum();
        total - in_basket
    }

    pub fn needs_quantity_prompt(&self, index: usize) -> bool {
        let Some(item) = self.item_at(index) else {
            return false;
        };
        if !item.item_type.is_stackable() {
            return false;
        }
        match self.mode {
            Some(NpcShopMode::Sell) => self.sell_item_remaining(index) > 1,
            Some(NpcShopMode::Buy) => true,
            None => false,
        }
    }

    pub fn visible_sell_indices(&self) -> Vec<usize> {
        (0..self.sell_items.len())
            .filter(|&i| self.sell_item_remaining(i) > 0)
            .collect()
    }

    pub fn remove_sold_items(&mut self) {
        for basket_item in &self.basket {
            if let Some(sell_item) = self.sell_items.get_mut(basket_item.source_index) {
                sell_item.item.count -= basket_item.quantity;
            }
        }
        self.sell_items.retain(|i| i.item.count > 0);
        self.basket.clear();
        self.selected_index = None;
    }

    pub fn apply_buy_list(
        &mut self,
        npc_id: u32,
        fallback_npc_id: u32,
        items: Vec<(u16, i32, i32, u8)>,
        data_table: &crate::data_table::DataTable,
    ) -> Vec<String> {
        let buy_items: Vec<ShopBuyItem> = items
            .into_iter()
            .map(|(item_id, price, discount_price, item_type)| {
                let name = data_table
                    .item_name
                    .as_ref()
                    .map(|t| t.get_name_or_id(item_id))
                    .unwrap_or_else(|| format!("Item #{item_id}"));
                let resource_name = data_table
                    .item_resource
                    .as_ref()
                    .and_then(|t| t.get_resource_name(item_id).map(|s| s.to_string()));
                ShopBuyItem {
                    item: Item {
                        index: 0,
                        item_id,
                        item_type: ItemType::from_value(item_type as usize),
                        count: 1,
                        is_identified: true,
                        is_damaged: false,
                        refining_level: 0,
                        slot: [0; 4],
                        location: 0,
                        wear_state: 0,
                        name,
                        resource_name,
                    },
                    price,
                    discount_price,
                }
            })
            .collect();
        let shop_npc_id = if npc_id != 0 { npc_id } else { fallback_npc_id };
        self.open_buy(shop_npc_id, buy_items);
        self.buy_items
            .iter()
            .filter_map(|i| i.item.icon_path())
            .collect()
    }

    pub fn apply_sell_list(
        &mut self,
        npc_id: u32,
        fallback_npc_id: u32,
        items: Vec<(i16, i32, i32)>,
        inventory: &crate::inventory::InventoryData,
    ) -> Vec<String> {
        let sell_items: Vec<ShopSellItem> = items
            .into_iter()
            .filter_map(|(index, price, overcharge_price)| {
                let inv_item = inventory.get_item(index as u16)?;
                Some(ShopSellItem {
                    item: inv_item.clone(),
                    price,
                    overcharge_price,
                })
            })
            .collect();
        let shop_npc_id = if npc_id != 0 { npc_id } else { fallback_npc_id };
        self.open_sell(shop_npc_id, sell_items);
        self.sell_items
            .iter()
            .filter_map(|i| i.item.icon_path())
            .collect()
    }

    pub fn apply_buy_result(&mut self, result: u8) -> &'static str {
        self.close();
        match result {
            0 => "Purchase completed.",
            1 => "Not enough zeny.",
            2 => "You are overweight.",
            _ => "Purchase failed.",
        }
    }

    pub fn apply_sell_result(&mut self, result: u8) -> &'static str {
        self.close();
        match result {
            0 => "Sale completed.",
            _ => "Sell failed.",
        }
    }

    pub fn resolve_resource_names(&mut self, table: &ItemResourceTable) {
        for buy_item in &mut self.buy_items {
            buy_item.item.resolve_resource_name(table);
        }
        for item in &mut self.sell_items {
            item.item.resolve_resource_name(table);
        }
    }

    pub fn close(&mut self) {
        self.mode = None;
        self.npc_id = 0;
        self.buy_items.clear();
        self.sell_items.clear();
        self.basket.clear();
        self.selected_index = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::enums::item::ItemType;

    fn make_item(item_id: u16, name: &str) -> Item {
        Item {
            index: 0,
            item_id,
            item_type: ItemType::Healing,
            count: 1,
            is_identified: true,
            is_damaged: false,
            refining_level: 0,
            slot: [0; 4],
            location: 0,
            wear_state: 0,
            name: name.into(),
            resource_name: None,
        }
    }

    fn sample_buy_items() -> Vec<ShopBuyItem> {
        vec![
            ShopBuyItem {
                item: make_item(501, "Red Potion"),
                price: 50,
                discount_price: 50,
            },
            ShopBuyItem {
                item: make_item(502, "Orange Potion"),
                price: 200,
                discount_price: 200,
            },
            ShopBuyItem {
                item: {
                    let mut i = make_item(1201, "Knife");
                    i.item_type = ItemType::Weapon;
                    i
                },
                price: 50000,
                discount_price: 50000,
            },
        ]
    }

    fn sample_sell_items() -> Vec<ShopSellItem> {
        vec![
            ShopSellItem {
                item: {
                    let mut i = make_item(501, "Red Potion");
                    i.index = 1;
                    i.count = 10;
                    i.resource_name = Some("빨간포션".into());
                    i
                },
                price: 25,
                overcharge_price: 25,
            },
            ShopSellItem {
                item: {
                    let mut i = make_item(1201, "Stiletto");
                    i.item_type = ItemType::Weapon;
                    i.index = 5;
                    i.count = 1;
                    i.resource_name = Some("스틸레토".into());
                    i
                },
                price: 5000,
                overcharge_price: 5500,
            },
        ]
    }

    #[test]
    fn shop_lifecycle_buy() {
        let mut shop = NpcShopData::new();
        assert!(!shop.is_open());

        shop.open_buy(100, sample_buy_items());
        assert!(shop.is_open());
        assert_eq!(shop.mode, Some(NpcShopMode::Buy));
        assert_eq!(shop.item_count(), 3);
        assert_eq!(shop.item_name(0), "Red Potion");
        assert_eq!(shop.item_price(1), 200);
        assert!(shop.needs_quantity_prompt(0));
        assert!(!shop.needs_quantity_prompt(2));

        shop.add_to_basket(0, 10);
        shop.add_to_basket(1, 5);
        assert_eq!(shop.basket.len(), 2);
        assert_eq!(shop.basket_total(), 10 * 50 + 5 * 200);

        shop.add_to_basket(0, 5);
        assert_eq!(shop.basket.len(), 2);
        assert_eq!(shop.basket[0].quantity, 15);
        assert_eq!(shop.basket_total(), 15 * 50 + 5 * 200);

        shop.add_to_basket(2, 1);
        shop.add_to_basket(2, 1);
        assert_eq!(
            shop.basket[2].quantity, 1,
            "equipment cannot be bought twice"
        );
        shop.remove_from_basket(2);

        shop.remove_from_basket(0);
        assert_eq!(shop.basket.len(), 1);
        assert_eq!(shop.basket_total(), 5 * 200);

        shop.close();
        assert!(!shop.is_open());
        assert!(shop.basket.is_empty());
    }

    #[test]
    fn base_price_differs_from_adjusted_when_skill_applies() {
        let mut shop = NpcShopData::new();
        shop.open_buy(
            100,
            vec![ShopBuyItem {
                item: make_item(501, "Red Potion"),
                price: 50,
                discount_price: 43,
            }],
        );
        assert_eq!(shop.item_base_price(0), 50);
        assert_eq!(shop.item_price(0), 43);

        shop.open_sell(200, sample_sell_items());
        assert_eq!(shop.item_base_price(1), 5000);
        assert_eq!(shop.item_price(1), 5500);
    }

    #[test]
    fn shop_lifecycle_sell() {
        let mut shop = NpcShopData::new();
        shop.open_sell(200, sample_sell_items());
        assert_eq!(shop.mode, Some(NpcShopMode::Sell));
        assert_eq!(shop.item_count(), 2);
        assert!(shop.needs_quantity_prompt(0));
        assert!(!shop.needs_quantity_prompt(1));

        shop.add_to_basket(1, 1);
        assert_eq!(shop.basket_total(), 5500);

        shop.close();
        assert!(!shop.is_open());
    }
}
