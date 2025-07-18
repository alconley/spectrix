use crate::fitter::common::{Data, Parameter};
use pyo3::{ffi::c_str, prelude::*, types::PyModule};

#[derive(PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PowerLawParameters {
    pub amplitude: Parameter,
    pub exponent: Parameter,
}

impl Default for PowerLawParameters {
    fn default() -> Self {
        Self {
            amplitude: Parameter {
                name: "amplitude".to_owned(),
                ..Default::default()
            },
            exponent: Parameter {
                name: "exponent".to_owned(),
                initial_guess: -1.0,
                ..Default::default()
            },
        }
    }
}

impl PowerLawParameters {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Fit Parameters");
            if ui.small_button("Reset").clicked() {
                *self = Self::default();
            }
        });
        // create a grid for the param
        egui::Grid::new("PowerLaw_params_grid")
            .striped(true)
            .num_columns(5)
            .show(ui, |ui| {
                ui.label("Parameter");
                ui.label("Initial Guess");
                ui.label("Min");
                ui.label("Max");
                ui.label("Vary");
                ui.end_row();
                self.amplitude.ui(ui);
                ui.end_row();
                self.exponent.ui(ui);
            });
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PowerLawFitter {
    pub data: Data,
    pub paramaters: PowerLawParameters,
    pub fit_points: Vec<[f64; 2]>,
    pub fit_report: String,
}

impl PowerLawFitter {
    pub fn new(data: Data) -> Self {
        Self {
            data,
            paramaters: PowerLawParameters::default(),
            fit_points: Vec::new(),
            fit_report: String::new(),
        }
    }

    pub fn new_from_parameters(
        amplitude: (f64, f64),
        exponent: (f64, f64),
        min_x: f64,
        max_x: f64,
    ) -> Self {
        let mut fitter = Self {
            data: Data::default(),
            paramaters: PowerLawParameters::default(),
            fit_points: Vec::new(),
            fit_report: "Fitted with other model".to_owned(),
        };

        // Set the parameter values and uncertainties
        fitter.paramaters.amplitude.value = Some(amplitude.0);
        fitter.paramaters.amplitude.uncertainty = Some(amplitude.1);
        fitter.paramaters.exponent.value = Some(exponent.0);
        fitter.paramaters.exponent.uncertainty = Some(exponent.1);

        // Generate fit points for the power-law model
        let num_points = 100;
        let step_size = (max_x - min_x) / (num_points as f64);
        fitter.fit_points.clear();
        for i in 0..=num_points {
            let x = min_x + i as f64 * step_size;
            let y = fitter.paramaters.amplitude.value.unwrap_or(1.0)
                * x.powf(fitter.paramaters.exponent.value.unwrap_or(-1.0));
            fitter.fit_points.push([x, y]);
        }

        fitter
    }

    pub fn lmfit(&mut self) -> PyResult<()> {
        log::info!("Fitting data with a PowerLaw line using `lmfit`.");
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
                eprintln!("Error: `lmfit` module could not be found. Make sure you are using the correct Python environment with `lmfit` installed.");
                return Err(PyErr::new::<pyo3::exceptions::PyImportError, _>(
                    "`lmfit` module not available",
                ));
            }

            if py.import("numpy").is_ok() {
                // println!("Successfully imported `lmfit` module.");
            } else {
                eprintln!("Error: `numpy` module could not be found. Make sure you are using the correct Python environment with `numpy` installed.");
                return Err(PyErr::new::<pyo3::exceptions::PyImportError, _>(
                    "`numpy` module not available",
                ));
            }

            // Define the Python code as a module
            let code = c_str!("
import lmfit
import numpy as np

def PowerLawFit(x_data: list, y_data: list, amplitude: list = ('amplitude', -np.inf, np.inf, 0.0, True), exponent = ('exponent', -np.inf, np.inf, 0.0, True)):    
    # params = [name, min, max, initial_guess, vary]
    
    model = lmfit.models.PowerLawModel()
    params = model.make_params(amplitude=amplitude[3], exponent=exponent[3])
    params['amplitude'].set(min=amplitude[1], max=amplitude[2], value=amplitude[3], vary=amplitude[4])
    params['exponent'].set(min=exponent[1], max=exponent[2], value=exponent[3], vary=exponent[4])
    result = model.fit(y_data, params, x=x_data)

    print(result.fit_report())

    # Extract Parameters
    amplitude = float(result.params['amplitude'].value)
    amplitude_err = result.params['amplitude'].stderr
    if amplitude_err is None:
        amplitude_err = float(0.0)
    else:
        amplitude_err = float(amplitude_err)
    
    exponent = float(result.params['exponent'].value)
    exponent_err = result.params['exponent'].stderr
    if exponent_err is None:
        exponent_err = float(0.0)
    else:
        exponent_err = float(exponent_err)

    params = [
        ('amplitude', amplitude, amplitude_err),
        ('exponent', exponent, exponent_err)
    ]

    x = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
    y = result.eval(x=x)

    fit_report = str(result.fit_report())

    return params, x, y, fit_report
");

            // Compile the Python code into a module
            let module = PyModule::from_code(py, code, c_str!("powerlaw.py"), c_str!("powerlaw"))?;

            let x_data = self.data.x.clone();
            let y_data = self.data.y.clone();
            let amplitude_para = (
                self.paramaters.amplitude.name.clone(),
                self.paramaters.amplitude.min,
                self.paramaters.amplitude.max,
                self.paramaters.amplitude.initial_guess,
                self.paramaters.amplitude.vary,
            );
            let exponent_para = (
                self.paramaters.exponent.name.clone(),
                self.paramaters.exponent.min,
                self.paramaters.exponent.max,
                self.paramaters.exponent.initial_guess,
                self.paramaters.exponent.vary,
            );

            let result = module.getattr("PowerLawFit")?.call1((
                x_data,
                y_data,
                amplitude_para,
                exponent_para,
            ))?;

            let params = result.get_item(0)?.extract::<Vec<(String, f64, f64)>>()?;
            let x = result.get_item(1)?.extract::<Vec<f64>>()?;
            let y = result.get_item(2)?.extract::<Vec<f64>>()?;
            let fit_report = result.get_item(3)?.extract::<String>()?;

            self.paramaters.amplitude.value = Some(params[0].1);
            self.paramaters.amplitude.uncertainty = Some(params[0].2);
            self.paramaters.exponent.value = Some(params[1].1);
            self.paramaters.exponent.uncertainty = Some(params[1].2);

            self.fit_points = x.iter().zip(y.iter()).map(|(&x, &y)| [x, y]).collect();
            self.fit_report = fit_report;

            Ok(())
        })
    }

    pub fn evaluate(&self, x: f64) -> f64 {
        self.paramaters.amplitude.value.unwrap_or(1.0)
            * x.powf(self.paramaters.exponent.value.unwrap_or(-1.0))
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        // add menu button for the fit report
        ui.horizontal(|ui| {
            if let Some(amplitude) = &self.paramaters.amplitude.value {
                ui.label(format!(
                    "amplitude: {:.3} ± {:.3}",
                    amplitude,
                    self.paramaters.amplitude.uncertainty.unwrap_or(0.0)
                ));
            }
            ui.separator();
            if let Some(exponent) = &self.paramaters.exponent.value {
                ui.label(format!(
                    "exponent: {:.3} ± {:.3}",
                    exponent,
                    self.paramaters.exponent.uncertainty.unwrap_or(0.0)
                ));
            }
            ui.separator();
            ui.menu_button("Fit Report", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(self.fit_report.clone());
                });
            });
        });
    }
}
