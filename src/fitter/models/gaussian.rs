use crate::fitter::common::{Data, Parameter};
use crate::fitter::main_fitter::{BackgroundModel, BackgroundResult};
use crate::fitter::models::exponential::ExponentialFitter;
use crate::fitter::models::linear::LinearFitter;
use crate::fitter::models::powerlaw::PowerLawFitter;
use crate::fitter::models::quadratic::QuadraticFitter;

use pyo3::{
    prelude::*,
    types::{PyDict, PyModule},
};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GaussianParameters {
    pub amplitude: Parameter,
    pub mean: Parameter,
    pub sigma: Parameter,
    pub fwhm: Parameter,
    pub area: Parameter,
    pub fit_points: Vec<[f64; 2]>, // Vector of (x, y) points representing the Gaussian curve
}

impl Default for GaussianParameters {
    fn default() -> Self {
        GaussianParameters {
            amplitude: Parameter {
                name: "amplitude".to_string(),
                ..Default::default()
            },
            mean: Parameter {
                name: "mean".to_string(),
                ..Default::default()
            },
            sigma: Parameter {
                name: "sigma".to_string(),
                ..Default::default()
            },
            fwhm: Parameter {
                name: "fwhm".to_string(),
                ..Default::default()
            },
            area: Parameter {
                name: "area".to_string(),
                ..Default::default()
            },
            fit_points: Vec::new(),
        }
    }
}

impl GaussianParameters {
    pub fn new(
        amp: (f64, f64),
        mean: (f64, f64),
        sigma: (f64, f64),
        fwhm: (f64, f64),
        area: (f64, f64),
    ) -> Self {
        GaussianParameters {
            amplitude: Parameter {
                name: "amplitude".to_string(),
                value: Some(amp.0),
                uncertainty: Some(amp.1),
                ..Default::default()
            },
            mean: Parameter {
                name: "mean".to_string(),
                value: Some(mean.0),
                uncertainty: Some(mean.1),
                ..Default::default()
            },
            sigma: Parameter {
                name: "sigma".to_string(),
                value: Some(sigma.0),
                uncertainty: Some(sigma.1),
                ..Default::default()
            },
            fwhm: Parameter {
                name: "fwhm".to_string(),
                value: Some(fwhm.0),
                uncertainty: Some(fwhm.1),
                ..Default::default()
            },
            area: Parameter {
                name: "area".to_string(),
                value: Some(area.0),
                uncertainty: Some(area.1),
                ..Default::default()
            },
            fit_points: Vec::new(),
        }
    }

    /// Function to generate fit points 5 sigma out from the mean.
    /// Fit points are generated in the range [mean - 5 * sigma, mean + 5 * sigma].
    pub fn generate_fit_points(&mut self, num_points: usize) {
        if let (Some(mean), Some(sigma)) = (self.mean.value, self.sigma.value) {
            let range_min = mean - 5.0 * sigma;
            let range_max = mean + 5.0 * sigma;
            let step_size = (range_max - range_min) / (num_points as f64);

            self.fit_points.clear();
            for i in 0..=num_points {
                let x = range_min + i as f64 * step_size;
                let y = self.amplitude.value.unwrap_or(1.0)
                    * (-((x - mean).powi(2)) / (2.0 * sigma.powi(2))).exp();
                self.fit_points.push([x, y]);
            }
        }
    }

    pub fn params_ui(&self, ui: &mut egui::Ui) {
        ui.label(format!(
            "{:.2} ± {:.2}",
            self.mean.value.unwrap_or(0.0),
            self.mean.uncertainty.unwrap_or(0.0)
        ));

        ui.label(format!(
            "{:.2} ± {:.2}",
            self.fwhm.value.unwrap_or(0.0),
            self.fwhm.uncertainty.unwrap_or(0.0)
        ));

        ui.label(format!(
            "{:.2} ± {:.2}",
            self.area.value.unwrap_or(0.0),
            self.area.uncertainty.unwrap_or(0.0)
        ));

        ui.label(format!(
            "{:.2} ± {:.2}",
            self.amplitude.value.unwrap_or(0.0),
            self.amplitude.uncertainty.unwrap_or(0.0)
        ));

        ui.label(format!(
            "{:.2} ± {:.2}",
            self.sigma.value.unwrap_or(0.0),
            self.sigma.uncertainty.unwrap_or(0.0)
        ));
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GaussianFitSettings {
    pub equal_stdev: bool,
    pub free_position: bool,
    pub bin_width: f64,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GaussianFitter {
    pub data: Data,
    pub peak_markers: Vec<f64>,
    pub fit_settings: GaussianFitSettings,
    pub background_model: BackgroundModel,
    pub background_result: Option<BackgroundResult>,
    pub fit_result: Vec<GaussianParameters>,
    pub fit_points: Vec<[f64; 2]>,
    pub fit_report: String,
}

impl GaussianFitter {
    pub fn new(
        data: Data,
        peak_markers: Vec<f64>,
        background_model: BackgroundModel,
        background_result: Option<BackgroundResult>,
        equal_stdev: bool,
        free_position: bool,
        bin_width: f64,
    ) -> Self {
        Self {
            data,
            peak_markers,
            background_model,
            background_result,
            fit_settings: GaussianFitSettings {
                equal_stdev,
                free_position,
                bin_width,
            },
            fit_result: Vec::new(),
            fit_points: Vec::new(),
            fit_report: String::new(),
        }
    }

    pub fn lmfit(&mut self) -> PyResult<()> {
        Python::with_gil(|py| {
            // let sys = py.import_bound("sys")?;
            // let version: String = sys.getattr("version")?.extract()?;
            // let executable: String = sys.getattr("executable")?.extract()?;
            // println!("Using Python version: {}", version);
            // println!("Python executable: {}", executable);

            // Check if the `uproot` module can be imported
            match py.import_bound("lmfit") {
                Ok(_) => {
                    // println!("Successfully imported `lmfit` module.");
                }
                Err(_) => {
                    eprintln!("Error: `lmfit` module could not be found. Make sure you are using the correct Python environment with `lmfit` installed.");
                    return Err(PyErr::new::<pyo3::exceptions::PyImportError, _>(
                        "`lmfit` module not available",
                    ));
                }
            }

            // Define the Python code as a module
            let code = r#"
import numpy as np
import lmfit
# from sigfig import round

def gaussian(x, amplitude, mean, sigma):
    return amplitude * np.exp(-(x - mean)**2 / (2 * sigma**2))

def MultipleGaussianFit(x_data: list, y_data: list, peak_markers: list, bin_width: float,
                        equal_sigma: bool = True, free_position: bool = True,
                        background_params: dict = None):
    """
    Multiple Gaussian fit function with background model support.
    
    Parameters:
    - x_data, y_data: Lists of data points.
    - peak_markers: List of peak positions for the Gaussians.
    - equal_sigma: Whether to constrain all Gaussians to have the same sigma.
    - free_position: Whether to allow the positions of Gaussians to vary.
    - background_params: Dictionary containing background model type and parameters.
    """
    
    # Default background params if none are provided
    if background_params is None:
        background_params = {
            'bg_type': 'linear',
            'slope': ("slope", -np.inf, np.inf, 0.0, True),
            'intercept': ("intercept", -np.inf, np.inf, 0.0, True),
            'a': ("a", -np.inf, np.inf, 0.0, True),
            'b': ("b", -np.inf, np.inf, 0.0, True),
            'c': ("c", -np.inf, np.inf, 0.0, True),
            'exponent': ("exponent", -np.inf, np.inf, 0.0, True),
            'amplitude': ("amplitude", -np.inf, np.inf, 0.0, True),
            'decay': ("decay", -np.inf, np.inf, 0.0, True),
        }
    
    bg_type = background_params.get('bg_type', 'linear')
    slope = background_params.get('slope')
    intercept = background_params.get('intercept')
    a = background_params.get('a')
    b = background_params.get('b')
    c = background_params.get('c')
    amplitude = background_params.get('amplitude')
    exponent = background_params.get('exponent')
    decay = background_params.get('decay')

    # Initialize the model with or without a background based on bg_type
    if bg_type == 'linear': 
        model = lmfit.models.LinearModel(prefix='bg_')
        params = model.make_params(slope=slope[3], intercept=intercept[3])
        params['bg_slope'].set(min=slope[1], max=slope[2], value=slope[3], vary=slope[4])
        params['bg_intercept'].set(min=intercept[1], max=intercept[2], value=intercept[3], vary=intercept[4])
    elif bg_type == 'quadratic':
        model = lmfit.models.QuadraticModel(prefix='bg_')
        params = model.make_params(a=a[3], b=b[3], c=c[3])
        params['bg_a'].set(min=a[1], max=a[2], value=a[3], vary=a[4])
        params['bg_b'].set(min=b[1], max=b[2], value=b[3], vary=b[4])
        params['bg_c'].set(min=c[1], max=c[2], value=c[3], vary=c[4])
    elif bg_type == 'exponential':
        model = lmfit.models.ExponentialModel(prefix='bg_')
        params = model.make_params(amplitude=amplitude[3], decay=decay[3])
        params['bg_amplitude'].set(min=amplitude[1], max=amplitude[2], value=amplitude[3], vary=amplitude[4])
        params['bg_decay'].set(min=decay[1], max=decay[2], value=decay[3], vary=decay[4])
    elif bg_type == 'powerlaw':
        model = lmfit.models.PowerLawModel(prefix='bg_')
        params = model.make_params(amplitude=amplitude[3], exponent=exponent[3])
        params['bg_amplitude'].set(min=amplitude[1], max=amplitude[2], value=amplitude[3], vary=amplitude[4])
        params['bg_exponent'].set(min=exponent[1], max=exponent[2], value=exponent[3], vary=exponent[4])
    elif bg_type == 'none':
        model = None
        params = lmfit.Parameters()
    else:
        raise ValueError("Unsupported background model")

    first_gaussian = lmfit.Model(gaussian, prefix=f'g0_')

    if model is None:
        model = first_gaussian
    else:
        model += first_gaussian
    
    if len(peak_markers) == 0:
        peak_markers = [x_data[np.argmax(y_data)]]


    peak_markers = sorted(peak_markers)  # sort the peak markers in ascending order

    estimated_amplitude = 1000
    estimated_sigma = 10

    params.update(first_gaussian.make_params(amplitude=estimated_amplitude, mean=peak_markers[0], sigma=estimated_sigma))
    params['g0_sigma'].set(min=0)  # Initial constraint for the first Gaussian's sigma
    params[f"g0_amplitude"].set(min=0)

    params.add(f'g0_fwhm', expr=f'2.35482 * g0_sigma')  # FWHM = 2 * sqrt(2 * ln(2)) * sigma
    params[f"g0_fwhm"].set(min=0)

    params.add(f'g0_area', expr=f'g0_amplitude * sqrt(2 * pi) * g0_sigma / {bin_width}')  # Area under the Gaussian
    params[f"g0_area"].set(min=0)

    if not free_position:
        params['g0_mean'].set(vary=False)

    params['g0_mean'].set(min=x_data[0], max=peak_markers[1] if len(peak_markers) > 1 else x_data[-1])

    # Add additional Gaussians
    for i, peak in enumerate(peak_markers[1:], start=1):
        g = lmfit.Model(gaussian, prefix=f'g{i}_')
        model += g

        estimated_amplitude = 1000
        params.update(g.make_params(amplitude=estimated_amplitude, mean=peak, sigma=10))

        min_mean = peak_markers[i-1]
        max_mean = peak_markers[i+1] if i + 1 < len(peak_markers) else x_data[-1]
        params[f'g{i}_mean'].set(min=min_mean, max=max_mean)

        params.add(f'g{i}_fwhm', expr=f'2.35482 * g{i}_sigma')
        params[f"g{i}_fwhm"].set(min=0)

        params.add(f'g{i}_area', expr=f'g{i}_amplitude * sqrt(2 * pi) * g{i}_sigma / {bin_width}')
        params[f"g{i}_area"].set(min=0)

        if equal_sigma:
            params[f'g{i}_sigma'].set(expr='g0_sigma')
        else:
            params[f'g{i}_sigma'].set(min=0)

        params[f'g{i}_amplitude'].set(min=0)

        if not free_position:
            params[f'g{i}_mean'].set(vary=False)

    # Fit the model to the data
    result = model.fit(y_data, params, x=x_data)

    print("\nInitial Parameter Guesses:")
    params.pretty_print()

    print("\nFit Report:")
    print(result.fit_report())

    # Extract Gaussian and background parameters
    gaussian_params = []
    for i in range(len(peak_markers)):
        amplitude = float(result.params[f'g{i}_amplitude'].value)
        amplitude_uncertainty = result.params[f'g{i}_amplitude'].stderr or 0.0
        mean = float(result.params[f'g{i}_mean'].value)
        mean_uncertainty = result.params[f'g{i}_mean'].stderr or 0.0
        sigma = float(result.params[f'g{i}_sigma'].value)
        sigma_uncertainty = result.params[f'g{i}_sigma'].stderr or 0.0
        fwhm = float(result.params[f'g{i}_fwhm'].value)
        fwhm_uncertainty = result.params[f'g{i}_fwhm'].stderr or 0.0
        area = float(result.params[f'g{i}_area'].value)
        area_uncertainty = result.params[f'g{i}_area'].stderr or 0.0

        gaussian_params.append((
            amplitude, amplitude_uncertainty, mean, mean_uncertainty,
            sigma, sigma_uncertainty, fwhm, fwhm_uncertainty, area, area_uncertainty
        ))

    # # Print Gaussian parameters in formatted table
    # print("\nGaussian Fit Parameters:")
    # print(f"{'Index':<5} {'Amplitude':<20} {'Mean':<20} {'Sigma':<20} {'FWHM':<20} {'Area':<20}")
    # print("-" * 100)
    # for i, params in enumerate(gaussian_params):
    #     amplitude = round(params[0], params[1], notation="drake")
    #     mean = round(params[2], params[3], notation="drake")
    #     sigma = round(params[4], params[5], notation="drake")
    #     fwhm = round(params[6], params[7], notation="drake")
    #     area = round(params[8], params[9], notation="drake")
    #     print(f"{i:<5} {amplitude:<20} {mean:<20} {sigma:<20} {fwhm:<20} {area:<20}")

    # Extract background parameters
    background_params = []
    if bg_type != 'None':
        for key in result.params:
            if 'bg_' in key:
                value = float(result.params[key].value)
                uncertainty = result.params[key].stderr or 0.0
                background_params.append((key, value, uncertainty))

    # Create smooth fit line
    x_data_line = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
    y_data_line = result.eval(x=x_data_line)

    fit_report = str(result.fit_report())

    return gaussian_params, background_params, x_data_line, y_data_line, fit_report
"#;

            // Compile the Python code into a module
            let module = PyModule::from_code_bound(py, code, "gaussian.py", "gaussian")?;

            let x_data = self.data.x.clone();
            let y_data = self.data.y.clone();
            let peak_markers = self.peak_markers.clone();
            let equal_sigma = self.fit_settings.equal_stdev;
            let free_position = self.fit_settings.free_position;
            let bin_width = self.fit_settings.bin_width;

            // Form the `background_params` dictionary
            let background_params = PyDict::new_bound(py);

            match self.background_model {
                BackgroundModel::Linear(ref params) => {
                    if let Some(BackgroundResult::Linear(ref fitter)) = &self.background_result {
                        // Use fitted values for slope and intercept and set `vary` to false
                        let fitted_slope = fitter
                            .paramaters
                            .slope
                            .value
                            .unwrap_or(fitter.paramaters.slope.initial_guess);
                        let fitted_intercept = fitter
                            .paramaters
                            .intercept
                            .value
                            .unwrap_or(fitter.paramaters.intercept.initial_guess);

                        background_params.set_item("bg_type", "linear")?;
                        background_params.set_item(
                            "slope",
                            (
                                "slope".to_string(),
                                params.slope.min,
                                params.slope.max,
                                fitted_slope,
                                false,
                            ),
                        )?;
                        background_params.set_item(
                            "intercept",
                            (
                                "intercept".to_string(),
                                params.intercept.min,
                                params.intercept.max,
                                fitted_intercept,
                                false,
                            ),
                        )?;
                    } else {
                        // Use the initial guesses and allow them to vary
                        background_params.set_item("bg_type", "linear")?;
                        background_params.set_item(
                            "slope",
                            (
                                "slope".to_string(),
                                params.slope.min,
                                params.slope.max,
                                params.slope.initial_guess,
                                params.slope.vary,
                            ),
                        )?;
                        background_params.set_item(
                            "intercept",
                            (
                                "intercept".to_string(),
                                params.intercept.min,
                                params.intercept.max,
                                params.intercept.initial_guess,
                                params.intercept.vary,
                            ),
                        )?;
                    }
                }
                BackgroundModel::Quadratic(ref params) => {
                    if let Some(BackgroundResult::Quadratic(ref fitter)) = &self.background_result {
                        // Use fitted values for a, b, and c, set `vary` to false
                        let fitted_a = fitter
                            .paramaters
                            .a
                            .value
                            .unwrap_or(fitter.paramaters.a.initial_guess);
                        let fitted_b = fitter
                            .paramaters
                            .b
                            .value
                            .unwrap_or(fitter.paramaters.b.initial_guess);
                        let fitted_c = fitter
                            .paramaters
                            .c
                            .value
                            .unwrap_or(fitter.paramaters.c.initial_guess);

                        background_params.set_item("bg_type", "quadratic")?;
                        background_params.set_item(
                            "a",
                            ("a".to_string(), params.a.min, params.a.max, fitted_a, false),
                        )?;
                        background_params.set_item(
                            "b",
                            ("b".to_string(), params.b.min, params.b.max, fitted_b, false),
                        )?;
                        background_params.set_item(
                            "c",
                            ("c".to_string(), params.c.min, params.c.max, fitted_c, false),
                        )?;
                    } else {
                        // Use the initial guesses and allow them to vary
                        background_params.set_item("bg_type", "quadratic")?;
                        background_params.set_item(
                            "a",
                            (
                                "a".to_string(),
                                params.a.min,
                                params.a.max,
                                params.a.initial_guess,
                                params.a.vary,
                            ),
                        )?;
                        background_params.set_item(
                            "b",
                            (
                                "b".to_string(),
                                params.b.min,
                                params.b.max,
                                params.b.initial_guess,
                                params.b.vary,
                            ),
                        )?;
                        background_params.set_item(
                            "c",
                            (
                                "c".to_string(),
                                params.c.min,
                                params.c.max,
                                params.c.initial_guess,
                                params.c.vary,
                            ),
                        )?;
                    }
                }
                BackgroundModel::Exponential(ref params) => {
                    if let Some(BackgroundResult::Exponential(ref fitter)) = &self.background_result
                    {
                        // Use fitted values for amplitude and decay, set `vary` to false
                        let fitted_amplitude = fitter
                            .paramaters
                            .amplitude
                            .value
                            .unwrap_or(fitter.paramaters.amplitude.initial_guess);
                        let fitted_decay = fitter
                            .paramaters
                            .decay
                            .value
                            .unwrap_or(fitter.paramaters.decay.initial_guess);

                        background_params.set_item("bg_type", "exponential")?;
                        background_params.set_item(
                            "amplitude",
                            (
                                "amplitude".to_string(),
                                params.amplitude.min,
                                params.amplitude.max,
                                fitted_amplitude,
                                false,
                            ),
                        )?;
                        background_params.set_item(
                            "decay",
                            (
                                "decay".to_string(),
                                params.decay.min,
                                params.decay.max,
                                fitted_decay,
                                false,
                            ),
                        )?;
                    } else {
                        // Use the initial guesses and allow them to vary
                        background_params.set_item("bg_type", "exponential")?;
                        background_params.set_item(
                            "amplitude",
                            (
                                "amplitude".to_string(),
                                params.amplitude.min,
                                params.amplitude.max,
                                params.amplitude.initial_guess,
                                params.amplitude.vary,
                            ),
                        )?;
                        background_params.set_item(
                            "decay",
                            (
                                "decay".to_string(),
                                params.decay.min,
                                params.decay.max,
                                params.decay.initial_guess,
                                params.decay.vary,
                            ),
                        )?;
                    }
                }
                BackgroundModel::PowerLaw(ref params) => {
                    if let Some(BackgroundResult::PowerLaw(ref fitter)) = &self.background_result {
                        // Use fitted values for amplitude and exponent, set `vary` to false
                        let fitted_amplitude = fitter
                            .paramaters
                            .amplitude
                            .value
                            .unwrap_or(fitter.paramaters.amplitude.initial_guess);
                        let fitted_exponent = fitter
                            .paramaters
                            .exponent
                            .value
                            .unwrap_or(fitter.paramaters.exponent.initial_guess);

                        background_params.set_item("bg_type", "powerlaw")?;
                        background_params.set_item(
                            "amplitude",
                            (
                                "amplitude".to_string(),
                                params.amplitude.min,
                                params.amplitude.max,
                                fitted_amplitude,
                                false,
                            ),
                        )?;
                        background_params.set_item(
                            "exponent",
                            (
                                "exponent".to_string(),
                                params.exponent.min,
                                params.exponent.max,
                                fitted_exponent,
                                false,
                            ),
                        )?;
                    } else {
                        // Use the initial guesses and allow them to vary
                        background_params.set_item("bg_type", "powerlaw")?;
                        background_params.set_item(
                            "amplitude",
                            (
                                "amplitude".to_string(),
                                params.amplitude.min,
                                params.amplitude.max,
                                params.amplitude.initial_guess,
                                params.amplitude.vary,
                            ),
                        )?;
                        background_params.set_item(
                            "exponent",
                            (
                                "exponent".to_string(),
                                params.exponent.min,
                                params.exponent.max,
                                params.exponent.initial_guess,
                                params.exponent.vary,
                            ),
                        )?;
                    }
                }
                BackgroundModel::None => {
                    background_params.set_item("bg_type", "none")?;
                }
            }

            log::info!("Fitting Gaussian model");

            let result = module.getattr("MultipleGaussianFit")?.call1((
                x_data.clone(),
                y_data,
                peak_markers,
                bin_width,
                equal_sigma,
                free_position,
                background_params,
            ))?;

            let gaussian_params =
                result
                    .get_item(0)?
                    .extract::<Vec<(f64, f64, f64, f64, f64, f64, f64, f64, f64, f64)>>()?;
            let background_params = result.get_item(1)?.extract::<Vec<(String, f64, f64)>>()?;
            let x_composition = result.get_item(2)?.extract::<Vec<f64>>()?;
            let y_composition = result.get_item(3)?.extract::<Vec<f64>>()?;
            let fit_report = result.get_item(4)?.extract::<String>()?;

            self.peak_markers.clear();

            for (amp, amp_err, mean, mean_err, sigma, sigma_err, fwhm, fwhm_err, area, area_err) in
                gaussian_params
            {
                log::info!("Amplitude: {:.3} ± {:.3}, Mean: {:.3} ± {:.3}, Sigma: {:.3} ± {:.3}, FWHM: {:.3} ± {:.3}, Area: {:.3} ± {:.3}", 
                            amp, amp_err, mean, mean_err, sigma, sigma_err, fwhm, fwhm_err, area, area_err);

                self.peak_markers.push(mean);

                // Create the GaussianParameters for each set of values
                let mut gaussian_param = GaussianParameters::new(
                    (amp, amp_err),
                    (mean, mean_err),
                    (sigma, sigma_err),
                    (fwhm, fwhm_err),
                    (area, area_err),
                );

                // Generate the fit points for this Gaussian, using 100 points (or as many as needed)
                gaussian_param.generate_fit_points(100);

                self.fit_result.push(gaussian_param);
            }

            if self.background_result.is_none() {
                let min_x = x_data.iter().cloned().fold(f64::INFINITY, f64::min);
                let max_x = x_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

                match self.background_model {
                    // Handle the Linear case
                    BackgroundModel::Linear(_) => {
                        let slope = background_params[0].1;
                        let slope_err = background_params[0].2;
                        let intercept = background_params[1].1;
                        let intercept_err = background_params[1].2;

                        let linear_fitter = LinearFitter::new_from_parameters(
                            (slope, slope_err),
                            (intercept, intercept_err),
                            min_x,
                            max_x,
                        );

                        self.background_result = Some(BackgroundResult::Linear(linear_fitter));
                    }

                    // Handle the Exponential case
                    BackgroundModel::Exponential(_) => {
                        let amplitude = background_params[0].1;
                        let amplitude_err = background_params[0].2;
                        let decay = background_params[1].1;
                        let decay_err = background_params[1].2;

                        let exponential_fitter = ExponentialFitter::new_from_parameters(
                            (amplitude, amplitude_err),
                            (decay, decay_err),
                            min_x,
                            max_x,
                        );

                        self.background_result =
                            Some(BackgroundResult::Exponential(exponential_fitter));
                    }

                    // Handle the Quadratic case (to be implemented similarly)
                    BackgroundModel::Quadratic(_) => {
                        let a = background_params[0].1;
                        let a_err = background_params[0].2;
                        let b = background_params[1].1;
                        let b_err = background_params[1].2;
                        let c = background_params[2].1;
                        let c_err = background_params[2].2;

                        let quadratic_fitter = QuadraticFitter::new_from_parameters(
                            (a, a_err),
                            (b, b_err),
                            (c, c_err),
                            min_x,
                            max_x,
                        );

                        self.background_result =
                            Some(BackgroundResult::Quadratic(quadratic_fitter));
                    }

                    // Handle the PowerLaw case (to be implemented similarly)
                    BackgroundModel::PowerLaw(_) => {
                        let amplitude = background_params[0].1;
                        let amplitude_err = background_params[0].2;
                        let exponent = background_params[1].1;
                        let exponent_err = background_params[1].2;

                        let powerlaw_fitter = PowerLawFitter::new_from_parameters(
                            (amplitude, amplitude_err),
                            (exponent, exponent_err),
                            min_x,
                            max_x,
                        );

                        self.background_result = Some(BackgroundResult::PowerLaw(powerlaw_fitter));
                    }

                    BackgroundModel::None => {}
                }
            }

            // Create the composition line
            let fit_points = x_composition
                .iter()
                .zip(y_composition.iter())
                .map(|(&x, &y)| [x, y])
                .collect();
            self.fit_points = fit_points;

            self.fit_report = fit_report;

            Ok(())
        })
    }

    pub fn fit_params_ui(&self, ui: &mut egui::Ui, skip_one: bool) {
        for (i, params) in self.fit_result.iter().enumerate() {
            if skip_one && i != 0 {
                ui.label("");
            }
            ui.label(format!("{}", i));
            params.params_ui(ui);

            if i == 0 {
                ui.menu_button("Fit Report", |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(self.fit_report.clone());
                        });
                    });
                });
            }

            ui.end_row();
        }
    }
}
