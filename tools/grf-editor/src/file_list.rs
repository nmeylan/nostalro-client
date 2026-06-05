use eframe::egui;
use ragnarok_formats::grf::GrfFileInfo;

pub fn format_size(bytes: u32) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

pub fn compute_visible_files(
    file_list: &[GrfFileInfo],
    selected_dir: &str,
    search_filter: &str,
) -> Vec<usize> {
    let filter_lower = search_filter.to_lowercase();
    file_list
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            if !selected_dir.is_empty() && !f.name.starts_with(selected_dir) {
                return false;
            }
            if !filter_lower.is_empty() && !f.name.to_lowercase().contains(&filter_lower) {
                return false;
            }
            true
        })
        .map(|(i, _)| i)
        .collect()
}

/// Returns the file index that should be selected after an arrow key press.
/// `down` selects the next visible file, otherwise the previous one. Selection
/// is clamped at both ends; with nothing selected it jumps to the first/last.
pub fn next_selection(
    visible_files: &[usize],
    selected_file: Option<usize>,
    down: bool,
) -> Option<usize> {
    if visible_files.is_empty() {
        return None;
    }
    let new_pos = match selected_file.and_then(|sel| visible_files.iter().position(|&i| i == sel)) {
        Some(pos) if down => (pos + 1).min(visible_files.len() - 1),
        Some(pos) => pos.saturating_sub(1),
        None if down => 0,
        None => visible_files.len() - 1,
    };
    Some(visible_files[new_pos])
}

pub fn show_file_list(
    ui: &mut egui::Ui,
    file_list: &[GrfFileInfo],
    visible_files: &[usize],
    selected_file: &mut Option<usize>,
) {
    let row_height = 20.0;

    let mut nav_down: Option<bool> = None;
    if ui.ctx().memory(|m| m.focused().is_none()) {
        ui.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                nav_down = Some(true);
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                nav_down = Some(false);
            }
        });
    }
    let mut scroll_to_pos: Option<usize> = None;
    if let Some(down) = nav_down {
        *selected_file = next_selection(visible_files, *selected_file, down);
        if let Some(sel) = *selected_file {
            scroll_to_pos = visible_files.iter().position(|&i| i == sel);
        }
    }

    egui::Grid::new("file_list_header")
        .num_columns(3)
        .spacing([40.0, 4.0])
        .show(ui, |ui| {
            ui.strong("Filename");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.strong("Size");
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.strong("Compressed");
            });
            ui.end_row();
        });
    ui.separator();

    let visible = visible_files.to_vec();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, visible.len(), |ui, row_range| {
            egui::Grid::new("file_list_grid")
                .num_columns(3)
                .spacing([40.0, 2.0])
                .striped(true)
                .show(ui, |ui| {
                    for row_idx in row_range {
                        let file_idx = visible[row_idx];
                        let file = &file_list[file_idx];
                        let is_selected = *selected_file == Some(file_idx);

                        let resp = ui.selectable_label(is_selected, &file.name);
                        if resp.clicked() {
                            *selected_file = Some(file_idx);
                        }
                        if scroll_to_pos == Some(row_idx) {
                            resp.scroll_to_me(None);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format_size(file.uncompressed_size));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format_size(file.compressed_size));
                        });
                        ui.end_row();
                    }
                });
        });
}

pub fn show_file_info(ui: &mut egui::Ui, file: &GrfFileInfo) {
    egui::Grid::new("file_info_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.strong("Filename:");
            ui.label(&file.name);
            ui.end_row();

            ui.strong("Size:");
            ui.label(format!(
                "{} ({} bytes)",
                format_size(file.uncompressed_size),
                file.uncompressed_size
            ));
            ui.end_row();

            ui.strong("Compressed:");
            ui.label(format!(
                "{} ({} bytes)",
                format_size(file.compressed_size),
                file.compressed_size
            ));
            ui.end_row();

            if file.uncompressed_size > 0 {
                let ratio = file.compressed_size as f64 / file.uncompressed_size as f64 * 100.0;
                ui.strong("Ratio:");
                ui.label(format!("{:.1}%", ratio));
                ui.end_row();
            }

            ui.strong("Flags:");
            ui.label(format!("0x{:02X}", file.flags));
            ui.end_row();
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(500), "500 B");
    }

    #[test]
    fn format_size_kilobytes() {
        assert_eq!(format_size(2048), "2.0 KB");
    }

    #[test]
    fn format_size_megabytes() {
        assert_eq!(format_size(1_572_864), "1.5 MB");
    }

    fn make_file_info(name: &str) -> GrfFileInfo {
        GrfFileInfo {
            name: name.to_string(),
            compressed_size: 100,
            compressed_size_aligned: 104,
            uncompressed_size: 200,
            flags: 0x01,
        }
    }

    #[test]
    fn filter_by_directory() {
        let files = vec![
            make_file_info("data/texture/a.bmp"),
            make_file_info("data/model/b.rsm"),
            make_file_info("data/texture/c.bmp"),
        ];
        let visible = compute_visible_files(&files, "data/texture/", "");
        assert_eq!(visible, vec![0, 2]);
    }

    #[test]
    fn filter_by_search() {
        let files = vec![
            make_file_info("data/texture/sky.bmp"),
            make_file_info("data/texture/ground.bmp"),
            make_file_info("data/model/tree.rsm"),
        ];
        let visible = compute_visible_files(&files, "", "sky");
        assert_eq!(visible, vec![0]);
    }

    #[test]
    fn filter_combined_dir_and_search() {
        let files = vec![
            make_file_info("data/texture/sky.bmp"),
            make_file_info("data/model/sky.rsm"),
            make_file_info("data/texture/ground.bmp"),
        ];
        let visible = compute_visible_files(&files, "data/texture/", "sky");
        assert_eq!(visible, vec![0]);
    }

    #[test]
    fn arrow_navigation_moves_clamps_and_starts() {
        let visible = vec![2, 5, 9];
        // no selection: down -> first, up -> last
        assert_eq!(next_selection(&visible, None, true), Some(2));
        assert_eq!(next_selection(&visible, None, false), Some(9));
        // moves to neighbour
        assert_eq!(next_selection(&visible, Some(5), true), Some(9));
        assert_eq!(next_selection(&visible, Some(5), false), Some(2));
        // clamps at both ends
        assert_eq!(next_selection(&visible, Some(9), true), Some(9));
        assert_eq!(next_selection(&visible, Some(2), false), Some(2));
        // selection no longer visible falls back to an edge
        assert_eq!(next_selection(&visible, Some(7), true), Some(2));
        // empty list yields nothing
        assert_eq!(next_selection(&[], Some(1), true), None);
    }

    #[test]
    fn filter_case_insensitive() {
        let files = vec![make_file_info("data/texture/SKY.BMP")];
        let visible = compute_visible_files(&files, "", "sky");
        assert_eq!(visible, vec![0]);
    }
}
