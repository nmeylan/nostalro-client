#[path = "shared/mod.rs"]
mod shared;

use ragnarok_game::event::ServerInfo;
use ragnarok_ui_component::server_list_window::ServerListWindow;

fn main() {
    let servers = vec![
        ServerInfo { ip: 0x0100007F, port: 6121, name: "Loki".into(), user_count: 342 },
        ServerInfo { ip: 0x0100007F, port: 6122, name: "Iris".into(), user_count: 128 },
        ServerInfo { ip: 0x0100007F, port: 6123, name: "Fenrir".into(), user_count: 57 },
        ServerInfo { ip: 0x0100007F, port: 6124, name: "Chaos".into(), user_count: 891 },
    ];
    let mut win = ServerListWindow::new(servers);

    let mut grf_init = false;
    shared::UiExampleApp::new("Server List", 800, 600, move |ctx| {
        if ctx.ui.has_grf_textures && !grf_init {
            win.has_grf_textures = true;
            win.set_texture_sizes(ctx.texture_size);
            grf_init = true;
        }
        let _events = win.build(&mut ctx.ui);
    })
    .with_grf_textures(ServerListWindow::grf_texture_paths())
    .run();
}
