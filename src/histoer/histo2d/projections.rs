use crate::egui_plot_stuff::egui_horizontal_line::EguiHorizontalLine;
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
        // check to see if the x/y values are the same as the current projection else add a new projection based off the naming scheme
        // then you dont have to recalculate the bins if the projection is already calculated

        if self.plot_settings.projections.add_y_projection {
            let x1 = self.plot_settings.projections.y_projection_line_1.x_value;
            let x2 = self.plot_settings.projections.y_projection_line_2.x_value;
            let (min_x, max_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) }; // sort the x values

            if self.plot_settings.projections.y_projection.is_some() {
                //check the name of the current projection and update the bins if different
                let name = self
                    .plot_settings
                    .projections
                    .y_projection
                    .as_ref()
                    .unwrap()
                    .name
                    .clone();

                if name != format!("Y-Projection of {}: x={:.2}-{:.2}", self.name, min_x, max_x) {
                    let bins = self.y_projection(min_x, max_x);
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
                        .name =
                        format!("Y-Projection of {}: x={:.2}-{:.2}", self.name, min_x, max_x);

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
                    &format!("Y-Projection of {}: x={:.2}-{:.2}", self.name, min_x, max_x),
                    self.bins.y,
                    (self.range.y.min, self.range.y.max),
                );
                let bins = self.y_projection(min_x, max_x);
                y_histogram.bins = bins.clone();
                y_histogram.original_bins = bins;
                y_histogram.plot_settings.egui_settings.reset_axis = true;

                self.plot_settings.projections.y_projection = Some(y_histogram);

                // set the projection range to be the min/max values of the histogram
                self.plot_settings.projections.y_projection_line_1.x_value = self.range.x.min;
                self.plot_settings.projections.y_projection_line_2.x_value = self.range.x.max;
            }
        }

        if self.plot_settings.projections.add_x_projection {
            let y1 = self.plot_settings.projections.x_projection_line_1.y_value;
            let y2 = self.plot_settings.projections.x_projection_line_2.y_value;
            let (min_y, max_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) }; // sort the y values

            if self.plot_settings.projections.x_projection.is_some() {
                //check the name of the current projection and update the bins if different
                let name = self
                    .plot_settings
                    .projections
                    .x_projection
                    .as_ref()
                    .unwrap()
                    .name
                    .clone();

                if name != format!("X-Projection of {}: y={:.2}-{:.2}", self.name, min_y, max_y) {
                    let bins = self.x_projection(min_y, max_y);
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
                        .name =
                        format!("X-Projection of {}: y={:.2}-{:.2}", self.name, min_y, max_y);

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
                    &format!("X-Projection of {}: y={:.2}-{:.2}", self.name, min_y, max_y),
                    self.bins.x,
                    (self.range.x.min, self.range.x.max),
                );
                let bins = self.x_projection(min_y, max_y);
                x_histogram.bins = bins.clone();
                x_histogram.original_bins = bins;
                x_histogram.plot_settings.egui_settings.reset_axis = true;

                self.plot_settings.projections.x_projection = Some(x_histogram);

                // set the projection range to be the min/max values of the histogram
                self.plot_settings.projections.x_projection_line_1.y_value = self.range.y.min;
                self.plot_settings.projections.x_projection_line_2.y_value = self.range.y.max;
            }
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
                mid_point_radius: 5.0,
                ..EguiVerticalLine::default()
            },
            y_projection_line_2: EguiVerticalLine {
                name: "Y Projection Line 2".to_string(),
                mid_point_radius: 5.0,
                ..EguiVerticalLine::default()
            },

            add_x_projection: false,
            x_projection: None,
            x_projection_line_1: EguiHorizontalLine {
                name: "X Projection Line 1".to_string(),
                mid_point_radius: 5.0,
                ..EguiHorizontalLine::default()
            },
            x_projection_line_2: EguiHorizontalLine {
                name: "X Projection Line 2".to_string(),
                mid_point_radius: 5.0,
                ..EguiHorizontalLine::default()
            },
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
        }
    }

    pub fn is_dragging(&self) -> bool {
        if self.add_y_projection
            && (self.y_projection_line_1.is_dragging || self.y_projection_line_2.is_dragging)
        {
            return true;
        }

        if self.add_x_projection
            && (self.x_projection_line_1.is_dragging || self.x_projection_line_2.is_dragging)
        {
            return true;
        }

        false
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

        ui.checkbox(&mut self.add_y_projection, "Add Y Projection").on_hover_text("Keybinds:\nY = Add Y Projection\nLeft click and drag the line at the center of the plot (cirlce)");

        if self.add_y_projection {
            ui.horizontal(|ui| {
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
            });
        }

        ui.checkbox(&mut self.add_x_projection, "Add X Projection").on_hover_text("Keybinds:\nX = Add X Projection\nLeft click and drag the line at the center of the plot (cirlce)");

        if self.add_x_projection {
            ui.horizontal(|ui| {
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
            });
        }
    }
}
