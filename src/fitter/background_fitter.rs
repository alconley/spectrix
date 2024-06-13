use super::fit_handler::{FitModel, FitResult};
use super::linear::LinearFitter;
use crate::egui_plot_stuff::egui_line::EguiLine;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BackgroundFitter {
    pub x_data: Vec<f64>,
    pub y_data: Vec<f64>,
    pub model: FitModel,
    pub result: Option<FitResult>,
    pub fit_line: EguiLine,
}

impl BackgroundFitter {
    pub fn new(x_data: Vec<f64>, y_data: Vec<f64>, model: FitModel) -> Self {
        BackgroundFitter {
            x_data,
            y_data,
            model,
            result: None,
            fit_line: EguiLine::new(egui::Color32::GREEN),
        }
    }

    pub fn fit(&mut self) {
        match self.model {
            FitModel::Gaussian(_) => {
                log::error!("Gaussian background fitting not yet implemented");
            }
            FitModel::Linear => {
                // Check x and y data are the same length
                if self.x_data.len() != self.y_data.len() {
                    log::error!("x_data and y_data must have the same length");
                    return;
                }

                let mut linear_fitter = LinearFitter::new(self.x_data.clone(), self.y_data.clone());
                linear_fitter.perform_linear_fit();

                // Update the fit line
                if linear_fitter.fit_params.is_some() {
                    self.fit_line.points = linear_fitter.fit_points.clone().unwrap();
                }

                self.fit_line.name = "Background".to_string();

                self.result = Some(FitResult::Linear(linear_fitter));
            }
        }
    }

    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi) {
        self.fit_line.draw(plot_ui);
    }

    pub fn get_background(&self, x_data: &[f64]) -> Option<Vec<f64>> {
        if let Some(FitResult::Linear(fitter)) = &self.result {
            Some(fitter.calculate_background(x_data))
        } else {
            None
        }
    }

    pub fn get_slope_intercept(&self) -> Option<(f64, f64)> {
        if let Some(FitResult::Linear(fitter)) = &self.result {
            fitter
                .fit_params
                .as_ref()
                .map(|params| (params.slope, params.intercept))
        } else {
            None
        }
    }
}
