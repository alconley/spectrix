use fnv::FnvHashMap;
use rayon::prelude::*;

use crate::egui_plot_stuff::egui_image::EguiImage;

use super::plot_settings::PlotSettings;

use polars::prelude::*;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Histogram2D {
    pub name: String,
    pub bins: Bins,
    pub range: Range,
    pub overflow: u64,
    pub underflow: u64,
    pub plot_settings: PlotSettings,
    pub image: EguiImage,
    pub backup_bins: Option<Bins>,
}

impl Histogram2D {
    // Create a new 2D Histogram with specified ranges and number of bins for each axis
    pub fn new(name: &str, bins: (usize, usize), range: ((f64, f64), (f64, f64))) -> Self {
        Self {
            name: name.to_owned(),
            bins: Bins {
                x: bins.0,
                x_width: (range.0.1 - range.0.0) / bins.0 as f64,
                y: bins.1,
                y_width: (range.1.1 - range.1.0) / bins.1 as f64,
                counts: FnvHashMap::default(),
                min_count: u64::MAX,
                max_count: u64::MIN,
            },
            range: Range {
                x: Value {
                    min: range.0.0,
                    max: range.0.1,
                },
                y: Value {
                    min: range.1.0,
                    max: range.1.1,
                },
            },
            overflow: 0,
            underflow: 0,
            plot_settings: PlotSettings::default(),
            image: EguiImage::heatmap(
                name.to_owned(),
                [range.0.0, range.0.1],
                [range.1.0, range.1.1],
            ),
            backup_bins: None,
        }
    }

    pub fn reset(&mut self) {
        self.bins.counts.clear();
        self.bins.min_count = u64::MAX;
        self.bins.max_count = u64::MIN;
        self.plot_settings.recalculate_image = true;
    }

    pub fn fill(&mut self, x_value: f64, y_value: f64) {
        if x_value < self.range.x.min {
            self.underflow += 1; // Increment x-axis underflow
        } else if x_value >= self.range.x.max {
            self.overflow += 1; // Increment x-axis overflow
        } else if y_value < self.range.y.min {
            self.underflow += 1; // Increment y-axis underflow
        } else if y_value >= self.range.y.max {
            self.overflow += 1; // Increment y-axis overflow
        } else {
            // Value is within range; proceed to calculate indices and update counts
            let x_index = ((x_value - self.range.x.min) / self.bins.x_width) as usize;
            let y_index = ((y_value - self.range.y.min) / self.bins.y_width) as usize;

            let count = self.bins.counts.entry((x_index, y_index)).or_insert(0);
            *count += 1;

            self.bins.min_count = self.bins.min_count.min(*count);
            self.bins.max_count = self.bins.max_count.max(*count);
        }
    }

    pub fn fill_from_lazyframe(
        &mut self,
        lf: LazyFrame,
        x_column: &str,
        y_column: &str,
        invalid_value: f64,
    ) -> PolarsResult<()> {
        let (x_min, x_max) = (self.range.x.min, self.range.x.max);
        let (y_min, y_max) = (self.range.y.min, self.range.y.max);
        let (x_width, y_width) = (self.bins.x_width, self.bins.y_width);

        let raw_x = ((col(x_column) - lit(x_min)) / lit(x_width)).cast(DataType::Int32);
        let raw_y = ((col(y_column) - lit(y_min)) / lit(y_width)).cast(DataType::Int32);

        let x_bin = when(col(x_column).lt(lit(x_min)))
            .then(lit(-2))
            .when(col(x_column).gt_eq(lit(x_max)))
            .then(lit(-1))
            .otherwise(raw_x)
            .alias("x_bin");

        let y_bin = when(col(y_column).lt(lit(y_min)))
            .then(lit(-2))
            .when(col(y_column).gt_eq(lit(y_max)))
            .then(lit(-1))
            .otherwise(raw_y)
            .alias("y_bin");

        let df = lf
            .filter(
                col(x_column)
                    .neq(lit(invalid_value))
                    .and(col(y_column).neq(lit(invalid_value))),
            )
            .with_columns([x_bin, y_bin])
            .group_by([col("x_bin"), col("y_bin")])
            .agg([col("x_bin").count().alias("count")])
            .sort(["x_bin", "y_bin"], Default::default())
            .collect()?;

        let x_bins = df.column("x_bin")?.i32()?;
        let y_bins = df.column("y_bin")?.i32()?;
        let counts = df.column("count")?.u32()?;

        for ((x_opt, y_opt), count_opt) in x_bins.into_iter().zip(y_bins).zip(counts) {
            if let ((Some(x), Some(y)), Some(count)) = ((x_opt, y_opt), count_opt) {
                match (x, y) {
                    (-2, _) | (_, -2) => self.underflow += count as u64,
                    (-1, _) | (_, -1) => self.overflow += count as u64,
                    (x_bin, y_bin) if x_bin >= 0 && y_bin >= 0 => {
                        let (xi, yi) = (x_bin as usize, y_bin as usize);
                        let bin = self.bins.counts.entry((xi, yi)).or_insert(0);
                        *bin += count as u64;
                        self.bins.min_count = self.bins.min_count.min(*bin);
                        self.bins.max_count = self.bins.max_count.max(*bin);
                    }
                    _ => {}
                }
            }
        }

        self.plot_settings.recalculate_image = true;
        self.plot_settings.x_column = x_column.to_owned();
        self.plot_settings.y_column = y_column.to_owned();

        Ok(())
    }

    // get the bin index for a given x value
    pub fn get_bin_index_x(&self, x: f64) -> Option<usize> {
        if x < self.range.x.min || x > self.range.x.max {
            return None;
        }

        let bin_index: usize = ((x - self.range.x.min) / self.bins.x_width).floor() as usize;

        Some(bin_index)
    }

    // get the bin index for a given y value
    pub fn get_bin_index_y(&self, y: f64) -> Option<usize> {
        if y < self.range.y.min || y > self.range.y.max {
            return None;
        }

        let bin_index: usize = ((y - self.range.y.min) / self.bins.y_width).floor() as usize;

        Some(bin_index)
    }

    fn data_2_image(&self) -> egui::ColorImage {
        let width = ((self.range.x.max - self.range.x.min) / self.bins.x_width) as usize;
        let height = ((self.range.y.max - self.range.y.min) / self.bins.y_width) as usize;
        let colormap_options = self.plot_settings.colormap_options;

        // Preallocate a flat buffer for the pixel data.
        let total_pixels = width * height;
        let mut pixels = vec![egui::Color32::default(); total_pixels];

        // Fill the pixel buffer in parallel.
        pixels.par_iter_mut().enumerate().for_each(|(i, pixel)| {
            // Compute the 2D coordinate.
            // (x, y) where we reverse the y coordinate so that y=0 is at the bottom.
            let x = i % width;
            let y = height - 1 - (i / width);
            let count = self.bins.counts.get(&(x, y)).copied().unwrap_or(0);
            *pixel = self.plot_settings.colormap.color(
                count,
                self.bins.min_count,
                self.bins.max_count,
                colormap_options,
            );
        });

        egui::ColorImage::new([width, height], pixels)
    }

    // Recalculate the image and replace the existing texture
    fn calculate_image(&mut self, ui: &mut egui::Ui) {
        self.image.texture = None;
        let color_image = self.data_2_image();
        self.image.get_texture(ui, &color_image);
    }

    fn limit_scrolling(&self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        let plot_bounds = plot_ui.plot_bounds();

        let current_x_min = plot_bounds.min()[0];
        let current_x_max = plot_bounds.max()[0];
        let current_y_min = plot_bounds.min()[1];
        let current_y_max = plot_bounds.max()[1];

        if current_x_min == -1.0
            || current_x_min == 0.0
                && current_x_max == 1.0
                && current_y_min == 0.0
                && current_y_max == 1.0
        {
            let default_bounds = egui_plot::PlotBounds::from_min_max(
                [self.range.x.min, self.range.y.min],
                [self.range.x.max, self.range.y.max],
            );

            plot_ui.set_plot_bounds(default_bounds);
            return;
        }

        // Clamping bounds only for scrolling
        let new_x_min = current_x_min.max(self.range.x.min);
        let new_x_max = current_x_max.min(self.range.x.max);
        let new_y_min = current_y_min.max(self.range.y.min);
        let new_y_max = current_y_max.min(self.range.y.max);

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

    // Draw the histogram on the plot
    fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        self.show_stats(plot_ui);

        let heatmap_image = self.image.get_plot_image_from_texture();

        if let Some(image) = heatmap_image {
            self.image.draw(plot_ui, image);
        }

        if plot_ui.response().hovered() {
            self.plot_settings.cursor_position = plot_ui.pointer_coordinate();
            self.plot_settings.egui_settings.limit_scrolling = true;
        } else {
            self.plot_settings.cursor_position = None;
        }

        self.plot_settings.draw(plot_ui);

        self.plot_settings.egui_settings.allow_drag = !self.plot_settings.projections.dragging;

        if self.plot_settings.egui_settings.reset_axis {
            self.plot_settings.egui_settings.reset_axis_lims(plot_ui);
        } else if self.plot_settings.egui_settings.limit_scrolling {
            self.limit_scrolling(plot_ui);
        }
    }

    // Render the histogram using egui_plot
    pub fn render(&mut self, ui: &mut egui::Ui) {
        // Recalculate the image if the settings have changed, like the colormap
        if self.plot_settings.recalculate_image {
            self.calculate_image(ui);
            self.plot_settings.recalculate_image = false;
        }

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

        let mut plot = egui_plot::Plot::new(self.name.clone());
        plot = self.plot_settings.egui_settings.apply_to_plot(plot);

        if self.image.texture.is_none() {
            self.calculate_image(ui);
        }

        self.check_projections();
        self.plot_settings.projections.show(ui);

        let plot_response = plot.show(ui, |plot_ui| {
            self.draw(plot_ui);

            if self.plot_settings.cursor_position.is_some()
                && let Some(delta_pos) = scroll
            {
                if delta_pos.y > 0.0 {
                    plot_ui.zoom_bounds_around_hovered(egui::Vec2::new(1.1, 1.1));
                } else if delta_pos.y < 0.0 {
                    plot_ui.zoom_bounds_around_hovered(egui::Vec2::new(0.9, 0.9));
                } else if delta_pos.x > 0.0 {
                    plot_ui.zoom_bounds_around_hovered(egui::Vec2::new(1.1, 1.1));
                } else if delta_pos.x < 0.0 {
                    plot_ui.zoom_bounds_around_hovered(egui::Vec2::new(0.9, 0.9));
                }
            }
        });

        plot_response.response.context_menu(|ui| {
            self.context_menu(ui);
        });

        self.plot_settings.interactive_response(&plot_response);

        self.keybinds(ui);
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Bins {
    pub x: usize,
    pub x_width: f64,
    pub y: usize,
    pub y_width: f64,
    pub counts: FnvHashMap<(usize, usize), u64>, // uses a hash map to store the histogram data (zero overhead for empty bins)
    pub min_count: u64,
    pub max_count: u64,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Value {
    pub min: f64,
    pub max: f64,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Range {
    pub x: Value,
    pub y: Value,
}
