use super::colormaps::ColorMap;
use super::histogram1d::Histogram;
use super::plot_settings::EguiPlotSettings;
use crate::egui_plot_stuff::egui_horizontal_line::EguiHorizontalLine;
use crate::egui_plot_stuff::egui_image::EguiImage;
use crate::egui_plot_stuff::egui_polygon::EguiPolygon;
use crate::egui_plot_stuff::egui_vertical_line::EguiVerticalLine;
use fnv::FnvHashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotSettings {
    #[serde(skip)]
    cursor_position: Option<egui_plot::PlotPoint>,
    egui_settings: EguiPlotSettings,
    cut_polygons: Vec<EguiPolygon>,
    stats_info: bool,
    colormap: ColorMap,
    log_norm_colormap: bool,
    projections: Projections,
}
impl Default for PlotSettings {
    fn default() -> Self {
        PlotSettings {
            cursor_position: None,
            egui_settings: EguiPlotSettings::default(),
            cut_polygons: Vec::new(),
            stats_info: false,
            colormap: ColorMap::default(),
            log_norm_colormap: true,
            projections: Projections::new(),
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

        self.projections.menu_button(ui);

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
            // polygon.mouse_interactions(plot_ui);
            polygon.draw(plot_ui);
        }

        self.projections.draw(plot_ui);
    }

    pub fn interactive_response(&mut self, plot_response: &egui_plot::PlotResponse<()>) {
        self.projections.interactive_dragging(plot_response);
        for polygon in self.cut_polygons.iter_mut() {
            polygon.handle_interactions(plot_response);
        }
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Projections {
    pub add_y_projection: bool,
    pub y_projection: Option<Histogram>,
    pub y_projection_line_1: EguiVerticalLine,
    pub y_projection_line_2: EguiVerticalLine,

    pub add_x_projection: bool,
    pub x_projection: Option<Histogram>,
    pub x_projection_line_1: EguiHorizontalLine,
    pub x_projection_line_2: EguiHorizontalLine,
}
impl Projections {
    pub fn new() -> Self {
        Projections {
            add_y_projection: false,
            y_projection: None,
            y_projection_line_1: EguiVerticalLine {
                name: "Y Projection Line 1".to_string(),
                ..EguiVerticalLine::default()
            },
            y_projection_line_2: EguiVerticalLine {
                name: "Y Projection Line 2".to_string(),
                ..EguiVerticalLine::default()
            },

            add_x_projection: false,
            x_projection: None,
            x_projection_line_1: EguiHorizontalLine {
                name: "X Projection Line 1".to_string(),
                ..EguiHorizontalLine::default()
            },
            x_projection_line_2: EguiHorizontalLine {
                name: "X Projection Line 2".to_string(),
                ..EguiHorizontalLine::default()
            },
        }
    }

    fn show_y_projection(&mut self, ui: &mut egui::Ui) {
        if self.add_y_projection && self.y_projection.is_some() {
            ui.ctx().show_viewport_immediate(
                egui::ViewportId::from_hash_of(
                    self.y_projection.as_ref().unwrap().name.to_string(),
                ),
                egui::ViewportBuilder::default()
                    .with_title(self.y_projection.as_ref().unwrap().name.to_string())
                    .with_inner_size([600.0, 400.0]),
                |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Immediate,
                        "This egui backend doesn't support multiple viewports"
                    );

                    egui::CentralPanel::default().show(ctx, |ui| {
                        if let Some(histogram) = &mut self.y_projection {
                            histogram.render(ui);
                        }
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        // Tell parent viewport that we should not show next frame:
                        self.y_projection = None;
                    }
                },
            );
        }
    }

    fn show_x_projection(&mut self, ui: &mut egui::Ui) {
        if self.add_x_projection && self.x_projection.is_some() {
            ui.ctx().show_viewport_immediate(
                egui::ViewportId::from_hash_of(
                    self.x_projection.as_ref().unwrap().name.to_string(),
                ),
                egui::ViewportBuilder::default()
                    .with_title(self.x_projection.as_ref().unwrap().name.to_string())
                    .with_inner_size([600.0, 400.0]),
                |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Immediate,
                        "This egui backend doesn't support multiple viewports"
                    );

                    egui::CentralPanel::default().show(ctx, |ui| {
                        if let Some(histogram) = &mut self.x_projection {
                            histogram.render(ui);
                        }
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        // Tell parent viewport that we should not show next frame:
                        self.x_projection = None;
                    }
                },
            );
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.show_y_projection(ui);
        self.show_x_projection(ui);
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi) {
        if self.add_y_projection {
            self.y_projection_line_1.draw(plot_ui);
            self.y_projection_line_2.draw(plot_ui);
        }

        if self.add_x_projection {
            self.x_projection_line_1.draw(plot_ui);
            self.x_projection_line_2.draw(plot_ui);
        }
    }

    pub fn interactive_dragging(&mut self, plot_response: &egui_plot::PlotResponse<()>) {
        if self.add_y_projection {
            self.y_projection_line_1.interactive_dragging(plot_response);
            self.y_projection_line_2.interactive_dragging(plot_response);
        }

        if self.add_x_projection {
            self.x_projection_line_1.interactive_dragging(plot_response);
            self.x_projection_line_2.interactive_dragging(plot_response);
        }
    }

    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
        ui.heading("Projections");

        ui.checkbox(&mut self.add_y_projection, "Add Y Projection");
        ui.horizontal(|ui| {
            if self.add_y_projection {
                ui.add(
                    egui::DragValue::new(&mut self.y_projection_line_1.x_value)
                        .speed(1.0)
                        .prefix("X1: "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.y_projection_line_2.x_value)
                        .speed(1.0)
                        .prefix("X2: "),
                );
            }
        });

        ui.checkbox(&mut self.add_x_projection, "Add X Projection");
        ui.horizontal(|ui| {
            if self.add_x_projection {
                ui.add(
                    egui::DragValue::new(&mut self.x_projection_line_1.y_value)
                        .speed(1.0)
                        .prefix("Y1: "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.x_projection_line_2.y_value)
                        .speed(1.0)
                        .prefix("Y2: "),
                );
            }
        });

        ui.label("Press 'P' to calculate the projections");
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
        }
    }

    // Add a value to the histogram
    pub fn fill(&mut self, x_value: f64, y_value: f64) {
        if x_value >= self.range.x.min
            && x_value < self.range.x.max
            && y_value >= self.range.y.min
            && y_value < self.range.y.max
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
                let count = self
                    .bins
                    .counts
                    .get(&(x, height - y - 1))
                    .cloned()
                    .unwrap_or(0);
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
        egui::ColorImage {
            size: [width, height],
            pixels,
        }
    }

    // Recalculate the image and replace the existing texture
    pub fn calculate_image(&mut self, ui: &mut egui::Ui) {
        self.image.texture = None;
        let color_image = self.data_2_image();
        self.image.get_texture(ui, color_image);
    }

    pub fn y_projection(&mut self, x_min: f64, x_max: f64) -> Histogram {
        // Extract the y-projection data
        let mut y_bins = vec![0; self.bins.y];

        for ((x_index, y_index), &count) in &self.bins.counts {
            let x_center = self.range.x.min + (*x_index as f64 + 0.5) * self.bins.x_width;
            if x_center >= x_min && x_center < x_max && *y_index < y_bins.len() {
                y_bins[*y_index] += count;
            }
        }

        // Create a new Histogram for the y-projection
        let mut y_histogram = Histogram::new(
            &format!("Y-Projection of {} between {}-{}", self.name, x_min, x_max),
            self.bins.y,
            (self.range.y.min, self.range.y.max),
        );

        // Fill the y-projection histogram
        y_histogram.bins = y_bins;

        y_histogram
    }

    pub fn x_projection(&mut self, y_min: f64, y_max: f64) -> Histogram {
        // Extract the x-projection data
        let mut x_bins = vec![0; self.bins.x];

        for ((x_index, y_index), &count) in &self.bins.counts {
            let y_center = self.range.y.min + (*y_index as f64 + 0.5) * self.bins.y_width;
            if y_center >= y_min && y_center < y_max && *x_index < x_bins.len() {
                x_bins[*x_index] += count;
            }
        }

        // Create a new Histogram for the x-projection
        let mut x_histogram = Histogram::new(
            &format!("X-Projection of {} between {}-{}", self.name, y_min, y_max),
            self.bins.x,
            (self.range.x.min, self.range.x.max),
        );

        // Fill the x-projection histogram
        x_histogram.bins = x_bins;

        x_histogram
    }

    pub fn keybinds(&mut self, ui: &mut egui::Ui) {
        if let Some(_cursor_position) = self.plot_settings.cursor_position {
            if ui.input(|i| i.key_pressed(egui::Key::P)) {
                if self.plot_settings.projections.add_y_projection {
                    let x1 = self.plot_settings.projections.y_projection_line_1.x_value;
                    let x2 = self.plot_settings.projections.y_projection_line_2.x_value;
                    let (min_x, max_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) }; // sort the x values
                    self.plot_settings.projections.y_projection =
                        Some(self.y_projection(min_x, max_x));
                }

                if self.plot_settings.projections.add_x_projection {
                    let y1 = self.plot_settings.projections.x_projection_line_1.y_value;
                    let y2 = self.plot_settings.projections.x_projection_line_2.y_value;
                    let (min_y, max_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) }; // sort the y values
                    self.plot_settings.projections.x_projection =
                        Some(self.x_projection(min_y, max_y));
                }
            }
        }
    }

    // Context menu for the plot (when you right-click on the plot)
    fn context_menu(&mut self, ui: &mut egui::Ui) {
        self.image.menu_button(ui);
        self.plot_settings.settings_ui(ui);
    }

    // Draw the histogram on the plot
    fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi) {
        self.show_stats(plot_ui);

        let heatmap_image = self.image.get_plot_image_from_texture();

        if let Some(image) = heatmap_image {
            self.image.draw(plot_ui, image);
        }
        // self.image.draw(plot_ui, plot_image);
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

        self.plot_settings.projections.show(ui);

        let plot_response = plot.show(ui, |plot_ui| {
            self.draw(plot_ui);
        });

        plot_response.response.context_menu(|ui| {
            if ui.button("Recalculate Image").clicked() {
                self.calculate_image(ui);
            }
            self.context_menu(ui);
        });

        self.plot_settings.interactive_response(&plot_response);
    }
}
