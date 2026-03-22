use std::path::Path;

use eframe::egui;
use egui_ltreeview::{Action, TreeView, TreeViewState};
use ragnarok_formats::grf::{GrfArchive, GrfFileInfo};

use crate::file_list;
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
}

impl LoadedGrf {
    fn new(archive: GrfArchive) -> Self {
        let name = archive
            .path()
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string());
        let file_list = archive.file_list();
        let indexed_names: Vec<(usize, &str)> =
            file_list.iter().enumerate().map(|(i, f)| (i, f.name.as_str())).collect();
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
        }
    }

    fn update_visible_files(&mut self) {
        self.visible_files = file_list::compute_visible_files(
            &self.file_list,
            &self.selected_dir,
            &self.search_filter,
        );
    }
}

pub struct GrfEditorApp {
    archives: Vec<LoadedGrf>,
    active_tab: usize,
    error_msg: Option<String>,
}

impl Default for GrfEditorApp {
    fn default() -> Self {
        Self {
            archives: Vec::new(),
            active_tab: 0,
            error_msg: None,
        }
    }
}

impl GrfEditorApp {
    pub fn open_grf(&mut self, path: &Path) {
        match GrfArchive::open(path) {
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

    fn action_open(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("GRF files", &["grf"])
            .pick_file()
        {
            self.open_grf(&path);
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

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Open").clicked() {
                self.action_open();
            }

            let has_selection = self
                .archives
                .get(self.active_tab)
                .is_some_and(|g| g.selected_file.is_some());
            if ui
                .add_enabled(has_selection, egui::Button::new("Extract"))
                .clicked()
            {
                self.action_extract();
            }
        });
    }

    fn show_tabs(&mut self, ui: &mut egui::Ui) {
        let mut close_tab = None;

        ui.horizontal(|ui| {
            for (i, grf) in self.archives.iter().enumerate() {
                let selected = i == self.active_tab;
                let response = ui.selectable_label(selected, &grf.name);
                if response.clicked() {
                    self.active_tab = i;
                }
                // Close button
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
        let (_response, actions) = TreeView::new(ui.make_persistent_id("grf_tree"))
            .show_state(ui, &mut grf.tree_state, |builder| {
                for child in &tree_nodes {
                    tree::add_tree_node(child, builder);
                }
            });

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

        file_list::show_file_list(
            ui,
            &grf.file_list,
            &grf.visible_files,
            &mut grf.selected_file,
        );
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

    fn show_file_info_panel(&self, ctx: &egui::Context) {
        let grf = match self.archives.get(self.active_tab) {
            Some(g) => g,
            None => return,
        };
        let file_idx = match grf.selected_file {
            Some(i) => i,
            None => return,
        };
        let file = match grf.file_list.get(file_idx) {
            Some(f) => f,
            None => return,
        };

        egui::TopBottomPanel::bottom("file_info")
            .resizable(true)
            .default_height(120.0)
            .show(ctx, |ui| {
                ui.heading("File Info");
                ui.separator();
                file_list::show_file_info(ui, file);
            });
    }
}

impl eframe::App for GrfEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
