use super::histo1d::histogram1d::Histogram;
use super::histo2d::histogram2d::Histogram2D;
use crate::ui::pane::Pane;
use polars::prelude::*;
use rfd::FileDialog;
use serde_json; // or use serde_yaml for YAML serialization
use std::collections::HashMap;
use std::fs::File;

use indicatif::ProgressBar;
use indicatif::ProgressStyle;

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Histogrammer {
    pub histograms1d: Vec<Histogram>,
    pub histograms2d: Vec<Histogram2D>,
    pub tabs: HashMap<String, Vec<Pane>>,
    pub tile_map: HashMap<egui_tiles::TileId, String>, // Stores tile_id to tab_name mapping
}

impl Histogrammer {
    pub fn new() -> Self {
        Self {
            histograms1d: Vec::new(),
            histograms2d: Vec::new(),
            tabs: HashMap::new(),
            tile_map: HashMap::new(),
        }
    }

    pub fn reset(&mut self) {
        self.histograms1d.clear();
        self.histograms2d.clear();
        self.tabs.clear();
        self.tile_map.clear();
    }

    pub fn add_hist1d(&mut self, name: &str, bins: usize, range: (f64, f64)) {
        // check to see if the histogram already exists
        // if the histogram already exists with the same bins and range, reset it. Else create a new one
        if let Some(hist) = self.histograms1d.iter_mut().find(|h| h.name == name) {
            if hist.bins.len() == bins && hist.range == range {
                hist.reset();
            }
        } else {
            let hist: Histogram = Histogram::new(name, bins, range); // Create a new histogram.
            self.histograms1d.push(hist); // Store it in the vector.
        }
    }

    pub fn fill_hist1d(&mut self, name: &str, lf: &LazyFrame, column_name: &str) -> bool {
        // Find the histogram by name
        let hist = match self.histograms1d.iter_mut().find(|h| h.name == name) {
            Some(h) => h,
            None => return false,
        };

        // Define the range filter
        let hist_range = hist.range;
        let filter_expr = col(column_name)
            .gt(lit(hist_range.0))
            .and(col(column_name).lt(lit(hist_range.1)));

        // Collect the values in the column
        match lf
            .clone()
            .select([col(column_name)])
            .filter(filter_expr)
            .collect()
        {
            Ok(df) => {
                let series = df.column(column_name).unwrap();
                let values = series.f64().unwrap();

                // Initialize progress bar
                let pb = ProgressBar::new(values.len() as u64);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template(&format!(
                            "{} [{{bar:40.cyan/blue}}] {{pos}}/{{len}} ({{eta}})",
                            name
                        ))
                        .expect("Failed to create progress style")
                        .progress_chars("#>-"),
                );

                // Fill histogram
                for value in values {
                    if let Some(v) = value {
                        hist.fill(v);
                    }
                    pb.inc(1);
                }

                pb.finish();
                true
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
        if let Some(hist) = self.histograms2d.iter_mut().find(|h| h.name == name) {
            if hist.bins.x == bins.0
                && hist.bins.y == bins.1
                && hist.range.x.min == range.0 .0
                && hist.range.x.max == range.0 .1
                && hist.range.y.min == range.1 .0
                && hist.range.y.max == range.1 .1
            {
                hist.reset();
            }
        } else {
            let hist: Histogram2D = Histogram2D::new(name, bins, range); // Create a new 2D histogram.
            self.histograms2d.push(hist); // Store it in the vector.
        }
    }

    pub fn fill_hist2d(
        &mut self,
        name: &str,
        lf: &LazyFrame,
        x_column_name: &str,
        y_column_name: &str,
    ) -> bool {
        let hist = match self.histograms2d.iter_mut().find(|h| h.name == name) {
            Some(h) => h,
            None => return false, // Return false if the histogram doesn't exist.
        };

        hist.plot_settings.cuts.x_column = x_column_name.to_string();
        hist.plot_settings.cuts.y_column = y_column_name.to_string();

        // Define the range filters for both dimensions
        let hist_range = hist.range.clone();
        let filter_expr = col(x_column_name)
            .gt(lit(hist_range.x.min))
            .and(col(x_column_name).lt(lit(hist_range.x.max)))
            .and(col(y_column_name).gt(lit(hist_range.y.min)))
            .and(col(y_column_name).lt(lit(hist_range.y.max)));

        // Collect the filtered DataFrame
        match lf
            .clone()
            .select([col(x_column_name), col(y_column_name)])
            .filter(filter_expr)
            .collect()
        {
            Ok(df) => {
                let x_values = df.column(x_column_name).unwrap().f64().unwrap();
                let y_values = df.column(y_column_name).unwrap().f64().unwrap();
                let total_values = x_values.len() as f32;

                // Initialize progress bar
                let pb = ProgressBar::new(total_values as u64);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template(&format!(
                            "{} [{{bar:40.cyan/blue}}] {{pos}}/{{len}} ({{eta}})",
                            name
                        ))
                        .expect("Failed to create progress style")
                        .progress_chars("#>-"),
                );

                // Efficiently fill the 2D histogram
                for (x_value, y_value) in x_values.into_iter().zip(y_values) {
                    if let (Some(x), Some(y)) = (x_value, y_value) {
                        hist.fill(x, y);
                    }

                    pb.inc(1);
                }

                pb.finish();

                true
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
