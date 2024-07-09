use crate::egui_plot_stuff::egui_line::EguiLine;

use super::main_fitter::{FitModel, FitResult};
use super::models::double_exponential::DoubleExponentialFitter;
use super::models::exponential::ExponentialFitter;
use super::models::polynomial::PolynomialFitter;

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
                log::error!("Gaussian background fitting not implemented");
            }

            FitModel::Polynomial(degree) => {
                log::info!("Fitting polynomial of degree {}", degree);
                let mut polynomial_fitter = PolynomialFitter::new(degree);
                polynomial_fitter.x_data.clone_from(&self.x_data);
                polynomial_fitter.y_data.clone_from(&self.y_data);
                polynomial_fitter.fit();

                // Update the fit line
                if polynomial_fitter.coefficients.is_some() {
                    self.fit_line
                        .points
                        .clone_from(&polynomial_fitter.fit_line.points);
                }

                self.fit_line.name = "Background".to_string();

                self.result = Some(FitResult::Polynomial(polynomial_fitter));
            }

            FitModel::Exponential(initial_b_guess) => {
                log::info!(
                    "Fitting exponential with initial b guess {}",
                    initial_b_guess
                );
                let mut exponential_fitter = ExponentialFitter::new(initial_b_guess);
                exponential_fitter.x_data.clone_from(&self.x_data);
                exponential_fitter.y_data.clone_from(&self.y_data);
                exponential_fitter.fit();

                // Update the fit line
                if exponential_fitter.coefficients.is_some() {
                    self.fit_line
                        .points
                        .clone_from(&exponential_fitter.fit_line.points);
                }

                self.fit_line.name = "Background".to_string();

                self.result = Some(FitResult::Exponential(exponential_fitter));
            }

            FitModel::DoubleExponential(initial_b_guess, initial_d_guess) => {
                log::info!(
                    "Fitting double exponential with initial b guess {} and initial d guess {}",
                    initial_b_guess,
                    initial_d_guess
                );
                let mut double_exponential_fitter =
                    DoubleExponentialFitter::new(initial_b_guess, initial_d_guess);
                double_exponential_fitter.x_data.clone_from(&self.x_data);
                double_exponential_fitter.y_data.clone_from(&self.y_data);
                double_exponential_fitter.fit();

                // Update the fit line
                if double_exponential_fitter.coefficients.is_some() {
                    self.fit_line
                        .points
                        .clone_from(&double_exponential_fitter.fit_line.points);

                    self.fit_line.name = "Background".to_string();

                    self.result = Some(FitResult::DoubleExponential(double_exponential_fitter));
                }
            }
        }
    }

    pub fn fitter_stats(&self, ui: &mut egui::Ui) {
        if let Some(fit) = &self.result {
            match fit {
                FitResult::Gaussian(fit) => fit.fit_params_ui(ui),
                FitResult::Polynomial(fit) => fit.fit_params_ui(ui),
                FitResult::Exponential(fit) => fit.fit_params_ui(ui),
                FitResult::DoubleExponential(fit) => fit.fit_params_ui(ui),
            }
        }
    }

    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi) {
        self.fit_line.draw(plot_ui);
    }
}
