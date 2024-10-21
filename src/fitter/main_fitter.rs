use crate::egui_plot_stuff::egui_line::EguiLine;
use super::models::gaussian::{Background, GaussianFitter};
use super::common::{Data, Value};
use super::models::linear::LinearFitter;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum Model {
    Gaussian(Vec<f64>, bool, bool, f64), // initial peak locations, free sigma, free position, bin width
    Linear, // slope, intercept, vary parameters
    None,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum FitResult {
    Gaussian(GaussianFitter),
    Linear(LinearFitter),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Fitter {
    pub name: String,
    pub data: Data,
    pub model: Model,
    pub background: Model,
    pub background_result: Option<FitResult>,
    pub result: Option<FitResult>,
    pub background_line: EguiLine,
    pub composition_line: EguiLine,
    pub decomposition_lines: Vec<EguiLine>,
}

impl Fitter {
    // Constructor to create a new Fitter with empty data and specified model
    pub fn new(model: Model, data: Data) -> Self {
        Fitter {
            name: "Fit".to_string(),
            data,
            model,
            background: Model::Linear,
            background_result: None,
            result: None,
            background_line: EguiLine::new(egui::Color32::GREEN),
            composition_line: EguiLine::new(egui::Color32::DARK_BLUE),
            decomposition_lines: Vec::new(),
        }
    }

    pub fn fit(&mut self) {
        // Perform the fit based on the model
        match &self.model {
            Model::Gaussian(peak_markers, equal_sigma, free_position, bin_width) => {
                // Extract background parameters if the background model is fitted
                let background_parameters = match &self.background_result {
                    Some(FitResult::Linear(fit)) => {
                        // Extract background parameters and fit points from LinearFitter
                        Some(Background {
                            model: Model::Linear,
                            parameters: Some(vec![
                                fit.slope.clone().unwrap(),
                                fit.intercept.clone().unwrap(),
                            ]),
                            varying: Some(vec![fit.vary_parameters.0, fit.vary_parameters.1]),
                            fit_points: fit.fit_points.clone(),
                        })
                    }
                    Some(FitResult::Gaussian(_)) => {
                        None // Background is Gaussian, not relevant here
                    }
                    None => {
                        None // No background result present
                    }
                };

                // Create the GaussianFitter instance with the pre-existing background parameters
                let mut fit = GaussianFitter::new(
                    self.data.clone(),
                    peak_markers.clone(),
                    self.background.clone(),
                    background_parameters,  // Pass background parameters if they exist
                    *equal_sigma,
                    *free_position,
                    *bin_width,
                );

                // Perform the fit using lmfit
                match fit.lmfit() {
                    Ok(_) => {
                        // Process Gaussian fit result and update lines
                        self.composition_line.points = fit.composition_points.clone();
                        self.result = Some(FitResult::Gaussian(fit));
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            Model::Linear => {
                // Perform Linear fit
                let mut fit = LinearFitter::new(self.data.clone());
                match fit.lmfit() {
                    Ok(_) => {
                        self.composition_line.points = fit.fit_points.clone();
                        self.result = Some(FitResult::Linear(fit));
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            Model::None => {
                // No fitting required for 'None'
            }
        }
    }


    pub fn fit_background(&mut self) {
        log::info!("Fitting background");
        // Perform the fit based on the background model
        match &self.background {
            Model::Gaussian(_, _, _, _) => {
                // Add Gaussian background fit logic here if needed
            }
            Model::Linear => {
                log::info!("Fitting background with a linear line using `lmfit`.");
                // Perform Linear fit for background
                let mut fit = LinearFitter::new(self.data.clone());
                log::info!("{:?}", fit);
                match fit.lmfit() {
                    Ok(_) => {
                        self.background_line.points = fit.fit_points.clone();
                        self.background_result = Some(FitResult::Linear(fit));
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            Model::None => {
                // No background fitting required for 'None'
                log::info!("No background fitting required for 'None'");
            }
        }

        log::info!("Finished fitting background");
    }

    pub fn set_background_color(&mut self, color: egui::Color32) {
        self.background_line.color = color;
    }

    pub fn set_composition_color(&mut self, color: egui::Color32) {
        self.composition_line.color = color;
    }

    pub fn set_decomposition_color(&mut self, color: egui::Color32) {
        for line in &mut self.decomposition_lines {
            line.color = color;
        }
    }

    pub fn show_decomposition(&mut self, show: bool) {
        for line in &mut self.decomposition_lines {
            line.draw = show;
        }
    }

    pub fn show_composition(&mut self, show: bool) {
        self.composition_line.draw = show;
    }

    pub fn show_background(&mut self, show: bool) {
        self.background_line.draw = show;
    }

    pub fn set_name(&mut self, name: String) {
        self.composition_line.name = format!("{}-Composition", name);
        
        for (i, line) in self.decomposition_lines.iter_mut().enumerate() {
            line.name = format!("{}-Peak {}", name, i);
        }

         self.background_line.name = format!("{}-Background", name);
    }

    pub fn lines_ui(&mut self, ui: &mut egui::Ui) {
        self.background_line.menu_button(ui);
        
        self.composition_line.menu_button(ui);

        for line in &mut self.decomposition_lines {
            line.menu_button(ui);
        }

        ui.separator();
    }

    // Draw the background, decomposition, and composition lines
    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi) {
        for line in &self.decomposition_lines {
            line.draw(plot_ui);
        }

        self.composition_line.draw(plot_ui);

        self.background_line.draw(plot_ui);
    }

    // Set the log_y flag for all lines
    pub fn set_log(&mut self, log_y: bool, log_x: bool) {
        for line in &mut self.decomposition_lines {
            line.log_y = log_y;
            line.log_x = log_x;
        }

        self.composition_line.log_y = log_y;
        self.composition_line.log_x = log_x;

        self.background_line.log_y = log_y;
        self.background_line.log_x = log_x;
    }
}