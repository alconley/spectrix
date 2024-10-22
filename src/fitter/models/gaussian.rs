use pyo3::{prelude::*, types::PyModule};
use crate::fitter::common::{Data, Parameter};
use crate::fitter::main_fitter::{BackgroundModel, BackgroundResult, FitModel};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GaussianParameters {
    pub amplitude: Parameter,
    pub mean: Parameter,
    pub sigma: Parameter,
    pub fwhm: Parameter,
    pub area: Parameter,
    pub fit_points: Vec<[f64; 2]>,  // Vector of (x, y) points representing the Gaussian curve
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
    pub fit_result: Option<Vec<GaussianParameters>>,
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
            fit_result: None,
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

def gaussian(x, amplitude, mean, sigma):
    return amplitude * np.exp(-(x - mean)**2 / (2 * sigma**2))

def MultipleGaussianFit(x_data: list, y_data: list, peak_markers: list, 
                        equal_sigma: bool = True, free_position: bool = True,
                        bg_type: str = 'linear', slope: list = ("slope", -np.inf, np.inf, 0.0, True), intercept = ("intercept", -np.inf, np.inf, 0.0, True),
                        a: list = ("a", -np.inf, np.inf, 0.0, True), b = ("b", -np.inf, np.inf, 0.0, True), c: list = ("a", -np.inf, np.inf, 0.0, True),
                        amplitude: list = ("amplitude", -np.inf, np.inf, 0.0, True), decay = ("decay", -np.inf, np.inf, 0.0, True)):

    # Initialize the model with or without a background based on the flag
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
        params = model.make_params(amplitude=amplitude[3], decay=decay[3])
        params['bg_amplitude'].set(min=amplitude[1], max=amplitude[2], value=amplitude[3], vary=amplitude[4])
        params['bg_decay'].set(min=decay[1], max=decay[2], value=decay[3], vary=decay[4])
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
        
    peak_markers = sorted(peak_markers) # sort the peak markers in ascending order
    
    # Function to find the local max amplitude near each peak marker
    def estimate_peak_amplitude(x_data, y_data, peak_marker, window=5):
        idx = np.abs(x_data - peak_marker).argmin()  # Find the index of the peak marker
        window_start = max(0, idx - window)
        window_end = min(len(x_data), idx + window)
        local_max = np.max(y_data[window_start:window_end])
        return local_max / 2  # Current logic divides by 2
    
    # Function to estimate sigma based on FWHM
    def estimate_sigma(x_data, y_data, peak_marker, window=10):
        idx = np.abs(x_data - peak_marker).argmin()  # Find the index of the peak marker
        window_start = max(0, idx - window)
        window_end = min(len(x_data), idx + window)
        
        local_max = np.max(y_data[window_start:window_end])  # Peak maximum
        half_max = local_max / 2.0  # Half maximum to find FWHM
        
        # Find the points where the data crosses half maximum
        left_idx = idx
        while left_idx > 0 and y_data[left_idx] > half_max:
            left_idx -= 1
        
        right_idx = idx
        while right_idx < len(x_data) - 1 and y_data[right_idx] > half_max:
            right_idx += 1
        
        # Calculate FWHM and then estimate sigma
        fwhm = x_data[right_idx] - x_data[left_idx]
        sigma_estimate = fwhm / 2.35482 / 2.35482  # Convert FWHM to sigma
        return sigma_estimate
    
    # estimated_amplitude = estimate_peak_amplitude(x_data, y_data, peak_markers[0])
    # estimated_sigma = estimate_sigma(x_data, y_data, peak_markers[0]) 
    
    estimated_amplitude = 1000
    estimated_sigma = 10

    params.update(first_gaussian.make_params(amplitude=estimated_amplitude, mean=peak_markers[0], sigma=estimated_sigma))
    params['g0_sigma'].set(min=0)  # Initial constraint for the first Gaussian's sigma
    params[f"g0_amplitude"].set(min=0)
    
    params.add(f'g0_fwhm', expr=f'2.35482 * g0_sigma')  # FWHM = 2 * sqrt(2 * ln(2)) * sigma
    params[f"g0_fwhm"].set(min=0)

    params.add(f'g0_area', expr=f'g0_amplitude * sqrt(2 * pi) * g0_sigma')  # Area under the Gaussian
    params[f"g0_area"].set(min=0)
    
    # Fix the center of the first Gaussian if free_position=False
    if not free_position:
        params['g0_mean'].set(vary=False)

    # Set min and max for the mean based on peak markers
    params['g0_mean'].set(min=x_data[0], max=peak_markers[1] if len(peak_markers) > 1 else x_data[-1])

    # Add additional Gaussians
    for i, peak in enumerate(peak_markers[1:], start=1):
        g = lmfit.Model(gaussian, prefix=f'g{i}_')
        model += g
        
        # Estimate amplitude for each peak
        estimated_amplitude = 1000

        params.update(g.make_params(amplitude=estimated_amplitude, mean=peak, sigma=10))
        
        # Set the mean min/max based on the current and next peak
        min_mean = peak_markers[i-1]
        max_mean = peak_markers[i+1] if i + 1 < len(peak_markers) else x_data[-1]
        params[f'g{i}_mean'].set(min=min_mean, max=max_mean)
        
        params.add(f'g{i}_fwhm', expr=f'2.35482 * g{i}_sigma')  # FWHM = 2 * sqrt(2 * ln(2)) * sigma
        params[f"g{i}_fwhm"].set(min=0)

        
        params.add(f'g{i}_area', expr=f'g{i}_amplitude * sqrt(2 * pi) * g{i}_sigma')  # Area under the Gaussian
        params[f"g{i}_area"].set(min=0)


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

    print("\nInitial Parameter Guesses:")
    params.pretty_print()

    print("\nFit Report:")
    print(result.fit_report())

    # Create a list of native Python float tuples for Gaussian parameters
    gaussian_params = []
    for i in range(len(peak_markers)):
        
        amplitude = float(result.params[f'g{i}_amplitude'].value)      # Convert the value to native Python float
        amplitude_uncertainty = result.params[f'g{i}_amplitude'].stderr      # Convert the uncertainty to native Python float
        if amplitude_uncertainty is None:
            amplitude_uncertainty = float(0.0)
        else:
            amplitude_uncertainty = float(amplitude_uncertainty)
            
        mean = float(result.params[f'g{i}_mean'].value)
        mean_uncertainty = result.params[f'g{i}_mean'].stderr
        if mean_uncertainty is None:
            mean_uncertainty = float(0.0)
        else:
            mean_uncertainty = float(mean_uncertainty)
            
        sigma = float(result.params[f'g{i}_sigma'].value)
        sigma_uncertainty = result.params[f'g{i}_sigma'].stderr
        if sigma_uncertainty is None:
            sigma_uncertainty = float(0.0)
        else:
            sigma_uncertainty = float(sigma_uncertainty)
            
        fwhm = float(result.params[f'g{i}_fwhm'].value)
        fwhm_uncertainty = result.params[f'g{i}_fwhm'].stderr
        if fwhm_uncertainty is None:
            fwhm_uncertainty = float(0.0)
        else:
            fwhm_uncertainty = float(fwhm_uncertainty)
            
        area = float(result.params[f'g{i}_area'].value)
        area_uncertainty = result.params[f'g{i}_area'].stderr
        if area_uncertainty is None:
            area_uncertainty = float(0.0)
        else:
            area_uncertainty = float(area_uncertainty)
        
        
        gaussian_params.append((
            amplitude,
            amplitude_uncertainty,
            mean,
            mean_uncertainty,
            sigma,
            sigma_uncertainty,
            fwhm,
            fwhm_uncertainty,
            area,
            area_uncertainty
        ))
    
    # Create a list of native Python float tuples for Background parameters
    background_params = []
    if bg_type != 'None':
        for key in result.params:
            if 'bg_' in key:
                value = float(result.params[key].value)      # Convert the value to native Python float
                uncertainty=result.params[key].stderr      # Convert the uncertainty to native Python float
                if uncertainty is None:
                    uncertainty = float(0.0)
                else:
                    uncertainty = float(uncertainty)
                
                background_params.append((
                    key,  # Keep the parameter name
                    value,      # Convert the value to native Python float
                    uncertainty      # Convert the uncertainty to native Python float
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
            let bg_type = "linear";

            // let result = if let BackgroundModel::Linear(params) = &mut self.background_model {
            //     let bg_type = "linear";
            //     let slope =if params.slope.value.is_none() {
            //         (params.slope.name.clone(), params.slope.min, params.slope.max, params.slope.initial_guess, params.slope.vary);
            //     };

            //     let intercept =if params.intercept.value.is_none() {
            //         (params.intercept.name.clone(), params.intercept.min, params.intercept.max, params.intercept.initial_guess, params.intercept.vary);
            //     };

            //     module.getattr("MultipleGaussianFit")?.call1((
            //         x_data,
            //         y_data,
            //         peak_markers,
            //         equal_sigma,
            //         free_position,
            //         bg_type,
            //     ))?
            // } else {
            //     return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            //         "Unsupported background model",
            //     ));
            // };

            log::info!("Fitting Gaussian model");

            let result = module.getattr("MultipleGaussianFit")?.call1((
                x_data,
                y_data,
                peak_markers,
                equal_sigma,
                free_position,
                bg_type,
            ))?;

            let gaussian_params = result.get_item(0)?.extract::<Vec<(f64, f64, f64, f64, f64, f64, f64, f64, f64, f64)>>()?;
            let background_params = result.get_item(1)?.extract::<Vec<(String, f64, f64)>>()?;
            let x_composition = result.get_item(2)?.extract::<Vec<f64>>()?;
            let y_composition = result.get_item(3)?.extract::<Vec<f64>>()?;
            let fit_report = result.get_item(4)?.extract::<String>()?;

            for (amp, amp_err, mean, mean_err, sigma, sigma_err, fwhm, fwhm_err, area, area_err) in gaussian_params {
                log::info!("Amplitude: {:.3} ± {:.3}, Mean: {:.3} ± {:.3}, Sigma: {:.3} ± {:.3}, FWHM: {:.3} ± {:.3}, Area: {:.3} ± {:.3}", 
                            amp, amp_err, mean, mean_err, sigma, sigma_err, fwhm, fwhm_err, area, area_err);

                // // Create a GaussianParameters struct
                // let amplitude = Value {
                //     name: "Amplitude".to_string(),
                //     value: amp,
                //     uncertainity: amp_err,
                // };

                // let mean = Value {
                //     name: "Mean".to_string(),
                //     value: mean,
                //     uncertainity: mean_err,
                // };

                // let sigma = Value {
                //     name: "Sigma".to_string(),
                //     value: sigma,
                //     uncertainity: sigma_err,
                // };

                // let gaussian = GaussianParameters::new(amplitude, mean, sigma, self.fit_settings.bin_width);

                // if let Some(fit_result) = &mut self.fit_result {
                //     fit_result.push(gaussian);
                // } else {
                //     self.fit_result = Some(vec![gaussian]);
                // }
            }

            // Process background fit results
            // let mut background_values = Vec::new();
            // for (name, value, uncertainity) in background_params {
            //     background_values.push(Value::new(name, value, uncertainity));
            // }

            // Create the composition line
            let fit_points = x_composition.iter().zip(y_composition.iter()).map(|(&x, &y)| [x, y]).collect();
            self.fit_points = fit_points;

            self.fit_report = fit_report;

            Ok(())
        })
    }
    
}