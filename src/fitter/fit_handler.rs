use rfd::FileDialog;

use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use super::fit_settings::FitSettings;
use super::main_fitter::{FitResult, Fitter};

use super::models::gaussian::GaussianFitter;

use crate::egui_plot_stuff::egui_line::EguiLine;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Fits {
    pub temp_fit: Option<Fitter>,
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
            // temp_background_fit: None,
            stored_fits: Vec::new(),
            settings: FitSettings::default(),
        }
    }

    pub fn store_temp_fit(&mut self) {
        if let Some(temp_fit) = &mut self.temp_fit.take() {
            temp_fit.set_background_color(egui::Color32::DARK_GREEN);
            temp_fit.set_composition_color(egui::Color32::DARK_BLUE);
            temp_fit.set_decomposition_color(egui::Color32::from_rgb(150, 0, 255));

            // remove Temp Fit from name
            let name = temp_fit.name.clone();
            let name = name.replace("Temp Fit", &format!("fit {}", self.stored_fits.len()));

            temp_fit.set_name(name);

            self.stored_fits.push(temp_fit.clone());
        }
    }

    pub fn set_log(&mut self, log_y: bool, log_x: bool) {
        if let Some(temp_fit) = &mut self.temp_fit {
            temp_fit.set_log(log_y, log_x);
        }

        for fit in &mut self.stored_fits {
            fit.set_log(log_y, log_x);
        }
    }

    pub fn set_stored_fits_background_color(&mut self, color: egui::Color32) {
        for fit in &mut self.stored_fits {
            fit.background_line.color = color;
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
                }
                Err(e) => {
                    log::error!("Error opening file: {:?}", e);
                }
            }
        }
    }

    pub fn export_all_lmfit_individual_files(&self) {
        if let Some(folder_path) = rfd::FileDialog::new().pick_folder() {
            for (i, fit) in self.stored_fits.iter().enumerate() {
                if let Some(FitResult::Gaussian(gauss)) = &fit.fit_result {
                    if let Some(text) = &gauss.lmfit_result {
                        let mut filename = format!("{}_fit_{}.sav", fit.name, i);
                        filename =
                            filename.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_"); // Sanitize filename
                        let mut full_path = PathBuf::from(&folder_path);
                        full_path.push(filename);

                        match File::create(&full_path) {
                            Ok(mut file) => {
                                if let Err(e) = file.write_all(text.as_bytes()) {
                                    log::error!(
                                        "Failed to write file {}: {:?}",
                                        full_path.display(),
                                        e
                                    );
                                }
                            }
                            Err(e) => {
                                log::error!("Error creating file {}: {:?}", full_path.display(), e)
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn export_lmfit(&self, dir: PathBuf) {
        for (i, fit) in self.stored_fits.iter().enumerate() {
            if let Some(FitResult::Gaussian(gauss)) = &fit.fit_result {
                if let Some(text) = &gauss.lmfit_result {
                    let mut filename = format!("{}_fit_{}.sav", fit.name, i);
                    filename =
                        filename.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_"); // Sanitize filename
                    let mut full_path = PathBuf::from(&dir);
                    full_path.push(filename);

                    match File::create(&full_path) {
                        Ok(mut file) => {
                            if let Err(e) = file.write_all(text.as_bytes()) {
                                log::error!(
                                    "Failed to write file {}: {:?}",
                                    full_path.display(),
                                    e
                                );
                            }
                        }
                        Err(e) => {
                            log::error!("Error creating file {}: {:?}", full_path.display(), e)
                        }
                    }
                }
            }
        }
    }

    pub fn save_and_load_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Save Fits").clicked() {
                self.save_to_file();
            }

            ui.separator();

            if ui.button("Load Fits").clicked() {
                self.load_from_file();
            }

            ui.separator();

            if ui.button("Export All lmfit Results").clicked() {
                self.export_all_lmfit_individual_files();
            }

            ui.separator();

            if ui.button("Load lmfit .sav").clicked() {
                if let Some(paths) = FileDialog::new().add_filter("SAV", &["sav"]).pick_files() {
                    for path in paths {
                        let mut gaussian_fitter = GaussianFitter::default();

                        match gaussian_fitter.lmfit(Some(path.clone())) {
                            Ok(_) => {
                                let mut new_fitter = Fitter::default();
                                new_fitter.set_name(
                                    path.file_stem()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or("lmfit_result")
                                        .to_string(),
                                );
                                new_fitter.composition_line.points =
                                    gaussian_fitter.fit_points.clone();

                                for (i, fit) in gaussian_fitter.fit_result.iter().enumerate() {
                                    let mut line =
                                        EguiLine::new(egui::Color32::from_rgb(150, 0, 255));
                                    line.points = fit.fit_points.clone();
                                    line.name = format!("{} Decomposition {}", new_fitter.name, i);
                                    new_fitter.decomposition_lines.push(line);
                                }

                                if let Some(background_result) = &gaussian_fitter.background_result
                                {
                                    new_fitter.background_result = Some(background_result.clone());
                                    new_fitter.background_line.points =
                                        background_result.get_fit_points();
                                }
                                new_fitter.fit_result =
                                    Some(FitResult::Gaussian(gaussian_fitter.clone()));

                                // new_fitter.fit_result =
                                //     Some(FitResult::Gaussian(gaussian_fitter.clone()));

                                self.stored_fits.push(new_fitter);
                                log::info!("Loaded lmfit result from {:?}", path);
                            }
                            Err(e) => {
                                log::error!("Failed to load lmfit result: {:?}", e);
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn remove_temp_fits(&mut self) {
        self.temp_fit = None;
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        self.apply_visibility_settings();

        if let Some(temp_fit) = &self.temp_fit {
            temp_fit.draw(plot_ui);
        }

        for fit in &mut self.stored_fits.iter() {
            fit.draw(plot_ui);

            // put the uuid above each peak if it is not 0
            if let Some(FitResult::Gaussian(gauss)) = &fit.fit_result {
                gauss.draw_uuid(plot_ui);
            }
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
                ui.label("Amplitude");
                ui.label("Sigma");
                ui.label("UUID");

                ui.end_row();

                if self.temp_fit.is_some() {
                    ui.label("Temp");

                    if let Some(temp_fit) = &mut self.temp_fit {
                        temp_fit.fitter_stats(ui, true);
                    }
                }

                if !self.stored_fits.is_empty() {
                    for (i, fit) in self.stored_fits.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", i));

                            ui.separator();

                            if ui.button("X").clicked() {
                                to_remove = Some(i);
                            }

                            ui.separator();
                        });
                        fit.fitter_stats(ui, true);
                    }
                }
            });

        if let Some(index) = to_remove {
            self.stored_fits.remove(index);
        }
    }

    pub fn fit_stats_ui(&mut self, ui: &mut egui::Ui) {
        if self.settings.show_fit_stats {
            ui.separator();

            self.fit_stats_grid_ui(ui);
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, show: bool) {
        if show {
            egui::ScrollArea::both()
                .id_salt("Context menu fit stats grid")
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("Fit Panel");
                        self.save_and_load_ui(ui);

                        self.settings.ui(ui);

                        self.fit_stats_grid_ui(ui);

                        ui.add_space(10.0);
                    });
                });
        }
    }

    pub fn fit_context_menu_ui(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Fits", |ui| {
            ui.horizontal(|ui| {
                ui.label("Fit Panel: ");
                ui.checkbox(&mut self.settings.show_fit_stats, "Show")
                    .on_hover_text("Show the fit statistics above the histogram");
            });

            self.ui(ui, true);
        });
    }
}
