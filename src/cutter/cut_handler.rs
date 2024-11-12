use super::cuts::Cut;
use crate::histoer::histogrammer::Histogrammer;
use polars::prelude::*;

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct CutHandler {
    pub cuts: Vec<Cut>,
}

impl CutHandler {
    // get a cut with a file dialog
    pub fn get_cut(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file_path) = rfd::FileDialog::new()
            .set_file_name("cut.json") // Suggest a default file name for convenience
            .add_filter("JSON Files", &["json"]) // Filter for json files
            .pick_file()
        {
            let file = File::open(file_path)?;
            let reader = BufReader::new(file);
            let cut: Cut = serde_json::from_reader(reader)?;
            self.cuts.push(cut);
        }
        Ok(())
    }

    pub fn cut_ui(&mut self, ui: &mut egui::Ui, histogrammer: &mut Histogrammer) {
        ui.collapsing("Cuts", |ui| {
            ui.horizontal(|ui| {
                if ui.button("Get Cut").clicked() {
                    if let Err(e) = self.get_cut() {
                        log::error!("Error loading cut: {:?}", e);
                    }
                }

                // Add a button to retrieve active cuts from histograms
                if ui.button("Retrieve Active Cuts").clicked() {
                    histogrammer.retrieve_active_cuts(self);
                }
            });

            if self.cuts.is_empty() {
                ui.label("No cuts loaded");
            } else {
                egui::Grid::new("cuts")
                    .striped(true)
                    .num_columns(6)
                    .show(ui, |ui| {
                        ui.label("Cuts");
                        ui.label("X Column\t\t\t\t\t");
                        ui.label("Y Column\t\t\t\t\t");
                        ui.label("Polygon");
                        ui.label("Active");
                        ui.end_row();

                        let mut index_to_remove = None;
                        for (index, cut) in self.cuts.iter_mut().enumerate() {
                            ui.label(format!("Cut {}", index));

                            cut.ui(ui);

                            ui.horizontal(|ui| {
                                if ui.button("🗙").clicked() {
                                    index_to_remove = Some(index);
                                }
                            });

                            ui.end_row();
                        }

                        if let Some(index) = index_to_remove {
                            self.cuts.remove(index);
                        }
                    });

                // add button to remove all
                if ui.button("Remove All").clicked() {
                    self.cuts.clear();
                }
            }
        });
    }

    pub fn filter_lf_with_selected_cuts(
        &mut self,
        lf: &LazyFrame,
    ) -> Result<LazyFrame, PolarsError> {
        let mut filtered_lf = lf.clone();

        // Iterate through all cuts and apply their respective filters.
        for cut in &mut self.cuts {
            filtered_lf = cut.filter_lf_with_cut(&filtered_lf)?;
        }

        Ok(filtered_lf)
    }

    pub fn filter_files_and_save_to_one_file(
        &mut self,
        file_paths: Vec<PathBuf>,
        output_path: &PathBuf,
    ) -> Result<(), PolarsError> {
        let files_arc: Arc<[PathBuf]> = Arc::from(file_paths.clone());

        let args = ScanArgsParquet::default();

        // Assuming LazyFrame::scan_parquet_files constructs a LazyFrame from the list of files
        let lf = LazyFrame::scan_parquet_files(files_arc, args)?;

        // Apply filtering logic as before, leading to a filtered LazyFrame
        let filtered_lf = self.filter_lf_with_selected_cuts(&lf)?; // Placeholder for applying cuts

        // Collect the LazyFrame into a DataFrame
        let mut filtered_df = filtered_lf.collect()?;

        // Open a file in write mode at the specified output path
        let file = File::create(output_path).map_err(|e| PolarsError::IO {
            error: Arc::new(e),
            msg: None,
        })?;

        // Write the filtered DataFrame to a Parquet file
        ParquetWriter::new(file)
            .set_parallel(true)
            .finish(&mut filtered_df)?;

        Ok(())
    }
}
