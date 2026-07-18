use eframe::egui;
use ragnarok_formats::grf::{GrfArchive, GrfFileInfo};

const MAX_PREVIEW_HEIGHT: f32 = 400.0;

pub struct BmpPreview {
    cached_file_idx: Option<usize>,
    texture: Option<egui::TextureHandle>,
    dimensions: Option<(u32, u32)>,
    error: Option<String>,
}

impl Default for BmpPreview {
    fn default() -> Self {
        Self {
            cached_file_idx: None,
            texture: None,
            dimensions: None,
            error: None,
        }
    }
}

impl BmpPreview {
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        selected_file_idx: Option<usize>,
        file_list: &[GrfFileInfo],
        archive: &GrfArchive,
    ) {
        if selected_file_idx == self.cached_file_idx {
            return;
        }
        self.clear();
        self.cached_file_idx = selected_file_idx;

        let file_idx = match selected_file_idx {
            Some(i) => i,
            None => return,
        };
        let file = match file_list.get(file_idx) {
            Some(f) => f,
            None => return,
        };
        let format = match image_format(&file.name) {
            Some(f) => f,
            None => return,
        };

        let data = match archive.read_file(&file.name) {
            Ok(d) => d,
            Err(e) => {
                self.error = Some(format!("Failed to read: {e}"));
                return;
            }
        };

        let img = match image::load_from_memory_with_format(&data, format) {
            Ok(img) => img.into_rgba8(),
            Err(e) => {
                self.error = Some(format!("Failed to decode image: {e}"));
                return;
            }
        };

        let (w, h) = (img.width(), img.height());
        let mut pixels = img.into_raw();
        ragnarok_formats::apply_magenta_transparency(&mut pixels);

        let color_image =
            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels);
        self.texture =
            Some(ctx.load_texture("preview", color_image, egui::TextureOptions::NEAREST));
        self.dimensions = Some((w, h));
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        if let Some(err) = &self.error {
            ui.colored_label(egui::Color32::RED, err);
            return;
        }

        let texture = match &self.texture {
            Some(t) => t,
            None => return,
        };

        let (orig_w, orig_h) = self.dimensions.unwrap();
        let available = ui.available_size();
        let max_h = available.y.min(MAX_PREVIEW_HEIGHT);
        let scale = (available.x / orig_w as f32)
            .min(max_h / orig_h as f32)
            .min(1.0);
        let display_size = egui::vec2(orig_w as f32 * scale, orig_h as f32 * scale);

        ui.vertical_centered(|ui| {
            ui.image(egui::load::SizedTexture::new(texture.id(), display_size));
            ui.label(format!("{}x{}", orig_w, orig_h));
        });
    }

    pub fn has_preview(&self) -> bool {
        self.texture.is_some() || self.error.is_some()
    }

    fn clear(&mut self) {
        self.cached_file_idx = None;
        self.texture = None;
        self.dimensions = None;
        self.error = None;
    }
}

pub fn image_format(name: &str) -> Option<image::ImageFormat> {
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    match ext.as_str() {
        "bmp" => Some(image::ImageFormat::Bmp),
        "tga" => Some(image::ImageFormat::Tga),
        _ => None,
    }
}

pub fn is_previewable(name: &str) -> bool {
    image_format(name).is_some()
}

/// A `.spr` is animatable when its sibling `.act` also exists in the archive.
pub fn is_sprite_previewable(name: &str, archive: &GrfArchive) -> bool {
    let lower = name.to_lowercase();
    let Some(base) = lower.strip_suffix(".spr") else {
        return false;
    };
    archive.file_exists(&format!("{base}.act"))
}

/// A `.act` is animatable when its sibling `.spr` also exists in the archive.
pub fn is_act_previewable(name: &str, archive: &GrfArchive) -> bool {
    let lower = name.to_lowercase();
    let Some(base) = lower.strip_suffix(".act") else {
        return false;
    };
    archive.file_exists(&format!("{base}.spr"))
}

pub fn is_str_previewable(name: &str) -> bool {
    name.to_lowercase().ends_with(".str")
}

pub fn is_model_previewable(name: &str) -> bool {
    name.to_lowercase().ends_with(".rsm")
}

pub fn is_gr2_previewable(name: &str) -> bool {
    name.to_lowercase().ends_with(".gr2")
}

pub fn is_audio_previewable(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".wav") || lower.ends_with(".mp3")
}

/// True for any file the GPU preview can render (sprite, STR effect, or model).
pub fn is_animated_previewable(name: &str, archive: &GrfArchive) -> bool {
    is_sprite_previewable(name, archive)
        || is_act_previewable(name, archive)
        || is_str_previewable(name)
        || is_model_previewable(name)
        || is_gr2_previewable(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_previewable_bmp_extensions() {
        assert!(is_previewable("texture.bmp"));
        assert!(is_previewable("data/texture/SKY.BMP"));
        assert!(is_previewable("ground.Bmp"));
        assert!(!is_previewable("model.rsm"));
        assert!(!is_previewable("script.txt"));
        assert!(!is_previewable("bmp"));
    }

    #[test]
    fn is_previewable_tga_extensions() {
        assert!(is_previewable("ring_blue.tga"));
        assert!(is_previewable("effect/ICE.TGA"));
        assert!(is_previewable("sprite.Tga"));
        assert!(!is_previewable("tga"));
    }

    #[test]
    fn is_audio_previewable_extensions() {
        assert!(is_audio_previewable("data/wav/effect/beep.wav"));
        assert!(is_audio_previewable("BGM\\01.MP3"));
        assert!(!is_audio_previewable("texture.bmp"));
    }

    #[test]
    fn image_format_detection() {
        assert!(matches!(
            image_format("foo.bmp"),
            Some(image::ImageFormat::Bmp)
        ));
        assert!(matches!(
            image_format("foo.BMP"),
            Some(image::ImageFormat::Bmp)
        ));
        assert!(matches!(
            image_format("foo.tga"),
            Some(image::ImageFormat::Tga)
        ));
        assert!(matches!(
            image_format("foo.TGA"),
            Some(image::ImageFormat::Tga)
        ));
        assert!(image_format("foo.rsm").is_none());
        assert!(image_format("foo").is_none());
    }

    #[test]
    fn sprite_previewable_requires_sibling_act() {
        let path = std::env::temp_dir().join("grf_editor_sprite_preview_test.grf");
        let _ = std::fs::remove_file(&path);
        let mut archive = GrfArchive::create(&path).unwrap();
        archive.add_file("data/sprite/poring.spr", b"spr").unwrap();
        archive.add_file("data/sprite/poring.act", b"act").unwrap();
        archive.add_file("data/sprite/lonely.spr", b"spr").unwrap();
        archive.add_file("data/sprite/orphan.act", b"act").unwrap();

        assert!(is_sprite_previewable("data/sprite/poring.spr", &archive));
        assert!(is_sprite_previewable("DATA/SPRITE/PORING.SPR", &archive));
        assert!(!is_sprite_previewable("data/sprite/lonely.spr", &archive));
        assert!(!is_sprite_previewable("data/texture/foo.bmp", &archive));

        assert!(is_act_previewable("data/sprite/poring.act", &archive));
        assert!(is_act_previewable("DATA/SPRITE/PORING.ACT", &archive));
        assert!(!is_act_previewable("data/sprite/orphan.act", &archive));
        assert!(!is_act_previewable("data/sprite/poring.spr", &archive));

        assert!(is_str_previewable("data/texture/effect/fire.str"));
        assert!(is_str_previewable("data/texture/effect/FIRE.STR"));
        assert!(!is_str_previewable("data/texture/foo.bmp"));

        assert!(is_model_previewable("data/model/tree.rsm"));
        assert!(is_model_previewable("data/model/TREE.RSM"));
        assert!(!is_model_previewable("data/sprite/poring.spr"));

        assert!(is_gr2_previewable("data/model/emperium.gr2"));
        assert!(is_gr2_previewable("data/model/EMPERIUM.GR2"));
        assert!(!is_gr2_previewable("data/model/tree.rsm"));

        assert!(is_animated_previewable("data/sprite/poring.spr", &archive));
        assert!(is_animated_previewable("data/sprite/poring.act", &archive));
        assert!(!is_animated_previewable("data/sprite/orphan.act", &archive));
        assert!(is_animated_previewable("data/texture/effect/fire.str", &archive));
        assert!(is_animated_previewable("data/model/tree.rsm", &archive));
        assert!(is_animated_previewable("data/model/emperium.gr2", &archive));
        assert!(!is_animated_previewable("data/sprite/lonely.spr", &archive));
        assert!(!is_animated_previewable("readme.txt", &archive));

        let _ = std::fs::remove_file(&path);
    }
}
