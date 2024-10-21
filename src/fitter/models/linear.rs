use pyo3::{prelude::*, types::PyModule};
use crate::fitter::common::{Data, Value};


#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LinearFitter {
    pub data: Data,
    pub initial_guess: (f64, f64),
    pub vary_parameters: (bool, bool),
    pub slope: Option<Value>,
    pub intercept: Option<Value>,
    pub fit_points: Vec<[f64; 2]>,
    pub fit_report: String,
}

impl LinearFitter {
    pub fn new(data: Data) -> Self {
        LinearFitter {
            data,
            initial_guess: (0.0, 0.0),
            vary_parameters: (true, true),
            slope: None,
            intercept: None,
            fit_points: Vec::new(),
            fit_report: String::new(),
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

            // Define the Python code as a module
            let code = r#"

def LinearFit(x_data: list, y_data: list, initial_guess: tuple = (0.0, 0.0), vary: tuple = (True, True)):
    import lmfit
    import numpy as np
    
    model = lmfit.models.LinearModel()
    params = model.make_params(slope=initial_guess[0], intercept=initial_guess[1])
    params['slope'].set(vary=vary[0])
    params['intercept'].set(vary=vary[1])

    result = model.fit(y_data, params, x=x_data)

    print(result.fit_report())

    # Extract Parameters
    slope = float(result.params['slope'].value)
    slope_err = float(result.params['slope'].stderr)
    intercept = float(result.params['intercept'].value)
    intercept_err = float(result.params['intercept'].stderr)

    params = [
        ('slope', slope, slope_err),
        ('intercept', intercept, intercept_err)
    ]

    x = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
    y = result.eval(x=x)

    fit_report = str(result.fit_report())

    print(fit_report)

    return params, x, y, fit_report
"#;

            // Compile the Python code into a module
            let module =
                PyModule::from_code_bound(py, code, "linear.py", "linear")?;

            let x_data = self.data.x.clone();
            let y_data = self.data.y.clone();
            let initial_guess = self.initial_guess;
            let vary = self.vary_parameters;

            let result = module.getattr("LinearFit")?.call1((x_data, y_data, initial_guess, vary))?;

            let params = result.get_item(0)?.extract::<Vec<(String, f64, f64)>>()?;
            let x = result.get_item(1)?.extract::<Vec<f64>>()?;
            let y = result.get_item(2)?.extract::<Vec<f64>>()?;
            let fit_report = result.get_item(3)?.extract::<String>()?;

            self.slope = Some(Value::new(params[0].0.clone(), params[0].1, params[0].2));
            self.intercept = Some(Value::new(params[1].0.clone(), params[1].1, params[1].2));
            self.fit_points = x.iter().zip(y.iter()).map(|(&x, &y)| [x, y]).collect();
            self.fit_report = fit_report;

            Ok(())
        })
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Initial Guess:");
            ui.add(egui::DragValue::new(&mut self.initial_guess.0).speed(0.1).prefix("Slope: "));
            ui.checkbox(&mut self.vary_parameters.0, "Vary");

            ui.separator();

            ui.add(egui::DragValue::new(&mut self.initial_guess.1).speed(0.1).prefix("Intercept: "));
            ui.checkbox(&mut self.vary_parameters.1, "Vary");
        });

        ui.horizontal(|ui| {
            if ui.button("Fit").clicked() {
                match self.lmfit() {
                    Ok(_) => {
                        log::info!("Successfully fit data with a linear line.");
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            ui.label("Best Fit:");
            ui.monospace(format!("y = {:.3}x + {:.3}", self.slope.as_ref().map_or(0.0, |v| v.value), self.intercept.as_ref().map_or(0.0, |v| v.value)));
        });

        ui.separator();

        ui.label("Fit Report:");
        ui.monospace(self.fit_report.clone());
    }
}