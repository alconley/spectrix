use nalgebra::DVector;
use varpro::model::builder::SeparableModelBuilder;
use varpro::solvers::levmar::{LevMarProblemBuilder, LevMarSolver};

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Value {
    pub value: f64,
    pub uncertainty: f64,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GaussianParams {
    pub amplitude: Value,
    pub mean: Value,
    pub sigma: Value,
    pub fwhm: Value,
    pub area: Value,
}

impl GaussianParams {
    // Constructor that also calculates FWHM and area
    pub fn new(amplitude: Value, mean: Value, sigma: Value) -> Result<Self, String> {
        if sigma.value <= 0.0 {
            let error_message = "Sigma must be positive.".to_string();
            // log::error!("{}", error_message);
            return Err(error_message);
        }

        let fwhm = Self::calculate_fwhm(sigma.value);
        let fwhm_uncertainty = Self::fwhm_uncertainty(sigma.uncertainty);

        let area = Self::calculate_area(amplitude.value, sigma.value);
        if area < 0.0 {
            let error_message = "Area is negative.".to_string();
            // log::error!("{}", error_message);
            return Err(error_message);
        }
        let area_uncertainty = Self::area_uncertainty(amplitude.clone(), sigma.clone());

        Ok(GaussianParams {
            amplitude,
            mean,
            sigma,
            fwhm: Value {
                value: fwhm,
                uncertainty: fwhm_uncertainty,
            },
            area: Value {
                value: area,
                uncertainty: area_uncertainty,
            },
        })
    }

    // Method to calculate FWHM
    fn calculate_fwhm(sigma: f64) -> f64 {
        2.0 * (2.0 * f64::ln(2.0)).sqrt() * sigma
    }

    // Method to calculate FWHM uncertainty
    fn fwhm_uncertainty(sigma_uncertainty: f64) -> f64 {
        2.0 * (2.0 * f64::ln(2.0)).sqrt() * sigma_uncertainty
    }

    // Method to calculate area
    fn calculate_area(amplitude: f64, sigma: f64) -> f64 {
        amplitude * sigma * (2.0 * std::f64::consts::PI).sqrt()
    }

    // Method to calculate area uncertainty
    fn area_uncertainty(amplitude: Value, sigma: Value) -> f64 {
        let two_pi_sqrt = (2.0 * std::f64::consts::PI).sqrt();
        ((sigma.value * two_pi_sqrt * amplitude.uncertainty).powi(2)
            + (amplitude.value * two_pi_sqrt * sigma.uncertainty).powi(2))
        .sqrt()
    }

    pub fn params_ui(&self, ui: &mut egui::Ui) {
        ui.label(format!(
            "{:.2} ± {:.2}",
            self.mean.value, self.mean.uncertainty
        ));
        ui.label(format!(
            "{:.2} ± {:.2}",
            self.fwhm.value, self.fwhm.uncertainty
        ));
        ui.label(format!(
            "{:.2} ± {:.2}",
            self.area.value, self.area.uncertainty
        ));
    }

    pub fn fit_line_points(&self) -> Vec<[f64; 2]> {
        let num_points = 1000;
        let start = self.mean.value - 5.0 * self.sigma.value; // Adjust start and end to be +/- 5 sigma from the mean
        let end = self.mean.value + 5.0 * self.sigma.value;
        let step = (end - start) / num_points as f64;

        (0..num_points)
            .map(|i| {
                let x = start + step * i as f64;
                let y = self.amplitude.value
                    * (-((x - self.mean.value).powi(2)) / (2.0 * self.sigma.value.powi(2))).exp();
                [x, y]
            })
            .collect()
    }
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GaussianFitter {
    x: Vec<f64>,
    y: Vec<f64>,
    pub peak_markers: Vec<f64>,
    pub fit_params: Option<Vec<GaussianParams>>,
    pub fit_lines: Option<Vec<Vec<[f64; 2]>>>,
}

impl GaussianFitter {
    pub fn new(x: Vec<f64>, y: Vec<f64>, peak_markers: Vec<f64>) -> Self {
        Self {
            x,
            y,
            peak_markers,
            fit_params: None,
            fit_lines: None,
        }
    }

    fn gaussian(x: &DVector<f64>, mean: f64, sigma: f64) -> DVector<f64> {
        x.map(|x_val| (-((x_val - mean).powi(2)) / (2.0 * sigma.powi(2))).exp())
    }

    fn gaussian_pd_mean(x: &DVector<f64>, mean: f64, sigma: f64) -> DVector<f64> {
        x.map(|x_val| {
            (x_val - mean) / sigma.powi(2)
                * (-((x_val - mean).powi(2)) / (2.0 * sigma.powi(2))).exp()
        })
    }

    fn gaussian_pd_std_dev(x: &DVector<f64>, mean: f64, sigma: f64) -> DVector<f64> {
        x.map(|x_val| {
            let exponent = -((x_val - mean).powi(2)) / (2.0 * sigma.powi(2));
            (x_val - mean).powi(2) / sigma.powi(3) * exponent.exp()
        })
    }

    fn initial_guess(&mut self) -> Vec<f64> {
        let mut initial_guesses: Vec<f64> = Vec::new();

        // if peak_marks is empty, find the max of the y data and use that index of the x data as the initial guess
        if self.peak_markers.is_empty() {
            let max_y = self.y.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let max_y_index = self.y.iter().position(|&r| r == max_y).unwrap();
            self.peak_markers.push(self.x[max_y_index]);
        }

        for &mean in &self.peak_markers {
            initial_guesses.push(mean);
        }

        let min_x = self.x.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_x = self.x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max_x - min_x;

        let average_sigma = range / (5.0 * self.peak_markers.len() as f64);

        log::info!("Average sigma: {}", average_sigma);

        initial_guesses.push(average_sigma);

        initial_guesses
    }

    fn generate_parameter_names(&self) -> Vec<String> {
        let mut parameter_names = Vec::new();

        for i in 0..self.peak_markers.len() {
            parameter_names.push(format!("mean{}", i));
        }
        parameter_names.push("sigma".to_string());

        parameter_names
    }

    pub fn multi_gauss_fit(&mut self) {
        self.fit_params = None;
        self.fit_lines = None;

        // Ensure x and y data have the same length
        if self.x.len() != self.y.len() {
            log::error!("x_data and y_data must have the same length");
            return;
        }

        // Convert x and y data to DVector
        let x_data = DVector::from_vec(self.x.clone());
        let y_data = DVector::from_vec(self.y.clone());

        let initial_guess = self.initial_guess();
        let parameter_names = self.generate_parameter_names();

        // Add parameters for the first peak manually
        let mut builder_proxy = SeparableModelBuilder::<f64>::new(parameter_names)
            .initial_parameters(initial_guess)
            .independent_variable(x_data)
            .function(&["mean0", "sigma"], Self::gaussian)
            .partial_deriv("mean0", Self::gaussian_pd_mean)
            .partial_deriv("sigma", Self::gaussian_pd_std_dev);

        // Now, iterate starting from the second peak since the first peak is already handled
        for i in 1..self.peak_markers.len() {
            // For each subsequent peak, add the function and its derivatives
            builder_proxy = builder_proxy
                .function(&[format!("mean{}", i), "sigma".to_owned()], Self::gaussian)
                .partial_deriv(format!("mean{}", i), Self::gaussian_pd_mean)
                .partial_deriv("sigma", Self::gaussian_pd_std_dev);
        }

        // Finalize the model building process
        let model = match builder_proxy.build() {
            Ok(model) => model,
            Err(e) => {
                log::error!("Failed to build model: {:?}", e);
                return;
            }
        };

        // Extract the parameters
        let problem = match LevMarProblemBuilder::new(model)
            .observations(y_data)
            .build()
        {
            Ok(problem) => problem,
            Err(e) => {
                log::error!("Failed to build problem: {:?}", e);
                return;
            }
        };

        match LevMarSolver::default().fit_with_statistics(problem) {
            Ok((fit_result, fit_statistics)) => {
                log::info!(
                    "Nonlinear Parameters: {:?}",
                    fit_result.nonlinear_parameters()
                );
                log::info!(
                    "nonlinear parameters variance: {:?}",
                    fit_statistics.nonlinear_parameters_variance()
                );

                log::info!(
                    "Linear Coefficients: {:?}",
                    fit_result.linear_coefficients().unwrap()
                );
                log::info!(
                    "linear coefficients variance: {:?}",
                    fit_statistics.linear_coefficients_variance()
                );

                let nonlinear_parameters = fit_result.nonlinear_parameters();
                let nonlinear_variances = fit_statistics.nonlinear_parameters_variance();

                let linear_coefficients = fit_result.linear_coefficients().unwrap();
                let linear_variances = fit_statistics.linear_coefficients_variance();

                let mut params: Vec<GaussianParams> = Vec::new();

                let sigma = nonlinear_parameters[nonlinear_parameters.len() - 1];
                let sigma_variance = nonlinear_variances[nonlinear_parameters.len() - 1];

                // Clear peak markers and update with the mean of the gaussians
                self.peak_markers.clear();

                // Assuming the amplitude (c) for each Gaussian comes first in linear_coefficients
                for (i, &amplitude) in linear_coefficients.iter().enumerate() {
                    let mean = nonlinear_parameters[i];
                    // Update peak markers
                    self.peak_markers.push(mean);

                    let mean_variance = nonlinear_variances[i];
                    let amplitude_variance = linear_variances[i];

                    // Create a GaussianParams instance which now includes FWHM and area calculations
                    match GaussianParams::new(
                        Value {
                            value: amplitude,
                            uncertainty: amplitude_variance.sqrt(),
                        },
                        Value {
                            value: mean,
                            uncertainty: mean_variance.sqrt(),
                        },
                        Value {
                            value: sigma,
                            uncertainty: sigma_variance.sqrt(),
                        },
                    ) {
                        Ok(gaussian_params) => {
                            // Log the Gaussian component parameters including FWHM and area
                            log::info!("Peak {}: Amplitude: {:.2} ± {:.2}, Mean: {:.2} ± {:.2}, Std Dev: {:.2} ± {:.2}, FWHM: {:.2} ± {:.2}, Area: {:.2} ± {:.2}",
                                i, amplitude, amplitude_variance.sqrt(), mean, mean_variance.sqrt(), sigma, sigma_variance.sqrt(),
                                gaussian_params.fwhm.value, gaussian_params.fwhm.uncertainty, gaussian_params.area.value, gaussian_params.area.uncertainty);

                            params.push(gaussian_params);
                        }
                        Err(e) => {
                            log::error!("Fit Failed: GaussianParams for peak {}: {}", i, e);
                            return;
                        }
                    }
                }

                self.fit_params = Some(params);
                self.get_fit_lines();
            }
            Err(e) => {
                log::error!("Failed to fit model: {:?}", e);
            }
        }
    }

    pub fn get_fit_lines(&mut self) {
        if let Some(fit_params) = &self.fit_params {
            let mut fit_lines = Vec::new();

            for params in fit_params.iter() {
                let line = params.fit_line_points();
                fit_lines.push(line);
            }

            self.fit_lines = Some(fit_lines);
        } else {
            self.fit_lines = None;
        }
    }

    pub fn composition_fit_points_linear_bg(&self, slope: f64, intercept: f64) -> Vec<[f64; 2]> {
        let num_points = 3000;
        let min_x = self.x.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_x = self.x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let step = (max_x - min_x) / num_points as f64;

        (0..=num_points)
            .map(|i| {
                let x = min_x + step * i as f64;
                let y_gauss = self.fit_params.as_ref().map_or(0.0, |params| {
                    params.iter().fold(0.0, |sum, param| {
                        sum + param.amplitude.value
                            * (-((x - param.mean.value).powi(2))
                                / (2.0 * param.sigma.value.powi(2)))
                            .exp()
                    })
                });
                let y_background = slope * x + intercept;
                let y_total = y_gauss + y_background;
                [x, y_total]
            })
            .collect()
    }

    pub fn fit_params_ui(&self, ui: &mut egui::Ui) {
        if let Some(fit_params) = &self.fit_params {
            for (i, params) in fit_params.iter().enumerate() {
                if i != 0 {
                    ui.label("");
                }

                ui.label(format!("{}", i));
                params.params_ui(ui);
                ui.end_row();
            }
        }
    }
}
