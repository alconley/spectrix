use super::lazyframer::LazyFramer;
use super::workspacer::Workspacer;
use crate::cutter::cut_handler::CutHandler;
use crate::histoer::histogrammer::Histogrammer;
use crate::histogram_scripter::histogram_script::HistogramScript;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Processer {
    pub workspacer: Workspacer,
    #[serde(skip)]
    pub lazyframer: Option<LazyFramer>,
    pub cut_handler: CutHandler,
    pub histogrammer: Histogrammer,
    #[serde(skip)]
    pub is_tree_ready: bool,
    pub histogram_script: HistogramScript,
    pub save_with_scanning: bool,
    pub suffix: String,
}

impl Processer {
    pub fn new() -> Self {
        Self {
            workspacer: Workspacer::default(),
            lazyframer: None,
            cut_handler: CutHandler::default(),
            histogrammer: Histogrammer::new(),
            is_tree_ready: false,
            histogram_script: HistogramScript::new(),
            save_with_scanning: true,
            suffix: "filtered".to_string(),
        }
    }

    pub fn reset(&mut self) {
        self.lazyframer = None;
        self.histogrammer = Histogrammer::new();
        self.is_tree_ready = false;
    }

    fn create_lazyframe(&mut self) {
        self.lazyframer = Some(LazyFramer::new(self.workspacer.selected_files.clone()));
    }

    fn perform_histogrammer_from_lazyframe(&mut self) {
        if let Some(lazyframer) = &self.lazyframer {
            if let Some(lf) = &lazyframer.lazyframe {
                // self.histogrammer.reset();

                self.histogram_script
                    .add_histograms(&mut self.histogrammer, lf.clone());
            } else {
                log::error!("LazyFrame is not loaded");
            }
        } else {
            log::error!("LazyFramer is not initialized");
        }
    }

    pub fn calculate_histograms(&mut self) {
        self.create_lazyframe();
        self.perform_histogrammer_from_lazyframe();
        self.is_tree_ready = true;
    }

    pub fn calculate_histograms_with_cuts(&mut self) {
        self.create_lazyframe();
        if let Some(ref mut lazyframer) = self.lazyframer {
            if let Some(ref lazyframe) = lazyframer.lazyframe {
                match self.cut_handler.filter_lf_with_selected_cuts(lazyframe) {
                    Ok(filtered_lf) => {
                        lazyframer.set_lazyframe(filtered_lf);
                        self.perform_histogrammer_from_lazyframe();
                        self.is_tree_ready = true;
                    }
                    Err(e) => {
                        log::error!("Failed to filter LazyFrame with cuts: {}", e);
                    }
                }
            }
        }
    }

    // save the selected files to a single file
    pub fn save_selected_files_to_single_file(&mut self) {
        let scan = self.save_with_scanning;
        if let Some(output_path) = rfd::FileDialog::new()
            .set_title("Save the selected files to a single file")
            .add_filter("Parquet file", &["parquet"])
            .save_file()
        {
            match self
                .workspacer
                .save_selected_files_to_single_file(&output_path, scan)
            {
                Ok(_) => println!("Selected files saved successfully."),
                Err(e) => log::error!("Failed to save selected files: {}", e),
            }
        }
    }

    // filter the files with the selected cuts and save to a single file
    pub fn save_filtered_files_to_single_file(&mut self) {
        let scan = self.save_with_scanning;

        if let Some(output_path) = rfd::FileDialog::new()
            .set_title("Filter the files with the selected cuts and save to a single file")
            .add_filter("Parquet file", &["parquet"])
            .save_file()
        {
            match self.workspacer.save_filtered_files_to_single_file(
                &output_path,
                &mut self.cut_handler,
                scan,
            ) {
                Ok(_) => println!("Filtered files saved successfully."),
                Err(e) => log::error!("Failed to save filtered files: {}", e),
            }
        }
    }

    pub fn save_filtered_files_individually(&mut self, suffix: &str) {
        let scan = self.save_with_scanning;

        if let Some(output_dir) = rfd::FileDialog::new()
            .set_title("Select Output Directory to Save Filtered Files Individually")
            .pick_folder()
        {
            if !suffix.is_empty() {
                match self.workspacer.save_individually_filtered_files(
                    &output_dir,
                    &mut self.cut_handler,
                    suffix,
                    scan,
                ) {
                    Ok(_) => println!("Filtered files saved individually."),
                    Err(e) => log::error!("Failed to save filtered files individually: {}", e),
                }
            } else {
                log::error!("No suffix provided, operation canceled.");
            }
        } else {
            log::error!("No output directory selected, operation canceled.");
        }
    }

    pub fn saving_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Parquet Writer", |ui| {
            ui.checkbox(&mut self.save_with_scanning, "Save with Scanning")
                .on_hover_text(
                    "This can save files that are larger than memory at the cost of being slower.",
                );

            egui::Grid::new("parquet_writer_grid")
                .striped(true)
                .show(ui, |ui| {
                    ui.label("");
                    ui.label("Single File");
                    ui.label("Individually");
                    ui.label("Suffix     ");
                    ui.end_row();

                    ui.label("Non-Filtered");

                    if ui
                        .add_enabled(
                            self.workspacer.selected_files.len() > 1,
                            egui::Button::new("Save"),
                        )
                        .clicked()
                    {
                        self.save_selected_files_to_single_file();
                    }

                    ui.end_row();

                    ui.label("Cut Filtered");

                    if ui
                        .add_enabled(
                            self.cut_handler.cuts_are_selected()
                                && !self.workspacer.selected_files.is_empty(),
                            egui::Button::new("Save"),
                        )
                        .on_disabled_hover_text("No cuts selected.")
                        .clicked()
                    {
                        self.save_filtered_files_to_single_file();
                    }

                    if ui
                        .add_enabled(
                            self.cut_handler.cuts_are_selected()
                                && !self.workspacer.selected_files.is_empty(),
                            egui::Button::new("Save"),
                        )
                        .on_disabled_hover_text("No cuts selected.")
                        .clicked()
                    {
                        self.save_filtered_files_individually(&self.suffix.clone());
                    }

                    ui.text_edit_singleline(&mut self.suffix);

                    ui.end_row();
                });
        });
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !self.workspacer.selected_files.is_empty(),
                    egui::Button::new("Calculate Histograms"),
                )
                .on_disabled_hover_text("No files selected.")
                .clicked()
            {
                self.calculate_histograms();
            }

            if ui
                .add_enabled(
                    !self.workspacer.selected_files.is_empty()
                        && self.cut_handler.cuts_are_selected(),
                    egui::Button::new("with Cuts"),
                )
                .on_disabled_hover_text("No files selected or cuts selected.")
                .clicked()
            {
                self.calculate_histograms_with_cuts();
            }
        });

        ui.separator();

        self.workspacer.workspace_ui(ui);

        ui.separator();

        self.cut_handler.cut_ui(ui);

        ui.separator();

        self.saving_ui(ui);

        ui.separator();

        if let Some(lazyframer) = &mut self.lazyframer {
            lazyframer.ui(ui);

            ui.separator();
        }
    }

    pub fn histogram_script_ui(&mut self, ui: &mut egui::Ui) {
        self.histogram_script.ui(ui);
    }
}
