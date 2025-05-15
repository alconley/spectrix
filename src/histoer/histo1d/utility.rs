use super::histogram1d::Histogram;

impl Histogram {
    pub fn get_bin_edges(&self) -> Vec<f64> {
        (0..=self.bins.len())
            .map(|i| self.range.0 + i as f64 * self.bin_width)
            .collect()
    }

    pub fn get_bin_centers(&self) -> Vec<f64> {
        (0..self.bins.len())
            .map(|i| self.range.0 + (i as f64 + 0.5) * self.bin_width)
            .collect()
    }

    pub fn get_bin_index(&self, x: f64) -> Option<usize> {
        if x < self.range.0 || x > self.range.1 {
            return None;
        }

        let bin_index: usize = ((x - self.range.0) / self.bin_width).floor() as usize;

        Some(bin_index)
    }

    pub fn get_bin_centers_between(&self, start_x: f64, end_x: f64) -> Vec<f64> {
        let start_bin = self.get_bin_index(start_x).unwrap_or(0);
        let end_bin = self.get_bin_index(end_x).unwrap_or(self.bins.len() - 1);

        (start_bin..=end_bin)
            .map(|bin| self.range.0 + (bin as f64 * self.bin_width) + self.bin_width * 0.5)
            .collect()
    }

    pub fn get_bin_counts_between(&self, start_x: f64, end_x: f64) -> Vec<f64> {
        let start_bin = self.get_bin_index(start_x).unwrap_or(0);
        let end_bin = self.get_bin_index(end_x).unwrap_or(self.bins.len() - 1);

        (start_bin..=end_bin)
            .map(|bin| self.bins[bin] as f64)
            .collect()
    }

    pub fn get_bin_count_and_center(&self, x: f64) -> Option<(f64, f64)> {
        self.get_bin_index(x).map(|bin| {
            let bin_center = self.range.0 + (bin as f64 * self.bin_width) + self.bin_width * 0.5;
            let bin_count = self.bins[bin] as f64;
            (bin_center, bin_count)
        })
    }
}
