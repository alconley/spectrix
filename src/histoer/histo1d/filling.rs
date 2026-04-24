use super::histogram1d::Histogram;
use polars::prelude::*;
use std::time::Instant;

fn has_default_x_bounds(histogram: &Histogram) -> bool {
    histogram
        .plot_settings
        .current_plot_bounds
        .is_some_and(|(x_min, x_max)| {
            (x_min - -1.0).abs() <= f64::EPSILON && (x_max - 1.0).abs() <= f64::EPSILON
        })
}

impl Histogram {
    pub fn fill(&mut self, value: f64) {
        if value.is_nan() {
            return;
        }

        if value >= self.range.0 && value < self.range.1 {
            let index = ((value - self.range.0) / self.bin_width) as usize;
            if index < self.bins.len() {
                self.bins[index] += 1;
                self.original_bins[index] += 1;
            }
        } else if value >= self.range.1 {
            self.overflow += 1;
        } else {
            self.underflow += 1;
        }
    }

    pub fn fill_from_lazyframe(
        &mut self,
        lf: LazyFrame,
        column: &str,
        invalid_value: f64,
    ) -> PolarsResult<()> {
        let start = Instant::now();

        let (min_val, max_val) = self.range;
        let bin_width = self.bin_width;

        let raw_bin = ((col(column) - lit(min_val)) / lit(bin_width)).cast(DataType::Int32);
        let valid_values = col(column)
            .neq(lit(invalid_value))
            .and(col(column).is_not_nan());

        let bin_index = when(col(column).lt(lit(min_val)))
            .then(lit(-2))
            .when(col(column).gt_eq(lit(max_val)))
            .then(lit(-1))
            .otherwise(raw_bin)
            .alias("bin_index");

        let df = lf
            .filter(valid_values)
            .with_columns([bin_index])
            .group_by([col("bin_index")])
            .agg([col("bin_index").count().alias("count")])
            .sort(["bin_index"], Default::default())
            .collect()?;

        let bin_indices = df.column("bin_index")?.i32()?;
        let counts = df.column("count")?.u32()?;

        for (bin_opt, count_opt) in bin_indices.into_iter().zip(counts) {
            if let (Some(bin), Some(count)) = (bin_opt, count_opt) {
                match bin {
                    -2 => self.underflow += count as u64,
                    -1 => self.overflow += count as u64,
                    i if i >= 0 && (i as usize) < self.bins.len() => {
                        let idx = i as usize;
                        self.bins[idx] += count as u64;
                        self.original_bins[idx] += count as u64;
                    }
                    _ => {}
                }
            }
        }

        let duration = start.elapsed();
        log::info!("Filled histogram {} in {:?}", self.name, duration);
        self.plot_settings.column_name = column.to_owned();
        for cut in &mut self.plot_settings.cuts {
            cut.set_column_name(column);
        }
        if has_default_x_bounds(self) {
            self.plot_settings.egui_settings.reset_axis = true;
        }

        Ok(())
    }

    pub fn set_counts(&mut self, counts: Vec<u64>) {
        self.bins = counts;
    }
}

#[cfg(test)]
mod tests {
    use super::Histogram;
    use polars::df;
    use polars::prelude::IntoLazy;

    #[test]
    fn lazyframe_fill_resets_default_bounds() {
        let mut histogram = Histogram::new("test", 4, (0.0, 4.0));
        histogram.plot_settings.current_plot_bounds = Some((-1.0, 1.0));

        let df = df!("value" => &[0.5_f64, 1.5, 2.5]).expect("Failed to build dataframe");
        histogram
            .fill_from_lazyframe(df.lazy(), "value", -1e6)
            .expect("Failed to fill histogram");

        assert!(histogram.plot_settings.egui_settings.reset_axis);
    }

    #[test]
    fn lazyframe_fill_preserves_non_default_bounds() {
        let mut histogram = Histogram::new("test", 4, (0.0, 4.0));
        histogram.plot_settings.current_plot_bounds = Some((0.0, 4.0));

        let df = df!("value" => &[0.5_f64, 1.5, 2.5]).expect("Failed to build dataframe");
        histogram
            .fill_from_lazyframe(df.lazy(), "value", -1e6)
            .expect("Failed to fill histogram");

        assert!(!histogram.plot_settings.egui_settings.reset_axis);
    }
}
