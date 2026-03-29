use eframe::egui;
use ragnarok_formats::grf::{GrfArchive, GrfFileInfo};

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
        if !is_previewable(&file.name) {
            return;
        }

        let data = match archive.read_file(&file.name) {
            Ok(d) => d,
            Err(e) => {
                self.error = Some(format!("Failed to read: {e}"));
                return;
            }
        };

        let img = match image::load_from_memory_with_format(&data, image::ImageFormat::Bmp) {
            Ok(img) => img.into_rgba8(),
            Err(e) => {
                self.error = Some(format!("Failed to decode BMP: {e}"));
                return;
            }
        };

        let (w, h) = (img.width(), img.height());
        let mut pixels = img.into_raw();
        ragnarok_formats::apply_magenta_transparency(&mut pixels);

        let color_image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels);
        self.texture = Some(ctx.load_texture(
            "bmp_preview",
            color_image,
            egui::TextureOptions::NEAREST,
        ));
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
        let scale = (available.x / orig_w as f32).min(available.y / orig_h as f32).min(1.0);
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

pub fn is_previewable(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".bmp")
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
}
