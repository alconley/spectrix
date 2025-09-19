use super::histogram1d::Histogram;

impl Histogram {
    // Compute the possible rebin factors based on the initial number of bins
    pub fn possible_rebin_factors(&self) -> Vec<usize> {
        let mut factors = vec![];
        factors.push(1);
        let mut factor = 1;
        while self.original_bins.len().is_multiple_of(factor * 2) {
            factor *= 2;
            factors.push(factor);
        }

        // remove the last factor if it is the same as the number of bins
        if factors.last() == Some(&self.original_bins.len()) {
            factors.pop();
        }
        factors
    }

    // Rebin the histogram according to the rebin factor
    pub fn rebin(&mut self) {
        let rebin_factor = self.plot_settings.rebin_factor;
        let new_bin_count = self.original_bins.len() / rebin_factor;
        let mut new_bins = vec![0; new_bin_count];

        for (i, &count) in self.original_bins.iter().enumerate() {
            let new_index = i / rebin_factor;
            new_bins[new_index] += count;
        }

        self.bins = new_bins;
        self.bin_width = (self.range.1 - self.range.0) / new_bin_count as f64;
        self.update_line_points();
    }
}
