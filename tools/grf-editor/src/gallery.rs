use std::collections::HashMap;

use eframe::egui;
use ragnarok_formats::grf::{GrfArchive, GrfFileInfo};

use crate::preview;
use crate::sprite_preview::SpritePreview;

const THUMB: u32 = 96;
const CELL_W: f32 = 108.0;
const CELL_H: f32 = 124.0;
const BOX: f32 = 96.0;
/// New thumbnails rendered per frame — keeps scrolling responsive while the
/// visible cells fill in over a few frames.
const MAX_RENDERS_PER_FRAME: usize = 4;

enum Kind {
    Image(image::ImageFormat),
    Gpu,
}

fn kind(name: &str, archive: &GrfArchive) -> Option<Kind> {
    if let Some(fmt) = preview::image_format(name) {
        return Some(Kind::Image(fmt));
    }
    if preview::is_str_previewable(name)
        || preview::is_model_previewable(name)
        || preview::is_sprite_previewable(name, archive)
    {
        return Some(Kind::Gpu);
    }
    None
}

pub struct Gallery {
    pub enabled: bool,
    thumbs: HashMap<String, Option<egui::TextureHandle>>,
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            enabled: false,
            thumbs: HashMap::new(),
        }
    }
}

impl Gallery {
    pub fn clear(&mut self) {
        self.thumbs.clear();
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        archive: &GrfArchive,
        file_list: &[GrfFileInfo],
        visible_files: &[usize],
        selected_file: &mut Option<usize>,
        sprite_preview: &mut Option<SpritePreview>,
    ) {
        let cells: Vec<usize> = visible_files
            .iter()
            .copied()
            .filter(|&i| kind(&file_list[i].name, archive).is_some())
            .collect();

        if cells.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No previewable files (bmp, tga, spr, str) in this directory");
            });
            return;
        }

        let avail_w = ui.available_width();
        let cols = (avail_w / CELL_W).floor().max(1.0) as usize;
        let rows = cells.len().div_ceil(cols);

        let mut budget = MAX_RENDERS_PER_FRAME;
        let mut pending = false;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, CELL_H, rows, |ui, row_range| {
                for row in row_range {
                    ui.horizontal(|ui| {
                        for col in 0..cols {
                            let cell = row * cols + col;
                            if cell >= cells.len() {
                                break;
                            }
                            let file_idx = cells[cell];
                            let name = &file_list[file_idx].name;
                            let tex = self.thumbnail(
                                name,
                                archive,
                                sprite_preview,
                                ui.ctx(),
                                &mut budget,
                                &mut pending,
                            );
                            let is_selected = *selected_file == Some(file_idx);
                            if cell_ui(ui, name, tex.as_ref(), is_selected).clicked() {
                                *selected_file = Some(file_idx);
                            }
                        }
                    });
                }
            });

        if pending {
            ui.ctx().request_repaint();
        }
    }

    fn thumbnail(
        &mut self,
        name: &str,
        archive: &GrfArchive,
        sprite_preview: &mut Option<SpritePreview>,
        ctx: &egui::Context,
        budget: &mut usize,
        pending: &mut bool,
    ) -> Option<egui::TextureHandle> {
        if let Some(cached) = self.thumbs.get(name) {
            return cached.clone();
        }
        if *budget == 0 {
            *pending = true;
            return None;
        }
        *budget -= 1;

        let image = match kind(name, archive) {
            Some(Kind::Image(fmt)) => image_thumbnail(archive, name, fmt),
            Some(Kind::Gpu) => sprite_preview
                .as_mut()
                .and_then(|sp| sp.thumbnail(archive, name, THUMB)),
            None => None,
        };
        let handle = image.map(|img| ctx.load_texture(name, img, egui::TextureOptions::NEAREST));
        self.thumbs.insert(name.to_string(), handle.clone());
        handle
    }
}

fn image_thumbnail(
    archive: &GrfArchive,
    name: &str,
    fmt: image::ImageFormat,
) -> Option<egui::ColorImage> {
    let data = archive.read_file(name).ok()?;
    let img = image::load_from_memory_with_format(&data, fmt).ok()?.into_rgba8();
    let (w, h) = (img.width(), img.height());
    let mut raw = img.into_raw();
    ragnarok_formats::apply_magenta_transparency(&mut raw);
    let src = image::RgbaImage::from_raw(w, h, raw)?;
    let scaled = image::imageops::thumbnail(&src, THUMB, THUMB);
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [scaled.width() as usize, scaled.height() as usize],
        scaled.as_raw(),
    ))
}

fn cell_ui(
    ui: &mut egui::Ui,
    name: &str,
    tex: Option<&egui::TextureHandle>,
    is_selected: bool,
) -> egui::Response {
    ui.allocate_ui_with_layout(
        egui::vec2(CELL_W, CELL_H),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            let (rect, resp) =
                ui.allocate_exact_size(egui::vec2(BOX, BOX), egui::Sense::click());
            let painter = ui.painter();
            painter.rect_filled(rect, 2.0, egui::Color32::from_gray(30));
            match tex {
                Some(t) => {
                    let [tw, th] = t.size();
                    let scale = (BOX / tw as f32).min(BOX / th as f32);
                    let size = egui::vec2(tw as f32 * scale, th as f32 * scale);
                    let img_rect = egui::Rect::from_center_size(rect.center(), size);
                    painter.image(
                        t.id(),
                        img_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                None => {
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "…",
                        egui::FontId::proportional(20.0),
                        egui::Color32::from_gray(120),
                    );
                }
            }
            if is_selected {
                painter.rect_stroke(
                    rect,
                    2.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(90, 150, 220)),
                    egui::StrokeKind::Inside,
                );
            }

            let base = name.rsplit('/').next().unwrap_or(name);
            ui.add(egui::Label::new(egui::RichText::new(base).small()).truncate())
                .on_hover_text(name);
            resp.on_hover_text(name)
        },
    )
    .inner
}
