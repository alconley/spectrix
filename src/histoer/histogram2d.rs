use super::colormaps::ColorMap;
use super::plot_settings::EguiPlotSettings;
use super::histogram1d::Histogram;
use crate::egui_plot_stuff::egui_image::EguiImage;
use crate::egui_plot_stuff::egui_polygon::EguiPolygon;
use crate::egui_plot_stuff::egui_vertical_line::EguiVerticalLine;
use crate::egui_plot_stuff::egui_horizontal_line::EguiHorizontalLine;

use egui::viewport::{ViewportBuilder, ViewportId, ViewportClass};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use fnv::FnvHashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotSettings {
    #[serde(skip)]
    cursor_position: Option<egui_plot::PlotPoint>,
    egui_settings: EguiPlotSettings,
    cut_polygons: Vec<EguiPolygon>,
    y_projection_lines: Vec<EguiVerticalLine>,
    show_y_projection: bool,
    x_projection_lines: Vec<EguiHorizontalLine>,
    stats_info: bool,
    colormap: ColorMap,
    log_norm_colormap: bool,
}

impl Default for PlotSettings {
    fn default() -> Self {
        PlotSettings {
            cursor_position: None,
            egui_settings: EguiPlotSettings::default(),
            cut_polygons: Vec::new(),
            y_projection_lines: Vec::new(),
            show_y_projection: false,
            x_projection_lines: Vec::new(),
            stats_info: false,
            colormap: ColorMap::default(),
            log_norm_colormap: true,
        }
    }
}

impl PlotSettings {
    pub fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Colormap", |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.log_norm_colormap, "Log Norm");
            });
            self.colormap.color_maps_ui(ui);
        });

        ui.separator();
        ui.checkbox(&mut self.stats_info, "Show Statitics");
        self.egui_settings.menu_button(ui);

        ui.separator();

        ui.heading("Projections ");
        ui.horizontal(|ui| {
            ui.label("Y-Projections");
            if ui.button("Add Y-Projection").clicked() {
                self.y_projection_lines.push(EguiVerticalLine::new(0.0, egui::Color32::RED));
            }
        });

        for line in self.y_projection_lines.iter_mut() {
            line.menu_button(ui);
        }

        ui.separator();

        ui.horizontal(|ui| {
            ui.heading("Cuts");
            if ui.button("Add Cut").clicked() {
                let name = format!("Cut {}", self.cut_polygons.len() + 1);
                self.cut_polygons.push(EguiPolygon::new(&name));
            }
        });

        let mut index_to_remove = None;
        for (index, polygon) in self.cut_polygons.iter_mut().enumerate() {

            ui.horizontal(|ui| {
                if ui.button("🗙").clicked() {
                    index_to_remove = Some(index);
                }

                ui.separator();

                polygon.menu_button(ui);

            });
        }

        if let Some(index) = index_to_remove {
            self.cut_polygons.remove(index);
        }
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi) {
        for polygon in self.cut_polygons.iter_mut() {
            polygon.mouse_interactions(plot_ui);
            polygon.draw(plot_ui);
        }

        for line in self.y_projection_lines.iter_mut() {
            line.draw(plot_ui);
        }

        for line in self.x_projection_lines.iter_mut() {
            line.draw(plot_ui);
        }
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Bins {
    x: usize,
    x_width: f64,
    y: usize,
    y_width: f64,
    counts: FnvHashMap<(usize, usize), u32>, // uses a hash map to store the histogram data (zero overhead for empty bins)
    min_count: u32,
    max_count: u32,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Value {
    min: f64,
    max: f64,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Range {
    x: Value,
    y: Value,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Histogram2D {
    pub name: String,
    pub bins: Bins,
    pub range: Range,
    pub plot_settings: PlotSettings,
    pub image: EguiImage,
    #[serde(skip)]
    pub y_projection_open: Arc<AtomicBool>,
}

impl Histogram2D {
    // Create a new 2D Histogram with specified ranges and number of bins for each axis
    pub fn new(name: &str, bins: (usize, usize), range: ((f64, f64), (f64, f64))) -> Self {
        Histogram2D {
            name: name.to_string(),
            bins: Bins {
                x: bins.0,
                x_width: (range.0 .1 - range.0 .0) / bins.0 as f64,
                y: bins.1,
                y_width: (range.1 .1 - range.1 .0) / bins.1 as f64,
                counts: FnvHashMap::default(),
                min_count: u32::MAX,
                max_count: u32::MIN,
            },
            range: Range {
                x: Value {
                    min: range.0 .0,
                    max: range.0 .1,
                },
                y: Value {
                    min: range.1 .0,
                    max: range.1 .1,
                },
            },
            plot_settings: PlotSettings::default(),
            image: EguiImage::heatmap(
                name.to_string(),
                [range.0 .0, range.0 .1],
                [range.1 .0, range.1 .1],
            ),
            y_projection_open: Arc::new(AtomicBool::new(false)),
        }
    }

    // Add a value to the histogram
    pub fn fill(&mut self, x_value: f64, y_value: f64) {
        if x_value >= self.range.x.min && x_value < self.range.x.max
            && y_value >= self.range.y.min && y_value < self.range.y.max
        {
            let x_index = ((x_value - self.range.x.min) / self.bins.x_width) as usize;
            let y_index = ((y_value - self.range.y.min) / self.bins.y_width) as usize;
            let count = self.bins.counts.entry((x_index, y_index)).or_insert(0);
            *count += 1;
    
            self.bins.min_count = self.bins.min_count.min(*count);
            self.bins.max_count = self.bins.max_count.max(*count);
        }
    }

    // get the bin index for a given x value
    fn get_bin_x(&self, x: f64) -> Option<usize> {
        if x < self.range.x.min || x > self.range.x.max {
            return None;
        }

        let bin_index: usize = ((x - self.range.x.min) / self.bins.x_width).floor() as usize;

        Some(bin_index)
    }

    // get the bin index for a given y value
    fn get_bin_y(&self, y: f64) -> Option<usize> {
        if y < self.range.y.min || y > self.range.y.max {
            return None;
        }

        let bin_index: usize = ((y - self.range.y.min) / self.bins.y_width).floor() as usize;

        Some(bin_index)
    }

    // Calculate statistics for a given range (Integral, Mean X, Stdev X, Mean Y, Stdev Y)
    fn stats(
        &self,
        start_x: f64,
        end_x: f64,
        start_y: f64,
        end_y: f64,
    ) -> (u32, f64, f64, f64, f64) {
        let start_x_index = self.get_bin_x(start_x).unwrap_or(0);
        let end_x_index = self.get_bin_x(end_x).unwrap_or_else(|| {
            self.bins
                .counts
                .keys()
                .max_by_key(|k| k.0)
                .map_or(0, |k| k.0)
        });

        let start_y_index = self.get_bin_y(start_y).unwrap_or(0);
        let end_y_index = self.get_bin_y(end_y).unwrap_or_else(|| {
            self.bins
                .counts
                .keys()
                .max_by_key(|k| k.1)
                .map_or(0, |k| k.1)
        });

        let mut total_count = 0;

        let mut sum_product_x = 0.0;
        let mut sum_product_y = 0.0;

        for (&(x_index, y_index), &count) in self.bins.counts.iter() {
            if x_index >= start_x_index
                && x_index <= end_x_index
                && y_index >= start_y_index
                && y_index <= end_y_index
            {
                let bin_center_x = self.range.x.min
                    + (x_index as f64 * self.bins.x_width)
                    + self.bins.x_width * 0.5;
                let bin_center_y = self.range.y.min
                    + (y_index as f64 * self.bins.y_width)
                    + self.bins.y_width * 0.5;

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

            for (&(x_index, y_index), &count) in self.bins.counts.iter() {
                if x_index >= start_x_index
                    && x_index <= end_x_index
                    && y_index >= start_y_index
                    && y_index <= end_y_index
                {
                    let bin_center_x = self.range.x.min
                        + (x_index as f64 * self.bins.x_width)
                        + self.bins.x_width * 0.5;
                    let bin_center_y = self.range.y.min
                        + (y_index as f64 * self.bins.y_width)
                        + self.bins.y_width * 0.5;

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

    // Show statistics on the plot
    fn show_stats(&self, plot_ui: &mut egui_plot::PlotUi) {
        if !self.plot_settings.stats_info {
            return;
        }

        let plot_min_x = plot_ui.plot_bounds().min()[0];
        let plot_max_x = plot_ui.plot_bounds().max()[0];
        let plot_min_y = plot_ui.plot_bounds().min()[1];
        let plot_max_y = plot_ui.plot_bounds().max()[1];

        let stats = self.stats(plot_min_x, plot_max_x, plot_min_y, plot_max_y);

        let stats_entries = [
            format!("Integral: {}", stats.0),
            format!("Mean X: {:.2}", stats.1),
            format!("Stdev X: {:.2}", stats.2),
            format!("Mean Y: {:.2}", stats.3),
            format!("Stdev Y: {:.2}", stats.4),
        ];

        for entry in stats_entries.iter() {
            plot_ui.text(
                egui_plot::Text::new(egui_plot::PlotPoint::new(0, 0), " ") // Placeholder for positioning; adjust as needed
                    .highlight(false)
                    .name(entry),
            );
        }
    }

    // Convert histogram data to a ColorImage
    fn data_2_image(&self) -> egui::ColorImage {

        let width = ((self.range.x.max - self.range.x.min) / self.bins.x_width) as usize; // number of pixels in x direction
        let height = ((self.range.y.max - self.range.y.min) / self.bins.y_width) as usize; // number of pixels in y direction
    
        // The pixels, row by row, from top to bottom. Each pixel is a Color32.
        let mut pixels = Vec::with_capacity(width * height);
    
        for y in 0..height {
            for x in 0..width {
                let count = self.bins.counts.get(&(x, height - y - 1)).cloned().unwrap_or(0);
                let color = self.plot_settings.colormap.color(
                    count,
                    self.bins.min_count,
                    self.bins.max_count,
                    self.plot_settings.log_norm_colormap,
                );
                pixels.push(color);
            }
        }
    
        // Create the ColorImage with the specified width and height and pixel data
        let image = egui::ColorImage {
            size: [width, height],
            pixels,
        };

        image
    }
    
    // Recalculate the image and replace the existing texture
    pub fn calculate_image(&mut self, ui: &mut egui::Ui)  {
        self.image.texture = None;
        let color_image = self.data_2_image();
        self.image.get_texture(ui, color_image);
    }

    // Extract a y-projection in the given x-range and display it in a new window
    pub fn y_projection(&self, ui: &mut egui::Ui, x_min: f64, x_max: f64) {
        // Extract the y-projection data
        let mut y_bins = vec![0; self.bins.y];

        for ((x_index, y_index), &count) in &self.bins.counts {
            let x_center = self.range.x.min + (*x_index as f64 + 0.5) * self.bins.x_width;
            if x_center >= x_min && x_center < x_max {
                if *y_index < y_bins.len() {
                    y_bins[*y_index] += count;
                }
            }
        }

        // Create a new Histogram for the y-projection
        let y_histogram = Arc::new(Mutex::new(Histogram::new(
            &format!("Y-Projection of {}", self.name),
            self.bins.y,
            (self.range.y.min, self.range.y.max),
        )));
        {
            let mut y_histogram = y_histogram.lock().unwrap();
            y_histogram.bins = y_bins;
        }

        // Create a unique ViewportId for the new viewport
        let viewport_id = ViewportId::from_hash_of(format!("Y-Projection-{}", self.name));

        // Create a viewport builder and set the title
        let viewport_builder = ViewportBuilder::default().with_title("Y-Projection");

        // Show the y-projection in a new window using show_viewport_deferred
        let y_histogram_clone = Arc::clone(&y_histogram);
        let open_clone = Arc::clone(&self.y_projection_open);
        self.y_projection_open.store(true, Ordering::Relaxed);
        ui.ctx().show_viewport_deferred(viewport_id, viewport_builder, move |ctx, class| {
            let y_histogram_clone = Arc::clone(&y_histogram_clone);
            let open_clone = Arc::clone(&open_clone);
            if let ViewportClass::Embedded = class {
                egui::Window::new("Y-Projection")
                    .open(&mut open_clone.load(Ordering::Relaxed))
                    .show(ctx, move |ui| {
                        let mut y_histogram = y_histogram_clone.lock().unwrap();
                        y_histogram.render(ui);
                        if ui.input(|i| i.viewport().close_requested()) {
                            open_clone.store(false, Ordering::Relaxed);
                        }
                    });
            } else {
                egui::CentralPanel::default().show(ctx, move |ui| {
                    let mut y_histogram = y_histogram_clone.lock().unwrap();
                    y_histogram.render(ui);
                    if ctx.input(|i| i.viewport().close_requested()) {
                        open_clone.store(false, Ordering::Relaxed);
                    }
                });
            }
        });
    }

    pub fn y_projection_keybinds(&mut self, ui: &mut egui::Ui) {
        if ui.input(|i| i.key_pressed(egui::Key::Y)) {
            // self.plot_settings.show_y_projection = !self.plot_settings.show_y_projection;

            if self.plot_settings.y_projection_lines.len() == 2 {
                let x_min = self.plot_settings.y_projection_lines[0].x_value;
                let x_max = self.plot_settings.y_projection_lines[1].x_value;
                self.y_projection(ui, x_min, x_max);
            }
        }
    }

    pub fn keybinds(&mut self, ui: &mut egui::Ui) {
        self.y_projection_keybinds(ui);
    }

    // Context menu for the plot (when you right-click on the plot)
    fn context_menu(&mut self, ui: &mut egui::Ui) {
        self.image.menu_button(ui);
        self.plot_settings.settings_ui(ui);
    }

    // Draw the histogram on the plot
    fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi, plot_image: egui_plot::PlotImage) {
        self.show_stats(plot_ui);
        self.image.draw(plot_ui, plot_image);
        self.plot_settings.cursor_position = plot_ui.pointer_coordinate();
        self.plot_settings.draw(plot_ui);
    }

    // Render the histogram using egui_plot
    pub fn render(&mut self, ui: &mut egui::Ui) {
        let mut plot = egui_plot::Plot::new(self.name.clone());
        plot = self.plot_settings.egui_settings.apply_to_plot(plot);

        self.keybinds(ui);

        if self.image.texture.is_none() {
            self.calculate_image(ui);
        }

        if ui.button("Show Y-Projection").clicked() {
            self.y_projection_open.store(true, Ordering::Relaxed);
        }

        if self.y_projection_open.load(Ordering::Relaxed) {
            self.y_projection(ui, 0.0, 10.0); // Adjust x_min and x_max as needed
        }

        let heatmap_image = self.image.get_plot_image_from_texture(ui);
    
        plot.show(ui, |plot_ui| {
            if let Some(image) = heatmap_image {
                self.draw(plot_ui, image);
            } else {
                log::error!("Failed to draw image: {}", self.name);
            }
        })
        .response
        .context_menu(|ui| {
            if ui.button("Recalculate Image").clicked() {
                self.calculate_image(ui);
            }
    
            ui.separator();
    
            self.context_menu(ui);
        });
    }
}
