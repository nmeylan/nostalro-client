#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcShopMode {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct ShopBuyItem {
    pub item_id: u16,
    pub price: i32,
    pub discount_price: i32,
    pub item_type: u8,
    pub name: String,
    pub resource_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ShopSellItem {
    pub index: i16,
    pub price: i32,
    pub overcharge_price: i32,
    pub name: String,
    pub resource_name: Option<String>,
    pub count: i16,
}

#[derive(Debug, Clone)]
pub struct ShopCartItem {
    pub source_index: usize,
    pub quantity: i16,
}

#[derive(Debug)]
pub struct NpcShopData {
    pub mode: Option<NpcShopMode>,
    pub npc_id: u32,
    pub buy_items: Vec<ShopBuyItem>,
    pub sell_items: Vec<ShopSellItem>,
    pub cart: Vec<ShopCartItem>,
    pub selected_index: Option<usize>,
}

impl NpcShopData {
    pub fn new() -> Self {
        Self {
            mode: None,
            npc_id: 0,
            buy_items: Vec::new(),
            sell_items: Vec::new(),
            cart: Vec::new(),
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
        self.cart.clear();
        self.selected_index = None;
    }

    pub fn open_sell(&mut self, npc_id: u32, items: Vec<ShopSellItem>) {
        self.mode = Some(NpcShopMode::Sell);
        self.npc_id = npc_id;
        self.sell_items = items;
        self.buy_items.clear();
        self.cart.clear();
        self.selected_index = None;
    }

    pub fn add_to_cart(&mut self, source_index: usize, quantity: i16) {
        if let Some(existing) = self.cart.iter_mut().find(|c| c.source_index == source_index) {
            existing.quantity += quantity;
        } else {
            self.cart.push(ShopCartItem { source_index, quantity });
        }
    }

    pub fn remove_from_cart(&mut self, cart_index: usize) {
        if cart_index < self.cart.len() {
            self.cart.remove(cart_index);
        }
    }

    pub fn cart_total(&self) -> i64 {
        self.cart.iter().map(|cart_item| {
            let unit_price = match self.mode {
                Some(NpcShopMode::Buy) => {
                    self.buy_items.get(cart_item.source_index)
                        .map(|i| i.discount_price)
                        .unwrap_or(0)
                }
                Some(NpcShopMode::Sell) => {
                    self.sell_items.get(cart_item.source_index)
                        .map(|i| i.overcharge_price)
                        .unwrap_or(0)
                }
                None => 0,
            };
            unit_price as i64 * cart_item.quantity as i64
        }).sum()
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
            Some(NpcShopMode::Buy) => self.buy_items.get(index).map(|i| i.name.as_str()).unwrap_or(""),
            Some(NpcShopMode::Sell) => self.sell_items.get(index).map(|i| i.name.as_str()).unwrap_or(""),
            None => "",
        }
    }

    pub fn item_icon_path(&self, index: usize) -> Option<String> {
        match self.mode {
            Some(NpcShopMode::Buy) => self.buy_items.get(index)
                .and_then(|i| i.resource_name.as_ref())
                .map(|name| format!("data/texture/유저인터페이스/item/{name}.bmp")),
            Some(NpcShopMode::Sell) => self.sell_items.get(index)
                .and_then(|i| i.resource_name.as_ref())
                .map(|name| format!("data/texture/유저인터페이스/item/{name}.bmp")),
            None => None,
        }
    }

    pub fn item_price(&self, index: usize) -> i32 {
        match self.mode {
            Some(NpcShopMode::Buy) => self.buy_items.get(index).map(|i| i.discount_price).unwrap_or(0),
            Some(NpcShopMode::Sell) => self.sell_items.get(index).map(|i| i.overcharge_price).unwrap_or(0),
            None => 0,
        }
    }

    pub fn sell_item_count(&self, index: usize) -> i16 {
        self.sell_items.get(index).map(|i| i.count).unwrap_or(1)
    }

    pub fn sell_item_remaining(&self, index: usize) -> i16 {
        let total = self.sell_item_count(index);
        let in_cart: i16 = self.cart.iter()
            .filter(|c| c.source_index == index)
            .map(|c| c.quantity)
            .sum();
        total - in_cart
    }

    pub fn visible_sell_indices(&self) -> Vec<usize> {
        (0..self.sell_items.len())
            .filter(|&i| self.sell_item_remaining(i) > 0)
            .collect()
    }

    pub fn remove_sold_items(&mut self) {
        for cart_item in &self.cart {
            if let Some(sell_item) = self.sell_items.get_mut(cart_item.source_index) {
                sell_item.count -= cart_item.quantity;
            }
        }
        self.sell_items.retain(|i| i.count > 0);
        self.cart.clear();
        self.selected_index = None;
    }

    pub fn close(&mut self) {
        self.mode = None;
        self.npc_id = 0;
        self.buy_items.clear();
        self.sell_items.clear();
        self.cart.clear();
        self.selected_index = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_buy_items() -> Vec<ShopBuyItem> {
        vec![
            ShopBuyItem { item_id: 501, price: 50, discount_price: 50, item_type: 0, name: "Red Potion".into(), resource_name: None },
            ShopBuyItem { item_id: 502, price: 200, discount_price: 200, item_type: 0, name: "Orange Potion".into(), resource_name: None },
            ShopBuyItem { item_id: 1201, price: 50000, discount_price: 50000, item_type: 3, name: "Knife".into(), resource_name: None },
        ]
    }

    fn sample_sell_items() -> Vec<ShopSellItem> {
        vec![
            ShopSellItem { index: 1, price: 25, overcharge_price: 25, name: "Red Potion".into(), resource_name: Some("빨간포션".into()), count: 10 },
            ShopSellItem { index: 5, price: 5000, overcharge_price: 5500, name: "Stiletto".into(), resource_name: Some("스틸레토".into()), count: 1 },
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

        shop.add_to_cart(0, 10);
        shop.add_to_cart(1, 5);
        assert_eq!(shop.cart.len(), 2);
        assert_eq!(shop.cart_total(), 10 * 50 + 5 * 200);

        shop.add_to_cart(0, 5);
        assert_eq!(shop.cart.len(), 2);
        assert_eq!(shop.cart[0].quantity, 15);
        assert_eq!(shop.cart_total(), 15 * 50 + 5 * 200);

        shop.remove_from_cart(0);
        assert_eq!(shop.cart.len(), 1);
        assert_eq!(shop.cart_total(), 5 * 200);

        shop.close();
        assert!(!shop.is_open());
        assert!(shop.cart.is_empty());
    }

    #[test]
    fn shop_lifecycle_sell() {
        let mut shop = NpcShopData::new();
        shop.open_sell(200, sample_sell_items());
        assert_eq!(shop.mode, Some(NpcShopMode::Sell));
        assert_eq!(shop.item_count(), 2);

        shop.add_to_cart(1, 1);
        assert_eq!(shop.cart_total(), 5500);

        shop.close();
        assert!(!shop.is_open());
    }
}
