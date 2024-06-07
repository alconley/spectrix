use super::colormaps::ColorMap;
use super::plot_settings::EguiPlotSettings;
use fnv::FnvHashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotSettings {
    // #[serde(skip)]
    // cursor_position: Option<egui_plot::PlotPoint>,
    egui_settings: EguiPlotSettings,
    stats_info: bool,
    colormap: ColorMap,
    log_norm_colormap: bool,
}

impl Default for PlotSettings {
    fn default() -> Self {
        PlotSettings {
            // cursor_position: None,
            egui_settings: EguiPlotSettings::default(),
            stats_info: false,
            colormap: ColorMap::default(),
            log_norm_colormap: true,
        }
    }
}

impl PlotSettings {
    pub fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label("Plot Settings:");
            ui.separator();
            ui.checkbox(&mut self.stats_info, "Show Statitics");
            self.egui_settings.menu_button(ui);

            ui.menu_button("Colormap", |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.log_norm_colormap, "Log Norm");
                });
                self.colormap.color_maps_ui(ui);
            });
        });
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

    #[serde(skip)]
    texture: Option<egui::TextureHandle>,
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
            texture: None,
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

            // Update min and max counts
            if *count < self.bins.min_count {
                self.bins.min_count = *count;
            }
            if *count > self.bins.max_count {
                self.bins.max_count = *count;
            }
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
    fn to_color_image(&self) -> epaint::ColorImage {
        let width = ((self.range.x.max - self.range.x.min) / self.bins.x_width) as usize;
        let height = ((self.range.y.max - self.range.y.min) / self.bins.y_width) as usize;

        // Initialize a vector to hold pixel data
        let mut pixels = Vec::with_capacity(width * height);

        // Loop through each bin and assign colors based on counts
        // Loop starts from the top row (y=0) to the bottom row (y=height-1)
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
        epaint::ColorImage {
            size: [width, height],
            pixels,
        }
    }

    // Convert ColorImage to ImageData (Byte array)
    fn to_image_data(&self) -> epaint::ImageData {
        let color_image = self.to_color_image();
        let width = color_image.size[0];
        let height = color_image.size[1];

        // Convert Color32 pixels to a flat RGBA byte array
        let mut rgba_data = Vec::with_capacity(width * height * 4);
        for pixel in color_image.pixels.iter() {
            rgba_data.extend_from_slice(&pixel.to_array()); // Assuming Color32 provides a method like `to_array()`
        }

        epaint::ImageData::Color(
            epaint::ColorImage::from_rgba_unmultiplied([width, height], &rgba_data).into(),
        )
    }

    // Get the center of the image
    fn get_image_center(&self) -> egui_plot::PlotPoint {
        egui_plot::PlotPoint::new(
            (self.range.x.min + self.range.x.max) / 2.0,
            (self.range.y.min + self.range.y.max) / 2.0,
        )
    }

    // Get the size of the image
    fn get_image_size(&self) -> egui::Vec2 {
        egui::Vec2::new(
            (self.range.x.max - self.range.x.min) as f32,
            (self.range.y.max - self.range.y.min) as f32,
        )
    }

    // Context menu for the plot (when you right-click on the plot)
    fn context_menu(&mut self, ui: &mut egui::Ui) {
        self.plot_settings.settings_ui(ui);
    }

    // Draw the histogram on the plot
    fn draw(&self, plot_ui: &mut egui_plot::PlotUi, plot_image: egui_plot::PlotImage) {
        self.show_stats(plot_ui);
        plot_ui.image(plot_image);
    }

    // Get the PlotImage from the ui textures
    fn get_plot_image_from_texture(&mut self, ui: &mut egui::Ui) -> Option<egui_plot::PlotImage> {
        if self.texture.is_none() {
            let image_data = self.to_image_data();
            self.texture = Some(ui.ctx().load_texture(
                self.name.clone(),
                image_data,
                egui::TextureOptions {
                    magnification: egui::TextureFilter::Nearest,
                    minification: egui::TextureFilter::Nearest,
                    wrap_mode: egui::TextureWrapMode::ClampToEdge,
                },
            ));
        }

        if let Some(texture) = &self.texture {
            Some(
                egui_plot::PlotImage::new(texture, self.get_image_center(), self.get_image_size())
                    .name(self.name.clone()),
            )
        } else {
            log::warn!("Failed to get texture for histogram: {}", self.name);
            None
        }
    }

    // Recalculate the image and replace the existing texture
    pub fn recalculate_image(&mut self, ui: &mut egui::Ui) {
        self.texture = None;
        self.get_plot_image_from_texture(ui);
    }

    // Render the histogram using egui_plot
    pub fn render(&mut self, ui: &mut egui::Ui) {
        let mut plot = egui_plot::Plot::new(self.name.clone());
        plot = self.plot_settings.egui_settings.apply_to_plot(plot);

        let heatmap_image = self.get_plot_image_from_texture(ui);

        plot.show(ui, |plot_ui| {
            if let Some(image) = heatmap_image {
                self.draw(plot_ui, image);
            }
        })
        .response
        .context_menu(|ui| {
            if ui.button("Recalculate Image").clicked() {
                self.recalculate_image(ui);
            }

            ui.separator();

            self.context_menu(ui);
        });
    }
}
