use eframe::egui;
use ragnarok_formats::grf::{GrfArchive, GrfFileInfo};

const MAX_PREVIEW_HEIGHT: f32 = 400.0;
const MAX_TEXT_BYTES: usize = 512 * 1024;
const MAX_BINARY_BYTES: usize = 4 * 1024 * 1024;
const HEX_BYTES_PER_ROW: usize = 16;

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

pub struct TextPreview {
    cached_file_idx: Option<usize>,
    content: String,
    full_size: usize,
    error: Option<String>,
}

impl Default for TextPreview {
    fn default() -> Self {
        Self {
            cached_file_idx: None,
            content: String::new(),
            full_size: 0,
            error: None,
        }
    }
}

impl TextPreview {
    fn load(&mut self, file_idx: usize, data: &[u8]) {
        self.cached_file_idx = Some(file_idx);
        self.full_size = data.len();
        let shown = &data[..data.len().min(MAX_TEXT_BYTES)];
        self.content = ragnarok_formats::lua_table::decode_euc_kr(shown);
    }

    fn fail(&mut self, file_idx: usize, message: String) {
        self.cached_file_idx = Some(file_idx);
        self.error = Some(message);
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        if let Some(err) = &self.error {
            ui.colored_label(egui::Color32::RED, err);
            return;
        }

        if self.full_size > MAX_TEXT_BYTES {
            ui.colored_label(
                egui::Color32::YELLOW,
                format!(
                    "Showing first {} of {}",
                    crate::file_list::format_size(MAX_TEXT_BYTES as u32),
                    crate::file_list::format_size(self.full_size as u32)
                ),
            );
        }

        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.content.as_str())
                        .code_editor()
                        .desired_width(f32::INFINITY),
                );
            });
    }

    fn clear(&mut self) {
        self.cached_file_idx = None;
        self.content.clear();
        self.full_size = 0;
        self.error = None;
    }
}

pub struct BinaryPreview {
    cached_file_idx: Option<usize>,
    data: Vec<u8>,
    full_size: usize,
}

impl Default for BinaryPreview {
    fn default() -> Self {
        Self {
            cached_file_idx: None,
            data: Vec::new(),
            full_size: 0,
        }
    }
}

impl BinaryPreview {
    fn load(&mut self, file_idx: usize, data: Vec<u8>) {
        self.cached_file_idx = Some(file_idx);
        self.full_size = data.len();
        self.data = data;
        self.data.truncate(MAX_BINARY_BYTES);
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        if self.full_size > MAX_BINARY_BYTES {
            ui.colored_label(
                egui::Color32::YELLOW,
                format!(
                    "Showing first {} of {}",
                    crate::file_list::format_size(MAX_BINARY_BYTES as u32),
                    crate::file_list::format_size(self.full_size as u32)
                ),
            );
        }

        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        let rows = self.data.len().div_ceil(HEX_BYTES_PER_ROW);
        egui::ScrollArea::both().auto_shrink([false; 2]).show_rows(
            ui,
            row_height,
            rows,
            |ui, range| {
                for row in range {
                    ui.monospace(hex_dump_row(&self.data, row));
                }
            },
        );
    }

    pub fn has_preview(&self) -> bool {
        self.cached_file_idx.is_some()
    }

    fn clear(&mut self) {
        self.cached_file_idx = None;
        self.data.clear();
        self.full_size = 0;
    }
}

/// Loads `file_idx` into whichever preview suits its content: compiled Lua goes
/// to the hex dump, everything else is decoded as text.
pub fn update_text_preview(
    text: &mut TextPreview,
    binary: &mut BinaryPreview,
    file_idx: usize,
    file_list: &[GrfFileInfo],
    archive: &GrfArchive,
) {
    if text.cached_file_idx == Some(file_idx) || binary.cached_file_idx == Some(file_idx) {
        return;
    }
    text.clear();
    binary.clear();

    let Some(file) = file_list.get(file_idx) else {
        return;
    };
    let data = match archive.read_file(&file.name) {
        Ok(d) => d,
        Err(e) => {
            text.fail(file_idx, format!("Failed to read: {e}"));
            return;
        }
    };

    if ragnarok_formats::lub::is_compiled_chunk(&data) {
        binary.load(file_idx, data);
    } else {
        text.load(file_idx, &data);
    }
}

fn hex_dump_row(data: &[u8], row: usize) -> String {
    let offset = row * HEX_BYTES_PER_ROW;
    let end = (offset + HEX_BYTES_PER_ROW).min(data.len());
    let chunk = &data[offset..end];

    let mut line = format!("{offset:08X}  ");
    for i in 0..HEX_BYTES_PER_ROW {
        match chunk.get(i) {
            Some(b) => line.push_str(&format!("{b:02X} ")),
            None => line.push_str("   "),
        }
        if i == HEX_BYTES_PER_ROW / 2 - 1 {
            line.push(' ');
        }
    }

    line.push('|');
    for &b in chunk {
        line.push(if (0x20..0x7f).contains(&b) {
            b as char
        } else {
            '.'
        });
    }
    line.push('|');
    line
}

pub fn is_text_previewable(name: &str) -> bool {
    let Some((_, ext)) = name.rsplit_once('.') else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "txt" | "xml" | "lua" | "lub" | "ini" | "log"
    )
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
    fn text_previewable_extensions() {
        assert!(is_text_previewable("data/idnum2itemdesctable.txt"));
        assert!(is_text_previewable("data/book/BOOK01.XML"));
        assert!(is_text_previewable(
            "data/lua files/datainfo/accessoryid.lua"
        ));
        assert!(is_text_previewable(
            "data/lua files/datainfo/jobidentity.lub"
        ));
        assert!(is_text_previewable("data/mp3nametable.ini"));
        assert!(is_text_previewable("data/error.log"));
        assert!(!is_text_previewable("data/texture/foo.bmp"));
        assert!(!is_text_previewable("readme"));
    }

    #[test]
    fn compiled_lub_goes_to_the_hex_dump() {
        let path = std::env::temp_dir().join("grf_editor_text_preview_test.grf");
        let _ = std::fs::remove_file(&path);
        let mut archive = GrfArchive::create(&path).unwrap();
        archive
            .add_file("data/source.lub", b"main = { 1, 2 }\n")
            .unwrap();
        archive
            .add_file("data/compiled.lub", b"\x1bLua\x51\x00\x01\x04rest")
            .unwrap();
        let file_list = archive.file_list();
        let index_of = |name: &str| file_list.iter().position(|f| f.name == name).unwrap();

        let mut text = TextPreview::default();
        let mut binary = BinaryPreview::default();

        update_text_preview(
            &mut text,
            &mut binary,
            index_of("data/source.lub"),
            &file_list,
            &archive,
        );
        assert!(!binary.has_preview());
        assert_eq!(text.content, "main = { 1, 2 }\n");

        update_text_preview(
            &mut text,
            &mut binary,
            index_of("data/compiled.lub"),
            &file_list,
            &archive,
        );
        assert!(binary.has_preview());
        assert!(text.content.is_empty());
        assert_eq!(
            hex_dump_row(&binary.data, 0),
            "00000000  1B 4C 75 61 51 00 01 04  72 65 73 74             |.LuaQ...rest|"
        );

        let _ = std::fs::remove_file(&path);
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
        assert!(is_animated_previewable(
            "data/texture/effect/fire.str",
            &archive
        ));
        assert!(is_animated_previewable("data/model/tree.rsm", &archive));
        assert!(is_animated_previewable("data/model/emperium.gr2", &archive));
        assert!(!is_animated_previewable("data/sprite/lonely.spr", &archive));
        assert!(!is_animated_previewable("readme.txt", &archive));

        let _ = std::fs::remove_file(&path);
    }
}
