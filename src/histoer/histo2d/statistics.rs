use super::histogram2d::Histogram2D;

impl Histogram2D {
    // Calculate statistics for a given range (Integral, Mean X, Stdev X, Mean Y, Stdev Y)
    pub fn get_statistics(
        &self,
        start_x: f64,
        end_x: f64,
        start_y: f64,
        end_y: f64,
    ) -> (u64, f64, f64, f64, f64) {
        let start_x_index = self.get_bin_index_x(start_x).unwrap_or(0);
        let end_x_index = self.get_bin_index_x(end_x).unwrap_or_else(|| {
            self.bins
                .counts
                .keys()
                .max_by_key(|k| k.0)
                .map_or(0, |k| k.0)
        });

        let start_y_index = self.get_bin_index_y(start_y).unwrap_or(0);
        let end_y_index = self.get_bin_index_y(end_y).unwrap_or_else(|| {
            self.bins
                .counts
                .keys()
                .max_by_key(|k| k.1)
                .map_or(0, |k| k.1)
        });

        let mut total_count = 0;

        let mut sum_product_x = 0.0;
        let mut sum_product_y = 0.0;

        for (&(x_index, y_index), &count) in self.bins.counts.iter() {
            if x_index >= start_x_index
                && x_index <= end_x_index
                && y_index >= start_y_index
                && y_index <= end_y_index
            {
                let bin_center_x = self.range.x.min
                    + (x_index as f64 * self.bins.x_width)
                    + self.bins.x_width * 0.5;
                let bin_center_y = self.range.y.min
                    + (y_index as f64 * self.bins.y_width)
                    + self.bins.y_width * 0.5;

                total_count += count;

                sum_product_x += count as f64 * bin_center_x;
                sum_product_y += count as f64 * bin_center_y;
            }
        }

        if total_count == 0 {
            (0, 0.0, 0.0, 0.0, 0.0)
        } else {
            let mean_x = sum_product_x / total_count as f64;
            let mean_y = sum_product_y / total_count as f64;

            let mut sum_squared_diff_x = 0.0;
            let mut sum_squared_diff_y = 0.0;

            for (&(x_index, y_index), &count) in self.bins.counts.iter() {
                if x_index >= start_x_index
                    && x_index <= end_x_index
                    && y_index >= start_y_index
                    && y_index <= end_y_index
                {
                    let bin_center_x = self.range.x.min
                        + (x_index as f64 * self.bins.x_width)
                        + self.bins.x_width * 0.5;
                    let bin_center_y = self.range.y.min
                        + (y_index as f64 * self.bins.y_width)
                        + self.bins.y_width * 0.5;

                    let diff_x = bin_center_x - mean_x;
                    let diff_y = bin_center_y - mean_y;

                    sum_squared_diff_x += count as f64 * diff_x * diff_x;
                    sum_squared_diff_y += count as f64 * diff_y * diff_y;
                }
            }

            let stdev_x = (sum_squared_diff_x / total_count as f64).sqrt();
            let stdev_y = (sum_squared_diff_y / total_count as f64).sqrt();

            (total_count, mean_x, stdev_x, mean_y, stdev_y)
        }
    }

    // Show statistics on the plot
    pub fn show_stats(&self, plot_ui: &mut egui_plot::PlotUi) {
        if !self.plot_settings.stats_info {
            return;
        }

        let plot_min_x = plot_ui.plot_bounds().min()[0];
        let plot_max_x = plot_ui.plot_bounds().max()[0];
        let plot_min_y = plot_ui.plot_bounds().min()[1];
        let plot_max_y = plot_ui.plot_bounds().max()[1];

        let stats = self.get_statistics(plot_min_x, plot_max_x, plot_min_y, plot_max_y);

        let stats_entries = [
            format!("Integral: {}", stats.0),
            format!("Mean: ({:.2}, {:.2})", stats.1, stats.3),
            format!("Stdev: ({:.2}, {:.2})", stats.2, stats.4),
            format!("Overflow: ({:}, {:})", self.overflow.0, self.overflow.1),
            format!("Underflow: ({:}, {:})", self.underflow.0, self.underflow.1),
        ];

        for entry in stats_entries.iter() {
            plot_ui.text(
                egui_plot::Text::new(egui_plot::PlotPoint::new(0, 0), " ") // Placeholder for positioning; adjust as needed
                    .highlight(false)
                    .name(entry),
            );
        }
    }
}
