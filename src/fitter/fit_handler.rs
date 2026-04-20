use rfd::FileDialog;

use egui::{Align2, Color32};
use egui_plot::{FilledArea, Line, MarkerShape, Plot, PlotUi, Points, Text};

use std::fs::File;
use std::io::Write as _;
use std::path::PathBuf;

use super::fit_settings::FitSettings;
use super::main_fitter::{FitResult, Fitter};

use super::models::gaussian::{GaussianFitter, HistogramDrawContext, UuidDrawOptions};

use super::common::{Calibration, fit_measurement_label};

use crate::custom_analysis::se_sps_analysis::uuid_map::FitUUID;
use crate::fitter::common::Data;
use crate::fitter::models::linear::LinearFitter;
use crate::fitter::models::quadratic;

use std::collections::HashMap;

#[derive(Debug, Clone)]
struct CalibrationPlotPoint {
    mean: f64,
    mean_uncertainty: f64,
    energy: f64,
    energy_uncertainty: f64,
    uuid: usize,
    fit_name: String,
    peak_index: usize,
}

fn padded_range(mut min: f64, mut max: f64) -> (f64, f64) {
    if !min.is_finite() || !max.is_finite() {
        return (0.0, 1.0);
    }

    if min > max {
        std::mem::swap(&mut min, &mut max);
    }

    let span = max - min;
    if span.abs() < f64::EPSILON {
        let pad = min.abs().max(1.0) * 0.1;
        (min - pad, max + pad)
    } else {
        let pad = span * 0.05;
        (min - pad, max + pad)
    }
}

fn unique_value_count(values: &[f64]) -> usize {
    let mut unique = values.to_vec();
    unique.sort_by(|left, right| left.total_cmp(right));
    unique.dedup_by(|left, right| {
        let scale = left.abs().max(right.abs()).max(1.0);
        (*left - *right).abs() <= scale * 1e-9
    });
    unique.len()
}

fn draw_plot_segment(
    plot_ui: &mut PlotUi<'_>,
    id_source: impl std::hash::Hash,
    points: Vec<[f64; 2]>,
    color: Color32,
    width: f32,
) {
    plot_ui.line(
        Line::new("", points)
            .allow_hover(false)
            .color(color)
            .width(width)
            .id(egui::Id::new(id_source)),
    );
}

#[derive(Debug, Clone, Copy)]
struct CrossErrorBars<'a> {
    id_prefix: &'a str,
    index: usize,
    x: f64,
    y: f64,
    x_uncertainty: f64,
    y_uncertainty: f64,
    x_cap_half_width: f64,
    y_cap_half_height: f64,
    color: Color32,
}

fn draw_cross_error_bars(plot_ui: &mut PlotUi<'_>, error_bar: CrossErrorBars<'_>) {
    let x_uncertainty = error_bar.x_uncertainty.max(0.0);
    let y_uncertainty = error_bar.y_uncertainty.max(0.0);

    if x_uncertainty.is_finite() && x_uncertainty > 0.0 {
        draw_plot_segment(
            plot_ui,
            (error_bar.id_prefix, error_bar.index, "x_bar"),
            vec![
                [error_bar.x - x_uncertainty, error_bar.y],
                [error_bar.x + x_uncertainty, error_bar.y],
            ],
            error_bar.color,
            1.0,
        );

        if error_bar.y_cap_half_height > 0.0 {
            draw_plot_segment(
                plot_ui,
                (error_bar.id_prefix, error_bar.index, "x_cap_left"),
                vec![
                    [
                        error_bar.x - x_uncertainty,
                        error_bar.y - error_bar.y_cap_half_height,
                    ],
                    [
                        error_bar.x - x_uncertainty,
                        error_bar.y + error_bar.y_cap_half_height,
                    ],
                ],
                error_bar.color,
                1.0,
            );
            draw_plot_segment(
                plot_ui,
                (error_bar.id_prefix, error_bar.index, "x_cap_right"),
                vec![
                    [
                        error_bar.x + x_uncertainty,
                        error_bar.y - error_bar.y_cap_half_height,
                    ],
                    [
                        error_bar.x + x_uncertainty,
                        error_bar.y + error_bar.y_cap_half_height,
                    ],
                ],
                error_bar.color,
                1.0,
            );
        }
    }

    if y_uncertainty.is_finite() && y_uncertainty > 0.0 {
        draw_plot_segment(
            plot_ui,
            (error_bar.id_prefix, error_bar.index, "y_bar"),
            vec![
                [error_bar.x, error_bar.y - y_uncertainty],
                [error_bar.x, error_bar.y + y_uncertainty],
            ],
            error_bar.color,
            1.0,
        );

        if error_bar.x_cap_half_width > 0.0 {
            draw_plot_segment(
                plot_ui,
                (error_bar.id_prefix, error_bar.index, "y_cap_bottom"),
                vec![
                    [
                        error_bar.x - error_bar.x_cap_half_width,
                        error_bar.y - y_uncertainty,
                    ],
                    [
                        error_bar.x + error_bar.x_cap_half_width,
                        error_bar.y - y_uncertainty,
                    ],
                ],
                error_bar.color,
                1.0,
            );
            draw_plot_segment(
                plot_ui,
                (error_bar.id_prefix, error_bar.index, "y_cap_top"),
                vec![
                    [
                        error_bar.x - error_bar.x_cap_half_width,
                        error_bar.y + y_uncertainty,
                    ],
                    [
                        error_bar.x + error_bar.x_cap_half_width,
                        error_bar.y + y_uncertainty,
                    ],
                ],
                error_bar.color,
                1.0,
            );
        }
    }
}

fn fmt_value_uncertainty(value: f64, uncertainty: f64, precision: usize) -> String {
    if uncertainty.is_finite() && uncertainty > 0.0 {
        format!("{value:.precision$} ± {uncertainty:.precision$}")
    } else {
        format!("{value:.precision$}")
    }
}

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
#[serde(default)]
pub struct SortState {
    pub col: SortCol,
    pub asc: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Fits {
    pub temp_fit: Option<Fitter>,
    pub stored_fits: Vec<Fitter>,
    pub settings: FitSettings,
    pub calibration: Calibration,
    pub sort_state: SortState,
    #[serde(skip)]
    pub pending_modify_fit: Option<usize>,
    #[serde(skip)]
    pub pending_refit_all: bool,
}

impl Default for Fits {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SortState {
    fn default() -> Self {
        Self {
            col: SortCol::Fit,
            asc: true,
        }
    }
}

impl Fits {
    fn ensure_extension_if_missing(mut path: PathBuf, extension: &str) -> PathBuf {
        if path.extension().is_none() {
            path.set_extension(extension);
        }
        path
    }

    fn sanitize_filename_component(name: &str) -> String {
        let mut out = String::with_capacity(name.len());
        let mut prev_was_underscore = false;

        for ch in name.chars() {
            let mapped = if matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
                || ch.is_whitespace()
            {
                '_'
            } else {
                ch
            };

            if mapped == '_' {
                if !prev_was_underscore {
                    out.push('_');
                }
                prev_was_underscore = true;
            } else {
                out.push(mapped);
                prev_was_underscore = false;
            }
        }

        let out = out.trim_matches('_').to_owned();
        if out.is_empty() {
            "fit".to_owned()
        } else {
            out
        }
    }

    fn save_lmfit_result_with_dialog(text: &str, suggested_file_name: &str) {
        if let Some(path) = FileDialog::new()
            .add_filter("SAV", &["sav"])
            .set_file_name(suggested_file_name)
            .save_file()
        {
            let path = Self::ensure_extension_if_missing(path, "sav");
            if let Err(e) = std::fs::write(&path, text) {
                log::error!("Failed to save lmfit result to {}: {e}", path.display());
            } else {
                log::info!("Saved lmfit result to {path:?}");
            }
        }
    }

    pub fn new() -> Self {
        Self {
            temp_fit: None,
            stored_fits: Vec::new(),
            settings: FitSettings::default(),
            calibration: Calibration::default(),
            sort_state: SortState::default(),
            pending_modify_fit: None,
            pending_refit_all: false,
        }
    }

    pub fn take_pending_modify_fit(&mut self) -> Option<usize> {
        self.pending_modify_fit.take()
    }

    pub fn take_pending_refit_all(&mut self) -> bool {
        std::mem::take(&mut self.pending_refit_all)
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
        if let Some(path) = FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name("spectrix_fits.json")
            .save_file()
        {
            let path = Self::ensure_extension_if_missing(path, "json");
            let file = File::create(path);
            match file {
                Ok(mut file) => match serde_json::to_string(self) {
                    Ok(json) => {
                        if let Err(e) = file.write_all(json.as_bytes()) {
                            log::error!("Failed to write fits file: {e}");
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to serialize fits: {e}");
                    }
                },
                Err(e) => {
                    log::error!("Error creating file: {e:?}");
                }
            }
        }
    }

    fn sync_calibration_values(&mut self) {
        for fit in &mut self.stored_fits {
            fit.sync_calibration_values(&self.calibration);
        }

        if let Some(temp_fit) = &mut self.temp_fit {
            temp_fit.sync_calibration_values(&self.calibration);
        }
    }

    fn apply_loaded_fits(&mut self, loaded_fits: Self) {
        self.settings = loaded_fits.settings;
        self.calibration = loaded_fits.calibration;
        self.sort_state = loaded_fits.sort_state;
        self.stored_fits.extend(loaded_fits.stored_fits);
        self.temp_fit = loaded_fits.temp_fit;
        self.sync_calibration_values();
    }

    fn load_from_file(&mut self) {
        if let Some(path) = FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match serde_json::from_str::<Self>(&contents) {
                    Ok(loaded_fits) => {
                        self.apply_loaded_fits(loaded_fits);
                    }
                    Err(e) => {
                        log::error!("Failed to deserialize fits from {}: {e}", path.display());
                    }
                },
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
                    let filename = format!(
                        "{}_fit_{}.sav",
                        Self::sanitize_filename_component(&fit.name),
                        i
                    );
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
                let filename = format!(
                    "{}_fit_{}.sav",
                    Self::sanitize_filename_component(&fit.name),
                    i
                );
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
        let mut energy = Vec::new();

        for fit in &mut self.stored_fits {
            // get the energy valye from the
            let fit_result = fit.fit_result.clone();

            if let Some(FitResult::Gaussian(gauss)) = fit_result {
                let cal_data = gauss.get_calibration_data();
                for (fit_mean, _fit_mean_unc, fit_energy, _fit_energy_unc) in cal_data {
                    mean.push(fit_mean);
                    energy.push(fit_energy);
                }
            }
        }

        let unique_mean_count = unique_value_count(&mean);

        if linear {
            // make sure there are at least 2 points to fit a linear
            if mean.len() < 2 || energy.len() < 2 || unique_mean_count < 2 {
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
                    let candidate = Calibration {
                        a: crate::fitter::common::Value {
                            value: 0.0,
                            uncertainty: 0.0,
                        },
                        b: crate::fitter::common::Value {
                            value: fitter.paramaters.slope.value.unwrap_or(1.0),
                            uncertainty: fitter.paramaters.slope.uncertainty.unwrap_or(0.0),
                        },
                        c: crate::fitter::common::Value {
                            value: fitter.paramaters.intercept.value.unwrap_or(0.0),
                            uncertainty: fitter.paramaters.intercept.uncertainty.unwrap_or(0.0),
                        },
                        cov: None,
                    };

                    if !candidate.coefficients_are_finite() {
                        log::error!("Calibration fit produced invalid linear coefficients.");
                        return;
                    }

                    self.calibration = candidate;
                }
                Err(e) => {
                    log::error!("Calibration fit failed: {e:?}");
                }
            }
        }

        if quadratic {
            // make sure there are at least 3 points to fit a quadratic
            if mean.len() < 3 || energy.len() < 3 || unique_mean_count < 3 {
                log::error!(
                    "Not enough points to fit a quadratic. Need at least 3 distinct points."
                );
                return;
            }

            let mut fitter = quadratic::QuadraticFitter::new(Data {
                x: mean.clone(),
                y: energy.clone(),
            });

            match fitter.lmfit() {
                Ok(_) => {
                    let candidate = Calibration {
                        a: crate::fitter::common::Value {
                            value: fitter.paramaters.a.value.unwrap_or(0.0),
                            uncertainty: fitter.paramaters.a.uncertainty.unwrap_or(0.0),
                        },
                        b: crate::fitter::common::Value {
                            value: fitter.paramaters.b.value.unwrap_or(1.0),
                            uncertainty: fitter.paramaters.b.uncertainty.unwrap_or(0.0),
                        },
                        c: crate::fitter::common::Value {
                            value: fitter.paramaters.c.value.unwrap_or(0.0),
                            uncertainty: fitter.paramaters.c.uncertainty.unwrap_or(0.0),
                        },
                        cov: fitter.covar,
                    };

                    if !candidate.coefficients_are_finite() {
                        log::error!("Calibration fit produced invalid quadratic coefficients.");
                        return;
                    }

                    let x_min = mean.iter().copied().fold(f64::INFINITY, f64::min);
                    let x_max = mean.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                    if !candidate.is_display_safe_on((x_min, x_max)) {
                        log::error!(
                            "Quadratic calibration is not monotonic over the fitted calibration points. Check the assigned energies and try again."
                        );
                        return;
                    }

                    self.calibration = candidate;
                }
                Err(e) => {
                    log::error!("Calibration fit failed: {e:?}");
                }
            }
        }

        if !self.calibration.coefficients_are_finite() {
            log::error!("Calibration contains invalid coefficients and was not applied.");
            return;
        }

        // Apply the calibration to all stored fits and temp fit
        for fit in &mut self.stored_fits {
            fit.calibrate(&self.calibration);
        }

        if let Some(temp_fit) = &mut self.temp_fit {
            temp_fit.calibrate(&self.calibration);
        }
    }

    fn calibration_plot_points(&self) -> Vec<CalibrationPlotPoint> {
        let mut points = Vec::new();

        for fit in &self.stored_fits {
            if let Some(FitResult::Gaussian(gauss)) = &fit.fit_result {
                points.extend(gauss.fit_result.iter().enumerate().filter_map(
                    |(peak_index, params)| {
                        let (
                            Some(energy),
                            Some(energy_uncertainty),
                            Some(mean),
                            Some(mean_uncertainty),
                        ) = (
                            params.energy.value,
                            params.energy.uncertainty,
                            params.mean.value,
                            params.mean.uncertainty,
                        )
                        else {
                            return None;
                        };

                        (energy != -1.0
                            && mean.is_finite()
                            && mean_uncertainty.is_finite()
                            && energy.is_finite()
                            && energy_uncertainty.is_finite())
                        .then(|| CalibrationPlotPoint {
                            mean,
                            mean_uncertainty: mean_uncertainty.max(0.0),
                            energy,
                            energy_uncertainty: energy_uncertainty.max(0.0),
                            uuid: params.uuid,
                            fit_name: fit.name.clone(),
                            peak_index,
                        })
                    },
                ));
            }
        }

        points.sort_by(|a, b| a.mean.total_cmp(&b.mean));
        points
    }

    fn calibration_curve_bounds(points: &[CalibrationPlotPoint]) -> ((f64, f64), (f64, f64)) {
        let x_min = points
            .iter()
            .map(|point| point.mean - point.mean_uncertainty.max(0.0))
            .fold(f64::INFINITY, f64::min);
        let x_max = points
            .iter()
            .map(|point| point.mean + point.mean_uncertainty.max(0.0))
            .fold(f64::NEG_INFINITY, f64::max);

        let y_min = points
            .iter()
            .map(|point| point.energy - point.energy_uncertainty.max(0.0))
            .fold(f64::INFINITY, f64::min);
        let y_max = points
            .iter()
            .map(|point| point.energy + point.energy_uncertainty.max(0.0))
            .fold(f64::NEG_INFINITY, f64::max);

        (padded_range(x_min, x_max), padded_range(y_min, y_max))
    }

    fn calibration_fit_series(
        &self,
        x_bounds: (f64, f64),
        samples: usize,
    ) -> (Vec<[f64; 2]>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let sample_count = samples.max(2);
        let step = (x_bounds.1 - x_bounds.0) / (sample_count.saturating_sub(1) as f64);

        let mut line_points = Vec::with_capacity(sample_count);
        let mut band_x = Vec::with_capacity(sample_count);
        let mut band_lower = Vec::with_capacity(sample_count);
        let mut band_upper = Vec::with_capacity(sample_count);

        for idx in 0..sample_count {
            let x = if idx + 1 == sample_count {
                x_bounds.1
            } else {
                x_bounds.0 + step * idx as f64
            };
            let Some(y) = self.calibration.calibrate_checked(x) else {
                continue;
            };
            let Some(uncertainty) = self.calibration.curve_uncertainty_checked(x) else {
                continue;
            };

            let lower = y - uncertainty;
            let upper = y + uncertainty;
            if !lower.is_finite() || !upper.is_finite() {
                continue;
            }

            line_points.push([x, y]);
            band_x.push(x);
            band_lower.push(lower);
            band_upper.push(upper);
        }

        (line_points, band_x, band_lower, band_upper)
    }

    fn energy_calibration_plots_ui(&self, ui: &mut egui::Ui, histogram_range: (f64, f64)) {
        let points = self.calibration_plot_points();
        if points.is_empty() {
            return;
        }

        let plot_width = ui.available_width().max(1.0);
        let point_color = Color32::from_rgb(210, 90, 90);
        let fit_color = Color32::from_rgb(70, 140, 235);
        let calibration_plot_id = ui.id().with("energy_calibration_plot");
        let calibration_band_id = ui.id().with("energy_calibration_fit_band");
        let calibration_line_id = ui.id().with("energy_calibration_fit_line");
        let residual_plot_id = ui.id().with("energy_calibration_residuals_plot");

        let (point_x_bounds, y_bounds) = Self::calibration_curve_bounds(&points);
        let fit_x_bounds = if histogram_range.0.is_finite()
            && histogram_range.1.is_finite()
            && histogram_range.0 != histogram_range.1
        {
            padded_range(
                histogram_range.0.min(histogram_range.1),
                histogram_range.0.max(histogram_range.1),
            )
        } else {
            point_x_bounds
        };
        let (fit_line, band_x, band_lower, band_upper) =
            self.calibration_fit_series(fit_x_bounds, 256);

        let x_span = (fit_x_bounds.1 - fit_x_bounds.0).abs().max(1.0);
        let y_span = (y_bounds.1 - y_bounds.0).abs().max(1.0);
        let x_cap_half_width = x_span * 0.01;
        let y_cap_half_height = y_span * 0.015;
        let uuid_label_offset = y_span * 0.02;

        ui.label("Energy Calibration");
        ui.label(format!(
            "Calibration curve shown over histogram range [{:.2}, {:.2}]",
            histogram_range.0.min(histogram_range.1),
            histogram_range.0.max(histogram_range.1)
        ));
        if !self.calibration.is_display_safe_on(histogram_range)
            && let Some(turning_point) = self.calibration.turning_point()
            && turning_point >= histogram_range.0.min(histogram_range.1)
            && turning_point <= histogram_range.0.max(histogram_range.1)
        {
            ui.colored_label(
                Color32::from_rgb(200, 120, 0),
                format!(
                    "Calibration turns over near x = {turning_point:.2} inside the histogram range."
                ),
            );
        }

        Plot::new(calibration_plot_id)
            .width(plot_width)
            .height(240.0)
            .show_x(true)
            .show_y(true)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_drag(false)
            .allow_boxed_zoom(false)
            .x_axis_label("Fit Mean")
            .y_axis_label("Energy")
            .label_formatter(|name, value| {
                if name.is_empty() {
                    format!("x: {:.3}\ny: {:.3}", value.x, value.y)
                } else {
                    name.to_owned()
                }
            })
            .show(ui, |plot_ui| {
                if band_x.len() >= 2 {
                    plot_ui.add(
                        FilledArea::new("Calibration Fit Band", &band_x, &band_lower, &band_upper)
                            .allow_hover(false)
                            .fill_color(fit_color.linear_multiply(0.18))
                            .id(calibration_band_id),
                    );
                }

                if fit_line.len() >= 2 {
                    plot_ui.line(
                        Line::new("Calibration Fit", fit_line.clone())
                            .allow_hover(false)
                            .color(fit_color)
                            .width(2.0)
                            .id(calibration_line_id),
                    );
                }

                for point in &points {
                    let uuid_line = if point.uuid != 0 {
                        format!("UUID: {}", point.uuid)
                    } else {
                        "UUID: —".to_owned()
                    };
                    let hover_label = format!(
                        "{}\nPeak: {}\n{}\nMean: {}\nEnergy: {}",
                        point.fit_name,
                        point.peak_index,
                        uuid_line,
                        fmt_value_uncertainty(point.mean, point.mean_uncertainty, 3),
                        fmt_value_uncertainty(point.energy, point.energy_uncertainty, 3),
                    );

                    plot_ui.points(
                        Points::new(hover_label, vec![[point.mean, point.energy]])
                            .shape(MarkerShape::Circle)
                            .color(point_color)
                            .filled(true)
                            .radius(4.0),
                    );
                }

                for (index, point) in points.iter().enumerate() {
                    draw_cross_error_bars(
                        plot_ui,
                        CrossErrorBars {
                            id_prefix: "energy_calibration",
                            index,
                            x: point.mean,
                            y: point.energy,
                            x_uncertainty: point.mean_uncertainty,
                            y_uncertainty: point.energy_uncertainty,
                            x_cap_half_width,
                            y_cap_half_height,
                            color: point_color,
                        },
                    );
                }

                for (index, point) in points.iter().enumerate() {
                    if point.uuid == 0 {
                        continue;
                    }

                    plot_ui.text(
                        Text::new(
                            format!("energy_calibration_uuid_{index}"),
                            [
                                point.mean,
                                point.energy
                                    + point.energy_uncertainty.max(0.0)
                                    + uuid_label_offset,
                            ]
                            .into(),
                            point.uuid.to_string(),
                        )
                        .anchor(Align2::CENTER_BOTTOM)
                        .color(point_color)
                        .allow_hover(false),
                    );
                }
            });

        ui.add_space(8.0);

        let residual_data: Vec<(CalibrationPlotPoint, [f64; 2], f64, f64, f64)> = points
            .iter()
            .filter_map(|point| {
                let fit_value = self.calibration.calibrate_checked(point.mean)?;
                let derivative = self.calibration.derivative_checked(point.mean)?;
                let residual = point.energy - fit_value;
                let residual_uncertainty = point
                    .energy_uncertainty
                    .hypot(derivative * point.mean_uncertainty);

                (residual.is_finite() && residual_uncertainty.is_finite()).then_some((
                    point.clone(),
                    [point.mean, residual],
                    point.mean_uncertainty,
                    residual_uncertainty,
                    fit_value,
                ))
            })
            .collect();

        if residual_data.is_empty() {
            ui.separator();
            return;
        }

        let residual_y_min = residual_data
            .iter()
            .map(|(_, point, _, residual_uncertainty, _)| point[1] - residual_uncertainty.max(0.0))
            .fold(f64::INFINITY, f64::min);
        let residual_y_max = residual_data
            .iter()
            .map(|(_, point, _, residual_uncertainty, _)| point[1] + residual_uncertainty.max(0.0))
            .fold(f64::NEG_INFINITY, f64::max);
        let residual_y_bounds = padded_range(residual_y_min, residual_y_max);
        let residual_y_span = (residual_y_bounds.1 - residual_y_bounds.0).abs().max(1.0);
        let residual_y_cap_half_height = residual_y_span * 0.015;

        Plot::new(residual_plot_id)
            .width(plot_width)
            .height(150.0)
            .show_x(true)
            .show_y(true)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_drag(false)
            .allow_boxed_zoom(false)
            .x_axis_label("Fit Mean")
            .y_axis_label("Residual")
            .label_formatter(|name, value| {
                if name.is_empty() {
                    format!("x: {:.3}\ny: {:.3}", value.x, value.y)
                } else {
                    name.to_owned()
                }
            })
            .show(ui, |plot_ui| {
                draw_plot_segment(
                    plot_ui,
                    ("energy_calibration_residuals", "zero_line"),
                    vec![[fit_x_bounds.0, 0.0], [fit_x_bounds.1, 0.0]],
                    Color32::GRAY,
                    1.0,
                );

                for (source_point, point, _, residual_uncertainty, fit_value) in &residual_data {
                    let uuid_line = if source_point.uuid != 0 {
                        format!("UUID: {}", source_point.uuid)
                    } else {
                        "UUID: —".to_owned()
                    };
                    let hover_label = format!(
                        "{}\nPeak: {}\n{}\nMean: {}\nResidual: {}\nEnergy: {}\nFit Energy: {:.3}",
                        source_point.fit_name,
                        source_point.peak_index,
                        uuid_line,
                        fmt_value_uncertainty(source_point.mean, source_point.mean_uncertainty, 3),
                        fmt_value_uncertainty(point[1], *residual_uncertainty, 3),
                        fmt_value_uncertainty(
                            source_point.energy,
                            source_point.energy_uncertainty,
                            3
                        ),
                        fit_value,
                    );

                    plot_ui.points(
                        Points::new(hover_label, vec![*point])
                            .shape(MarkerShape::Circle)
                            .color(point_color)
                            .filled(true)
                            .radius(4.0),
                    );
                }

                for (index, (_, point, mean_uncertainty, residual_uncertainty, _)) in
                    residual_data.iter().enumerate()
                {
                    draw_cross_error_bars(
                        plot_ui,
                        CrossErrorBars {
                            id_prefix: "energy_calibration_residuals",
                            index,
                            x: point[0],
                            y: point[1],
                            x_uncertainty: *mean_uncertainty,
                            y_uncertainty: *residual_uncertainty,
                            x_cap_half_width,
                            y_cap_half_height: residual_y_cap_half_height,
                            color: point_color,
                        },
                    );
                }
            });

        ui.separator();
    }

    pub fn save_and_load_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {

            if !self.stored_fits.is_empty() {
                if ui
                        .button("Refit")
                        .on_hover_text(
                            "Re-run each stored fit on current data by loading it into temp, fitting, then storing it again.",
                        )
                        .clicked()
                {
                    self.pending_refit_all = true;
                }

                ui.separator();
            }

            if ui
                .button("Save Fits")
                .on_hover_text(
                    "Save Spectrix fits as .json for restoring and continuing work inside Spectrix.\n\
                    Best option for normal Spectrix workflows.",
                )
                .clicked()
            {
                self.save_to_file();
            }

            ui.separator();

            if ui
                .button("Load Fits")
                .on_hover_text(
                    "Load Spectrix .json fits saved with 'Save Fits'.\n\
                    Use this to restore fits for continued work in Spectrix.",
                )
                .clicked()
            {
                self.load_from_file();
            }
        });

        ui.separator();

        ui.horizontal_wrapped(|ui| {
            if ui
                .button("Export All lmfit Results")
                .on_hover_text(
                    "Export each stored fit as lmfit .sav.\n\
                    Use this when you want to continue analysis in Python/lmfit.\n\
                    For Spectrix-only workflows, 'Save Fits' is usually better.",
                )
                .clicked()
            {
                self.export_all_lmfit_individual_files();
            }

            ui.separator();

            if ui
                .button("Load lmfit .sav")
                .on_hover_text(
                    "Import lmfit .sav files (for example, generated by Python/lmfit or exported from Spectrix).\n\
                    Use this to bring external lmfit results into Spectrix.",
                )
                .clicked()
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
                            new_fitter.apply_gaussian_fit_visuals(&gaussian_fitter);

                            if let Some(background_result) = &gaussian_fitter.background_result {
                                new_fitter.background_result = Some(background_result.clone());
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

    pub fn draw(
        &mut self,
        plot_ui: &mut egui_plot::PlotUi<'_>,
        histogram_bins: &[u64],
        histogram_range: (f64, f64),
        histogram_bin_width: f64,
        calibration: Option<&Calibration>,
    ) {
        self.apply_visibility_settings();

        let calibrated = calibration.is_some();
        let uuid_draw_options = UuidDrawOptions {
            calibrate: calibrated,
            log_x: false,
            log_y: false,
            label_size: self.settings.uuid_label_size,
            label_lift: self.settings.uuid_label_lift,
            draw_label_guide: self.settings.uuid_label_guides,
        };
        let histogram_draw_context = HistogramDrawContext {
            bins: histogram_bins,
            range: histogram_range,
            bin_width: histogram_bin_width,
        };
        if let Some(temp_fit) = &self.temp_fit {
            temp_fit.draw(plot_ui, calibration, self.settings.show_fit_lines_area);

            if let Some(FitResult::Gaussian(gauss)) = &temp_fit.fit_result {
                gauss.draw_uuid(
                    plot_ui,
                    UuidDrawOptions {
                        log_x: temp_fit.composition_line.log_x,
                        log_y: temp_fit.composition_line.log_y,
                        ..uuid_draw_options
                    },
                    histogram_draw_context,
                );
            }
        }

        for fit in &mut self.stored_fits.iter() {
            fit.draw(plot_ui, calibration, self.settings.show_fit_lines_area);

            // put the uuid above each peak if it is not 0
            if let Some(FitResult::Gaussian(gauss)) = &fit.fit_result {
                gauss.draw_uuid(
                    plot_ui,
                    UuidDrawOptions {
                        log_x: fit.composition_line.log_x,
                        log_y: fit.composition_line.log_y,
                        ..uuid_draw_options
                    },
                    histogram_draw_context,
                );
            }
        }
    }

    pub fn has_uuid_labels(&self) -> bool {
        let fit_has_uuid_labels = |fit: &Fitter| {
            if let Some(FitResult::Gaussian(gauss)) = &fit.fit_result {
                gauss.fit_result.iter().any(|params| params.uuid != 0)
            } else {
                false
            }
        };

        self.temp_fit.as_ref().is_some_and(fit_has_uuid_labels)
            || self.stored_fits.iter().any(fit_has_uuid_labels)
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
        let mut to_modify: Option<usize> = None;

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

        let sort_rows = |rows: &mut [Row], state: SortState| {
            rows.sort_by(|a, b| {
                let ka = key(a, state.col);
                let kb = key(b, state.col);
                let ord = ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal);
                if state.asc { ord } else { ord.reverse() }
            });
        };

        sort_rows(&mut rows, current);

        let mut uuid_updates: Vec<(Option<usize>, usize, usize)> = Vec::new(); // (fit_idx, peak, new_uuid)
        let mut energy_updates: Vec<(Option<usize>, usize, f64, f64)> = Vec::new(); // (fit_idx, peak, energy, unc)

        ui.horizontal(|ui| {
            if ui
                .button("\u{1F4CB}")
                .on_hover_text(
                    "Copy the currently displayed fit table to the clipboard as comma-delimited text, including values and uncertainties.",
                )
                .clicked()
            {
                let csv_number = |value: f64| {
                    if value.is_finite() {
                        value.to_string()
                    } else {
                        String::new()
                    }
                };

                let mut csv_lines = Vec::with_capacity(rows.len() + 1);
                csv_lines.push(
                    "fit,peak,mean,mean_uncertainty,fwhm,fwhm_uncertainty,area,area_uncertainty,amplitude,amplitude_uncertainty,sigma,sigma_uncertainty,uuid,energy,energy_uncertainty".to_owned(),
                );

                for row in &rows {
                    let fit_label = row
                        .fit_idx
                        .map_or_else(|| "Temp".to_owned(), |fit_idx| fit_idx.to_string());
                    csv_lines.push(format!(
                        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                        fit_label,
                        row.peak,
                        csv_number(row.mean.0),
                        csv_number(row.mean.1),
                        csv_number(row.fwhm.0),
                        csv_number(row.fwhm.1),
                        csv_number(row.area.0),
                        csv_number(row.area.1),
                        csv_number(row.amplitude.0),
                        csv_number(row.amplitude.1),
                        csv_number(row.sigma.0),
                        csv_number(row.sigma.1),
                        row.uuid,
                        csv_number(row.energy.0),
                        csv_number(row.energy.1),
                    ));
                }

                ui.ctx().copy_text(csv_lines.join("\n"));
            }
        });

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
                ui.label("Options");
                ui.end_row();

                // decide effective sort *after* header clicks
                let effective = pending.unwrap_or(current);
                let sort_col = effective.col;
                let asc = effective.asc;

                // ADD THIS so the choice persists next frame
                new_state = effective;

                // apply sort
                sort_rows(&mut rows, SortState { col: sort_col, asc });

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
                    fit_measurement_label(ui, Some(r.mean.0), Some(r.mean.1));
                    fit_measurement_label(ui, Some(r.fwhm.0), Some(r.fwhm.1));
                    fit_measurement_label(ui, Some(r.area.0), Some(r.area.1));
                    fit_measurement_label(ui, Some(r.amplitude.0), Some(r.amplitude.1));
                    fit_measurement_label(ui, Some(r.sigma.0), Some(r.sigma.1));
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

                    // options cell: show actions only on the first peak row of a fit
                    if let Some(i) = r.fit_idx {
                        // show actions on the first peak row of that fit
                        let is_first_peak = r.peak == 0;
                        if is_first_peak {
                            if let Some(super::main_fitter::FitResult::Gaussian(g)) =
                                &self.stored_fits[i].fit_result
                            {
                                ui.menu_button("Options", |ui| {
                                    if let Some(ref text) = g.lmfit_result
                                        && ui.button("Export lmfit").clicked()
                                    {
                                        let suggested_name = format!(
                                            "{}_lmfit_result.sav",
                                            Self::sanitize_filename_component(
                                                &self.stored_fits[i].name
                                            )
                                        );
                                        Self::save_lmfit_result_with_dialog(text, &suggested_name);
                                        ui.close();
                                    }

                                    if ui.button("Modify").clicked() {
                                        to_modify = Some(i);
                                        ui.close();
                                    }

                                    ui.menu_button("Fit Report", |ui| {
                                        egui::ScrollArea::vertical().show(ui, |ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(self.stored_fits[i].get_fit_report());
                                            });
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
                        // Temp fit options cell
                        if let Some(temp) = &self.temp_fit {
                            if let Some(super::main_fitter::FitResult::Gaussian(g)) =
                                &temp.fit_result
                            {
                                if r.peak == 0 {
                                    ui.menu_button("Options", |ui| {
                                        if let Some(ref text) = g.lmfit_result
                                            && ui.button("Export lmfit").clicked()
                                        {
                                            Self::save_lmfit_result_with_dialog(
                                                text,
                                                "temp_fit_lmfit_result.sav",
                                            );
                                            ui.close();
                                        }

                                        ui.menu_button("Fit Report", |ui| {
                                            egui::ScrollArea::vertical().show(ui, |ui| {
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label(temp.get_fit_report());
                                                });
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

        self.pending_modify_fit = to_modify;
    }

    fn fit_panel_contents_ui(&mut self, ui: &mut egui::Ui, histogram_range: (f64, f64)) {
        ui.horizontal(|ui| {
            ui.heading("Fit Panel");
            ui.separator();
            ui.checkbox(&mut self.settings.fit_panel_popout, "Pop Out")
                .on_hover_text("Open the fit panel in a separate native window when supported.");
        });

        self.save_and_load_ui(ui);

        self.settings.ui(ui);

        self.calubration_ui(ui);

        self.fit_stats_grid_ui(ui);
        self.energy_calibration_plots_ui(ui, histogram_range);

        ui.add_space(10.0);
    }

    pub fn calubration_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.settings.calibrated, "Calibration");

            if self.settings.calibrated {
                if self.calibration.ui(ui) {
                    self.calibration.cov = None;
                }

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

    pub fn ui(&mut self, ui: &mut egui::Ui, hist_name: &str, histogram_range: (f64, f64)) {
        if self.settings.show_fit_stats {
            let title = format!("Fit Panel: {hist_name}");
            let scroll_id = format!("fit_panel_scroll_{hist_name}");
            let mut open = self.settings.show_fit_stats;

            if self.settings.fit_panel_popout {
                let viewport_id =
                    egui::ViewportId::from_hash_of((ui.id(), hist_name, "fit_panel_viewport"));
                let viewport_builder = egui::ViewportBuilder::default()
                    .with_title(title)
                    .with_inner_size([960.0, 720.0])
                    .with_min_inner_size([520.0, 360.0]);

                ui.ctx()
                    .show_viewport_immediate(viewport_id, viewport_builder, |ui, _class| {
                        if ui.ctx().input(|input| input.viewport().close_requested()) {
                            open = false;
                            return;
                        }

                        egui::CentralPanel::default().show_inside(ui, |ui| {
                            egui::ScrollArea::both()
                                .id_salt(scroll_id.as_str())
                                .show(ui, |ui| {
                                    self.fit_panel_contents_ui(ui, histogram_range);
                                });
                        });
                    });
            } else {
                egui::Window::new(title)
                    .open(&mut open)
                    .show(ui.ctx(), |ui| {
                        egui::ScrollArea::both()
                            .id_salt(scroll_id.as_str())
                            .show(ui, |ui| {
                                self.fit_panel_contents_ui(ui, histogram_range);
                            });
                    });
            }

            self.settings.show_fit_stats = open;
        }
    }

    pub fn fit_context_menu_ui(&mut self, ui: &mut egui::Ui, histogram_range: (f64, f64)) {
        egui::ScrollArea::both()
            .id_salt(ui.id().with("fit_context_menu_scroll"))
            .max_height(450.0)
            .show(ui, |ui| {
                self.fit_panel_contents_ui(ui, histogram_range);
            });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fitter::common::Value;
    use crate::fitter::models::gaussian::GaussianParameters;

    fn calibration(a: f64, b: f64, c: f64) -> Calibration {
        Calibration {
            a: Value {
                value: a,
                uncertainty: 0.0,
            },
            b: Value {
                value: b,
                uncertainty: 0.0,
            },
            c: Value {
                value: c,
                uncertainty: 0.0,
            },
            cov: None,
        }
    }

    #[test]
    fn apply_loaded_fits_restores_saved_calibration_and_fit_settings() {
        let mut current = Fits::new();
        let mut loaded = Fits::new();
        loaded.settings.calibrated = true;
        loaded.settings.show_fit_stats = true;
        loaded.calibration = calibration(0.02, 1.5, 8.0);
        loaded.sort_state = SortState {
            col: SortCol::Energy,
            asc: false,
        };

        current.apply_loaded_fits(loaded);

        assert!(current.settings.calibrated);
        assert!(current.settings.show_fit_stats);
        assert_eq!(current.calibration.a.value, 0.02);
        assert_eq!(current.calibration.b.value, 1.5);
        assert_eq!(current.calibration.c.value, 8.0);
        assert_eq!(current.sort_state.col, SortCol::Energy);
        assert!(!current.sort_state.asc);
    }

    #[test]
    fn apply_loaded_fits_refreshes_live_calibrated_parameter_values() {
        let mut gaussian = GaussianFitter::default();
        let mut params = GaussianParameters::new(
            (100.0, 1.0),
            (4.0, 0.2),
            (1.0, 0.1),
            (2.355, 0.2),
            (250.0, 5.0),
        );
        params.energy.value = Some(42.0);
        params.energy.uncertainty = Some(0.3);
        gaussian.fit_result.push(params);

        let mut fitter = Fitter::default();
        fitter.fit_result = Some(FitResult::Gaussian(gaussian));

        let mut loaded = Fits::new();
        loaded.settings.calibrated = true;
        loaded.calibration = calibration(0.5, 2.0, 10.0);
        loaded.stored_fits.push(fitter);

        let mut current = Fits::new();
        current.apply_loaded_fits(loaded);

        let Some(FitResult::Gaussian(gaussian)) = &current.stored_fits[0].fit_result else {
            panic!("loaded fit should remain Gaussian");
        };
        let params = &gaussian.fit_result[0];

        assert_eq!(current.stored_fits[0].calibration.a.value, 0.5);
        assert_eq!(params.mean.calibrated_value, Some(26.0));
        assert_eq!(params.sigma.calibrated_value, Some(6.0));
        assert_eq!(params.energy.calibrated_value, Some(42.0));
        assert_eq!(params.area.calibrated_value, Some(250.0));
    }
}
