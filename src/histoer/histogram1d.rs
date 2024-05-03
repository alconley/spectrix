#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Histogram {
    pub name: String,
    pub bins: Vec<u32>,
    pub range: (f64, f64),
    pub bin_width: f64,
}

impl Histogram {
    // Create a new Histogram with specified min, max, and number of bins
    pub fn new(name: &str, number_of_bins: usize, range: (f64, f64)) -> Self {
        Histogram {
            name: name.to_string(),
            bins: vec![0; number_of_bins],
            range,
            bin_width: (range.1 - range.0) / number_of_bins as f64,
        }
    }

    // Add a value to the histogram
    pub fn fill(&mut self, value: f64) {
        if value >= self.range.0 && value < self.range.1 {
            let index = ((value - self.range.0) / self.bin_width) as usize;
            if index < self.bins.len() {
                self.bins[index] += 1;
            }
        }
    }

    // Get the bin number for a given x position.
    pub fn get_bin(&self, x: f64) -> Option<usize> {
        if x < self.range.0 || x > self.range.1 {
            return None;
        }

        let bin_index: usize = ((x - self.range.0) / self.bin_width).floor() as usize;

        Some(bin_index)
    }

    fn stats(&self, start_x: f64, end_x: f64) -> (u32, f64, f64) {
        let start_bin = self.get_bin(start_x).unwrap_or(0);
        let end_bin = self.get_bin(end_x).unwrap_or(self.bins.len() - 1);

        let mut sum_product = 0.0;
        let mut total_count = 0;

        for bin in start_bin..=end_bin {
            if bin < self.bins.len() {
                let bin_center =
                    self.range.0 + (bin as f64 * self.bin_width) + self.bin_width * 0.5;
                sum_product += self.bins[bin] as f64 * bin_center;
                total_count += self.bins[bin];
            } else {
                break;
            }
        }

        if total_count == 0 {
            (0, 0.0, 0.0)
        } else {
            let mean = sum_product / total_count as f64;

            let mut sum_squared_diff = 0.0;

            for bin in start_bin..=end_bin {
                if bin < self.bins.len() {
                    let bin_center =
                        self.range.0 + (bin as f64 * self.bin_width) + (self.bin_width * 0.5);
                    let diff = bin_center - mean;
                    sum_squared_diff += self.bins[bin] as f64 * diff * diff;
                } else {
                    break;
                }
            }

            let stdev = (sum_squared_diff / total_count as f64).sqrt();

            (total_count, mean, stdev)
        }
    }

    fn step_histogram_points(&self) -> Vec<(f64, f64)> {
        let mut line_points: Vec<(f64, f64)> = Vec::new();

        for (index, &count) in self.bins.iter().enumerate() {
            let start = self.range.0 + index as f64 * self.bin_width; // Start of the bin
            let end = start + self.bin_width; // End of the bin

            // Add points for the line at the start and end of each bar
            line_points.push((start, count as f64));
            line_points.push((end, count as f64));
        }

        line_points
    }

    /// Generates legend entries for the histogram based on the specified x range.
    fn legend_entries(&self, start_x: f64, end_x: f64) -> Vec<String> {
        let stats = self.stats(start_x, end_x);
        let integral_text = format!("Integral: {}", stats.0);
        let mean_text = format!("Mean: {:.2}", stats.1);
        let stdev_text = format!("Stdev: {:.2}", stats.2);

        vec![integral_text, mean_text, stdev_text]
    }

    // Generates a line to form a step histogram using egui_plot
    fn egui_histogram_step(&self, color: egui::Color32) -> egui_plot::Line {
        
        let line_points = self.step_histogram_points();

        // Convert line_points to a Vec<[f64; 2]>
        let plot_points: egui_plot::PlotPoints = line_points.iter().map(|&(x, y)| [x, y]).collect();

        egui_plot::Line::new(plot_points)
            .color(color)
            .name(self.name.clone())
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        /* For custom 2d histogram plot manipulation settings*/
        let (scroll, pointer_down, modifiers) = ui.input(|i| {
            let scroll = i.events.iter().find_map(|e| match e {
                egui::Event::MouseWheel { delta, .. } => Some(*delta),
                _ => None,
            });
            (scroll, i.pointer.primary_down(), i.modifiers)
        });

        let plot = egui_plot::Plot::new(self.name.clone())
            .legend(egui_plot::Legend::default())
            .allow_drag(false)
            .allow_zoom(false)
            .allow_boxed_zoom(true)
            .auto_bounds(egui::Vec2b::new(true, true))
            .allow_scroll(false);

        let color = if ui.ctx().style().visuals.dark_mode {
            // check if the ui is in dark mode.
            // Light blue looks nice on dark mode but hard to see in light mode.
            egui::Color32::LIGHT_BLUE
        } else {
            egui::Color32::BLACK
        };

        plot.show(ui, |plot_ui| {
            custom_plot_manipulation(plot_ui, scroll, pointer_down, modifiers);

            let plot_min_x = plot_ui.plot_bounds().min()[0];
            let plot_max_x = plot_ui.plot_bounds().max()[0];

            let step_line = self.egui_histogram_step(color);

            plot_ui.line(step_line);

            let stats_entries = self.legend_entries(plot_min_x, plot_max_x);
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
