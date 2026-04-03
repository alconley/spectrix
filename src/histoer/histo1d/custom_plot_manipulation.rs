use super::histogram1d::Histogram;

impl Histogram {
    fn visible_raw_x_bounds(&self, x_min: f64, x_max: f64) -> (f64, f64) {
        self.display_x_bounds_to_raw_bounds(x_min, x_max)
    }

    fn max_bin_value_in_raw_range(&self, raw_x_min: f64, raw_x_max: f64) -> u64 {
        if self.bins.is_empty() {
            return 0;
        }

        let clamped_x_min = raw_x_min.max(self.range.0);
        let clamped_x_max = raw_x_max.min(self.range.1);

        if clamped_x_min > clamped_x_max {
            return 0;
        }

        let start_bin = self.get_bin_index(clamped_x_min).unwrap_or(0);
        let end_bin = self
            .get_bin_index(clamped_x_max)
            .unwrap_or(self.bins.len().saturating_sub(1));

        self.bins[start_bin..=end_bin]
            .iter()
            .copied()
            .max()
            .unwrap_or(0)
    }

    fn auto_fit_y_bounds(&self, raw_x_min: f64, raw_x_max: f64) -> (f64, f64) {
        let y_min = if self.plot_settings.egui_settings.log_y {
            0.01
        } else {
            0.0
        };

        let visible_max = self.max_bin_value_in_raw_range(raw_x_min, raw_x_max) as f64;
        let base_top_multiplier = if self.plot_settings.egui_settings.log_y {
            self.plot_settings.auto_fit_y_max_multiplier_log.max(1.0)
        } else {
            self.plot_settings.auto_fit_y_max_multiplier_linear.max(1.0)
        };
        let label_size = f64::from(self.fits.settings.uuid_label_size.clamp(8.0, 32.0));
        let label_lift = f64::from(self.fits.settings.uuid_label_lift.max(0.0));
        let log_top_multiplier = if self.fits.has_uuid_labels() {
            1.10 + label_size * 0.06 * label_lift.max(0.25)
        } else {
            base_top_multiplier
        };
        let padded_top = if self.plot_settings.egui_settings.log_y {
            (visible_max * base_top_multiplier.max(log_top_multiplier)).max(1.0)
        } else {
            (visible_max * base_top_multiplier).max(1.0)
        };

        let y_max = if self.plot_settings.egui_settings.log_y {
            padded_top.max(1.1).log10().max(y_min + 0.01)
        } else {
            padded_top
        };

        (y_min, y_max)
    }

    pub fn limit_scrolling(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        let plot_bounds = plot_ui.plot_bounds();

        let range = if let Some(calibration) = self.display_calibration() {
            calibration
                .display_bounds_for_raw_range(self.range)
                .unwrap_or(self.range)
        } else {
            self.range
        };

        let current_x_min = plot_bounds.min()[0];
        let current_x_max = plot_bounds.max()[0];
        let current_y_min = plot_bounds.min()[1];
        let current_y_max = plot_bounds.max()[1];

        let y_max = self.bins.iter().max().copied().unwrap_or(0) as f64;
        let y_min = self.bins.iter().min().copied().unwrap_or(0) as f64;

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
            let (default_y_min, default_y_max) = if self.plot_settings.auto_fit_y_to_visible_range {
                self.auto_fit_y_bounds(self.range.0, self.range.1)
            } else {
                (y_min, y_max)
            };

            let default_bounds = egui_plot::PlotBounds::from_min_max(
                [range.0, default_y_min],
                [range.1, default_y_max],
            );

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

        let (new_y_min, new_y_max) = if self.plot_settings.auto_fit_y_to_visible_range {
            let (raw_x_min, raw_x_max) = self.visible_raw_x_bounds(new_x_min, new_x_max);
            self.auto_fit_y_bounds(raw_x_min, raw_x_max)
        } else {
            (new_y_min, new_y_max)
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
