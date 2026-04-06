#[path = "shared/mod.rs"]
mod shared;

use ragnarok_game::npc_shop::{ShopBuyItem, ShopSellItem};
use ragnarok_ui_component::npc_shop::NpcShop;

fn main() {
    let buy_items = vec![
        ShopBuyItem { item_id: 501, price: 50, discount_price: 50, item_type: 0, name: "Red Potion".into(), resource_name: None },
        ShopBuyItem { item_id: 502, price: 200, discount_price: 200, item_type: 0, name: "Orange Potion".into(), resource_name: None },
        ShopBuyItem { item_id: 503, price: 550, discount_price: 550, item_type: 0, name: "Yellow Potion".into(), resource_name: None },
        ShopBuyItem { item_id: 504, price: 1200, discount_price: 1200, item_type: 0, name: "White Potion".into(), resource_name: None },
        ShopBuyItem { item_id: 505, price: 5000, discount_price: 5000, item_type: 0, name: "Blue Potion".into(), resource_name: None },
        ShopBuyItem { item_id: 506, price: 10, discount_price: 10, item_type: 0, name: "Green Potion".into(), resource_name: None },
        ShopBuyItem { item_id: 601, price: 40, discount_price: 40, item_type: 0, name: "Fly Wing".into(), resource_name: None },
        ShopBuyItem { item_id: 602, price: 175, discount_price: 175, item_type: 0, name: "Butterfly Wing".into(), resource_name: None },
    ];

    let sell_items = vec![
        ShopSellItem { index: 0, price: 25, overcharge_price: 25, name: "Red Potion".into(), resource_name: None, count: 30 },
        ShopSellItem { index: 1, price: 100, overcharge_price: 110, name: "Orange Potion".into(), resource_name: None, count: 12 },
        ShopSellItem { index: 2, price: 275, overcharge_price: 300, name: "Yellow Potion".into(), resource_name: None, count: 5 },
        ShopSellItem { index: 3, price: 5000, overcharge_price: 5500, name: "Stiletto".into(), resource_name: None, count: 1 },
        ShopSellItem { index: 4, price: 20, overcharge_price: 22, name: "Fly Wing".into(), resource_name: None, count: 47 },
        ShopSellItem { index: 5, price: 87, overcharge_price: 95, name: "Butterfly Wing".into(), resource_name: None, count: 8 },
    ];

    let mut shop = NpcShop::new();
    shop.shop.open_buy(100, buy_items.clone());

    let mut grf_init = false;
    let mut is_sell_mode = false;
    shared::UiExampleApp::new("NPC Shop (Tab to toggle Buy/Sell)", 800, 600, move |ctx| {
        if ctx.ui.has_grf_textures && !grf_init {
            if let Some(table) = ctx.item_resource_table {
                shop.shop.resolve_resource_names(table);
            }
            shop.has_grf_textures = true;
            shop.set_texture_sizes(ctx.texture_size);
            grf_init = true;
        }

        // Toggle buy/sell mode on Tab
        if ctx.ui.ctx.key_tab {
            is_sell_mode = !is_sell_mode;
            if is_sell_mode {
                shop.shop.open_sell(100, sell_items.clone());
            } else {
                shop.shop.open_buy(100, buy_items.clone());
                if let Some(table) = ctx.item_resource_table {
                    shop.shop.resolve_resource_names(table);
                }
            }
        }

        let _events = shop.build(&mut ctx.ui);
    })
    .with_grf_textures(NpcShop::grf_texture_paths())
    .run();
}
