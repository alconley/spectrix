use super::gaussian::GaussianFitter;
use super::models::double_exponential::DoubleExponentialFitter;
use super::models::exponential::ExponentialFitter;
use super::models::polynomial::PolynomialFitter;

use crate::egui_plot_stuff::egui_line::EguiLine;

use crate::fitter::background_fitter::BackgroundFitter;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum FitModel {
    Gaussian(Vec<f64>),          // put the initial peak locations in here
    Polynomial(usize), // the degree of the polynomial: 1 for linear, 2 for quadratic, etc.
    Exponential(f64),  // the initial guess for the exponential decay constant
    DoubleExponential(f64, f64), // the initial guess for the exponential decay constants
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum FitResult {
    Gaussian(GaussianFitter),
    Polynomial(PolynomialFitter),
    Exponential(ExponentialFitter),
    DoubleExponential(DoubleExponentialFitter),
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Fitter {
    pub name: String,
    pub x_data: Vec<f64>,
    pub y_data: Vec<f64>,
    pub y_err: Option<Vec<f64>>,
    pub background: Option<BackgroundFitter>,
    pub model: FitModel,
    pub result: Option<FitResult>,
    pub decomposition_lines: Vec<EguiLine>,
    pub composition_line: EguiLine,
}

impl Fitter {
    // Constructor to create a new Fitter with empty data and specified model
    pub fn new(model: FitModel, background: Option<BackgroundFitter>) -> Self {
        Fitter {
            name: "Fit".to_string(),
            x_data: Vec::new(),
            y_data: Vec::new(),
            y_err: None,
            background,
            model,
            result: None,
            decomposition_lines: Vec::new(),
            composition_line: EguiLine::default(),
        }
    }

    fn subtract_background(&self) -> Vec<f64> {
        if let Some(bg_fitter) = &self.background {
            match &bg_fitter.result {
                Some(FitResult::Polynomial(fitter)) => {
                    fitter.subtract_background(self.x_data.clone(), self.y_data.clone())
                }
                Some(FitResult::Exponential(fitter)) => {
                    fitter.subtract_background(self.x_data.clone(), self.y_data.clone())
                }
                Some(FitResult::DoubleExponential(fitter)) => {
                    fitter.subtract_background(self.x_data.clone(), self.y_data.clone())
                }
                _ => self.y_data.clone(),
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

                // get the fit_lines and store them in the decomposition_lines
                let decomposition_default_color = egui::Color32::from_rgb(255, 0, 255);
                if let Some(fit_lines) = &fit.fit_lines {
                    for (i, line) in fit_lines.iter().enumerate() {
                        let mut fit_line = EguiLine::new(decomposition_default_color);
                        fit_line.name = format!("Peak {}", i);

                        fit_line.points.clone_from(line);
                        fit_line.name_in_legend = false;
                        self.decomposition_lines.push(fit_line);
                    }
                }

                // calculate the composition line
                if let Some(background) = &self.background {
                    match &background.result {
                        Some(FitResult::Polynomial(fitter)) => {
                            if let Some(coef) = &fitter.coefficients {
                                let composition_points =
                                    fit.composition_fit_points_polynomial(coef.clone());
                                let mut line = EguiLine::new(egui::Color32::BLUE);
                                line.name = "Composition".to_string();
                                line.points = composition_points;
                                self.composition_line = line;
                            }
                        }
                        Some(FitResult::Exponential(fitter)) => {
                            if let Some(coef) = &fitter.coefficients {
                                let a = coef.a.value;
                                let b = coef.b.value;
                                let composition_points =
                                    fit.composition_fit_points_exponential(a, b);
                                let mut line = EguiLine::new(egui::Color32::BLUE);
                                line.name = "Composition".to_string();
                                line.points = composition_points;
                                self.composition_line = line;
                            }
                        }
                        Some(FitResult::DoubleExponential(fitter)) => {
                            if let Some(coef) = &fitter.coefficients {
                                let a = coef.a.value;
                                let b = coef.b.value;
                                let c = coef.c.value;
                                let d = coef.d.value;
                                let composition_points =
                                    fit.composition_fit_points_double_exponential(a, b, c, d);
                                let mut line = EguiLine::new(egui::Color32::BLUE);
                                line.name = "Composition".to_string();
                                line.points = composition_points;
                                self.composition_line = line;
                            }
                        }
                        _ => {}
                    }
                    // if let Some((slope, intercept)) = background.get_slope_intercept() {
                    //     let composition_points =
                    //         fit.composition_fit_points_linear_bg(slope, intercept);

                    // let mut line = EguiLine::new(egui::Color32::BLUE);
                    // line.name = "Composition".to_string();
                    // line.points = composition_points;
                    // self.composition_line = line;
                    // }
                }

                self.result = Some(FitResult::Gaussian(fit));
            }

            FitModel::Polynomial(degree) => {
                // Perform Polynomial fit
                let mut fit = PolynomialFitter::new(*degree);
                fit.x_data.clone_from(&self.x_data);
                fit.y_data.clone_from(&y_data_corrected);
                fit.fit();

                self.result = Some(FitResult::Polynomial(fit));
            }

            FitModel::Exponential(initial_b_guess) => {
                // Perform Exponential fit
                let mut fit = ExponentialFitter::new(*initial_b_guess);
                fit.x_data.clone_from(&self.x_data);
                fit.y_data.clone_from(&y_data_corrected);
                fit.fit();

                self.result = Some(FitResult::Exponential(fit));
            }

            FitModel::DoubleExponential(initial_b_guess, initial_d_guess) => {
                // Perform Double Exponential fit
                let mut fit = DoubleExponentialFitter::new(*initial_b_guess, *initial_d_guess);
                fit.x_data.clone_from(&self.x_data);
                fit.y_data.clone_from(&y_data_corrected);
                fit.fit();

                self.result = Some(FitResult::DoubleExponential(fit));
            }
        }
    }

    pub fn fitter_stats(&self, ui: &mut egui::Ui) {
        if let Some(fit) = &self.result {
            match fit {
                FitResult::Gaussian(fit) => fit.fit_params_ui(ui),
                FitResult::Polynomial(fit) => fit.fit_params_ui(ui),
                FitResult::Exponential(fit) => fit.fit_params_ui(ui),
                FitResult::DoubleExponential(fit) => fit.fit_params_ui(ui),
            }
        }
    }

    pub fn set_background_color(&mut self, color: egui::Color32) {
        if let Some(background) = &mut self.background {
            background.fit_line.color = color;
        }
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
        if let Some(background) = &mut self.background {
            background.fit_line.draw = show;
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.composition_line.name = format!("{}-Composition", name);

        for (i, line) in self.decomposition_lines.iter_mut().enumerate() {
            line.name = format!("{}-Peak {}", name, i);
        }

        if let Some(background) = &mut self.background {
            background.fit_line.name = format!("{}-Background", name);
        }
    }

    pub fn lines_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(background) = &mut self.background {
            background.fit_line.menu_button(ui);
        }

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

        // Draw the background if it exists
        if let Some(background) = &self.background {
            background.draw(plot_ui);
        }

        // Draw the composition line
        self.composition_line.draw(plot_ui);
    }

    // Set the log_y flag for all lines
    pub fn set_log(&mut self, log_y: bool, log_x: bool) {
        for line in &mut self.decomposition_lines {
            line.log_y = log_y;
            line.log_x = log_x;
        }

        if let Some(background) = &mut self.background {
            background.fit_line.log_y = log_y;
            background.fit_line.log_x = log_x;
        }

        self.composition_line.log_y = log_y;
        self.composition_line.log_x = log_x;
    }
}
