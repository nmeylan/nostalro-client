#[path = "shared/mod.rs"]
mod shared;

use ragnarok_ui_component::system_menu::SystemMenu;

fn main() {
    let mut menu = SystemMenu::new();
    menu.open = true;

    let mut grf_init = false;
    shared::UiExampleApp::new("System Menu", 800, 600, move |ctx| {
        if ctx.ui.has_grf_textures && !grf_init {
            menu.has_grf_textures = true;
            menu.set_texture_sizes(ctx.texture_size);
            grf_init = true;
        }
        let _events = menu.build(&mut ctx.ui, true);
    })
    .with_grf_textures(SystemMenu::grf_texture_paths())
    .run();
}
