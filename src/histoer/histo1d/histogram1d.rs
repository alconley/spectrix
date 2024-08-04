use super::plot_settings::PlotSettings;
use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::fitter::background_fitter::BackgroundFitter;
use crate::fitter::fit_handler::Fits;
use crate::fitter::main_fitter::{FitModel, Fitter};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Histogram {
    pub name: String,
    pub bins: Vec<u64>,
    pub range: (f64, f64),
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

    // Add a value to the histogram
    pub fn fill(&mut self, value: f64) {
        if value >= self.range.0 && value < self.range.1 {
            let index = ((value - self.range.0) / self.bin_width) as usize;
            if index < self.bins.len() {
                self.bins[index] += 1;
                self.original_bins[index] += 1;
            }
        }
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
        self.fits.remove_temp_fits();

        let marker_positions = self.plot_settings.markers.get_background_marker_positions();
        if marker_positions.len() < 2 {
            log::error!("Need to set at least two background markers to fit the histogram");
            return;
        }

        let (x_data, y_data): (Vec<f64>, Vec<f64>) = marker_positions
            .iter()
            .filter_map(|&pos| self.get_bin_count_and_center(pos))
            .unzip();

        // let mut background_fitter = BackgroundFitter::new(x_data, y_data, FitModel::Linear);
        let mut background_fitter =
            BackgroundFitter::new(x_data, y_data, self.fits.settings.background_model.clone());
        background_fitter.fit();

        background_fitter.fit_line.name = format!("{} Temp Background", self.name);
        self.fits.temp_background_fit = Some(background_fitter);
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

        if self.fits.temp_background_fit.is_none() {
            if self.plot_settings.markers.background_markers.len() <= 1 {
                for position in region_marker_positions.iter() {
                    self.plot_settings.markers.add_background_marker(*position);
                }
            }
            self.fit_background();
        }

        let mut fitter = Fitter::new(
            FitModel::Gaussian(
                peak_positions,
                self.fits.settings.free_stddev,
                self.fits.settings.free_position,
                self.bin_width,
            ),
            self.fits.temp_background_fit.clone(),
        );

        let (start_x, end_x) = (region_marker_positions[0], region_marker_positions[1]);

        fitter.x_data = self.get_bin_centers_between(start_x, end_x);
        fitter.y_data = self.get_bin_counts_between(start_x, end_x);

        fitter.fit();

        fitter.set_name(self.name.clone());

        // clear peak markers and add the new peak markers
        self.plot_settings.markers.clear_peak_markers();

        let peak_values = fitter.get_peak_markers();
        for peak in peak_values {
            self.plot_settings.markers.add_peak_marker(peak);
        }

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

        if plot_ui.response().hovered() {
            self.plot_settings.cursor_position = plot_ui.pointer_coordinate();
        } else {
            self.plot_settings.cursor_position = None;
        }

        if self.plot_settings.egui_settings.limit_scrolling {
            self.limit_scrolling(plot_ui);
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
        self.update_line_points(); // Ensure line points are updated for projections
        self.keybinds(ui); // Handle interactive elements

        let mut plot = egui_plot::Plot::new(self.name.clone());
        plot = self.plot_settings.egui_settings.apply_to_plot(plot);

        self.fits.fit_stats_ui(ui);

        let plot_response = plot.show(ui, |plot_ui| {
            self.draw(plot_ui);
        });

        plot_response.response.context_menu(|ui| {
            self.context_menu(ui);
        });

        self.plot_settings.interactive_response(&plot_response);
    }
}
