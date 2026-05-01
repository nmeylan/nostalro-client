use crate::App;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::sprite_loader;
use ragnarok_renderer::upload_sprite_textures;

impl App {
    pub(crate) fn load_emotion_sprite(&mut self, grf: &GrfArchive) {
        let renderer = match &self.renderer {
            Some(r) => r,
            None => return,
        };
        if let Some(sprite_data) = sprite_loader::load_emotion_sprite(grf) {
            let textures = upload_sprite_textures(
                &sprite_data.images,
                sprite_data.indexed_count,
                &renderer.device.device,
                &renderer.device.queue,
                &renderer.texture_cache.bind_group_layout,
            );
            self.game.emotion_textures = Some(textures);
            self.game.emotion_act = Some(sprite_data.act);
        }
    }

    pub(crate) fn load_damage_sprites(&mut self, grf: &GrfArchive) {
        let renderer = match &self.renderer {
            Some(r) => r,
            None => return,
        };
        if let Some(sprite_data) = sprite_loader::load_damage_number_sprite(grf) {
            let textures = upload_sprite_textures(
                &sprite_data.images,
                sprite_data.indexed_count,
                &renderer.device.device,
                &renderer.device.queue,
                &renderer.texture_cache.bind_group_layout,
            );
            self.game.damage_number_textures = Some(textures);
            self.game.damage_number_act = Some(sprite_data.act);
        }
        if let Some(sprite_data) = sprite_loader::load_damage_miss_msg_sprite(grf) {
            let textures = upload_sprite_textures(
                &sprite_data.images,
                sprite_data.indexed_count,
                &renderer.device.device,
                &renderer.device.queue,
                &renderer.texture_cache.bind_group_layout,
            );
            self.game.damage_msg_textures = Some(textures);
            self.game.damage_msg_act = Some(sprite_data.act);
        }
    }

    pub(crate) fn preload_item_icons(&mut self, icon_paths: Vec<String>) {
        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            let icon_refs: Vec<&str> = icon_paths.iter().map(|s| s.as_str()).collect();
            renderer.preload_textures(&icon_refs, grf);
        }
    }
}
