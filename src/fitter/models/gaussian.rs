use crate::fitter::common::{Data, Parameter};
use crate::fitter::main_fitter::{BackgroundModel, BackgroundResult};
use crate::fitter::models::exponential::ExponentialFitter;
use crate::fitter::models::linear::LinearFitter;
use crate::fitter::models::powerlaw::PowerLawFitter;
use crate::fitter::models::quadratic::QuadraticFitter;

use crate::fitter::common::Calibration;

use pyo3::{
    ffi::c_str,
    prelude::*,
    types::{PyDict, PyModule},
};

use std::fs::File;
use std::io::Write as _;
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GaussianParameters {
    pub amplitude: Parameter,
    pub mean: Parameter,
    pub sigma: Parameter,
    pub fwhm: Parameter,
    pub area: Parameter,
    pub uuid: usize,
    pub energy: Parameter,
    pub fit_points: Vec<[f64; 2]>, // Vector of (x, y) points representing the Gaussian curve
}

impl Default for GaussianParameters {
    fn default() -> Self {
        Self {
            amplitude: Parameter {
                name: "amplitude".to_owned(),
                ..Default::default()
            },
            mean: Parameter {
                name: "mean".to_owned(),
                ..Default::default()
            },
            sigma: Parameter {
                name: "sigma".to_owned(),
                ..Default::default()
            },
            fwhm: Parameter {
                name: "fwhm".to_owned(),
                ..Default::default()
            },
            area: Parameter {
                name: "area".to_owned(),
                ..Default::default()
            },
            uuid: 0,
            energy: Parameter {
                name: "energy".to_owned(),
                vary: false,
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
        Self {
            amplitude: Parameter {
                name: "amplitude".to_owned(),
                value: Some(amp.0),
                uncertainty: Some(amp.1),
                ..Default::default()
            },
            mean: Parameter {
                name: "mean".to_owned(),
                value: Some(mean.0),
                uncertainty: Some(mean.1),
                ..Default::default()
            },
            sigma: Parameter {
                name: "sigma".to_owned(),
                value: Some(sigma.0),
                uncertainty: Some(sigma.1),
                ..Default::default()
            },
            fwhm: Parameter {
                name: "fwhm".to_owned(),
                value: Some(fwhm.0),
                uncertainty: Some(fwhm.1),
                ..Default::default()
            },
            area: Parameter {
                name: "area".to_owned(),
                value: Some(area.0),
                uncertainty: Some(area.1),
                ..Default::default()
            },
            uuid: 0,
            energy: Parameter {
                name: "energy".to_owned(),
                value: None,
                uncertainty: None,
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
                let y = self.amplitude.value.unwrap_or(1.0) / sigma
                    * (1.0 / (2.0 * std::f64::consts::PI).sqrt())
                    * (-((x - mean).powi(2)) / (2.0 * sigma.powi(2))).exp();
                self.fit_points.push([x, y]);
            }
        }
    }

    pub fn params_ui(&mut self, ui: &mut egui::Ui, calibrate: bool) {
        if calibrate {
            ui.label(format!(
                "{:.2} ± {:.2}",
                self.mean.calibrated_value.unwrap_or(0.0),
                self.mean.calibrated_uncertainty.unwrap_or(0.0)
            ));
            ui.label(format!(
                "{:.2} ± {:.2}",
                self.fwhm.calibrated_value.unwrap_or(0.0),
                self.fwhm.calibrated_uncertainty.unwrap_or(0.0)
            ));
            ui.label(format!(
                "{:.2} ± {:.2}",
                self.area.calibrated_value.unwrap_or(0.0),
                self.area.calibrated_uncertainty.unwrap_or(0.0)
            ));
            ui.label(format!(
                "{:.2} ± {:.2}",
                self.amplitude.calibrated_value.unwrap_or(0.0),
                self.amplitude.calibrated_uncertainty.unwrap_or(0.0)
            ));
            ui.label(format!(
                "{:.2} ± {:.2}",
                self.sigma.calibrated_value.unwrap_or(0.0),
                self.sigma.calibrated_uncertainty.unwrap_or(0.0)
            ));
        } else {
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
}

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GaussianFitSettings {
    pub equal_stdev: bool,
    pub free_position: bool,
}

#[derive(Default, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GaussianFitter {
    pub data: Data,
    pub region_markers: Vec<f64>,
    pub peak_markers: Vec<f64>,
    pub background_markers: Vec<(f64, f64)>,
    pub fit_settings: GaussianFitSettings,
    pub background_model: BackgroundModel,
    pub background_result: Option<BackgroundResult>,
    pub fit_result: Vec<GaussianParameters>,
    pub fit_points: Vec<[f64; 2]>,
    pub fit_report: String,
    pub lmfit_result: Option<String>,
}

impl GaussianFitter {
    #[expect(clippy::too_many_arguments)]
    pub fn new(
        data: Data,
        region_markers: Vec<f64>,
        peak_markers: Vec<f64>,
        background_markers: Vec<(f64, f64)>,
        background_model: BackgroundModel,
        background_result: Option<BackgroundResult>,
        equal_stdev: bool,
        free_position: bool,
    ) -> Self {
        Self {
            data,
            region_markers,
            peak_markers,
            background_markers,
            background_model,
            background_result,
            fit_settings: GaussianFitSettings {
                equal_stdev,
                free_position,
            },
            fit_result: Vec::new(),
            fit_points: Vec::new(),
            fit_report: String::new(),
            lmfit_result: None,
        }
    }

    pub fn get_calibration_data(&self) -> Vec<(f64, f64, f64, f64)> {
        let mut calibration_data = Vec::new();

        for params in &self.fit_result {
            if let (Some(energy), Some(energy_unc), Some(mean), Some(mean_unc)) = (
                params.energy.value,
                params.energy.uncertainty,
                params.mean.value,
                params.mean.uncertainty,
            )
                && energy != -1.0 {
                    calibration_data.push((mean, mean_unc, energy, energy_unc));
                }
        }

        calibration_data
    }

    pub fn calibrate(&mut self, calibration: &Calibration) {
        log::info!("Calibrating");
        // Calibration logic goes here

        // calibrate the parameters
        for param in &mut self.fit_result {
            // param.amplitude.calibrate(calibration);
            param.mean.calibrate_energy(calibration);
            param
                .sigma
                .calibrate_sigma(calibration, param.mean.value.unwrap_or(0.0));
            param
                .fwhm
                .calibrate_fwhm(calibration, param.mean.value.unwrap_or(0.0));

            param.energy.calibrated_value = param.energy.value;
            param.energy.calibrated_uncertainty = param.energy.uncertainty;

            param.amplitude.calibrated_value = param.amplitude.value;
            param.amplitude.calibrated_uncertainty = param.amplitude.uncertainty;

            param.area.calibrated_value = param.area.value;
            param.area.calibrated_uncertainty = param.area.uncertainty;
        }

        // add calibration to result file
        if let Err(e) = self.add_calibration_to_result(calibration) {
            log::error!("Failed to add calibration to lmfit result: {e:?}");
        }
    }

    pub fn lmfit(&mut self, load_result_path: Option<PathBuf>) -> PyResult<()> {
        Python::with_gil(|py| {
            // let sys = py.import("sys")?;
            // let version: String = sys.getattr("version")?.extract()?;
            // let executable: String = sys.getattr("executable")?.extract()?;
            // println!("Using Python version: {}", version);
            // println!("Python executable: {}", executable);

            // Check if the `uproot` module can be imported
            if py.import("lmfit").is_ok() {
                // println!("Successfully imported `lmfit` module.");
            } else {
                eprintln!(
                    "Error: `lmfit` module could not be found. Make sure you are using the correct Python environment with `lmfit` installed."
                );
                return Err(PyErr::new::<pyo3::exceptions::PyImportError, _>(
                    "`lmfit` module not available",
                ));
            }

            // Define the Python code as a module
            let code = c_str!("
import numpy as np
import lmfit
from lmfit.model import load_modelresult, save_modelresult

def GaussianFit(counts: list, centers: list,
                region_markers: list, peak_markers: list = [], background_markers: list = [],
                equal_sigma: bool = True, free_position: bool = True,                 
                background_params: dict = {'bg_type': 'linear', 
                                            'slope': (0, -np.inf, np.inf, 1.0, True), 
                                            'intercept': (0, -np.inf, np.inf, 0.0, True),
                                            'a': (0, -np.inf, np.inf, 0.0, True),
                                            'b': (0, -np.inf, np.inf, 1.0, True),
                                            'c': (0, -np.inf, np.inf, 0.0, True),
                                            'amplitude': (0, -np.inf, np.inf, 1.0, True),
                                            'decay': (0, -np.inf, np.inf, 1.0, True),
                                            'exponent': (0, -np.inf, np.inf, 1.0, True)
                                            }):

    # ensure the edges is the same length as counts + 1
    if len(centers) != len(counts):
        raise ValueError('The length of edges must be one more than the length of counts.')
    
    centers = np.array(centers)
    counts = np.array(counts)
    
    bin_width = centers[1] - centers[0]
    
    # Ensure there are 2 region markers
    if len(region_markers) != 2:
        raise ValueError('Region markers must have exactly 2 values.')
    
    # sort the region markers
    region_markers = sorted(region_markers)

    # ensure there are only 2 region markers
    if len(region_markers) != 2:
        raise ValueError('Region markers must have exactly 2 values.')
    
    # Extract fitting region
    region_mask = (centers >= region_markers[0]) & (centers <= region_markers[1])

    x_data = centers[region_mask]
    y_data = counts[region_mask]

    # If there is not a peak marker, set it to the max bin value in the region
    if len(peak_markers) == 0:
        # find the bin with the max value in the region
        max_bin_idx = np.argmax(y_data)
        peak_markers = [x_data[max_bin_idx]]

    # sort the peak markers
    peak_markers = sorted(peak_markers)

    # Remove any peak markers that are outside the region
    peak_markers = [peak for peak in peak_markers if peak >= region_markers[0] and peak <= region_markers[1]]

    bg_type = background_params.get('bg_type', 'linear')
    
    if bg_type == 'linear':
        bg_model = lmfit.models.LinearModel(prefix='bg_')
        params = bg_model.make_params(slope=background_params['slope'][3], intercept=background_params['intercept'][3])
        params['bg_slope'].set(vary=background_params['slope'][4])
        params['bg_intercept'].set(vary=background_params['intercept'][4])
    elif bg_type == 'quadratic':
        bg_model = lmfit.models.QuadraticModel(prefix='bg_')
        params = bg_model.make_params(a=background_params['a'][3], b=background_params['b'][3], c=background_params['c'][3])
        params['bg_a'].set(vary=background_params['a'][4])
        params['bg_b'].set(vary=background_params['b'][4])
        params['bg_c'].set(vary=background_params['c'][4])
    elif bg_type == 'exponential':
        bg_model = lmfit.models.ExponentialModel(prefix='bg_')
        params = bg_model.make_params(amplitude=background_params['amplitude'][3], decay=background_params['decay'][3])
        params['bg_amplitude'].set(vary=background_params['amplitude'][4])
        params['bg_decay'].set(vary=background_params['decay'][4])
    elif bg_type == 'powerlaw':
        bg_model = lmfit.models.PowerLawModel(prefix='bg_')
        params = bg_model.make_params(amplitude=background_params['amplitude'][3], exponent=background_params['exponent'][3])
        params['bg_amplitude'].set(vary=background_params['amplitude'][4])
        params['bg_exponent'].set(vary=background_params['exponent'][4])
    elif bg_type == 'None':
        bg_model = lmfit.models.ConstantModel(prefix='bg_')
        params = bg_model.make_params(c=0)
        params['bg_c'].set(vary=False)
    else:
        raise ValueError('Unsupported background model')
    
    # Fit the background model to the data of the background markers before fitting the peaks
    if len(background_markers) == 0:
        # put marker at the start and end of the region
        background_markers = [(region_markers[0]-bin_width, region_markers[0]), (region_markers[1], region_markers[1]+bin_width)]

    bg_x = []
    bg_y = []
    for bg_start, bg_end in background_markers:
        # sort the background markers
        bg_start, bg_end = sorted([bg_start, bg_end])

        bg_mask = (centers >= bg_start) & (centers <= bg_end)
        bg_x.extend(centers[bg_mask])
        bg_y.extend(counts[bg_mask])

    bg_x = np.array(bg_x)
    bg_y = np.array(bg_y)
    
    bg_result = bg_model.fit(bg_y, params, x=bg_x)

    # print intial parameter guesses
    print('Initial Background Parameter Guesses:')
    params.pretty_print()

    # print fit report
    print('Background Fit Report:')
    print(bg_result.fit_report())

    # **Adjust background parameters based on their errors**
    for param in bg_result.params:
        params[param].set(value=bg_result.params[param].value, vary=False)

    # Add background model to overall model
    model = bg_model

    # Estimate sigma
    # **Find the peak marker with the highest bin count**
    peak_max_idx = np.argmax([y_data[np.abs(x_data - peak).argmin()] for peak in peak_markers])
    peak_with_max_count = peak_markers[peak_max_idx]

    # **Estimate sigma using FWHM method**
    def estimate_sigma(x_data, y_data, peak):
        peak_idx = np.abs(x_data - peak).argmin()
        peak_height = y_data[peak_idx]
        half_max = peak_height / 2

        # Find indices where y is closest to half the peak height
        left_idx = np.where(y_data[:peak_idx] <= half_max)[0]
        right_idx = np.where(y_data[peak_idx:] <= half_max)[0] + peak_idx

        if len(left_idx) == 0 or len(right_idx) == 0:
            return (x_data[1] - x_data[0]) * 2  # Fallback: Use bin width * 2

        left_fwhm = x_data[left_idx[-1]]
        right_fwhm = x_data[right_idx[0]]

        fwhm = right_fwhm - left_fwhm
        return max(fwhm / 2.3548, (x_data[1] - x_data[0]) * 2)  # Convert FWHM to sigma

    # **Get the estimated sigma from the strongest peak**
    estimated_sigma = estimate_sigma(x_data, y_data, peak_with_max_count)

    # **Estimate Amplitude for Each Peak**
    estimated_amplitude = []
    for peak in peak_markers:
        # Find closest bin index
        closest_idx = np.abs(x_data - peak).argmin()
        height = y_data[closest_idx]

        if bg_result is not None:
            # Estimate background contribution at this point if there is an background model
            bg_at_peak = bg_result.eval(x=peak)
        else:
            bg_at_peak = 0
        
        # Subtract background to get height
        adjusted_height = height - bg_at_peak
        estimated_amplitude.append(adjusted_height * estimated_sigma/ 0.3989423)

    # Add Gaussian peaks
    peak_markers = sorted(peak_markers)
    for i, peak in enumerate(peak_markers):
        # g = lmfit.Model(gaussian, prefix=f'g{i}_')
        g = lmfit.models.GaussianModel(prefix=f'g{i}_')
        model += g

        params.update(g.make_params(amplitude=estimated_amplitude[i], mean=peak, sigma=estimated_sigma))

        if equal_sigma and i > 0:
            params[f'g{i}_sigma'].set(expr='g0_sigma')
        else:
            params[f'g{i}_sigma'].set(min=0)

        params.add(f'g{i}_area', expr=f'g{i}_amplitude / {bin_width}')
        params[f'g{i}_area'].set(min=0)  # Use estimated area

        if not free_position:
            params[f'g{i}_center'].set(vary=False)

        if len(peak_markers) == 1:
            params[f'g{i}_center'].set(value=peak, min=region_markers[0], max=region_markers[1])  
        else:
            # Default to using neighboring peaks
            prev_peak = region_markers[0] if i == 0 else peak_markers[i - 1]
            next_peak = region_markers[1] if i == len(peak_markers) - 1 else peak_markers[i + 1]

            # Calculate distance to previous and next peaks
            prev_dist = abs(peak - prev_peak)
            next_dist = abs(peak - next_peak)

            # Adjust min/max based 1 sigma of peak
            sigma_range = 1

            min_val = prev_peak if prev_dist <= sigma_range * estimated_sigma else peak - sigma_range * estimated_sigma
            max_val = next_peak if next_dist <= sigma_range * estimated_sigma else peak + sigma_range * estimated_sigma

            # Ensure bounds are within the region
            min_val = max(region_markers[0], min_val)
            max_val = min(region_markers[1], max_val)

            params[f'g{i}_center'].set(value=peak, min=min_val, max=max_val)

    # Fit the model to the data
    result = model.fit(y_data, params, x=x_data)

    # Print initial parameter guesses
    print('Initial Parameter Guesses:')
    params.pretty_print()

    # Print fit report
    print('Fit Report:')

    fit_report = result.fit_report()
    print(fit_report)

    # Extract Gaussian and background parameters
    gaussian_params = []
    additional_params = []
    for i in range(len(peak_markers)):
        amplitude = float(result.params[f'g{i}_amplitude'].value)
        amplitude_uncertainty = result.params[f'g{i}_amplitude'].stderr or 0.0
        mean = float(result.params[f'g{i}_center'].value)
        mean_uncertainty = result.params[f'g{i}_center'].stderr or 0.0
        sigma = float(result.params[f'g{i}_sigma'].value)
        sigma_uncertainty = result.params[f'g{i}_sigma'].stderr or 0.0
        fwhm = float(result.params[f'g{i}_fwhm'].value)
        fwhm_uncertainty = result.params[f'g{i}_fwhm'].stderr or 0.0
        area = float(result.params[f'g{i}_area'].value)
        area_uncertainty = result.params[f'g{i}_area'].stderr or 0.0

        # default
        uuid = 0
        energy = -1.0
        energy_uncertainty = 0.0

        gaussian_params.append((
            amplitude, amplitude_uncertainty, mean, mean_uncertainty,
            sigma, sigma_uncertainty, fwhm, fwhm_uncertainty, area, area_uncertainty, uuid
        ))

        additional_params.append((
            energy, energy_uncertainty,
        ))

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

    # save the fit result to a temp file
    save_modelresult(result, 'temp_fit.sav')

    return gaussian_params, background_params, x_data_line, y_data_line, fit_report, additional_params

def load_result(filename: str):
    result = load_modelresult(filename)

    params = result.params

    peak_markers = []
    for key in params:
        if 'g' in key and '_center' in key:
            peak_markers.append(params[key].value)

    x_min = result.userkws['x'].min()
    x_max = result.userkws['x'].max()
    x_data = np.linspace(x_min, x_max, 1000)

    # Print initial parameter guesses
    print('Initial Parameter Guesses:')
    params.pretty_print()

    # Print fit report
    print('Fit Report:')

    fit_report = result.fit_report()
    print(fit_report)

    # Extract Gaussian and background parameters
    gaussian_params = []
    additional_params = []
    for i in range(len(peak_markers)):
        keys = [f'g{i}_amplitude', f'g{i}_center', f'g{i}_sigma', f'g{i}_fwhm', f'g{i}_area']
        if not all(k in result.params for k in keys):
            print(f'Skipping peak g{i} due to missing parameters.')
            continue

        amplitude = float(result.params[f'g{i}_amplitude'].value)
        amplitude_uncertainty = result.params[f'g{i}_amplitude'].stderr or 0.0
        mean = float(result.params[f'g{i}_center'].value)
        mean_uncertainty = result.params[f'g{i}_center'].stderr or 0.0
        sigma = float(result.params[f'g{i}_sigma'].value)
        sigma_uncertainty = result.params[f'g{i}_sigma'].stderr or 0.0
        fwhm = float(result.params[f'g{i}_fwhm'].value)
        fwhm_uncertainty = result.params[f'g{i}_fwhm'].stderr or 0.0
        area = float(result.params[f'g{i}_area'].value)
        area_uncertainty = result.params[f'g{i}_area'].stderr or 0.0

        uuid = result.params.get(f'g{i}_uuid', 0)

        # check if energy parameter exists
        if f'g{i}_energy' in result.params:
            energy = float(result.params[f'g{i}_energy'].value)
        else:
            energy = -1.0  # Default value if not present

        # check if energy uncertainty parameter exists
        if f'g{i}_energy_uncertainty' in result.params:
            energy_uncertainty = result.params[f'g{i}_energy_uncertainty'].value
        else:
            energy_uncertainty = 0.0  # Default value if not present

        gaussian_params.append((
            amplitude, amplitude_uncertainty, mean, mean_uncertainty,
            sigma, sigma_uncertainty, fwhm, fwhm_uncertainty, area, area_uncertainty, uuid
        ))

        additional_params.append((
            energy, energy_uncertainty,
        ))

        # Extract background parameters
        background_params = []
        # if bg_type != 'None':
        for key in result.params:
            if 'bg_' in key:
                value = float(result.params[key].value)
                uncertainty = result.params[key].stderr or 0.0
                background_params.append((key, value, uncertainty))

        # Create smooth fit line
        x_data_line = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
        y_data_line = result.eval(x=x_data_line)

    # save the fit result to a temp file
    save_modelresult(result, 'temp_fit.sav')

    return gaussian_params, background_params, x_data_line, y_data_line, fit_report, additional_params

");

            // Compile the Python code into a module
            let module = PyModule::from_code(py, code, c_str!("gaussian.py"), c_str!("gaussian"))?;

            let y_data = self.data.y.clone();
            let x_data = self.data.x.clone();
            let region_markers = self.region_markers.clone();
            let peak_markers = self.peak_markers.clone();
            let background_markers = self.background_markers.clone();
            let equal_sigma = self.fit_settings.equal_stdev;
            let free_position = self.fit_settings.free_position;

            // Form the `background_params` dictionary
            let background_params = PyDict::new(py);

            match self.background_model {
                BackgroundModel::Linear(ref params) => {
                    if let Some(BackgroundResult::Linear(fitter)) = &self.background_result {
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
                                "slope".to_owned(),
                                params.slope.min,
                                params.slope.max,
                                fitted_slope,
                                false,
                            ),
                        )?;
                        background_params.set_item(
                            "intercept",
                            (
                                "intercept".to_owned(),
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
                                "slope".to_owned(),
                                params.slope.min,
                                params.slope.max,
                                params.slope.initial_guess,
                                params.slope.vary,
                            ),
                        )?;
                        background_params.set_item(
                            "intercept",
                            (
                                "intercept".to_owned(),
                                params.intercept.min,
                                params.intercept.max,
                                params.intercept.initial_guess,
                                params.intercept.vary,
                            ),
                        )?;
                    }
                }
                BackgroundModel::Quadratic(ref params) => {
                    if let Some(BackgroundResult::Quadratic(fitter)) = &self.background_result {
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
                            ("a".to_owned(), params.a.min, params.a.max, fitted_a, false),
                        )?;
                        background_params.set_item(
                            "b",
                            ("b".to_owned(), params.b.min, params.b.max, fitted_b, false),
                        )?;
                        background_params.set_item(
                            "c",
                            ("c".to_owned(), params.c.min, params.c.max, fitted_c, false),
                        )?;
                    } else {
                        // Use the initial guesses and allow them to vary
                        background_params.set_item("bg_type", "quadratic")?;
                        background_params.set_item(
                            "a",
                            (
                                "a".to_owned(),
                                params.a.min,
                                params.a.max,
                                params.a.initial_guess,
                                params.a.vary,
                            ),
                        )?;
                        background_params.set_item(
                            "b",
                            (
                                "b".to_owned(),
                                params.b.min,
                                params.b.max,
                                params.b.initial_guess,
                                params.b.vary,
                            ),
                        )?;
                        background_params.set_item(
                            "c",
                            (
                                "c".to_owned(),
                                params.c.min,
                                params.c.max,
                                params.c.initial_guess,
                                params.c.vary,
                            ),
                        )?;
                    }
                }
                BackgroundModel::Exponential(ref params) => {
                    if let Some(BackgroundResult::Exponential(fitter)) = &self.background_result {
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
                                "amplitude".to_owned(),
                                params.amplitude.min,
                                params.amplitude.max,
                                fitted_amplitude,
                                false,
                            ),
                        )?;
                        background_params.set_item(
                            "decay",
                            (
                                "decay".to_owned(),
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
                                "amplitude".to_owned(),
                                params.amplitude.min,
                                params.amplitude.max,
                                params.amplitude.initial_guess,
                                params.amplitude.vary,
                            ),
                        )?;
                        background_params.set_item(
                            "decay",
                            (
                                "decay".to_owned(),
                                params.decay.min,
                                params.decay.max,
                                params.decay.initial_guess,
                                params.decay.vary,
                            ),
                        )?;
                    }
                }
                BackgroundModel::PowerLaw(ref params) => {
                    if let Some(BackgroundResult::PowerLaw(fitter)) = &self.background_result {
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
                                "amplitude".to_owned(),
                                params.amplitude.min,
                                params.amplitude.max,
                                fitted_amplitude,
                                false,
                            ),
                        )?;
                        background_params.set_item(
                            "exponent",
                            (
                                "exponent".to_owned(),
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
                                "amplitude".to_owned(),
                                params.amplitude.min,
                                params.amplitude.max,
                                params.amplitude.initial_guess,
                                params.amplitude.vary,
                            ),
                        )?;
                        background_params.set_item(
                            "exponent",
                            (
                                "exponent".to_owned(),
                                params.exponent.min,
                                params.exponent.max,
                                params.exponent.initial_guess,
                                params.exponent.vary,
                            ),
                        )?;
                    }
                }
                BackgroundModel::None => {
                    background_params.set_item("bg_type", "None")?;
                }
            }

            log::info!("Fitting Gaussian model");

            let result = if let Some(path) = load_result_path {
                // Load the lmfit result from the file
                module.getattr("load_result")?.call1((path,))?
            } else {
                // Call the GaussianFit function
                module.getattr("GaussianFit")?.call1((
                    y_data,
                    x_data.clone(),
                    region_markers,
                    peak_markers,
                    background_markers,
                    equal_sigma,
                    free_position,
                    background_params,
                ))?
            };

            let gaussian_params =
                result
                    .get_item(0)?
                    .extract::<Vec<(f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64)>>()?;
            let background_params = result.get_item(1)?.extract::<Vec<(String, f64, f64)>>()?;
            let x_composition = result.get_item(2)?.extract::<Vec<f64>>()?;
            let y_composition = result.get_item(3)?.extract::<Vec<f64>>()?;
            let fit_report = result.get_item(4)?.extract::<String>()?;
            let additional_params = result.get_item(5)?.extract::<Vec<(f64, f64)>>()?;

            // get the temp fit result, store the text in the struct
            let fit_text = std::fs::read_to_string("temp_fit.sav")
                .unwrap_or_else(|_| "Failed to read fit result file.".to_owned());

            self.lmfit_result = Some(fit_text);

            // remove the temp file
            std::fs::remove_file("temp_fit.sav").unwrap_or_else(|_| {
                log::warn!("Failed to remove temp fit result file.");
            });

            self.peak_markers.clear();

            for (
                (
                    amp,
                    amp_err,
                    mean,
                    mean_err,
                    sigma,
                    sigma_err,
                    fwhm,
                    fwhm_err,
                    area,
                    area_err,
                    uuid,
                ),
                (energy, energy_err),
            ) in gaussian_params.iter().zip(additional_params.iter())
            {
                log::info!(
                    "Amplitude: {amp:.3} ± {amp_err:.3}, Mean: {mean:.3} ± {mean_err:.3}, Sigma: {sigma:.3} ± {sigma_err:.3}, FWHM: {fwhm:.3} ± {fwhm_err:.3}, Area: {area:.3} ± {area_err:.3}"
                );

                self.peak_markers.push(*mean);

                // Create the GaussianParameters for each set of values
                let mut gaussian_param = GaussianParameters::new(
                    (*amp, *amp_err),
                    (*mean, *mean_err),
                    (*sigma, *sigma_err),
                    (*fwhm, *fwhm_err),
                    (*area, *area_err),
                );

                gaussian_param.uuid = *uuid as usize;

                gaussian_param.energy.value = Some(*energy);
                gaussian_param.energy.uncertainty = Some(*energy_err);

                // Generate the fit points for this Gaussian, using 100 points (or as many as needed)
                gaussian_param.generate_fit_points(100);

                self.fit_result.push(gaussian_param);
            }

            if self.background_result.is_none() && !background_params.is_empty() {
                let bg_type = background_params[0].0.as_str();

                let min_x = x_composition.iter().copied().fold(f64::INFINITY, f64::min);
                let max_x = x_composition
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max);

                match bg_type {
                    "bg_slope" | "bg_intercept" => {
                        // assume Linear
                        self.background_model = BackgroundModel::Linear(Default::default());

                        let slope = background_params
                            .iter()
                            .find(|(name, _, _)| name == "bg_slope")
                            .map(|(_, val, _)| *val)
                            .unwrap_or(0.0);

                        let intercept = background_params
                            .iter()
                            .find(|(name, _, _)| name == "bg_intercept")
                            .map(|(_, val, _)| *val)
                            .unwrap_or(0.0);

                        let linear = LinearFitter::new_from_parameters(
                            (slope, 0.0),
                            (intercept, 0.0),
                            min_x,
                            max_x,
                        );
                        self.background_result = Some(BackgroundResult::Linear(linear));
                    }

                    "bg_a" | "bg_b" | "bg_c" => {
                        self.background_model = BackgroundModel::Quadratic(Default::default());

                        let a = background_params
                            .iter()
                            .find(|(name, _, _)| name == "bg_a")
                            .map(|(_, val, _)| *val)
                            .unwrap_or(0.0);
                        let b = background_params
                            .iter()
                            .find(|(name, _, _)| name == "bg_b")
                            .map(|(_, val, _)| *val)
                            .unwrap_or(0.0);
                        let c = background_params
                            .iter()
                            .find(|(name, _, _)| name == "bg_c")
                            .map(|(_, val, _)| *val)
                            .unwrap_or(0.0);

                        let quad = QuadraticFitter::new_from_parameters(
                            (a, 0.0),
                            (b, 0.0),
                            (c, 0.0),
                            min_x,
                            max_x,
                        );
                        self.background_result = Some(BackgroundResult::Quadratic(quad));
                    }

                    "bg_amplitude"
                        if background_params
                            .iter()
                            .any(|(k, _, _)| k.contains("decay")) =>
                    {
                        self.background_model = BackgroundModel::Exponential(Default::default());

                        let amplitude = background_params
                            .iter()
                            .find(|(name, _, _)| name == "bg_amplitude")
                            .map(|(_, val, _)| *val)
                            .unwrap_or(0.0);
                        let decay = background_params
                            .iter()
                            .find(|(name, _, _)| name == "bg_decay")
                            .map(|(_, val, _)| *val)
                            .unwrap_or(0.0);

                        let expo = ExponentialFitter::new_from_parameters(
                            (amplitude, 0.0),
                            (decay, 0.0),
                            min_x,
                            max_x,
                        );
                        self.background_result = Some(BackgroundResult::Exponential(expo));
                    }

                    "bg_amplitude"
                        if background_params
                            .iter()
                            .any(|(k, _, _)| k.contains("exponent")) =>
                    {
                        self.background_model = BackgroundModel::PowerLaw(Default::default());

                        let amplitude = background_params
                            .iter()
                            .find(|(name, _, _)| name == "bg_amplitude")
                            .map(|(_, val, _)| *val)
                            .unwrap_or(0.0);
                        let exponent = background_params
                            .iter()
                            .find(|(name, _, _)| name == "bg_exponent")
                            .map(|(_, val, _)| *val)
                            .unwrap_or(0.0);

                        let powerlaw = PowerLawFitter::new_from_parameters(
                            (amplitude, 0.0),
                            (exponent, 0.0),
                            min_x,
                            max_x,
                        );
                        self.background_result = Some(BackgroundResult::PowerLaw(powerlaw));
                    }

                    _ => {
                        self.background_model = BackgroundModel::None;
                        self.background_result = None;
                    }
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

    pub fn update_uuid_for_peak(&mut self, peak_index: usize, new_uuid: usize) -> PyResult<()> {
        // Step 1: Write current lmfit_result to a temp file
        let temp_path = PathBuf::from("temp_fit_uuid_update.sav");
        if let Some(ref lmfit) = self.lmfit_result {
            let mut file = File::create(&temp_path)?;
            file.write_all(lmfit.as_bytes())?;
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "No lmfit_result to update.",
            ));
        }

        // Step 2: Python call to update UUID
        Python::with_gil(|py| {
            let module = PyModule::from_code(
                py,
                c_str!(
                    "
from lmfit.model import load_modelresult, save_modelresult

def Add_UUID_to_Result(file_path: str, peak_number: int, uuid: int):
    result = load_modelresult(file_path)

    if f'g{peak_number}_uuid' not in result.params:
        result.params.add(f'g{peak_number}_uuid', value=uuid, vary=False)
    else:
        result.params[f'g{peak_number}_uuid'].set(value=uuid, vary=False)
    save_modelresult(result, file_path)

    fit_report = result.fit_report()

    return result.fit_report()
"
                ),
                c_str!("uuid_patch.py"),
                c_str!("uuid_patch"),
            )?;

            let fit_report: String = module
                .getattr("Add_UUID_to_Result")?
                .call1((
                    temp_path.to_str().expect("Temp path should be valid"),
                    peak_index,
                    new_uuid,
                ))?
                .extract()?;

            // Step 3: Reload updated file into lmfit_result
            let updated_lmfit = std::fs::read_to_string(&temp_path)
                .unwrap_or_else(|_| "Failed to read updated fit.".to_owned());

            std::fs::remove_file(&temp_path).unwrap_or_else(|err| {
                eprintln!("Warning: failed to remove temp fit file: {err}");
            });

            self.fit_result[peak_index].uuid = new_uuid;
            self.lmfit_result = Some(updated_lmfit);
            self.fit_report = fit_report;

            Ok(())
        })
    }

    pub fn add_calibration_to_result(&mut self, calibration: &Calibration) -> PyResult<()> {
        // Step 1: Write current lmfit_result to a temp file
        let temp_path = PathBuf::from("temp_fit_energy_calibration_update.sav");
        if let Some(ref lmfit) = self.lmfit_result {
            let mut file = File::create(&temp_path)?;
            file.write_all(lmfit.as_bytes())?;
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "No lmfit_result to update.",
            ));
        }

        // Step 2: Python call to update energy
        Python::with_gil(|py| {
            let module = PyModule::from_code(
                py,
                c_str!("
from lmfit.model import load_modelresult, save_modelresult
import numpy as np

def Update_EnergyCalibration(file_path: str, a: float, a_uncertainty: float, b: float, b_uncertainty: float, c: float, c_uncertainty: float):
    result = load_modelresult(file_path)

    def set_param(name, value, uncertainty):
        if name not in result.params:
            result.params.add(name, value=value, vary=False)
        else:
            result.params[name].set(value=value, vary=False)

        unc_name = f'{name}_uncertainty'
        if unc_name not in result.params:
            result.params.add(unc_name, value=uncertainty, vary=False)
        else:
            result.params[unc_name].set(value=uncertainty, vary=False)

    # Store calibration values as constants
    set_param('calibration_a', a, a_uncertainty)
    set_param('calibration_b', b, b_uncertainty)
    set_param('calibration_c', c, c_uncertainty)

    i = 0
    while f'g{i}_center' in result.params:
        # Add expression-based parameters
        result.params.add(
            f'g{i}_center_calibrated',
            expr=f'calibration_a * g{i}_center**2 + calibration_b * g{i}_center + calibration_c',
            vary=False,
        )
        result.params.add(
            f'g{i}_sigma_calibrated',
            expr=f'abs((2 * calibration_a * g{i}_center + calibration_b) * g{i}_sigma)',
            vary=False,
        )
        result.params.add(
            f'g{i}_fwhm_calibrated',
            expr=f'2.3548200 * g{i}_sigma_calibrated',
            vary=False,
        )

        # Direct copies of uncalibrated
        for param in ['area', 'amplitude', 'height']:
            name = f'g{i}_{param}_calibrated'
            if name not in result.params:
                result.params.add(name, expr=f'g{i}_{param}', vary=False)
            else:
                result.params[name].set(expr=f'g{i}_{param}', vary=False)

        # Evaluate expressions and set stderr manually
        center = result.params[f'g{i}_center'].value
        center_unc = result.params[f'g{i}_center'].stderr or 0.0
        sigma = result.params[f'g{i}_sigma'].value
        sigma_unc = result.params[f'g{i}_sigma'].stderr or 0.0

        dx_dE = 2 * a * center + b

        # Propagated uncertainty for calibrated center
        d_center_cal = np.sqrt(
            (center ** 2 * a_uncertainty) ** 2 +
            (center * b_uncertainty) ** 2 +
            c_uncertainty ** 2 +
            ((2 * a * center + b) * center_unc) ** 2
        )

        # Calibrated sigma
        d_sigma_cal = np.sqrt(
            (2 * center * a_uncertainty * sigma) ** 2 +
            (b_uncertainty * sigma) ** 2 +
            (dx_dE * sigma_unc) ** 2
        )

        # FWHM
        d_fwhm_cal = 2.3548200 * d_sigma_cal

        result.params[f'g{i}_center_calibrated'].stderr = d_center_cal
        result.params[f'g{i}_sigma_calibrated'].stderr = d_sigma_cal
        result.params[f'g{i}_fwhm_calibrated'].stderr = d_fwhm_cal

        i += 1

    save_modelresult(result, file_path)

    print('Post Fit Report:')
    fit_report = result.fit_report()
    print(fit_report)

    return result.fit_report()
"),
                c_str!("energy_patch.py"),
                c_str!("energy_patch"),
            )?;

            let a = calibration.a.value;
            let a_uncertainty = calibration.a.uncertainty;
            let b = calibration.b.value;
            let b_uncertainty = calibration.b.uncertainty;
            let c = calibration.c.value;
            let c_uncertainty = calibration.c.uncertainty;

            let fit_report: String = module
                .getattr("Update_EnergyCalibration")?
                .call1((
                    temp_path.to_str().expect("Temp path should be valid"),
                    a,
                    a_uncertainty,
                    b,
                    b_uncertainty,
                    c,
                    c_uncertainty,
                ))?
                .extract()?;

            // Step 3: Reload updated file into lmfit_result
            let updated_lmfit = std::fs::read_to_string(&temp_path)
                .unwrap_or_else(|_| "Failed to read updated fit.".to_owned());

            std::fs::remove_file(&temp_path).unwrap_or_else(|err| {
                eprintln!("Warning: failed to remove temp fit file: {err}");
            });

            self.lmfit_result = Some(updated_lmfit);
            self.fit_report = fit_report;

            Ok(())
        })
    }

    pub fn update_energy_for_peak(
        &mut self,
        peak_index: usize,
        new_energy: f64,
        new_uncertainty: f64,
    ) -> PyResult<()> {
        // Step 1: Write current lmfit_result to a temp file
        let temp_path = PathBuf::from("temp_fit_energy_update.sav");
        if let Some(ref lmfit) = self.lmfit_result {
            let mut file = File::create(&temp_path)?;
            file.write_all(lmfit.as_bytes())?;
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "No lmfit_result to update.",
            ));
        }

        // Step 2: Python call to update energy
        Python::with_gil(|py| {
            let module = PyModule::from_code(
                py,
                c_str!(
                    "
from lmfit.model import load_modelresult, save_modelresult

def Update_Energy(file_path: str, peak_number: int, energy: float, uncertainty: float):
    result = load_modelresult(file_path)

    if f'g{peak_number}_energy' not in result.params:
        result.params.add(f'g{peak_number}_energy', value=energy, vary=False)
    else:
        result.params[f'g{peak_number}_energy'].set(value=energy, vary=False)

    if f'g{peak_number}_energy_uncertainty' not in result.params:
        result.params.add(f'g{peak_number}_energy_uncertainty', value=uncertainty, vary=False)
    else:
        result.params[f'g{peak_number}_energy_uncertainty'].set(value=uncertainty, vary=False)

    save_modelresult(result, file_path)

    fit_report = result.fit_report()

    return result.fit_report()
"
                ),
                c_str!("energy_patch.py"),
                c_str!("energy_patch"),
            )?;

            let fit_report: String = module
                .getattr("Update_Energy")?
                .call1((
                    temp_path.to_str().expect("Temp path should be valid"),
                    peak_index,
                    new_energy,
                    new_uncertainty,
                ))?
                .extract()?;

            // Step 3: Reload updated file into lmfit_result
            let updated_lmfit = std::fs::read_to_string(&temp_path)
                .unwrap_or_else(|_| "Failed to read updated fit.".to_owned());

            std::fs::remove_file(&temp_path).unwrap_or_else(|err| {
                eprintln!("Warning: failed to remove temp fit file: {err}");
            });

            self.fit_result[peak_index].energy.value = Some(new_energy);
            self.fit_result[peak_index].energy.uncertainty = Some(new_uncertainty);
            self.lmfit_result = Some(updated_lmfit);
            self.fit_report = fit_report;

            Ok(())
        })
    }

    pub fn draw_uuid(&self, plot_ui: &mut egui_plot::PlotUi<'_>, calibrate: bool) {
        use egui::Align2;
        use egui_plot::Text;

        for params in &self.fit_result {
            if params.uuid == 0 {
                continue; // Skip if UUID is not set
            }
            if let Some(mean) = params.mean.value {
                let position = if calibrate {
                    if let Some(calibrated_mean) = params.mean.calibrated_value {
                        calibrated_mean
                    } else {
                        mean
                    }
                } else {
                    mean
                };
                let label = Text::new("", [position, 0.0].into(), params.uuid.to_string())
                    .anchor(Align2::CENTER_BOTTOM);

                plot_ui.text(label);
            }
        }
    }

    pub fn fit_params_ui(&mut self, ui: &mut egui::Ui, skip_one: bool, calibrate: bool) {
        let mut uuid_updates = Vec::new();
        let mut energy_updates = Vec::new();

        for (i, params) in self.fit_result.iter_mut().enumerate() {
            if skip_one && i != 0 {
                ui.label("");
            }
            ui.label(format!("{i}"));
            params.params_ui(ui, calibrate);

            let mut uuid = params.uuid;
            if ui.add(egui::DragValue::new(&mut uuid).speed(1)).changed() {
                uuid_updates.push((i, uuid)); // defer the update
            }

            let mut energy = params.energy.value.unwrap_or(-1.0);
            let mut uncertainty = params.energy.uncertainty.unwrap_or(0.0);

            ui.horizontal(|ui| {
                let mut changed = false;
                changed |= ui
                    .add(egui::DragValue::new(&mut energy).speed(0.1))
                    .changed();
                ui.label("±");
                changed |= ui
                    .add(egui::DragValue::new(&mut uncertainty).speed(0.1))
                    .changed();

                if changed {
                    energy_updates.push((i, energy, uncertainty));
                }
            });

            if i == 0 {
                if let Some(ref text) = self.lmfit_result
                    && ui.button("Export").clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .set_file_name("fit_result.txt")
                            .save_file()
                        {
                            if let Err(e) = std::fs::write(&path, text) {
                                eprintln!("Failed to save lmfit result: {e}");
                            } else {
                                log::info!("Saved lmfit result to {path:?}");
                            }
                        }

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

        for (index, new_uuid) in uuid_updates {
            println!("Updating UUID for peak {index}: {new_uuid}");
            if let Err(e) = self.update_uuid_for_peak(index, new_uuid) {
                eprintln!("UUID update failed: {e}");
            }
        }

        for (index, new_energy, new_uncertainty) in energy_updates {
            println!(
                "Updating energy for peak {index}: {new_energy}, uncertainty: {new_uncertainty}"
            );
            if let Err(e) = self.update_energy_for_peak(index, new_energy, new_uncertainty) {
                eprintln!("Energy update failed: {e}");
            }
        }
    }
}
