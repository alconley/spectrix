use fnv::FnvHashMap;

// Define the BarData struct
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BarData {
    pub x: f64,
    pub y: f64,
    pub bar_width: f64,
    pub height: f64,
    pub count: u32,
}

// uses a hash map to store the histogram data (zero overhead for empty bins)
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Histogram2D {
    pub bins: FnvHashMap<(usize, usize), u32>,
    pub x_range: (f64, f64),
    pub y_range: (f64, f64),
    pub x_bin_width: f64,
    pub y_bin_width: f64,
    pub min_count: u32,
    pub max_count: u32,
}

impl Histogram2D {
    // Create a new 2D Histogram with specified ranges and number of bins for each axis
    pub fn new(x_bins: usize, x_range: (f64, f64), y_bins: usize, y_range: (f64, f64)) -> Self {
        Histogram2D {
            bins: FnvHashMap::default(),
            x_range,
            y_range,
            x_bin_width: (x_range.1 - x_range.0) / x_bins as f64,
            y_bin_width: (y_range.1 - y_range.0) / y_bins as f64,
            min_count: u32::MAX,
            max_count: u32::MIN,
        }
    }

    // Add a value to the histogram
    pub fn fill(&mut self, x_value: f64, y_value: f64) {
        if x_value >= self.x_range.0
            && x_value < self.x_range.1
            && y_value >= self.y_range.0
            && y_value < self.y_range.1
        {
            let x_index = ((x_value - self.x_range.0) / self.x_bin_width) as usize;
            let y_index = ((y_value - self.y_range.0) / self.y_bin_width) as usize;
            let count = self.bins.entry((x_index, y_index)).or_insert(0);
            *count += 1;

            // Update min and max counts
            if *count < self.min_count {
                self.min_count = *count;
            }
            if *count > self.max_count {
                self.max_count = *count;
            }
        }
    }

    // Method to generate data for egui heatmap
    pub fn generate_bar_data(&self) -> Vec<BarData> {
        let mut bars = Vec::new();

        for (&(x_index, y_index), &count) in &self.bins {
            if count == 0 {
                continue; // Skip empty bins
            }

            let x_bin_start = self.x_range.0 + x_index as f64 * self.x_bin_width;
            let x_bin_end = x_bin_start + self.x_bin_width;
            let y_bin_start = self.y_range.0 + y_index as f64 * self.y_bin_width;
            let y_bin_end = y_bin_start + self.y_bin_width;

            bars.push(BarData {
                x: (x_bin_start + x_bin_end) / 2.0,
                y: (y_bin_start + y_bin_end) / 2.0,
                bar_width: self.x_bin_width,
                height: self.y_bin_width,
                count,
            });
        }

        bars
    }

    fn get_bin_x(&self, x: f64) -> Option<usize> {
        if x < self.x_range.0 || x > self.x_range.1 {
            return None;
        }

        let bin_index: usize = ((x - self.x_range.0) / self.x_bin_width).floor() as usize;

        Some(bin_index)
    }

    fn get_bin_y(&self, y: f64) -> Option<usize> {
        if y < self.y_range.0 || y > self.y_range.1 {
            return None;
        }

        let bin_index: usize = ((y - self.y_range.0) / self.y_bin_width).floor() as usize;

        Some(bin_index)
    }

    pub fn stats(
        &self,
        start_x: f64,
        end_x: f64,
        start_y: f64,
        end_y: f64,
    ) -> (u32, f64, f64, f64, f64) {
        let start_x_index = self.get_bin_x(start_x).unwrap_or(0);
        let end_x_index = self
            .get_bin_x(end_x)
            .unwrap_or_else(|| self.bins.keys().max_by_key(|k| k.0).map_or(0, |k| k.0));

        let start_y_index = self.get_bin_y(start_y).unwrap_or(0);
        let end_y_index = self
            .get_bin_y(end_y)
            .unwrap_or_else(|| self.bins.keys().max_by_key(|k| k.1).map_or(0, |k| k.1));

        let mut total_count = 0;

        let mut sum_product_x = 0.0;
        let mut sum_product_y = 0.0;

        for (&(x_index, y_index), &count) in self.bins.iter() {
            if x_index >= start_x_index
                && x_index <= end_x_index
                && y_index >= start_y_index
                && y_index <= end_y_index
            {
                let bin_center_x =
                    self.x_range.0 + (x_index as f64 * self.x_bin_width) + self.x_bin_width * 0.5;
                let bin_center_y =
                    self.y_range.0 + (y_index as f64 * self.y_bin_width) + self.y_bin_width * 0.5;

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

            for (&(x_index, y_index), &count) in self.bins.iter() {
                if x_index >= start_x_index
                    && x_index <= end_x_index
                    && y_index >= start_y_index
                    && y_index <= end_y_index
                {
                    let bin_center_x = self.x_range.0
                        + (x_index as f64 * self.x_bin_width)
                        + self.x_bin_width * 0.5;
                    let bin_center_y = self.y_range.0
                        + (y_index as f64 * self.y_bin_width)
                        + self.y_bin_width * 0.5;

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

    /// Generates legend entries for the histogram based on the specified x range.
    pub fn legend_entries(
        &self,
        start_x: f64,
        end_x: f64,
        start_y: f64,
        end_y: f64,
    ) -> Vec<String> {
        let stats = self.stats(start_x, end_x, start_y, end_y);
        let integral_text = format!("Integral: {}", stats.0);
        let mean_x_text = format!("Mean X: {:.2}", stats.1);
        let stdev_x_text = format!("Stdev X: {:.2}", stats.2);
        let mean_y_text = format!("Mean Y: {:.2}", stats.3);
        let stdev_y_text = format!("Stdev Y: {:.2}", stats.4);

        vec![
            integral_text,
            mean_x_text,
            stdev_x_text,
            mean_y_text,
            stdev_y_text,
        ]
    }
}
