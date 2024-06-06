#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EguiPlotSettings {
    pub legend: bool,
    pub log_x: bool,
    pub log_y: bool,
    pub show_x_value: bool,
    pub show_y_value: bool,
    pub center_x_axis: bool,
    pub center_y_axis: bool,
    pub allow_zoom: bool,
    pub allow_boxed_zoom: bool,
    pub allow_drag: bool,
    pub allow_scroll: bool,
    pub clamp_grid: bool,
    pub show_grid: bool,
    pub sharp_grid_lines: bool,
    pub show_background: bool,
}

impl Default for EguiPlotSettings {
    fn default() -> Self {
        EguiPlotSettings {
            legend: true,
            log_x: false,
            log_y: false,
            show_x_value: true,
            show_y_value: true,
            center_x_axis: false,
            center_y_axis: false,
            allow_zoom: true,
            allow_boxed_zoom: true,
            allow_drag: true,
            allow_scroll: true,
            clamp_grid: true,
            show_grid: true,
            sharp_grid_lines: true,
            show_background: true,
        }
    }
}

impl EguiPlotSettings {
    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("egui Plot Settings", |ui| {
            ui.vertical(|ui| {
                ui.checkbox(&mut self.legend, "Legend");
                ui.checkbox(&mut self.log_x, "Log X");
                ui.checkbox(&mut self.log_y, "Log Y");
                ui.checkbox(&mut self.show_x_value, "Show X Value");
                ui.checkbox(&mut self.show_y_value, "Show Y Value");
                ui.checkbox(&mut self.center_x_axis, "Center X Axis");
                ui.checkbox(&mut self.center_y_axis, "Center Y Axis");
                ui.checkbox(&mut self.allow_zoom, "Allow Zoom");
                ui.checkbox(&mut self.allow_boxed_zoom, "Allow Boxed Zoom");
                ui.checkbox(&mut self.allow_drag, "Allow Drag");
                ui.checkbox(&mut self.allow_scroll, "Allow Scroll");
                ui.checkbox(&mut self.clamp_grid, "Clamp Grid");
                ui.checkbox(&mut self.show_grid, "Show Grid");
                ui.checkbox(&mut self.sharp_grid_lines, "Sharp Grid Lines");
                ui.checkbox(&mut self.show_background, "Show Background");

                ui.separator();

                if ui.button("Reset").clicked() {
                    *self = EguiPlotSettings::default();
                }
            });
        });
    }

    // some function i can call that adds the settings to the plot
    pub fn apply_to_plot(&self, plot: egui_plot::Plot) -> egui_plot::Plot {
        let log_x = self.log_x;
        let log_y = self.log_y;

        let plot = plot
            .show_x(self.show_x_value)
            .show_y(self.show_y_value)
            .center_x_axis(self.center_x_axis)
            .center_y_axis(self.center_y_axis)
            .allow_zoom(self.allow_zoom)
            .allow_boxed_zoom(self.allow_boxed_zoom)
            .allow_drag(self.allow_drag)
            .allow_scroll(self.allow_scroll)
            .clamp_grid(self.clamp_grid)
            .show_grid(self.show_grid)
            .sharp_grid_lines(self.sharp_grid_lines)
            .show_background(self.show_background)
            .auto_bounds(egui::Vec2b::new(true, true))
            .label_formatter(move |name, value| {
                let x = if log_x {
                    10.0f64.powf(value.x)
                } else {
                    value.x
                };
                let y = if log_y {
                    10.0f64.powf(value.y)
                } else {
                    value.y
                };
                if !name.is_empty() {
                    format!("{name}: {x:.2}, {y:.2}")
                } else {
                    format!("{x:.2}, {y:.2}")
                }
            });

        let plot = if self.legend {
            plot.legend(egui_plot::Legend::default())
        } else {
            plot
        };

        let plot = if log_x {
            plot.x_grid_spacer(log_axis_spacer)
                .x_axis_formatter(log_axis_formatter)
        } else {
            plot
        };

        if log_y {
            plot.y_grid_spacer(log_axis_spacer)
                .y_axis_formatter(log_axis_formatter)
        } else {
            plot
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn log_axis_spacer(input: egui_plot::GridInput) -> Vec<egui_plot::GridMark> {
    let (min, max) = input.bounds;
    let mut marks = vec![];
    for i in min.floor() as i32..=max.ceil() as i32 {
        marks.extend(
            (10..100)
                .map(|j| {
                    let value = i as f64 + (j as f64).log10() - 1.0;
                    let step_size = if j == 10 {
                        1.0
                    } else if j % 10 == 0 {
                        0.1
                    } else {
                        0.01
                    };
                    egui_plot::GridMark { value, step_size }
                })
                .filter(|gm| (min..=max).contains(&gm.value)),
        );
    }
    marks
}

fn log_axis_formatter(
    gm: egui_plot::GridMark,
    max_size: usize,
    _bounds: &std::ops::RangeInclusive<f64>,
) -> String {
    let min_precision = (-gm.value + 1.0).ceil().clamp(1.0, 10.0) as usize;
    let digits = (gm.value).ceil().max(1.0) as usize;
    let size = digits + min_precision + 1;
    let value = 10.0f64.powf(gm.value);
    if size < max_size {
        let precision = max_size.saturating_sub(digits + 1);
        format!("{value:.precision$}")
    } else {
        let exp_digits = (digits as f64).log10() as usize;
        let precision = max_size.saturating_sub(exp_digits).saturating_sub(3);
        format!("{value:.precision$e}")
    }
}
