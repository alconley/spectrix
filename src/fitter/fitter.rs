use super::gaussian::GaussianFitter;
use super::linear::LinearFitter;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum FitModel {
    Gaussian(Vec<f64>), // put the initial peak locations in here
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
                // Check x and y data are the same length
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

    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi, color: egui::Color32) {
        // Draw the fit lines
        if let Some(fit) = &self.result {
            match fit {
                FitResult::Gaussian(fit) => {
                    fit.draw(plot_ui, color);
                }

                FitResult::Linear(fit) => {
                    fit.draw(plot_ui, color);
                }
            }
        }
    }

    pub fn get_background(&self, x_data: &[f64]) -> Option<Vec<f64>> {
        if let Some(FitResult::Linear(fitter)) = &self.result {
            Some(fitter.calculate_background(x_data))
        } else {
            None
        }
    }

    pub fn get_slope_intercept(&self) -> Option<(f64, f64)> {
        if let Some(FitResult::Linear(fitter)) = &self.result {
            fitter
                .fit_params
                .as_ref()
                .map(|params| (params.slope, params.intercept))
        } else {
            None
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

    fn subtract_background(&self) -> Vec<f64> {
        if let Some(bg_fitter) = &self.background {
            if let Some(bg_result) = bg_fitter.get_background(&self.x_data) {
                self.y_data
                    .iter()
                    .zip(bg_result.iter())
                    .map(|(y, bg)| y - bg)
                    .collect()
            } else {
                self.y_data.clone()
            }
        } else {
            self.y_data.clone()
        }
    }

    pub fn get_peak_markers(&self) -> Vec<f64> {
        if let Some(FitResult::Gaussian(fit)) = &self.result {
            fit.peak_markers.clone()
        } else if let FitModel::Gaussian(peak_markers) = &self.model {
            peak_markers.clone()
        } else {
            Vec::new()
        }
    }

    pub fn fit(&mut self) {
        // Fit the background if it's defined and there is no background result
        if let Some(bg_fitter) = &mut self.background {
            if bg_fitter.result.is_none() {
                bg_fitter.fit();
            }
        }

        // Perform the background subtraction if necessary
        let y_data_corrected = self.subtract_background();

        // Perform the fit based on the model
        match &self.model {
            FitModel::Gaussian(peak_markers) => {
                // Perform Gaussian fit
                let mut fit = GaussianFitter::new(
                    self.x_data.clone(),
                    y_data_corrected,
                    peak_markers.clone(),
                );

                fit.multi_gauss_fit();

                self.result = Some(FitResult::Gaussian(fit));
            }

            FitModel::Linear => {
                // Perform Linear fit
                let mut fit = LinearFitter::new(self.x_data.clone(), y_data_corrected);

                fit.perform_linear_fit();

                self.result = Some(FitResult::Linear(fit));
            }
        }
    }

    pub fn draw(
        &self,
        plot_ui: &mut egui_plot::PlotUi,
        fit_color: egui::Color32,
        background_color: egui::Color32,
        convoluted_color: egui::Color32,
    ) {
        // Draw the fit lines
        if let Some(fit) = &self.result {
            match fit {
                FitResult::Gaussian(fit) => {
                    fit.draw(plot_ui, fit_color);

                    if let Some(bg_fitter) = &self.background {
                        // Draw the background fit
                        bg_fitter.draw(plot_ui, background_color);

                        if let Some((slope, intercept)) = bg_fitter.get_slope_intercept() {
                            let convoluted_points = fit
                                .calculate_convoluted_fit_points_with_linear_background(
                                    slope, intercept,
                                );
                            let line = egui_plot::Line::new(egui_plot::PlotPoints::Owned(
                                convoluted_points,
                            ))
                            .color(convoluted_color)
                            .stroke(egui::Stroke::new(1.0, convoluted_color));
                            plot_ui.line(line);
                        }
                    }
                }

                FitResult::Linear(fit) => {
                    fit.draw(plot_ui, fit_color);
                }
            }
        }
    }
}
