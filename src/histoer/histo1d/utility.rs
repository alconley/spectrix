use super::histogram1d::Histogram;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

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

        let bin_index = ((x - self.range.0) / self.bin_width).floor() as usize;
        let last_bin = self.bins.len().saturating_sub(1);

        Some(bin_index.min(last_bin))
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

    pub fn export_bin_counts_csv_dialog(&self) {
        let suggested_file_name = format!(
            "{}_bin_counts.csv",
            sanitize_histogram_file_name_component(&self.name)
        );

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name(suggested_file_name)
            .save_file()
        {
            let path = ensure_extension_if_missing(path, "csv");
            match self.write_bin_counts_csv_to_path(&path) {
                Ok(()) => log::info!("Exported histogram bin counts to {}", path.display()),
                Err(error) => log::error!(
                    "Failed to export histogram bin counts to {}: {error}",
                    path.display()
                ),
            }
        }
    }

    pub fn write_bin_counts_csv_to_path(&self, path: &Path) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        self.write_bin_counts_csv(BufWriter::new(file))
    }

    pub fn write_bin_counts_csv<W: std::io::Write>(&self, mut writer: W) -> std::io::Result<()> {
        writeln!(writer, "bin_left,bin_right,count")?;

        for (index, count) in self.bins.iter().enumerate() {
            let bin_left = self.range.0 + index as f64 * self.bin_width;
            let bin_right = if index + 1 == self.bins.len() {
                self.range.1
            } else {
                bin_left + self.bin_width
            };

            writeln!(writer, "{bin_left},{bin_right},{count}")?;
        }

        Ok(())
    }
}

fn ensure_extension_if_missing(mut path: PathBuf, extension: &str) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension(extension);
    }
    path
}

fn sanitize_histogram_file_name_component(name: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());
    let mut previous_was_underscore = false;

    for character in name.trim().chars() {
        let mapped = if matches!(
            character,
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
        ) || character.is_whitespace()
        {
            '_'
        } else {
            character
        };

        if mapped == '_' {
            if !previous_was_underscore {
                sanitized.push('_');
            }
            previous_was_underscore = true;
        } else {
            sanitized.push(mapped);
            previous_was_underscore = false;
        }
    }

    let sanitized = sanitized.trim_matches('_').to_owned();
    if sanitized.is_empty() {
        "histogram".to_owned()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::Histogram;

    #[test]
    fn bin_counts_csv_uses_bin_edges_and_counts() {
        let mut histogram = Histogram::new("test", 4, (0.0, 4.0));
        histogram.bins = vec![3, 0, 7, 11];

        let mut csv = Vec::new();
        histogram
            .write_bin_counts_csv(&mut csv)
            .expect("failed to write histogram CSV");

        let csv = String::from_utf8(csv).expect("CSV should be valid UTF-8");
        assert_eq!(
            csv,
            "bin_left,bin_right,count\n0,1,3\n1,2,0\n2,3,7\n3,4,11\n"
        );
    }
}
