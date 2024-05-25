use super::gaussian::GaussianFitter;
use super::linear::LinearFitter;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum FitModel {
    Gaussian(Vec<f64>), // put the inital peak locations in here
    Linear,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum FitResult {
    Gaussian(GaussianFitter),
    Linear(LinearFitter),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BackgroundFitter {
    pub x_data: Vec<f64>,
    pub y_data: Vec<f64>,
    pub model: FitModel,
    pub result: Option<FitResult>,
}

impl BackgroundFitter {
    pub fn new(x_data: Vec<f64>, y_data: Vec<f64>, model: FitModel) -> Self {
        BackgroundFitter {
            x_data,
            y_data,
            model,
            result: None,
        }
    }

    pub fn fit(&mut self) {
        match self.model {
            FitModel::Gaussian(_) => {
                eprintln!("Gaussian background fitting not yet implemented");
            }
            FitModel::Linear => {
                // check x and y data are the same length
                if self.x_data.len() != self.y_data.len() {
                    eprintln!("x_data and y_data must have the same length");
                    return;
                }

                let mut linear_fitter = LinearFitter::new(self.x_data.clone(), self.y_data.clone());
                linear_fitter.perform_linear_fit();

                self.result = Some(FitResult::Linear(linear_fitter));
            }
        }
    }

    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi) {
        // Draw the fit lines
        if let Some(fit) = &self.result {
            match fit {
                FitResult::Gaussian(fit) => {
                    let color = egui::Color32::from_rgb(255, 0, 255); // purple
                    fit.draw(plot_ui, color);
                }

                FitResult::Linear(fit) => {
                    let color = egui::Color32::GREEN;
                    fit.draw(plot_ui, color);
                }
            }
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Fitter {
    pub x_data: Vec<f64>,
    pub y_data: Vec<f64>,
    pub y_err: Option<Vec<f64>>,
    pub background: Option<BackgroundFitter>,
    pub model: FitModel,
    pub result: Option<FitResult>,
}

impl Fitter {
    // Constructor to create a new Fitter with empty data and specified model
    pub fn new(model: FitModel, background: Option<BackgroundFitter>) -> Self {
        Fitter {
            x_data: Vec::new(),
            y_data: Vec::new(),
            y_err: None,
            background,
            model,
            result: None,
        }
    }

    pub fn fit(&mut self) -> Result<(), &'static str> {
        // Perform the fit based on the model
        match &self.model {
            FitModel::Gaussian(peak_markers) => {
                // Perform Gaussian fit
                let mut fit = GaussianFitter::new(
                    self.x_data.clone(),
                    self.y_data.clone(),
                    peak_markers.clone(),
                );

                fit.multi_gauss_fit();

                self.result = Some(FitResult::Gaussian(fit));
            }

            FitModel::Linear => {
                // Perform Linear fit
                let mut fit = LinearFitter::new(self.x_data.clone(), self.y_data.clone());

                fit.perform_linear_fit();

                self.result = Some(FitResult::Linear(fit));
            }
        }

        Ok(())
    }

    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi) {
        // Draw the fit lines
        if let Some(fit) = &self.result {
            match fit {
                FitResult::Gaussian(fit) => {
                    let color = egui::Color32::from_rgb(255, 0, 255); // purple
                    fit.draw(plot_ui, color);
                }

                FitResult::Linear(fit) => {
                    let color = egui::Color32::GREEN;
                    fit.draw(plot_ui, color);
                }
            }
        }
    }
}
