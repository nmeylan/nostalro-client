#[path = "shared/mod.rs"]
mod shared;

use ragnarok_ui_component::login_window::LoginWindow;

fn main() {
    let mut login = LoginWindow::new();
    let mut grf_init = false;
    shared::UiExampleApp::new("Login Window", 800, 600, move |ctx| {
        if ctx.ui.has_grf_textures && !grf_init {
            login.has_grf_textures = true;
            login.set_texture_sizes(ctx.texture_size);
            grf_init = true;
        }
        let _events = login.build(&mut ctx.ui);
    })
    .with_grf_textures(LoginWindow::grf_texture_paths())
    .run();
}
