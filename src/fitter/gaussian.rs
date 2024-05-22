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
    pub fn new(amplitude: Value, mean: Value, sigma: Value) -> Self {
        let fwhm = Self::calculate_fwhm(sigma.value);
        let fwhm_uncertainty = Self::fwhm_uncertainty(sigma.uncertainty);

        let area = Self::calculate_area(amplitude.value, sigma.value);
        let area_uncertainty = Self::area_uncertainty(amplitude.clone(), sigma.clone());

        GaussianParams {
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
        }
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
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GaussianFitter {
    x: Vec<f64>,
    y: Vec<f64>,
    pub peak_markers: Vec<f64>,
    pub fit_params: Option<Vec<GaussianParams>>,
    pub fit_lines: Option<Vec<Vec<(f64, f64)>>>,
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

    fn linear(x: &DVector<f64>, b: f64) -> DVector<f64> {
        x.map(|x_val| (x_val * b ))
    }

    fn linear_pd(x: &DVector<f64>, b: f64) -> DVector<f64> {
        x.map(|x_val| ( x_val ))
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

        // change initial sigma guess to something more resonable later
        initial_guesses.push(1.0);

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

        // convert x and y data to DVector
        let x_data = DVector::from_vec(self.x.clone());
        let y_data = DVector::from_vec(self.y.clone());

        let initial_guess = self.initial_guess();

        let parameter_names = self.generate_parameter_names();

        // Add parameters for the first peak manually
        let mut builder_proxy = SeparableModelBuilder::<f64>::new(parameter_names)
            .initial_parameters(initial_guess)
            .independent_variable(x_data)
            .function(&["b"], Self::linear)
            .partial_deriv("b", Self::linear_pd)
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
        let model = builder_proxy.build().unwrap();

        // extract the parameters
        let problem = LevMarProblemBuilder::new(model)
            .observations(y_data)
            .build()
            .unwrap();

        if let Ok((fit_result, fit_statistics)) =
            LevMarSolver::default().fit_with_statistics(problem)
        {
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

            // clear peak markers and update with the mean of the gaussians
            self.peak_markers.clear();

            // Assuming the amplitude (c) for each Gaussian comes first in linear_coefficients
            for (i, &amplitude) in linear_coefficients.iter().enumerate() {
                let mean = nonlinear_parameters[i];
                // update peak markers
                self.peak_markers.push(mean);

                let mean_variance = nonlinear_variances[i];
                let amplitude_variance = linear_variances[i];

                // Create a GaussianParams instance which now includes FWHM and area calculations
                let gaussian_params = GaussianParams::new(
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
                );

                // Log the Gaussian component parameters including FWHM and area
                log::info!("Peak {}: Amplitude: {:.2} ± {:.2}, Mean: {:.2} ± {:.2}, Std Dev: {:.2} ± {:.2}, FWHM: {:.2} ± {:.2}, Area: {:.2} ± {:.2}",
                    i, amplitude, amplitude_variance.sqrt(), mean, mean_variance.sqrt(), sigma, sigma_variance.sqrt(),
                    gaussian_params.fwhm.value, gaussian_params.fwhm.uncertainty, gaussian_params.area.value, gaussian_params.area.uncertainty);

                params.push(gaussian_params);
            }

            self.fit_params = Some(params);

            self.get_fit_lines();
        }
    }

    pub fn get_fit_lines(&mut self) {
        if let Some(fit_params) = &self.fit_params {
            let mut fit_lines = Vec::new();

            for params in fit_params.iter() {
                let num_points = 1000;
                let start = params.mean.value - 5.0 * params.sigma.value; // Adjust start and end to be +/- 5 sigma from the mean
                let end = params.mean.value + 5.0 * params.sigma.value;
                let step = (end - start) / num_points as f64;

                let mut fit_line_points = Vec::new();
                for i in 0..num_points {
                    let x = start + step * i as f64;
                    // Using coefficient (amplitude) for the Gaussian equation
                    let y = params.amplitude.value
                        * (-((x - params.mean.value).powi(2)) / (2.0 * params.sigma.value.powi(2)))
                            .exp();
                    fit_line_points.push((x, y));
                }

                fit_lines.push(fit_line_points);
            }

            self.fit_lines = Some(fit_lines);
        } else {
            self.fit_lines = None;
        }
    }

    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi, color: egui::Color32) {
        if let Some(fit_lines) = &self.fit_lines {
            for fit in fit_lines.iter() {
                let points: Vec<egui_plot::PlotPoint> = fit
                    .iter()
                    .map(|(x, y)| egui_plot::PlotPoint::new(*x, *y))
                    .collect();

                let line = egui_plot::Line::new(egui_plot::PlotPoints::Owned(points.clone()))
                    .color(color)
                    .stroke(egui::Stroke::new(1.0, color));

                plot_ui.line(line);
            }
        }
    }
}

// fn decompostition_fit_points(&self, params: GaussianParams, num_points: i32) -> Vec<(f64, f64)> {

//     let start = params.mean.0 - 5.0 * params.sigma.0; // Adjust start and end to be +/- 5 sigma from the mean
//     let end = params.mean.0 + 5.0 * params.sigma.0;
//     let step = (end - start) / num_points as f64;

//     let mut decomposition_points = Vec::new();
//     for i in 0..num_points {
//         let x = start + step * i as f64;
//         // Using coefficient (amplitude) for the Gaussian equation
//         let y = params.amplitude.0
//             * (-((x - params.mean.0).powi(2)) / (2.0 * params.sigma.0.powi(2)))
//                 .exp();
//         decomposition_points.push((x, y));
//     }

//     decomposition_points
// }

// pub fn get_fit_decomposition_line_points(&mut self) {
//     if let Some(fit_params) = &self.fit_params {
//         let mut decomposition_fit_line_points = Vec::new();

//         // Loop through each GaussianParams struct in fit_params
//         for params in fit_params.iter() {
//             let num_points = 100;
//             let start = params.mean.0 - 5.0 * params.sigma.0; // Adjust start and end to be +/- 5 sigma from the mean
//             let end = params.mean.0 + 5.0 * params.sigma.0;
//             let step = (end - start) / num_points as f64;

//             let plot_points: Vec<PlotPoint> = (0..=num_points)
//                 .map(|i| {
//                     let x = start + step * i as f64;
//                     // Using coefficient (amplitude) for the Gaussian equation
//                     let y = params.amplitude.0
//                         * (-((x - params.mean.0).powi(2)) / (2.0 * params.sigma.0.powi(2)))
//                             .exp();
//                     PlotPoint::new(x, y)
//                 })
//                 .collect();

//             decomposition_fit_line_points.push(plot_points);
//         }

//         self.decomposition_fit_line_points = Some(decomposition_fit_line_points);
//     } else {
//         self.decomposition_fit_line_points = None;
//     }
// }

// pub fn draw_fit(&self, plot_ui: &mut PlotUi, color: Color32) {
//     if let Some(fit_lines) = &self.fit_lines {
//         for fit in fit_lines.iter() {
//             let points: Vec<PlotPoint> = fit.iter().map(|(x, y)| PlotPoint::new(*x, *y)).collect();

//             let line = Line::new(PlotPoints::Owned(points.clone()))
//                 .color(color)
//                 .stroke(Stroke::new(1.0, color));

//             plot_ui.line(line);
//         }
//     }
// }

// pub fn calculate_convoluted_fit_points_with_background(
//     &self,
//     slope: f64,
//     intercept: f64,
// ) -> Vec<PlotPoint> {
//     let num_points = 1000;
//     let min_x = self.x.iter().cloned().fold(f64::INFINITY, f64::min);
//     let max_x = self.x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
//     let step = (max_x - min_x) / num_points as f64;

//     (0..=num_points)
//         .map(|i| {
//             let x = min_x + step * i as f64;
//             // Adjust the calculation for y_gauss to use the GaussianParams struct
//             let y_gauss = self.fit_params.as_ref().map_or(0.0, |params| {
//                 params.iter().fold(0.0, |sum, param| {
//                     sum + param.amplitude.0
//                         * (-((x - param.mean.0).powi(2)) / (2.0 * param.sigma.0.powi(2))).exp()
//                 })
//             });
//             // Directly use slope and intercept to calculate the background estimate for x
//             let y_background = slope * x + intercept;
//             let y_total = y_gauss + y_background; // Correcting the Gaussian fit with the background estimate
//             PlotPoint::new(x, y_total)
//         })
//         .collect()
// }
