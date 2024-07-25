use super::histogram2d::{Bins, Histogram2D};

use fnv::FnvHashMap;

impl Histogram2D {
    // Compute the possible rebin factors based on the initial number of bins
    pub fn possible_x_rebin_factors(&self) -> Vec<usize> {
        let mut factors = vec![];
        factors.push(1);
        let mut factor = 1;

        if self.backup_bins.is_none() {
            while self.bins.x % (factor * 2) == 0 {
                factor *= 2;
                factors.push(factor);
            }

            // remove the last factor if it is the same as the number of bins
            if factors.last() == Some(&self.bins.x) {
                factors.pop();
            }
            factors
        } else {
            while self.backup_bins.as_ref().unwrap().x % (factor * 2) == 0 {
                factor *= 2;
                factors.push(factor);
            }

            // remove the last factor if it is the same as the number of bins
            if factors.last() == Some(&self.backup_bins.as_ref().unwrap().x) {
                factors.pop();
            }
            factors
        }
    }

    // Compute the possible rebin factors based on the initial number of bins
    pub fn possible_y_rebin_factors(&self) -> Vec<usize> {
        let mut factors = vec![];
        factors.push(1);
        let mut factor = 1;

        if self.backup_bins.is_none() {
            while self.bins.y % (factor * 2) == 0 {
                factor *= 2;
                factors.push(factor);
            }

            // remove the last factor if it is the same as the number of bins
            if factors.last() == Some(&self.bins.y) {
                factors.pop();
            }
            factors
        } else {
            while self.backup_bins.as_ref().unwrap().y % (factor * 2) == 0 {
                factor *= 2;
                factors.push(factor);
            }

            // remove the last factor if it is the same as the number of bins
            if factors.last() == Some(&self.backup_bins.as_ref().unwrap().y) {
                factors.pop();
            }
            factors
        }
    }
    // Rebin the histogram with new bin sizes
    pub fn rebin(&mut self) {
        // Backup the original bins
        if self.backup_bins.is_none() {
            self.backup_bins = Some(self.bins.clone());
        }

        let x_rebin_factor = self.plot_settings.rebin_x_factor;
        let y_rebin_factor = self.plot_settings.rebin_y_factor;

        if let Some(backup_bins) = &self.backup_bins {
            let new_x_bins = (backup_bins.x as f64 / x_rebin_factor as f64).ceil() as usize;
            let new_y_bins = (backup_bins.y as f64 / y_rebin_factor as f64).ceil() as usize;

            // Create new binning structure
            let mut new_bins = Bins {
                x: new_x_bins,
                x_width: (self.range.x.max - self.range.x.min) / new_x_bins as f64,
                y: new_y_bins,
                y_width: (self.range.y.max - self.range.y.min) / new_y_bins as f64,
                counts: FnvHashMap::default(),
                min_count: u64::MAX,
                max_count: u64::MIN,
            };

            // Transfer counts to new bins
            for ((old_x_index, old_y_index), &count) in &backup_bins.counts {
                let old_x_value = self.range.x.min + *old_x_index as f64 * backup_bins.x_width;
                let old_y_value = self.range.y.min + *old_y_index as f64 * backup_bins.y_width;

                let new_x_index =
                    ((old_x_value - self.range.x.min) / new_bins.x_width).floor() as usize;
                let new_y_index =
                    ((old_y_value - self.range.y.min) / new_bins.y_width).floor() as usize;

                let new_count = new_bins
                    .counts
                    .entry((new_x_index, new_y_index))
                    .or_insert(0);
                *new_count += count;

                new_bins.min_count = new_bins.min_count.min(*new_count);
                new_bins.max_count = new_bins.max_count.max(*new_count);
            }

            self.bins = new_bins;
            self.plot_settings.recalculate_image = true;

            if x_rebin_factor == 1 && y_rebin_factor == 1 {
                self.backup_bins = None;
            }
        }
    }
}
