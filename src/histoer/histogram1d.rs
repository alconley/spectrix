use super::peak_finder::PeakFindingSettings;
use super::plot_settings::EguiPlotSettings;
use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::fitter::background_fitter::BackgroundFitter;
use crate::fitter::fit_handler::Fits;
use crate::fitter::fit_markers::EguiFitMarkers;
use crate::fitter::main_fitter::{FitModel, Fitter};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotSettings {
    #[serde(skip)]
    cursor_position: Option<egui_plot::PlotPoint>,
    egui_settings: EguiPlotSettings,
    stats_info: bool,
    markers: EguiFitMarkers,
    rebin_factor: usize,
    find_peaks_settings: PeakFindingSettings,
}
impl Default for PlotSettings {
    fn default() -> Self {
        PlotSettings {
            cursor_position: None,
            egui_settings: EguiPlotSettings::default(),
            stats_info: false,
            markers: EguiFitMarkers::new(),
            rebin_factor: 1,
            find_peaks_settings: PeakFindingSettings::default(),
        }
    }
}
impl PlotSettings {
    pub fn settings_ui(&mut self, ui: &mut egui::Ui) {
        self.egui_settings.menu_button(ui);
        ui.checkbox(&mut self.stats_info, "Show Statistics");
        self.markers.menu_button(ui);
    }

    fn interactive_response(&mut self, response: &egui_plot::PlotResponse<()>) {
        self.markers.interactive_dragging(response);
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
    pub original_bins: Vec<u32>,
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

    // Rebin the histogram according to the rebin factor
    fn rebin(&mut self) {
        let rebin_factor = self.plot_settings.rebin_factor;
        let new_bin_count = self.original_bins.len() / rebin_factor;
        let mut new_bins = vec![0; new_bin_count];

        for (i, &count) in self.original_bins.iter().enumerate() {
            let new_index = i / rebin_factor;
            new_bins[new_index] += count;
        }

        self.bins = new_bins;
        self.bin_width = (self.range.1 - self.range.0) / new_bin_count as f64;
        self.update_line_points();
    }

    // Compute the possible rebin factors based on the initial number of bins
    fn possible_rebin_factors(&self) -> Vec<usize> {
        let mut factors = vec![];
        factors.push(1);
        let mut factor = 1;
        while self.original_bins.len() % (factor * 2) == 0 {
            factor *= 2;
            factors.push(factor);
        }

        // remove the last factor if it is the same as the number of bins
        if factors.last() == Some(&self.original_bins.len()) {
            factors.pop();
        }
        factors
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
        let marker_positions = self.plot_settings.markers.get_region_marker_positions();
        if marker_positions.len() == 2 {
            let start_x = marker_positions[0];
            let end_x = marker_positions[1];
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

    // Get the legend stat entries for the histogram
    fn show_stats(&self, plot_ui: &mut egui_plot::PlotUi) {
        if self.plot_settings.stats_info {
            let plot_min_x = plot_ui.plot_bounds().min()[0];
            let plot_max_x = plot_ui.plot_bounds().max()[0];

            let (integral, mean, stdev) = self.stats(plot_min_x, plot_max_x);
            let stats_entries = [
                format!("Integral: {}", integral),
                format!("Mean: {:.2}", mean),
                format!("Stdev: {:.2}", stdev),
            ];

            for entry in stats_entries.iter() {
                plot_ui.text(
                    egui_plot::Text::new(egui_plot::PlotPoint::new(0, 0), " ") // Placeholder for positioning; adjust as needed
                        .highlight(false)
                        .color(self.line.color)
                        .name(entry),
                );
            }
        }
    }

    fn fit_background(&mut self) {
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

    fn fit_gaussians(&mut self) {
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

    // Add a function to find peaks
    pub fn find_peaks(&mut self) {
        // Clear the peak markers
        self.plot_settings.markers.clear_peak_markers();

        let region_marker_positions = self.plot_settings.markers.get_region_marker_positions();
        let mut background_marker_positions =
            self.plot_settings.markers.get_background_marker_positions();

        // Sort background markers
        background_marker_positions.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut peaks_found_with_background = false;
        let mut peaks_found_with_region = false;

        let y_data: Vec<f64> = if background_marker_positions.len() >= 2 {
            self.fit_background();

            // Extract the y data from the temp background fit
            if let Some(temp_background) = &self.fits.temp_background_fit {
                // Get the x and y data between the min and max background markers
                let (start_x, end_x) = (
                    background_marker_positions[0],
                    background_marker_positions[background_marker_positions.len() - 1],
                );
                let x_data = self.get_bin_centers_between(start_x, end_x);
                let y_data = self.get_bin_counts_between(start_x, end_x);

                peaks_found_with_background = true;

                // Put the data in the background fitter to subtract the background
                temp_background.subtract_background(x_data, y_data)
            } else {
                log::error!("Failed to fit background");
                return;
            }
        } else if region_marker_positions.len() == 2 {
            let (start_x, end_x) = (region_marker_positions[0], region_marker_positions[1]);
            peaks_found_with_region = true;
            self.get_bin_counts_between(start_x, end_x)
        } else {
            self.bins.iter().map(|&count| count as f64).collect()
        };

        let peaks = self.plot_settings.find_peaks_settings.find_peaks(y_data);

        // Add peak markers at detected peaks
        for peak in &peaks {
            let peak_position = peak.middle_position();
            log::info!("Peak at position: {}", peak_position);
            // Adjust peak position relative to the first background marker
            if peaks_found_with_background {
                let adjusted_peak_position =
                    self.bin_width * peak_position as f64 + background_marker_positions[0];
                self.plot_settings
                    .markers
                    .add_peak_marker(adjusted_peak_position);
            } else if peaks_found_with_region {
                let adjusted_peak_position =
                    self.bin_width * peak_position as f64 + region_marker_positions[0];
                self.plot_settings
                    .markers
                    .add_peak_marker(adjusted_peak_position);
            } else {
                let adjusted_peak_position = self.bin_width * peak_position as f64 + self.range.0;
                self.plot_settings
                    .markers
                    .add_peak_marker(adjusted_peak_position);
            }
        }
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
                self.fits.store_temp_fit();
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

            if ui.input(|i| i.key_pressed(egui::Key::O)) {
                self.find_peaks();
            }
        }
    }

    // create a ui function to show the keybinds in the context menu
    fn keybinds_ui(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Keybind Help", |ui| {
            egui::ScrollArea::vertical()
            .id_source("keybinds_scroll")
            .max_height(300.0)
            .show(ui, |ui| {
                ui.heading("Keybinds");
                ui.separator();
                ui.label("Markers");
                ui.label("P: Add Marker");
                ui.label("B: Add Background Marker");
                ui.label("R: Add Region Marker");
                ui.label("-: Remove Marker Closest to Cursor");
                ui.label("Delete: Remove All Markers and Temp Fits");
                ui.label("Middle: Move marker").on_hover_text("Markers can be dragged to new positions with the middle mouse button when hovered over center point");
                ui.separator();
                ui.label("Fitting");
                ui.label("G: Fit Background").on_hover_text("Fit a linear background using the background markers");
                ui.label("F: Fit Gaussians").on_hover_text("Fit gaussians at the peak markers give some region with a linear background");
                ui.label("S: Store Fit").on_hover_text("Store the current fit as a permanent fit which can be saved and loaded later");
                ui.separator();
                ui.label("Plot");
                ui.label("I: Toggle Stats");
                ui.label("L: Toggle Log Y");
                ui.separator();
                ui.label("Peak Finder");
                ui.label("O: Detect Peaks").on_hover_text("Detect peaks in the spectrum using the peak finding parameters");

            });
        });
    }

    // Draw the histogram, fit lines, markers, and stats
    fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi) {
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
    }

    // Handles the context menu for the histogram
    fn context_menu(&mut self, ui: &mut egui::Ui) {
        self.line.menu_button(ui);
        self.plot_settings.settings_ui(ui);
        self.fits.fit_context_menu_ui(ui);
        self.keybinds_ui(ui);

        ui.separator();
        ui.heading("Rebin");

        let possible_factors = self.possible_rebin_factors();

        ui.label("Rebin Factor:");

        ui.horizontal_wrapped(|ui| {
            for &factor in &possible_factors {
                if ui
                    .selectable_label(
                        self.plot_settings.rebin_factor == factor,
                        format!("{}", factor),
                    )
                    .clicked()
                {
                    self.plot_settings.rebin_factor = factor;
                    self.rebin();
                }
            }
        });

        // Add find peaks button
        ui.separator();
        ui.heading("Find Peaks");
        if ui.button("Detect Peaks")
            .on_hover_text("Takes the settings (adjust below) and finds peaks in the spectrum\nIf there are background markers, it will fit a background before it finds the peaks in between the min and max values. Likewise for region markers.\nKeybind: o").clicked() {
            self.find_peaks();
        }
        self.plot_settings.find_peaks_settings.menu_button(ui);
    }

    // Renders the histogram using egui_plot
    pub fn render(&mut self, ui: &mut egui::Ui) {
        self.update_line_points(); // Ensure line points are updated
        self.interactive(ui); // Handle interactive elements

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
