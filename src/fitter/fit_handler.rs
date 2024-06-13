use rfd::FileDialog;

use std::fs::File;
use std::io::{Read, Write};

use super::gaussian::GaussianFitter;
use super::linear::LinearFitter;
use crate::egui_plot_stuff::egui_line::EguiLine;

use crate::fitter::background_fitter::BackgroundFitter;

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
                    if let Some((slope, intercept)) = background.get_slope_intercept() {
                        let composition_points =
                            fit.composition_fit_points_linear_bg(slope, intercept);

                        let mut line = EguiLine::new(egui::Color32::BLUE);
                        line.name = "Composition".to_string();
                        line.points = composition_points;
                        self.composition_line = line;
                    }
                }

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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FitSettings {
    pub show_decomposition: bool,
    pub show_composition: bool,
    pub show_background: bool,
    pub show_fit_stats: bool,
    pub fit_stats_height: f32,
}

impl Default for FitSettings {
    fn default() -> Self {
        FitSettings {
            show_decomposition: true,
            show_composition: true,
            show_background: true,
            show_fit_stats: false,
            fit_stats_height: 0.0,
        }
    }
}

impl FitSettings {
    pub fn menu_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Fit Stats: ");
            ui.checkbox(&mut self.show_fit_stats, "Show")
                .on_hover_text("Show the fit statistics above the histogram");

            ui.add(
                egui::DragValue::new(&mut self.fit_stats_height)
                    .speed(1.0)
                    .clamp_range(0.0..=f32::INFINITY)
                    .prefix("Height: ")
                    .suffix(" px"),
            )
            .on_hover_text("Set the height of the fit statistics grid to see more fits at once");
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Show Fit Lines: ");
            ui.checkbox(&mut self.show_decomposition, "Decomposition")
                .on_hover_text("Show the decomposition peaks");
            ui.checkbox(&mut self.show_composition, "Composition")
                .on_hover_text("Show the composition line");
            ui.checkbox(&mut self.show_background, "Background")
                .on_hover_text("Show the background line");
        });
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Fits {
    pub temp_fit: Option<Fitter>,
    pub temp_background_fit: Option<BackgroundFitter>,
    pub stored_fits: Vec<Fitter>,
    pub settings: FitSettings,
}

impl Default for Fits {
    fn default() -> Self {
        Self::new()
    }
}

impl Fits {
    pub fn new() -> Self {
        Fits {
            temp_fit: None,
            temp_background_fit: None,
            stored_fits: Vec::new(),
            settings: FitSettings::default(),
        }
    }

    pub fn store_temp_fit(&mut self) {
        if let Some(temp_fit) = &mut self.temp_fit.take() {
            temp_fit.set_background_color(egui::Color32::DARK_GREEN);
            temp_fit.set_composition_color(egui::Color32::DARK_BLUE);
            temp_fit.set_decomposition_color(egui::Color32::from_rgb(150, 0, 255));

            temp_fit.set_name(format!("Fit {}", self.stored_fits.len()));

            self.stored_fits.push(temp_fit.clone());
        }

        self.temp_background_fit = None;
    }

    pub fn set_log(&mut self, log_y: bool, log_x: bool) {
        if let Some(temp_fit) = &mut self.temp_fit {
            temp_fit.set_log(log_y, log_x);
        }

        if let Some(temp_background_fit) = &mut self.temp_background_fit {
            temp_background_fit.fit_line.log_y = log_y;
            temp_background_fit.fit_line.log_x = log_x;
        }

        for fit in &mut self.stored_fits {
            fit.set_log(log_y, log_x);
        }
    }

    pub fn set_stored_fits_background_color(&mut self, color: egui::Color32) {
        for fit in &mut self.stored_fits {
            if let Some(background) = &mut fit.background {
                background.fit_line.color = color;
            }
        }
    }

    pub fn set_stored_fits_composition_color(&mut self, color: egui::Color32) {
        for fit in &mut self.stored_fits {
            fit.composition_line.color = color;
        }
    }

    pub fn set_stored_fits_decomposition_color(&mut self, color: egui::Color32) {
        for fit in &mut self.stored_fits {
            for line in &mut fit.decomposition_lines {
                line.color = color;
            }
        }
    }

    pub fn update_visibility(&mut self) {
        if let Some(temp_fit) = &mut self.temp_fit {
            temp_fit.show_decomposition(self.settings.show_decomposition);
            temp_fit.show_composition(self.settings.show_composition);
            temp_fit.show_background(self.settings.show_background);
        }

        for fit in &mut self.stored_fits {
            fit.show_decomposition(self.settings.show_decomposition);
            fit.show_composition(self.settings.show_composition);
            fit.show_background(self.settings.show_background);
        }
    }

    pub fn apply_visibility_settings(&mut self) {
        self.update_visibility();
    }

    fn save_to_file(&self) {
        if let Some(path) = FileDialog::new().add_filter("JSON", &["json"]).save_file() {
            let file = File::create(path);
            match file {
                Ok(mut file) => {
                    let json = serde_json::to_string(self).expect("Failed to serialize fits");
                    file.write_all(json.as_bytes())
                        .expect("Failed to write file");
                }
                Err(e) => {
                    log::error!("Error creating file: {:?}", e);
                }
            }
        }
    }

    fn load_from_file(&mut self) {
        if let Some(path) = FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
            let file = File::open(path);
            match file {
                Ok(mut file) => {
                    let mut contents = String::new();
                    file.read_to_string(&mut contents)
                        .expect("Failed to read file");
                    let loaded_fits: Fits =
                        serde_json::from_str(&contents).expect("Failed to deserialize fits");
                    self.stored_fits.extend(loaded_fits.stored_fits); // Append loaded fits to current stored fits
                    self.temp_fit = loaded_fits.temp_fit; // override temp_fit
                    self.temp_background_fit = loaded_fits.temp_background_fit; // override temp_background_fit
                }
                Err(e) => {
                    log::error!("Error opening file: {:?}", e);
                }
            }
        }
    }

    pub fn save_and_load_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Save Fits").clicked() {
                self.save_to_file();
            }

            ui.separator();

            if ui.button("Load Fits").clicked() {
                self.load_from_file();
            }
        });
    }

    pub fn remove_temp_fits(&mut self) {
        self.temp_fit = None;
        self.temp_background_fit = None;
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi) {
        self.apply_visibility_settings();

        if let Some(temp_fit) = &self.temp_fit {
            temp_fit.draw(plot_ui);
        }

        if let Some(temp_background_fit) = &self.temp_background_fit {
            temp_background_fit.draw(plot_ui);
        }

        for fit in &mut self.stored_fits.iter() {
            fit.draw(plot_ui);
        }
    }

    pub fn fit_stats_grid_ui(&mut self, ui: &mut egui::Ui) {
        // only show the grid if there is something to show
        if self.temp_fit.is_none() && self.stored_fits.is_empty() {
            return;
        }

        let mut to_remove = None;

        egui::Grid::new("fit_params_grid")
            .striped(true)
            .show(ui, |ui| {
                ui.label("Fit");
                ui.label("Peak");
                ui.label("Mean");
                ui.label("FWHM");
                ui.label("Area");
                ui.end_row();

                if self.temp_fit.is_some() {
                    ui.label("Current");

                    if let Some(temp_fit) = &self.temp_fit {
                        temp_fit.fitter_stats(ui);
                    }
                }

                if !self.stored_fits.is_empty() {
                    for (i, fit) in self.stored_fits.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", i));

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

        if let Some(index) = to_remove {
            self.stored_fits.remove(index);
        }
    }

    pub fn fit_stats_ui(&mut self, ui: &mut egui::Ui) {
        if self.settings.show_fit_stats {
            egui::ScrollArea::vertical()
                .max_height(self.settings.fit_stats_height)
                .show(ui, |ui| {
                    self.fit_stats_grid_ui(ui);
                });
        }
    }

    pub fn fit_lines_ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label("Fit Lines");

                ui.separator();

                if let Some(temp_fit) = &mut self.temp_fit {
                    temp_fit.lines_ui(ui);
                }

                for fit in &mut self.stored_fits {
                    fit.lines_ui(ui);
                }
            });
        });
    }

    pub fn fit_context_menu_ui(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Fits", |ui| {
            self.save_and_load_ui(ui);

            ui.separator();

            self.settings.menu_ui(ui);

            ui.separator();

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .id_source("Context menu fit stats grid")
                .show(ui, |ui| {
                    self.fit_stats_grid_ui(ui);
                });

            ui.separator();

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    self.fit_lines_ui(ui);
                });
        });
    }
}
