use std::fs;
use std::path::{Path, PathBuf};

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Workspacer {
    pub directory: Option<PathBuf>,
    pub files: Vec<PathBuf>,
    pub selected_files: Vec<PathBuf>,
    pub file_selecton: bool,
}

impl Workspacer {
    pub fn new() -> Self {
        Self {
            directory: None,
            files: Vec::new(),
            selected_files: Vec::new(),
            file_selecton: false,
        }
    }

    // Method for the user to select a directory
    fn select_directory(&mut self) {
        let directory = rfd::FileDialog::new().pick_folder();
        if let Some(dir) = directory {
            self.directory = Some(dir.clone());
            // After directory selection, automatically load .parquet files
            self.get_parquet_files_in_directory(&dir);
            self.validate_selected_files(); // Ensure selected_files are still valid
        }
    }

    // Helper method to load .parquet files from the selected directory
    fn get_parquet_files_in_directory(&mut self, dir: &Path) {
        self.files.clear(); // Clear any existing files

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                    self.files.push(path);
                }
            }
        }
    }

    fn refresh_files(&mut self) {
        if let Some(ref dir) = self.directory.clone() {
            self.get_parquet_files_in_directory(dir);
            self.validate_selected_files(); // Ensure selected_files are still valid
        }
    }

    // Validates that all selected_files actually exist in the files list
    fn validate_selected_files(&mut self) {
        let valid_selected_files = self
            .selected_files
            .iter()
            .filter(|selected_file| self.files.contains(selected_file))
            .cloned()
            .collect::<Vec<PathBuf>>();

        self.selected_files = valid_selected_files;
    }

    // clear the selected files
    pub fn clear_selected_files(&mut self) {
        self.selected_files.clear();
    }

    // select all files
    pub fn select_all_files(&mut self) {
        self.selected_files = self.files.clone();
    }

    // Method to get the selected directory
    fn get_directory(&self) -> Option<&PathBuf> {
        self.directory.as_ref()
    }

    pub fn select_directory_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Select Directory").clicked() {
                self.select_directory();
            }

            ui.separator();

            ui.label("Current Directory: ");

            if let Some(dir) = self.get_directory() {
                ui.label(format!("{:?}", dir));

                if ui.button("↻").clicked() {
                    self.refresh_files();
                }
            } else {
                ui.label("None");
            }
        });
    }

    pub fn file_selection_settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.file_selecton, "Show File Selection UI");

            ui.separator();

            if ui
                .button("Select All Files")
                .on_hover_text("Select all files in the directory")
                .clicked()
            {
                self.select_all_files();
            }

            ui.separator();

            if ui
                .button("Clear Selected Files")
                .on_hover_text("Clear all selected files")
                .clicked()
            {
                self.clear_selected_files();
            }
        });
    }

    pub fn file_selection_ui_side_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Workspace");

            if ui.button("↻").clicked() {
                self.refresh_files();
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for file in &self.files {
                let file_stem = file.file_stem().unwrap_or_default().to_string_lossy();
                let is_selected = self.selected_files.contains(file);

                let response = ui.selectable_label(is_selected, file_stem);

                if response.clicked() {
                    if is_selected {
                        self.selected_files.retain(|f| f != file);
                    } else {
                        self.selected_files.push(file.clone());
                    }
                }
            }
        });
    }

    pub fn file_selection_ui_in_menu(&mut self, ui: &mut egui::Ui) {
        ui.label("Parquet Files in Directory:");

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Use egui's Grid to allow side by side file display
            // currently set to 9 columns
            egui::Grid::new("file_selection_grid")
                .num_columns(5)
                .show(ui, |ui| {
                    for (index, file) in self.files.iter().enumerate() {
                        let file_stem = file.file_stem().unwrap_or_default().to_string_lossy();
                        let is_selected = self.selected_files.contains(file);

                        let response = ui.selectable_label(is_selected, file_stem);

                        if response.clicked() {
                            if is_selected {
                                self.selected_files.retain(|f| f != file);
                            } else {
                                self.selected_files.push(file.clone());
                            }
                        }

                        // After adding each file, check if it's time to end the row
                        if (index + 1) % 5 == 0 {
                            ui.end_row(); // End the current row after every 6 files
                        }
                    }
                });
        });
    }

    pub fn workspace_ui(&mut self, ui: &mut egui::Ui) {
        self.select_directory_ui(ui);
        self.file_selection_settings_ui(ui);
        self.file_selection_ui_in_menu(ui);
    }
}
