use crate::fitter::common::{Data, Parameter};
use pyo3::{prelude::*, types::PyModule};

#[derive(PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LinearParameters {
    pub slope: Parameter,
    pub intercept: Parameter,
}

impl Default for LinearParameters {
    fn default() -> Self {
        LinearParameters {
            slope: Parameter {
                name: "slope".to_string(),
                ..Default::default()
            },
            intercept: Parameter {
                name: "intercept".to_string(),
                ..Default::default()
            },
        }
    }
}

impl LinearParameters {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Fit Parameters");
            if ui.small_button("Reset").clicked() {
                *self = LinearParameters::default();
            }
        });
        // create a grid for the param
        egui::Grid::new("linear_params_grid")
            .striped(true)
            .num_columns(5)
            .show(ui, |ui| {
                ui.label("Parameter");
                ui.label("Initial Guess");
                ui.label("Min");
                ui.label("Max");
                ui.label("Vary");
                ui.end_row();
                self.slope.ui(ui);
                ui.end_row();
                self.intercept.ui(ui);
            });
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LinearFitter {
    pub data: Data,
    pub paramaters: LinearParameters,
    pub fit_points: Vec<[f64; 2]>,
    pub fit_report: String,
}

impl LinearFitter {
    pub fn new(data: Data) -> Self {
        LinearFitter {
            data,
            paramaters: LinearParameters::default(),
            fit_points: Vec::new(),
            fit_report: String::new(),
        }
    }

    pub fn new_from_parameters(
        slope: (f64, f64),
        intercept: (f64, f64),
        min_x: f64,
        max_x: f64,
    ) -> Self {
        let fit_points = vec![
            [min_x, slope.0 * min_x + intercept.0],
            [max_x, slope.0 * max_x + intercept.0],
        ];
        let paramaters = LinearParameters {
            slope: Parameter {
                name: "slope".to_string(),
                min: f64::NEG_INFINITY,
                max: f64::INFINITY,
                initial_guess: slope.0,
                vary: true,
                value: Some(slope.0),
                uncertainty: Some(slope.1),
            },
            intercept: Parameter {
                name: "intercept".to_string(),
                min: f64::NEG_INFINITY,
                max: f64::INFINITY,
                initial_guess: intercept.0,
                vary: true,
                value: Some(intercept.0),
                uncertainty: Some(intercept.1),
            },
        };
        LinearFitter {
            data: Data {
                x: Vec::new(),
                y: Vec::new(),
            },
            paramaters,
            fit_points,
            fit_report: "Fitter with other model".to_string(),
        }
    }

    pub fn lmfit(&mut self) -> PyResult<()> {
        log::info!("Fitting data with a linear line using `lmfit`.");
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

def LinearFit(x_data: list, y_data: list, slope: list = ("slope", -np.inf, np.inf, 0.0, True), intercept = ("intercept", -np.inf, np.inf, 0.0, True)):    
    # params
    # slope=[name, min, max, initial_guess, vary]
    # intercept = [name, min, max, initial_guess, vary]
    
    model = lmfit.models.LinearModel()
    params = model.make_params(slope=slope[3], intercept=intercept[3])
    params['slope'].set(min=slope[1], max=slope[2], value=slope[3], vary=slope[4])
    params['intercept'].set(min=intercept[1], max=intercept[2], value=intercept[3], vary=intercept[4])

    result = model.fit(y_data, params, x=x_data)

    print(result.fit_report())

    # Extract Parameters
    slope = float(result.params['slope'].value)
    slope_err = result.params['slope'].stderr
    
    if slope_err is None:
        slope_err = float(0.0)
    else:
        slope_err = float(slope_err)

    intercept = float(result.params['intercept'].value)
    
    intercept_err = result.params['intercept'].stderr
    if intercept_err is None:
        intercept_err = float(0.0)
    else:
        intercept_err = float(intercept_err)

    params = [
        ('slope', slope, slope_err),
        ('intercept', intercept, intercept_err)
    ]

    x = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
    y = result.eval(x=x)

    fit_report = str(result.fit_report())

    return params, x, y, fit_report
"#;

            // Compile the Python code into a module
            let module = PyModule::from_code_bound(py, code, "linear.py", "linear")?;

            let x_data = self.data.x.clone();
            let y_data = self.data.y.clone();
            let slope_para = (
                self.paramaters.slope.name.clone(),
                self.paramaters.slope.min,
                self.paramaters.slope.max,
                self.paramaters.slope.initial_guess,
                self.paramaters.slope.vary,
            );
            let intercept_para: (String, f64, f64, f64, bool) = (
                self.paramaters.intercept.name.clone(),
                self.paramaters.intercept.min,
                self.paramaters.intercept.max,
                self.paramaters.intercept.initial_guess,
                self.paramaters.intercept.vary,
            );

            let result =
                module
                    .getattr("LinearFit")?
                    .call1((x_data, y_data, slope_para, intercept_para))?;

            let params = result.get_item(0)?.extract::<Vec<(String, f64, f64)>>()?;
            let x = result.get_item(1)?.extract::<Vec<f64>>()?;
            let y = result.get_item(2)?.extract::<Vec<f64>>()?;
            let fit_report = result.get_item(3)?.extract::<String>()?;

            self.paramaters.slope.value = Some(params[0].1);
            self.paramaters.slope.uncertainty = Some(params[0].2);
            self.paramaters.intercept.value = Some(params[1].1);
            self.paramaters.intercept.uncertainty = Some(params[1].2);

            self.fit_points = x.iter().zip(y.iter()).map(|(&x, &y)| [x, y]).collect();
            self.fit_report = fit_report;

            Ok(())
        })
    }

    pub fn evaluate(&self, x: f64) -> f64 {
        let slope = self.paramaters.slope.value.unwrap_or(0.0);
        let intercept = self.paramaters.intercept.value.unwrap_or(0.0);
        slope * x + intercept
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        // add menu button for the fit report
        ui.horizontal(|ui| {
            if let Some(slope) = &self.paramaters.slope.value {
                ui.label(format!(
                    "Slope: {:.3} ± {:.3}",
                    slope,
                    self.paramaters.slope.uncertainty.unwrap_or(0.0)
                ));
            }
            ui.separator();
            if let Some(intercept) = &self.paramaters.intercept.value {
                ui.label(format!(
                    "Intercept: {:.3} ± {:.3}",
                    intercept,
                    self.paramaters.intercept.uncertainty.unwrap_or(0.0)
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
