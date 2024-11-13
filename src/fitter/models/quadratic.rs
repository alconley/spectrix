use crate::fitter::common::{Data, Parameter};
use pyo3::{prelude::*, types::PyModule};

#[derive(PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct QuadraticParameters {
    pub a: Parameter,
    pub b: Parameter,
    pub c: Parameter,
}

impl Default for QuadraticParameters {
    fn default() -> Self {
        QuadraticParameters {
            a: Parameter {
                name: "a".to_string(),
                ..Default::default()
            },
            b: Parameter {
                name: "b".to_string(),
                ..Default::default()
            },
            c: Parameter {
                name: "c".to_string(),
                ..Default::default()
            },
        }
    }
}

impl QuadraticParameters {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Fit Parameters");
            if ui.small_button("Reset").clicked() {
                *self = QuadraticParameters::default();
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
                self.a.ui(ui);
                ui.end_row();
                self.b.ui(ui);
                ui.end_row();
                self.c.ui(ui);
            });
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct QuadraticFitter {
    pub data: Data,
    pub paramaters: QuadraticParameters,
    pub fit_points: Vec<[f64; 2]>,
    pub fit_report: String,
}

impl QuadraticFitter {
    pub fn new(data: Data) -> Self {
        QuadraticFitter {
            data,
            paramaters: QuadraticParameters::default(),
            fit_points: Vec::new(),
            fit_report: String::new(),
        }
    }

    pub fn new_from_parameters(
        a: (f64, f64),
        b: (f64, f64),
        c: (f64, f64),
        min_x: f64,
        max_x: f64,
    ) -> Self {
        let mut fitter = QuadraticFitter {
            data: Data::default(),
            paramaters: QuadraticParameters::default(),
            fit_points: Vec::new(),
            fit_report: "Fitted with other model".to_string(),
        };

        // Set the parameter values and uncertainties
        fitter.paramaters.a.value = Some(a.0);
        fitter.paramaters.a.uncertainty = Some(a.1);
        fitter.paramaters.b.value = Some(b.0);
        fitter.paramaters.b.uncertainty = Some(b.1);
        fitter.paramaters.c.value = Some(c.0);
        fitter.paramaters.c.uncertainty = Some(c.1);

        // Generate fit points
        let num_points = 100;
        let step_size = (max_x - min_x) / (num_points as f64);
        fitter.fit_points.clear();
        for i in 0..=num_points {
            let x = min_x + i as f64 * step_size;
            let y = fitter.paramaters.a.value.unwrap_or(0.0) * x.powi(2)
                + fitter.paramaters.b.value.unwrap_or(0.0) * x
                + fitter.paramaters.c.value.unwrap_or(0.0);
            fitter.fit_points.push([x, y]);
        }

        fitter
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

def QuadraticFit(x_data: list, y_data: list, a: list = ("a", -np.inf, np.inf, 0.0, True), b = ("b", -np.inf, np.inf, 0.0, True), c: list = ("a", -np.inf, np.inf, 0.0, True),):    
    # params = [name, min, max, initial_guess, vary]
    
    model = lmfit.models.QuadraticModel()
    params = model.make_params(a=a[3], b=b[3], c=c[3])
    params['a'].set(min=a[1], max=a[2], value=a[3], vary=a[4])
    params['b'].set(min=b[1], max=b[2], value=b[3], vary=b[4])
    params['c'].set(min=c[1], max=c[2], value=c[3], vary=c[4])
    result = model.fit(y_data, params, x=x_data)

    print(result.fit_report())

    # Extract Parameters
    a = float(result.params['a'].value)
    a_err = result.params['a'].stderr
    if a_err is None:
        a_err = float(0.0)
    else:
        a_err = float(a_err)

    b = float(result.params['b'].value)
    b_err = result.params['b'].stderr
    if b_err is None:
        b_err = float(0.0)
    else:
        b_err = float(b_err)

    c = float(result.params['c'].value)
    c_err = result.params['c'].stderr
    if c_err is None:
        c_err = float(0.0)
    else:
        c_err = float(c_err)


    params = [
        ('a', a, a_err),
        ('b', b, b_err),
        ('c', c, c_err)
    ]

    x = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
    y = result.eval(x=x)

    fit_report = str(result.fit_report())

    return params, x, y, fit_report
"#;

            // Compile the Python code into a module
            let module = PyModule::from_code_bound(py, code, "quadratic.py", "quadratic")?;

            let x_data = self.data.x.clone();
            let y_data = self.data.y.clone();
            let a_para = (
                self.paramaters.a.name.clone(),
                self.paramaters.a.min,
                self.paramaters.a.max,
                self.paramaters.a.initial_guess,
                self.paramaters.a.vary,
            );
            let b_para = (
                self.paramaters.b.name.clone(),
                self.paramaters.b.min,
                self.paramaters.b.max,
                self.paramaters.b.initial_guess,
                self.paramaters.b.vary,
            );
            let c_para = (
                self.paramaters.c.name.clone(),
                self.paramaters.c.min,
                self.paramaters.c.max,
                self.paramaters.c.initial_guess,
                self.paramaters.c.vary,
            );

            let result = module
                .getattr("QuadraticFit")?
                .call1((x_data, y_data, a_para, b_para, c_para))?;

            let params = result.get_item(0)?.extract::<Vec<(String, f64, f64)>>()?;
            let x = result.get_item(1)?.extract::<Vec<f64>>()?;
            let y = result.get_item(2)?.extract::<Vec<f64>>()?;
            let fit_report = result.get_item(3)?.extract::<String>()?;

            self.paramaters.a.value = Some(params[0].1);
            self.paramaters.a.uncertainty = Some(params[0].2);
            self.paramaters.b.value = Some(params[1].1);
            self.paramaters.b.uncertainty = Some(params[1].2);
            self.paramaters.c.value = Some(params[2].1);
            self.paramaters.c.uncertainty = Some(params[2].2);

            self.fit_points = x.iter().zip(y.iter()).map(|(&x, &y)| [x, y]).collect();
            self.fit_report = fit_report;

            Ok(())
        })
    }

    pub fn evaluate(&self, x: f64) -> f64 {
        self.paramaters.a.value.unwrap_or(0.0) * x.powi(2)
            + self.paramaters.b.value.unwrap_or(0.0) * x
            + self.paramaters.c.value.unwrap_or(0.0)
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        // add menu button for the fit report
        ui.horizontal(|ui| {
            if let Some(a) = &self.paramaters.a.value {
                ui.label(format!(
                    "a: {:.3} ± {:.3}",
                    a,
                    self.paramaters.a.uncertainty.unwrap_or(0.0)
                ));
            }
            ui.separator();
            if let Some(b) = &self.paramaters.b.value {
                ui.label(format!(
                    "b: {:.3} ± {:.3}",
                    b,
                    self.paramaters.b.uncertainty.unwrap_or(0.0)
                ));
            }
            ui.separator();
            if let Some(c) = &self.paramaters.c.value {
                ui.label(format!(
                    "c: {:.3} ± {:.3}",
                    c,
                    self.paramaters.c.uncertainty.unwrap_or(0.0)
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
