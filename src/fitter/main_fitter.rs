use super::common::Data;
use super::models::exponential::{ExponentialFitter, ExponentialParameters};
use super::models::gaussian::GaussianFitter;
use super::models::linear::{LinearFitter, LinearParameters};
use super::models::powerlaw::{PowerLawFitter, PowerLawParameters};
use super::models::quadratic::{QuadraticFitter, QuadraticParameters};
use crate::egui_plot_stuff::egui_line::EguiLine;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum FitModel {
    Gaussian(Vec<f64>, bool, bool, f64), // initial peak locations, free sigma, free position, bin width
    None,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum FitResult {
    Gaussian(GaussianFitter),
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

impl BackgroundResult {
    pub fn get_fit_points(&self) -> Vec<[f64; 2]> {
        match self {
            BackgroundResult::Linear(fit) => fit.fit_points.clone(),
            BackgroundResult::Quadratic(fit) => fit.fit_points.clone(),
            BackgroundResult::PowerLaw(fit) => fit.fit_points.clone(),
            BackgroundResult::Exponential(fit) => fit.fit_points.clone(),
        }
    }
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
            composition_line: EguiLine::new(egui::Color32::BLUE),
            decomposition_lines: Vec::new(),
        }
    }

    pub fn fit(&mut self) {
        match &self.fit_model {
            FitModel::Gaussian(peak_markers, equal_stdev, free_position, bin_width) => {
                let mut fit = GaussianFitter::new(
                    self.data.clone(),
                    peak_markers.clone(),
                    self.background_model.clone(),
                    self.background_result.clone(),
                    *equal_stdev,
                    *free_position,
                    *bin_width,
                );

                match fit.lmfit() {
                    Ok(_) => {
                        self.composition_line.points = fit.fit_points.clone();
                        for fit in &fit.fit_result {
                            let mut line = EguiLine::new(egui::Color32::from_rgb(150, 0, 255));
                            line.points = fit.fit_points.clone();
                            self.decomposition_lines.push(line);
                        }

                        if self.background_result.is_none() {
                            if let Some(background_result) = &fit.background_result {
                                self.background_line.points = background_result.get_fit_points();
                                self.background_result = Some(background_result.clone());
                            }
                        }

                        self.fit_result = Some(FitResult::Gaussian(fit));
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            FitModel::None => {
                log::info!("No fitting required for 'None'");
            }
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

    pub fn subtract_background(&mut self, x_data: Vec<f64>, y_data: Vec<f64>) -> Option<Vec<f64>> {
        // Ensure background fitting has been performed
        if self.background_result.is_none() {
            self.fit_background(); // Fit background if not done already
        }

        // Check if background fitting was successful
        let background_fit = self.background_result.as_ref()?;

        // Generate background values for each x_data point
        let background_values: Vec<f64> = x_data
            .iter()
            .map(|&x| match background_fit {
                BackgroundResult::Linear(fit) => fit.evaluate(x),
                BackgroundResult::Quadratic(fit) => fit.evaluate(x),
                BackgroundResult::PowerLaw(fit) => fit.evaluate(x),
                BackgroundResult::Exponential(fit) => fit.evaluate(x),
            })
            .collect();

        // Subtract the background values from the actual y_data
        let corrected_y_data: Vec<f64> = y_data
            .iter()
            .zip(background_values.iter())
            .map(|(&y, &bg)| y - bg)
            .collect();

        Some(corrected_y_data)
    }

    pub fn get_peak_markers(&self) -> Vec<f64> {
        if self.fit_result.is_none() {
            match &self.fit_model {
                FitModel::Gaussian(peak_markers, _, _, _) => peak_markers.clone(),
                FitModel::None => Vec::new(),
            }
        } else {
            match &self.fit_result {
                Some(FitResult::Gaussian(fit)) => fit.peak_markers.clone(),
                None => Vec::new(),
            }
        }
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
        self.name = name;
    }

    pub fn fit_result_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing(self.name.clone(), |ui| {
            egui::ScrollArea::vertical()
                .min_scrolled_height(300.0)
                .show(ui, |ui| {
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

                    if self.fit_result.is_some() {
                        egui::Grid::new("fit_params_grid")
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label("Peak");
                                ui.label("Mean");
                                ui.label("FWHM");
                                ui.label("Area");
                                ui.label("Amplitude");
                                ui.label("Sigma");

                                ui.end_row();

                                self.fitter_stats(ui, false);
                            });

                        for line in &mut self.decomposition_lines {
                            line.menu_button(ui);
                        }

                        self.composition_line.menu_button(ui);
                    }
                });
        });
    }

    pub fn fitter_stats(&mut self, ui: &mut egui::Ui, skip_one: bool) {
        if let Some(fit_result) = &self.fit_result {
            match fit_result {
                FitResult::Gaussian(fit) => {
                    fit.fit_params_ui(ui, skip_one);
                }
            }
        }
    }

    // Draw the background, decomposition, and composition lines
    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi<'_>) {
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
