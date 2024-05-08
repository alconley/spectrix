use fnv::FnvHashMap;

// Define the BarData struct
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BarData {
    pub x: f64,
    pub y: f64,
    pub bar_width: f64,
    pub height: f64,
    pub count: u32,
}

// uses a hash map to store the histogram data (zero overhead for empty bins)
#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Histogram2D {
    pub name: String,
    pub bins: FnvHashMap<(usize, usize), u32>,
    pub x_range: (f64, f64),
    pub y_range: (f64, f64),
    pub x_bin_width: f64,
    pub y_bin_width: f64,
    pub min_count: u32,
    pub max_count: u32,

    #[serde(skip)]
    texture: Option<egui::TextureHandle>,
}

impl Histogram2D {
    // Create a new 2D Histogram with specified ranges and number of bins for each axis
    pub fn new(
        name: &str,
        x_bins: usize,
        x_range: (f64, f64),
        y_bins: usize,
        y_range: (f64, f64),
    ) -> Self {
        Histogram2D {
            name: name.to_string(),
            bins: FnvHashMap::default(),
            x_range,
            y_range,
            x_bin_width: (x_range.1 - x_range.0) / x_bins as f64,
            y_bin_width: (y_range.1 - y_range.0) / y_bins as f64,
            min_count: u32::MAX,
            max_count: u32::MIN,
            texture: None,
        }
    }

    // Add a value to the histogram
    pub fn fill(&mut self, x_value: f64, y_value: f64) {
        if x_value >= self.x_range.0
            && x_value < self.x_range.1
            && y_value >= self.y_range.0
            && y_value < self.y_range.1
        {
            let x_index = ((x_value - self.x_range.0) / self.x_bin_width) as usize;
            let y_index = ((y_value - self.y_range.0) / self.y_bin_width) as usize;
            let count = self.bins.entry((x_index, y_index)).or_insert(0);
            *count += 1;

            // Update min and max counts
            if *count < self.min_count {
                self.min_count = *count;
            }
            if *count > self.max_count {
                self.max_count = *count;
            }
        }
    }

    fn get_bin_x(&self, x: f64) -> Option<usize> {
        if x < self.x_range.0 || x > self.x_range.1 {
            return None;
        }

        let bin_index: usize = ((x - self.x_range.0) / self.x_bin_width).floor() as usize;

        Some(bin_index)
    }

    fn get_bin_y(&self, y: f64) -> Option<usize> {
        if y < self.y_range.0 || y > self.y_range.1 {
            return None;
        }

        let bin_index: usize = ((y - self.y_range.0) / self.y_bin_width).floor() as usize;

        Some(bin_index)
    }

    fn stats(
        &self,
        start_x: f64,
        end_x: f64,
        start_y: f64,
        end_y: f64,
    ) -> (u32, f64, f64, f64, f64) {
        let start_x_index = self.get_bin_x(start_x).unwrap_or(0);
        let end_x_index = self
            .get_bin_x(end_x)
            .unwrap_or_else(|| self.bins.keys().max_by_key(|k| k.0).map_or(0, |k| k.0));

        let start_y_index = self.get_bin_y(start_y).unwrap_or(0);
        let end_y_index = self
            .get_bin_y(end_y)
            .unwrap_or_else(|| self.bins.keys().max_by_key(|k| k.1).map_or(0, |k| k.1));

        let mut total_count = 0;

        let mut sum_product_x = 0.0;
        let mut sum_product_y = 0.0;

        for (&(x_index, y_index), &count) in self.bins.iter() {
            if x_index >= start_x_index
                && x_index <= end_x_index
                && y_index >= start_y_index
                && y_index <= end_y_index
            {
                let bin_center_x =
                    self.x_range.0 + (x_index as f64 * self.x_bin_width) + self.x_bin_width * 0.5;
                let bin_center_y =
                    self.y_range.0 + (y_index as f64 * self.y_bin_width) + self.y_bin_width * 0.5;

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

            for (&(x_index, y_index), &count) in self.bins.iter() {
                if x_index >= start_x_index
                    && x_index <= end_x_index
                    && y_index >= start_y_index
                    && y_index <= end_y_index
                {
                    let bin_center_x = self.x_range.0
                        + (x_index as f64 * self.x_bin_width)
                        + self.x_bin_width * 0.5;
                    let bin_center_y = self.y_range.0
                        + (y_index as f64 * self.y_bin_width)
                        + self.y_bin_width * 0.5;

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

    /// Generates legend entries for the histogram based on the specified x range.
    fn legend_entries(&self, start_x: f64, end_x: f64, start_y: f64, end_y: f64) -> Vec<String> {
        let stats = self.stats(start_x, end_x, start_y, end_y);
        let integral_text = format!("Integral: {}", stats.0);
        let mean_x_text = format!("Mean X: {:.2}", stats.1);
        let stdev_x_text = format!("Stdev X: {:.2}", stats.2);
        let mean_y_text = format!("Mean Y: {:.2}", stats.3);
        let stdev_y_text = format!("Stdev Y: {:.2}", stats.4);

        vec![
            integral_text,
            mean_x_text,
            stdev_x_text,
            mean_y_text,
            stdev_y_text,
        ]
    }

    fn to_color_image(&self) -> epaint::ColorImage {
        let width = ((self.x_range.1 - self.x_range.0) / self.x_bin_width) as usize;
        let height = ((self.y_range.1 - self.y_range.0) / self.y_bin_width) as usize;

        // Initialize a vector to hold pixel data
        let mut pixels = Vec::with_capacity(width * height);

        // Loop through each bin and assign colors based on counts
        // Loop starts from the top row (y=0) to the bottom row (y=height-1)
        for y in 0..height {
            for x in 0..width {
                let count = self.bins.get(&(x, height - y - 1)).cloned().unwrap_or(0);
                let color = if count == 0 {
                    egui::Color32::TRANSPARENT // Use transparent for zero counts
                } else {
                    viridis_colormap(count, self.min_count, self.max_count)
                };
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

    pub fn render(&mut self, ui: &mut egui::Ui) {
        // Check if texture is loaded, if not load it
        if self.texture.is_none() {
            let image_data = self.to_image_data(); // Convert the histogram to image data once
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
            let plot = egui_plot::Plot::new(self.name.clone())
                .allow_zoom(false)
                .allow_drag(false)
                .allow_scroll(false)
                .legend(egui_plot::Legend::default())
                .auto_bounds(egui::Vec2b::new(true, true));

            let color = if ui.ctx().style().visuals.dark_mode {
                egui::Color32::LIGHT_BLUE
            } else {
                egui::Color32::DARK_BLUE
            };

            /* For custom plot manipulation settings*/
            let (scroll, pointer_down, modifiers) = ui.input(|i| {
                let scroll = i.events.iter().find_map(|e| match e {
                    egui::Event::MouseWheel { delta, .. } => Some(*delta),
                    _ => None,
                });
                (scroll, i.pointer.primary_down(), i.modifiers)
            });

            // Calculate the center position
            let center_x = (self.x_range.0 + self.x_range.1) / 2.0;
            let center_y = (self.y_range.0 + self.y_range.1) / 2.0;

            // Calculate the size of the image in plot coordinates
            let size_x = (self.x_range.1 - self.x_range.0) as f32;
            let size_y = (self.y_range.1 - self.y_range.0) as f32;

            let heatmap_image = egui_plot::PlotImage::new(
                &texture.clone(),
                egui_plot::PlotPoint::new(center_x, center_y),
                egui::Vec2::new(size_x, size_y),
            )
            .name(self.name.clone());

            plot.show(ui, |plot_ui| {
                custom_plot_manipulation(plot_ui, scroll, pointer_down, modifiers);

                let plot_min_x = plot_ui.plot_bounds().min()[0];
                let plot_max_x = plot_ui.plot_bounds().max()[0];
                let plot_min_y = plot_ui.plot_bounds().min()[1];
                let plot_max_y = plot_ui.plot_bounds().max()[1];

                // make bars instead of image
                // let heatmap = self.egui_heatmap();
                // plot_ui.bar_chart(heatmap.color(color));

                plot_ui.image(heatmap_image);

                let stats_entries =
                    self.legend_entries(plot_min_x, plot_max_x, plot_min_y, plot_max_y);

                for entry in stats_entries.iter() {
                    plot_ui.text(
                        egui_plot::Text::new(egui_plot::PlotPoint::new(0, 0), " ") // Placeholder for positioning; adjust as needed
                            .highlight(false)
                            .color(color)
                            .name(entry),
                    );
                }
            });
        }
    }

    /*

    // This was my inital method to make a heatmap.
    // Found the performace to be lacking when there was a large number of bins or plots on the screen.

    // Method to generate data for egui heatmap
    fn generate_bar_data(&self) -> Vec<BarData> {
        let mut bars = Vec::new();

        for (&(x_index, y_index), &count) in &self.bins {
            if count == 0 {
                continue; // Skip empty bins
            }

            let x_bin_start = self.x_range.0 + x_index as f64 * self.x_bin_width;
            let x_bin_end = x_bin_start + self.x_bin_width;
            let y_bin_start = self.y_range.0 + y_index as f64 * self.y_bin_width;
            let y_bin_end = y_bin_start + self.y_bin_width;

            bars.push(BarData {
                x: (x_bin_start + x_bin_end) / 2.0,
                y: (y_bin_start + y_bin_end) / 2.0,
                bar_width: self.x_bin_width,
                height: self.y_bin_width,
                count,
            });
        }

        bars
    }

    fn egui_heatmap(&self) -> egui_plot::BarChart {

        let bars_data = self.generate_bar_data();
        let mut bars = Vec::new();

        let min: u32 = self.min_count;
        let max: u32 = self.max_count;
        for bar_data in bars_data {
            let color: egui::Color32 = viridis_colormap(bar_data.count, min, max); // Determine color based on the count, using a colormap.

            let bar = egui_plot::Bar {
                orientation: egui_plot::Orientation::Vertical,
                argument: bar_data.x,
                value: bar_data.height,
                bar_width: bar_data.bar_width,
                fill: color,
                stroke: egui::Stroke::new(1.0, color),
                name: format!("x = {}\ny = {}\n{}", bar_data.x, bar_data.y, bar_data.count),
                base_offset: Some(bar_data.y - bar_data.height / 2.0),
            };
            bars.push(bar);
        }

        // Return a BarChart object if the histogram exists, otherwise return None.
        egui_plot::BarChart::new(bars).name(self.name.clone())

    }

    */
}

// Function to generate a color based on a value using the Viridis colormap, the matplotlib default.
fn viridis_colormap(value: u32, min: u32, max: u32) -> egui::Color32 {
    // Handle case where min == max to avoid division by zero
    let normalized: f64 = if max > min {
        (value as f64 - min as f64) / (max as f64 - min as f64)
    } else {
        0.0
    }
    .clamp(0.0, 1.0);

    // Key colors from the Viridis colormap
    let viridis_colors: [(f32, f32, f32); 32] = [
        (0.267_003_98, 0.004872566, 0.329_415_08),
        (0.277_229, 0.051716984, 0.376_949_9),
        (0.282_479_7, 0.097334964, 0.419_510_57),
        (0.282_711_27, 0.139_317_69, 0.456_197_05),
        (0.278_092_62, 0.179_895_88, 0.486_377_42),
        (0.269_137_8, 0.219_429_66, 0.50989087),
        (0.256_733_54, 0.257_754_4, 0.527_183_8),
        (0.242_031_46, 0.294_643_82, 0.539_209),
        (0.226_243_75, 0.329_989_34, 0.547_162_83),
        (0.210_443_17, 0.363_856_05, 0.552_221_3),
        (0.195_412_49, 0.396_435_86, 0.555_350_9),
        (0.181_477_32, 0.428_017_32, 0.557_198_9),
        (0.168_574_23, 0.458_905_25, 0.558_067_3),
        (0.156_365_95, 0.489_384_6, 0.557_941_2),
        (0.144_535_29, 0.519_685_6, 0.556_527_7),
        (0.133_249_55, 0.549_958_2, 0.553_339_24),
        (0.123_833_07, 0.580_259_26, 0.547_771_63),
        (0.119_442_11, 0.610_546_23, 0.539_182),
        (0.124_881_9, 0.640_695_04, 0.526_954_95),
        (0.144_277_74, 0.670_499_74, 0.510_554_73),
        (0.178_281_44, 0.699_705_66, 0.489_567_13),
        (0.224_797_44, 0.728_014_4, 0.463_677_88),
        (0.281_243_44, 0.755_097_75, 0.432_683_2),
        (0.345_693_5, 0.780_604_8, 0.396_465_7),
        (0.416_705_43, 0.80418531, 0.355_029_97),
        (0.493_228_82, 0.825_506_2, 0.308_497_67),
        (0.574_270_25, 0.844_288_8, 0.257_257_7),
        (0.658_654_03, 0.860_389_95, 0.202_434_47),
        (0.744_780_54, 0.873_933, 0.147_547_83),
        (0.830_610_04, 0.885_437_7, 0.10427358),
        (0.914_002_4, 0.895_811_26, 0.100134278),
        (0.993_248_16, 0.906_154_75, 0.143_935_95),
    ];

    // Interpolate between colors in the colormap
    let scaled_val: f64 = normalized * (viridis_colors.len() - 1) as f64;
    let index: usize = scaled_val.floor() as usize;
    let fraction: f32 = scaled_val.fract() as f32;

    let color1: (f32, f32, f32) = viridis_colors[index];
    let color2: (f32, f32, f32) = viridis_colors[(index + 1).min(viridis_colors.len() - 1)];

    let red: f32 = (color1.0 + fraction * (color2.0 - color1.0)) * 255.0;
    let green: f32 = (color1.1 + fraction * (color2.1 - color1.1)) * 255.0;
    let blue: f32 = (color1.2 + fraction * (color2.2 - color1.2)) * 255.0;

    egui::Color32::from_rgb(red as u8, green as u8, blue as u8)
}

fn custom_plot_manipulation(
    plot_ui: &mut egui_plot::PlotUi,
    scroll: Option<egui::Vec2>,
    pointer_down: bool,
    modifiers: egui::Modifiers,
) {
    /* For custom plot manipulation settings, add this before the plot.show()
        let (scroll, pointer_down, modifiers) = ui.input(|i| {
            let scroll = i.events.iter().find_map(|e| match e {
                egui::Event::MouseWheel { delta, .. } => Some(*delta),
                _ => None,
            });
            (scroll, i.pointer.primary_down(), i.modifiers)
        });
    */

    if plot_ui.response().hovered() {
        if let Some(mut scroll) = scroll {
            // Default behavior for zooming and panning, with fixed parameters
            let lock_x = false;
            let lock_y = false;
            let zoom_speed = 0.1; // Default zoom speed
            let scroll_speed = 1.0; // Default scroll speed
            let ctrl_to_zoom = false;
            let shift_to_horizontal = false;

            if modifiers.ctrl == ctrl_to_zoom {
                scroll = egui::Vec2::splat(scroll.x + scroll.y);
                let mut zoom_factor = egui::Vec2::from([
                    (scroll.x * zoom_speed / 15.0).exp(),
                    (scroll.y * zoom_speed / 15.0).exp(),
                ]);
                if lock_x {
                    zoom_factor.x = 1.0;
                }
                if lock_y {
                    zoom_factor.y = 1.0;
                }
                plot_ui.zoom_bounds_around_hovered(zoom_factor);
            } else {
                if modifiers.shift == shift_to_horizontal {
                    scroll = egui::Vec2::new(scroll.y, scroll.x);
                }
                if lock_x {
                    scroll.x = 0.0;
                }
                if lock_y {
                    scroll.y = 0.0;
                }
                let delta_pos = scroll_speed * scroll;
                plot_ui.translate_bounds(delta_pos);
            }
        }

        if pointer_down {
            let pointer_translate = -plot_ui.pointer_coordinate_drag_delta();
            // Lock axis functionality removed for simplification, add if needed
            plot_ui.translate_bounds(pointer_translate);
        }
    }
}
