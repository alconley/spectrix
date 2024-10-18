use pyo3::{prelude::*, types::PyModule};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum BackgroundModels {
    None,
    Linear,
    Quadratic,
    Exponential,
    Power,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GaussianFitter {
    pub x_data: Vec<f64>,
    pub y_data: Vec<f64>,
    pub peak_markers: Vec<f64>,
    pub background_model: BackgroundModels,
    pub equal_stdev: bool,
    pub free_position: bool,
    pub bin_width: f64,
}

impl GaussianFitter {
    pub fn new(
        x_data: Vec<f64>,
        y_data: Vec<f64>,
        peak_markers: Vec<f64>,
        background_model: BackgroundModels,
        equal_stdev: bool,
        free_position: bool,
        bin_width: f64,
    ) -> Self {
        Self {
            x_data,
            y_data,
            peak_markers,
            background_model,
            equal_stdev,
            free_position,
            bin_width,
        }
    }

    pub fn fit_with_lmfit(&mut self) -> PyResult<()> {
        Python::with_gil(|py| {
            let sys = py.import_bound("sys")?;
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

# Multiple Gaussian fitting function
def MultipleGaussianFit(x_data: list, y_data: list, peak_markers: list, equal_sigma:bool=True, free_position:bool=True, background_type:str='linear'):
    
    # Initialize the model with or without a background based on the flag
    if background_type == 'linear':
        model = lmfit.models.LinearModel(prefix='bg_')
        params = model.make_params(slope=0, intercept=0)  # Initial guesses for linear background
    elif background_type == 'quadratic':
        model = lmfit.models.QuadraticModel(prefix='bg_')
        params = model.make_params(a=0, b=1, c=0)  # Initial guesses for quadratic background
    elif background_type == 'exponential':
        model = lmfit.models.ExponentialModel(prefix='bg_')
        params = model.make_params(amplitude=1, decay=100)  # Initial guesses for exponential background
    elif background_type == 'powerlaw':
        model = lmfit.models.PowerLawModel(prefix='bg_')
        params = model.make_params(amplitude=1, exponent=-0.5)  # Initial guesses for power-law background
    elif background_type is None:
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

    print(result.fit_report())

    # Create an array of (name, value, uncertainty) tuples
    param_array = [(param, result.params[param].value, result.params[param].stderr) for param in result.params]
    
    # Return the fit result
    return param_array
"#;

            // Compile the Python code into a module
            let module =
                PyModule::from_code_bound(py, code, "gaussian.py", "gaussian")?;

            let x_data = self.x_data.clone();
            let y_data = self.y_data.clone();
            let peak_markers = self.peak_markers.clone();
            let equal_sigma = self.equal_stdev;
            let free_position = self.free_position;
            let background_model = match self.background_model {
                BackgroundModels::None => "None",
                BackgroundModels::Linear => "linear",
                BackgroundModels::Quadratic => "quadratic",
                BackgroundModels::Exponential => "exponential",
                BackgroundModels::Power => "powerlaw",
            };

            let result = module.getattr("MultipleGaussianFit")?.call1((x_data, y_data, peak_markers, equal_sigma, free_position, background_model))?;
            println!("result: {:?}", result);

            // [(String, f64, f64), ...]

            let length: usize = result.len()?;
            for i in 0..length {
                let item = result.get_item(i)?;
                let name: String = item.get_item(0)?.extract()?;
                let value: f64 = item.get_item(1)?.extract()?;
                let uncertainty: f64 = item.get_item(2)?.extract()?;

                println!("name: {}, value: {}, uncertainty: {}", name, value, uncertainty);
            }


            Ok(())
        })
    }
    
}