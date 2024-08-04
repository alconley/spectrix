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
                self.histogrammer.reset();

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

    pub fn save_current_lazyframe(&mut self) {
        if let Some(output_path) = rfd::FileDialog::new()
            .set_title("Collect Lazyframe and save the DataFrame to a single file")
            .add_filter("Parquet file", &["parquet"])
            .save_file()
        {
            if let Some(lazyframer) = &mut self.lazyframer {
                match lazyframer.save_lazyframe(&output_path) {
                    Ok(_) => println!("LazyFrame saved successfully."),
                    Err(e) => log::error!("Failed to save LazyFrame: {}", e),
                }
            } else {
                log::error!("No LazyFrame loaded to save.");
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if !self.workspacer.selected_files.is_empty() {
            // Properly clone the shared state for processing

            ui.horizontal(|ui| {
                if ui.button("Calculate Histograms").clicked() {
                    self.calculate_histograms();
                }

                if !self.cut_handler.cuts.is_empty() && ui.button("with Cuts").clicked() {
                    self.calculate_histograms_with_cuts();
                }
            });

            ui.separator();
        }

        self.workspacer.workspace_ui(ui);

        self.cut_handler.cut_ui(ui);

        if let Some(lazyframer) = &mut self.lazyframer {
            lazyframer.ui(ui);
        }
    }

    pub fn histogram_script_ui(&mut self, ui: &mut egui::Ui) {
        self.histogram_script.ui(ui);
    }
}
