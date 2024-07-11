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
    pub fn new(amplitude: Value, mean: Value, sigma: Value, bin_width: f64) -> Option<Self> {
        if sigma.value < 0.0 {
            log::error!("Sigma value is negative");
            return None;
        }

        let fwhm = Self::calculate_fwhm(sigma.value);
        let fwhm_uncertainty = Self::fwhm_uncertainty(sigma.uncertainty);

        let area = Self::calculate_area(amplitude.value, sigma.value, bin_width);
        if area < 0.0 {
            log::error!("Area is negative");
            return None;
        }
        let area_uncertainty = Self::area_uncertainty(amplitude.clone(), sigma.clone());

        Some(GaussianParams {
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
    fn calculate_area(amplitude: f64, sigma: f64, bin_width: f64) -> f64 {
        amplitude * sigma * (2.0 * std::f64::consts::PI).sqrt() / bin_width
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
    pub free_stddev: bool, // false = fit all the gaussians with the same sigma
    pub free_position: bool, // false = fix the position of the gaussians to the peak_markers
    pub bin_width: f64,
}

impl GaussianFitter {
    pub fn new(
        x: Vec<f64>,
        y: Vec<f64>,
        peak_markers: Vec<f64>,
        free_stddev: bool,
        free_position: bool,
        bin_width: f64,
    ) -> Self {
        Self {
            x,
            y,
            peak_markers,
            fit_params: None,
            fit_lines: None,
            free_stddev,
            free_position,
            bin_width,
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

    fn average_sigma(&self) -> f64 {
        let min_x = self.x.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_x = self.x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max_x - min_x;

        range / (5.0 * self.peak_markers.len() as f64)
    }

    fn initial_guess(&mut self) -> Vec<f64> {
        let mut initial_guesses: Vec<f64> = Vec::new();

        // if peak_marks is empty, find the max of the y data and use that index of the x data as the initial guess
        if self.peak_markers.is_empty() {
            let max_y = self.y.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let max_y_index = match self.y.iter().position(|&r| r == max_y) {
                Some(index) => index,
                None => {
                    log::error!("Max y value not found in y data");
                    return vec![];
                }
            };
            self.peak_markers.push(self.x[max_y_index]);
        }

        for &mean in &self.peak_markers {
            initial_guesses.push(mean);
        }

        let average_sigma = self.average_sigma();

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

    fn multi_gauss_fit_free_stddev_free_position(&mut self) {
        self.fit_params = None;
        self.fit_lines = None;

        // Ensure x and y data have the same length
        if self.x.len() != self.y.len() {
            log::error!("x_data and y_data must have the same length");
            return;
        }

        let mut initial_guesses: Vec<f64> = Vec::new();
        let mut parameter_names: Vec<String> = Vec::new();

        // if peak_marks is empty, find the max of the y data and use that index of the x data as the initial guess
        if self.peak_markers.is_empty() {
            let max_y = self.y.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let max_y_index = match self.y.iter().position(|&r| r == max_y) {
                Some(index) => index,
                None => {
                    log::error!("Max y value not found in y data");
                    return;
                }
            };
            self.peak_markers.push(self.x[max_y_index]);
        }

        let average_sigma = self.average_sigma();

        for (index, &mean) in self.peak_markers.iter().enumerate() {
            initial_guesses.push(mean);
            parameter_names.push(format!("mean{}", index));
            initial_guesses.push(average_sigma);
            parameter_names.push(format!("sigma{}", index));
        }

        // Convert x and y data to DVector
        let x_data = DVector::from_vec(self.x.clone());
        let y_data = DVector::from_vec(self.y.clone());

        // Add parameters for the first peak manually
        let mut builder_proxy = SeparableModelBuilder::<f64>::new(parameter_names)
            .initial_parameters(initial_guesses)
            .independent_variable(x_data)
            .function(&["mean0", "sigma0"], Self::gaussian)
            .partial_deriv("mean0", Self::gaussian_pd_mean)
            .partial_deriv("sigma0", Self::gaussian_pd_std_dev);

        // Now, iterate starting from the second peak since the first peak is already handled
        for i in 1..self.peak_markers.len() {
            // For each subsequent peak, add the function and its derivatives
            builder_proxy = builder_proxy
                .function(
                    &[format!("mean{}", i), format!("sigma{}", i)],
                    Self::gaussian,
                )
                .partial_deriv(format!("mean{}", i), Self::gaussian_pd_mean)
                .partial_deriv(format!("sigma{}", i), Self::gaussian_pd_std_dev);
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
                let nonlinear_parameters = fit_result.nonlinear_parameters();
                let nonlinear_variances = fit_statistics.nonlinear_parameters_variance();
                let linear_coefficients = match fit_result.linear_coefficients() {
                    Some(coefficients) => coefficients,
                    None => {
                        log::error!("Failed to get linear coefficients");
                        return;
                    }
                };
                let linear_variances = fit_statistics.linear_coefficients_variance();
                let mut params: Vec<GaussianParams> = Vec::new();

                for (i, &amplitude) in linear_coefficients.iter().enumerate() {
                    let mean = nonlinear_parameters[i * 2];
                    let mean_variance = nonlinear_variances[i * 2];
                    let sigma = nonlinear_parameters[i * 2 + 1];
                    let sigma_variance = nonlinear_variances[i * 2 + 1];
                    let amplitude_variance = linear_variances[i];

                    if let Some(gaussian_params) = GaussianParams::new(
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
                        self.bin_width,
                    ) {
                        params.push(gaussian_params);
                    } else {
                        // Remove the peak marker with the negative area and retry the fit
                        self.peak_markers.remove(i);
                        self.multi_gauss_fit_free_stddev_free_position();
                        return;
                    }
                }

                // Clear peak markers and update with the mean of the gaussians
                self.peak_markers.clear();
                for mean in &params {
                    self.peak_markers.push(mean.mean.value);
                }

                self.fit_params = Some(params);
                self.get_fit_lines();
            }
            Err(e) => {
                log::error!("Failed to fit model: {:?}", e);
            }
        }
    }

    fn multi_gauss_fit_fixed_stdev_free_position(&mut self) {
        self.fit_params = None;
        self.fit_lines = None;

        if self.x.len() != self.y.len() {
            log::error!("x_data and y_data must have the same length");
            return;
        }

        let x_data = DVector::from_vec(self.x.clone());
        let y_data = DVector::from_vec(self.y.clone());
        let initial_guess = self.initial_guess();
        let parameter_names = self.generate_parameter_names();

        let mut builder_proxy = SeparableModelBuilder::<f64>::new(parameter_names)
            .initial_parameters(initial_guess)
            .independent_variable(x_data)
            .function(&["mean0", "sigma"], Self::gaussian)
            .partial_deriv("mean0", Self::gaussian_pd_mean)
            .partial_deriv("sigma", Self::gaussian_pd_std_dev);

        for i in 1..self.peak_markers.len() {
            builder_proxy = builder_proxy
                .function(&[format!("mean{}", i), "sigma".to_owned()], Self::gaussian)
                .partial_deriv(format!("mean{}", i), Self::gaussian_pd_mean)
                .partial_deriv("sigma", Self::gaussian_pd_std_dev);
        }

        let model = match builder_proxy.build() {
            Ok(model) => model,
            Err(e) => {
                log::error!("Failed to build model: {:?}", e);
                return;
            }
        };

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
                let nonlinear_parameters = fit_result.nonlinear_parameters();
                let nonlinear_variances = fit_statistics.nonlinear_parameters_variance();
                let linear_coefficients = match fit_result.linear_coefficients() {
                    Some(coefficients) => coefficients,
                    None => {
                        log::error!("Failed to get linear coefficients");
                        return;
                    }
                };
                let linear_variances = fit_statistics.linear_coefficients_variance();
                let mut params: Vec<GaussianParams> = Vec::new();

                let sigma = nonlinear_parameters[nonlinear_parameters.len() - 1];
                let sigma_variance = nonlinear_variances[nonlinear_parameters.len() - 1];

                for (i, &amplitude) in linear_coefficients.iter().enumerate() {
                    let mean = nonlinear_parameters[i];
                    let mean_variance = nonlinear_variances[i];
                    let amplitude_variance = linear_variances[i];

                    if let Some(gaussian_params) = GaussianParams::new(
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
                        self.bin_width,
                    ) {
                        params.push(gaussian_params);
                    } else {
                        self.peak_markers.remove(i);
                        self.multi_gauss_fit_fixed_stdev_free_position();
                        return;
                    }
                }

                self.peak_markers.clear();
                for mean in &params {
                    self.peak_markers.push(mean.mean.value);
                }

                self.fit_params = Some(params);
                self.get_fit_lines();
            }
            Err(e) => {
                log::error!("Failed to fit model: {:?}", e);
            }
        }
    }

    fn multi_gauss_fit_fixed_stdev_fixed_position(&mut self) {
        self.fit_params = None;
        self.fit_lines = None;

        if self.x.len() != self.y.len() {
            log::error!("x_data and y_data must have the same length");
            return;
        }

        if self.peak_markers.is_empty() {
            log::error!(
                "Peak markers are empty. Must have at least 1 marker to fit with a fixed position"
            );
            return;
        }

        let x_data = DVector::from_vec(self.x.clone());
        let y_data = DVector::from_vec(self.y.clone());
        let parameter_names = ["sigma".to_string()];
        let initial_guess = vec![self.average_sigma()];
        let peak_markers = self.peak_markers.clone();
        let peak = peak_markers[0];

        let mut builder_proxy = SeparableModelBuilder::<f64>::new(parameter_names)
            .initial_parameters(initial_guess)
            .independent_variable(x_data)
            .function(["sigma".to_owned()], move |x: &DVector<f64>, sigma: f64| {
                x.map(|x_val| (-((x_val - peak).powi(2)) / (2.0 * sigma.powi(2))).exp())
            })
            .partial_deriv("sigma", move |x: &DVector<f64>, sigma: f64| {
                x.map(|x_val| {
                    let exponent = -((x_val - peak).powi(2)) / (2.0 * sigma.powi(2));
                    (x_val - peak).powi(2) / sigma.powi(3) * exponent.exp()
                })
            });

        for i in 1..self.peak_markers.len() {
            let peak = self.peak_markers[i];
            builder_proxy = builder_proxy
                .function(["sigma".to_owned()], move |x: &DVector<f64>, sigma: f64| {
                    x.map(|x_val| (-((x_val - peak).powi(2)) / (2.0 * sigma.powi(2))).exp())
                })
                .partial_deriv("sigma", move |x: &DVector<f64>, sigma: f64| {
                    x.map(|x_val| {
                        let exponent = -((x_val - peak).powi(2)) / (2.0 * sigma.powi(2));
                        (x_val - peak).powi(2) / sigma.powi(3) * exponent.exp()
                    })
                });
        }

        let model = match builder_proxy.build() {
            Ok(model) => model,
            Err(e) => {
                log::error!("Failed to build model: {:?}", e);
                return;
            }
        };

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
                let nonlinear_parameters = fit_result.nonlinear_parameters();
                let nonlinear_variances = fit_statistics.nonlinear_parameters_variance();
                let linear_coefficients = match fit_result.linear_coefficients() {
                    Some(coefficients) => coefficients,
                    None => {
                        log::error!("Failed to get linear coefficients");
                        return;
                    }
                };
                let linear_variances = fit_statistics.linear_coefficients_variance();
                let mut params: Vec<GaussianParams> = Vec::new();

                let sigma = nonlinear_parameters[nonlinear_parameters.len() - 1];
                let sigma_variance = nonlinear_variances[nonlinear_parameters.len() - 1];

                for (i, &amplitude) in linear_coefficients.iter().enumerate() {
                    let mean = self.peak_markers[i];
                    let mean_uncertainty = 0.0;
                    let amplitude_variance = linear_variances[i];

                    if let Some(gaussian_params) = GaussianParams::new(
                        Value {
                            value: amplitude,
                            uncertainty: amplitude_variance.sqrt(),
                        },
                        Value {
                            value: mean,
                            uncertainty: mean_uncertainty,
                        },
                        Value {
                            value: sigma,
                            uncertainty: sigma_variance.sqrt(),
                        },
                        self.bin_width,
                    ) {
                        params.push(gaussian_params);
                    } else {
                        self.peak_markers.remove(i);
                        self.multi_gauss_fit_fixed_stdev_fixed_position();
                        return;
                    }
                }

                self.peak_markers.clear();
                for mean in &params {
                    self.peak_markers.push(mean.mean.value);
                }

                self.fit_params = Some(params);
                self.get_fit_lines();
            }
            Err(e) => {
                log::error!("Failed to fit model: {:?}", e);
            }
        }
    }

    fn multi_gauss_fit_free_stdev_fixed_position(&mut self) {
        self.fit_params = None;
        self.fit_lines = None;

        if self.x.len() != self.y.len() {
            log::error!("x_data and y_data must have the same length");
            return;
        }

        if self.peak_markers.is_empty() {
            log::error!(
                "Peak markers are empty. Must have at least 1 marker to fit with a fixed position"
            );
            return;
        }

        let x_data = DVector::from_vec(self.x.clone());
        let y_data = DVector::from_vec(self.y.clone());
        let mut initial_guess: Vec<f64> = Vec::new();
        let mut parameter_names: Vec<String> = Vec::new();
        let average_sigma = self.average_sigma();

        for (index, &_mean) in self.peak_markers.iter().enumerate() {
            initial_guess.push(average_sigma);
            parameter_names.push(format!("sigma{}", index));
        }

        let peak = self.peak_markers[0];

        let mut builder_proxy = SeparableModelBuilder::<f64>::new(parameter_names)
            .initial_parameters(initial_guess)
            .independent_variable(x_data)
            .function(
                ["sigma0".to_owned()],
                move |x: &DVector<f64>, sigma: f64| {
                    x.map(|x_val| (-((x_val - peak).powi(2)) / (2.0 * sigma.powi(2))).exp())
                },
            )
            .partial_deriv("sigma0", move |x: &DVector<f64>, sigma: f64| {
                x.map(|x_val| {
                    let exponent = -((x_val - peak).powi(2)) / (2.0 * sigma.powi(2));
                    (x_val - peak).powi(2) / sigma.powi(3) * exponent.exp()
                })
            });

        for i in 1..self.peak_markers.len() {
            let peak = self.peak_markers[i];
            builder_proxy = builder_proxy
                .function(
                    &[format!("sigma{}", i)],
                    move |x: &DVector<f64>, sigma: f64| {
                        x.map(|x_val| (-((x_val - peak).powi(2)) / (2.0 * sigma.powi(2))).exp())
                    },
                )
                .partial_deriv(
                    format!("sigma{}", i),
                    move |x: &DVector<f64>, sigma: f64| {
                        x.map(|x_val| {
                            let exponent = -((x_val - peak).powi(2)) / (2.0 * sigma.powi(2));
                            (x_val - peak).powi(2) / sigma.powi(3) * exponent.exp()
                        })
                    },
                );
        }

        let model = match builder_proxy.build() {
            Ok(model) => model,
            Err(e) => {
                log::error!("Failed to build model: {:?}", e);
                return;
            }
        };

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
                let nonlinear_parameters = fit_result.nonlinear_parameters();
                let nonlinear_variances = fit_statistics.nonlinear_parameters_variance();
                let linear_coefficients = match fit_result.linear_coefficients() {
                    Some(coefficients) => coefficients,
                    None => {
                        log::error!("Failed to get linear coefficients");
                        return;
                    }
                };
                let linear_variances = fit_statistics.linear_coefficients_variance();
                let mut params: Vec<GaussianParams> = Vec::new();

                for (i, &amplitude) in linear_coefficients.iter().enumerate() {
                    let sigma = nonlinear_parameters[i];
                    let sigma_variance = nonlinear_variances[i];
                    let mean = self.peak_markers[i];
                    let mean_uncertainty = 0.0;
                    let amplitude_variance = linear_variances[i];

                    if let Some(gaussian_params) = GaussianParams::new(
                        Value {
                            value: amplitude,
                            uncertainty: amplitude_variance.sqrt(),
                        },
                        Value {
                            value: mean,
                            uncertainty: mean_uncertainty,
                        },
                        Value {
                            value: sigma,
                            uncertainty: sigma_variance.sqrt(),
                        },
                        self.bin_width,
                    ) {
                        params.push(gaussian_params);
                    } else {
                        self.peak_markers.remove(i);
                        self.multi_gauss_fit_free_stdev_fixed_position();
                        return;
                    }
                }

                self.peak_markers.clear();
                for mean in &params {
                    self.peak_markers.push(mean.mean.value);
                }

                self.fit_params = Some(params);
                self.get_fit_lines();
            }
            Err(e) => {
                log::error!("Failed to fit model: {:?}", e);
            }
        }
    }

    pub fn multi_gauss_fit(&mut self) {
        if self.free_stddev && self.free_position {
            self.multi_gauss_fit_free_stddev_free_position();
        } else if !self.free_stddev && self.free_position {
            self.multi_gauss_fit_fixed_stdev_free_position();
        } else if !self.free_stddev && !self.free_position {
            self.multi_gauss_fit_fixed_stdev_fixed_position();
        } else if self.free_stddev && !self.free_position {
            self.multi_gauss_fit_free_stdev_fixed_position();
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

    pub fn composition_fit_points_polynomial(&self, coef: Vec<f64>) -> Vec<[f64; 2]> {
        // coef = [c0, c1, c2, ...] c0 + c1*x + c2*x^2 + ...
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
                let y_background = coef
                    .iter()
                    .enumerate()
                    .fold(0.0, |sum, (j, c)| sum + c * x.powi(j as i32));
                let y_total = y_gauss + y_background;
                [x, y_total]
            })
            .collect()
    }

    pub fn composition_fit_points_exponential(&self, a: f64, b: f64) -> Vec<[f64; 2]> {
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
                let y_background = a * (-x / b).exp();
                let y_total = y_gauss + y_background;
                [x, y_total]
            })
            .collect()
    }

    pub fn composition_fit_points_double_exponential(
        &self,
        a: f64,
        b: f64,
        c: f64,
        d: f64,
    ) -> Vec<[f64; 2]> {
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
                let y_background = a * (-x / b).exp() + c * (-x / d).exp();
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
