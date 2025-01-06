use core::f64;

use crate::egui_plot_stuff::egui_horizontal_line::EguiHorizontalLine;
use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::egui_plot_stuff::egui_vertical_line::EguiVerticalLine;
use crate::histoer::histo1d::histogram1d::Histogram;

use super::histogram2d::Histogram2D;

impl Histogram2D {
    pub fn y_projection(&self, x_min: f64, x_max: f64) -> Vec<u64> {
        // Extract the y-projection data
        let mut y_bins = vec![0; self.bins.y];

        for ((x_index, y_index), &count) in &self.bins.counts {
            let x_center = self.range.x.min + (*x_index as f64 + 0.5) * self.bins.x_width;
            if x_center >= x_min && x_center < x_max && *y_index < y_bins.len() {
                y_bins[*y_index] += count;
            }
        }

        y_bins
    }

    pub fn x_projection(&self, y_min: f64, y_max: f64) -> Vec<u64> {
        // Extract the x-projection data
        let mut x_bins = vec![0; self.bins.x];

        for ((x_index, y_index), &count) in &self.bins.counts {
            let y_center = self.range.y.min + (*y_index as f64 + 0.5) * self.bins.y_width;
            if y_center >= y_min && y_center < y_max && *x_index < x_bins.len() {
                x_bins[*x_index] += count;
            }
        }

        x_bins
    }

    pub fn check_projections(&mut self) {
        if self.plot_settings.projections.add_y_projection {
            let x1 = self.plot_settings.projections.y_projection_line_1.x_value;
            let x2 = self.plot_settings.projections.y_projection_line_2.x_value;
            let (min_x, max_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) }; // sort the x values

            if self.plot_settings.projections.y_projection.is_some() {
                if self.plot_settings.projections.dragging {
                    let bins = self.y_projection(min_x, max_x);

                    self.plot_settings
                        .projections
                        .y_projection
                        .as_mut()
                        .unwrap()
                        .plot_settings
                        .rebin_factor = 1;

                    self.plot_settings
                        .projections
                        .y_projection
                        .as_mut()
                        .unwrap()
                        .rebin();

                    self.plot_settings
                        .projections
                        .y_projection
                        .as_mut()
                        .unwrap()
                        .bins = bins.clone();

                    self.plot_settings
                        .projections
                        .y_projection
                        .as_mut()
                        .unwrap()
                        .original_bins = bins;

                    self.plot_settings
                        .projections
                        .y_projection
                        .as_mut()
                        .unwrap()
                        .plot_settings
                        .egui_settings
                        .reset_axis = true;
                }
            } else {
                // create a new histogram and set the bins
                let mut y_histogram = Histogram::new(
                    &format!("Y-Projection of {}", self.name),
                    self.bins.y,
                    (self.range.y.min, self.range.y.max),
                );
                let bins = self.y_projection(min_x, max_x);
                y_histogram.bins = bins.clone();
                y_histogram.original_bins = bins;
                y_histogram.plot_settings.rebin_factor = 1;
                y_histogram.rebin();
                y_histogram.line.color = egui::Color32::from_rgb(255, 0, 0);

                self.plot_settings.projections.y_projection = Some(y_histogram);

                // set the projection range to be the min/max values of the histogram
                self.plot_settings.projections.y_projection_line_1.x_value = self.range.x.min;
                self.plot_settings.projections.y_projection_line_2.x_value = self.range.x.max;

                self.plot_settings
                    .projections
                    .y_projection
                    .as_mut()
                    .unwrap()
                    .plot_settings
                    .egui_settings
                    .reset_axis = true;
            }

            // Update fill_y_line with top-left and top-right points only
            self.plot_settings.projections.fill_y_line.points = vec![
                (min_x, self.range.y.max).into(), // Top-left
                (max_x, self.range.y.max).into(), // Top-right
            ];

            self.plot_settings.projections.fill_y_line.fill = self.range.y.min as f32;
        }

        if self.plot_settings.projections.add_x_projection {
            let y1 = self.plot_settings.projections.x_projection_line_1.y_value;
            let y2 = self.plot_settings.projections.x_projection_line_2.y_value;
            let (min_y, max_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) }; // sort the y values

            if self.plot_settings.projections.x_projection.is_some() {
                if self.plot_settings.projections.dragging {
                    let bins = self.x_projection(min_y, max_y);

                    self.plot_settings
                        .projections
                        .x_projection
                        .as_mut()
                        .unwrap()
                        .plot_settings
                        .rebin_factor = 1;

                    self.plot_settings
                        .projections
                        .x_projection
                        .as_mut()
                        .unwrap()
                        .rebin();

                    self.plot_settings
                        .projections
                        .x_projection
                        .as_mut()
                        .unwrap()
                        .bins = bins.clone();

                    self.plot_settings
                        .projections
                        .x_projection
                        .as_mut()
                        .unwrap()
                        .original_bins = bins;

                    self.plot_settings
                        .projections
                        .x_projection
                        .as_mut()
                        .unwrap()
                        .plot_settings
                        .egui_settings
                        .reset_axis = true;
                }
            } else {
                let mut x_histogram = Histogram::new(
                    &format!("X-Projection of {}", self.name),
                    self.bins.x,
                    (self.range.x.min, self.range.x.max),
                );
                let bins = self.x_projection(min_y, max_y);
                x_histogram.plot_settings.rebin_factor = 1;
                x_histogram.rebin();
                x_histogram.bins = bins.clone();
                x_histogram.original_bins = bins;
                x_histogram.line.color = egui::Color32::from_rgb(0, 0, 255);

                self.plot_settings.projections.x_projection = Some(x_histogram);

                // set the projection range to be the min/max values of the histogram
                self.plot_settings.projections.x_projection_line_1.y_value = self.range.y.min;
                self.plot_settings.projections.x_projection_line_2.y_value = self.range.y.max;

                self.plot_settings
                    .projections
                    .x_projection
                    .as_mut()
                    .unwrap()
                    .plot_settings
                    .egui_settings
                    .reset_axis = true;
            }

            // Update fill_x_line with top-left and top-right points only
            self.plot_settings.projections.fill_x_line.points = vec![
                (self.range.x.min, max_y).into(), // Top-left
                (self.range.x.max, max_y).into(), // Top-right
            ];

            // Set the reference fill to the minimum X value
            self.plot_settings.projections.fill_x_line.fill = min_y as f32;
        }
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Projections {
    pub add_y_projection: bool,
    pub y_projection: Option<Histogram>,
    pub y_projection_line_1: EguiVerticalLine,
    pub y_projection_line_2: EguiVerticalLine,
    pub fill_y_line: EguiLine,

    pub add_x_projection: bool,
    pub x_projection: Option<Histogram>,
    pub x_projection_line_1: EguiHorizontalLine,
    pub x_projection_line_2: EguiHorizontalLine,
    pub fill_x_line: EguiLine,

    pub dragging: bool,
}
impl Projections {
    pub fn new() -> Self {
        Projections {
            add_y_projection: false,
            y_projection: None,
            y_projection_line_1: EguiVerticalLine {
                name: "Y Projection Line 1".to_string(),
                color: egui::Color32::from_rgb(255, 0, 0),
                mid_point_radius: 5.0,
                x_value: 0.0,
                ..EguiVerticalLine::default()
            },
            y_projection_line_2: EguiVerticalLine {
                name: "Y Projection Line 2".to_string(),
                color: egui::Color32::from_rgb(255, 0, 0),
                mid_point_radius: 5.0,
                x_value: 4096.0,
                ..EguiVerticalLine::default()
            },
            fill_y_line: EguiLine {
                name: "Y Projection Fill".to_string(),
                draw: true,
                color: egui::Color32::from_rgb(255, 0, 0),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 0, 0)),
                reference_fill: true,
                ..EguiLine::default()
            },

            add_x_projection: false,
            x_projection: None,
            x_projection_line_1: EguiHorizontalLine {
                name: "X Projection Line 1".to_string(),
                color: egui::Color32::from_rgb(0, 0, 255),
                mid_point_radius: 5.0,
                y_value: 0.0,
                ..EguiHorizontalLine::default()
            },
            x_projection_line_2: EguiHorizontalLine {
                name: "X Projection Line 2".to_string(),
                color: egui::Color32::from_rgb(0, 0, 255),
                mid_point_radius: 5.0,
                y_value: 4096.0,
                ..EguiHorizontalLine::default()
            },
            fill_x_line: EguiLine {
                name: "X Projection Fill".to_string(),
                draw: true,
                color: egui::Color32::from_rgb(0, 0, 255),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 0, 255)),
                reference_fill: true,
                ..EguiLine::default()
            },
            dragging: false,
        }
    }

    fn show_y_projection(&mut self, ui: &mut egui::Ui) {
        if self.add_y_projection && self.y_projection.is_some() {
            let name = if let Some(histogram) = &self.y_projection {
                let name = histogram.name.clone();
                name.split(':').collect::<Vec<&str>>()[0].to_string()
            } else {
                "Y-Projection".to_string()
            };
            let ctx = ui.ctx().clone();
            egui::Window::new(name).show(&ctx, |ui| {
                if let Some(histogram) = &mut self.y_projection {
                    histogram.render(ui);
                }
            });
        } else {
            self.y_projection = None;
        }
    }

    fn show_x_projection(&mut self, ui: &mut egui::Ui) {
        if self.add_x_projection && self.x_projection.is_some() {
            let name = if let Some(histogram) = &self.x_projection {
                let name = histogram.name.clone();
                name.split(':').collect::<Vec<&str>>()[0].to_string()
            } else {
                "X-Projection".to_string()
            };
            let ctx = ui.ctx().clone();
            egui::Window::new(name).show(&ctx, |ui| {
                if let Some(histogram) = &mut self.x_projection {
                    histogram.render(ui);
                }
            });
        } else {
            self.x_projection = None;
        }
    }

    pub fn is_dragging(&mut self) {
        self.dragging = (self.add_y_projection
            && (self.y_projection_line_1.is_dragging || self.y_projection_line_2.is_dragging))
            || (self.add_x_projection
                && (self.x_projection_line_1.is_dragging || self.x_projection_line_2.is_dragging));
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.show_y_projection(ui);
        self.show_x_projection(ui);
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi) {
        if self.add_y_projection {
            self.y_projection_line_1.draw(plot_ui);
            self.y_projection_line_2.draw(plot_ui);
            self.fill_y_line.draw(plot_ui);
        }

        if self.add_x_projection {
            self.x_projection_line_1.draw(plot_ui);
            self.x_projection_line_2.draw(plot_ui);
            self.fill_x_line.draw(plot_ui);
        }

        self.is_dragging();
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

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.add_y_projection, "Add Y Projection").on_hover_text("Keybinds:\nY = Add Y Projection\nLeft click and drag the line at the center of the plot (cirlce)");

            let range = if let Some(histogram) = &self.y_projection {
                histogram.range
            } else {
                (0.0, 1.0)
            };
            if ui.add_enabled(
                self.add_y_projection,
                egui::DragValue::new(&mut self.y_projection_line_1.x_value)
                    .range(range.0..=range.1)
                    .speed(1.0)
                    .prefix("X1: "),
            ).changed() {
                self.dragging = true;
            }

            if ui.add_enabled(
                self.add_y_projection,
                egui::DragValue::new(&mut self.y_projection_line_2.x_value)
                    .speed(1.0)
                    .range(range.0..=range.1)
                    .prefix("X2: "),
            ).changed() {
                self.dragging = true;
            }
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.add_x_projection, "Add X Projection").on_hover_text("Keybinds:\nX = Add X Projection\nLeft click and drag the line at the center of the plot (cirlce)");

            let range = if let Some(histogram) = &self.x_projection {
                histogram.range
            } else {
                (0.0, 1.0)
            };
            if ui.add_enabled(
                self.add_x_projection,
                egui::DragValue::new(&mut self.x_projection_line_1.y_value)
                    .speed(1.0)
                    .range(range.0..=range.1)
                    .prefix("Y1: "),
            ).changed() {
                self.dragging = true;
            }
            if ui.add_enabled(
                self.add_x_projection,
                egui::DragValue::new(&mut self.x_projection_line_2.y_value)
                    .speed(1.0)
                    .range(range.0..=range.1)
                    .prefix("Y2: "),
            ).changed() {
                self.dragging = true;
            }
        });
    }
}
