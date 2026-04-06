#[path = "shared/mod.rs"]
mod shared;

use ragnarok_ui_component::chat_window::ChatWindow;

fn main() {
    let mut chat = ChatWindow::new();
    chat.active = true;
    chat.add_chat("Welcome to Ragnarok Online!".into());
    chat.add_chat("Type /help for a list of commands.".into());
    chat.add_chat("[Swordsman]: Anyone want to party for Payon Dungeon?".into());
    chat.add_chat("[Merchant]: Selling Red Potions 50z each!".into());
    chat.add_chat("[Acolyte]: LFG Byalan Island".into());
    chat.add_chat("[Archer]: WTB Composite Bow +5".into());
    chat.add_chat("^FF0000[System]: Server maintenance in 30 minutes.".into());
    chat.add_chat("[Mage]: Trading Fire Bolt 10 for Cold Bolt 10".into());

    let mut grf_init = false;
    shared::UiExampleApp::new("Chat Window", 800, 600, move |ctx| {
        if ctx.ui.has_grf_textures && !grf_init {
            chat.has_grf_textures = true;
            grf_init = true;
        }
        let _events = chat.build(&mut ctx.ui);
    })
    .with_grf_textures(ChatWindow::grf_texture_paths())
    .run();
}
