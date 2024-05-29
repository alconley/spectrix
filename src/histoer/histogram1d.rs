use crate::fitter::egui_markers::EguiFitMarkers;
use crate::fitter::fitter::{BackgroundFitter, FitModel, Fitter};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotSettings {
    #[serde(skip)]
    cursor_position: Option<egui_plot::PlotPoint>,
    info: bool,
    color: egui::Color32,
    markers: EguiFitMarkers,
    show_x_value: bool,
    show_y_value: bool,
    center_x_axis: bool,
    center_y_axis: bool,
    allow_zoom: bool,
    allow_boxed_zoom: bool,
    allow_drag: bool,
    allow_scroll: bool,
    clamp_grid: bool,
    show_grid: bool,
    sharp_grid_lines: bool,
    show_background: bool,
}

impl Default for PlotSettings {
    fn default() -> Self {
        PlotSettings {
            cursor_position: None,
            info: true,
            color: egui::Color32::LIGHT_BLUE,
            markers: EguiFitMarkers::new(),
            show_x_value: true,
            show_y_value: true,
            center_x_axis: false,
            center_y_axis: false,
            allow_zoom: true,
            allow_boxed_zoom: true,
            allow_drag: true,
            allow_scroll: true,
            clamp_grid: true,
            show_grid: true,
            sharp_grid_lines: true,
            show_background: true,
        }
    }
}

impl PlotSettings {
    pub fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label("Plot Settings:");
            ui.separator();
            ui.checkbox(&mut self.info, "Show Info");
            ui.menu_button("Manipulation Settings", |ui| {
                ui.vertical(|ui| {
                    ui.checkbox(&mut self.show_x_value, "Show X Value");
                    ui.checkbox(&mut self.show_y_value, "Show Y Value");
                    ui.checkbox(&mut self.center_x_axis, "Center X Axis");
                    ui.checkbox(&mut self.center_y_axis, "Center Y Axis");
                    ui.checkbox(&mut self.allow_zoom, "Allow Zoom");
                    ui.checkbox(&mut self.allow_boxed_zoom, "Allow Boxed Zoom");
                    ui.checkbox(&mut self.allow_drag, "Allow Drag");
                    ui.checkbox(&mut self.allow_scroll, "Allow Scroll");
                    ui.checkbox(&mut self.clamp_grid, "Clamp Grid");
                    ui.checkbox(&mut self.show_grid, "Show Grid");
                    ui.checkbox(&mut self.sharp_grid_lines, "Sharp Grid Lines");
                    ui.checkbox(&mut self.show_background, "Show Background");
                });
            });
            ui.color_edit_button_srgba(&mut self.color);
        });
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Fits {
    pub temp_fit: Option<Fitter>,
    pub temp_background_fit: Option<BackgroundFitter>,
    pub stored_fits: Vec<Fitter>,
}

impl Fits {
    pub fn new() -> Self {
        Fits {
            temp_fit: None,
            temp_background_fit: None,
            stored_fits: Vec::new(),
        }
    }

    pub fn remove_temp_fits(&mut self) {
        self.temp_fit = None;
        self.temp_background_fit = None;
    }

    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi) {
        if let Some(temp_fit) = &self.temp_fit {
            temp_fit.draw(
                plot_ui,
                egui::Color32::from_rgb(255, 0, 255),
                egui::Color32::GREEN,
                egui::Color32::BLUE,
            );
        }

        if let Some(temp_background_fit) = &self.temp_background_fit {
            temp_background_fit.draw(plot_ui, egui::Color32::GREEN);
        }

        for fit in self.stored_fits.iter() {
            fit.draw(
                plot_ui,
                egui::Color32::from_rgb(162, 0, 255),
                egui::Color32::from_rgb(162, 0, 255),
                egui::Color32::from_rgb(162, 0, 255),
            );
        }
    }

    pub fn fit_stats_ui(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Fits:");
            ui.separator();
            ui.label(format!("Stored fits: {}", self.stored_fits.len()));
            ui.separator();
            ui.label("Temp fit:");
            if self.temp_fit.is_some() {
                ui.label("Yes");
            } else {
                ui.label("No");
            }
            ui.separator();
            ui.label("Temp background fit:");
            if self.temp_background_fit.is_some() {
                ui.label("Yes");
            } else {
                ui.label("No");
            }
        });
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Histogram {
    pub name: String,
    pub bins: Vec<u32>,
    pub range: (f64, f64),
    pub bin_width: f64,
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

    // Get the bin number for a given x position.
    pub fn get_bin(&self, x: f64) -> Option<usize> {
        if x < self.range.0 || x > self.range.1 {
            return None;
        }

        let bin_index: usize = ((x - self.range.0) / self.bin_width).floor() as usize;

        Some(bin_index)
    }

    // Get the bin centers for the histogram
    pub fn get_bin_centers(&self) -> Vec<f64> {
        let mut bin_centers = Vec::new();

        for (index, _) in self.bins.iter().enumerate() {
            let bin_center = self.range.0 + (index as f64 * self.bin_width) + self.bin_width * 0.5;
            bin_centers.push(bin_center);
        }

        bin_centers
    }

    // get the x_values (bin centers) between the start and end x values (inclusive
    fn get_bin_centers_between(&self, start_x: f64, end_x: f64) -> Vec<f64> {
        let start_bin = self.get_bin(start_x).unwrap_or(0);
        let end_bin = self.get_bin(end_x).unwrap_or(self.bins.len() - 1);

        let mut bin_centers = Vec::new();

        for bin in start_bin..=end_bin {
            if bin < self.bins.len() {
                let bin_center =
                    self.range.0 + (bin as f64 * self.bin_width) + self.bin_width * 0.5;
                bin_centers.push(bin_center);
            } else {
                break;
            }
        }

        bin_centers
    }

    // get the bin counts between the start and end x values (inclusive)
    fn get_bin_counts_between(&self, start_x: f64, end_x: f64) -> Vec<f64> {
        let start_bin = self.get_bin(start_x).unwrap_or(0);
        let end_bin = self.get_bin(end_x).unwrap_or(self.bins.len() - 1);

        let mut bin_counts = Vec::new();

        for bin in start_bin..=end_bin {
            if bin < self.bins.len() {
                bin_counts.push(self.bins[bin] as f64);
            } else {
                break;
            }
        }

        bin_counts
    }

    // get bin counts and bin center at x value
    fn get_bin_count_and_center(&self, x: f64) -> Option<(f64, f64)> {
        let bin = self.get_bin(x);
        if let Some(bin) = bin {
            let bin_center = self.range.0 + (bin as f64 * self.bin_width) + self.bin_width * 0.5;
            let bin_count = self.bins[bin] as f64;
            Some((bin_center, bin_count))
        } else {
            None
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

    // Generate points for the line to form a step histogram.
    fn step_histogram_points(&self) -> Vec<(f64, f64)> {
        let mut line_points: Vec<(f64, f64)> = Vec::new();

        for (index, &count) in self.bins.iter().enumerate() {
            let start = self.range.0 + index as f64 * self.bin_width; // Start of the bin
            let end = start + self.bin_width; // End of the bin

            // Add points for the line at the start and end of each bar
            line_points.push((start, count as f64));
            line_points.push((end, count as f64));
        }

        line_points
    }

    /// Generates legend entries for the histogram based on the specified x range.
    fn legend_entries(&self, start_x: f64, end_x: f64) -> Vec<String> {
        let stats = self.stats(start_x, end_x);
        let integral_text = format!("Integral: {}", stats.0);
        let mean_text = format!("Mean: {:.2}", stats.1);
        let stdev_text = format!("Stdev: {:.2}", stats.2);

        vec![integral_text, mean_text, stdev_text]
    }

    // Generates a line to form a step histogram using egui_plot
    fn egui_histogram_step(&self, color: egui::Color32) -> egui_plot::Line {
        let line_points = self.step_histogram_points();

        // Convert line_points to a Vec<[f64; 2]>
        let plot_points: egui_plot::PlotPoints = line_points.iter().map(|&(x, y)| [x, y]).collect();

        egui_plot::Line::new(plot_points)
            .color(color)
            .name(self.name.clone())
    }

    // Fit the background with a linear line using the background markers
    fn fit_background(&mut self) {
        // remove temp fits
        self.fits.remove_temp_fits();

        // check to see if there are at least two background markers
        if self.plot_settings.markers.background_markers.len() < 2 {
            log::error!("Need to set at least two background markers to fit the histogram");
            return;
        }

        // get the bin centers and counts at the background markers
        let marker_positions = self.plot_settings.markers.background_markers.clone();

        let mut x_data = Vec::new();
        let mut y_data = Vec::new();
        for position in marker_positions.iter() {
            if let Some((bin_center, bin_count)) = self.get_bin_count_and_center(position.clone()) {
                x_data.push(bin_center);
                y_data.push(bin_count);
            }
        }

        let mut background_fitter = BackgroundFitter::new(x_data, y_data, FitModel::Linear);

        background_fitter.fit();

        let temp_background_fit = background_fitter;

        self.fits.temp_background_fit = Some(temp_background_fit);
    }

    // Fit the gaussians at the peak markers
    fn fit_gaussians(&mut self) {
        // Check to see if there are two region markers
        if self.plot_settings.markers.region_markers.len() != 2 {
            log::error!("Need to set two region markers to fit the histogram");
            return;
        }

        // Remove peak markers outside of region and the position
        self.plot_settings
            .markers
            .remove_peak_markers_outside_region();
        let peak_positions = self.plot_settings.markers.peak_markers.clone();

        // Check to see if there is a temp background fit
        if self.fits.temp_background_fit.is_none() {
            // If there are no background fit, perform the background fit
            // If there are 0 or only 1 background marker, set the background markers to the region markers
            if self.plot_settings.markers.background_markers.len() == 0
                || self.plot_settings.markers.background_markers.len() == 1
            {
                self.plot_settings.markers.background_markers =
                    self.plot_settings.markers.region_markers.clone();
            }

            self.fit_background();
        }

        // Create a new Fitter for Gaussian fitting
        let mut fitter = Fitter::new(
            FitModel::Gaussian(peak_positions),
            self.fits.temp_background_fit.clone(),
        );

        // Get the data within the region markers
        let start_x = self.plot_settings.markers.region_markers[0];
        let end_x = self.plot_settings.markers.region_markers[1];
        fitter.x_data = self.get_bin_centers_between(start_x, end_x);
        fitter.y_data = self.get_bin_counts_between(start_x, end_x);

        // Perform the fit
        fitter.fit();

        // get the new peak markers
        let new_peak_markers = fitter.get_peak_markers();

        // Remove the old peak markers
        self.plot_settings.markers.peak_markers.clear();

        // Add the new peak markers
        self.plot_settings.markers.peak_markers = new_peak_markers;

        // Store the temporary fit
        self.fits.temp_fit = Some(fitter);
    }

    // Store the temporary fit
    fn store_fit(&mut self) {
        if let Some(temp_fit) = self.fits.temp_fit.clone() {
            self.fits.stored_fits.push(temp_fit);
            self.fits.temp_fit = None;
        }
    }

    // Handles the interactive elements of the histogram
    fn interactive(&mut self, ui: &mut egui::Ui) {
        /* Keybindings for the histogram
        "P" - Add peak marker
        "R" - Add region marker
        "B" - Add background marker
        "-" - Remove the marker closest to the cursor
        "Delete" - Remove all markers
        "G" - Fit the background
        "F" - Fit gaussians at the peak markers
        "S" - Store the fit
        "I" - Toggle information visibility
         */
        self.plot_settings.markers.cursor_position = self.plot_settings.cursor_position;

        if let Some(_cursor_position) = self.plot_settings.cursor_position {
            // Functions adds the keybindings related to the markers
            self.plot_settings.markers.interactive_markers(ui);

            // remove temp fits with "-" or "Delete"
            if ui.input(|i| i.key_pressed(egui::Key::Minus) || i.key_pressed(egui::Key::Delete)) {
                self.fits.remove_temp_fits();
            }

            // Fit the background using "G"
            if ui.input(|i| i.key_pressed(egui::Key::G)) {
                self.fit_background();
            }

            // Fit gaussians at the peak markers with "F"
            if ui.input(|i| i.key_pressed(egui::Key::F)) {
                self.fit_gaussians();
            }

            // Store the fit with "S"
            if ui.input(|i| i.key_pressed(egui::Key::S)) {
                self.store_fit();
            }

            // change the information visibility boolean with "I"
            if ui.input(|i| i.key_pressed(egui::Key::I)) {
                self.plot_settings.info = !self.plot_settings.info;
            }
        }
    }

    // Renders the histogram using egui_plot
    pub fn render(&mut self, ui: &mut egui::Ui) {
        /* For custom 2d histogram plot manipulation settings*/
        // let (scroll, pointer_down, modifiers) = ui.input(|i| {
        //     let scroll = i.events.iter().find_map(|e| match e {
        //         egui::Event::MouseWheel { delta, .. } => Some(*delta),
        //         _ => None,
        //     });
        //     (scroll, i.pointer.primary_down(), i.modifiers)
        // });

        let plot = egui_plot::Plot::new(self.name.clone())
            .legend(egui_plot::Legend::default())
            .show_x(self.plot_settings.show_x_value)
            .show_y(self.plot_settings.show_y_value)
            .center_x_axis(self.plot_settings.center_x_axis)
            .center_y_axis(self.plot_settings.center_y_axis)
            .allow_zoom(self.plot_settings.allow_zoom)
            .allow_boxed_zoom(self.plot_settings.allow_boxed_zoom)
            .allow_drag(self.plot_settings.allow_drag)
            .allow_scroll(self.plot_settings.allow_scroll)
            .clamp_grid(self.plot_settings.clamp_grid)
            .show_grid(self.plot_settings.show_grid)
            .sharp_grid_lines(self.plot_settings.sharp_grid_lines)
            .show_background(self.plot_settings.show_background)
            .auto_bounds(egui::Vec2b::new(true, true));

        self.interactive(ui);

        plot.show(ui, |plot_ui| {
            // custom_plot_manipulation(plot_ui, scroll, pointer_down, modifiers);

            let plot_min_x = plot_ui.plot_bounds().min()[0];
            let plot_max_x = plot_ui.plot_bounds().max()[0];

            let step_line = self.egui_histogram_step(self.plot_settings.color);

            if self.plot_settings.info {
                let stats_entries = self.legend_entries(plot_min_x, plot_max_x);
                for entry in stats_entries.iter() {
                    plot_ui.text(
                        egui_plot::Text::new(egui_plot::PlotPoint::new(0, 0), " ") // Placeholder for positioning; adjust as needed
                            .highlight(false)
                            .color(self.plot_settings.color)
                            .name(entry),
                    );
                }
            }

            plot_ui.line(step_line);

            if plot_ui.response().hovered() {
                self.plot_settings.cursor_position = plot_ui.pointer_coordinate();
            } else {
                self.plot_settings.cursor_position = None;
            }

            self.plot_settings.markers.draw_markers(plot_ui);

            self.fits.draw(plot_ui);
        })
        .response
        .context_menu(|ui| {
            self.plot_settings.settings_ui(ui);
        });
    }
}

/*
fn custom_plot_manipulation(
    plot_ui: &mut egui_plot::PlotUi,
    scroll: Option<egui::Vec2>,
    pointer_down: bool,
    modifiers: egui::Modifiers,
) {
    /* For custom plot manipulation settings, add this before the plot.show()
        let (scroll, pointer_down, modifiers) = ui.input(|i| {
            let scroll = i.events.iter().find_map(|e| match e {
                egui::Event::MouseWheel { delta, .. } => Some(*delta),
                _ => None,
            });
            (scroll, i.pointer.primary_down(), i.modifiers)
        });
    */

    if plot_ui.response().hovered() {
        if let Some(mut scroll) = scroll {
            // Default behavior for zooming and panning, with fixed parameters
            let lock_x = false;
            let lock_y = false;
            let zoom_speed = 0.1; // Default zoom speed
            let scroll_speed = 1.0; // Default scroll speed
            let ctrl_to_zoom = false;
            let shift_to_horizontal = false;

            if modifiers.ctrl == ctrl_to_zoom {
                scroll = egui::Vec2::splat(scroll.x + scroll.y);
                let mut zoom_factor = egui::Vec2::from([
                    (scroll.x * zoom_speed / 15.0).exp(),
                    (scroll.y * zoom_speed / 15.0).exp(),
                ]);
                if lock_x {
                    zoom_factor.x = 1.0;
                }
                if lock_y {
                    zoom_factor.y = 1.0;
                }
                plot_ui.zoom_bounds_around_hovered(zoom_factor);
            } else {
                if modifiers.shift == shift_to_horizontal {
                    scroll = egui::Vec2::new(scroll.y, scroll.x);
                }
                if lock_x {
                    scroll.x = 0.0;
                }
                if lock_y {
                    scroll.y = 0.0;
                }
                let delta_pos = scroll_speed * scroll;
                plot_ui.translate_bounds(delta_pos);
            }
        }

        if pointer_down {
            let pointer_translate = -plot_ui.pointer_coordinate_drag_delta();
            // Lock axis functionality removed for simplification, add if needed
            plot_ui.translate_bounds(pointer_translate);
        }
    }
}
*/
