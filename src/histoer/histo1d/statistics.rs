use super::histogram1d::Histogram;

impl Histogram {
    // Calculate the statistics for the histogram within the specified x range.
    pub fn get_statistics(&self, start_x: f64, end_x: f64) -> (u64, f64, f64) {
        let start_bin = self.get_bin_index(start_x).unwrap_or(0);
        let end_bin = self.get_bin_index(end_x).unwrap_or(self.bins.len() - 1);

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
    pub fn show_stats(&self, plot_ui: &mut egui_plot::PlotUi) {
        if self.plot_settings.stats_info {
            let plot_min_x = plot_ui.plot_bounds().min()[0];
            let plot_max_x = plot_ui.plot_bounds().max()[0];

            let (integral, mean, stdev) = self.get_statistics(plot_min_x, plot_max_x);
            let stats_entries = [
                format!("Integral: {}", integral),
                format!("Mean: {:.2}", mean),
                format!("Stdev: {:.2}", stdev),
                // format!("Overflow: {:}", self.overflow),
                // format!("Underflow: {:}", self.underflow),
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
}
