use super::common::Data;
use super::models::exponential::{ExponentialFitter, ExponentialParameters};
use super::models::gaussian::GaussianFitter;
use super::models::linear::{LinearFitter, LinearParameters};
use super::models::powerlaw::{PowerLawFitter, PowerLawParameters};
use super::models::quadratic::{QuadraticFitter, QuadraticParameters};
use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::fitter::common::Calibration;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum FitModel {
    Gaussian(Vec<f64>, Vec<f64>, Vec<(f64, f64)>, bool, bool), // region markers, initial peak locations, free sigma, free position
    None,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum FitResult {
    Gaussian(GaussianFitter),
}

impl FitResult {
    pub fn get_calibration_data(&self) -> Vec<(f64, f64, f64, f64)> {
        match self {
            Self::Gaussian(fit) => fit.get_calibration_data(),
        }
    }
}

#[derive(Default, PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum BackgroundModel {
    Linear(LinearParameters),
    Quadratic(QuadraticParameters),
    PowerLaw(PowerLawParameters),
    Exponential(ExponentialParameters),
    #[default]
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
            Self::Linear(fit) => fit.fit_points.clone(),
            Self::Quadratic(fit) => fit.fit_points.clone(),
            Self::PowerLaw(fit) => fit.fit_points.clone(),
            Self::Exponential(fit) => fit.fit_points.clone(),
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

impl Default for Fitter {
    fn default() -> Self {
        Self {
            name: "Fit".to_owned(),

            data: Data::default(),

            background_model: BackgroundModel::None,
            background_result: None,

            fit_model: FitModel::None,
            fit_result: None,

            background_line: EguiLine::new(egui::Color32::GREEN),
            composition_line: EguiLine::new(egui::Color32::BLUE),
            decomposition_lines: Vec::new(),
        }
    }
}

impl Fitter {
    // Constructor to create a new Fitter with empty data and specified model
    pub fn new(data: Data) -> Self {
        Self {
            name: "Fit".to_owned(),

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
            FitModel::Gaussian(
                region_markers,
                peak_markers,
                background_markrs,
                equal_stdev,
                free_position,
            ) => {
                let mut fit = GaussianFitter::new(
                    self.data.clone(),
                    region_markers.clone(),
                    peak_markers.clone(),
                    background_markrs.clone(),
                    self.background_model.clone(),
                    self.background_result.clone(),
                    *equal_stdev,
                    *free_position,
                );

                match fit.lmfit(None) {
                    Ok(_) => {
                        self.composition_line.points = fit.fit_points.clone();
                        for fit in &fit.fit_result {
                            let mut line = EguiLine::new(egui::Color32::from_rgb(150, 0, 255));
                            line.points = fit.fit_points.clone();
                            self.decomposition_lines.push(line);
                        }

                        if self.background_result.is_none()
                            && let Some(background_result) = &fit.background_result
                        {
                            self.background_line.points = background_result.get_fit_points();
                            self.background_result = Some(background_result.clone());
                        }

                        self.fit_result = Some(FitResult::Gaussian(fit));
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                    }
                }
            }
            FitModel::None => {
                log::info!("No fitting required for 'None'");
            }
        }
    }

    pub fn calibrate(&mut self, calibration: &Calibration) {
        log::info!("Calibrating");
        // Calibration logic goes here

        // update gaussian fit parameters
        if let Some(fit_result) = &mut self.fit_result {
            match fit_result {
                FitResult::Gaussian(fit) => {
                    fit.calibrate(calibration);
                }
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
                        eprintln!("Error: {e}");
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
                        eprintln!("Error: {e}");
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
                        eprintln!("Error: {e}");
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
                        eprintln!("Error: {e}");
                    }
                }
            }
            BackgroundModel::None => {
                log::info!("No background fitting required for 'None'");
            }
        }
        log::info!("Finished fitting background");
    }

    pub fn get_peak_markers(&self) -> Vec<f64> {
        if self.fit_result.is_none() {
            match &self.fit_model {
                FitModel::Gaussian(_, peak_markers, _, _, _) => peak_markers.clone(),
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
        self.composition_line.name = format!("{name}-Composition");

        for (i, line) in self.decomposition_lines.iter_mut().enumerate() {
            line.name = format!("{name}-Peak {i}");
        }

        self.background_line.name = format!("{name}-Background");
        self.name = name;
    }

    pub fn fit_result_ui(&mut self, ui: &mut egui::Ui, calibrate: bool) {
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
                                ui.label("Energy");

                                ui.end_row();

                                self.fitter_stats(ui, false, calibrate);
                            });

                        // for line in &mut self.decomposition_lines {
                        //     line.menu_button(ui);
                        // }

                        // self.composition_line.menu_button(ui);
                    }
                });
        });
    }

    pub fn fitter_stats(&mut self, ui: &mut egui::Ui, skip_one: bool, calibrate: bool) {
        if let Some(fit_result) = &mut self.fit_result {
            match fit_result {
                FitResult::Gaussian(fit) => {
                    fit.fit_params_ui(ui, skip_one, calibrate);
                }
            }
        }
    }

    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi<'_>, calibration: Option<&Calibration>) {
        for line in &self.decomposition_lines {
            line.draw(plot_ui, calibration);
        }

        self.composition_line.draw(plot_ui, calibration);

        self.background_line.draw(plot_ui, calibration);
    }

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
