use crate::fitter::egui_markers::EguiFitMarkers;
use crate::fitter::fitter::{BackgroundFitter, FitModel, Fits, Fitter};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotSettings {
    #[serde(skip)]
    cursor_position: Option<egui_plot::PlotPoint>,
    info: bool,
    show_fit_stats: bool,
    color: egui::Color32,
    show_color_changer: bool,
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
            show_fit_stats: true,
            color: egui::Color32::LIGHT_BLUE,
            show_color_changer: false,
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
            ui.checkbox(&mut self.show_fit_stats, "Show Fit Stats");
            ui.menu_button("Manipulation Settings", |ui| {
                ui.vertical(|ui| {
                    ui.checkbox(&mut self.show_color_changer, "Show Color Changer");
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
        });
    }

    pub fn above_histo_ui(&mut self, ui: &mut egui::Ui) {
        if self.show_color_changer {
            ui.horizontal(|ui| {
                ui.label("Color: ");
                ui.color_edit_button_srgba(&mut self.color);
            });
        }
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
    fn get_bin(&self, x: f64) -> Option<usize> {
        if x < self.range.0 || x > self.range.1 {
            return None;
        }

        let bin_index: usize = ((x - self.range.0) / self.bin_width).floor() as usize;

        Some(bin_index)
    }

    // Get the bin centers for the histogram
    fn get_bin_centers(&self) -> Vec<f64> {
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

    // Generate points for the line to form a step histogram.
    fn step_histogram_points(&self) -> Vec<(f64, f64)> {
        self.bins
            .iter()
            .enumerate()
            .flat_map(|(index, &count)| {
                let start = self.range.0 + index as f64 * self.bin_width;
                let end = start + self.bin_width;
                vec![(start, count as f64), (end, count as f64)]
            })
            .collect()
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

    // Generates a line to form a step histogram using egui_plot
    fn egui_histogram_step(&self, color: egui::Color32) -> egui_plot::Line {
        let plot_points: egui_plot::PlotPoints = self
            .step_histogram_points()
            .iter()
            .map(|&(x, y)| [x, y])
            .collect();
        egui_plot::Line::new(plot_points)
            .color(color)
            .name(self.name.clone())
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
                self.plot_settings.markers.background_markers =
                    self.plot_settings.markers.region_markers.clone();
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

    // Store the temporary fit
    fn store_fit(&mut self) {
        if let Some(temp_fit) = self.fits.temp_fit.take() {
            self.fits.stored_fits.push(temp_fit);
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
                self.store_fit();
            }

            if ui.input(|i| i.key_pressed(egui::Key::I)) {
                self.plot_settings.info = !self.plot_settings.info;
            }

            if ui.input(|i| i.key_pressed(egui::Key::A)) {
                let total_count = self.sum_counts_between_region_markers();
                log::info!("Total count between region markers: {}", total_count);
            }
        }
    }

    // Renders the histogram using egui_plot
    pub fn render(&mut self, ui: &mut egui::Ui) {
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

        ui.vertical(|ui| {
            self.plot_settings.above_histo_ui(ui);

            if self.plot_settings.show_fit_stats {
                self.fits.fit_stats_grid_ui(ui);
            }

            plot.show(ui, |plot_ui| {
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
        });
    }
}
