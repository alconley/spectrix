use nalgebra::DVector;
use prettytable::{row, Table};
use varpro::model::builder::SeparableModelBuilder;
use varpro::model::SeparableModel;
use varpro::solvers::levmar::{FitResult, LevMarProblemBuilder, LevMarSolver};
use varpro::statistics::FitStatistics; // Import prettytable

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

    pub fn multi_gauss_fit(&mut self) {
        self.fit_params = None;
        self.fit_lines = None;

        if self.x.len() != self.y.len() {
            log::error!("x_data and y_data must have the same length");
            return;
        }

        if self.peak_markers.is_empty() {
            // set peak marker at the x value when y is the maximum value of the data
            let max_y = self.y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let max_x = self
                .x
                .iter()
                .cloned()
                .zip(self.y.iter().cloned())
                .find(|&(_, y)| y == max_y)
                .map(|(x, _)| x)
                .unwrap_or_default();
            self.peak_markers.push(max_x);
        }

        let x_data = DVector::from_vec(self.x.clone());
        let y_data = DVector::from_vec(self.y.clone());

        let (parameter_names, initial_guesses) = match (self.free_stddev, self.free_position) {
            (true, true) => self.generate_free_stddev_free_position(),
            (false, true) => self.generate_fixed_stddev_free_position(),
            (false, false) => self.generate_fixed_stddev_fixed_position(),
            (true, false) => self.generate_free_stddev_fixed_position(),
        };

        let mut builder_proxy = SeparableModelBuilder::<f64>::new(parameter_names)
            .initial_parameters(initial_guesses)
            .independent_variable(x_data);

        for i in 0..self.peak_markers.len() {
            let sigma_param = if self.free_stddev {
                format!("sigma{}", i)
            } else {
                "sigma".to_string()
            };

            let mean_param = format!("mean{}", i);

            // If `mean` is free, include its function and derivatives
            if self.free_position {
                builder_proxy = builder_proxy
                    .function(&[mean_param.clone(), sigma_param.clone()], Self::gaussian)
                    .partial_deriv(mean_param, Self::gaussian_pd_mean)
                    .partial_deriv(sigma_param.clone(), Self::gaussian_pd_std_dev);
            } else {
                // If `mean` is fixed, only include the function and sigma's derivative
                let fixed_mean = self.peak_markers[i];
                builder_proxy = builder_proxy
                    .function(
                        [sigma_param.clone()],
                        move |x: &DVector<f64>, sigma: f64| {
                            x.map(|x_val| {
                                (-((x_val - fixed_mean).powi(2)) / (2.0 * sigma.powi(2))).exp()
                            })
                        },
                    )
                    .partial_deriv(sigma_param, move |x: &DVector<f64>, sigma: f64| {
                        x.map(|x_val| {
                            let exponent = -((x_val - fixed_mean).powi(2)) / (2.0 * sigma.powi(2));
                            (x_val - fixed_mean).powi(2) / sigma.powi(3) * exponent.exp()
                        })
                    });
            }
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
                self.process_fit_result(fit_result, fit_statistics);
            }
            Err(e) => {
                log::error!("Failed to fit model: {:?}", e);
            }
        }
    }

    fn generate_free_stddev_free_position(&self) -> (Vec<String>, Vec<f64>) {
        let mut parameter_names = Vec::new();
        let mut initial_guesses = Vec::new();

        for (i, &mean) in self.peak_markers.iter().enumerate() {
            parameter_names.push(format!("mean{}", i));
            parameter_names.push(format!("sigma{}", i));
            initial_guesses.push(mean);
            initial_guesses.push(self.average_sigma());
        }

        (parameter_names, initial_guesses)
    }

    fn generate_fixed_stddev_free_position(&self) -> (Vec<String>, Vec<f64>) {
        let mut parameter_names = Vec::new();
        let mut initial_guesses = Vec::new();

        for (i, &mean) in self.peak_markers.iter().enumerate() {
            parameter_names.push(format!("mean{}", i));
            initial_guesses.push(mean);
        }

        parameter_names.push("sigma".to_string());
        initial_guesses.push(self.average_sigma());

        (parameter_names, initial_guesses)
    }

    fn generate_fixed_stddev_fixed_position(&self) -> (Vec<String>, Vec<f64>) {
        let parameter_names = vec!["sigma".to_string()];
        let initial_guesses = vec![self.average_sigma()];

        (parameter_names, initial_guesses)
    }

    fn generate_free_stddev_fixed_position(&self) -> (Vec<String>, Vec<f64>) {
        let mut parameter_names = Vec::new();
        let mut initial_guesses = Vec::new();

        for i in 0..self.peak_markers.len() {
            parameter_names.push(format!("sigma{}", i));
            initial_guesses.push(self.average_sigma());
        }

        (parameter_names, initial_guesses)
    }

    fn process_fit_result(
        &mut self,
        fit_result: FitResult<SeparableModel<f64>, false>,
        fit_statistics: FitStatistics<SeparableModel<f64>>,
    ) {
        let nonlinear_parameters = fit_result.nonlinear_parameters(); // DVector<f64>
        let nonlinear_variances = fit_statistics.nonlinear_parameters_variance(); // DVector<f64>
        let linear_coefficients = match fit_result.linear_coefficients() {
            Some(coefficients) => coefficients,
            None => {
                log::error!("Failed to get linear coefficients.");
                return;
            }
        };
        let linear_variances = fit_statistics.linear_coefficients_variance(); // DVector<f64>

        // Check if sizes match
        if nonlinear_parameters.len() != nonlinear_variances.len() {
            log::error!(
                "Mismatch in sizes: nonlinear_parameters ({}), nonlinear_variances ({}).",
                nonlinear_parameters.len(),
                nonlinear_variances.len()
            );
            return;
        }

        if nonlinear_parameters.is_empty() || linear_coefficients.is_empty() {
            log::error!("Empty fit result: no nonlinear or linear parameters.");
            return;
        }

        let mut params: Vec<GaussianParams> = Vec::new();
        let sigma_value = if self.free_stddev {
            None // Free sigma, so extract it for each Gaussian
        } else {
            match self.extract_sigma(&nonlinear_parameters, &nonlinear_variances) {
                Some(sigma) => Some(sigma),
                None => {
                    log::error!("Failed to extract sigma value.");
                    return;
                }
            }
        };

        for (i, &amplitude) in linear_coefficients.iter().enumerate() {
            let mean_value = if self.free_position {
                match self.extract_mean(i, &nonlinear_parameters, &nonlinear_variances) {
                    Some(mean) => mean,
                    None => {
                        log::error!("Failed to extract mean value for Gaussian {}.", i);
                        return;
                    }
                }
            } else {
                Value {
                    value: self.peak_markers.get(i).copied().unwrap_or_default(),
                    uncertainty: 0.0, // Fixed position, so uncertainty is 0
                }
            };

            let sigma_value = sigma_value.clone().unwrap_or_else(|| {
                self.extract_sigma_for_gaussian(i, &nonlinear_parameters, &nonlinear_variances)
                    .unwrap_or_else(|| {
                        log::error!("Failed to extract sigma for Gaussian {}.", i);
                        Value {
                            value: 0.0,
                            uncertainty: 0.0,
                        }
                    })
            });

            let amplitude_variance = linear_variances.get(i).copied().unwrap_or(0.0);

            if let Some(gaussian_params) = GaussianParams::new(
                Value {
                    value: amplitude,
                    uncertainty: amplitude_variance.sqrt(),
                },
                mean_value,
                sigma_value,
                self.bin_width,
            ) {
                params.push(gaussian_params);
            } else {
                log::error!(
                    "Invalid Gaussian parameters for Gaussian {}. Removing peak and trying again.",
                    i
                );
                self.peak_markers.remove(i);
                self.multi_gauss_fit(); // Retry the fit after removing this Gaussian
                return;
            }
        }

        self.peak_markers.clear();
        for mean in &params {
            self.peak_markers.push(mean.mean.value);
        }

        self.fit_params = Some(params);
        self.get_fit_lines();

        self.print_fit_statistics(&fit_statistics);
        self.print_peak_info();
    }

    fn extract_mean(
        &self,
        index: usize,
        nonlinear_parameters: &DVector<f64>,
        nonlinear_variances: &DVector<f64>,
    ) -> Option<Value> {
        if index >= nonlinear_parameters.len() || index >= nonlinear_variances.len() {
            log::error!("Index out of bounds when extracting mean.");
            return None;
        }
        Some(Value {
            value: nonlinear_parameters[index],
            uncertainty: nonlinear_variances[index].sqrt(),
        })
    }

    fn extract_sigma(
        &self,
        nonlinear_parameters: &DVector<f64>,
        nonlinear_variances: &DVector<f64>,
    ) -> Option<Value> {
        let sigma_index = nonlinear_parameters.len() - 1; // Sigma is the last parameter when fixed
        if sigma_index >= nonlinear_variances.len() {
            log::error!("Sigma index out of bounds when extracting sigma.");
            return None;
        }
        Some(Value {
            value: nonlinear_parameters[sigma_index],
            uncertainty: nonlinear_variances[sigma_index].sqrt(),
        })
    }

    fn extract_sigma_for_gaussian(
        &self,
        index: usize,
        nonlinear_parameters: &DVector<f64>,
        nonlinear_variances: &DVector<f64>,
    ) -> Option<Value> {
        let sigma_index = index * 2 + 1; // Free sigma, after each mean
        if sigma_index >= nonlinear_parameters.len() || sigma_index >= nonlinear_variances.len() {
            log::error!(
                "Index out of bounds when extracting sigma for Gaussian {}.",
                index
            );
            return None;
        }
        Some(Value {
            value: nonlinear_parameters[sigma_index],
            uncertainty: nonlinear_variances[sigma_index].sqrt(),
        })
    }

    pub fn print_peak_info(&self) {
        if let Some(fit_params) = &self.fit_params {
            // Create a new table
            let mut table = Table::new();

            // Add the header row
            table.add_row(row!["Index", "Amplitude", "Mean", "Sigma", "FWHM", "Area"]);

            // Add each peak's parameters as rows
            for (i, params) in fit_params.iter().enumerate() {
                table.add_row(row![
                    i,
                    format!(
                        "{:.3} ± {:.3}",
                        params.amplitude.value, params.amplitude.uncertainty
                    ),
                    format!("{:.3} ± {:.3}", params.mean.value, params.mean.uncertainty),
                    format!(
                        "{:.3} ± {:.3}",
                        params.sigma.value, params.sigma.uncertainty
                    ),
                    format!("{:.3} ± {:.3}", params.fwhm.value, params.fwhm.uncertainty),
                    format!("{:.3} ± {:.3}", params.area.value, params.area.uncertainty),
                ]);
            }

            // Print the table to the terminal
            table.printstd();
        } else {
            println!("No fit parameters available to display.");
        }
    }

    pub fn print_fit_statistics(&self, fit_statistics: &FitStatistics<SeparableModel<f64>>) {
        // Print covariance matrix
        // let covariance_matrix = fit_statistics.covariance_matrix();
        // println!("Covariance Matrix:");
        // Self::print_matrix(covariance_matrix);

        // // Print correlation matrix
        // let correlation_matrix = fit_statistics.calculate_correlation_matrix();
        // println!("Correlation Matrix:");
        // Self::print_matrix(&correlation_matrix);

        // Print regression standard error
        let regression_standard_error = fit_statistics.regression_standard_error();
        println!(
            "Regression Standard Error: {:.6}",
            regression_standard_error
        );

        // Print reduced chi-squared
        let reduced_chi2 = fit_statistics.reduced_chi2();
        println!("Reduced Chi-Squared: {:.6}", reduced_chi2);
    }

    // // Helper function to print a matrix using prettytable
    // fn print_matrix(matrix: &DMatrix<f64>) {
    //     let mut table = Table::new();

    //     // Iterate over the rows of the matrix
    //     for row in matrix.row_iter() {
    //         let mut table_row = Vec::new();
    //         for val in row.iter() {
    //             table_row.push(cell!(format!("{:.6}", val)));
    //         }
    //         // Add the row using add_row, but without row! macro
    //         table.add_row(prettytable::Row::new(table_row));
    //     }

    //     // Print the table
    //     table.printstd();
    // }

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
