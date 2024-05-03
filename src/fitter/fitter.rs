use std::collections::HashMap;
use super::gaussian::GaussianFitter;
pub enum FitModel {
    Gaussian(Vec<f64>), // put the inital peak locations in here
    // Linear,
    // Exponential,
    // DoubleExponential,
}

pub struct Fitter {
    pub x_data: Vec<f64>,
    pub y_data: Vec<f64>,
    pub y_err: Option<Vec<f64>>,
    pub model: FitModel,
    pub model_result: HashMap<String, f64>
}

impl Fitter {
    // Constructor to create a new Fitter with empty data and specified model
    pub fn new(model: FitModel) -> Self {
        Fitter {
            x_data: Vec::new(),
            y_data: Vec::new(),
            y_err: None,
            model,
            model_result: HashMap::new(),
        }
    }

    pub fn fit(&mut self) -> Result<(), &'static str> {
        // Perform the fit based on the model
        match &self.model {
            FitModel::Gaussian(peak_markers) => {
                // Perform Gaussian fit
                let mut fit = GaussianFitter::new(self.x_data.clone(), self.y_data.clone(), peak_markers.clone());
                
                fit.multi_gauss_fit();
            }
        }

        Ok(())
    }

    
}