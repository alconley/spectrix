use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::egui_plot_stuff::egui_vertical_line::EguiVerticalLine;
use crate::fitter::common::Calibration;
use crate::histoer::cuts::Cut1D;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InteractiveCut1D {
    pub cut: Cut1D,
    pub column_name: String,
    pub axis_range: (f64, f64),
    pub line_1: EguiVerticalLine,
    pub line_2: EguiVerticalLine,
    pub fill_line: EguiLine,
    #[serde(skip)]
    pub area_dragging: bool,
    #[serde(skip)]
    pub drag_anchor: Option<f64>,
}

impl InteractiveCut1D {
    const DEFAULT_AXIS_OFFSET_FRACTION: f64 = 0.05;

    pub fn new(
        name: &str,
        column_name: &str,
        axis_range: (f64, f64),
        visible_range: (f64, f64),
        color: egui::Color32,
    ) -> Self {
        let mut cut = Self {
            cut: Cut1D::new(name, ""),
            column_name: column_name.to_owned(),
            axis_range,
            line_1: EguiVerticalLine {
                name: format!("{name} Left"),
                color,
                stroke: egui::Stroke::new(1.0, color),
                mid_point_radius: 5.0,
                ..EguiVerticalLine::default()
            },
            line_2: EguiVerticalLine {
                name: format!("{name} Right"),
                color,
                stroke: egui::Stroke::new(1.0, color),
                mid_point_radius: 5.0,
                ..EguiVerticalLine::default()
            },
            fill_line: EguiLine {
                name: format!("{name} Fill"),
                draw: true,
                color,
                stroke: egui::Stroke::new(1.0, color),
                reference_fill: true,
                fill_alpha: 0.12,
                ..EguiLine::default()
            },
            area_dragging: false,
            drag_anchor: None,
        };

        cut.initialize_lines(visible_range);
        cut.sync_definition();
        cut
    }

    fn axis_offset(min: f64, max: f64) -> f64 {
        let width = (max - min).abs();
        if width > 0.0 {
            width * Self::DEFAULT_AXIS_OFFSET_FRACTION
        } else {
            0.0
        }
    }

    fn initialize_lines(&mut self, visible_range: (f64, f64)) {
        let (min_x, max_x) = visible_range;
        let offset = Self::axis_offset(min_x, max_x);
        self.line_1.x_value = min_x + offset;
        self.line_2.x_value = max_x - offset;
        self.clamp_line_positions(self.axis_range);
    }

    fn ordered_limits(&self) -> (f64, f64) {
        (
            self.line_1.x_value.min(self.line_2.x_value),
            self.line_1.x_value.max(self.line_2.x_value),
        )
    }

    fn width(&self) -> f64 {
        (self.line_2.x_value - self.line_1.x_value).abs()
    }

    fn center(&self) -> f64 {
        (self.line_1.x_value + self.line_2.x_value) / 2.0
    }

    fn clamp_center(center: f64, width: f64, axis_range: (f64, f64)) -> f64 {
        let half_width = width / 2.0;
        let min_center = axis_range.0 + half_width;
        let max_center = axis_range.1 - half_width;

        if min_center > max_center {
            (axis_range.0 + axis_range.1) / 2.0
        } else {
            center.clamp(min_center, max_center)
        }
    }

    fn set_center_and_width(&mut self, center: f64, width: f64, axis_range: (f64, f64)) {
        let max_width = (axis_range.1 - axis_range.0).abs();
        let clamped_width = width.clamp(0.0, max_width);
        let center = Self::clamp_center(center, clamped_width, axis_range);
        let half_width = clamped_width / 2.0;

        self.line_1.x_value = center - half_width;
        self.line_2.x_value = center + half_width;
    }

    fn clamp_line_positions(&mut self, axis_range: (f64, f64)) {
        self.line_1.x_value = self.line_1.x_value.clamp(axis_range.0, axis_range.1);
        self.line_2.x_value = self.line_2.x_value.clamp(axis_range.0, axis_range.1);
    }

    fn format_value(value: f64) -> String {
        if value.is_nan() {
            return "nan".to_owned();
        }
        if value.is_infinite() {
            return if value.is_sign_positive() {
                "inf".to_owned()
            } else {
                "-inf".to_owned()
            };
        }

        let mut formatted = format!("{value:.15}");
        if formatted.contains('.') {
            formatted = formatted
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_owned();
        }

        if formatted == "-0" {
            "0".to_owned()
        } else {
            formatted
        }
    }

    fn sync_definition(&mut self) {
        self.clamp_line_positions(self.axis_range);
        let (x1, x2) = self.ordered_limits();
        self.cut.expression = format!(
            "({} >= ({})) & ({} <= {})",
            self.column_name,
            Self::format_value(x1),
            self.column_name,
            Self::format_value(x2)
        );
        self.cut.parse_conditions();
    }

    pub fn set_column_name(&mut self, column_name: &str) {
        self.column_name = column_name.to_owned();
        self.sync_definition();
    }

    fn pointer_x_raw(
        plot_response: &egui_plot::PlotResponse<()>,
        calibration: Option<&Calibration>,
        pointer_pos: egui::Pos2,
    ) -> f64 {
        let display_x = plot_response.transform.value_from_position(pointer_pos).x;
        if let Some(calibration) = calibration {
            calibration.invert(display_x).unwrap_or(display_x)
        } else {
            display_x
        }
    }

    fn interactive_drag_area(
        &mut self,
        plot_response: &egui_plot::PlotResponse<()>,
        calibration: Option<&Calibration>,
        current_plot_bounds: Option<(f64, f64)>,
    ) {
        let pointer_state = plot_response.response.ctx.input(|i| i.pointer.clone());

        if let Some(pointer_pos) = pointer_state.hover_pos() {
            let pointer_x = Self::pointer_x_raw(plot_response, calibration, pointer_pos);
            let (min_x, max_x) = self.ordered_limits();

            if pointer_state.button_pressed(egui::PointerButton::Primary)
                && !self.line_1.is_dragging
                && !self.line_2.is_dragging
                && pointer_x >= min_x
                && pointer_x <= max_x
            {
                self.area_dragging = true;
                self.drag_anchor = Some(pointer_x);
            }

            if self.area_dragging {
                let drag_bounds = current_plot_bounds.unwrap_or(self.axis_range);
                let delta = pointer_x - self.drag_anchor.unwrap_or(pointer_x);
                self.set_center_and_width(self.center() + delta, self.width(), drag_bounds);
                self.drag_anchor = Some(pointer_x);

                if pointer_state.button_released(egui::PointerButton::Primary) {
                    self.area_dragging = false;
                    self.drag_anchor = None;
                }
            }
        } else if pointer_state.button_released(egui::PointerButton::Primary) {
            self.area_dragging = false;
            self.drag_anchor = None;
        }
    }

    fn projection_bins(width: f64, bin_width: f64) -> usize {
        if bin_width <= 0.0 {
            0
        } else {
            (width / bin_width).round().max(0.0) as usize
        }
    }

    fn drag_step(axis_range: (f64, f64), bin_width: f64) -> f64 {
        let axis_span = (axis_range.1 - axis_range.0).abs();
        let fallback_step = if axis_span > 0.0 {
            axis_span / 100.0
        } else {
            0.1
        };

        let step = if bin_width > 0.0 {
            bin_width.min(fallback_step)
        } else {
            fallback_step
        };

        step.max(0.0001)
    }

    fn drag_decimals(step: f64) -> usize {
        if step >= 1.0 {
            0
        } else {
            (-step.log10()).ceil().clamp(0.0, 6.0) as usize
        }
    }

    fn width_controls(&self, ui: &mut egui::Ui, bin_width: f64) -> Option<f64> {
        let mut next_width = self.width();
        let mut width_bins = Self::projection_bins(next_width, bin_width);
        let mut changed = false;
        let drag_step = Self::drag_step(self.axis_range, bin_width);
        let drag_decimals = Self::drag_decimals(drag_step);

        ui.horizontal(|ui| {
            ui.label("Span:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut width_bins)
                        .range(0..=usize::MAX)
                        .speed(1.0)
                        .suffix(" bins"),
                )
                .changed();

            changed |= ui
                .add(
                    egui::DragValue::new(&mut next_width)
                        .range(0.0..=((self.axis_range.1 - self.axis_range.0).abs()))
                        .speed(drag_step)
                        .min_decimals(drag_decimals)
                        .max_decimals(drag_decimals)
                        .prefix("range: "),
                )
                .changed();
        });

        if changed {
            let bins_width = width_bins as f64 * bin_width;
            Some(if (next_width - self.width()).abs() > f64::EPSILON {
                next_width
            } else {
                bins_width
            })
        } else {
            None
        }
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>, calibration: Option<&Calibration>) {
        let plot_bounds = plot_ui.plot_bounds();
        let (x1, x2) = self.ordered_limits();
        self.fill_line.points = vec![[x1, plot_bounds.max()[1]], [x2, plot_bounds.max()[1]]];
        self.fill_line.fill = plot_bounds.min()[1] as f32;

        self.fill_line.draw(plot_ui, calibration);
        self.line_1.draw(plot_ui, calibration);
        self.line_2.draw(plot_ui, calibration);
    }

    pub fn interactive_dragging(
        &mut self,
        plot_response: &egui_plot::PlotResponse<()>,
        calibration: Option<&Calibration>,
        current_plot_bounds: Option<(f64, f64)>,
    ) {
        let previous_name = self.cut.name.clone();
        let previous_expression = self.cut.expression.clone();
        self.line_1.interactive_dragging(plot_response, calibration);
        self.line_2.interactive_dragging(plot_response, calibration);
        self.clamp_line_positions(self.axis_range);

        if self.line_1.is_dragging || self.line_2.is_dragging {
            self.area_dragging = false;
            self.drag_anchor = None;
        }

        self.interactive_drag_area(plot_response, calibration, current_plot_bounds);
        self.sync_definition();

        if self.cut.name != previous_name || self.cut.expression != previous_expression {
            self.cut.autosave_to_saved_path();
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.line_1.is_dragging || self.line_2.is_dragging || self.area_dragging
    }

    pub fn menu_ui(&mut self, ui: &mut egui::Ui, bin_width: f64) {
        let drag_step = Self::drag_step(self.axis_range, bin_width);
        let drag_decimals = Self::drag_decimals(drag_step);
        let mut changed = false;

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.cut.active, "");
                changed |= ui
                    .add(
                        egui::TextEdit::singleline(&mut self.cut.name)
                            .hint_text("Cut Name")
                            .clip_text(false),
                    )
                    .changed();

                if ui.button("Save").clicked()
                    && let Err(error) = self.cut.save_cut_to_json()
                {
                    log::error!("Error saving 1D cut: {error:?}");
                }

                self.cut
                    .info_button(ui, Some(format!("Histogram Column: {}", self.column_name)));
            });

            ui.horizontal(|ui| {
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.line_1.x_value)
                            .range(self.axis_range.0..=self.axis_range.1)
                            .speed(drag_step)
                            .min_decimals(drag_decimals)
                            .max_decimals(drag_decimals)
                            .prefix("X1: "),
                    )
                    .changed();

                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.line_2.x_value)
                            .range(self.axis_range.0..=self.axis_range.1)
                            .speed(drag_step)
                            .min_decimals(drag_decimals)
                            .max_decimals(drag_decimals)
                            .prefix("X2: "),
                    )
                    .changed();
            });

            if let Some(width) = self.width_controls(ui, bin_width) {
                self.set_center_and_width(self.center(), width, self.axis_range);
                changed = true;
            }

            ui.label(&self.cut.expression);
        });

        if changed {
            self.sync_definition();
            self.cut.autosave_to_saved_path();
        }
    }
}
