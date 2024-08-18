use polars::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::lazyframer::LazyFramer;
use crate::cutter::cut_handler::CutHandler;

#[derive(Default, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Workspacer {
    pub directory: Option<PathBuf>,
    pub files: Vec<PathBuf>,
    pub selected_files: Vec<PathBuf>,
    pub sorting_option: SortingOption,
}

#[derive(Default, Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum SortingOption {
    #[default]
    AlphabeticalAsc,
    AlphabeticalDesc,
    SizeAsc,
    SizeDesc,
    ModifiedTimeAsc,
    ModifiedTimeDesc,
    CreationTimeAsc,
    CreationTimeDesc,
}

impl SortingOption {
    fn display_name(&self) -> &str {
        match self {
            SortingOption::AlphabeticalAsc => "A-Z",
            SortingOption::AlphabeticalDesc => "Z-A",
            SortingOption::SizeAsc => "Size ⬆",
            SortingOption::SizeDesc => "Size ⬇",
            SortingOption::ModifiedTimeAsc => "Modified Time ⬆",
            SortingOption::ModifiedTimeDesc => "Modified Time ⬇",
            SortingOption::CreationTimeAsc => "Creation Time ⬆",
            SortingOption::CreationTimeDesc => "Creation Time ⬇",
        }
    }
}

impl Workspacer {
    // combine the selected files and saveinto a single parquet file
    pub fn save_selected_files_to_single_file(
        &self,
        output_path: &PathBuf,
        scan: bool,
    ) -> Result<(), PolarsError> {
        let selected_files = &self.selected_files;
        // create a lazyframe from the selected files
        let mut lazyframer = LazyFramer::new(selected_files.clone());

        // save the lazyframe to a single file
        lazyframer.save_lazyframe(output_path, scan)
    }

    pub fn save_filtered_files_to_single_file(
        &self,
        output_path: &PathBuf,
        cut_handler: &mut CutHandler,
        scan: bool,
    ) -> Result<(), PolarsError> {
        let selected_files = &self.selected_files;
        // create a lazyframe from the selected files
        let mut lazyframer = LazyFramer::new(selected_files.clone());

        if let Some(ref mut lazyframe) = lazyframer.lazyframe {
            match cut_handler.filter_lf_with_selected_cuts(lazyframe) {
                Ok(filtered_lf) => {
                    lazyframer.lazyframe = Some(filtered_lf);
                    lazyframer.save_lazyframe(output_path, scan)
                }
                Err(e) => {
                    log::error!("Error filtering LazyFrame: {}", e);
                    Err(e) // Return the error here to propagate the error
                }
            }
        } else {
            Err(PolarsError::NoData("No LazyFrame available".into())) // Handle the case where lazyframe is None
        }
    }

    pub fn save_individually_filtered_files(
        &self,
        output_dir: &Path,
        cut_handler: &mut CutHandler,
        suffix: &str,
        scan: bool,
    ) -> Result<(), PolarsError> {
        for file in &self.selected_files {
            // Create a LazyFramer for the current file
            let mut lazyframer = LazyFramer::new(vec![file.clone()]);

            if let Some(ref mut lazyframe) = lazyframer.lazyframe {
                match cut_handler.filter_lf_with_selected_cuts(lazyframe) {
                    Ok(filtered_lf) => {
                        lazyframer.lazyframe = Some(filtered_lf);

                        // need to put suffix before .parquet
                        let file_name = file.file_name().unwrap().to_string_lossy();
                        // strip the .parquet extension
                        let file_name = &file_name[..file_name.len() - 8];

                        let new_file_name = format!("{}_{}.parquet", file_name, suffix);

                        let mut new_path = output_dir.to_path_buf();
                        new_path.push(new_file_name);

                        // Save the filtered LazyFrame to the new file
                        if let Err(e) = lazyframer.save_lazyframe(&new_path, scan) {
                            log::error!(
                                "Failed to save filtered LazyFrame to {}: {}",
                                new_path.display(),
                                e
                            );
                            return Err(e);
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "Error filtering LazyFrame for file {}: {}",
                            file.display(),
                            e
                        );
                        return Err(e);
                    }
                }
            } else {
                log::error!("No LazyFrame available for file {}", file.display());
                return Err(PolarsError::NoData("No LazyFrame available".into()));
            }
        }
        Ok(())
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
        let files = &mut self.files;
        files.clear(); // Clear any existing files

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                    files.push(path);
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
        let files = &mut self.files;
        let selected_files = &mut self.selected_files;
        selected_files.retain(|selected_file| files.contains(selected_file));
    }

    // clear the selected files
    fn clear_selected_files(&mut self) {
        self.selected_files.clear();
    }

    // select all files
    fn select_all_files(&mut self) {
        let files = self.files.clone();
        self.selected_files = files;
    }

    fn sort_files(&mut self) {
        match self.sorting_option {
            SortingOption::AlphabeticalAsc => self.alphabetize_files(false),
            SortingOption::AlphabeticalDesc => self.alphabetize_files(true),
            SortingOption::SizeAsc => self.size_sort_files(false),
            SortingOption::SizeDesc => self.size_sort_files(true),
            SortingOption::ModifiedTimeAsc => self.time_sort_files(false),
            SortingOption::ModifiedTimeDesc => self.time_sort_files(true),
            SortingOption::CreationTimeAsc => self.creation_time_sort_files(false),
            SortingOption::CreationTimeDesc => self.creation_time_sort_files(true),
        }
    }

    fn time_sort_files(&mut self, reverse: bool) {
        self.files.sort_by(|a, b| {
            let a_time = a.metadata().unwrap().modified().unwrap();
            let b_time = b.metadata().unwrap().modified().unwrap();
            if reverse {
                b_time.cmp(&a_time)
            } else {
                a_time.cmp(&b_time)
            }
        });
    }

    fn alphabetize_files(&mut self, reverse: bool) {
        if reverse {
            self.files.sort_by(|a, b| b.cmp(a));
        } else {
            self.files.sort();
        }
    }

    fn size_sort_files(&mut self, reverse: bool) {
        self.files.sort_by(|a, b| {
            let a_size = a.metadata().unwrap().len();
            let b_size = b.metadata().unwrap().len();
            if reverse {
                b_size.cmp(&a_size)
            } else {
                a_size.cmp(&b_size)
            }
        });
    }

    fn creation_time_sort_files(&mut self, reverse: bool) {
        self.files.sort_by(|a, b| {
            let a_time = a.metadata().unwrap().created().unwrap_or(SystemTime::now());
            let b_time = b.metadata().unwrap().created().unwrap_or(SystemTime::now());
            if reverse {
                b_time.cmp(&a_time)
            } else {
                a_time.cmp(&b_time)
            }
        });
    }

    // Method to get the selected directory
    fn get_directory(&self) -> Option<&PathBuf> {
        self.directory.as_ref()
    }

    fn select_directory_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let dir_name: String;
            if let Some(dir) = self.get_directory() {
                dir_name = format!("{:?}", dir);
            } else {
                dir_name = "No Directory is currently selected".to_string();
            }

            if ui
                .button("Select Directory")
                .on_hover_text(dir_name)
                .clicked()
            {
                self.select_directory();
            }

            if let Some(_dir) = self.get_directory() {
                if ui
                    .button("↻")
                    .on_hover_text("Refresh the directory")
                    .clicked()
                {
                    self.refresh_files();
                }
            }
        });
    }

    fn file_selection_settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .small_button("Select All")
                .on_hover_text("Select all files in the directory")
                .clicked()
            {
                self.select_all_files();
            }

            if ui
                .small_button("Clear")
                .on_hover_text("Clear all selected files")
                .clicked()
            {
                self.clear_selected_files();
            }
        });

        ui.horizontal(|ui| {
            let current_sorting_option = self.sorting_option.clone();
            egui::ComboBox::from_label("Sorting")
                .selected_text(current_sorting_option.display_name())
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(
                            &mut self.sorting_option,
                            SortingOption::AlphabeticalAsc,
                            SortingOption::AlphabeticalAsc.display_name(),
                        )
                        .clicked()
                    {
                        self.sort_files();
                    }
                    if ui
                        .selectable_value(
                            &mut self.sorting_option,
                            SortingOption::AlphabeticalDesc,
                            SortingOption::AlphabeticalDesc.display_name(),
                        )
                        .clicked()
                    {
                        self.sort_files();
                    }
                    if ui
                        .selectable_value(
                            &mut self.sorting_option,
                            SortingOption::SizeAsc,
                            SortingOption::SizeAsc.display_name(),
                        )
                        .clicked()
                    {
                        self.sort_files();
                    }
                    if ui
                        .selectable_value(
                            &mut self.sorting_option,
                            SortingOption::SizeDesc,
                            SortingOption::SizeDesc.display_name(),
                        )
                        .clicked()
                    {
                        self.sort_files();
                    }
                    if ui
                        .selectable_value(
                            &mut self.sorting_option,
                            SortingOption::ModifiedTimeAsc,
                            SortingOption::ModifiedTimeAsc.display_name(),
                        )
                        .clicked()
                    {
                        self.sort_files();
                    }
                    if ui
                        .selectable_value(
                            &mut self.sorting_option,
                            SortingOption::ModifiedTimeDesc,
                            SortingOption::ModifiedTimeDesc.display_name(),
                        )
                        .clicked()
                    {
                        self.sort_files();
                    }
                    if ui
                        .selectable_value(
                            &mut self.sorting_option,
                            SortingOption::CreationTimeAsc,
                            SortingOption::CreationTimeAsc.display_name(),
                        )
                        .clicked()
                    {
                        self.sort_files();
                    }
                    if ui
                        .selectable_value(
                            &mut self.sorting_option,
                            SortingOption::CreationTimeDesc,
                            SortingOption::CreationTimeDesc.display_name(),
                        )
                        .clicked()
                    {
                        self.sort_files();
                    }
                });
        });
    }

    fn file_selection_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(".parquet Files");

        let files = &mut self.files;
        let selected_files = &mut self.selected_files;

        ui.horizontal_wrapped(|ui| {
            for file in files.iter() {
                let file_stem = file.file_stem().unwrap_or_default().to_string_lossy();
                let is_selected = selected_files.contains(file);

                let response = ui.selectable_label(is_selected, file_stem);
                if response.clicked() {
                    if is_selected {
                        selected_files.retain(|f| f != file);
                    } else {
                        selected_files.push(file.clone());
                    }
                }
            }
        });
    }

    pub fn workspace_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Workspace", |ui| {
            self.select_directory_ui(ui);
            self.file_selection_settings_ui(ui);
            self.file_selection_ui(ui);
        });
    }
}
