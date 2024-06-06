use rfd::FileDialog;
use serde_json; // or use serde_yaml for YAML serialization
use std::fs::File;

use polars::prelude::*;

use super::histogram1d::Histogram;
use super::histogram2d::Histogram2D;

use crate::pane::Pane;

use std::collections::HashMap;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Histogrammer {
    pub histograms1d: Vec<Histogram>,
    pub histograms2d: Vec<Histogram2D>,

    #[serde(skip)]
    pub tabs: HashMap<String, Vec<Pane>>,
}

impl Histogrammer {
    // Creates a new instance of Histogrammer.
    pub fn new() -> Self {
        Self {
            histograms1d: Vec::new(),
            histograms2d: Vec::new(),
            tabs: HashMap::new(),
        }
    }

    // Adds a new 1D histogram to the histogram list.
    pub fn add_hist1d(&mut self, name: &str, bins: usize, range: (f64, f64)) {
        let hist: Histogram = Histogram::new(name, bins, range); // Create a new histogram.
        self.histograms1d.push(hist); // Store it in the vector.
    }

    // Fills a 1D histogram with data from a polars dataframe/column.
    pub fn fill_hist1d(&mut self, name: &str, lf: &LazyFrame, column_name: &str) -> bool {
        // find the histogram by name
        let hist: &mut Histogram = match self.histograms1d.iter_mut().find(|h| h.name == name) {
            Some(h) => h,
            None => return false, // Return false if the histogram doesn't exist.
        };

        // Attempt to collect the LazyFrame into a DataFrame
        let df_result = lf
            .clone()
            .filter(col(column_name).neq(lit(-1e6))) // Filter out the -1e6 values.
            .select([col(column_name)])
            .collect();

        // Handle the Result before proceeding
        match df_result {
            Ok(df) => {
                // Now that we have a DataFrame, we can attempt to convert it to an ndarray
                let ndarray_df_result = df.to_ndarray::<Float64Type>(IndexOrder::Fortran);

                match ndarray_df_result {
                    Ok(ndarray_df) => {
                        // You now have the ndarray and can proceed with your logic
                        let shape = ndarray_df.shape();
                        let rows = shape[0];

                        // Iterating through the ndarray and filling the histogram
                        for i in 0..rows {
                            let value = ndarray_df[[i, 0]];
                            hist.fill(value);
                        }

                        true
                    }
                    Err(e) => {
                        // Handle the error, for example, log it or return an error
                        log::error!("Failed to convert DataFrame to ndarray: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                // Handle the error, for example, log it or return an error
                log::error!("Failed to collect LazyFrame: {}", e);
                false
            }
        }
    }

    // Adds and fills a 1D histogram with data from a Polars LazyFrame.
    pub fn add_fill_hist1d(
        &mut self,
        name: &str,
        lf: &LazyFrame,
        column_name: &str,
        bins: usize,
        range: (f64, f64),
    ) {
        self.add_hist1d(name, bins, range); // Add the histogram.
        self.fill_hist1d(name, lf, column_name); // Fill it with data.
    }

    // Adds a new 2D histogram to the histogram list.
    pub fn add_hist2d(
        &mut self,
        name: &str,
        bins: (usize, usize),
        range: ((f64, f64), (f64, f64)),
    ) {
        let hist: Histogram2D = Histogram2D::new(name, bins, range); // Create a new 2D histogram.
        self.histograms2d.push(hist); // Store it in the vector.
    }

    // Fills a 2D histogram with x and y data.
    pub fn fill_hist2d(
        &mut self,
        name: &str,
        lf: &LazyFrame,
        x_column_name: &str,
        y_column_name: &str,
    ) -> bool {
        // find the histogram by name
        let hist: &mut Histogram2D = match self.histograms2d.iter_mut().find(|h| h.name == name) {
            Some(h) => h,
            None => return false, // Return false if the histogram doesn't exist.
        };

        // Attempt to collect the LazyFrame into a DataFrame
        let df_result = lf
            .clone()
            .select([col(x_column_name), col(y_column_name)])
            .filter(col(x_column_name).neq(lit(-1e6)))
            .filter(col(y_column_name).neq(lit(-1e6)))
            .collect();

        // Handle the Result before proceeding
        match df_result {
            Ok(df) => {
                // Now that we have a DataFrame, we can attempt to convert it to an ndarray
                let ndarray_df_result = df.to_ndarray::<Float64Type>(IndexOrder::Fortran);

                match ndarray_df_result {
                    Ok(ndarray_df) => {
                        // You now have the ndarray and can proceed with your logic
                        let shape = ndarray_df.shape();
                        let rows = shape[0];

                        // Iterating through the ndarray rows and filling the 2D histogram
                        for i in 0..rows {
                            let x_value = ndarray_df[[i, 0]];
                            let y_value = ndarray_df[[i, 1]];

                            hist.fill(x_value, y_value);
                        }

                        true
                    }
                    Err(e) => {
                        // Handle the error, for example, log it or return an error
                        log::error!("Failed to convert DataFrame to ndarray: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                // Handle the error, for example, log it or return an error
                log::error!("Failed to collect LazyFrame: {}", e);
                false
            }
        }
    }

    // Adds and fills a 2D histogram with data from Polars LazyFrame columns.
    pub fn add_fill_hist2d(
        &mut self,
        name: &str,
        lf: &LazyFrame,
        x_column_name: &str,
        y_column_name: &str,
        bins: (usize, usize),
        range: ((f64, f64), (f64, f64)),
    ) {
        self.add_hist2d(name, bins, range); // Add the histogram.
        self.fill_hist2d(name, lf, x_column_name, y_column_name); // Fill it with data.
    }

    // Function to save the Histogrammer as JSON using a file dialog
    pub fn _save_to_json_with_dialog(&self) -> Result<(), std::io::Error> {
        if let Some(path) = FileDialog::new()
            .set_title("Save as JSON")
            .add_filter("JSON files", &["json"])
            .save_file()
        {
            let file = File::create(path)?;
            let writer = std::io::BufWriter::new(file);
            serde_json::to_writer_pretty(writer, &self)?;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No file selected",
            ))
        }
    }

    pub fn get_histogram1d_name_list(&self) -> Vec<String> {
        self.histograms1d.iter().map(|h| h.name.clone()).collect()
    }

    pub fn get_histogram2d_name_list(&self) -> Vec<String> {
        self.histograms2d.iter().map(|h| h.name.clone()).collect()
    }

    pub fn get_histogram1d(&self, name: &str) -> Option<&Histogram> {
        self.histograms1d.iter().find(|h| h.name == name)
    }

    pub fn get_histogram2d(&self, name: &str) -> Option<&Histogram2D> {
        self.histograms2d.iter().find(|h| h.name == name)
    }

    pub fn get_histogram1d_panes(&self) -> Vec<Pane> {
        let mut panes = vec![];

        for hist in &self.histograms1d {
            panes.push(Pane::Histogram(hist.clone()));
        }

        panes
    }

    pub fn get_histogram2d_panes(&self) -> Vec<Pane> {
        let mut panes = vec![];

        for hist in &self.histograms2d {
            panes.push(Pane::Histogram2D(hist.clone()));
        }

        panes
    }

    pub fn add_tab(&mut self, tab_name: &str) {
        self.tabs.insert(tab_name.to_string(), Vec::new());
    }

    pub fn histogrammer_tree(&mut self) -> egui_tiles::Tree<Pane> {
        // Initialize the egui_tiles::Tiles which will manage the Pane layout
        let mut tiles = egui_tiles::Tiles::default();

        // Create a separate tab for 1D and 2D histograms
        let hist1d_panes: Vec<_> = self
            .get_histogram1d_panes()
            .into_iter()
            .map(|pane| tiles.insert_pane(pane))
            .collect();
        let hist2d_panes: Vec<_> = self
            .get_histogram2d_panes()
            .into_iter()
            .map(|pane| tiles.insert_pane(pane))
            .collect();

        // Insert these pane groups into a tab tile structure
        let tab1 = tiles.insert_grid_tile(hist1d_panes);
        let tab2 = tiles.insert_grid_tile(hist2d_panes);

        // Collect the tabs into a vector and create the root tab tile
        let root_tab = tiles.insert_tab_tile(vec![tab1, tab2]);

        // Construct the tree with a meaningful title and the root_tab, associating it with the tiles
        egui_tiles::Tree::new("Histogrammer", root_tab, tiles)
    }
}
