// use super::cutter::cut_handler::CutHandler;
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
    // pub cut_handler: CutHandler,
}

impl Processer {
    pub fn new() -> Self {
        Self {
            lazyframer: None,
            files: Vec::new(),
            histogrammer: Histogrammer::new(),
            // cut_handler: CutHandler::new(),
        }
    }

    fn create_lazyframe(&mut self) {
        self.lazyframer = Some(LazyFramer::new(self.files.clone()));

        // // Update CutHandler with column names from LazyFramer
        // if let Some(ref lazyframer) = self.lazyframer {
        //     let column_names = lazyframer.get_column_names();
        //     self.cut_handler.update_column_names(column_names);
        //     log::info!("Column names: {:?}", self.cut_handler.column_names.clone());
        // }
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

    // pub fn filter_lazyframe_with_cuts(&mut self) {
    //     // First, check if `self.lazyframer` is Some and get a mutable reference to it
    //     if let Some(ref mut lazyframer) = self.lazyframer {
    //         // Now you can access `lazyframer.lazyframe` because `lazyframer` is a mutable reference to `LazyFramer`
    //         if let Some(ref lazyframe) = lazyframer.lazyframe {
    //             match self.cut_handler.filter_lf_with_all_cuts(lazyframe) {
    //                 Ok(filtered_lf) => {
    //                     // Use the setter method to update the lazyframe
    //                     lazyframer.set_lazyframe(filtered_lf);
    //                     self.perform_histogrammer_from_lazyframe();
    //                 }
    //                 Err(e) => {
    //                     log::error!("Failed to filter LazyFrame with cuts: {}", e);
    //                 }
    //             }
    //         }
    //     }
    // }

    pub fn save_current_lazyframe(&mut self) {
        // First, check if `self.lazyframer` is Some and get a mutable reference to it
        // if let Some(ref mut lazyframer) = self.lazyframer {
        // Now you can access `lazyframer.lazyframe` because `lazyframer` is a mutable reference to `LazyFramer`
        // Ask user for output file path
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
        // }
    }

    pub fn calculation_ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();

        ui.horizontal(|ui| {

            if ui.button("Calculate Histograms").clicked() {
                self.calculate_histograms();
            }

            // check to see if there is a lazyframe to cut
            if self.lazyframer.is_some() {

                ui.separator();

                if ui.button("Save Lazyframe").on_hover_text("CAUTION: The collected lazyframe must fit it memory\nThis saves the current lazyframe. It is advised to filter the lazyframe with cuts.").clicked() {
                    self.save_current_lazyframe();
                }

            //     if !self.cut_handler.cuts.is_empty() {

            //         ui.separator();
            //         if ui.button("Filter with Cuts").on_hover_text("CAUTION: The collected lazyframe must fit it memory").clicked() {
            //             self.filter_lazyframe_with_cuts();
            //         }
            //     }

            // } else if !self.cut_handler.cuts.is_empty() {
            //     ui.separator();

            //     ui.label("Recalculate histograms to filter with cuts");
            }
        });

        ui.separator();
    }
}
