use super::plot_settings::PlotSettings;
use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::fitter::fit_handler::Fits;

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
        Self {
            name: name.to_owned(),
            bins: vec![0; number_of_bins],
            range,
            overflow: 0,
            underflow: 0,
            bin_width: (range.1 - range.0) / number_of_bins as f64,
            line: EguiLine {
                name: name.to_owned(),
                ..Default::default()
            },
            plot_settings: PlotSettings::default(),
            fits: Fits::new(),
            original_bins: vec![0; number_of_bins],
        }
    }

    pub fn reset(&mut self) {
        self.bins = vec![0; self.original_bins.len()];
        self.original_bins = vec![0; self.original_bins.len()];
        self.plot_settings.rebin_factor = 1;
        self.bin_width = (self.range.1 - self.range.0) / self.bins.len() as f64;
        self.overflow = 0;
        self.underflow = 0;
    }

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

    pub(crate) fn display_x_to_raw_x(&self, display_x: f64) -> f64 {
        let linear_display_x = if self.plot_settings.egui_settings.log_x {
            10_f64.powf(display_x)
        } else {
            display_x
        };

        if self.fits.settings.calibrated {
            self.fits
                .calibration
                .invert(linear_display_x)
                .unwrap_or(linear_display_x)
        } else {
            linear_display_x
        }
    }

    pub(crate) fn display_x_bounds_to_raw_bounds(&self, x_min: f64, x_max: f64) -> (f64, f64) {
        let raw_x_min = self.display_x_to_raw_x(x_min);
        let raw_x_max = self.display_x_to_raw_x(x_max);

        if raw_x_min <= raw_x_max {
            (raw_x_min, raw_x_max)
        } else {
            (raw_x_max, raw_x_min)
        }
    }

    fn current_raw_x_bounds(&self, plot_ui: &egui_plot::PlotUi<'_>) -> (f64, f64) {
        let plot_bounds = plot_ui.plot_bounds();
        self.display_x_bounds_to_raw_bounds(plot_bounds.min()[0], plot_bounds.max()[0])
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        // update the histogram and fit lines with the log setting and draw
        let log_y = self.plot_settings.egui_settings.log_y;
        let log_x = self.plot_settings.egui_settings.log_x;

        self.line.log_y = log_y;
        self.line.log_x = log_x;
        let calibration = if self.fits.settings.calibrated {
            Some(self.fits.calibration.clone())
        } else {
            None
        };
        let calibration_ref = calibration.as_ref();

        self.line.draw(plot_ui, calibration_ref);
        self.plot_settings
            .markers
            .draw_all_markers(plot_ui, calibration_ref);

        self.fits.set_log(log_y, log_x);
        self.fits
            .draw(plot_ui, &self.bins, self.range, self.bin_width);
        self.show_stats(plot_ui);

        self.update_background_pair_lines();
        for bg_pair in &mut self.plot_settings.markers.background_markers {
            bg_pair.histogram_line.log_x = log_x;
            bg_pair.histogram_line.log_y = log_y;
        }

        if plot_ui.response().hovered() {
            self.plot_settings.cursor_position = plot_ui.pointer_coordinate();
            self.plot_settings.egui_settings.limit_scrolling = true;
        } else {
            self.plot_settings.cursor_position = None;
        }

        self.plot_settings.draw(plot_ui, calibration_ref);

        self.custom_plot_manipulation_update(plot_ui);
        self.plot_settings.current_plot_bounds = Some(self.current_raw_x_bounds(plot_ui));

        // self.plot_settings.egui_settings.y_label = format!("Counts/{:.}", self.bin_width);
    }

    pub fn draw_other_histograms(
        &mut self,
        plot_ui: &mut egui_plot::PlotUi<'_>,
        histograms: &[Self],
    ) {
        for histogram in histograms {
            let mut hist = histogram.clone();
            hist.draw(plot_ui);
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        if ui.visuals().dark_mode {
            self.line.set_color(egui::Color32::LIGHT_BLUE);
        } else {
            self.line.set_color(egui::Color32::BLACK);
        }

        self.update_line_points();
        self.keybinds(ui);

        self.fits.ui(ui, &self.name);
        self.apply_refit_all_request();
        self.apply_modify_fit_request();

        let width = ui.available_width();
        let mut plot = egui_plot::Plot::new(self.name.clone()).width(width);
        plot = self.plot_settings.egui_settings.apply_to_plot(plot);

        let (scroll, _pointer_down, _modifiers) = ui.input(|i| {
            let scroll = i.events.iter().find_map(|e| match e {
                egui::Event::MouseWheel { delta, .. } => Some(*delta),
                _ => None,
            });
            (scroll, i.pointer.primary_down(), i.modifiers)
        });

        let plot_response = plot.show(ui, |plot_ui| {
            self.draw(plot_ui);

            if self.plot_settings.progress.is_some()
                && !self.plot_settings.auto_fit_y_to_visible_range
            {
                let y_max = self.bins.iter().max().copied().unwrap_or(0) as f64;
                let mut plot_bounds = plot_ui.plot_bounds();
                plot_bounds.extend_with_y(y_max * 1.1);
                plot_ui.set_plot_bounds(plot_bounds);
            }

            if self.plot_settings.egui_settings.reset_axis {
                plot_ui.auto_bounds();
                self.plot_settings.egui_settings.reset_axis = false;
            }

            if self.plot_settings.cursor_position.is_some()
                && let Some(delta_pos) = scroll
            {
                let zoom_factor = if delta_pos.y > 0.0 || delta_pos.x > 0.0 {
                    1.1
                } else {
                    0.9
                };
                plot_ui.zoom_bounds_around_hovered(egui::Vec2::new(zoom_factor, 1.0));
            }
        });

        plot_response.response.context_menu(|ui| {
            self.context_menu(ui);
        });

        let calibration = {
            if self.fits.settings.calibrated {
                Some(&self.fits.calibration)
            } else {
                None
            }
        };

        self.plot_settings
            .interactive_response(&plot_response, calibration);
    }
}
