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

    pub fn fitter_stats(&self, ui: &mut egui::Ui) {
        if let Some(fit) = &self.result {
            match fit {
                FitResult::Gaussian(fit) => fit.fit_params_ui(ui),
                FitResult::Linear(fit) => fit.fit_params_ui(ui),
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Fits {
    pub temp_fit: Option<Fitter>,
    pub temp_background_fit: Option<BackgroundFitter>,
    pub stored_fits: Vec<Fitter>,
}

impl Fits {
    pub fn new() -> Self {
        Fits {
            temp_fit: None,
            temp_background_fit: None,
            stored_fits: Vec::new(),
        }
    }

    pub fn remove_temp_fits(&mut self) {
        self.temp_fit = None;
        self.temp_background_fit = None;
    }

    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi) {
        if let Some(temp_fit) = &self.temp_fit {
            temp_fit.draw(
                plot_ui,
                egui::Color32::from_rgb(255, 0, 255),
                egui::Color32::GREEN,
                egui::Color32::BLUE,
            );
        }

        if let Some(temp_background_fit) = &self.temp_background_fit {
            temp_background_fit.draw(plot_ui, egui::Color32::GREEN);
        }

        for fit in self.stored_fits.iter() {
            fit.draw(
                plot_ui,
                egui::Color32::from_rgb(162, 0, 255),
                egui::Color32::from_rgb(162, 0, 255),
                egui::Color32::from_rgb(162, 0, 255),
            );
        }
    }

    pub fn fit_stats_grid_ui(&mut self, ui: &mut egui::Ui) {

        // only show the grid if there is something to show
        if self.temp_fit.is_none() && self.stored_fits.is_empty() {
            return;
        }

        let mut to_remove = None;

        // make this a scrollable grid

        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("fit_params_grid")
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Fit");
                    ui.label("Peak");
                    ui.label("Mean");
                    ui.label("FWHM");
                    ui.label("Area");
                    ui.end_row();

                    if !self.temp_fit.is_none() {
                        ui.label("Current");

                        if let Some(temp_fit) = &self.temp_fit {
                            temp_fit.fitter_stats(ui);
                        }
                    }

                    if !self.stored_fits.is_empty() {
                        for (i, fit) in self.stored_fits.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("Fit {}", i));

                                ui.separator();

                                if ui.button("X").clicked() {
                                    to_remove = Some(i);
                                }

                                ui.separator();
                            });
                            fit.fitter_stats(ui);
                        }
                    }
                });
        });

        if let Some(index) = to_remove {
            self.stored_fits.remove(index);
        }
    }
}
