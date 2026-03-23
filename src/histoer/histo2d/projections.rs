use core::f64;

use crate::egui_plot_stuff::egui_horizontal_line::EguiHorizontalLine;
use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::egui_plot_stuff::egui_vertical_line::EguiVerticalLine;
use crate::histoer::histo1d::histogram1d::Histogram;

use super::histogram2d::Histogram2D;

#[derive(Debug, Clone, Copy)]
pub struct ProjectionAxisSettings {
    pub axis_range: (f64, f64),
    pub bin_width: f64,
}

impl Histogram2D {
    pub fn y_projection(&self, x_min: f64, x_max: f64) -> Vec<u64> {
        // Extract the y-projection data
        let mut y_bins = vec![0; self.bins.y];

        let mut entries: Vec<_> = self.bins.counts.iter().collect();
        entries.sort_by_key(|&(&(x, y), _)| (y, x)); // row-major sort

        for ((x_index, y_index), &count) in entries {
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

        let mut entries: Vec<_> = self.bins.counts.iter().collect();
        entries.sort_by_key(|&(&(x, y), _)| (y, x)); // row-major order

        for ((x_index, y_index), &count) in entries {
            let y_center = self.range.y.min + (*y_index as f64 + 0.5) * self.bins.y_width;
            if y_center >= y_min && y_center < y_max && *x_index < x_bins.len() {
                x_bins[*x_index] += count;
            }
        }

        x_bins
    }

    pub fn check_projections(&mut self) {
        if self.plot_settings.projections.add_y_projection {
            if self.plot_settings.projections.y_projection.is_none() {
                self.plot_settings
                    .projections
                    .initialize_y_projection_lines((self.range.x.min, self.range.x.max));
            }

            let x1 = self.plot_settings.projections.y_projection_line_1.x_value;
            let x2 = self.plot_settings.projections.y_projection_line_2.x_value;
            let (min_x, max_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };

            if self.plot_settings.projections.dragging {
                let bins = self.y_projection(min_x, max_x);

                if let Some(y_projection) = self.plot_settings.projections.y_projection.as_mut() {
                    y_projection.plot_settings.rebin_factor = 1;
                    y_projection.rebin();
                    y_projection.bins = bins.clone();
                    y_projection.original_bins = bins;
                    y_projection.plot_settings.egui_settings.reset_axis = true;
                }
            } else if self.plot_settings.projections.y_projection.is_none() {
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

                if let Some(y_projection) = self.plot_settings.projections.y_projection.as_mut() {
                    y_projection.plot_settings.egui_settings.reset_axis = true;
                }
            }

            self.plot_settings.projections.fill_y_line.points = vec![
                (min_x, self.range.y.max).into(),
                (max_x, self.range.y.max).into(),
            ];

            self.plot_settings.projections.fill_y_line.fill = self.range.y.min as f32;
        }

        if self.plot_settings.projections.add_x_projection {
            if self.plot_settings.projections.x_projection.is_none() {
                self.plot_settings
                    .projections
                    .initialize_x_projection_lines((self.range.y.min, self.range.y.max));
            }

            let y1 = self.plot_settings.projections.x_projection_line_1.y_value;
            let y2 = self.plot_settings.projections.x_projection_line_2.y_value;
            let (min_y, max_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };

            if self.plot_settings.projections.dragging {
                let bins = self.x_projection(min_y, max_y);

                if let Some(x_projection) = self.plot_settings.projections.x_projection.as_mut() {
                    x_projection.plot_settings.rebin_factor = 1;
                    x_projection.rebin();
                    x_projection.bins = bins.clone();
                    x_projection.original_bins = bins;
                    x_projection.plot_settings.egui_settings.reset_axis = true;
                }
            } else if self.plot_settings.projections.x_projection.is_none() {
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

                if let Some(x_projection) = self.plot_settings.projections.x_projection.as_mut() {
                    x_projection.plot_settings.egui_settings.reset_axis = true;
                }
            }

            self.plot_settings.projections.fill_x_line.points = vec![
                (self.range.x.min, max_y).into(),
                (self.range.x.max, max_y).into(),
            ];

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
    #[serde(skip)]
    pub current_plot_bounds: Option<((f64, f64), (f64, f64))>,
    #[serde(skip)]
    pub y_area_dragging: bool,
    #[serde(skip)]
    pub x_area_dragging: bool,
    #[serde(skip)]
    pub y_drag_anchor: Option<f64>,
    #[serde(skip)]
    pub x_drag_anchor: Option<f64>,
}
impl Projections {
    const DEFAULT_AXIS_OFFSET_FRACTION: f64 = 0.05;

    pub fn new() -> Self {
        Self {
            add_y_projection: false,
            y_projection: None,
            y_projection_line_1: EguiVerticalLine {
                name: "Y Projection Line 1".to_owned(),
                color: egui::Color32::from_rgb(255, 0, 0),
                mid_point_radius: 5.0,
                x_value: 0.0,
                ..EguiVerticalLine::default()
            },
            y_projection_line_2: EguiVerticalLine {
                name: "Y Projection Line 2".to_owned(),
                color: egui::Color32::from_rgb(255, 0, 0),
                mid_point_radius: 5.0,
                x_value: 4096.0,
                ..EguiVerticalLine::default()
            },
            fill_y_line: EguiLine {
                name: "Y Projection Fill".to_owned(),
                draw: true,
                color: egui::Color32::from_rgb(255, 0, 0),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 0, 0)),
                reference_fill: true,
                ..EguiLine::default()
            },

            add_x_projection: false,
            x_projection: None,
            x_projection_line_1: EguiHorizontalLine {
                name: "X Projection Line 1".to_owned(),
                color: egui::Color32::from_rgb(0, 0, 255),
                mid_point_radius: 5.0,
                y_value: 0.0,
                ..EguiHorizontalLine::default()
            },
            x_projection_line_2: EguiHorizontalLine {
                name: "X Projection Line 2".to_owned(),
                color: egui::Color32::from_rgb(0, 0, 255),
                mid_point_radius: 5.0,
                y_value: 4096.0,
                ..EguiHorizontalLine::default()
            },
            fill_x_line: EguiLine {
                name: "X Projection Fill".to_owned(),
                draw: true,
                color: egui::Color32::from_rgb(0, 0, 255),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 0, 255)),
                reference_fill: true,
                ..EguiLine::default()
            },
            dragging: false,
            current_plot_bounds: None,
            y_area_dragging: false,
            x_area_dragging: false,
            y_drag_anchor: None,
            x_drag_anchor: None,
        }
    }

    fn axis_offset(min: f64, max: f64) -> f64 {
        let width = (max - min).abs();
        if width > 0.0 {
            width * Self::DEFAULT_AXIS_OFFSET_FRACTION
        } else {
            0.0
        }
    }

    pub fn initialize_y_projection_lines(&mut self, fallback_x_range: (f64, f64)) {
        let (x_min, x_max) = self
            .current_plot_bounds
            .map(|(x_range, _)| x_range)
            .unwrap_or(fallback_x_range);
        let offset = Self::axis_offset(x_min, x_max);

        self.y_projection_line_1.x_value = x_min + offset;
        self.y_projection_line_2.x_value = x_max - offset;
    }

    pub fn initialize_x_projection_lines(&mut self, fallback_y_range: (f64, f64)) {
        let (y_min, y_max) = self
            .current_plot_bounds
            .map(|(_, y_range)| y_range)
            .unwrap_or(fallback_y_range);
        let offset = Self::axis_offset(y_min, y_max);

        self.x_projection_line_1.y_value = y_min + offset;
        self.x_projection_line_2.y_value = y_max - offset;
    }

    fn clamp_projection_center(center: f64, width: f64, axis_range: (f64, f64)) -> f64 {
        let half_width = width / 2.0;
        let min_center = axis_range.0 + half_width;
        let max_center = axis_range.1 - half_width;

        if min_center > max_center {
            (axis_range.0 + axis_range.1) / 2.0
        } else {
            center.clamp(min_center, max_center)
        }
    }

    fn y_projection_width(&self) -> f64 {
        (self.y_projection_line_2.x_value - self.y_projection_line_1.x_value).abs()
    }

    fn x_projection_width(&self) -> f64 {
        (self.x_projection_line_2.y_value - self.x_projection_line_1.y_value).abs()
    }

    fn y_projection_center(&self) -> f64 {
        (self.y_projection_line_1.x_value + self.y_projection_line_2.x_value) / 2.0
    }

    fn x_projection_center(&self) -> f64 {
        (self.x_projection_line_1.y_value + self.x_projection_line_2.y_value) / 2.0
    }

    fn set_y_projection_center_and_width(
        &mut self,
        center: f64,
        width: f64,
        axis_range: (f64, f64),
    ) {
        let max_width = (axis_range.1 - axis_range.0).abs();
        let clamped_width = width.clamp(0.0, max_width);
        let center = Self::clamp_projection_center(center, clamped_width, axis_range);
        let half_width = clamped_width / 2.0;

        self.y_projection_line_1.x_value = center - half_width;
        self.y_projection_line_2.x_value = center + half_width;
    }

    fn set_x_projection_center_and_width(
        &mut self,
        center: f64,
        width: f64,
        axis_range: (f64, f64),
    ) {
        let max_width = (axis_range.1 - axis_range.0).abs();
        let clamped_width = width.clamp(0.0, max_width);
        let center = Self::clamp_projection_center(center, clamped_width, axis_range);
        let half_width = clamped_width / 2.0;

        self.x_projection_line_1.y_value = center - half_width;
        self.x_projection_line_2.y_value = center + half_width;
    }

    fn projection_bins(width: f64, bin_width: f64) -> usize {
        if bin_width <= 0.0 {
            0
        } else {
            (width / bin_width).round().max(0.0) as usize
        }
    }

    fn show_y_projection(&mut self, ui: &egui::Ui) {
        if self.add_y_projection && self.y_projection.is_some() {
            let name = if let Some(histogram) = &self.y_projection {
                let name = histogram.name.clone();
                name.split(':').collect::<Vec<&str>>()[0].to_owned()
            } else {
                "Y-Projection".to_owned()
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

    fn show_x_projection(&mut self, ui: &egui::Ui) {
        if self.add_x_projection && self.x_projection.is_some() {
            let name = if let Some(histogram) = &self.x_projection {
                let name = histogram.name.clone();
                name.split(':').collect::<Vec<&str>>()[0].to_owned()
            } else {
                "X-Projection".to_owned()
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
            && (self.y_projection_line_1.is_dragging
                || self.y_projection_line_2.is_dragging
                || self.y_area_dragging))
            || (self.add_x_projection
                && (self.x_projection_line_1.is_dragging
                    || self.x_projection_line_2.is_dragging
                    || self.x_area_dragging));
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.show_y_projection(ui);
        self.show_x_projection(ui);
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        if self.add_y_projection {
            self.y_projection_line_1.draw(plot_ui, None);
            self.y_projection_line_2.draw(plot_ui, None);
            self.fill_y_line.draw(plot_ui, None);
        }

        if self.add_x_projection {
            self.x_projection_line_1.draw(plot_ui);
            self.x_projection_line_2.draw(plot_ui);
            self.fill_x_line.draw(plot_ui, None);
        }

        self.is_dragging();
    }

    pub fn interactive_dragging(&mut self, plot_response: &egui_plot::PlotResponse<()>) {
        if self.add_y_projection {
            self.y_projection_line_1
                .interactive_dragging(plot_response, None);
            self.y_projection_line_2
                .interactive_dragging(plot_response, None);
            if self.y_projection_line_1.is_dragging || self.y_projection_line_2.is_dragging {
                self.y_area_dragging = false;
                self.y_drag_anchor = None;
            }
            self.interactive_drag_y_projection_area(plot_response);
        }

        if self.add_x_projection {
            self.x_projection_line_1.interactive_dragging(plot_response);
            self.x_projection_line_2.interactive_dragging(plot_response);
            if self.x_projection_line_1.is_dragging || self.x_projection_line_2.is_dragging {
                self.x_area_dragging = false;
                self.x_drag_anchor = None;
            }
            self.interactive_drag_x_projection_area(plot_response);
        }
    }

    fn interactive_drag_y_projection_area(&mut self, plot_response: &egui_plot::PlotResponse<()>) {
        let pointer_state = plot_response.response.ctx.input(|i| i.pointer.clone());
        if let Some(pointer_pos) = pointer_state.hover_pos() {
            let pointer = plot_response.transform.value_from_position(pointer_pos);
            let min_x = self
                .y_projection_line_1
                .x_value
                .min(self.y_projection_line_2.x_value);
            let max_x = self
                .y_projection_line_1
                .x_value
                .max(self.y_projection_line_2.x_value);

            if pointer_state.button_pressed(egui::PointerButton::Primary)
                && !self.y_projection_line_1.is_dragging
                && !self.y_projection_line_2.is_dragging
                && pointer.x >= min_x
                && pointer.x <= max_x
            {
                self.y_area_dragging = true;
                self.y_drag_anchor = Some(pointer.x);
            }

            if self.y_area_dragging {
                if let Some(((x_min, x_max), _)) = self.current_plot_bounds {
                    let center = self.y_projection_center();
                    let delta = pointer.x - self.y_drag_anchor.unwrap_or(pointer.x);
                    self.set_y_projection_center_and_width(
                        center + delta,
                        self.y_projection_width(),
                        (x_min, x_max),
                    );
                    self.y_drag_anchor = Some(pointer.x);
                }

                if pointer_state.button_released(egui::PointerButton::Primary) {
                    self.y_area_dragging = false;
                    self.y_drag_anchor = None;
                }
            }
        } else if pointer_state.button_released(egui::PointerButton::Primary) {
            self.y_area_dragging = false;
            self.y_drag_anchor = None;
        }
    }

    fn interactive_drag_x_projection_area(&mut self, plot_response: &egui_plot::PlotResponse<()>) {
        let pointer_state = plot_response.response.ctx.input(|i| i.pointer.clone());
        if let Some(pointer_pos) = pointer_state.hover_pos() {
            let pointer = plot_response.transform.value_from_position(pointer_pos);
            let min_y = self
                .x_projection_line_1
                .y_value
                .min(self.x_projection_line_2.y_value);
            let max_y = self
                .x_projection_line_1
                .y_value
                .max(self.x_projection_line_2.y_value);

            if pointer_state.button_pressed(egui::PointerButton::Primary)
                && !self.x_projection_line_1.is_dragging
                && !self.x_projection_line_2.is_dragging
                && pointer.y >= min_y
                && pointer.y <= max_y
            {
                self.x_area_dragging = true;
                self.x_drag_anchor = Some(pointer.y);
            }

            if self.x_area_dragging {
                if let Some((_, (y_min, y_max))) = self.current_plot_bounds {
                    let center = self.x_projection_center();
                    let delta = pointer.y - self.x_drag_anchor.unwrap_or(pointer.y);
                    self.set_x_projection_center_and_width(
                        center + delta,
                        self.x_projection_width(),
                        (y_min, y_max),
                    );
                    self.x_drag_anchor = Some(pointer.y);
                }

                if pointer_state.button_released(egui::PointerButton::Primary) {
                    self.x_area_dragging = false;
                    self.x_drag_anchor = None;
                }
            }
        } else if pointer_state.button_released(egui::PointerButton::Primary) {
            self.x_area_dragging = false;
            self.x_drag_anchor = None;
        }
    }

    fn projection_width_controls(
        ui: &mut egui::Ui,
        enabled: bool,
        label: &str,
        center: f64,
        width: f64,
        axis_settings: ProjectionAxisSettings,
    ) -> Option<(f64, f64)> {
        let mut next_width = width;
        let mut width_bins = Self::projection_bins(width, axis_settings.bin_width);
        let mut changed = false;

        ui.vertical(|ui| {
            ui.label(label);
            ui.horizontal(|ui| {
                ui.label("Width:");
                changed |= ui
                    .add_enabled(
                        enabled,
                        egui::DragValue::new(&mut width_bins)
                            .range(0..=usize::MAX)
                            .speed(1.0)
                            .suffix(" bins"),
                    )
                    .changed();

                changed |= ui
                    .add_enabled(
                        enabled,
                        egui::DragValue::new(&mut next_width)
                            .range(
                                0.0..=((axis_settings.axis_range.1 - axis_settings.axis_range.0)
                                    .abs()),
                            )
                            .speed(axis_settings.bin_width.max(0.1))
                            .prefix("range: "),
                    )
                    .changed();
            });
        });

        if changed {
            let bins_width = width_bins as f64 * axis_settings.bin_width;
            let width_from_range = next_width;
            let width = if (width_from_range - width).abs() > f64::EPSILON {
                width_from_range
            } else {
                bins_width
            };
            Some((center, width))
        } else {
            None
        }
    }

    pub fn menu_button(
        &mut self,
        ui: &mut egui::Ui,
        x_axis_settings: ProjectionAxisSettings,
        y_axis_settings: ProjectionAxisSettings,
    ) {
        ui.heading("Projections");

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.add_y_projection, "Add Y Projection").on_hover_text("Keybinds:\nY = Add Y Projection\nLeft click and drag the line at the center of the plot (cirlce)");

            if ui.add_enabled(
                self.add_y_projection,
                egui::DragValue::new(&mut self.y_projection_line_1.x_value)
                    .range(x_axis_settings.axis_range.0..=x_axis_settings.axis_range.1)
                    .speed(1.0)
                    .prefix("X1: "),
            ).changed() {
                self.dragging = true;
            }

            if ui.add_enabled(
                self.add_y_projection,
                egui::DragValue::new(&mut self.y_projection_line_2.x_value)
                    .speed(1.0)
                    .range(x_axis_settings.axis_range.0..=x_axis_settings.axis_range.1)
                    .prefix("X2: "),
            ).changed() {
                self.dragging = true;
            }
        });

        if let Some((center, width)) = Self::projection_width_controls(
            ui,
            self.add_y_projection,
            "Y Projection Span",
            self.y_projection_center(),
            self.y_projection_width(),
            x_axis_settings,
        ) {
            self.set_y_projection_center_and_width(center, width, x_axis_settings.axis_range);
            self.dragging = true;
        }

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.add_x_projection, "Add X Projection").on_hover_text("Keybinds:\nX = Add X Projection\nLeft click and drag the line at the center of the plot (cirlce)");

            if ui.add_enabled(
                self.add_x_projection,
                egui::DragValue::new(&mut self.x_projection_line_1.y_value)
                    .speed(1.0)
                    .range(y_axis_settings.axis_range.0..=y_axis_settings.axis_range.1)
                    .prefix("Y1: "),
            ).changed() {
                self.dragging = true;
            }
            if ui.add_enabled(
                self.add_x_projection,
                egui::DragValue::new(&mut self.x_projection_line_2.y_value)
                    .speed(1.0)
                    .range(y_axis_settings.axis_range.0..=y_axis_settings.axis_range.1)
                    .prefix("Y2: "),
            ).changed() {
                self.dragging = true;
            }
        });

        if let Some((center, width)) = Self::projection_width_controls(
            ui,
            self.add_x_projection,
            "X Projection Span",
            self.x_projection_center(),
            self.x_projection_width(),
            y_axis_settings,
        ) {
            self.set_x_projection_center_and_width(center, width, y_axis_settings.axis_range);
            self.dragging = true;
        }
    }
}
