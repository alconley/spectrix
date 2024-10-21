use pyo3::{prelude::*, types::PyModule};
use crate::fitter::common::{Data, Value};
use crate::fitter::main_fitter::Model;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Background {
    pub model: Model,
    pub parameters: Option<Vec<Value>>,
    pub varying: Option<Vec<bool>>,
    pub fit_points: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GaussianParameters {
    pub amplitude: Value,
    pub mean: Value,
    pub sigma: Value,
    pub fwhm: Value,
    pub area: Value,
    pub fit_points: Vec<[f64; 2]>,  // Vector of (x, y) points representing the Gaussian curve
}

impl GaussianParameters {
    pub fn new(
        amplitude: Value,
        mean: Value,
        sigma: Value,
        bin_width: f64,
    ) -> Self {
        // Calculate Full Width at Half Maximum (FWHM)
        let fwhm = Value {
            name: "FWHM".to_string(),
            value: 2.0 * sigma.value * (2.0_f64.ln().sqrt()),
            uncertainity: 2.0 * sigma.uncertainity * (2.0_f64.ln().sqrt()),
        };

        // Calculate the area under the Gaussian curve
        let area = Value {
            name: "Area".to_string(),
            value: amplitude.value * sigma.value * (2.0 * std::f64::consts::PI).sqrt() / bin_width,
            uncertainity: amplitude.uncertainity * sigma.value * (2.0 * std::f64::consts::PI).sqrt() / bin_width,
        };

        // Generate fit points for the Gaussian curve
        let mut fit_points = Vec::new();
        let num_points = 100; // Number of points to generate for the curve
        let start_x = mean.value - 4.0 * sigma.value;  // Start at mean - 4 * sigma
        let end_x = mean.value + 4.0 * sigma.value;    // End at mean + 4 * sigma
        let step = (end_x - start_x) / num_points as f64;

        for i in 0..=num_points {
            let x = start_x + i as f64 * step;
            let y = amplitude.value * (-((x - mean.value).powi(2)) / (2.0 * sigma.value.powi(2))).exp();
            fit_points.push([x, y]);
        }

        // Return the constructed GaussianParameters
        Self {
            amplitude,
            mean,
            sigma,
            fwhm,
            area,
            fit_points,
        }
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
    pub background_model: Model,
    pub background_parameters: Option<Background>,  // Adjusted to accept Background struct
    pub fit_result: Option<Vec<GaussianParameters>>,
    pub composition_points: Vec<[f64; 2]>,
}

impl GaussianFitter {
    pub fn new(
        data: Data,
        peak_markers: Vec<f64>,
        background_model: Model,
        background_parameters: Option<Background>,  // Accept Background struct
        equal_stdev: bool,
        free_position: bool,
        bin_width: f64,
    ) -> Self {
        Self {
            data,
            peak_markers,
            background_model,
            background_parameters,
            fit_settings: GaussianFitSettings {
                equal_stdev,
                free_position,
                bin_width,
            },
            fit_result: None,
            composition_points: Vec::new(),
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
import lmfit
import numpy as np

def gaussian(x, amplitude, mean, sigma):
    return amplitude * np.exp(-(x - mean)**2 / (2 * sigma**2))

def MultipleGaussianFit(x_data: list, y_data: list, peak_markers: list, 
                        equal_sigma: bool = True, free_position: bool = True,
                        bg_type: str = 'linear', bg_initial_guesses: list = (0, 0), bg_vary: list = (True, True)):
    
    # Initialize the model with or without a background based on the flag
    if bg_type == 'linear':
        model = lmfit.models.LinearModel(prefix='bg_')
        params = model.make_params(slope=bg_initial_guesses[0], intercept=bg_initial_guesses[1])
        params['bg_slope'].set(vary=bg_vary[0])
        params['bg_intercept'].set(vary=bg_vary[1])
    elif bg_type == 'quadratic':
        model = lmfit.models.QuadraticModel(prefix='bg_')
        params = model.make_params(a=bg_initial_guesses[0], b=bg_initial_guesses[1], c=bg_initial_guesses[2])
        params['bg_a'].set(vary=bg_vary[0])
        params['bg_b'].set(vary=bg_vary[1])
        params['bg_c'].set(vary=bg_vary[2])
    elif bg_type == 'exponential':
        model = lmfit.models.ExponentialModel(prefix='bg_')
        params = model.make_params(amplitude=bg_initial_guesses[0], decay=bg_initial_guesses[1])
        params['bg_amplitude'].set(vary=bg_vary[0])
        params['bg_decay'].set(vary=bg_vary[1])
    elif bg_type == 'powerlaw':
        model = lmfit.models.PowerLawModel(prefix='bg_')
        params = model.make_params(amplitude=bg_initial_guesses[0], exponent=bg_initial_guesses[1])
        params['bg_amplitude'].set(vary=bg_vary[0])
        params['bg_exponent'].set(vary=bg_vary[1])
    elif bg_type is None:
        model = None
        params = lmfit.Parameters()
    else:
        raise ValueError("Unsupported background model")

    first_gaussian = lmfit.Model(gaussian, prefix=f'g0_')
    
    if model is None:
        model = first_gaussian
    else:
        model += first_gaussian
        
    params.update(first_gaussian.make_params(amplitude=1, mean=peak_markers[0], sigma=1))
    params['g0_sigma'].set(min=0)  # Initial constraint for the first Gaussian's sigma
    
    # Fix the center of the first Gaussian if free_position=False
    if not free_position:
        params['g0_mean'].set(vary=False)

    # Add additional Gaussians
    for i, peak in enumerate(peak_markers[1:], start=1):
        g = lmfit.Model(gaussian, prefix=f'g{i}_')
        model += g
        params.update(g.make_params(amplitude=1, mean=peak))

        # Set the sigma parameter depending on whether equal_sigma is True or False
        if equal_sigma:
            params[f'g{i}_sigma'].set(expr='g0_sigma')  # Constrain sigma to be the same as g1_sigma
        else:
            params[f'g{i}_sigma'].set(min=0)  # Allow different sigmas for each Gaussian

        params[f'g{i}_amplitude'].set(min=0)

        # Fix the center of the Gaussian if free_position=False
        if not free_position:
            params[f'g{i}_mean'].set(vary=False)

    # Fit the model to the data
    result = model.fit(y_data, params, x=x_data)

    print("\nFit Report:")
    print(result.fit_report())

    # Create a list of native Python float tuples for Gaussian parameters
    gaussian_params = []
    for i in range(len(peak_markers)):
        gaussian_params.append((
            float(result.params[f'g{i}_amplitude'].value),
            float(result.params[f'g{i}_amplitude'].stderr),
            float(result.params[f'g{i}_mean'].value),
            float(result.params[f'g{i}_mean'].stderr),
            float(result.params[f'g{i}_sigma'].value),
            float(result.params[f'g{i}_sigma'].stderr)
        ))

    # Create a list of native Python float tuples for Background parameters
    background_params = []
    if bg_type != 'None':
        for key in result.params:
            if 'bg_' in key:
                background_params.append((
                    key,  # Keep the parameter name
                    float(result.params[key].value),      # Convert the value to native Python float
                    float(result.params[key].stderr)      # Convert the uncertainty to native Python float
                ))

    # Create smooth fit line with plenty of data points
    x_data_line = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
    y_data_line = result.eval(x=x_data_line)

    fit_report = str(result.fit_report())

    return gaussian_params, background_params, x_data_line, y_data_line, fit_report
"#;

            // Compile the Python code into a module
            let module =
                PyModule::from_code_bound(py, code, "gaussian.py", "gaussian")?;

            let x_data = self.data.x.clone();
            let y_data = self.data.y.clone();
            let peak_markers = self.peak_markers.clone();
            let equal_sigma = self.fit_settings.equal_stdev;
            let free_position = self.fit_settings.free_position;

            // Handle the background model and force parameters if provided
            let (bg_type, bg_initial_guesses, bg_vary) = match &self.background_parameters {
                Some(background) => {
                    match &background.model {
                        Model::Linear => (
                            "linear",
                            vec![
                                background.parameters.as_ref().unwrap()[0].value,
                                background.parameters.as_ref().unwrap()[1].value,
                            ],
                            vec![
                                background.varying.as_ref().unwrap()[0],
                                background.varying.as_ref().unwrap()[1],
                            ],
                        ),
                        _ => ("None", vec![0.0], vec![false]), // Handle other background models if necessary
                    }
                }
                None => ("None", vec![0.0], vec![false]),
            };

            // Call the Python function for Gaussian fitting
            let result = module.getattr("MultipleGaussianFit")?.call1((
                x_data,
                y_data,
                peak_markers,
                equal_sigma,
                free_position,
                bg_type,
                bg_initial_guesses,
                bg_vary,
            ))?;

            let gaussian_params = result.get_item(0)?.extract::<Vec<(f64, f64, f64, f64, f64, f64)>>()?;
            let background_params = result.get_item(1)?.extract::<Vec<(String, f64, f64)>>()?;
            let x_composition = result.get_item(2)?.extract::<Vec<f64>>()?;
            let y_composition = result.get_item(3)?.extract::<Vec<f64>>()?;
            let fit_report = result.get_item(4)?.extract::<String>()?;

            for (amp, amp_err, mean, mean_err, sigma, sigma_err) in gaussian_params {
                log::info!("Amplitude: {:.3} ± {:.3}, Mean: {:.3} ± {:.3}, Sigma: {:.3} ± {:.3}", amp, amp_err, mean, mean_err, sigma, sigma_err);

                // Create a GaussianParameters struct
                let amplitude = Value {
                    name: "Amplitude".to_string(),
                    value: amp,
                    uncertainity: amp_err,
                };

                let mean = Value {
                    name: "Mean".to_string(),
                    value: mean,
                    uncertainity: mean_err,
                };

                let sigma = Value {
                    name: "Sigma".to_string(),
                    value: sigma,
                    uncertainity: sigma_err,
                };

                let gaussian = GaussianParameters::new(amplitude, mean, sigma, self.fit_settings.bin_width);

                if let Some(fit_result) = &mut self.fit_result {
                    fit_result.push(gaussian);
                } else {
                    self.fit_result = Some(vec![gaussian]);
                }
            }

            // Process background fit results
            let mut background_values = Vec::new();
            for (name, value, uncertainity) in background_params {
                background_values.push(Value::new(name, value, uncertainity));
            }

            // Create the composition line
            let composition_points = x_composition.iter().zip(y_composition.iter()).map(|(&x, &y)| [x, y]).collect();
            self.composition_points = composition_points;

            Ok(())
        })
    }
    
}