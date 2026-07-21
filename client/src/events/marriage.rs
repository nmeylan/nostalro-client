use crate::App;
use models::enums::effect_id::EffectId;

/// GRF base path for an NPC cutin illustration.
pub(crate) fn cutin_texture_path(image: &str) -> String {
    format!("data/texture/유저인터페이스/illust/{image}.bmp")
}

impl App {
    pub(super) fn handle_wedding_celebration(&mut self, account_id: u32) {
        let key = self.game.world.entities.resolve_key(account_id);
        self.effect_queue.spawn_on(EffectId::Colorpaper, key);
    }

    pub(super) fn handle_divorced(&mut self, name: String) {
        self.game.character.partner_name.clear();
        self.game
            .chat_window
            .add_system(format!("You have divorced {name}."));
    }

    pub(super) fn handle_npc_cutin(&mut self, image: String, position: u8) {
        if position == 255 {
            self.game.npc_cutins = [None, None, None];
            return;
        }
        let Some(slot) = self.game.npc_cutins.get_mut(position as usize) else {
            return;
        };
        if image.is_empty() {
            *slot = None;
            return;
        }
        let path = cutin_texture_path(&image);
        *slot = Some(image);
        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            renderer.preload_textures(&[path.as_str()], grf);
        }
    }
}
