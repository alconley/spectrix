use egui_plot::{FilledArea, PlotUi};

use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::fitter::common::Calibration;

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct EguiFilledArea {
    pub xs: Vec<f64>,
    pub lower: Vec<f64>,
    pub upper: Vec<f64>,
}

impl EguiFilledArea {
    pub fn new(xs: Vec<f64>, lower: Vec<f64>, upper: Vec<f64>) -> Self {
        Self { xs, lower, upper }
    }

    pub fn clear(&mut self) {
        self.xs.clear();
        self.lower.clear();
        self.upper.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.xs.is_empty() || self.lower.is_empty() || self.upper.is_empty()
    }

    pub fn draw(
        &self,
        plot_ui: &mut PlotUi<'_>,
        calibration: Option<&Calibration>,
        name: &str,
        line: &EguiLine,
        fill_alpha: f32,
    ) {
        if self.is_empty() || self.xs.len() != self.lower.len() || self.xs.len() != self.upper.len()
        {
            return;
        }

        let mut xs = Vec::with_capacity(self.xs.len());
        let mut lower = Vec::with_capacity(self.lower.len());
        let mut upper = Vec::with_capacity(self.upper.len());

        for ((&x, &y_min), &y_max) in self.xs.iter().zip(&self.lower).zip(&self.upper) {
            let calibrated_x = if let Some(calibration) = calibration {
                calibration.calibrate(x)
            } else {
                x
            };

            let transformed_x = if line.log_x && calibrated_x > 0.0 {
                calibrated_x.log10().max(0.0001)
            } else {
                calibrated_x
            };

            let transform_y = |y: f64| {
                if line.log_y {
                    y.max(f64::MIN_POSITIVE).log10().max(0.0001)
                } else {
                    y
                }
            };

            xs.push(transformed_x);
            lower.push(transform_y(y_min));
            upper.push(transform_y(y_max));
        }

        plot_ui.add(
            FilledArea::new("", &xs, &lower, &upper)
                .allow_hover(false)
                .fill_color(line.color.linear_multiply(fill_alpha))
                .id(egui::Id::new(name.to_owned())),
        );
    }
}
