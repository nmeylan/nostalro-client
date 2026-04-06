#[path = "shared/mod.rs"]
mod shared;

use ragnarok_game::inventory::{InventoryData, InventoryItem};
use ragnarok_ui_component::equipment_window::EquipmentWindow;

fn make_equipped(index: u16, item_id: u16, name: &str, wear_state: u16) -> InventoryItem {
    InventoryItem {
        index, item_id, item_type: 5, count: 1,
        is_identified: true, is_damaged: false, refining_level: 0,
        slot: [0; 4], location: 0, wear_state,
        name: name.into(), resource_name: None,
    }
}

fn main() {
    let mut equip = EquipmentWindow::new();
    equip.open = true;
    let mut inventory = InventoryData::new();
    // wear_state masks: HeadTop=256, Armor=16, HandRight=2, HandLeft=32, Shoes=64, Garment=4
    inventory.add_item(make_equipped(0, 1101, "Sword", 2));        // Weapon
    inventory.add_item(make_equipped(1, 2101, "Guard", 32));       // Shield
    inventory.add_item(make_equipped(2, 2301, "Chain Mail", 16));  // Armor
    inventory.add_item(make_equipped(3, 2401, "Sandals", 64));     // Shoes
    inventory.add_item(make_equipped(4, 2501, "Hood", 4));         // Garment

    let mut grf_init = false;
    shared::UiExampleApp::new("Equipment Window", 800, 600, move |ctx| {
        if ctx.ui.has_grf_textures && !grf_init {
            if let Some(table) = ctx.item_resource_table {
                inventory.resolve_resource_names(table);
            }
            equip.has_grf_textures = true;
            equip.set_texture_sizes(ctx.texture_size);
            grf_init = true;
        }
        let _events = equip.build(&mut ctx.ui, &inventory, None, None);
    })
    .with_grf_textures(EquipmentWindow::grf_texture_paths())
    .run();
}
