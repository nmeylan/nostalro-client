#[path = "shared/mod.rs"]
mod shared;

use ragnarok_ui_component::npc_dialog::NpcDialog;

fn main() {
    let mut npc = NpcDialog::new();
    npc.dialog.open_text(100, "Hello adventurer!\nWelcome to Prontera.\nHow can I help you today?");
    npc.dialog.wait_for_next(100);

    let mut grf_init = false;
    shared::UiExampleApp::new("NPC Dialog", 800, 600, move |ctx| {
        if ctx.ui.has_grf_textures && !grf_init {
            npc.has_grf_textures = true;
            npc.set_texture_sizes(ctx.texture_size);
            grf_init = true;
        }
        let _events = npc.build(&mut ctx.ui);
    })
    .with_grf_textures(NpcDialog::grf_texture_paths())
    .run();
}
