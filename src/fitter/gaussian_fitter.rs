use nalgebra::DVector;
use varpro::model::builder::SeparableModelBuilder;
use varpro::solvers::levmar::{LevMarProblemBuilder, LevMarSolver};

use egui::{Color32, Stroke};
use egui_plot::{Line, PlotPoint, PlotPoints, PlotUi};

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GaussianParams {
    pub amplitude: (f64, f64), // Value and uncertainty
    pub mean: (f64, f64),
    pub sigma: (f64, f64),
    pub fwhm: (f64, f64),
    pub area: (f64, f64),
}

impl GaussianParams {
    // Constructor that also calculates FWHM and area
    pub fn new(amplitude: (f64, f64), mean: (f64, f64), sigma: (f64, f64)) -> Self {
        let fwhm = Self::calculate_fwhm(sigma.0);
        let fwhm_uncertainty = Self::fwhm_uncertainty(sigma.1);

        let area = Self::calculate_area(amplitude.0, sigma.0);
        let area_uncertainty = Self::area_uncertainty(amplitude, sigma);

        GaussianParams {
            amplitude,
            mean,
            sigma,
            fwhm: (fwhm, fwhm_uncertainty),
            area: (area, area_uncertainty),
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
    fn area_uncertainty(amplitude: (f64, f64), sigma: (f64, f64)) -> f64 {
        let two_pi_sqrt = (2.0 * std::f64::consts::PI).sqrt();
        ((sigma.0 * two_pi_sqrt * amplitude.1).powi(2)
            + (amplitude.0 * two_pi_sqrt * sigma.1).powi(2))
        .sqrt()
    }
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GaussianFitter {
    pub fit_params: Option<Vec<GaussianParams>>,
    x: Vec<f64>,
    y: Vec<f64>,
    pub peak_markers: Vec<f64>,

    #[serde(skip)]
    pub decomposition_fit_line_points: Option<Vec<Vec<PlotPoint>>>,
}

impl GaussianFitter {
    pub fn new(x: Vec<f64>, y: Vec<f64>, peak_markers: Vec<f64>) -> Self {
        Self {
            fit_params: None,
            x,
            y,
            peak_markers,
            decomposition_fit_line_points: None,
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

        // convert x and y data to DVector
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
        let model = builder_proxy.build().unwrap();

        // extract the parameters
        let problem = LevMarProblemBuilder::new(model)
            .observations(y_data)
            .build()
            .unwrap();

        // let fit_result = LevMarSolver::default()
        //     .fit_with_statistics(problem)
        //     .expect("fit must succeed");

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
                    (amplitude, amplitude_variance.sqrt()),
                    (mean, mean_variance.sqrt()),
                    (sigma, sigma_variance.sqrt()),
                );

                // Log the Gaussian component parameters including FWHM and area
                log::info!("Peak {}: Amplitude: {:.2} ± {:.2}, Mean: {:.2} ± {:.2}, Std Dev: {:.2} ± {:.2}, FWHM: {:.2} ± {:.2}, Area: {:.2} ± {:.2}",
                    i, amplitude, amplitude_variance.sqrt(), mean, mean_variance.sqrt(), sigma, sigma_variance.sqrt(),
                    gaussian_params.fwhm.0, gaussian_params.fwhm.1, gaussian_params.area.0, gaussian_params.area.1);

                params.push(gaussian_params);
            }

            self.fit_params = Some(params);
        }
    }

    pub fn get_fit_decomposition_line_points(&mut self) {
        if let Some(fit_params) = &self.fit_params {
            let mut decomposition_fit_line_points = Vec::new();

            // Loop through each GaussianParams struct in fit_params
            for params in fit_params.iter() {
                let num_points = 100;
                let start = params.mean.0 - 5.0 * params.sigma.0; // Adjust start and end to be +/- 5 sigma from the mean
                let end = params.mean.0 + 5.0 * params.sigma.0;
                let step = (end - start) / num_points as f64;

                let plot_points: Vec<PlotPoint> = (0..=num_points)
                    .map(|i| {
                        let x = start + step * i as f64;
                        // Using coefficient (amplitude) for the Gaussian equation
                        let y = params.amplitude.0
                            * (-((x - params.mean.0).powi(2)) / (2.0 * params.sigma.0.powi(2)))
                                .exp();
                        PlotPoint::new(x, y)
                    })
                    .collect();

                decomposition_fit_line_points.push(plot_points);
            }

            self.decomposition_fit_line_points = Some(decomposition_fit_line_points);
        } else {
            self.decomposition_fit_line_points = None;
        }
    }

    pub fn draw_decomposition_fit_lines(&self, plot_ui: &mut PlotUi, color: Color32) {
        if let Some(decomposition_fit_line_points) = &self.decomposition_fit_line_points {
            for points in decomposition_fit_line_points.iter() {
                /*
                // get the gaussian parameters
                if let Some(params) = &self.fit_params {
                    let mean_text = format!("Mean: {:.2} ± {:.2}", params[index].mean.0, params[index].mean.1);
                    let fwhm_text = format!("FWHM: {:.2} ± {:.2}", params[index].fwhm.0, params[index].fwhm.1);
                    let area_text = format!("Area: {:.2} ± {:.2}", params[index].area.0, params[index].area.1);

                    let formatted_text = format!("{}\n{}\n{}", mean_text, fwhm_text, area_text);
                    // would like the stats to appear when hovering over the line
                    // add later?
                }
                */

                let line = Line::new(PlotPoints::Owned(points.clone()))
                    .color(color)
                    .stroke(Stroke::new(1.0, color));

                plot_ui.line(line);
            }
        }
    }

    pub fn calculate_convoluted_fit_points_with_background(
        &self,
        slope: f64,
        intercept: f64,
    ) -> Vec<PlotPoint> {
        let num_points = 1000;
        let min_x = self.x.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_x = self.x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let step = (max_x - min_x) / num_points as f64;

        (0..=num_points)
            .map(|i| {
                let x = min_x + step * i as f64;
                // Adjust the calculation for y_gauss to use the GaussianParams struct
                let y_gauss = self.fit_params.as_ref().map_or(0.0, |params| {
                    params.iter().fold(0.0, |sum, param| {
                        sum + param.amplitude.0
                            * (-((x - param.mean.0).powi(2)) / (2.0 * param.sigma.0.powi(2))).exp()
                    })
                });
                // Directly use slope and intercept to calculate the background estimate for x
                let y_background = slope * x + intercept;
                let y_total = y_gauss + y_background; // Correcting the Gaussian fit with the background estimate
                PlotPoint::new(x, y_total)
            })
            .collect()
    }
}
