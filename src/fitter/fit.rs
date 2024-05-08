// use egui_plot::PlotUi;

// use egui::{Color32, Stroke};
// use egui_plot::{Line, PlotPoints};

// use rfd::FileDialog;

// use std::fs::File;
// use std::io::{self, Read};

// use serde_yaml;

// use super::background_fitter::BackgroundFitter;
// use super::egui_markers::EguiFitMarkers;
// use super::gaussian::GaussianFitter;

// use crate::histoer::histogram1d::Histogram;

// #[derive(serde::Deserialize, serde::Serialize)]
// pub struct Fit {
//     pub fit: Option<GaussianFitter>,
//     background: Option<BackgroundFitter>,
//     histogram: Histogram,
//     pub markers: EguiFitMarkers,
// }

// impl Fit {
//     pub fn new(histogram: Histogram, markers: EguiFitMarkers) -> Self {
//         Self {
//             fit: None,
//             background: None,
//             histogram,
//             markers,
//         }
//     }

//     pub fn get_background_marker_data(&self) -> (Vec<f64>, Vec<f64>) {
//         let bg_markers = self.markers.background_markers.clone();

//         let mut y_values = Vec::new();
//         let mut x_values = Vec::new();

//         for x in bg_markers {
//             // get the bin index
//             if let Some(bin_index) = self.histogram.get_bin(x) {
//                 let bin_center = self.histogram.range.0
//                     + (bin_index as f64 * self.histogram.bin_width)
//                     + (self.histogram.bin_width * 0.5);
//                 x_values.push(bin_center);
//                 y_values.push(self.histogram.bins[bin_index] as f64);
//             }
//         }

//         (x_values, y_values)
//     }

//     pub fn fit_background(&mut self) -> Result<(), &'static str> {
//         let (x_values, y_values) = self.get_background_marker_data();

//         // Initialize BackgroundFitter with the obtained x and y values
//         let mut background_fitter = BackgroundFitter::new(x_values, y_values);

//         // Perform the fit and calculate background line points
//         background_fitter.fit()?;

//         // Update the background property with the fitted background_fitter
//         self.background = Some(background_fitter);

//         Ok(())
//     }

//     pub fn create_background_subtracted_histogram(&self) -> Result<Histogram, &'static str> {
//         if let Some(background_fitter) = &self.background {
//             let (slope, intercept) = background_fitter
//                 .background_params
//                 .ok_or("Background parameters not set.")?;

//             let mut subtracted_histogram = self.histogram.clone();

//             // Subtract background estimate from each bin
//             for (index, bin_count) in subtracted_histogram.bins.iter_mut().enumerate() {
//                 let bin_center = self.histogram.range.0
//                     + (self.histogram.bin_width * index as f64)
//                     + (self.histogram.bin_width / 2.0);
//                 let background_estimate = slope * bin_center + intercept;
//                 *bin_count = bin_count.saturating_sub(background_estimate.round() as u32);
//             }

//             Ok(subtracted_histogram)
//         } else {
//             Err("No background fitter available for background subtraction.")
//         }
//     }

//     pub fn fit_gaussian(&mut self) -> Result<(), &'static str> {
//         // Ensure there are exactly two region markers to define a fit region
//         if self.markers.region_markers.len() != 2 {
//             return Err("Need two region markers to define a fit region.");
//         }

//         // remove peak markers that are outside the region markers
//         self.markers.remove_peak_markers_outside_region();

//         // if there are no background markers, use the region markers as defaults
//         if self.markers.background_markers.is_empty() {
//             self.markers
//                 .background_markers
//                 .push(self.markers.region_markers[0]);
//             self.markers
//                 .background_markers
//                 .push(self.markers.region_markers[1]);
//         }

//         // fit the background
//         let _ = self.fit_background();

//         // Ensure background subtraction has been performed
//         let bg_subtracted_histogram = self.create_background_subtracted_histogram()?;

//         // Extract x and y data between region markers
//         let start_bin = bg_subtracted_histogram
//             .get_bin(self.markers.region_markers[0])
//             .unwrap_or(0);
//         let end_bin = bg_subtracted_histogram
//             .get_bin(self.markers.region_markers[1])
//             .unwrap_or(bg_subtracted_histogram.bins.len() - 1);

//         let mut x_data = Vec::new();
//         let mut y_data = Vec::new();

//         for bin_index in start_bin..=end_bin {
//             let bin_center = bg_subtracted_histogram.range.0
//                 + (bg_subtracted_histogram.bin_width * bin_index as f64)
//                 + (bg_subtracted_histogram.bin_width / 2.0);
//             let bin_count = bg_subtracted_histogram.bins[bin_index];
//             x_data.push(bin_center);
//             y_data.push(bin_count as f64);
//         }

//         // Initialize GaussianFitter with x and y data
//         let mut gaussian_fitter =
//             GaussianFitter::new(x_data, y_data, self.markers.peak_markers.clone());

//         // Perform Gaussian fit
//         gaussian_fitter.multi_gauss_fit();

//         // get the decomposition fit lines
//         // gaussian_fitter.get_fit_decomposition_line_points();

//         // update peak markers with the fitted peak markers
//         self.markers.peak_markers = gaussian_fitter.peak_markers.clone();

//         // Update the fit property with the fitted GaussianFitter
//         self.fit = Some(gaussian_fitter);

//         Ok(())
//     }

//     pub fn draw(
//         &mut self,
//         plot_ui: &mut PlotUi,
//         convoluted_color: Color32,
//         decomposition_color: Color32,
//     ) {
//         if let Some(background_fitter) = &self.background {
//             background_fitter.draw_background_line(plot_ui);

//             if let Some(gaussian_fitter) = &self.fit {
//                 // gaussian_fitter.draw_decomposition_fit_lines(plot_ui, decomposition_color);

//                 let slope = background_fitter.background_params.unwrap().0;
//                 let intercept = background_fitter.background_params.unwrap().1;

//                 // Calculate and draw the convoluted fit
//                 let convoluted_fit_points = gaussian_fitter
//                     .calculate_convoluted_fit_points_with_background(slope, intercept);
//                 let line = Line::new(PlotPoints::Owned(convoluted_fit_points))
//                     .color(convoluted_color) // Choose a distinct color for the convoluted fit
//                     .stroke(Stroke::new(1.0, convoluted_color));
//                 plot_ui.line(line);
//             }
//         }
//     }

//     pub fn clear(&mut self) {
//         self.fit = None;
//         self.background = None;
//     }

//     pub fn save_fit_to_file(&self) -> Result<(), io::Error> {
//         if let Some(path) = FileDialog::new()
//             .set_title("Save Fit")
//             .add_filter("YAML files", &["yaml"])
//             .save_file()
//         {
//             let file = File::create(path)?;
//             serde_yaml::to_writer(file, &self)
//                 .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
//         }
//         Ok(())
//     }

//     pub fn load_fit_from_file() -> Result<Self, io::Error> {
//         if let Some(path) = FileDialog::new()
//             .set_title("Open Fit")
//             .add_filter("YAML files", &["yaml"])
//             .pick_file()
//         {
//             let mut file = File::open(path)?;
//             let mut contents = String::new();
//             file.read_to_string(&mut contents)?;
//             let fit_handler: Self = serde_yaml::from_str(&contents)
//                 .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
//             return Ok(fit_handler);
//         }
//         Err(io::Error::new(io::ErrorKind::NotFound, "No file selected"))
//     }

//     pub fn fit_ui(&mut self, ui: &mut egui::Ui) {
//         if let Some(gaussian_fitter) = &self.fit {
//             if let Some(params) = &gaussian_fitter.fit_params {
//                 egui::ScrollArea::vertical()
//                     .id_source("current_fit_scroll")
//                     .show(ui, |ui| {
//                         egui::Grid::new("current_fit_stats_grid")
//                             .striped(true) // Adds a subtle background color to every other row for readability
//                             // .min_col_width(100.0) // Ensures that each column has a minimum width for better alignment
//                             .show(ui, |ui| {
//                                 // Headers
//                                 ui.label("Fit #");
//                                 ui.label("Mean");
//                                 ui.label("FWHM");
//                                 ui.label("Area");
//                                 ui.end_row(); // End the header row

//                                 // Iterate over params to fill in the grid with fit statistics
//                                 for (index, param) in params.iter().enumerate() {
//                                     ui.label(format!("{}", index)); // Fit number
//                                     ui.label(format!("{:.2} ± {:.2}", param.mean.0, param.mean.1)); // Mean
//                                     ui.label(format!("{:.2} ± {:.2}", param.fwhm.0, param.fwhm.1)); // FWHM
//                                     ui.label(format!("{:.0} ± {:.0}", param.area.0, param.area.1)); // Area
//                                     ui.end_row(); // Move to the next row for the next set of stats
//                                 }
//                             });
//                     });
//             }
//         }
//     }
// }
