use eframe::egui::{Color32, Stroke};
use std::collections::HashMap;

use rfd::FileDialog;
use serde_json; // or use serde_yaml for YAML serialization
use std::fs::File;

use egui_plot::{Bar, BarChart, Line, Orientation, PlotPoints};
use polars::prelude::*;

use super::histogram1d::Histogram;
use super::histogram2d::Histogram2D;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum HistogramTypes {
    Hist1D(Histogram),
    Hist2D(Histogram2D),
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Histogrammer {
    pub histogram_list: HashMap<String, HistogramTypes>,
}

impl Histogrammer {
    // Creates a new instance of Histogrammer.
    pub fn new() -> Self {
        Self {
            histogram_list: HashMap::new(),
        }
    }

    // Adds a new 1D histogram to the histogram list.
    pub fn add_hist1d(&mut self, name: &str, bins: usize, range: (f64, f64)) {
        let hist: Histogram = Histogram::new(bins, range); // Create a new histogram.
        self.histogram_list
            .insert(name.to_string(), HistogramTypes::Hist1D(hist)); // Store it in the hashmap.
    }

    // Fills a 1D histogram with data from a polars dataframe/column.
    pub fn fill_hist1d(&mut self, name: &str, lf: &LazyFrame, column_name: &str) -> bool {
        let hist: &mut Histogram = match self.histogram_list.get_mut(name) {
            Some(HistogramTypes::Hist1D(hist)) => hist,
            _ => return false, // Return false if the histogram doesn't exist.
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
                        eprintln!("Failed to convert DataFrame to ndarray: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                // Handle the error, for example, log it or return an error
                eprintln!("Failed to collect LazyFrame: {}", e);
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

    // Generates a histogram using the bar chart from the `egui` library.
    pub fn egui_histogram_step(&self, name: &str, color: Color32) -> Option<Line> {
        if let Some(HistogramTypes::Hist1D(hist)) = self.histogram_list.get(name) {
            let line_points = hist.step_histogram_points();

            // Convert line_points to a Vec<[f64; 2]>
            let plot_points: PlotPoints = line_points.iter().map(|&(x, y)| [x, y]).collect();

            Some(Line::new(plot_points).color(color).name(name))
        } else {
            None
        }
    }

    // Adds a new 2D histogram to the histogram list.
    pub fn add_hist2d(
        &mut self,
        name: &str,
        x_bins: usize,
        x_range: (f64, f64),
        y_bins: usize,
        y_range: (f64, f64),
    ) {
        let hist: Histogram2D = Histogram2D::new(x_bins, x_range, y_bins, y_range); // Create a new 2D histogram.
        self.histogram_list
            .insert(name.to_string(), HistogramTypes::Hist2D(hist)); // Store it in the hashmap.
    }

    // Fills a 2D histogram with x and y data.
    pub fn fill_hist2d(
        &mut self,
        name: &str,
        lf: &LazyFrame,
        x_column_name: &str,
        y_column_name: &str,
    ) -> bool {
        let hist: &mut Histogram2D = match self.histogram_list.get_mut(name) {
            Some(HistogramTypes::Hist2D(hist)) => hist,
            _ => return false, // Return false if the histogram doesn't exist.
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
                        eprintln!("Failed to convert DataFrame to ndarray: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                // Handle the error, for example, log it or return an error
                eprintln!("Failed to collect LazyFrame: {}", e);
                false
            }
        }
    }

    // Adds and fills a 2D histogram with data from Polars LazyFrame columns.
    #[allow(clippy::too_many_arguments)]
    pub fn add_fill_hist2d(
        &mut self,
        name: &str,
        lf: &LazyFrame,
        x_column_name: &str,
        x_bins: usize,
        x_range: (f64, f64),
        y_column_name: &str,
        y_bins: usize,
        y_range: (f64, f64),
    ) {
        self.add_hist2d(name, x_bins, x_range, y_bins, y_range); // Add the histogram.
        self.fill_hist2d(name, lf, x_column_name, y_column_name); // Fill it with data.
    }

    // Generates a heatmap using the `egui` library based on a 2D histogram.
    pub fn egui_heatmap(&self, name: &str) -> Option<BarChart> {
        if let Some(HistogramTypes::Hist2D(hist)) = self.histogram_list.get(name) {
            let bars_data = hist.generate_bar_data();
            let mut bars = Vec::new();

            let min: u32 = hist.min_count;
            let max: u32 = hist.max_count;
            for bar_data in bars_data {
                let color: Color32 = viridis_colormap(bar_data.count, min, max); // Determine color based on the count, using a colormap.

                let bar = Bar {
                    orientation: Orientation::Vertical,
                    argument: bar_data.x,
                    value: bar_data.height,
                    bar_width: bar_data.bar_width,
                    fill: color,
                    stroke: Stroke::new(1.0, color),
                    name: format!("x = {}\ny = {}\n{}", bar_data.x, bar_data.y, bar_data.count),
                    base_offset: Some(bar_data.y - bar_data.height / 2.0),
                };
                bars.push(bar);
            }

            // Return a BarChart object if the histogram exists, otherwise return None.
            Some(BarChart::new(bars).name(name))
        } else {
            None
        }
    }

    // additional functions
    pub fn get_histogram_list(&self) -> Vec<String> {
        let mut names: Vec<String> = self.histogram_list.keys().cloned().collect();
        names.sort();

        names
    }

    pub fn get_histogram_type(&self, name: &str) -> Option<&HistogramTypes> {
        self.histogram_list.get(name)
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
}

// Function to generate a color based on a value using the Viridis colormap, the matplotlib default.
fn viridis_colormap(value: u32, min: u32, max: u32) -> Color32 {
    // Handle case where min == max to avoid division by zero
    let normalized: f64 = if max > min {
        (value as f64 - min as f64) / (max as f64 - min as f64)
    } else {
        0.0
    }
    .clamp(0.0, 1.0);

    // Key colors from the Viridis colormap
    let viridis_colors: [(f32, f32, f32); 32] = [
        (0.267_003_98, 0.004872566, 0.329_415_08),
        (0.277_229, 0.051716984, 0.376_949_9),
        (0.282_479_7, 0.097334964, 0.419_510_57),
        (0.282_711_27, 0.139_317_69, 0.456_197_05),
        (0.278_092_62, 0.179_895_88, 0.486_377_42),
        (0.269_137_8, 0.219_429_66, 0.50989087),
        (0.256_733_54, 0.257_754_4, 0.527_183_8),
        (0.242_031_46, 0.294_643_82, 0.539_209),
        (0.226_243_75, 0.329_989_34, 0.547_162_83),
        (0.210_443_17, 0.363_856_05, 0.552_221_3),
        (0.195_412_49, 0.396_435_86, 0.555_350_9),
        (0.181_477_32, 0.428_017_32, 0.557_198_9),
        (0.168_574_23, 0.458_905_25, 0.558_067_3),
        (0.156_365_95, 0.489_384_6, 0.557_941_2),
        (0.144_535_29, 0.519_685_6, 0.556_527_7),
        (0.133_249_55, 0.549_958_2, 0.553_339_24),
        (0.123_833_07, 0.580_259_26, 0.547_771_63),
        (0.119_442_11, 0.610_546_23, 0.539_182),
        (0.124_881_9, 0.640_695_04, 0.526_954_95),
        (0.144_277_74, 0.670_499_74, 0.510_554_73),
        (0.178_281_44, 0.699_705_66, 0.489_567_13),
        (0.224_797_44, 0.728_014_4, 0.463_677_88),
        (0.281_243_44, 0.755_097_75, 0.432_683_2),
        (0.345_693_5, 0.780_604_8, 0.396_465_7),
        (0.416_705_43, 0.80418531, 0.355_029_97),
        (0.493_228_82, 0.825_506_2, 0.308_497_67),
        (0.574_270_25, 0.844_288_8, 0.257_257_7),
        (0.658_654_03, 0.860_389_95, 0.202_434_47),
        (0.744_780_54, 0.873_933, 0.147_547_83),
        (0.830_610_04, 0.885_437_7, 0.10427358),
        (0.914_002_4, 0.895_811_26, 0.100134278),
        (0.993_248_16, 0.906_154_75, 0.143_935_95),
    ];

    // Interpolate between colors in the colormap
    let scaled_val: f64 = normalized * (viridis_colors.len() - 1) as f64;
    let index: usize = scaled_val.floor() as usize;
    let fraction: f32 = scaled_val.fract() as f32;

    let color1: (f32, f32, f32) = viridis_colors[index];
    let color2: (f32, f32, f32) = viridis_colors[(index + 1).min(viridis_colors.len() - 1)];

    let red: f32 = (color1.0 + fraction * (color2.0 - color1.0)) * 255.0;
    let green: f32 = (color1.1 + fraction * (color2.1 - color1.1)) * 255.0;
    let blue: f32 = (color1.2 + fraction * (color2.2 - color1.2)) * 255.0;

    Color32::from_rgb(red as u8, green as u8, blue as u8)
}
