use crate::fitter::common::{Data, Parameter};
use pyo3::{prelude::*, types::PyModule};

#[derive(PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ExponentialParameters {
    pub amplitude: Parameter,
    pub decay: Parameter,
}

impl Default for ExponentialParameters {
    fn default() -> Self {
        ExponentialParameters {
            amplitude: Parameter {
                name: "amplitude".to_string(),
                ..Default::default()
            },
            decay: Parameter {
                name: "decay".to_string(),
                ..Default::default()
            },
        }
    }
}

impl ExponentialParameters {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Fit Parameters");
            if ui.small_button("Reset").clicked() {
                *self = ExponentialParameters::default();
            }
        });
        // create a grid for the param
        egui::Grid::new("Exponential_params_grid")
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
                self.decay.ui(ui);
            });
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ExponentialFitter {
    pub data: Data,
    pub paramaters: ExponentialParameters,
    pub fit_points: Vec<[f64; 2]>,
    pub fit_report: String,
}

impl ExponentialFitter {
    pub fn new(data: Data) -> Self {
        ExponentialFitter {
            data,
            paramaters: ExponentialParameters::default(),
            fit_points: Vec::new(),
            fit_report: String::new(),
        }
    }

    pub fn lmfit(&mut self) -> PyResult<()> {
        log::info!("Fitting data with a Exponential line using `lmfit`.");
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

            match py.import_bound("numpy") {
                Ok(_) => {
                    // println!("Successfully imported `lmfit` module.");
                }
                Err(_) => {
                    eprintln!("Error: `numpy` module could not be found. Make sure you are using the correct Python environment with `numpy` installed.");
                    return Err(PyErr::new::<pyo3::exceptions::PyImportError, _>(
                        "`numpy` module not available",
                    ));
                }
            }

            // Define the Python code as a module
            let code = r#"
import lmfit
import numpy as np

def ExponentialFit(x_data: list, y_data: list, amplitude: list = ("amplitude", -np.inf, np.inf, 0.0, True), decay = ("decay", -np.inf, np.inf, 0.0, True)):    
    # params = [name, min, max, initial_guess, vary]
    
    model = lmfit.models.ExponentialModel()
    params = model.make_params(amplitude=amplitude[3], decay=decay[3])
    params['amplitude'].set(min=amplitude[1], max=amplitude[2], value=amplitude[3], vary=amplitude[4])
    params['decay'].set(min=decay[1], max=decay[2], value=decay[3], vary=decay[4])
    result = model.fit(y_data, params, x=x_data)

    print(result.fit_report())

    # Extract Parameters
    amplitude = float(result.params['amplitude'].value)
    amplitude_err = result.params['amplitude'].stderr
    if amplitude_err is None:
        amplitude_err = float(0.0)
    else:
        amplitude_err = float(amplitude_err)
    
    decay = float(result.params['decay'].value)
    decay_err = result.params['decay'].stderr
    if decay_err is None:
        decay_err = float(0.0)
    else:
        decay_err = float(decay_err)

    params = [
        ('amplitude', amplitude, amplitude_err),
        ('decay', decay, decay_err)
    ]

    x = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
    y = result.eval(x=x)

    fit_report = str(result.fit_report())

    return params, x, y, fit_report
"#;

            // Compile the Python code into a module
            let module = PyModule::from_code_bound(py, code, "Exponential.py", "Exponential")?;

            let x_data = self.data.x.clone();
            let y_data = self.data.y.clone();
            let amplitude_para = (
                self.paramaters.amplitude.name.clone(),
                self.paramaters.amplitude.min,
                self.paramaters.amplitude.max,
                self.paramaters.amplitude.initial_guess,
                self.paramaters.amplitude.vary,
            );
            let decay_para = (
                self.paramaters.decay.name.clone(),
                self.paramaters.decay.min,
                self.paramaters.decay.max,
                self.paramaters.decay.initial_guess,
                self.paramaters.decay.vary,
            );

            let result = module.getattr("ExponentialFit")?.call1((
                x_data,
                y_data,
                amplitude_para,
                decay_para,
            ))?;

            let params = result.get_item(0)?.extract::<Vec<(String, f64, f64)>>()?;
            let x = result.get_item(1)?.extract::<Vec<f64>>()?;
            let y = result.get_item(2)?.extract::<Vec<f64>>()?;
            let fit_report = result.get_item(3)?.extract::<String>()?;

            self.paramaters.amplitude.value = Some(params[0].1);
            self.paramaters.amplitude.uncertainity = Some(params[0].2);
            self.paramaters.decay.value = Some(params[1].1);
            self.paramaters.decay.uncertainity = Some(params[1].2);

            self.fit_points = x.iter().zip(y.iter()).map(|(&x, &y)| [x, y]).collect();
            self.fit_report = fit_report;

            Ok(())
        })
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        // add menu button for the fit report
        ui.horizontal(|ui| {
            if let Some(amplitude) = &self.paramaters.amplitude.value {
                ui.label(format!(
                    "amplitude: {:.3} ± {:.3}",
                    amplitude,
                    self.paramaters.amplitude.uncertainity.unwrap_or(0.0)
                ));
            }
            ui.separator();
            if let Some(decay) = &self.paramaters.decay.value {
                ui.label(format!(
                    "decay: {:.3} ± {:.3}",
                    decay,
                    self.paramaters.decay.uncertainity.unwrap_or(0.0)
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
