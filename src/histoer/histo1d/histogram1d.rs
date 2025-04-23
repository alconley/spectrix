use super::plot_settings::PlotSettings;
use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::fitter::common::Data;
use crate::fitter::fit_handler::Fits;
use crate::fitter::main_fitter::{FitModel, Fitter};

use std::time::Instant;

use polars::prelude::*;
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Histogram {
    pub name: String,
    pub bins: Vec<u64>,
    pub range: (f64, f64),
    pub overflow: u64,
    pub underflow: u64,
    pub bin_width: f64,
    pub line: EguiLine,
    pub plot_settings: PlotSettings,
    pub fits: Fits,
    pub original_bins: Vec<u64>,
}

impl Histogram {
    // Create a new Histogram with specified min, max, and number of bins
    pub fn new(name: &str, number_of_bins: usize, range: (f64, f64)) -> Self {
        Histogram {
            name: name.to_string(),
            bins: vec![0; number_of_bins],
            range,
            overflow: 0,
            underflow: 0,
            bin_width: (range.1 - range.0) / number_of_bins as f64,
            line: EguiLine {
                name: name.to_string(),
                ..Default::default()
            },
            plot_settings: PlotSettings::default(),
            fits: Fits::new(),
            original_bins: vec![0; number_of_bins],
        }
    }

    pub fn reset(&mut self) {
        self.bins = vec![0; self.bins.len()];
        self.original_bins = vec![0; self.original_bins.len()];
        self.overflow = 0;
        self.underflow = 0;
    }

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

    pub fn update_line_points(&mut self) {
        self.line.points = self
            .bins
            .iter()
            .enumerate()
            .flat_map(|(index, &count)| {
                let start = self.range.0 + index as f64 * self.bin_width;
                let end = start + self.bin_width;
                let y_value = count as f64;
                vec![[start, y_value], [end, y_value]]
            })
            .collect();
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

    pub fn fit_background(&mut self) {
        log::info!("Fitting background for histogram: {}", self.name);
        self.fits.temp_fit = None;

        let marker_positions = self.plot_settings.markers.get_background_marker_positions();
        if marker_positions.is_empty() {
            log::error!("Need to set at least one background marker pair to fit the histogram");
            return;
        }

        let mut x_data = Vec::new();
        let mut y_data = Vec::new();

        for (start_x, end_x) in marker_positions {
            let bin_centers = self.get_bin_centers_between(start_x, end_x);
            let bin_counts = self.get_bin_counts_between(start_x, end_x);

            x_data.extend(bin_centers);
            y_data.extend(bin_counts);
        }

        if x_data.is_empty() || y_data.is_empty() {
            log::error!("No valid data points found between background markers.");
            return;
        }

        let mut fitter = Fitter::new(Data {
            x: x_data,
            y: y_data,
        });

        fitter.background_model = self.fits.settings.background_model.clone();
        fitter.fit_background();

        fitter.name = format!("{} Temp Fit", self.name);
        fitter.set_name(self.name.clone());

        self.fits.temp_fit = Some(fitter);
    }

    pub fn fit_gaussians(&mut self) {
        let region_markers = self.plot_settings.markers.get_region_marker_positions();
        let peak_positions = self.plot_settings.markers.get_peak_marker_positions();
        let background_markers = self.plot_settings.markers.get_background_marker_positions();

        let centers = self.get_bin_centers();
        let counts = self.bins.clone();

        let data = Data {
            x: centers,
            y: counts.iter().map(|&c| c as f64).collect(),
        };

        let mut fitter = Fitter::new(data);

        let background_model = self.fits.settings.background_model.clone();

        let background_result = if let Some(temp_fit) = &self.fits.temp_fit {
            fitter.background_line = temp_fit.background_line.clone();
            temp_fit.background_result.clone()
        } else {
            None
        };

        let equal_stdev = self.fits.settings.equal_stddev;
        let free_position = self.fits.settings.free_position;

        fitter.background_model = background_model;
        fitter.background_result = background_result;

        fitter.fit_model = FitModel::Gaussian(
            region_markers.clone(),
            peak_positions.clone(),
            background_markers.clone(),
            equal_stdev,
            free_position,
        );

        fitter.fit();

        self.plot_settings.markers.clear_peak_markers();
        let updated_markers = fitter.get_peak_markers();
        for marker in updated_markers {
            self.plot_settings.markers.add_peak_marker(marker);
        }

        fitter.set_name(self.name.clone());
        self.fits.temp_fit = Some(fitter);
    }

    pub fn update_background_pair_lines(&mut self) {
        // Extract bin edges and counts **before** modifying anything
        let bin_edges = self.get_bin_edges();
        let bin_counts = self.bins.clone();

        // Extract immutable background marker positions first
        let marker_positions: Vec<(f64, f64)> = self
            .plot_settings
            .markers
            .background_markers
            .iter()
            .map(|bg_pair| (bg_pair.start.x_value, bg_pair.end.x_value))
            .collect();

        // Compute bin indices based on marker positions **before** modifying anything
        let bin_indices: Vec<(usize, usize)> = marker_positions
            .iter()
            .map(|&(start_x, end_x)| {
                let start_bin = self.get_bin_index(start_x).unwrap_or(0);
                let end_bin = self
                    .get_bin_index(end_x)
                    .unwrap_or(self.bins.len().saturating_sub(1));
                (start_bin, end_bin)
            })
            .collect();

        // Now, modify `background_markers` without conflicting borrows
        for (bg_pair, &(start_bin, end_bin)) in self
            .plot_settings
            .markers
            .background_markers
            .iter_mut()
            .zip(bin_indices.iter())
        {
            bg_pair.histogram_line.points.clear(); // Clear previous points

            // Collect the **actual bin edges** and counts in the correct range
            for i in start_bin..=end_bin {
                if i < bin_edges.len() - 1 {
                    // Ensure no out-of-bounds access
                    let x_start = bin_edges[i]; // Start of the bin
                    let x_end = bin_edges[i + 1]; // End of the bin
                    let y = bin_counts[i] as f64; // Bin count

                    // Add both edges of the bin to the histogram line
                    bg_pair.histogram_line.points.push([x_start, y]);
                    bg_pair.histogram_line.points.push([x_end, y]);
                }
            }
        }
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        // update the histogram and fit lines with the log setting and draw
        let log_y = self.plot_settings.egui_settings.log_y;
        let log_x = self.plot_settings.egui_settings.log_x;

        self.line.log_y = log_y;
        self.line.log_x = log_x;
        self.line.draw(plot_ui);

        self.fits.set_log(log_y, log_x);
        self.fits.draw(plot_ui);

        self.show_stats(plot_ui);

        self.plot_settings.markers.draw_all_markers(plot_ui);
        self.update_background_pair_lines();
        for bg_pair in &mut self.plot_settings.markers.background_markers {
            bg_pair.histogram_line.log_x = log_x;
            bg_pair.histogram_line.log_y = log_y;
        }

        // Check if markers are being dragged
        if self.plot_settings.markers.is_dragging() {
            // Disable dragging if a marker is being dragged
            self.plot_settings.egui_settings.allow_drag = false;
        } else {
            self.plot_settings.egui_settings.allow_drag = true;
        }

        if plot_ui.response().hovered() {
            self.plot_settings.cursor_position = plot_ui.pointer_coordinate();
            self.plot_settings.egui_settings.limit_scrolling = true;
        } else {
            self.plot_settings.cursor_position = None;
        }

        if self.plot_settings.egui_settings.reset_axis {
            self.plot_settings.egui_settings.reset_axis_lims(plot_ui);
            self.plot_settings.egui_settings.reset_axis = false;
        } else {
            self.limit_scrolling(plot_ui);
        }

        // self.plot_settings.egui_settings.y_label = format!("Counts/{:.}", self.bin_width);
    }

    pub fn draw_other_histograms(
        &mut self,
        plot_ui: &mut egui_plot::PlotUi<'_>,
        histograms: &[Histogram],
    ) {
        for histogram in histograms {
            let mut hist = histogram.clone();
            hist.draw(plot_ui);
        }
    }

    pub fn limit_scrolling(&self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        let plot_bounds = plot_ui.plot_bounds();

        let current_x_min = plot_bounds.min()[0];
        let current_x_max = plot_bounds.max()[0];
        let current_y_min = plot_bounds.min()[1];
        let current_y_max = plot_bounds.max()[1];

        let y_max = self.bins.iter().max().cloned().unwrap_or(0) as f64;
        let y_min = self.bins.iter().min().cloned().unwrap_or(0) as f64;

        // account for log y
        let y_min = if self.plot_settings.egui_settings.log_y {
            0.01
        } else {
            y_min
        };

        let y_max = if self.plot_settings.egui_settings.log_y {
            y_max.log10().max(0.0001)
        } else {
            y_max
        };

        if current_x_min == -1.0
            && current_x_max == 1.0
            && current_y_min == 0.0
            && current_y_max == 1.0
        {
            let default_bounds =
                egui_plot::PlotBounds::from_min_max([self.range.0, y_min], [self.range.1, y_max]);

            plot_ui.set_plot_bounds(default_bounds);
            return;
        }

        // Clamping bounds only for scrolling
        let new_x_min = current_x_min.max(self.range.0 * 1.1);
        let new_x_max = current_x_max.min(self.range.1 * 1.1);
        let new_y_min = current_y_min.max(y_min * 1.1);
        let new_y_max = current_y_max.min(y_max * 1.1);

        let new_y_min = if self.plot_settings.egui_settings.log_y {
            0.01
        } else {
            new_y_min
        };

        if new_x_min != current_x_min
            || new_x_max != current_x_max
            || new_y_min != current_y_min
            || new_y_max != current_y_max
        {
            let clamped_bounds =
                egui_plot::PlotBounds::from_min_max([new_x_min, new_y_min], [new_x_max, new_y_max]);
            plot_ui.set_plot_bounds(clamped_bounds);
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        // if light mode, se the color to black
        if ui.visuals().dark_mode {
            self.line.set_color(egui::Color32::LIGHT_BLUE);
        } else {
            self.line.set_color(egui::Color32::BLACK);
        }

        // Display progress bar while hist is being filled
        // disabled since the row calculation is done in chucks
        // self.plot_settings.progress_ui(ui);

        self.update_line_points(); // Ensure line points are updated for projections
        self.keybinds(ui); // Handle interactive elements

        let mut plot = egui_plot::Plot::new(self.name.clone());
        plot = self.plot_settings.egui_settings.apply_to_plot(plot);

        self.fits.fit_stats_ui(ui);

        let (scroll, _pointer_down, _modifiers) = ui.input(|i| {
            let scroll = i.events.iter().find_map(|e| match e {
                egui::Event::MouseWheel {
                    unit: _,
                    delta,
                    modifiers: _,
                } => Some(*delta),
                _ => None,
            });
            (scroll, i.pointer.primary_down(), i.modifiers)
        });

        let plot_response = plot.show(ui, |plot_ui| {
            self.draw(plot_ui);

            if self.plot_settings.progress.is_some() {
                let y_max = self.bins.iter().max().cloned().unwrap_or(0) as f64;
                let mut plot_bounds = plot_ui.plot_bounds();
                plot_bounds.extend_with_y(y_max * 1.1);
                plot_ui.set_plot_bounds(plot_bounds);
            }

            if self.plot_settings.egui_settings.reset_axis {
                plot_ui.auto_bounds();
                self.plot_settings.egui_settings.reset_axis = false;
            }

            if self.plot_settings.cursor_position.is_some() {
                if let Some(delta_pos) = scroll {
                    if delta_pos.y > 0.0 {
                        plot_ui.zoom_bounds_around_hovered(egui::Vec2::new(1.1, 1.0));
                    } else if delta_pos.y < 0.0 {
                        plot_ui.zoom_bounds_around_hovered(egui::Vec2::new(0.9, 1.0));
                    } else if delta_pos.x > 0.0 {
                        plot_ui.zoom_bounds_around_hovered(egui::Vec2::new(1.1, 1.0));
                    } else if delta_pos.x < 0.0 {
                        plot_ui.zoom_bounds_around_hovered(egui::Vec2::new(0.9, 1.0));
                    }
                }
            }
        });

        plot_response.response.context_menu(|ui| {
            self.context_menu(ui);
        });

        self.plot_settings.interactive_response(&plot_response);
    }
}
