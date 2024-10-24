use crate::egui_plot_stuff::egui_line::EguiLine;

use nalgebra::DVector;
use varpro::model::builder::SeparableModelBuilder;
use varpro::solvers::levmar::{LevMarProblemBuilder, LevMarSolver};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Coefficient {
    pub value: f64,
    pub uncertainty: f64,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Coefficients {
    pub a: Coefficient,
    pub b: Coefficient,
    pub c: Coefficient,
    pub d: Coefficient,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DoubleExponentialFitter {
    pub x_data: Vec<f64>,
    pub y_data: Vec<f64>,
    pub weights: Vec<f64>,
    pub initial_b_guess: f64,
    pub initial_d_guess: f64,
    pub coefficients: Option<Coefficients>,
    pub fit_line: EguiLine,
}

impl DoubleExponentialFitter {
    /// Creates a new ExponentialFitter with the given data.
    pub fn new(initial_b_guess: f64, initial_d_guess: f64) -> Self {
        let mut fit_line = EguiLine::new(egui::Color32::GREEN);
        fit_line.name = "Double Exponential Fit".to_string();
        fit_line.width = 1.0;

        DoubleExponentialFitter {
            x_data: Vec::new(),
            y_data: Vec::new(),
            weights: Vec::new(),
            initial_b_guess,
            initial_d_guess,
            coefficients: None,
            fit_line,
        }
    }

    fn exponential(x: &DVector<f64>, b: f64) -> DVector<f64> {
        x.map(|x_val| (-x_val / b).exp())
    }

    fn exponential_pd_b(x: &DVector<f64>, b: f64) -> DVector<f64> {
        x.map(|x_val| (x_val / b.powi(2)) * (-x_val / b).exp())
    }

    fn exponential_pd_d(x: &DVector<f64>, d: f64) -> DVector<f64> {
        x.map(|x_val| (x_val / d.powi(2)) * (-x_val / d).exp())
    }

    pub fn fit(&mut self) {
        let x_data = DVector::from_vec(self.x_data.clone());
        let y_data = DVector::from_vec(self.y_data.clone());
        // let weights = DVector::from_vec(self.weights.clone());

        if x_data.len() < 4 {
            log::error!("Not enough data points to fit exponential");
            return;
        }

        let parameter_names: Vec<String> = vec!["b".to_string(), "d".to_string()];

        let intitial_parameters = vec![self.initial_b_guess, self.initial_d_guess];

        let builder_proxy = SeparableModelBuilder::<f64>::new(parameter_names)
            .initial_parameters(intitial_parameters)
            .independent_variable(x_data)
            .function(&["b"], Self::exponential)
            .partial_deriv("b", Self::exponential_pd_b)
            .function(&["d"], Self::exponential)
            .partial_deriv("d", Self::exponential_pd_d);

        let model = match builder_proxy.build() {
            Ok(model) => model,
            Err(err) => {
                log::error!("Error building model: {}", err);
                return;
            }
        };

        let problem = match LevMarProblemBuilder::new(model)
            .observations(y_data)
            // .weights(weights)
            .build()
        {
            Ok(problem) => problem,
            Err(err) => {
                log::error!("Error building problem: {}", err);
                return;
            }
        };

        if let Ok((fit_result, fit_statistics)) =
            LevMarSolver::default().fit_with_statistics(problem)
        {
            log::info!("fit_result: {:?}", fit_result);
            log::info!("fit_statistics: {:?}", fit_statistics);
            log::info!(
                "Weighted residuals: {:?}",
                fit_statistics.weighted_residuals()
            );
            log::info!(
                "Regression standard error: {:?}",
                fit_statistics.regression_standard_error()
            );
            log::info!(
                "Covariance matrix: {:?}\n",
                fit_statistics.covariance_matrix()
            );

            let nonlinear_parameters = fit_result.nonlinear_parameters();
            log::info!("nonlinear_parameters: {:?}", nonlinear_parameters);

            let nonlinear_variances = fit_statistics.nonlinear_parameters_variance();

            let linear_coefficients = fit_result.linear_coefficients();

            let linear_coefficients = match linear_coefficients {
                Some(coefficients) => coefficients,
                None => {
                    log::error!("No linear coefficients found");
                    return;
                }
            };

            log::info!("linear_coefficients: {:?}", linear_coefficients);

            let linear_variances = fit_statistics.linear_coefficients_variance();

            let parameter_a = linear_coefficients[0];
            let parameter_a_variance = linear_variances[0];
            let parameter_a_uncertainity = parameter_a_variance.sqrt();

            let parameter_b = nonlinear_parameters[0];
            let parameter_b_variance = nonlinear_variances[0];
            let parameter_b_uncertainity = parameter_b_variance.sqrt();

            let parameter_c = linear_coefficients[1];
            let parameter_c_variance = linear_variances[1];
            let parameter_c_uncertainity = parameter_c_variance.sqrt();

            let parameter_d = nonlinear_parameters[1];
            let parameter_d_variance = nonlinear_variances[1];
            let parameter_d_uncertainity = parameter_d_variance.sqrt();

            self.coefficients = Some(Coefficients {
                a: Coefficient {
                    value: parameter_a,
                    uncertainty: parameter_a_uncertainity,
                },
                b: Coefficient {
                    value: parameter_b,
                    uncertainty: parameter_b_uncertainity,
                },
                c: Coefficient {
                    value: parameter_c,
                    uncertainty: parameter_c_uncertainity,
                },
                d: Coefficient {
                    value: parameter_d,
                    uncertainty: parameter_d_uncertainity,
                },
            });

            self.compute_fit_points();
        }
    }

    fn compute_fit_points(&mut self) {
        if let Some(coefficients) = &self.coefficients {
            let a = coefficients.a.value;
            let b = coefficients.b.value;
            let c = coefficients.c.value;
            let d = coefficients.d.value;

            let x_min = self.x_data.iter().cloned().fold(f64::INFINITY, f64::min);
            let x_max = self
                .x_data
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);

            let number_points = 1000;
            for i in 0..number_points {
                let x = x_min + (x_max - x_min) / (number_points as f64) * (i as f64);
                let y = a * (-x / b).exp() + c * (-x / d).exp();
                self.fit_line.add_point(x, y);
            }
        }
    }

    pub fn subtract_background(&self, x_data: Vec<f64>, y_data: Vec<f64>) -> Vec<f64> {
        if let Some(coefficients) = &self.coefficients {
            let a = coefficients.a.value;
            let b = coefficients.b.value;
            let c = coefficients.c.value;
            let d = coefficients.d.value;

            let mut y_data = y_data.clone();

            for (i, x) in x_data.iter().enumerate() {
                let y = a * (-x / b).exp() + c * (-x / d).exp();
                y_data[i] -= y;
            }

            y_data
        } else {
            y_data
        }
    }

    fn _draw(&self, plot_ui: &mut egui_plot::PlotUi) {
        self.fit_line.draw(plot_ui);
    }

    pub fn fit_params_ui(&self, ui: &mut egui::Ui) {
        ui.label("Coefficients:");
        if let Some(coef) = &self.coefficients {
            ui.label(format!(
                "a: {:.3} ± {:.3}",
                coef.a.value, coef.a.uncertainty
            ));
            ui.label(format!(
                "b: {:.3} ± {:.3}",
                coef.b.value, coef.b.uncertainty
            ));
            ui.label(format!(
                "c: {:.3} ± {:.3}",
                coef.c.value, coef.c.uncertainty
            ));
            ui.label(format!(
                "d: {:.3} ± {:.3}",
                coef.d.value, coef.d.uncertainty
            ));
        } else {
            ui.label("No coefficients found");
        }
    }
}
