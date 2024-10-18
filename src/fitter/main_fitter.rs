// use super::models::double_exponential::DoubleExponentialFitter;
// use super::models::exponential::ExponentialFitter;
// use super::models::gaussian::GaussianFitter;
// use super::models::polynomial::PolynomialFitter;

use crate::egui_plot_stuff::egui_line::EguiLine;
use super::models::gaussian::{GaussianFitter, BackgroundModels};

// use crate::fitter::background_fitter::BackgroundFitter;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum FitModel {
    Gaussian(Vec<f64>, bool, bool, f64), // put the initial peak locations in here, free sigma, free position
    // Polynomial(usize), // the degree of the polynomial: 1 for linear, 2 for quadratic, etc.
    // Exponential(f64),  // the initial guess for the exponential decay constant
    // DoubleExponential(f64, f64), // the initial guess for the exponential decay constants
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum FitResult {
    Gaussian(GaussianFitter),
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Fitter {
    pub name: String,
    pub x_data: Vec<f64>,
    pub y_data: Vec<f64>,
    pub y_err: Option<Vec<f64>>,
    pub model: FitModel,
    pub background_model: BackgroundModels,
    pub result: Option<FitResult>,
    pub decomposition_lines: Vec<EguiLine>,
    pub composition_line: EguiLine,
}

impl Fitter {
    // Constructor to create a new Fitter with empty data and specified model
    pub fn new(model: FitModel) -> Self {
        Fitter {
            name: "Fit".to_string(),
            x_data: Vec::new(),
            y_data: Vec::new(),
            y_err: None,
            model,
            background_model: BackgroundModels::Linear,
            result: None,
            decomposition_lines: Vec::new(),
            composition_line: EguiLine::default(),
        }
    }

    pub fn get_peak_markers(&self) -> Vec<f64> {
        if let Some(FitResult::Gaussian(fit)) = &self.result {
            fit.peak_markers.clone()
        } else if let FitModel::Gaussian(peak_markers, _, _, _) = &self.model {
            peak_markers.clone()
        } else {
            Vec::new()
        }
    }

    pub fn fit(&mut self) {

        // Perform the fit based on the model
        match &self.model {
            FitModel::Gaussian(peak_markers, equal_sigma, free_position, bin_width ) => {
                // Perform Gaussian fit
                let mut fit = GaussianFitter::new(
                    self.x_data.clone(),
                    self.y_data.clone(),
                    peak_markers.clone(),
                    self.background_model.clone(),
                    *equal_sigma,
                    *free_position,
                    *bin_width,
                );

                fit.fit_with_lmfit();
            }
        }
    }

    // pub fn fitter_stats(&self, ui: &mut egui::Ui) {
    //     if let Some(fit) = &self.result {
    //         match fit {
    //             FitResult::Gaussian(fit) => fit.fit_params_ui(ui),
    //         }
    //     }
    // }

    // pub fn set_background_color(&mut self, color: egui::Color32) {
    //     if let Some(background) = &mut self.background {
    //         background.fit_line.color = color;
    //     }
    // }

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

    // pub fn show_background(&mut self, show: bool) {
    //     if let Some(background) = &mut self.background {
    //         background.fit_line.draw = show;
    //     }
    // }

    pub fn set_name(&mut self, name: String) {
        self.composition_line.name = format!("{}-Composition", name);

        for (i, line) in self.decomposition_lines.iter_mut().enumerate() {
            line.name = format!("{}-Peak {}", name, i);
        }

        // if let Some(background) = &mut self.background {
        //     background.fit_line.name = format!("{}-Background", name);
        // }
    }

    pub fn lines_ui(&mut self, ui: &mut egui::Ui) {
        // if let Some(background) = &mut self.background {
        //     background.fit_line.menu_button(ui);
        // }

        self.composition_line.menu_button(ui);

        for line in &mut self.decomposition_lines {
            line.menu_button(ui);
        }

        ui.separator();
    }

    // Draw the background, decomposition, and composition lines
    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi) {
        // Draw the decomposition lines
        for line in &self.decomposition_lines {
            line.draw(plot_ui);
        }

        // // Draw the background if it exists
        // if let Some(background) = &self.background {
        //     background.draw(plot_ui);
        // }

        // Draw the composition line
        self.composition_line.draw(plot_ui);
    }

    // Set the log_y flag for all lines
    pub fn set_log(&mut self, log_y: bool, log_x: bool) {
        for line in &mut self.decomposition_lines {
            line.log_y = log_y;
            line.log_x = log_x;
        }

        // if let Some(background) = &mut self.background {
        //     background.fit_line.log_y = log_y;
        //     background.fit_line.log_x = log_x;
        // }

        self.composition_line.log_y = log_y;
        self.composition_line.log_x = log_x;
    }
}
