use rfd::FileDialog;

use std::fs::File;
use std::io::Write as _;
use std::path::PathBuf;

use super::fit_settings::FitSettings;
use super::main_fitter::{FitResult, Fitter};

use super::models::gaussian::GaussianFitter;

use super::common::Calibration;

use crate::custom_analysis::se_sps::FitUUID;
use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::fitter::common::Data;
use crate::fitter::models::linear::LinearFitter;
use crate::fitter::models::quadratic;

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum SortCol {
    Fit,
    Peak,
    Mean,
    Fwhm,
    Area,
    Amplitude,
    Sigma,
    Uuid,
    Energy,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub struct SortState {
    pub col: SortCol,
    pub asc: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Fits {
    pub temp_fit: Option<Fitter>,
    pub stored_fits: Vec<Fitter>,
    pub settings: FitSettings,
    pub calibration: Calibration,
    pub sort_state: SortState,
}

impl Default for Fits {
    fn default() -> Self {
        Self::new()
    }
}

impl Fits {
    pub fn new() -> Self {
        Self {
            temp_fit: None,
            stored_fits: Vec::new(),
            settings: FitSettings::default(),
            calibration: Calibration::default(),
            sort_state: SortState {
                col: SortCol::Fit,
                asc: true,
            },
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
                    log::error!("Error creating file: {e:?}");
                }
            }
        }
    }

    fn load_from_file(&mut self) {
        if let Some(path) = FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
            match std::fs::read_to_string(&path) {
                Ok(contents) => {
                    let loaded_fits: Self =
                        serde_json::from_str(&contents).expect("Failed to deserialize fits");
                    self.stored_fits.extend(loaded_fits.stored_fits);
                    self.temp_fit = loaded_fits.temp_fit;
                }
                Err(e) => {
                    log::error!("Error reading file: {e:?}");
                }
            }
        }
    }

    pub fn export_all_lmfit_individual_files(&self) {
        if let Some(folder_path) = rfd::FileDialog::new().pick_folder() {
            for (i, fit) in self.stored_fits.iter().enumerate() {
                if let Some(FitResult::Gaussian(gauss)) = &fit.fit_result
                    && let Some(text) = &gauss.lmfit_result
                {
                    let mut filename = format!("{}_fit_{}.sav", fit.name, i);
                    filename =
                        filename.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_"); // Sanitize filename
                    let full_path = PathBuf::from(&folder_path).join(filename);

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
                            log::error!("Error creating file {}: {:?}", full_path.display(), e);
                        }
                    }
                }
            }
        }
    }

    pub fn export_lmfit(&self, dir: &PathBuf) {
        for (i, fit) in self.stored_fits.iter().enumerate() {
            if let Some(FitResult::Gaussian(gauss)) = &fit.fit_result
                && let Some(text) = &gauss.lmfit_result
            {
                let mut filename = format!("{}_fit_{}.sav", fit.name, i);
                filename = filename.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_"); // Sanitize filename
                let full_path = PathBuf::from(&dir).join(filename);

                match File::create(&full_path) {
                    Ok(mut file) => {
                        if let Err(e) = file.write_all(text.as_bytes()) {
                            log::error!("Failed to write file {}: {:?}", full_path.display(), e);
                        }
                    }
                    Err(e) => {
                        log::error!("Error creating file {}: {:?}", full_path.display(), e);
                    }
                }
            }
        }
    }

    pub fn calibrate_stored_fits(&mut self, linear: bool, quadratic: bool) {
        let mut mean = Vec::new();
        let mut mean_uncertainty = Vec::new();
        let mut energy = Vec::new();
        let mut energy_uncertainty = Vec::new();

        for fit in &mut self.stored_fits {
            // get the energy valye from the
            let fit_result = fit.fit_result.clone();

            if let Some(FitResult::Gaussian(gauss)) = fit_result {
                let cal_data = gauss.get_calibration_data();
                for (fit_mean, fit_mean_unc, fit_energy, fit_energy_unc) in cal_data {
                    mean.push(fit_mean);
                    mean_uncertainty.push(fit_mean_unc);
                    energy.push(fit_energy);
                    energy_uncertainty.push(fit_energy_unc);
                }
            }
        }

        if linear {
            // make sure there are at least 2 points to fit a linear
            if mean.len() < 2 || energy.len() < 2 {
                log::error!("Not enough points to fit a linear. Need at least 2 points.");
                return;
            }

            // Fit a linear model
            let mut fitter = LinearFitter::new(Data {
                x: mean.clone(),
                y: energy.clone(),
            });

            match fitter.lmfit() {
                Ok(_) => {
                    self.calibration.a = crate::fitter::common::Value {
                        value: 0.0,
                        uncertainty: 0.0,
                    };
                    self.calibration.b = crate::fitter::common::Value {
                        value: fitter.paramaters.slope.value.unwrap_or(1.0),
                        uncertainty: fitter.paramaters.slope.uncertainty.unwrap_or(0.0),
                    };
                    self.calibration.c = crate::fitter::common::Value {
                        value: fitter.paramaters.intercept.value.unwrap_or(0.0),
                        uncertainty: fitter.paramaters.intercept.uncertainty.unwrap_or(0.0),
                    };
                }
                Err(e) => {
                    log::error!("Calibration fit failed: {e:?}");
                }
            }
        }

        if quadratic {
            // make sure there are at least 3 points to fit a quadratic
            if mean.len() < 3 || energy.len() < 3 {
                log::error!("Not enough points to fit a quadratic. Need at least 3 points.");
                return;
            }

            let mut fitter = quadratic::QuadraticFitter::new(Data {
                x: mean.clone(),
                y: energy.clone(),
            });

            match fitter.lmfit() {
                Ok(_) => {
                    self.calibration.a = crate::fitter::common::Value {
                        value: fitter.paramaters.a.value.unwrap_or(0.0),
                        uncertainty: fitter.paramaters.a.uncertainty.unwrap_or(0.0),
                    };
                    self.calibration.b = crate::fitter::common::Value {
                        value: fitter.paramaters.b.value.unwrap_or(1.0),
                        uncertainty: fitter.paramaters.b.uncertainty.unwrap_or(0.0),
                    };
                    self.calibration.c = crate::fitter::common::Value {
                        value: fitter.paramaters.c.value.unwrap_or(0.0),
                        uncertainty: fitter.paramaters.c.uncertainty.unwrap_or(0.0),
                    };
                }
                Err(e) => {
                    log::error!("Calibration fit failed: {e:?}");
                }
            }
        }

        // Apply the calibration to all stored fits and temp fit
        for fit in &mut self.stored_fits {
            fit.calibrate(&self.calibration);
        }

        if let Some(temp_fit) = &mut self.temp_fit {
            temp_fit.calibrate(&self.calibration);
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

            if ui.button("Load lmfit .sav").clicked()
                && let Some(paths) = FileDialog::new().add_filter("SAV", &["sav"]).pick_files()
            {
                for path in paths {
                    let mut gaussian_fitter = GaussianFitter::default();

                    match gaussian_fitter.lmfit(Some(path.clone())) {
                        Ok(_) => {
                            let mut new_fitter = Fitter::default();
                            new_fitter.set_name(
                                path.file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("lmfit_result")
                                    .to_owned(),
                            );
                            new_fitter.composition_line.points = gaussian_fitter.fit_points.clone();

                            for (i, fit) in gaussian_fitter.fit_result.iter().enumerate() {
                                let mut line = EguiLine::new(egui::Color32::from_rgb(150, 0, 255));
                                line.points = fit.fit_points.clone();
                                line.name = format!("{} Decomposition {}", new_fitter.name, i);
                                new_fitter.decomposition_lines.push(line);
                            }

                            if let Some(background_result) = &gaussian_fitter.background_result {
                                new_fitter.background_result = Some(background_result.clone());
                                new_fitter.background_line.points =
                                    background_result.get_fit_points();
                            }
                            new_fitter.fit_result =
                                Some(FitResult::Gaussian(gaussian_fitter.clone()));

                            // new_fitter.fit_result =
                            //     Some(FitResult::Gaussian(gaussian_fitter.clone()));

                            self.stored_fits.push(new_fitter);
                            log::info!("Loaded lmfit result from {path:?}");
                        }
                        Err(e) => {
                            log::error!("Failed to load lmfit result: {e:?}");
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

        let calibration = if self.settings.calibrated {
            Some(&self.calibration)
        } else {
            None
        };

        if let Some(temp_fit) = &self.temp_fit {
            temp_fit.draw(plot_ui, calibration);
        }

        let calibrated = self.settings.calibrated;
        for fit in &mut self.stored_fits.iter() {
            fit.draw(plot_ui, calibration);

            // put the uuid above each peak if it is not 0
            if let Some(FitResult::Gaussian(gauss)) = &fit.fit_result {
                gauss.draw_uuid(plot_ui, calibrated);
            }
        }
    }

    pub fn fit_stats_grid_ui(&mut self, ui: &mut egui::Ui) {
        // only show the grid if there is something to show
        if self.temp_fit.is_none() && self.stored_fits.is_empty() {
            return;
        }

        // --- helper row & collectors (local to this fn) ---
        #[derive(Clone)]
        struct Row {
            fit_idx: Option<usize>, // None => Temp
            peak: usize,            // peak index
            mean: (f64, f64),       // (val, unc)
            fwhm: (f64, f64),
            area: (f64, f64),
            amplitude: (f64, f64),
            sigma: (f64, f64),
            uuid: usize,
            energy: (f64, f64),
        }

        let fmt = |v: f64, u: f64| -> String {
            if v.is_finite() && u.is_finite() {
                format!("{v:.2} ± {u:.2}")
            } else if v.is_finite() {
                format!("{v:.2}")
            } else {
                "—".to_owned()
            }
        };

        let pick = |v: Option<f64>, u: Option<f64>| -> (f64, f64) {
            (v.unwrap_or(f64::NAN), u.unwrap_or(f64::NAN))
        };

        let pick_cal = |raw: (Option<f64>, Option<f64>),
                        cal: (Option<f64>, Option<f64>),
                        use_cal: bool|
         -> (f64, f64) {
            if use_cal {
                pick(cal.0, cal.1)
            } else {
                pick(raw.0, raw.1)
            }
        };

        let mut rows: Vec<Row> = Vec::new();
        let calibrated = self.settings.calibrated;

        // Temp fit rows (if any)
        if let Some(temp) = &mut self.temp_fit
            && let Some(super::main_fitter::FitResult::Gaussian(g)) = &temp.fit_result
        {
            for (i, p) in g.fit_result.iter().enumerate() {
                rows.push(Row {
                    fit_idx: None,
                    peak: i,
                    mean: pick_cal(
                        (p.mean.value, p.mean.uncertainty),
                        (p.mean.calibrated_value, p.mean.calibrated_uncertainty),
                        calibrated,
                    ),
                    fwhm: pick_cal(
                        (p.fwhm.value, p.fwhm.uncertainty),
                        (p.fwhm.calibrated_value, p.fwhm.calibrated_uncertainty),
                        calibrated,
                    ),
                    area: pick_cal(
                        (p.area.value, p.area.uncertainty),
                        (p.area.calibrated_value, p.area.calibrated_uncertainty),
                        calibrated,
                    ),
                    amplitude: pick_cal(
                        (p.amplitude.value, p.amplitude.uncertainty),
                        (
                            p.amplitude.calibrated_value,
                            p.amplitude.calibrated_uncertainty,
                        ),
                        calibrated,
                    ),
                    sigma: pick_cal(
                        (p.sigma.value, p.sigma.uncertainty),
                        (p.sigma.calibrated_value, p.sigma.calibrated_uncertainty),
                        calibrated,
                    ),
                    uuid: p.uuid,
                    energy: pick(p.energy.value, p.energy.uncertainty),
                });
            }
        }

        // Stored fits rows
        for (fi, fit) in self.stored_fits.iter().enumerate() {
            if let Some(super::main_fitter::FitResult::Gaussian(g)) = &fit.fit_result {
                for (i, p) in g.fit_result.iter().enumerate() {
                    rows.push(Row {
                        fit_idx: Some(fi),
                        peak: i,
                        mean: pick_cal(
                            (p.mean.value, p.mean.uncertainty),
                            (p.mean.calibrated_value, p.mean.calibrated_uncertainty),
                            calibrated,
                        ),
                        fwhm: pick_cal(
                            (p.fwhm.value, p.fwhm.uncertainty),
                            (p.fwhm.calibrated_value, p.fwhm.calibrated_uncertainty),
                            calibrated,
                        ),
                        area: pick_cal(
                            (p.area.value, p.area.uncertainty),
                            (p.area.calibrated_value, p.area.calibrated_uncertainty),
                            calibrated,
                        ),
                        amplitude: pick_cal(
                            (p.amplitude.value, p.amplitude.uncertainty),
                            (
                                p.amplitude.calibrated_value,
                                p.amplitude.calibrated_uncertainty,
                            ),
                            calibrated,
                        ),
                        sigma: pick_cal(
                            (p.sigma.value, p.sigma.uncertainty),
                            (p.sigma.calibrated_value, p.sigma.calibrated_uncertainty),
                            calibrated,
                        ),
                        uuid: p.uuid,
                        energy: pick(p.energy.value, p.energy.uncertainty),
                    });
                }
            }
        }

        // local snapshot (read-only this frame) + click stash
        let current = self.sort_state;
        let mut new_state = current;

        // NEW: stash a pending deletion
        let mut to_remove: Option<usize> = None;

        // NEW: sorting key helper
        let key = |r: &Row, col: SortCol| -> f64 {
            let nan_hi = |v: f64| if v.is_nan() { f64::INFINITY } else { v };
            match col {
                SortCol::Fit => r.fit_idx.map(|x| x as f64).unwrap_or(-1.0),
                SortCol::Peak => r.peak as f64,
                SortCol::Mean => nan_hi(r.mean.0),
                SortCol::Fwhm => nan_hi(r.fwhm.0),
                SortCol::Area => nan_hi(r.area.0),
                SortCol::Amplitude => nan_hi(r.amplitude.0),
                SortCol::Sigma => nan_hi(r.sigma.0),
                SortCol::Uuid => r.uuid as f64,
                SortCol::Energy => nan_hi(r.energy.0),
            }
        };

        let mut uuid_updates: Vec<(Option<usize>, usize, usize)> = Vec::new(); // (fit_idx, peak, new_uuid)
        let mut energy_updates: Vec<(Option<usize>, usize, f64, f64)> = Vec::new(); // (fit_idx, peak, energy, unc)

        egui::Grid::new("fit_params_grid_sortable")
            .striped(true)
            .show(ui, |ui| {
                // define these INSIDE this closure to avoid cross-borrows
                let mut pending: Option<SortState> = None;

                let mut header = |ui: &mut egui::Ui, label: &str, col: SortCol| {
                    let arrow = if current.col == col {
                        if current.asc { " ⬆" } else { " ⬇" }
                    } else {
                        ""
                    };
                    if ui.button(format!("{label}{arrow}")).clicked() {
                        pending = Some(if current.col == col {
                            SortState {
                                col,
                                asc: !current.asc,
                            }
                        } else {
                            SortState { col, asc: true }
                        });
                    }
                };

                // header row (sets `pending` if clicked)
                header(ui, "Fit #", SortCol::Fit);
                header(ui, "Peak #", SortCol::Peak);
                header(ui, "Mean", SortCol::Mean);
                header(ui, "FWHM", SortCol::Fwhm);
                header(ui, "Area", SortCol::Area);
                header(ui, "Amplitude", SortCol::Amplitude);
                header(ui, "Sigma", SortCol::Sigma);
                header(ui, "UUID", SortCol::Uuid);
                header(ui, "Energy", SortCol::Energy);
                ui.label("lmfit");
                ui.end_row();

                // decide effective sort *after* header clicks
                let effective = pending.unwrap_or(current);
                let sort_col = effective.col;
                let asc = effective.asc;

                // ADD THIS so the choice persists next frame
                new_state = effective;

                // apply sort
                rows.sort_by(|a, b| {
                    let ka = key(a, sort_col);
                    let kb = key(b, sort_col);
                    let ord = ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal);
                    if asc { ord } else { ord.reverse() }
                });

                // draw rows (your existing rendering code stays the same)
                for r in &rows {
                    // Fit cell: name + (X) button for stored fits
                    ui.horizontal(|ui| {
                        ui.label(match r.fit_idx {
                            // Some(i) => format!("{i} ({})", r.fit_name),
                            Some(i) => format!("{i}"),
                            None => "Temp".to_owned(),
                        });

                        if let Some(i) = r.fit_idx {
                            ui.separator();
                            if ui.button("X").clicked() {
                                to_remove = Some(i);
                            }
                        }
                    });

                    ui.label(format!("{}", r.peak));
                    ui.label(fmt(r.mean.0, r.mean.1));
                    ui.label(fmt(r.fwhm.0, r.fwhm.1));
                    ui.label(fmt(r.area.0, r.area.1));
                    ui.label(fmt(r.amplitude.0, r.amplitude.1));
                    ui.label(fmt(r.sigma.0, r.sigma.1));
                    let mut uuid_edit = r.uuid;
                    if ui
                        .add(
                            egui::DragValue::new(&mut uuid_edit)
                                .speed(1)
                                .update_while_editing(false),
                        )
                        .changed()
                    {
                        uuid_updates.push((r.fit_idx, r.peak, uuid_edit));
                    }
                    let mut e_val = if r.energy.0.is_finite() {
                        r.energy.0
                    } else {
                        0.0
                    };
                    let mut e_unc = if r.energy.1.is_finite() {
                        r.energy.1
                    } else {
                        0.0
                    };
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut e_val)
                                    .speed(0.1)
                                    .update_while_editing(false),
                            )
                            .changed();
                        ui.label("±");
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut e_unc)
                                    .speed(0.1)
                                    .update_while_editing(false),
                            )
                            .changed();
                    });
                    if changed {
                        energy_updates.push((r.fit_idx, r.peak, e_val, e_unc));
                    }

                    // lmfit cell: show buttons only on first peak of a given fit (optional)
                    if let Some(i) = r.fit_idx {
                        // show export/report on the first peak row of that fit
                        let is_first_peak = r.peak == 0;
                        if is_first_peak {
                            if let Some(super::main_fitter::FitResult::Gaussian(g)) =
                                &self.stored_fits[i].fit_result
                            {
                                if let Some(ref text) = g.lmfit_result
                                    && ui.button("Export").clicked()
                                    && let Some(path) = rfd::FileDialog::new()
                                        .set_file_name("fit_result.txt")
                                        .save_file()
                                {
                                    if let Err(e) = std::fs::write(&path, text) {
                                        eprintln!("Failed to save lmfit result: {e}");
                                    } else {
                                        log::info!("Saved lmfit result to {path:?}");
                                    }
                                }
                                ui.menu_button("Fit Report", |ui| {
                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(self.stored_fits[i].get_fit_report());
                                        });
                                    });
                                });
                            } else {
                                ui.label("—");
                            }
                        } else {
                            ui.label("—");
                        }
                    } else {
                        // Temp fit lmfit cell
                        if let Some(temp) = &self.temp_fit {
                            if let Some(super::main_fitter::FitResult::Gaussian(g)) =
                                &temp.fit_result
                            {
                                if r.peak == 0 {
                                    if let Some(ref text) = g.lmfit_result
                                        && ui.button("Export").clicked()
                                        && let Some(path) = rfd::FileDialog::new()
                                            .set_file_name("fit_result.txt")
                                            .save_file()
                                    {
                                        if let Err(e) = std::fs::write(&path, text) {
                                            eprintln!("Failed to save lmfit result: {e}");
                                        } else {
                                            log::info!("Saved lmfit result to {path:?}");
                                        }
                                    }
                                    ui.menu_button("Fit Report", |ui| {
                                        egui::ScrollArea::vertical().show(ui, |ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(temp.get_fit_report());
                                            });
                                        });
                                    });
                                } else {
                                    ui.label("—");
                                }
                            } else {
                                ui.label("—");
                            }
                        } else {
                            ui.label("—");
                        }
                    }

                    ui.end_row();
                }
            });

        self.sort_state = new_state;

        // Apply UUID changes
        for (fit_idx, peak, new_uuid) in uuid_updates {
            match fit_idx {
                Some(i) => {
                    if let Some(super::main_fitter::FitResult::Gaussian(g)) =
                        &mut self.stored_fits[i].fit_result
                        && let Err(e) = g.update_uuid_for_peak(peak, new_uuid)
                    {
                        eprintln!("UUID update failed: {e}");
                    }
                }
                None => {
                    if let Some(temp) = &mut self.temp_fit
                        && let Some(super::main_fitter::FitResult::Gaussian(g)) =
                            &mut temp.fit_result
                        && let Err(e) = g.update_uuid_for_peak(peak, new_uuid)
                    {
                        eprintln!("UUID update failed: {e}");
                    }
                }
            }
        }
        // Apply Energy changes
        for (fit_idx, peak, e, du) in energy_updates {
            match fit_idx {
                Some(i) => {
                    if let Some(super::main_fitter::FitResult::Gaussian(g)) =
                        &mut self.stored_fits[i].fit_result
                        && let Err(e) = g.update_energy_for_peak(peak, e, du)
                    {
                        eprintln!("Energy update failed: {e}");
                    }
                }
                None => {
                    if let Some(temp) = &mut self.temp_fit
                        && let Some(super::main_fitter::FitResult::Gaussian(g)) =
                            &mut temp.fit_result
                        && let Err(e) = g.update_energy_for_peak(peak, e, du)
                    {
                        eprintln!("Energy update failed: {e}");
                    }
                }
            }
        }

        // apply deletion (single fit index)
        if let Some(i) = to_remove
            && i < self.stored_fits.len()
        {
            self.stored_fits.remove(i);
        }
    }

    pub fn fit_stats_ui(&mut self, ui: &mut egui::Ui) {
        if self.settings.show_fit_stats {
            ui.separator();

            self.fit_stats_grid_ui(ui);
        }
    }

    pub fn calubration_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.settings.calibrated, "Calibration");

            if self.settings.calibrated {
                self.calibration.ui(ui);

                ui.separator();

                if ui.button("Calibrate").clicked() {
                    self.calibrate_stored_fits(false, false);
                }

                if ui.button("Linear").clicked() {
                    self.calibrate_stored_fits(true, false);
                }

                if ui.button("Quadratic").clicked() {
                    self.calibrate_stored_fits(false, true);
                }
            }
        });

        ui.separator();
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

                        self.calubration_ui(ui);

                        self.fit_stats_grid_ui(ui);

                        ui.add_space(10.0);
                    });
                });
        }
    }

    pub fn fit_context_menu_ui(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.settings.show_fit_stats, "Show Fit Panel")
            .on_hover_text("Show the fit statistics to the left of the histogram");

        self.ui(ui, true);
    }

    pub fn sync_uuid(&mut self, uuid_map: &[FitUUID]) {
        // UUID -> (energy, uncertainty)
        let lut: HashMap<usize, (f64, f64)> = uuid_map
            .iter()
            .map(|m| (m.uuid, (m.energy.0, m.energy.1)))
            .collect();

        // helper to apply to a single fitter
        let apply = |fitter: &mut Fitter| {
            if let Some(FitResult::Gaussian(gauss)) = &mut fitter.fit_result {
                // iterate by index so we can also call &mut methods on `gauss`
                for peak_idx in 0..gauss.fit_result.len() {
                    let uuid = gauss.fit_result[peak_idx].uuid;
                    if let Some(&(e, de)) = lut.get(&uuid)
                        && let Err(err) = gauss.update_energy_for_peak(peak_idx, e, de)
                    {
                        log::error!(
                            "Failed to update energy for UUID {uuid} (peak {peak_idx}): {err:?}"
                        );
                    }
                }
            }
        };

        // Stored fits
        for fitter in &mut self.stored_fits {
            apply(fitter);
        }

        // Temp fit
        if let Some(ref mut temp) = self.temp_fit {
            apply(temp);
        }
    }

    // pub fn
}
