use super::fit_handler::{FitModel, FitResult};
use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::fitter::exponential::ExponentialFitter;
use crate::fitter::polynomial::PolynomialFitter;

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
                polynomial_fitter.x_data = self.x_data.clone();
                polynomial_fitter.y_data = self.y_data.clone();
                polynomial_fitter.fit();

                // Update the fit line
                if polynomial_fitter.coefficients.is_some() {
                    self.fit_line.points = polynomial_fitter.fit_line.points.clone();
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
                exponential_fitter.x_data = self.x_data.clone();
                exponential_fitter.y_data = self.y_data.clone();
                exponential_fitter.fit();

                // Update the fit line
                if exponential_fitter.coefficients.is_some() {
                    self.fit_line.points = exponential_fitter.fit_line.points.clone();
                }

                self.fit_line.name = "Background".to_string();

                self.result = Some(FitResult::Exponential(exponential_fitter));
            }
        }
    }

    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi) {
        self.fit_line.draw(plot_ui);
    }
}
