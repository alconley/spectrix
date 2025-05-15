use super::histogram1d::Histogram;
use polars::prelude::*;
use std::time::Instant;

impl Histogram {
    pub fn fill(&mut self, value: f64) {
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

        let bin_index = when(col(column).lt(lit(min_val)))
            .then(lit(-2))
            .when(col(column).gt_eq(lit(max_val)))
            .then(lit(-1))
            .otherwise(raw_bin)
            .alias("bin_index");

        let df = lf
            .filter(col(column).neq(lit(invalid_value)))
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

        Ok(())
    }

    pub fn set_counts(&mut self, counts: Vec<u64>) {
        self.bins = counts;
    }
}
