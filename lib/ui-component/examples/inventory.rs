#[path = "shared/mod.rs"]
mod shared;

use ragnarok_game::inventory::InventoryItem;
use ragnarok_ui_component::inventory_window::InventoryWindow;

fn main() {
    let mut inv = InventoryWindow::new();
    inv.inventory.toggle();
    let items = vec![
        InventoryItem { index: 0, item_id: 501, item_type: 0, count: 25, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 0, name: "Red Potion".into(), resource_name: None },
        InventoryItem { index: 1, item_id: 502, item_type: 0, count: 10, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 0, name: "Orange Potion".into(), resource_name: None },
        InventoryItem { index: 2, item_id: 601, item_type: 0, count: 5, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 0, name: "Fly Wing".into(), resource_name: None },
        InventoryItem { index: 3, item_id: 1101, item_type: 1, count: 1, is_identified: true, is_damaged: false, refining_level: 7, slot: [0; 4], location: 0, wear_state: 0, name: "Sword".into(), resource_name: None },
        InventoryItem { index: 4, item_id: 2101, item_type: 4, count: 1, is_identified: true, is_damaged: false, refining_level: 5, slot: [0; 4], location: 0, wear_state: 0, name: "Guard".into(), resource_name: None },
        InventoryItem { index: 5, item_id: 910, item_type: 3, count: 50, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 0, name: "Jellopy".into(), resource_name: None },
        InventoryItem { index: 6, item_id: 911, item_type: 3, count: 30, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 0, name: "Shell".into(), resource_name: None },
    ];
    for item in items {
        inv.inventory.add_item(item);
    }

    let mut grf_init = false;
    shared::UiExampleApp::new("Inventory", 800, 600, move |ctx| {
        if ctx.ui.has_grf_textures && !grf_init {
            if let Some(table) = ctx.item_resource_table {
                inv.inventory.resolve_resource_names(table);
            }
            inv.has_grf_textures = true;
            inv.set_texture_sizes(ctx.texture_size);
            grf_init = true;
        }
        let _events = inv.build(&mut ctx.ui);
    })
    .with_grf_textures(InventoryWindow::grf_texture_paths())
    .run();
}
