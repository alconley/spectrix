#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Histogram {
    pub bins: Vec<u32>,
    pub range: (f64, f64),
    pub bin_width: f64,
}

impl Histogram {
    // Create a new Histogram with specified min, max, and number of bins
    pub fn new(number_of_bins: usize, range: (f64, f64)) -> Self {
        Histogram {
            bins: vec![0; number_of_bins],
            range,
            bin_width: (range.1 - range.0) / number_of_bins as f64,
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

    pub fn stats(&self, start_x: f64, end_x: f64) -> (u32, f64, f64) {
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

    pub fn step_histogram_points(&self) -> Vec<(f64, f64)> {
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
    pub fn legend_entries(&self, start_x: f64, end_x: f64) -> Vec<String> {
        let stats = self.stats(start_x, end_x);
        let integral_text = format!("Integral: {}", stats.0);
        let mean_text = format!("Mean: {:.2}", stats.1);
        let stdev_text = format!("Stdev: {:.2}", stats.2);

        vec![integral_text, mean_text, stdev_text]
    }

    // get the bin centers
    // pub fn bin_centers(&self) -> Vec<f64> {
    //     let mut centers = Vec::new();
    //     for (index, _) in self.bins.iter().enumerate() {
    //         let center = self.range.0 + (index as f64 * self.bin_width) + (self.bin_width * 0.5);
    //         centers.push(center);
    //     }
    //     centers
    // }
}
