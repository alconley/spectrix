use super::histogram1d::Histogram;

impl Histogram {
    pub fn limit_scrolling(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        let plot_bounds = plot_ui.plot_bounds();

        let range = if self.fits.settings.calibrated {
            let range_1 = self.fits.calibration.calibrate(self.range.0);
            let range_2 = self.fits.calibration.calibrate(self.range.1);
            (range_1.min(range_2), range_1.max(range_2))
        } else {
            self.range
        };

        let current_x_min = plot_bounds.min()[0];
        let current_x_max = plot_bounds.max()[0];
        let current_y_min = plot_bounds.min()[1];
        let current_y_max = plot_bounds.max()[1];

        let y_max = self.bins.iter().max().cloned().unwrap_or(0) as f64;
        let y_min = self.bins.iter().min().cloned().unwrap_or(0) as f64;

        // account for log y
        let y_min = if self.plot_settings.egui_settings.log_y {
            0.01
        } else {
            y_min
        };

        let y_max = if self.plot_settings.egui_settings.log_y {
            y_max.log10().max(0.0001)
        } else {
            y_max
        };

        if current_x_min == -1.0
            && current_x_max == 1.0
            && current_y_min == 0.0
            && current_y_max == 1.0
        {
            let default_bounds =
                egui_plot::PlotBounds::from_min_max([range.0, y_min], [range.1, y_max]);

            plot_ui.set_plot_bounds(default_bounds);
            return;
        }

        // Clamping bounds only for scrolling
        let new_x_min = current_x_min.max(range.0 * 1.1);
        let new_x_max = current_x_max.min(range.1 * 1.1);
        let new_y_min = current_y_min.max(y_min * 1.1);
        let new_y_max = current_y_max.min(y_max * 1.1);

        let new_y_min = if self.plot_settings.egui_settings.log_y {
            0.01
        } else {
            new_y_min
        };

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

    pub fn custom_plot_manipulation_update(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        if self.plot_settings.egui_settings.reset_axis {
            self.plot_settings.egui_settings.reset_axis_lims(plot_ui);
            self.plot_settings.egui_settings.reset_axis = false;
        } else {
            self.limit_scrolling(plot_ui);
        }
    }
}
