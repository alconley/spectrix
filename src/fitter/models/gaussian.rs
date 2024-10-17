use nalgebra::{DMatrix, DVector};
use prettytable::{cell, row, Table};
use varpro::model::builder::SeparableModelBuilder;
use varpro::model::SeparableModel;
use varpro::solvers::levmar::{FitResult, LevMarProblemBuilder, LevMarSolver};
use varpro::statistics::FitStatistics; // Import prettytable

#[derive(Default, Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Value {
    pub value: f64,
    pub uncertainty: f64,
}

#[derive(Default, Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct GaussianParams {
    pub amplitude: Value,
    pub mean: Value,
    pub sigma: Value,
    pub fwhm: Value,
    pub area: Value,
    pub left_tail: Option<Value>,
    pub right_tail: Option<Value>,
}

impl GaussianParams {
    pub fn new(amplitude: Value, mean: Value, sigma: Value, bin_width: f64, left_tail: Option<Value>, right_tail: Option<Value>) -> Option<Self> {
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
            left_tail,
            right_tail,
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
        if let Some(left_tail) = &self.left_tail {
            ui.label(format!(
                "{:.2} ± {:.2}",
                left_tail.value, left_tail.uncertainty
            ));
        }
        if let Some(right_tail) = &self.right_tail {
            ui.label(format!(
                "{:.2} ± {:.2}",
                right_tail.value, right_tail.uncertainty
            ));
        }
    }

    pub fn fit_line_points(&self) -> Vec<[f64; 2]> {
        let num_points = 1000;
        let start = self.mean.value - 5.0 * self.sigma.value; // Adjust start and end to be +/- 5 sigma from the mean
        let end = self.mean.value + 5.0 * self.sigma.value;
        let step = (end - start) / num_points as f64;
    
        (0..num_points)
            .map(|i| {
                let x = start + step * i as f64;
                let z = (x - self.mean.value) / self.sigma.value;
                let y = if let Some(left_tail) = &self.left_tail {
                    if z <= -left_tail.value {
                        self.amplitude.value * (left_tail.value.powi(2) / 2.0 + left_tail.value * z).exp()
                    } else if let Some(right_tail) = &self.right_tail {
                        if z > right_tail.value {
                            self.amplitude.value * (right_tail.value.powi(2) / 2.0 - right_tail.value * z).exp()
                        } else {
                            self.amplitude.value * (-0.5 * z.powi(2)).exp() // Gaussian core
                        }
                    } else {
                        self.amplitude.value * (-0.5 * z.powi(2)).exp() // Gaussian core (right tail not enabled)
                    }
                } else if let Some(right_tail) = &self.right_tail {
                    if z > right_tail.value {
                        self.amplitude.value * (right_tail.value.powi(2) / 2.0 - right_tail.value * z).exp()
                    } else {
                        self.amplitude.value * (-0.5 * z.powi(2)).exp() // Gaussian core (left tail not enabled)
                    }
                } else {
                    self.amplitude.value * (-0.5 * z.powi(2)).exp() // Gaussian core (no tails)
                };
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
    pub left_tail: bool,
    pub right_tail: bool,
    pub bin_width: f64,
}

impl GaussianFitter {
    pub fn new(
        x: Vec<f64>,
        y: Vec<f64>,
        peak_markers: Vec<f64>,
        free_stddev: bool,
        free_position: bool,
        left_tail: bool,
        right_tail: bool,
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
            left_tail,
            right_tail,
            bin_width,
        }
    }

    fn piecewise_gaussian(
        x: &DVector<f64>, 
        mean: f64, 
        sigma: f64, 
        k_l: f64, 
        k_r: f64, 
        left_tail: &bool, 
        right_tail: &bool
    ) -> DVector<f64> {
        let z = x.map(|x_val| (x_val - mean) / sigma);
        z.map(|z_val| {
            if *left_tail && z_val <= -k_l {
                (k_l.powi(2) / 2.0 + k_l * z_val).exp()
            } else if *right_tail && k_r < z_val {
                (k_r.powi(2) / 2.0 - k_r * z_val).exp()
            } else {
                (-0.5 * z_val.powi(2)).exp() // Gaussian core
            }
        })
    }

    fn piecewise_gaussian_pd_mean(
        x: &DVector<f64>, 
        mean: f64, 
        sigma: f64, 
        k_l: f64, 
        k_r: f64, 
        left_tail: &bool, 
        right_tail: &bool
    ) -> DVector<f64> {
        let z = x.map(|x_val| (x_val - mean) / sigma);
        z.map(|z_val| {
            if *left_tail && z_val <= -k_l {
                (k_l.powi(2) / 2.0 + k_l * z_val).exp() * -(k_l / sigma)
            } else if *right_tail && k_r < z_val {
                (k_r.powi(2) / 2.0 - k_r * z_val).exp() * (k_r) / sigma
            } else {
                (-0.5 * z_val.powi(2)).exp() * (z_val / sigma)
            }
        })
    }
    
    fn piecewise_gaussian_pd_sigma(
        x: &DVector<f64>, 
        mean: f64, 
        sigma: f64, 
        k_l: f64, 
        k_r: f64, 
        left_tail: &bool, 
        right_tail: &bool
    ) -> DVector<f64> {
        let z = x.map(|x_val| (x_val - mean) / sigma);
        z.map(|z_val| {
            if *left_tail && z_val <= -k_l {
                (k_l.powi(2) / 2.0 + k_l * z_val).exp() * (-k_l) * (z_val / sigma)
            } else if *right_tail && k_r < z_val {
                (k_r.powi(2) / 2.0 - k_r * z_val).exp() * (k_r) * (z_val / sigma)
            } else {
                (-0.5 * z_val.powi(2)).exp() * (z_val.powi(2) / sigma)
            }
        })
    }

    fn piecewise_gaussian_pd_k_l(
        x: &DVector<f64>, 
        mean: f64, 
        sigma: f64, 
        k_l: f64, 
        k_r: f64, 
        left_tail: &bool, 
        right_tail: &bool
    ) -> DVector<f64> {
        let z = x.map(|x_val| (x_val - mean) / sigma);
        z.map(|z_val| {
            if *left_tail && z_val <= -k_l {
                (k_l + z_val) * (k_l.powi(2) / 2.0 + k_l * z_val).exp()
            } else if *right_tail && z_val > k_r {
                0.0 // No contribution from the right tail
            } else {
                0.0 // No contribution in the Gaussian core region
            }
        })
    }

    fn piecewise_gaussian_pd_k_r(
        x: &DVector<f64>, 
        mean: f64, 
        sigma: f64, 
        k_l: f64, 
        k_r: f64, 
        left_tail: &bool, 
        right_tail: &bool
    ) -> DVector<f64> {
        let z = x.map(|x_val| (x_val - mean) / sigma);
        z.map(|z_val| {
            if *right_tail && k_r < z_val {
                (k_r - z_val) * (k_r.powi(2) / 2.0 - k_r * z_val).exp()
            } else if *left_tail && z_val <= -k_l {
                0.0 // No contribution from the left tail
            } else {
                0.0 // No contribution in the Gaussian core region
            }
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

        log::info!("Parameter names: {:?}", parameter_names);
        log::info!("Initial guesses: {:?}", initial_guesses);

        let mut builder_proxy = SeparableModelBuilder::<f64>::new(parameter_names)
            .initial_parameters(initial_guesses)
            .independent_variable(x_data);

        let left_tail = self.left_tail;
        let right_tail = self.right_tail;
        let peak_markers = self.peak_markers.clone();

        let mut non_linear_parameters = Vec::new();

        for i in 0..peak_markers.len() {

            let mean = peak_markers[i];

            let mut function_params = Vec::new();
            if self.free_position {
                function_params.push(format!("mean{}", i));
                non_linear_parameters.push(format!("mean{}", i));
            }

            if self.free_stddev {
                function_params.push(format!("sigma{}", i));
                non_linear_parameters.push(format!("sigma{}", i));
            }

            if self.left_tail {
                function_params.push(format!("k_l{}", i));
                non_linear_parameters.push(format!("k_l{}", i));
            }

            if self.right_tail {
                function_params.push(format!("k_r{}", i));
                non_linear_parameters.push(format!("k_r{}", i));
            }

            if !self.free_stddev {
                function_params.push("sigma".to_string());
                if i == peak_markers.len() - 1 {
                    non_linear_parameters.push("sigma".to_string());
                }
            }

            let mean_str = format!("mean{}", i);
            let sigma_str = if !self.free_stddev {
                "sigma".to_string()
            } else {
                format!("sigma{}", i)
            };
            let k_l_str = format!("k_l{}", i);
            let k_r_str = format!("k_r{}", i);
            let k_l = 0.0;
            let k_r = 0.0;


            // 16 unique combinations, but since stdev not fixed but rather can vary, we have 8 unique combinations
            // 1: free_position = true, free_stddev = true, left_tail = true, right_tail = true
            // 2: free_position = true, free_stddev = true, left_tail = true, right_tail = false
            // 3: free_position = true, free_stddev = true, left_tail = false, right_tail = true
            // 4: free_position = true, free_stddev = true, left_tail = false, right_tail = false
            // 5: free_position = true, free_stddev = false, left_tail = true, right_tail = true
            // 6: free_position = true, free_stddev = false, left_tail = true, right_tail = false
            // 7: free_position = true, free_stddev = false, left_tail = false, right_tail = true
            // 8: free_position = true, free_stddev = false, left_tail = false, right_tail = false
            // 9: free_position = false, free_stddev = true, left_tail = true, right_tail = true
            // 10: free_position = false, free_stddev = true, left_tail = true, right_tail = false
            // 11: free_position = false, free_stddev = true, left_tail = false, right_tail = true
            // 12: free_position = false, free_stddev = true, left_tail = false, right_tail = false
            // 13: free_position = false, free_stddev = false, left_tail = true, right_tail = true
            // 14: free_position = false, free_stddev = false, left_tail = true, right_tail = false
            // 15: free_position = false, free_stddev = false, left_tail = false, right_tail = true
            // 16: free_position = false, free_stddev = false, left_tail = false, right_tail = false


            // Add combinations for each condition
            if self.free_position && self.left_tail && self.right_tail {
                builder_proxy = builder_proxy
                    .function(&function_params, move |x: &DVector<f64>, mean: f64, sigma: f64, k_l: f64, k_r: f64| {
                        Self::piecewise_gaussian(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                    })
                    .partial_deriv(mean_str, move |x: &DVector<f64>, mean: f64, sigma: f64, k_l: f64, k_r: f64| {
                        Self::piecewise_gaussian_pd_mean(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                    })
                    .partial_deriv(sigma_str, move |x: &DVector<f64>, mean: f64, sigma: f64, k_l: f64, k_r: f64| {
                        Self::piecewise_gaussian_pd_sigma(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                    })
                    .partial_deriv(k_l_str, move |x: &DVector<f64>, mean: f64, sigma: f64, k_l: f64, k_r: f64| {
                        Self::piecewise_gaussian_pd_k_l(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                    })
                    .partial_deriv(k_r_str, move |x: &DVector<f64>, mean: f64, sigma: f64, k_l: f64, k_r: f64| {
                        Self::piecewise_gaussian_pd_k_r(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                    });
            } else if self.free_position && self.left_tail && !self.right_tail {
                builder_proxy = builder_proxy
                .function(&function_params, move |x: &DVector<f64>, mean: f64, sigma: f64, k_l: f64| {
                    Self::piecewise_gaussian(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(mean_str, move |x: &DVector<f64>, mean: f64, sigma: f64, k_l: f64,| {
                    Self::piecewise_gaussian_pd_mean(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(sigma_str, move |x: &DVector<f64>, mean: f64, sigma: f64, k_l: f64| {
                    Self::piecewise_gaussian_pd_sigma(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(k_l_str, move |x: &DVector<f64>, mean: f64, sigma: f64, k_l: f64| {
                    Self::piecewise_gaussian_pd_k_l(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                });
            } else if self.free_position  && !self.left_tail && self.right_tail {
                builder_proxy = builder_proxy
                .function(&function_params, move |x: &DVector<f64>, mean: f64, sigma: f64, k_r: f64| {
                    Self::piecewise_gaussian(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(mean_str, move |x: &DVector<f64>, mean: f64, sigma: f64, k_r: f64| {
                    Self::piecewise_gaussian_pd_mean(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(sigma_str, move |x: &DVector<f64>, mean: f64, sigma: f64, k_r: f64| {
                    Self::piecewise_gaussian_pd_sigma(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(k_r_str, move |x: &DVector<f64>, mean: f64, sigma: f64, k_r: f64| {
                    Self::piecewise_gaussian_pd_k_r(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                });
            } else if self.free_position && !self.left_tail && !self.right_tail {
                builder_proxy = builder_proxy
                .function(&function_params, move |x: &DVector<f64>, mean: f64, sigma: f64| {
                    Self::piecewise_gaussian(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(mean_str, move |x: &DVector<f64>, mean: f64, sigma: f64| {
                    Self::piecewise_gaussian_pd_mean(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(sigma_str, move |x: &DVector<f64>, mean: f64, sigma: f64| {
                    Self::piecewise_gaussian_pd_sigma(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                });
            } else if !self.free_position && self.left_tail && self.right_tail {
                builder_proxy = builder_proxy
                .function(&function_params, move |x: &DVector<f64>, sigma: f64, k_l: f64, k_r: f64| {
                    Self::piecewise_gaussian(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(sigma_str, move |x: &DVector<f64>, sigma: f64, k_l: f64, k_r: f64| {
                    Self::piecewise_gaussian_pd_sigma(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(k_l_str, move |x: &DVector<f64>, sigma: f64, k_l: f64, k_r: f64| {
                    Self::piecewise_gaussian_pd_k_l(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(k_r_str, move |x: &DVector<f64>, sigma: f64, k_l: f64, k_r: f64| {
                    Self::piecewise_gaussian_pd_k_r(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                });
            } else if !self.free_position && self.left_tail && !self.right_tail {
                builder_proxy = builder_proxy
                .function(&function_params, move |x: &DVector<f64>, sigma: f64, k_l: f64| {
                    Self::piecewise_gaussian(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(sigma_str, move |x: &DVector<f64>, sigma: f64, k_l: f64| {
                    Self::piecewise_gaussian_pd_sigma(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(k_l_str, move |x: &DVector<f64>, sigma: f64, k_l: f64| {
                    Self::piecewise_gaussian_pd_k_l(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                });
            } else if !self.free_position && !self.left_tail && self.right_tail {
                builder_proxy = builder_proxy
                .function(&function_params, move |x: &DVector<f64>, sigma: f64, k_r: f64| {
                    Self::piecewise_gaussian(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(sigma_str, move |x: &DVector<f64>, sigma: f64, k_r: f64| {
                    Self::piecewise_gaussian_pd_sigma(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(k_r_str, move |x: &DVector<f64>, sigma: f64, k_r: f64| {
                    Self::piecewise_gaussian_pd_k_r(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                });
            } else if !self.free_position && !self.left_tail && !self.right_tail {
                builder_proxy = builder_proxy
                .function(&function_params, move |x: &DVector<f64>, sigma: f64| {
                    Self::piecewise_gaussian(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                })
                .partial_deriv(sigma_str, move |x: &DVector<f64>, sigma: f64| {
                    Self::piecewise_gaussian_pd_sigma(x, mean, sigma, k_l, k_r, &left_tail, &right_tail)
                });
            }
        }

        log::info!("Non-linear parameters: {:?}", non_linear_parameters);

        // Build the model and solve the problem
        let model = builder_proxy.build().expect("Failed to build the model");
    
        let problem = LevMarProblemBuilder::new(model)
            .observations(y_data)
            .build()
            .expect("Failed to build the fitting problem");
    
        match LevMarSolver::default().fit_with_statistics(problem) {
            Ok((fit_result, fit_statistics)) => {
                self.process_fit_result(fit_result, fit_statistics, non_linear_parameters);
            }
            Err(e) => {
                log::error!("Failed to fit the model: {:?}", e);
            }
        }
    }
    
    

    fn generate_free_stddev_free_position(&self) -> (Vec<String>, Vec<f64>) {
        let mut parameter_names = Vec::new();
        let mut initial_guesses = Vec::new();

        for (i, &mean) in self.peak_markers.iter().enumerate() {
            parameter_names.push(format!("mean{}", i));
            initial_guesses.push(mean);

            parameter_names.push(format!("sigma{}", i));
            initial_guesses.push(self.average_sigma());

            if self.left_tail {
                parameter_names.push(format!("k_l{}", i));
                initial_guesses.push(0.1);
            }
            if self.right_tail {
                parameter_names.push(format!("k_r{}", i));
                initial_guesses.push(0.1);
            }
        }

        (parameter_names, initial_guesses)
    }

    fn generate_fixed_stddev_free_position(&self) -> (Vec<String>, Vec<f64>) {
        let mut parameter_names = Vec::new();
        let mut initial_guesses = Vec::new();

        for (i, &mean) in self.peak_markers.iter().enumerate() {
            parameter_names.push(format!("mean{}", i));
            initial_guesses.push(mean);

            if self.left_tail {
                parameter_names.push(format!("k_l{}", i));
                initial_guesses.push(0.1);

            }
            if self.right_tail {
                parameter_names.push(format!("k_r{}", i));
                initial_guesses.push(0.1);

            }
        }

        parameter_names.push("sigma".to_string());
        initial_guesses.push(self.average_sigma());

        (parameter_names, initial_guesses)
    }

    fn generate_fixed_stddev_fixed_position(&self) -> (Vec<String>, Vec<f64>) {
        let mut parameter_names = Vec::new();
        let mut initial_guesses = Vec::new();

        for (i, &_mean) in self.peak_markers.iter().enumerate() {

            if self.left_tail {
                parameter_names.push(format!("k_l{}", i));
                initial_guesses.push(0.1);

            }
            if self.right_tail {
                parameter_names.push(format!("k_r{}", i));
                initial_guesses.push(0.1);

            }
        }

        parameter_names.push("sigma".to_string());
        initial_guesses.push(self.average_sigma());

        (parameter_names, initial_guesses)
    }

    fn generate_free_stddev_fixed_position(&self) -> (Vec<String>, Vec<f64>) {
        let mut parameter_names = Vec::new();
        let mut initial_guesses = Vec::new();

        for i in 0..self.peak_markers.len() {
            parameter_names.push(format!("sigma{}", i));
            initial_guesses.push(self.average_sigma());

            if self.left_tail {
                parameter_names.push(format!("k_l{}", i));
                initial_guesses.push(0.1);

            }
            if self.right_tail {
                parameter_names.push(format!("k_r{}", i));
                initial_guesses.push(0.1);

            }
        }

        (parameter_names, initial_guesses)
    }

    fn process_fit_result(
        &mut self,
        fit_result: FitResult<SeparableModel<f64>, false>,
        fit_statistics: FitStatistics<SeparableModel<f64>>,
        non_linear_parameters: Vec<String>,
    ) {
        let nonlinear_parameters = fit_result.nonlinear_parameters(); // DVector<f64>
        let nonlinear_variances = fit_statistics.nonlinear_parameters_variance(); // DVector<f64>
        let linear_coefficients = fit_result.linear_coefficients().unwrap(); // DVector<f64>
        let linear_variances = fit_statistics.linear_coefficients_variance(); // DVector<f64>
    
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
    
        log::info!("Nonlinear parameters: {:?}", nonlinear_parameters);
        log::info!("Nonlinear variances: {:?}", nonlinear_variances);
        log::info!("Linear coefficients: {:?}", linear_coefficients);
        log::info!("Linear variances: {:?}", linear_variances);
    
        let mut params: Vec<GaussianParams> = Vec::new();
        let mut current_index = 0;
    
        for (i, &amplitude) in linear_coefficients.iter().enumerate() {
            log::info!("Amplitude {}: {}", i, amplitude);
            let amplitude_uncertainty = linear_variances[i].sqrt();
    
            let mut mean = self.peak_markers[i];
            let mut mean_uncertainty = 0.0;
    
            let mut sigma = self.average_sigma();
            let mut sigma_uncertainty = 0.0;
    
            let mut left_tail: Option<f64> = None;
            let mut left_tail_uncertainty = 0.0;
    
            let mut right_tail: Option<f64> = None;
            let mut right_tail_uncertainty = 0.0;
    
            // Retrieve parameter names from `non_linear_parameters` vector
            let mean_str = format!("mean{}", i);
            let sigma_str = if !self.free_stddev {
                "sigma".to_string()
            } else {
                format!("sigma{}", i)
            };
            let k_l_str = format!("k_l{}", i);
            let k_r_str = format!("k_r{}", i);
    
            // Find the index of the parameter in the non_linear_parameters vector
            if let Some(mean_index) = non_linear_parameters.iter().position(|p| p == &mean_str) {
                mean = nonlinear_parameters[mean_index];
                mean_uncertainty = nonlinear_variances[mean_index].sqrt();
            }
    
            if let Some(sigma_index) = non_linear_parameters.iter().position(|p| p == &sigma_str) {
                sigma = nonlinear_parameters[sigma_index];
                sigma_uncertainty = nonlinear_variances[sigma_index].sqrt();
            }
    
            if let Some(k_l_index) = non_linear_parameters.iter().position(|p| p == &k_l_str) {
                left_tail = Some(nonlinear_parameters[k_l_index]);
                left_tail_uncertainty = nonlinear_variances[k_l_index].sqrt();
            }
    
            if let Some(k_r_index) = non_linear_parameters.iter().position(|p| p == &k_r_str) {
                right_tail = Some(nonlinear_parameters[k_r_index]);
                right_tail_uncertainty = nonlinear_variances[k_r_index].sqrt();
            }
    
            // Construct GaussianParams and add it to the list
            let amplitude_value = Value {
                value: amplitude,
                uncertainty: amplitude_uncertainty,
            };
            let mean_value = Value {
                value: mean,
                uncertainty: mean_uncertainty,
            };
            let sigma_value = Value {
                value: sigma,
                uncertainty: sigma_uncertainty,
            };
    
            let left_tail_value = left_tail.map(|val| Value {
                value: val,
                uncertainty: left_tail_uncertainty,
            });
    
            let right_tail_value = right_tail.map(|val| Value {
                value: val,
                uncertainty: right_tail_uncertainty,
            });
    
            let gaussian_param = GaussianParams::new(
                amplitude_value,
                mean_value,
                sigma_value,
                self.bin_width,
                left_tail_value,
                right_tail_value,
            );
    
            if let Some(param) = gaussian_param {
                params.push(param);
            }
        }
    
        // Update peak markers and other parameters
        self.peak_markers.clear();
        for mean in &params {
            self.peak_markers.push(mean.mean.value);
        }
    
        self.fit_params = Some(params);
        self.get_fit_lines();
    
        // Print fit statistics
        self.print_fit_statistics(&fit_statistics);
        self.print_peak_info();
    }
    
    
    // Function to print information about the fitted peaks using prettytable
    pub fn print_peak_info(&self) {
        if let Some(fit_params) = &self.fit_params {
            // Create a new table
            let mut table = Table::new();
            
            // Add a header row to the table
            table.add_row(row!["Peak", "Amplitude", "Mean", "Sigma", "FWHM", "Area", "Left Tail", "Right Tail"]);
            
            // Iterate over the fit parameters and add each row to the table
            for (i, params) in fit_params.iter().enumerate() {
                let left_tail_str = if let Some(left_tail) = &params.left_tail {
                    format!("{:.3} ± {:.3}", left_tail.value, left_tail.uncertainty)
                } else {
                    "-".to_string()
                };

                let right_tail_str = if let Some(right_tail) = &params.right_tail {
                    format!("{:.3} ± {:.3}", right_tail.value, right_tail.uncertainty)
                } else {
                    "-".to_string()
                };

                // Add a row with the peak info
                table.add_row(row![
                    i + 1, // Peak number
                    format!("{:.3} ± {:.3}", params.amplitude.value, params.amplitude.uncertainty),
                    format!("{:.3} ± {:.3}", params.mean.value, params.mean.uncertainty),
                    format!("{:.3} ± {:.3}", params.sigma.value, params.sigma.uncertainty),
                    format!("{:.3} ± {:.3}", params.fwhm.value, params.fwhm.uncertainty),
                    format!("{:.3} ± {:.3}", params.area.value, params.area.uncertainty),
                    left_tail_str,
                    right_tail_str
                ]);
            }

            // Print the table to the console
            table.printstd();
        } else {
            log::warn!("No fit parameters available to print.");
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

    fn print_matrix(matrix: &DMatrix<f64>) {
        let mut table = Table::new();

        // Iterate over the rows of the matrix
        for row in matrix.row_iter() {
            let mut table_row = Vec::new();
            for val in row.iter() {
                table_row.push(cell!(format!("{:.6}", val)));
            }
            // Add the row using add_row, but without row! macro
            table.add_row(prettytable::Row::new(table_row));
        }

        // Print the table
        table.printstd();
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