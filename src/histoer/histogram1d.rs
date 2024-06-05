use crate::fitter::background_fitter::BackgroundFitter;
use crate::fitter::egui_fit_markers::EguiFitMarkers;
use crate::fitter::egui_line::EguiLine;
use crate::fitter::fit_handler::{FitModel, Fits, Fitter};

use super::plot_settings::EguiPlotSettings;

// background_fit_line: DrawLine::new(true, egui::Color32::GREEN),
// deconvoluted_fit_line: DrawLine::new(true, egui::Color32::from_rgb(255, 0, 255)),
// convoluted_fit_line: DrawLine::new(true, egui::Color32::BLUE),
// stored_fit_lines: DrawLine::new(true, egui::Color32::BLUE),

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotSettings {
    #[serde(skip)]
    cursor_position: Option<egui_plot::PlotPoint>,
    egui_settings: EguiPlotSettings,
    stats_info: bool,
    show_fit_stats: bool,
    fit_stats_height: f32,
    markers: EguiFitMarkers,
}

impl Default for PlotSettings {
    fn default() -> Self {
        PlotSettings {
            cursor_position: None,
            egui_settings: EguiPlotSettings::default(),
            stats_info: true,
            show_fit_stats: true,
            fit_stats_height: 0.0,
            markers: EguiFitMarkers::new(),
        }
    }
}

impl PlotSettings {
    pub fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label("Plot Settings:");
            ui.separator();
            ui.checkbox(&mut self.stats_info, "Show Statitics");
            ui.checkbox(&mut self.show_fit_stats, "Show Fit Statitics");
            ui.add(
                egui::DragValue::new(&mut self.fit_stats_height)
                    .speed(1.0)
                    .clamp_range(0.0..=f32::INFINITY)
                    .prefix("Fit Stats Height: ")
                    .suffix(" px"),
            );

            self.egui_settings.menu_button(ui);
            self.markers.menu_button(ui);
        });
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Histogram {
    pub name: String,
    pub bins: Vec<u32>,
    pub range: (f64, f64),
    pub bin_width: f64,
    pub line: EguiLine,
    pub plot_settings: PlotSettings,
    pub fits: Fits,
}

impl Histogram {
    // Create a new Histogram with specified min, max, and number of bins
    pub fn new(name: &str, number_of_bins: usize, range: (f64, f64)) -> Self {
        Histogram {
            name: name.to_string(),
            bins: vec![0; number_of_bins],
            range,
            bin_width: (range.1 - range.0) / number_of_bins as f64,
            line: EguiLine::new(name.to_string(), egui::Color32::LIGHT_BLUE),
            plot_settings: PlotSettings::default(),
            fits: Fits::new(),
        }
    }

    // Add a value to the histogram
    pub fn fill(&mut self, value: f64) {
        if value >= self.range.0 && value < self.range.1 {
            let index = ((value - self.range.0) / self.bin_width) as usize;
            if index < self.bins.len() {
                self.bins[index] += 1;
            }
        }
    }

    // Convert histogram bins to line points
    fn update_line_points(&mut self) {
        self.line.points = self
            .bins
            .iter()
            .enumerate()
            .flat_map(|(index, &count)| {
                let start = self.range.0 + index as f64 * self.bin_width;
                let end = start + self.bin_width;
                let y_value = count as f64;
                vec![[start, y_value], [end, y_value]]
            })
            .collect();
    }

    // Get the bin number for a given x position.
    fn get_bin(&self, x: f64) -> Option<usize> {
        if x < self.range.0 || x > self.range.1 {
            return None;
        }

        let bin_index: usize = ((x - self.range.0) / self.bin_width).floor() as usize;

        Some(bin_index)
    }

    // Get the bin centers for the histogram
    fn _get_bin_centers(&self) -> Vec<f64> {
        self.bins
            .iter()
            .enumerate()
            .map(|(index, _)| self.range.0 + (index as f64 * self.bin_width) + self.bin_width * 0.5)
            .collect()
    }

    // Get the bin centers between the start and end x values (inclusive)
    fn get_bin_centers_between(&self, start_x: f64, end_x: f64) -> Vec<f64> {
        let start_bin = self.get_bin(start_x).unwrap_or(0);
        let end_bin = self.get_bin(end_x).unwrap_or(self.bins.len() - 1);

        (start_bin..=end_bin)
            .map(|bin| self.range.0 + (bin as f64 * self.bin_width) + self.bin_width * 0.5)
            .collect()
    }

    // Get the bin counts between the start and end x values (inclusive)
    fn get_bin_counts_between(&self, start_x: f64, end_x: f64) -> Vec<f64> {
        let start_bin = self.get_bin(start_x).unwrap_or(0);
        let end_bin = self.get_bin(end_x).unwrap_or(self.bins.len() - 1);

        (start_bin..=end_bin)
            .map(|bin| self.bins[bin] as f64)
            .collect()
    }

    // Get bin counts and bin center at x value
    fn get_bin_count_and_center(&self, x: f64) -> Option<(f64, f64)> {
        self.get_bin(x).map(|bin| {
            let bin_center = self.range.0 + (bin as f64 * self.bin_width) + self.bin_width * 0.5;
            let bin_count = self.bins[bin] as f64;
            (bin_center, bin_count)
        })
    }

    // Sum counts between the region markers
    fn sum_counts_between_region_markers(&self) -> f64 {
        if self.plot_settings.markers.region_markers.len() == 2 {
            let start_x = self.plot_settings.markers.region_markers[0];
            let end_x = self.plot_settings.markers.region_markers[1];
            self.get_bin_counts_between(start_x, end_x).iter().sum()
        } else {
            0.0
        }
    }

    // Calculate the statistics for the histogram within the specified x range.
    fn stats(&self, start_x: f64, end_x: f64) -> (u32, f64, f64) {
        let start_bin = self.get_bin(start_x).unwrap_or(0);
        let end_bin = self.get_bin(end_x).unwrap_or(self.bins.len() - 1);

        let mut sum_product = 0.0;
        let mut total_count = 0;

        for bin in start_bin..=end_bin {
            if bin < self.bins.len() {
                let bin_center =
                    self.range.0 + (bin as f64 * self.bin_width) + self.bin_width * 0.5;
                sum_product += self.bins[bin] as f64 * bin_center;
                total_count += self.bins[bin];
            } else {
                break;
            }
        }

        if total_count == 0 {
            (0, 0.0, 0.0)
        } else {
            let mean = sum_product / total_count as f64;

            let mut sum_squared_diff = 0.0;

            for bin in start_bin..=end_bin {
                if bin < self.bins.len() {
                    let bin_center =
                        self.range.0 + (bin as f64 * self.bin_width) + (self.bin_width * 0.5);
                    let diff = bin_center - mean;
                    sum_squared_diff += self.bins[bin] as f64 * diff * diff;
                } else {
                    break;
                }
            }

            let stdev = (sum_squared_diff / total_count as f64).sqrt();

            (total_count, mean, stdev)
        }
    }

    // Generates legend entries for the histogram based on the specified x range.
    fn legend_entries(&self, start_x: f64, end_x: f64) -> Vec<String> {
        let (integral, mean, stdev) = self.stats(start_x, end_x);
        vec![
            format!("Integral: {}", integral),
            format!("Mean: {:.2}", mean),
            format!("Stdev: {:.2}", stdev),
        ]
    }

    // Fit the background with a linear line using the background markers
    fn fit_background(&mut self) {
        self.fits.remove_temp_fits();

        if self.plot_settings.markers.background_markers.len() < 2 {
            log::error!("Need to set at least two background markers to fit the histogram");
            return;
        }

        let marker_positions = self.plot_settings.markers.background_markers.clone();
        let (x_data, y_data): (Vec<f64>, Vec<f64>) = marker_positions
            .iter()
            .filter_map(|&pos| {
                self.get_bin_count_and_center(pos)
                    .map(|(center, count)| (center, count))
            })
            .unzip();

        let mut background_fitter = BackgroundFitter::new(x_data, y_data, FitModel::Linear);
        background_fitter.fit();
        self.fits.temp_background_fit = Some(background_fitter);
    }

    // Fit the gaussians at the peak markers
    fn fit_gaussians(&mut self) {
        if self.plot_settings.markers.region_markers.len() != 2 {
            log::error!("Need to set two region markers to fit the histogram");
            return;
        }

        self.plot_settings
            .markers
            .remove_peak_markers_outside_region();
        let peak_positions = self.plot_settings.markers.peak_markers.clone();

        if self.fits.temp_background_fit.is_none() {
            if self.plot_settings.markers.background_markers.len() <= 1 {
                self.plot_settings
                    .markers
                    .background_markers
                    .clone_from(&self.plot_settings.markers.region_markers)
            }
            self.fit_background();
        }

        let mut fitter = Fitter::new(
            FitModel::Gaussian(peak_positions),
            self.fits.temp_background_fit.clone(),
        );

        let (start_x, end_x) = (
            self.plot_settings.markers.region_markers[0],
            self.plot_settings.markers.region_markers[1],
        );

        fitter.x_data = self.get_bin_centers_between(start_x, end_x);
        fitter.y_data = self.get_bin_counts_between(start_x, end_x);

        fitter.fit();

        self.plot_settings.markers.peak_markers = fitter.get_peak_markers();
        self.fits.temp_fit = Some(fitter);
    }

    // Handles the interactive elements of the histogram
    fn interactive(&mut self, ui: &mut egui::Ui) {
        self.plot_settings.markers.cursor_position = self.plot_settings.cursor_position;

        if let Some(_cursor_position) = self.plot_settings.cursor_position {
            self.plot_settings.markers.interactive_markers(ui);

            if ui.input(|i| i.key_pressed(egui::Key::Minus) || i.key_pressed(egui::Key::Delete)) {
                self.fits.remove_temp_fits();
            }

            if ui.input(|i| i.key_pressed(egui::Key::G)) {
                self.fit_background();
            }

            if ui.input(|i| i.key_pressed(egui::Key::F)) {
                self.fit_gaussians();
            }

            if ui.input(|i| i.key_pressed(egui::Key::S)) {
                // Store the temporary fit if it exists
                if let Some(temp_fit) = self.fits.temp_fit.take() {
                    self.fits.stored_fits.push(temp_fit);
                }
            }

            if ui.input(|i| i.key_pressed(egui::Key::I)) {
                self.plot_settings.stats_info = !self.plot_settings.stats_info;
            }

            if ui.input(|i| i.key_pressed(egui::Key::A)) {
                let total_count = self.sum_counts_between_region_markers();
                log::info!("Total count between region markers: {}", total_count);
            }

            if ui.input(|i| i.key_pressed(egui::Key::L)) {
                self.plot_settings.egui_settings.log_y = !self.plot_settings.egui_settings.log_y;
            }
        }
    }

    // Renders the histogram using egui_plot
    pub fn render(&mut self, ui: &mut egui::Ui) {
        self.update_line_points(); // Ensure line points are updated
        self.interactive(ui); // Handle interactive elements

        let log_y = self.plot_settings.egui_settings.log_y;

        let mut plot = egui_plot::Plot::new(self.name.clone());
        plot = self.plot_settings.egui_settings.apply_to_plot(plot);

        ui.vertical(|ui| {
            if self.plot_settings.show_fit_stats {
                egui::ScrollArea::both()
                    .max_height(self.plot_settings.fit_stats_height)
                    .show(ui, |ui| {
                        self.fits.fit_stats_grid_ui(ui);
                    });
            }

            plot.show(ui, |plot_ui| {
                let plot_min_x = plot_ui.plot_bounds().min()[0];
                let plot_max_x = plot_ui.plot_bounds().max()[0];

                if self.plot_settings.stats_info {
                    let stats_entries = self.legend_entries(plot_min_x, plot_max_x);
                    for entry in stats_entries.iter() {
                        plot_ui.text(
                            egui_plot::Text::new(egui_plot::PlotPoint::new(0, 0), " ") // Placeholder for positioning; adjust as needed
                                .highlight(false)
                                .color(self.line.color)
                                .name(entry),
                        );
                    }
                }

                self.line.draw(plot_ui);

                if plot_ui.response().hovered() {
                    self.plot_settings.cursor_position = plot_ui.pointer_coordinate();
                } else {
                    self.plot_settings.cursor_position = None;
                }

                self.plot_settings.markers.draw_all_markers(plot_ui);

                self.fits.draw(plot_ui, log_y);
            })
            .response
            .context_menu(|ui| {
                self.line.menu_button(ui);
                self.plot_settings.settings_ui(ui);
                self.fits.fit_context_menu_ui(ui);
            });
        });
    }
}
