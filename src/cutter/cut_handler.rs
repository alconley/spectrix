use super::egui_polygon::EditableEguiPolygon;

use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

use egui_plot::PlotUi;
use polars::prelude::*;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct CutHandler {
    pub cuts: HashMap<String, EditableEguiPolygon>,
    pub active_cut_id: Option<String>,
    pub column_names: Vec<String>,
    pub draw_flag: bool,
    pub selected_cut: Option<String>,
}

impl CutHandler {
    // Creates a new `CutHandler` instance.
    pub fn new() -> Self {
        Self {
            cuts: HashMap::new(),
            active_cut_id: None,
            column_names: Vec::new(),
            draw_flag: true,
            selected_cut: None,
        }
    }

    // Adds a new cut and makes it the active one
    pub fn add_new_cut(&mut self) {
        self.draw_flag = true;
        let new_id = format!("cut_{}", self.cuts.len() + 1);
        self.selected_cut = Some(new_id.clone());
        self.cuts.insert(
            new_id.clone(),
            EditableEguiPolygon::new(self.column_names.clone()),
        );
        self.active_cut_id = Some(new_id); // Automatically make the new cut active
    }

    // Method to update the column names
    pub fn update_column_names(&mut self, column_names: Vec<String>) {
        // Assuming `CutHandler` has a field to store column names
        // Update it with the new column names
        self.column_names = column_names;
    }

    // Method to draw the active cut
    pub fn draw_active_cut(&mut self, plot_ui: &mut PlotUi) {
        if self.draw_flag {
            if let Some(active_id) = &self.active_cut_id {
                if let Some(active_cut) = self.cuts.get_mut(active_id) {
                    active_cut.draw(plot_ui);
                }
            }
        }
    }

    pub fn filter_lf_with_all_cuts(&mut self, lf: &LazyFrame) -> Result<LazyFrame, PolarsError> {
        let mut filtered_lf = lf.clone();

        // Iterate through all cuts and apply their respective filters.
        for cut in self.cuts.values() {
            // Directly call filter_lf_with_cut on each cut.
            filtered_lf = cut.filter_lf_with_cut(&filtered_lf)?;
        }

        Ok(filtered_lf)
    }

    pub fn _filter_files_and_save_to_one_file(
        &mut self,
        file_paths: Vec<PathBuf>,
        output_path: &PathBuf,
    ) -> Result<(), PolarsError> {
        let files_arc: Arc<[PathBuf]> = Arc::from(file_paths.clone());

        let args = ScanArgsParquet::default();

        // Assuming LazyFrame::scan_parquet_files constructs a LazyFrame from the list of files
        let lf = LazyFrame::scan_parquet_files(files_arc, args)?;

        // Apply filtering logic as before, leading to a filtered LazyFrame
        let filtered_lf = self.filter_lf_with_all_cuts(&lf)?; // Placeholder for applying cuts

        // Collect the LazyFrame into a DataFrame
        let mut filtered_df = filtered_lf.collect()?;

        // Open a file in write mode at the specified output path
        let file = File::create(output_path)
            .map_err(|e| PolarsError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        // Write the filtered DataFrame to a Parquet file
        ParquetWriter::new(file)
            .set_parallel(true)
            .finish(&mut filtered_df)?;

        Ok(())
    }

    /*
    pub fn _filter_files_and_save_separately(
        &mut self,
        file_paths: Vec<PathBuf>,
        output_dir: &PathBuf,
        custom_text: &str,
    ) -> Result<(), PolarsError> {
        for file_path in file_paths.iter() {
            let args = ScanArgsParquet::default();

            let file_arc: Arc<PathBuf> = Arc::from(file_path.clone());

            // Construct a LazyFrame for each file
            let lf = LazyFrame::scan_parquet(file_arc.as_ref(), args.clone())?;

            // Apply filtering logic as before, leading to a filtered LazyFrame
            let filtered_lf = self.filter_lf_with_all_cuts(&lf)?; // Placeholder for applying cuts

            // Collect the LazyFrame into a DataFrame
            let mut filtered_df = filtered_lf.collect()?;

            // Generate a new output file name by appending custom text to the original file name
            let original_file_name = file_path.file_stem().unwrap_or(OsStr::new("default"));
            let new_file_name = format!(
                "{}_{}.parquet",
                original_file_name.to_string_lossy(),
                custom_text
            );
            let output_file_path = output_dir.join(new_file_name);

            // Open a file in write mode at the newly specified output path
            let file = File::create(&output_file_path)
                .map_err(|e| PolarsError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

            // Write the filtered DataFrame to a new Parquet file
            ParquetWriter::new(file)
                .set_parallel(true)
                .finish(&mut filtered_df)?;
        }

        Ok(())
    }
    */

    pub fn cut_handler_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Cutter: ");

            if ui.button("New Cut").clicked() {
                self.add_new_cut();
            }

            // only display the cut selection UI if there are cuts
            if !self.cuts.is_empty() {
                self.select_cut_ui(ui);
            }

            self.selected_cut_ui(ui);
        });
    }

    pub fn select_cut_ui(&mut self, ui: &mut egui::Ui) {
        // Start with a separator for visual clarity
        ui.separator();

        // Label for the combo box to indicate its purpose
        ui.label("Select Cut:");

        // Initialize a temporary variable for the selected cut ID for the combo box
        let mut current_selection = self.selected_cut.clone().unwrap_or_default();

        // Create a combo box to list all available cuts
        egui::ComboBox::from_label("")
            .selected_text(current_selection.clone())
            .show_ui(ui, |ui| {
                // Iterate over all cut IDs in the HashMap and add them as selectable items
                for cut_id in self.cuts.keys() {
                    ui.selectable_value(&mut current_selection, cut_id.clone(), cut_id);
                }
            });

        // Check if the selection has changed, and update the selected cut ID accordingly
        if current_selection != self.selected_cut.clone().unwrap_or_default() {
            self.selected_cut = Some(current_selection);
        }
    }

    pub fn selected_cut_ui(&mut self, ui: &mut egui::Ui) {
        // Display UI for the selected cut
        if let Some(selected_id) = &self.selected_cut {
            if let Some(selected_cut) = self.cuts.get_mut(selected_id) {
                selected_cut.cut_ui(ui); // Display the `cut_ui` of the selected cut

                // check box to draw/edit the cut
                ui.checkbox(&mut self.draw_flag, "Draw");

                ui.separator();

                // button to remove cut
                if ui.button("Delete").clicked() {
                    self.cuts.remove(selected_id);
                    self.selected_cut = None;
                }
            }
        }
    }
}
