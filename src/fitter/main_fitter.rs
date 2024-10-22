use crate::egui_plot_stuff::egui_line::EguiLine;
// use super::models::gaussian::{Background, GaussianFitter};
use super::common::Data;
use super::models::exponential::{ExponentialFitter, ExponentialParameters};
use super::models::linear::{LinearFitter, LinearParameters};
use super::models::powerlaw::{PowerLawFitter, PowerLawParameters};
use super::models::quadratic::{QuadraticFitter, QuadraticParameters};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum FitModel {
    Gaussian(Vec<f64>, bool, bool, f64), // initial peak locations, free sigma, free position, bin width
    None,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum FitResult {
    // Gaussian(GaussianFitter),
}

#[derive(PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum BackgroundModel {
    Linear(LinearParameters),
    Quadratic(QuadraticParameters),
    PowerLaw(PowerLawParameters),
    Exponential(ExponentialParameters),
    None,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum BackgroundResult {
    Linear(LinearFitter),
    Quadratic(QuadraticFitter),
    PowerLaw(PowerLawFitter),
    Exponential(ExponentialFitter),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Fitter {
    pub name: String,

    pub data: Data,

    pub background_model: BackgroundModel,
    pub background_result: Option<BackgroundResult>,

    pub fit_model: FitModel,
    pub fit_result: Option<FitResult>,

    pub background_line: EguiLine,
    pub composition_line: EguiLine,
    pub decomposition_lines: Vec<EguiLine>,
}

impl Fitter {
    // Constructor to create a new Fitter with empty data and specified model
    pub fn new(data: Data) -> Self {
        Fitter {
            name: "Fit".to_string(),

            data,

            background_model: BackgroundModel::None,
            background_result: None,

            fit_model: FitModel::None,
            fit_result: None,

            background_line: EguiLine::new(egui::Color32::GREEN),
            composition_line: EguiLine::new(egui::Color32::DARK_BLUE),
            decomposition_lines: Vec::new(),
        }
    }

    pub fn fit_background(&mut self) {
        log::info!("Fitting background");
        match &self.background_model {
            BackgroundModel::Linear(params) => {
                let mut fit = LinearFitter::new(self.data.clone());

                // Copy the parameters from the background model to the LinearFitter
                fit.paramaters = params.clone(); // Ensure LinearParameters are used

                // Perform the fit
                match fit.lmfit() {
                    Ok(_) => {
                        self.background_line.points = fit.fit_points.clone();
                        self.background_result = Some(BackgroundResult::Linear(fit));
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            BackgroundModel::Quadratic(params) => {
                let mut fit = QuadraticFitter::new(self.data.clone());

                // Copy the parameters from the background model to the QuadraticFitter
                fit.paramaters = params.clone(); // Ensure QuadraticParameters are used

                // Perform the fit
                match fit.lmfit() {
                    Ok(_) => {
                        self.background_line.points = fit.fit_points.clone();
                        self.background_result = Some(BackgroundResult::Quadratic(fit));
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            BackgroundModel::PowerLaw(params) => {
                let mut fit = PowerLawFitter::new(self.data.clone());

                // Copy the parameters from the background model to the PowerLawFitter
                fit.paramaters = params.clone(); // Ensure PowerLawParameters are used

                // Perform the fit
                match fit.lmfit() {
                    Ok(_) => {
                        self.background_line.points = fit.fit_points.clone();
                        self.background_result = Some(BackgroundResult::PowerLaw(fit));
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            BackgroundModel::Exponential(params) => {
                let mut fit = ExponentialFitter::new(self.data.clone());

                // Copy the parameters from the background model to the ExponentialFitter
                fit.paramaters = params.clone(); // Ensure ExponentialParameters are used

                // Perform the fit
                match fit.lmfit() {
                    Ok(_) => {
                        self.background_line.points = fit.fit_points.clone();
                        self.background_result = Some(BackgroundResult::Exponential(fit));
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            BackgroundModel::None => {
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

    pub fn fit_result_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing(self.name.clone(), |ui| {
            egui::ScrollArea::vertical()
                .min_scrolled_height(300.0)
                .show(ui, |ui| {
                    if let Some(fit_result) = &self.fit_result {
                        self.composition_line.menu_button(ui);
                    }

                    for line in &mut self.decomposition_lines {
                        line.menu_button(ui);
                    }

                    ui.separator();

                    if let Some(background_result) = &self.background_result {
                        ui.label("Background");
                        match background_result {
                            BackgroundResult::Linear(fit) => {
                                fit.ui(ui);
                            }
                            BackgroundResult::Quadratic(fit) => {
                                fit.ui(ui);
                            }
                            BackgroundResult::PowerLaw(fit) => {
                                fit.ui(ui);
                            }
                            BackgroundResult::Exponential(fit) => {
                                fit.ui(ui);
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.label("Line");
                            self.background_line.menu_button(ui);
                        });
                    }
                });
        });
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
