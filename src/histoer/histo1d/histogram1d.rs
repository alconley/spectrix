use super::plot_settings::PlotSettings;
use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::fitter::common::Data;
use crate::fitter::fit_handler::Fits;
use crate::fitter::main_fitter::{FitModel, Fitter};
use egui::Vec2b;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Histogram {
    pub name: String,
    pub bins: Vec<u64>,
    pub range: (f64, f64),
    pub overflow: u64,
    pub underflow: u64,
    pub bin_width: f64,
    pub line: EguiLine,
    pub plot_settings: PlotSettings,
    pub fits: Fits,
    pub original_bins: Vec<u64>,
}

impl Histogram {
    // Create a new Histogram with specified min, max, and number of bins
    pub fn new(name: &str, number_of_bins: usize, range: (f64, f64)) -> Self {
        Histogram {
            name: name.to_string(),
            bins: vec![0; number_of_bins],
            range,
            overflow: 0,
            underflow: 0,
            bin_width: (range.1 - range.0) / number_of_bins as f64,
            line: EguiLine {
                name: name.to_string(),
                ..Default::default()
            },
            plot_settings: PlotSettings::default(),
            fits: Fits::new(),
            original_bins: vec![0; number_of_bins],
        }
    }

    pub fn reset(&mut self) {
        self.bins = vec![0; self.bins.len()];
        self.original_bins = vec![0; self.original_bins.len()];
        self.overflow = 0;
        self.underflow = 0;
    }

    // Add a value to the histogram
    pub fn fill(&mut self, value: f64, current_step: usize, total_steps: usize) {
        if value >= self.range.0 && value < self.range.1 {
            let index = ((value - self.range.0) / self.bin_width) as usize;
            if index < self.bins.len() {
                self.bins[index] += 1;
                self.original_bins[index] += 1;
            }
        } else if value >= self.range.1 {
            self.overflow += 1;
        } else {
            self.underflow += 1;
        }
        // Update progress
        self.plot_settings.progress = Some(current_step as f32 / total_steps as f32);
    }

    pub fn auto_axis_lims(&mut self, plot_ui: &mut egui_plot::PlotUi) {
        plot_ui.set_auto_bounds(Vec2b::new(true, true));
    }

    pub fn set_counts(&mut self, counts: Vec<u64>) {
        self.bins = counts;
    }

    // Get the bin edges
    pub fn get_bin_edges(&self) -> Vec<f64> {
        (0..=self.bins.len())
            .map(|i| self.range.0 + i as f64 * self.bin_width)
            .collect()
    }

    // Convert histogram bins to line points
    pub fn update_line_points(&mut self) {
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

    // Get the bin index for a given x position.
    pub fn get_bin_index(&self, x: f64) -> Option<usize> {
        if x < self.range.0 || x > self.range.1 {
            return None;
        }

        let bin_index: usize = ((x - self.range.0) / self.bin_width).floor() as usize;

        Some(bin_index)
    }

    // Get the bin centers between the start and end x values (inclusive)
    pub fn get_bin_centers_between(&self, start_x: f64, end_x: f64) -> Vec<f64> {
        let start_bin = self.get_bin_index(start_x).unwrap_or(0);
        let end_bin = self.get_bin_index(end_x).unwrap_or(self.bins.len() - 1);

        (start_bin..=end_bin)
            .map(|bin| self.range.0 + (bin as f64 * self.bin_width) + self.bin_width * 0.5)
            .collect()
    }

    // Get the bin counts between the start and end x values (inclusive)
    pub fn get_bin_counts_between(&self, start_x: f64, end_x: f64) -> Vec<f64> {
        let start_bin = self.get_bin_index(start_x).unwrap_or(0);
        let end_bin = self.get_bin_index(end_x).unwrap_or(self.bins.len() - 1);

        (start_bin..=end_bin)
            .map(|bin| self.bins[bin] as f64)
            .collect()
    }

    // Get bin counts and bin center at x value
    pub fn get_bin_count_and_center(&self, x: f64) -> Option<(f64, f64)> {
        self.get_bin_index(x).map(|bin| {
            let bin_center = self.range.0 + (bin as f64 * self.bin_width) + self.bin_width * 0.5;
            let bin_count = self.bins[bin] as f64;
            (bin_center, bin_count)
        })
    }

    pub fn fit_background(&mut self) {
        log::info!("Fitting background for histogram: {}", self.name);
        self.fits.temp_fit = None;

        let marker_positions = self.plot_settings.markers.get_background_marker_positions();
        if marker_positions.len() < 2 {
            log::error!("Need to set at least two background markers to fit the histogram");
            return;
        }

        let (x_data, y_data): (Vec<f64>, Vec<f64>) = marker_positions
            .iter()
            .filter_map(|&pos| self.get_bin_count_and_center(pos))
            .unzip();

        let mut fitter = Fitter::new(Data {
            x: x_data,
            y: y_data,
        });

        fitter.background_model = self.fits.settings.background_model.clone();

        fitter.fit_background();

        fitter.name = format!("{} Temp Fit", self.name);
        fitter.set_name(self.name.clone());

        self.fits.temp_fit = Some(fitter);
    }

    pub fn fit_gaussians(&mut self) {
        let region_marker_positions = self.plot_settings.markers.get_region_marker_positions();
        if region_marker_positions.len() != 2 {
            log::error!("Need to set two region markers to fit the histogram");
            return;
        }

        self.plot_settings
            .markers
            .remove_peak_markers_outside_region();
        let peak_positions = self.plot_settings.markers.get_peak_marker_positions();

        let (start_x, end_x) = (region_marker_positions[0], region_marker_positions[1]);

        let data = Data {
            x: self.get_bin_centers_between(start_x, end_x),
            y: self.get_bin_counts_between(start_x, end_x),
        };

        let mut fitter = Fitter::new(data);

        let background_model = self.fits.settings.background_model.clone();

        let background_result = if let Some(temp_fit) = &self.fits.temp_fit {
            fitter.background_line = temp_fit.background_line.clone();
            temp_fit.background_result.clone()
        } else {
            None
        };

        let equal_stdev = self.fits.settings.equal_stddev;
        let free_position = self.fits.settings.free_position;
        let bin_width = self.bin_width;

        fitter.background_model = background_model;
        fitter.background_result = background_result;

        fitter.fit_model = FitModel::Gaussian(
            peak_positions.clone(),
            equal_stdev,
            free_position,
            bin_width,
        );

        fitter.fit();

        self.plot_settings.markers.clear_peak_markers();
        let updated_markers = fitter.get_peak_markers();
        for marker in updated_markers {
            self.plot_settings.markers.add_peak_marker(marker);
        }

        fitter.set_name(self.name.clone());
        self.fits.temp_fit = Some(fitter);
    }

    // Draw the histogram, fit lines, markers, and stats
    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi) {
        // update the histogram and fit lines with the log setting and draw
        let log_y = self.plot_settings.egui_settings.log_y;
        let log_x = self.plot_settings.egui_settings.log_x;

        self.line.log_y = log_y;
        self.line.log_x = log_x;
        self.line.draw(plot_ui);

        self.fits.set_log(log_y, log_x);
        self.fits.draw(plot_ui);

        self.show_stats(plot_ui);

        self.plot_settings.markers.draw_all_markers(plot_ui);
        // Check if markers are being dragged
        if self.plot_settings.markers.is_dragging() {
            // Disable dragging if a marker is being dragged
            self.plot_settings.egui_settings.allow_drag = false;
        } else {
            self.plot_settings.egui_settings.allow_drag = true;
        }

        if plot_ui.response().hovered() {
            self.plot_settings.cursor_position = plot_ui.pointer_coordinate();
            self.plot_settings.egui_settings.limit_scrolling = true;
        } else {
            self.plot_settings.cursor_position = None;
        }

        if self.plot_settings.egui_settings.limit_scrolling {
            self.limit_scrolling(plot_ui);
        }
    }

    pub fn draw_other_histograms(
        &mut self,
        plot_ui: &mut egui_plot::PlotUi,
        histograms: &[Histogram],
    ) {
        for histogram in histograms {
            let mut hist = histogram.clone();
            hist.draw(plot_ui);
        }
    }

    pub fn limit_scrolling(&self, plot_ui: &mut egui_plot::PlotUi) {
        let plot_bounds = plot_ui.plot_bounds();

        let current_x_min = plot_bounds.min()[0];
        let current_x_max = plot_bounds.max()[0];
        let current_y_min = plot_bounds.min()[1];
        let current_y_max = plot_bounds.max()[1];

        let y_max = self.bins.iter().max().cloned().unwrap_or(0) as f64;

        if current_x_min == -1.0
            && current_x_max == 1.0
            && current_y_min == 0.0
            && current_y_max == 1.0
        {
            let default_bounds =
                egui_plot::PlotBounds::from_min_max([self.range.0, 0.0], [self.range.1, y_max]);

            plot_ui.set_plot_bounds(default_bounds);
            return;
        }

        // Clamping bounds only for scrolling
        let new_x_min = current_x_min.max(self.range.0);
        let new_x_max = current_x_max.min(self.range.1);
        let new_y_min = current_y_min.max(0.0);
        let new_y_max = current_y_max.min(y_max);

        if new_x_min != current_x_min
            || new_x_max != current_x_max
            || new_y_min != current_y_min
            || new_y_max != current_y_max
        {
            let clamped_bounds =
                egui_plot::PlotBounds::from_min_max([new_x_min, new_y_min], [new_x_max, new_y_max]);
            plot_ui.set_plot_bounds(clamped_bounds);
        }
    }

    // Renders the histogram using egui_plot
    pub fn render(&mut self, ui: &mut egui::Ui) {
        // Display progress bar while hist is being filled
        self.plot_settings.progress_ui(ui);

        self.update_line_points(); // Ensure line points are updated for projections
        self.keybinds(ui); // Handle interactive elements

        let mut plot = egui_plot::Plot::new(self.name.clone());
        plot = self.plot_settings.egui_settings.apply_to_plot(plot);

        self.fits.fit_stats_ui(ui);

        let plot_response = plot.show(ui, |plot_ui| {
            self.draw(plot_ui);

            // if progress is updating, turn on the auto bounds
            if self.plot_settings.progress.is_some() {
                plot_ui.set_auto_bounds(Vec2b::new(true, true));
            }
        });

        plot_response.response.context_menu(|ui| {
            self.context_menu(ui);
        });

        self.plot_settings.interactive_response(&plot_response);
    }
}
