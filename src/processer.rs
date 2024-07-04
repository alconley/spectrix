use super::cutter::cut_handler::CutHandler;
use super::histoer::histogram_script::add_histograms;
use super::histoer::histogrammer::Histogrammer;
use super::lazyframer::LazyFramer;

use std::path::PathBuf;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Processer {
    #[serde(skip)]
    pub lazyframer: Option<LazyFramer>,
    pub files: Vec<PathBuf>,
    pub histogrammer: Histogrammer,
    pub cut_handler: CutHandler,
}

impl Processer {
    pub fn new() -> Self {
        Self {
            lazyframer: None,
            files: Vec::new(),
            histogrammer: Histogrammer::new(),
            cut_handler: CutHandler::default(),
        }
    }

    fn create_lazyframe(&mut self) {
        self.lazyframer = Some(LazyFramer::new(self.files.clone()));
    }

    fn perform_histogrammer_from_lazyframe(&mut self) {
        if let Some(lazyframer) = &self.lazyframer {
            if let Some(lf) = &lazyframer.lazyframe {
                match add_histograms(lf.clone()) {
                    Ok(h) => {
                        self.histogrammer = h;
                    }
                    Err(e) => {
                        log::error!("Failed to create histograms: {}", e);
                    }
                }
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
    }

    pub fn calculate_histograms_with_cuts(&mut self) {
        self.create_lazyframe();
        if let Some(ref mut lazyframer) = self.lazyframer {
            if let Some(ref lazyframe) = lazyframer.lazyframe {
                match self.cut_handler.filter_lf_with_selected_cuts(lazyframe) {
                    Ok(filtered_lf) => {
                        lazyframer.set_lazyframe(filtered_lf);
                        self.perform_histogrammer_from_lazyframe();
                    }
                    Err(e) => {
                        log::error!("Failed to filter LazyFrame with cuts: {}", e);
                    }
                }
            }
        }

        self.perform_histogrammer_from_lazyframe();
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
        // if let Some(lazyframer) = &mut self.lazyframer {
        //     lazyframer.ui(ui);
        // }

        ui.separator();

        self.cut_handler.cut_ui(ui);
    }
}
