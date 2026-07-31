use std::path::Path;

use eframe::egui;
use egui_ltreeview::{Action, TreeView, TreeViewState};
use ragnarok_audio::SoundManager;
use ragnarok_formats::grf::{GrfArchive, GrfFileInfo};

use crate::file_list;
use crate::gallery::Gallery;
use crate::preview::{self, BmpPreview};
use crate::sprite_preview::SpritePreview;
use crate::tree::{self, TreeNode};

struct LoadedGrf {
    archive: GrfArchive,
    name: String,
    file_list: Vec<GrfFileInfo>,
    tree: Vec<TreeNode>,
    tree_state: TreeViewState<u32>,
    visible_files: Vec<usize>,
    selected_dir: String,
    selected_file: Option<usize>,
    search_filter: String,
    dirty: bool,
    preview: BmpPreview,
    sprite_preview: Option<SpritePreview>,
    gallery: Gallery,
}

impl LoadedGrf {
    fn new(archive: GrfArchive) -> Self {
        let name = archive
            .path()
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string());
        let file_list = archive.file_list();
        let indexed_names: Vec<(usize, &str)> = file_list
            .iter()
            .enumerate()
            .map(|(i, f)| (i, f.name.as_str()))
            .collect();
        let tree = tree::build_tree(&indexed_names);
        let visible_files = (0..file_list.len()).collect();

        Self {
            archive,
            name,
            file_list,
            tree,
            tree_state: TreeViewState::default(),
            visible_files,
            selected_dir: String::new(),
            selected_file: None,
            search_filter: String::new(),
            dirty: false,
            preview: BmpPreview::default(),
            sprite_preview: None,
            gallery: Gallery::default(),
        }
    }

    fn refresh(&mut self) {
        self.file_list = self.archive.file_list();
        let indexed_names: Vec<(usize, &str)> = self
            .file_list
            .iter()
            .enumerate()
            .map(|(i, f)| (i, f.name.as_str()))
            .collect();
        self.tree = tree::build_tree(&indexed_names);
        self.selected_file = None;
        self.gallery.clear();
        self.update_visible_files();
    }

    fn update_visible_files(&mut self) {
        self.visible_files = file_list::compute_visible_files(
            &self.file_list,
            &self.selected_dir,
            &self.search_filter,
        );
    }
}

const DEFAULT_VOLUME: f32 = 0.8;

pub struct GrfEditorApp {
    archives: Vec<LoadedGrf>,
    active_tab: usize,
    error_msg: Option<String>,
    confirm_delete: Option<usize>,
    sound: SoundManager,
    volume: f32,
    autoplayed: Option<String>,
}

impl Default for GrfEditorApp {
    fn default() -> Self {
        Self {
            archives: Vec::new(),
            active_tab: 0,
            error_msg: None,
            confirm_delete: None,
            sound: SoundManager::new(0.0, DEFAULT_VOLUME),
            volume: DEFAULT_VOLUME,
            autoplayed: None,
        }
    }
}

impl GrfEditorApp {
    pub fn open_grf(&mut self, path: &Path) {
        match GrfArchive::open_rw(path) {
            Ok(archive) => {
                self.archives.push(LoadedGrf::new(archive));
                self.active_tab = self.archives.len() - 1;
                self.error_msg = None;
            }
            Err(e) => {
                self.error_msg = Some(format!("Failed to open {}: {e}", path.display()));
            }
        }
    }

    fn action_new(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("GRF files", &["grf"])
            .save_file()
        {
            match GrfArchive::create(&path) {
                Ok(mut archive) => {
                    if let Err(e) = archive.save() {
                        self.error_msg = Some(format!("Failed to save new GRF: {e}"));
                        return;
                    }
                    self.archives.push(LoadedGrf::new(archive));
                    self.active_tab = self.archives.len() - 1;
                    self.error_msg = None;
                }
                Err(e) => {
                    self.error_msg = Some(format!("Failed to create GRF: {e}"));
                }
            }
        }
    }

    fn action_open(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("GRF files", &["grf"])
            .pick_file()
        {
            self.open_grf(&path);
        }
    }

    fn action_add_files(&mut self) {
        if let Some(paths) = rfd::FileDialog::new().pick_files() {
            let grf = match self.archives.get_mut(self.active_tab) {
                Some(g) => g,
                None => return,
            };
            let dir_prefix = if grf.selected_dir.is_empty() {
                String::new()
            } else {
                grf.selected_dir.clone()
            };
            for path in paths {
                let basename = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let grf_path = format!("{dir_prefix}{basename}");
                match std::fs::read(&path) {
                    Ok(data) => {
                        if let Err(e) = grf.archive.add_file(&grf_path, &data) {
                            self.error_msg = Some(format!("Failed to add {basename}: {e}"));
                            return;
                        }
                    }
                    Err(e) => {
                        self.error_msg = Some(format!("Failed to read {basename}: {e}"));
                        return;
                    }
                }
            }
            grf.dirty = true;
            grf.refresh();
            self.error_msg = None;
        }
    }

    fn action_save(&mut self) {
        let grf = match self.archives.get_mut(self.active_tab) {
            Some(g) => g,
            None => return,
        };
        match grf.archive.save() {
            Ok(()) => {
                grf.dirty = false;
                self.error_msg = None;
            }
            Err(e) => {
                self.error_msg = Some(format!("Save failed: {e}"));
            }
        }
    }

    fn action_extract(&mut self) {
        let grf = match self.archives.get(self.active_tab) {
            Some(g) => g,
            None => return,
        };
        let file_idx = match grf.selected_file {
            Some(i) => i,
            None => return,
        };
        let filename = &grf.file_list[file_idx].name;

        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            let dest = dir.join(filename);
            match grf.archive.read_file(filename) {
                Ok(data) => {
                    if let Some(parent) = dest.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            self.error_msg = Some(format!("Failed to create directories: {e}"));
                            return;
                        }
                    }
                    match std::fs::write(&dest, &data) {
                        Ok(()) => {
                            self.error_msg = None;
                        }
                        Err(e) => {
                            self.error_msg = Some(format!("Failed to write file: {e}"));
                        }
                    }
                }
                Err(e) => {
                    self.error_msg = Some(format!("Failed to read from GRF: {e}"));
                }
            }
        }
    }

    fn action_remove(&mut self) {
        let grf = match self.archives.get_mut(self.active_tab) {
            Some(g) => g,
            None => return,
        };
        let file_idx = match grf.selected_file {
            Some(i) => i,
            None => return,
        };
        let filename = grf.file_list[file_idx].name.clone();
        match grf.archive.remove_file(&filename) {
            Ok(_) => {
                if let Err(e) = grf.archive.save() {
                    self.error_msg = Some(format!("Failed to save after remove: {e}"));
                    return;
                }
                grf.dirty = false;
                grf.refresh();
                self.error_msg = None;
            }
            Err(e) => {
                self.error_msg = Some(format!("Failed to remove: {e}"));
            }
        }
        self.confirm_delete = None;
    }

    fn action_repack(&mut self) {
        let grf = match self.archives.get_mut(self.active_tab) {
            Some(g) => g,
            None => return,
        };
        match grf.archive.repack() {
            Ok(()) => {
                grf.dirty = false;
                grf.refresh();
                self.error_msg = None;
            }
            Err(e) => {
                self.error_msg = Some(format!("Repack failed: {e}"));
            }
        }
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("New").clicked() {
                self.action_new();
            }
            if ui.button("Open").clicked() {
                self.action_open();
            }

            let has_grf = self.archives.get(self.active_tab).is_some();
            let is_writable = self
                .archives
                .get(self.active_tab)
                .is_some_and(|g| g.archive.is_writable());
            let is_dirty = self.archives.get(self.active_tab).is_some_and(|g| g.dirty);
            let has_selection = self
                .archives
                .get(self.active_tab)
                .is_some_and(|g| g.selected_file.is_some());

            ui.separator();

            if ui
                .add_enabled(is_writable, egui::Button::new("Add Files"))
                .clicked()
            {
                self.action_add_files();
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Extract"))
                .clicked()
            {
                self.action_extract();
            }
            if ui
                .add_enabled(has_selection && is_writable, egui::Button::new("Remove"))
                .clicked()
            {
                if let Some(grf) = self.archives.get(self.active_tab) {
                    if grf.selected_file.is_some() {
                        self.confirm_delete = grf.selected_file;
                    }
                }
            }

            ui.separator();

            if ui
                .add_enabled(is_dirty && is_writable, egui::Button::new("Save"))
                .clicked()
            {
                self.action_save();
            }
            if ui
                .add_enabled(has_grf && is_writable, egui::Button::new("Repack"))
                .clicked()
            {
                self.action_repack();
            }
        });
    }

    fn show_delete_confirmation(&mut self, ctx: &egui::Context) {
        if let Some(idx) = self.confirm_delete {
            let filename = self
                .archives
                .get(self.active_tab)
                .and_then(|g| g.file_list.get(idx))
                .map(|f| f.name.clone())
                .unwrap_or_default();

            let mut open = true;
            egui::Window::new("Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(format!("Delete \"{filename}\"?"));
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            self.action_remove();
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_delete = None;
                        }
                    });
                });
            if !open {
                self.confirm_delete = None;
            }
        }
    }

    fn show_tabs(&mut self, ui: &mut egui::Ui) {
        let mut close_tab = None;

        ui.horizontal(|ui| {
            for (i, grf) in self.archives.iter().enumerate() {
                let selected = i == self.active_tab;
                let label = if grf.dirty {
                    format!("{}*", grf.name)
                } else {
                    grf.name.clone()
                };
                let response = ui.selectable_label(selected, &label);
                if response.clicked() {
                    self.active_tab = i;
                }
                if ui.small_button("x").clicked() {
                    close_tab = Some(i);
                }
                ui.separator();
            }

            if ui.button("+").clicked() {
                self.action_open();
            }
        });

        if let Some(idx) = close_tab {
            self.archives.remove(idx);
            if self.active_tab >= self.archives.len() && !self.archives.is_empty() {
                self.active_tab = self.archives.len() - 1;
            }
        }
    }

    fn show_tree_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tree");
        ui.separator();

        let grf = match self.archives.get_mut(self.active_tab) {
            Some(g) => g,
            None => return,
        };

        let tree_nodes = grf.tree.clone();
        let (_response, actions) = TreeView::new(ui.make_persistent_id("grf_tree")).show_state(
            ui,
            &mut grf.tree_state,
            |builder| {
                for child in &tree_nodes {
                    tree::add_tree_node(child, builder);
                }
            },
        );

        for action in actions {
            if let Action::SetSelected(selected) = action {
                if let Some(&selected_id) = selected.last() {
                    if let Some(node) = tree::find_node_by_id(&tree_nodes, selected_id) {
                        if node.is_dir {
                            grf.selected_dir = node.full_path.clone();
                        } else {
                            if let Some(pos) = node.full_path.rfind('/') {
                                grf.selected_dir = format!("{}/", &node.full_path[..pos]);
                            } else {
                                grf.selected_dir.clear();
                            }
                            if let Some(file_idx) = node.file_index {
                                grf.selected_file = Some(file_idx);
                            }
                        }
                    } else {
                        grf.selected_dir.clear();
                    }
                    grf.update_visible_files();
                }
            }
        }
    }

    fn show_file_panel(&mut self, ui: &mut egui::Ui) {
        let grf = match self.archives.get_mut(self.active_tab) {
            Some(g) => g,
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label("Open a GRF file to get started");
                });
                return;
            }
        };

        ui.horizontal(|ui| {
            ui.label("Filter:");
            let changed = ui
                .add(egui::TextEdit::singleline(&mut grf.search_filter).desired_width(200.0))
                .changed();
            if changed {
                grf.update_visible_files();
            }
            ui.separator();
            ui.selectable_value(&mut grf.gallery.enabled, false, "List");
            ui.selectable_value(&mut grf.gallery.enabled, true, "Grid");
            if !grf.selected_dir.is_empty() {
                ui.separator();
                ui.label(format!("Dir: {}", grf.selected_dir));
                if ui.small_button("x").clicked() {
                    grf.selected_dir.clear();
                    grf.update_visible_files();
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{} files shown", grf.visible_files.len()));
            });
        });
        ui.separator();

        if grf.gallery.enabled {
            if grf.sprite_preview.is_none() {
                grf.sprite_preview = SpritePreview::new();
            }
            grf.gallery.show(
                ui,
                &grf.archive,
                &grf.file_list,
                &grf.visible_files,
                &mut grf.selected_file,
                &mut grf.sprite_preview,
            );
        } else {
            file_list::show_file_list(
                ui,
                &grf.file_list,
                &grf.visible_files,
                &mut grf.selected_file,
            );
        }
    }

    fn show_status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(grf) = self.archives.get(self.active_tab) {
                ui.label(format!("{} files", grf.file_list.len()));
                ui.separator();
                ui.label(grf.archive.path().display().to_string());
            } else {
                ui.label("No file open");
            }
            if let Some(err) = &self.error_msg {
                ui.separator();
                ui.colored_label(egui::Color32::RED, err);
            }
        });
    }

    fn show_file_info_panel(&mut self, ctx: &egui::Context) {
        let grf = match self.archives.get_mut(self.active_tab) {
            Some(g) => g,
            None => return,
        };
        let file_idx = match grf.selected_file {
            Some(i) => i,
            None => return,
        };
        if grf.file_list.get(file_idx).is_none() {
            return;
        }

        let name = grf.file_list[file_idx].name.clone();

        if self.autoplayed.as_deref() != Some(name.as_str()) {
            self.autoplayed = Some(name.clone());
            if preview::is_audio_previewable(&name) {
                self.sound.stop_all_sfx();
                let archive = &grf.archive;
                self.sound
                    .play_sfx(&name, 1.0, 0.0, || archive.read_file(&name).ok());
            }
        }

        if preview::is_audio_previewable(&name) {
            let file = &grf.file_list[file_idx];
            let mut play = false;
            let mut stop = false;
            let mut volume_changed = false;
            egui::TopBottomPanel::bottom("file_info")
                .resizable(true)
                .default_height(120.0)
                .show(ctx, |ui| {
                    ui.heading("File Info");
                    ui.separator();
                    file_list::show_file_info(ui, file);
                    ui.separator();
                    ui.horizontal(|ui| {
                        play = ui.button("▶ Play").clicked();
                        stop = ui.button("⏹ Stop").clicked();
                        volume_changed = ui
                            .add(egui::Slider::new(&mut self.volume, 0.0..=1.0).text("Volume"))
                            .changed();
                    });
                });
            if volume_changed {
                self.sound.set_volumes(0.0, self.volume);
            }
            if play {
                let archive = &grf.archive;
                self.sound
                    .play_sfx(&name, 1.0, 0.0, || archive.read_file(&name).ok());
            }
            if stop {
                self.sound.stop_all_sfx();
            }
            return;
        }

        let is_animated = preview::is_animated_previewable(&name, &grf.archive);

        if is_animated {
            if grf.sprite_preview.is_none() {
                grf.sprite_preview = SpritePreview::new();
            }
            let spr_path = grf.file_list[file_idx].name.clone();
            let file = &grf.file_list[file_idx];
            egui::TopBottomPanel::bottom("file_info")
                .resizable(true)
                .default_height(460.0)
                .show(ctx, |ui| {
                    ui.heading("File Info");
                    ui.separator();
                    file_list::show_file_info(ui, file);
                    ui.separator();
                    match &mut grf.sprite_preview {
                        Some(sp) => sp.show(ui, &grf.archive, &spr_path, file_idx),
                        None => {
                            ui.colored_label(
                                egui::Color32::RED,
                                "Sprite preview unavailable (no GPU adapter)",
                            );
                        }
                    }
                });
            return;
        }

        grf.preview
            .update(ctx, grf.selected_file, &grf.file_list, &grf.archive);
        let has_preview = grf.preview.has_preview();
        let default_height = if has_preview { 300.0 } else { 120.0 };
        let preview = &grf.preview;
        let file = &grf.file_list[file_idx];

        egui::TopBottomPanel::bottom("file_info")
            .resizable(true)
            .default_height(default_height)
            .show(ctx, |ui| {
                ui.heading("File Info");
                ui.separator();
                file_list::show_file_info(ui, file);
                if has_preview {
                    ui.separator();
                    preview.show(ui);
                }
            });
    }
}

impl eframe::App for GrfEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sound.tick();
        self.show_delete_confirmation(ctx);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            self.show_toolbar(ui);
        });

        if !self.archives.is_empty() {
            egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
                self.show_tabs(ui);
            });
        }

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.show_status_bar(ui);
        });

        self.show_file_info_panel(ctx);

        egui::SidePanel::left("tree_panel")
            .default_width(250.0)
            .show(ctx, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    self.show_tree_panel(ui);
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_file_panel(ui);
        });
    }
}
