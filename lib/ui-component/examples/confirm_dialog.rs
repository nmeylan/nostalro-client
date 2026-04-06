#[path = "shared/mod.rs"]
mod shared;

use ragnarok_ui_component::confirm_dialog::ConfirmDialog;

fn main() {
    let mut dialog = ConfirmDialog::new("Are you sure you want to quit?");

    let mut grf_init = false;
    shared::UiExampleApp::new("Confirm Dialog", 800, 600, move |ctx| {
        if ctx.ui.has_grf_textures && !grf_init {
            dialog.has_grf_textures = true;
            dialog.set_texture_sizes(ctx.texture_size);
            grf_init = true;
        }
        let _result = dialog.build(&mut ctx.ui);
    })
    .with_grf_textures(ConfirmDialog::grf_texture_paths())
    .run();
}
