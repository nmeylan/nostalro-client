use crate::App;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::sprite_loader;

impl App {
    pub(crate) fn load_emotion_sprite(&mut self, grf: &GrfArchive) {
        if let Some(sprite_data) = sprite_loader::load_emotion_sprite(grf)
            && let Some(textures) = self.upload_sprite(&sprite_data)
        {
            self.game.assets.emotion_textures = Some(textures);
            self.game.assets.emotion_act = Some(sprite_data.act);
        }
    }

    pub(crate) fn load_status_overlay_sprites(&mut self, grf: &GrfArchive) {
        for overlay in ragnarok_game::ailment::AilmentOverlay::ALL {
            if let Some(sprite_data) = sprite_loader::load_status_overlay_sprite(grf, overlay)
                && let Some(textures) = self.upload_sprite(&sprite_data)
            {
                self.game
                    .assets
                    .status_overlay_sprites
                    .insert(overlay, (textures, sprite_data.act));
            }
        }
    }

    pub(crate) fn load_damage_sprites(&mut self, grf: &GrfArchive) {
        if let Some(sprite_data) = sprite_loader::load_damage_number_sprite(grf)
            && let Some(textures) = self.upload_sprite(&sprite_data)
        {
            self.game.assets.damage_number_textures = Some(textures);
            self.game.assets.damage_number_act = Some(sprite_data.act);
        }
        if let Some(sprite_data) = sprite_loader::load_damage_miss_msg_sprite(grf)
            && let Some(textures) = self.upload_sprite(&sprite_data)
        {
            self.game.assets.damage_msg_textures = Some(textures);
            self.game.assets.damage_msg_act = Some(sprite_data.act);
        }
        if let Some(sprite_data) = sprite_loader::load_rank_font_sprite(grf)
            && let Some(textures) = self.upload_sprite(&sprite_data)
        {
            self.game.assets.rank_font_textures = Some(textures);
            self.game.assets.rank_font_act = Some(sprite_data.act);
        }
        if let Some(sprite_data) = sprite_loader::load_time_font_sprite(grf)
            && let Some(textures) = self.upload_sprite(&sprite_data)
        {
            self.game.assets.time_font_textures = Some(textures);
            self.game.assets.time_font_act = Some(sprite_data.act);
        }
    }

    pub(crate) fn preload_item_icons(&mut self, icon_paths: Vec<String>) {
        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            let icon_refs: Vec<&str> = icon_paths.iter().map(|s| s.as_str()).collect();
            renderer.preload_textures(&icon_refs, grf);
        }
    }
}
