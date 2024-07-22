use super::histogram1d::Histogram;
use super::histogram2d::Histogram2D;
use crate::ui::pane::Pane;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use polars::prelude::*;
use rfd::FileDialog;
use serde_json; // or use serde_yaml for YAML serialization
use std::collections::HashMap;
use std::fs::File;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Histogrammer {
    pub histograms1d: Vec<Histogram>,
    pub histograms2d: Vec<Histogram2D>,
    pub tabs: HashMap<String, Vec<Pane>>,
    pub tile_map: HashMap<egui_tiles::TileId, String>, // Stores tile_id to tab_name mapping
    pub show_progress: bool,
}

impl Histogrammer {
    pub fn new() -> Self {
        Self {
            histograms1d: Vec::new(),
            histograms2d: Vec::new(),
            tabs: HashMap::new(),
            tile_map: HashMap::new(),
            show_progress: false,
        }
    }

    pub fn add_hist1d(&mut self, name: &str, bins: usize, range: (f64, f64)) {
        let hist: Histogram = Histogram::new(name, bins, range); // Create a new histogram.
        self.histograms1d.push(hist); // Store it in the vector.
    }

    pub fn fill_hist1d(&mut self, name: &str, lf: &LazyFrame, column_name: &str) -> bool {
        let hist: &mut Histogram = match self.histograms1d.iter_mut().find(|h| h.name == name) {
            Some(h) => h,
            None => return false, // Return false if the histogram doesn't exist.
        };

        let df_result = lf
            .clone()
            .filter(col(column_name).neq(lit(-1e6))) // Filter out the -1e6 values.
            .select([col(column_name)])
            .collect();

        match df_result {
            Ok(df) => {
                let ndarray_df_result = df.to_ndarray::<Float64Type>(IndexOrder::Fortran);

                match ndarray_df_result {
                    Ok(ndarray_df) => {
                        let shape = ndarray_df.shape();
                        let rows = shape[0];

                        if self.show_progress {
                            let pb = ProgressBar::new(rows as u64);
                            let style = ProgressStyle::default_bar()
                                .template(&format!(
                                    "{} [{{bar:40.cyan/blue}}] {{pos}}/{{len}} ({{eta}})",
                                    name
                                ))
                                .expect("Failed to create progress style");
                            pb.set_style(style.progress_chars("#>-"));

                            for i in 0..rows {
                                let value = ndarray_df[[i, 0]];
                                hist.fill(value);
                                pb.inc(1);
                            }
                            pb.finish_with_message("1D Histogram filling complete");
                        } else {
                            for i in 0..rows {
                                let value = ndarray_df[[i, 0]];
                                hist.fill(value);
                            }
                        }

                        true
                    }
                    Err(e) => {
                        log::error!("Failed to convert DataFrame to ndarray: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to collect LazyFrame: {}", e);
                false
            }
        }
    }

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

    pub fn add_hist2d(
        &mut self,
        name: &str,
        bins: (usize, usize),
        range: ((f64, f64), (f64, f64)),
    ) {
        let hist: Histogram2D = Histogram2D::new(name, bins, range); // Create a new 2D histogram.
        self.histograms2d.push(hist); // Store it in the vector.
    }

    pub fn fill_hist2d(
        &mut self,
        name: &str,
        lf: &LazyFrame,
        x_column_name: &str,
        y_column_name: &str,
    ) -> bool {
        let hist: &mut Histogram2D = match self.histograms2d.iter_mut().find(|h| h.name == name) {
            Some(h) => h,
            None => return false, // Return false if the histogram doesn't exist.
        };

        let df_result = lf
            .clone()
            .select([col(x_column_name), col(y_column_name)])
            .filter(col(x_column_name).neq(lit(-1e6)))
            .filter(col(y_column_name).neq(lit(-1e6)))
            .collect();

        match df_result {
            Ok(df) => {
                let ndarray_df_result = df.to_ndarray::<Float64Type>(IndexOrder::Fortran);

                match ndarray_df_result {
                    Ok(ndarray_df) => {
                        let shape = ndarray_df.shape();
                        let rows = shape[0];

                        if self.show_progress {
                            let pb = ProgressBar::new(rows as u64);
                            let style = ProgressStyle::default_bar()
                                .template(&format!(
                                    "{} [{{bar:40.cyan/blue}}] {{pos}}/{{len}} ({{eta}})",
                                    name
                                ))
                                .expect("Failed to create progress style");
                            pb.set_style(style.progress_chars("#>-"));

                            for i in 0..rows {
                                let x_value = ndarray_df[[i, 0]];
                                let y_value = ndarray_df[[i, 1]];
                                hist.fill(x_value, y_value);
                                pb.inc(1);
                            }
                            pb.finish_with_message("2D Histogram filling complete");
                        } else {
                            for i in 0..rows {
                                let x_value = ndarray_df[[i, 0]];
                                let y_value = ndarray_df[[i, 1]];
                                hist.fill(x_value, y_value);
                            }
                        }

                        true
                    }
                    Err(e) => {
                        log::error!("Failed to convert DataFrame to ndarray: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to collect LazyFrame: {}", e);
                false
            }
        }
    }

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

    pub fn get_pane(&self, name: &str) -> Option<Pane> {
        if let Some(hist) = self.get_histogram1d(name) {
            return Some(Pane::Histogram(Box::new(hist.clone())));
        }

        if let Some(hist) = self.get_histogram2d(name) {
            return Some(Pane::Histogram2D(Box::new(hist.clone())));
        }

        None
    }

    pub fn get_panes(&self, histogram_names: Vec<&str>) -> Vec<Pane> {
        let mut panes = vec![];
        for name in histogram_names {
            let pane = self.get_pane(name);
            if let Some(p) = pane {
                panes.push(p);
            } else {
                log::error!("Failed to get pane for histogram: {}", name);
            }
        }

        panes
    }

    pub fn get_histogram1d_panes(&self) -> Vec<Pane> {
        let mut panes = vec![];

        for hist in &self.histograms1d {
            panes.push(Pane::Histogram(Box::new(hist.clone())));
        }

        panes
    }

    pub fn get_histogram2d_panes(&self) -> Vec<Pane> {
        let mut panes = vec![];

        for hist in &self.histograms2d {
            panes.push(Pane::Histogram2D(Box::new(hist.clone())));
        }

        panes
    }

    pub fn add_tab(&mut self, tab_name: &str) {
        self.tabs.insert(tab_name.to_string(), Vec::new());
    }

    pub fn histogrammer_tree(&mut self) -> egui_tiles::Tree<Pane> {
        let mut tiles = egui_tiles::Tiles::default();

        let mut children = Vec::new();
        for (tab_name, panes) in &self.tabs {
            let tab_panes: Vec<_> = panes
                .iter()
                .map(|pane| tiles.insert_pane(pane.clone()))
                .collect();
            let tab_tile = tiles.insert_grid_tile(tab_panes);
            self.tile_map.insert(tab_tile, tab_name.clone());
            children.push(tab_tile);
        }

        let root_tab = tiles.insert_tab_tile(children);
        egui_tiles::Tree::new("Histogrammer", root_tab, tiles)
    }
}
