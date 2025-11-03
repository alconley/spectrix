use std::collections::HashMap;
use std::path::PathBuf;

use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::fitter::main_fitter::FitResult;
use crate::histoer::histogrammer;
use egui::{Color32, Vec2b};
use egui_plot::MarkerShape;
use std::io::Write as _;

use crate::egui_plot_stuff::egui_points::EguiPoints;
use crate::fitter::models::gaussian::GaussianParameters;

use super::integration_methods::{
    IntegrationRule, cell_polygon_with_baseline, integrate_left_endpoint, integrate_midpoint,
    integrate_right_endpoint, integrate_trapezoidal,
};
use super::run_info::{Run, Runs};
use super::settings::SPSAnalysisSettings;
use super::uuid_map::FitUUIDMap;

use super::helpers::{
    average_points_by_angle, avg_calibrated_mean, log_axis_formatter, log_axis_spacer,
    nice_log_ceil, nice_log_floor,
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SPSAnalysis {
    target_thickness: (f64, f64), // in ug/cm^2, uncertainty
    target_molar_mass: f64,       // in g/mol
    beam_energy: f64,             // in MeV
    beam_z: u32,                  // atomic number
    runs: Runs,
    fit_uuid_map: FitUUIDMap,
    settings: SPSAnalysisSettings,
}

impl Default for SPSAnalysis {
    fn default() -> Self {
        Self {
            target_thickness: (130.0, 13.0),
            target_molar_mass: 149.920887,
            beam_energy: 16.0,
            beam_z: 1,
            runs: Runs::default(),
            fit_uuid_map: FitUUIDMap::default(),
            settings: SPSAnalysisSettings::default(),
        }
    }
}

impl SPSAnalysis {
    pub fn ui(
        &mut self,
        ctx: &mut egui::Ui,
        files: &[(PathBuf, bool)],
        histogrammer: &mut histogrammer::Histogrammer,
    ) {
        self.ensure_runs_for_files(files);

        // left panel
        egui::SidePanel::left("sps_left_panel")
            .resizable(true)
            .default_width(300.0)
            .show_animated(ctx.ctx(), self.settings.panel_open, |ui| {
                egui::ScrollArea::both()
                    .id_salt("sps_left_scroll_area")
                    .show(ui, |ui| {
                        self.left_panel(ui, histogrammer);
                    });
            });

        self.panel_toggle_button(ctx.ctx());

        egui::CentralPanel::default().show(ctx.ctx(), |ui| {
            self.cross_section_ui(ui, histogrammer);
        });
    }

    pub fn panel_toggle_button(&mut self, ctx: &egui::Context) {
        // Secondary left panel for the toggle button
        egui::SidePanel::left("spectrix_toggle_left_panel")
            .resizable(false)
            .show_separator_line(false)
            .min_width(1.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 10.0); // Center the button vertically
                    if ui
                        .small_button(if self.settings.panel_open {
                            "◀"
                        } else {
                            "▶"
                        })
                        .clicked()
                    {
                        self.settings.panel_open = !self.settings.panel_open;
                    }
                });
            });
    }

    pub fn left_panel(&mut self, ui: &mut egui::Ui, histogrammer: &mut histogrammer::Histogrammer) {
        let mut dirty = false; // <- track changes

        egui::CollapsingHeader::new("General Settings")
            .default_open(true)
            .show(ui, |ui| {
                if self.general_settings_ui(ui) {
                    dirty = true;
                }
            });

        egui::CollapsingHeader::new("Runs")
            .default_open(true)
            .show(ui, |ui| {
                if self.runs.ui(ui) {
                    dirty = true;
                }
            });

        egui::CollapsingHeader::new("Fit UUID Map")
            .default_open(true)
            .show(ui, |ui| {
                self.fit_uuid_map.ui(ui, Some(histogrammer));
            });

        egui::CollapsingHeader::new("Settings")
            .default_open(false)
            .show(ui, |ui| {
                self.settings.ui(ui);
                if ui.button("Export [.csv]").clicked() {
                    self.export_cross_section_csv(histogrammer);
                }
            });

        // Recompute once if anything changed
        if dirty {
            self.calculate_normalization_factor();
        }
    }

    pub fn general_settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("sps_target_grid")
            .striped(true)
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Target Thickness:");
                ui.horizontal(|ui| {
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut self.target_thickness.0)
                                .speed(1.0)
                                .range(0.0..=f64::INFINITY),
                        )
                        .changed();
                    ui.label("±");
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut self.target_thickness.1)
                                .speed(0.1)
                                .range(0.0..=f64::INFINITY)
                                .suffix(" ug/cm²"),
                        )
                        .changed();
                });
                ui.end_row();

                ui.label("Target Molar Mass:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.target_molar_mass)
                            .speed(0.1)
                            .range(0.0..=f64::INFINITY)
                            .suffix(" g/mol"),
                    )
                    .changed();
                ui.end_row();

                ui.label("Beam Energy:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.beam_energy)
                            .speed(0.1)
                            .range(0.0..=f64::INFINITY)
                            .suffix(" MeV"),
                    )
                    .changed();
                ui.end_row();

                ui.label("Beam Z:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.beam_z)
                            .speed(1)
                            .range(1..=92),
                    )
                    .changed();
                ui.end_row();
            });

        changed
    }

    /// Ensure each *checked* file appears once in `runs` (add missing; keep existing).
    fn ensure_runs_for_files(&mut self, files: &[(PathBuf, bool)]) {
        for (path, checked) in files {
            if !*checked {
                continue;
            }
            let key = path.to_string_lossy();
            let exists = self.runs.runs.iter().any(|r| r.file_name == key);

            if !exists {
                self.runs.runs.push(Run::from_path(path));
            }
        }
    }

    fn calculate_normalization_factor(&mut self) {
        for run in &mut self.runs.runs {
            let q_b = run.bci_scaler * (run.bci_scale as f64) * 1e-9 / 100.0; // total charge of particles incident on the target
            let n_b = q_b / ((self.beam_z as f64) * 1.602e-19); // number of incident particles
            let f_target =
                1.0e-24 * 6.023e23 * (self.target_thickness.0 / 1.0e6) / self.target_molar_mass; // areal density of target atoms in atoms/cm^2
            let solid_angle = run.slits * 1.0e-3; // convert msr to sr

            let normalization_factor = n_b * f_target * solid_angle * 1.0e-6; // for units of ub/sr
            let normalization_factor_uncertainty = normalization_factor
                * ((run.bci_uncertainty / 100.0).powi(2)
                    + (self.target_thickness.1 / self.target_thickness.0).powi(2)
                    + (run.slits_uncertainty / 100.0).powi(2))
                .sqrt();

            run.normalization_factor =
                Some((normalization_factor, normalization_factor_uncertainty));
        }
    }

    #[expect(clippy::type_complexity)]
    pub fn get_data_from_histogrammer(
        &self,
        histogrammer: &histogrammer::Histogrammer,
    ) -> HashMap<usize, Vec<(f64, f64, f64, f64, GaussianParameters, Color32, MarkerShape)>> {
        let mut map: HashMap<
            usize,
            Vec<(f64, f64, f64, f64, GaussianParameters, Color32, MarkerShape)>,
        > = HashMap::new();

        for run in &self.runs.runs {
            let angle = run.angle;
            let color = run.color;
            let markershape = run.get_egui_markershape();
            let field = run.magnetic_field;

            let (norm, norm_unc) = match run.normalization_factor {
                Some((n, dn)) if n != 0.0 => (n, dn),
                _ => continue,
            };

            // get base file name without extension
            let base_name = std::path::Path::new(&run.file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            for (_id, tile) in histogrammer.tree.tiles.iter() {
                if let egui_tiles::Tile::Pane(pane) = tile
                    && let crate::histoer::pane::Pane::Histogram(h_arc) = pane
                    && let Ok(h) = h_arc.lock()
                {
                    let hist_name = h.name.to_ascii_lowercase();
                    if !hist_name.starts_with(&format!("{base_name}/")) {
                        continue;
                    }
                    for fit in &h.fits.stored_fits {
                        if let Some(fit_result) = &fit.fit_result {
                            let FitResult::Gaussian(gauss_fit) = fit_result;
                            for params in &gauss_fit.fit_result {
                                let uuid = params.uuid;
                                if uuid == 0 {
                                    continue; // skip unset
                                }
                                if let Some(area) = params.area.value {
                                    let cross_section_ub_sr = area / norm;
                                    let area_uncertainty = params.area.uncertainty.unwrap_or(0.0);
                                    let cross_section_uncertainty = cross_section_ub_sr
                                        * (area_uncertainty / area).hypot(norm_unc / norm);

                                    map.entry(uuid).or_default().push((
                                        angle,
                                        cross_section_ub_sr,
                                        cross_section_uncertainty,
                                        field,
                                        params.clone(),
                                        color,
                                        markershape,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        map
    }

    pub fn export_cross_section_csv(
        &self,
        histogrammer: &crate::histoer::histogrammer::Histogrammer,
    ) {
        let data = self.get_data_from_histogrammer(histogrammer);
        if data.is_empty() {
            log::warn!("No cross-section data to export.");
            return;
        }

        let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("cross_sections.csv")
            .save_file()
        else {
            return;
        };

        let mut f = match std::fs::File::create(&path) {
            Ok(f) => f,
            Err(e) => {
                log::error!("Failed to create CSV: {e}");
                return;
            }
        };

        // Header
        if let Err(e) = writeln!(
            f,
            "UUID,Angle,Magnetic Field,Cross Section,Cross Section Uncertainty,\
             Mean,Mean Unc,Sigma,Sigma Unc,FWHM,FWHM Unc,Amplitude,Amplitude Unc,\
             Area,Area Unc,Assigned Energy,Energy Unc,Calibrated Mean,Calibrated Mean Unc,\
             Calibrated FWHM,Calibrated FWHM Unc"
        ) {
            log::error!("Failed to write header: {e}");
            return;
        }

        // Sort UUIDs
        let mut uuids: Vec<_> = data.keys().copied().collect();
        uuids.sort_unstable();

        for uuid in uuids {
            for (angle, cs, dcs, field, params, _, _) in &data[&uuid] {
                let fmt = |v: Option<f64>| v.map(|x| format!("{x:.6}")).unwrap_or_default();
                let fmt_u = |u: Option<f64>| u.map(|x| format!("{x:.6}")).unwrap_or_default();

                if let Err(e) = writeln!(
                    f,
                    "{},{:.6},{:.6},{:.6},{:.6},\
                     {},{},\
                     {},{},\
                     {},{},\
                     {},{},\
                     {},{},\
                     {},{},\
                     {},{},\
                     {},{}",
                    uuid,
                    angle,
                    field,
                    cs,
                    dcs,
                    fmt(params.mean.value),
                    fmt_u(params.mean.uncertainty),
                    fmt(params.sigma.value),
                    fmt_u(params.sigma.uncertainty),
                    fmt(params.fwhm.value),
                    fmt_u(params.fwhm.uncertainty),
                    fmt(params.amplitude.value),
                    fmt_u(params.amplitude.uncertainty),
                    fmt(params.area.value),
                    fmt_u(params.area.uncertainty),
                    fmt(params.energy.value),
                    fmt_u(params.energy.uncertainty),
                    fmt(params.mean.calibrated_value),
                    fmt_u(params.mean.calibrated_uncertainty),
                    fmt(params.fwhm.calibrated_value),
                    fmt_u(params.fwhm.calibrated_uncertainty),
                ) {
                    log::error!("Failed to write row for UUID {uuid}: {e}");
                    return;
                }
            }
        }

        log::info!("Saved full cross sections CSV to {path:?}");
    }

    pub fn cross_section_ui(
        &self,
        ui: &mut egui::Ui,
        histogrammer: &mut histogrammer::Histogrammer,
    ) {
        use egui_extras::{Column, TableBuilder};
        use egui_plot::Plot;
        const EPS: f64 = 1e-12;

        let data = self.get_data_from_histogrammer(histogrammer);
        if data.is_empty() {
            ui.label("No cross-section data found.");
            return;
        }

        // Sort UUIDs for stable layout
        let mut uuids: Vec<usize> = data.keys().copied().collect();
        uuids.sort_unstable();

        let ncols = self.settings.n_columns.max(1);
        let nrows = uuids.len().div_ceil(ncols);

        let available_w = ui.available_width() - 8.0 * (ncols as f32); // 8px gap per column

        let col_w = available_w / (ncols as f32);

        // Build dynamic table: ncols equal plot columns
        let mut table = TableBuilder::new(ui)
            .striped(false)
            .resizable(false) // keep widths stable as the window resizes
            .vscroll(true);

        // Create ncols equal-width, growable columns with a reasonable minimum
        table = table.columns(Column::exact(col_w), ncols);

        let row_h: f32 = col_w / self.settings.view_aspect;

        // Optional header with UUIDs for the first row positions
        table.body(|mut body| {
            for r in 0..nrows {
                body.row(row_h, |mut row| {
                    for c in 0..ncols {
                        let idx = r * ncols + c;
                        row.col(|ui| {
                            if idx >= uuids.len() {
                                return;
                            }
                            let uuid = uuids[idx];
                            let pts: &Vec<(f64, f64, f64, f64, GaussianParameters, Color32, MarkerShape)> = &data[&uuid];

                            // ---------- Build a per-UUID label from params ----------
                            fn auto_fmt(value: Option<f64>, unc: Option<f64>, units: &str) -> String {
                                match value {
                                    Some(val) => {
                                        let unc = unc.unwrap_or(0.0);
                                        if unc > 0.0 && unc.is_finite() {
                                            // 2 sig figs in the uncertainty → decimals from its magnitude
                                            let exp = unc.abs().log10().floor() as i32;
                                            let digits = (-(exp) + 1).max(0) as usize;
                                            format!("{val:.digits$} ± {unc:.digits$} {units}")
                                        } else {
                                            format!("{val:.3} {units}")
                                        }
                                    }
                                    None => "—".to_owned(),
                                }
                            }

                            // Assigned energy (expect same for all points; show “(mixed)” if not)
                            let mut energy_val: Option<f64> = None;
                            let mut energy_unc: Option<f64> = None;
                            let mut energy_mixed = false;

                            // Expand bounds by calibrated uncertainty if available (1σ)
                            let (min_calibrated_mean, max_calibrated_mean) = pts
                                .iter()
                                .filter_map(|(_, _, _, _, p, _, _)| {
                                    if let Some(m) = p.mean.calibrated_value {
                                        let u = p.mean.calibrated_uncertainty.unwrap_or(0.0).abs();
                                        // include both ends so points with uncertainty expand the envelope
                                        Some((m - u, m + u))
                                    } else {
                                        None
                                    }
                                })
                                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), (a, b)| {
                                    (lo.min(a), hi.max(b))
                                });

                            for &(_ang, _y, _dy, _field, ref params, _col, _shape) in pts {
                                if let Some(e) = params.energy.value && e != -1.0 {
                                    match energy_val {
                                        None => {
                                            energy_val = Some(e);
                                            energy_unc = params.energy.uncertainty;
                                        }
                                        Some(prev) if (prev - e).abs() > f64::EPSILON => {
                                            energy_mixed = true;
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            // Compose the label shown on-plot (top-left)
                            let mut label = format!("UUID {uuid}");
                            if let Some(e) = energy_val {
                                let mut line = format!("\nAssigned Energy: {}", auto_fmt(Some(e), energy_unc, "keV"));
                                if energy_mixed {
                                    line.push_str(" (mixed)");
                                }
                                label.push_str(&line);
                            }
                            let (avg_cal_mean, avg_cal_mean_unc) = avg_calibrated_mean(pts);
                            if avg_cal_mean.is_some() {
                                label.push_str(&format!(
                                    "\nAverage Calibrated Mean: {}",
                                    auto_fmt(avg_cal_mean, avg_cal_mean_unc, "keV")
                                ));
                            }

                            // --------------------------------------------------------
                            // ---- Bounds (linear space first) ----
                            let xmin_lin = 0.0;
                            let xmax_lin = 65.0;

                            let mut y_max_lin = 1e-9;
                            let mut y_min_lin = f64::INFINITY;

                            for &(_ang, y, dy, _field, ref _params, _col, _markershape) in pts {
                                // Upper always includes the +unc
                                let hi = (y + dy).max(EPS);

                                // LOWER:
                                // If the bar would go <= 0, use one decade below the datum (y/10)
                                // instead of (y - dy) or EPS.
                                let lo = if y - dy > 0.0 {
                                    y - dy
                                } else {
                                    (y / 10.0).max(EPS)
                                };

                                if hi.is_finite() && hi > y_max_lin {
                                    y_max_lin = hi;
                                }
                                if lo.is_finite() && lo < y_min_lin {
                                    y_min_lin = lo;
                                }
                            }

                            if !y_max_lin.is_finite() || y_max_lin <= 0.0 {
                                y_max_lin = 1.0;
                            }
                            if !y_min_lin.is_finite() || y_min_lin <= 0.0 {
                                y_min_lin = EPS;
                            }

                            // ---- Convert bounds to plot-space ----
                            let (ymin_plot, ymax_plot) = if self.settings.log_scale {
                                let ymin_nice = nice_log_floor(y_min_lin.max(EPS));
                                let ymax_nice = nice_log_ceil(y_max_lin.max(EPS));
                                (ymin_nice.log10(), ymax_nice.log10())
                            } else {
                                (0.0, y_max_lin * 1.1)
                            };

                            let plot_id = ui.id().with(("cs_plot_uuid", uuid));
                            let available_w = ui.available_width();

                            let cs_height = if self.settings.fit_mean { row_h * 0.75 } else { row_h };

                            // ---- Build the Cross section vs Angle plot ----
                            let mut plot = Plot::new(plot_id)
                                .width(available_w)
                                .height(cs_height)
                                .allow_zoom(false)
                                .allow_drag(false)
                                .allow_scroll(false)
                                .allow_double_click_reset(false)
                                .auto_bounds(Vec2b::new(false, false))
                                .label_formatter({
                                    let log_y = self.settings.log_scale;
                                    move |name, value| {
                                        let x = value.x;
                                        let y = if log_y {
                                            10.0f64.powf(value.y)
                                        } else {
                                            value.y
                                        };
                                        if !name.is_empty() {
                                            name.to_owned()
                                        } else {
                                            format!("{x:.2}, {y:.2}")
                                        }
                                    }
                                });

                            if c == 0 {
                                plot = plot.y_axis_label("dΩ/dσ [μb/sr]");
                            }

                            if r == nrows - 1 && !self.settings.fit_mean {
                                plot = plot.x_axis_label("θ [°]");
                            }

                            // Log Y ticks/formatter if requested
                            if self.settings.log_scale {
                                let max_size = 4;
                                plot = plot.y_grid_spacer(log_axis_spacer).y_axis_formatter(
                                    move |gm, bounds| log_axis_formatter(gm, bounds, max_size),
                                );
                            }

                            let label_color = if ui.visuals().dark_mode {
                                egui::Color32::LIGHT_BLUE
                            } else {
                                egui::Color32::BLACK
                            };

                            let mean_color = if ui.visuals().dark_mode {
                                egui::Color32::LIGHT_BLUE
                            } else {
                                egui::Color32::DARK_BLUE
                            };

                            let assigned_energy_color = if ui.visuals().dark_mode {
                                egui::Color32::LIGHT_GREEN
                            } else {
                                egui::Color32::DARK_GREEN
                            };

                            // Render Cross section vs angle
                            plot.show(ui, |pui| {
                                // create hash map of uuid, angle, cross section, color
                                let mut uuid_map: std::collections::HashMap<usize, Vec<(f64, f64, f64, egui::Color32)>> = std::collections::HashMap::new();
                                for &(ang, y, dy, field, ref params, col, markershape) in pts {
                                    let uuid: usize = params.uuid;

                                    // Base label
                                    let mut name = format!(
                                        "UUID {uuid}\nAngle: {ang:.1}°\ndΩ/dσ: {y:.2} ± {dy:.2} [μb/sr]\nMagnetic Field: {field:.2} kG"
                                    );

                                    let gaussian_summary = params.summary_string(Some("keV"), Some("mm"));
                                    if !gaussian_summary.is_empty() {
                                        name.push_str(&format!("\n{gaussian_summary}"));
                                    }

                                    uuid_map.entry(uuid).or_default().push((ang, y, dy, col));

                                    let mut point = EguiPoints::new_cross_section(
                                        &name,
                                        ang,
                                        y,
                                        dy,
                                        col,
                                    );
                                    point.log_y = self.settings.log_scale;
                                    point.radius = self.settings.markersize;
                                    point.shape = Some(markershape);
                                    point.draw(pui, true);
                                }

                                let mut uuids: Vec<_> = uuid_map.keys().copied().collect();
                                uuids.sort_unstable();

                                for uuid in uuids {
                                    let points = &uuid_map[&uuid];
                                    let mut sorted_points = points.clone(); // (ang_deg, y, dy, color)
                                    const ANG_MERGE_TOL_DEG: f64 = 0.05;
                                    let averaged = average_points_by_angle(&mut sorted_points, ANG_MERGE_TOL_DEG);

                                    // build the working arrays (θ in degrees, y = dσ/dΩ as plotted)
                                    let angles_deg: Vec<f64> = averaged.iter().map(|(ang, _, _, _)| *ang).collect();
                                    let dsdo:       Vec<f64> = averaged.iter().map(|(_, y,  _, _)| *y ).collect();

                                    // integrate dσ/dΩ vs θ (in degrees), NumPy-trapz equivalent
                                    let total_diff = match self.settings.integration_rule {
                                        IntegrationRule::Left        => integrate_left_endpoint(&angles_deg, &dsdo),
                                        IntegrationRule::Right       => integrate_right_endpoint(&angles_deg, &dsdo),
                                        IntegrationRule::Midpoint    => integrate_midpoint(&angles_deg, &dsdo),
                                        IntegrationRule::Trapezoidal => integrate_trapezoidal(&angles_deg, &dsdo),
                                    };

                                    label.push_str(&format!(
                                        "\ndσ/dΩ [Total] = {total_diff:.2} μb"
                                    ));

                                    // log-safe baseline for fills
                                    let baseline_y = if self.settings.log_scale {
                                        ymin_plot.max(1e-12)
                                    } else {
                                        0.0
                                    };

                                    if self.settings.show_integration {
                                        // draw translucent polygons per interval (visible dσ/dΩ vs θ)
                                        for i in 0..(angles_deg.len() - 1) {
                                            let x0_deg = angles_deg[i];
                                            let x1_deg = angles_deg[i + 1];
                                            let y0_vis = dsdo[i];
                                            let y1_vis = dsdo[i + 1];
                                            let color  = averaged[i].3;
                                            let poly_pts = cell_polygon_with_baseline(
                                                self.settings.integration_rule,
                                                x0_deg, x1_deg, y0_vis, y1_vis, baseline_y,
                                                self.settings.log_scale,
                                            );
                                            let poly_vec: Vec<[f64; 2]> = poly_pts.into();

                                            // apply 30% opacity to the color
                                            let color_with_alpha = egui::Color32::from_rgba_unmultiplied(
                                                color.r(), color.g(), color.b(), (255.0 * 0.3) as u8,
                                            );

                                            let poly = egui_plot::Polygon::new(format!("uuid_{uuid}_cell_{i}"), poly_vec)
                                                .fill_color(color_with_alpha)
                                                .stroke(egui::Stroke::NONE);
                                            pui.polygon(poly);
                                        }
                                    }
                                }

                                // Put the per-UUID label in the top-left corner
                                let label_item = egui_plot::Text::new(
                                    format!("uuid_label_{uuid}"),              // item name/id
                                    egui_plot::PlotPoint::new(xmin_lin + 1.0, ymax_plot),
                                    label,                                     // the string we built
                                )
                                .anchor(egui::Align2::LEFT_TOP).color(label_color);

                                pui.text(label_item);

                                // X bounds are linear in both modes
                                pui.set_plot_bounds_x(xmin_lin..=xmax_lin);
                                // Y bounds are in plot coords (log-transformed if log mode)
                                pui.set_plot_bounds_y(ymin_plot..=ymax_plot);
                            });

                            if self.settings.fit_mean {
                                let mut mean_plot = Plot::new(plot_id)
                                    .width(available_w)
                                    .height(row_h * 0.25)
                                    .allow_zoom(false)
                                    .allow_drag(false)
                                    .allow_scroll(false)
                                    .allow_double_click_reset(false)
                                    .auto_bounds(Vec2b::new(false, false))
                                    .label_formatter({
                                        move |name, value| {
                                            let x = value.x;
                                            let y = value.y;
                                            if !name.is_empty() {
                                                name.to_owned()
                                            } else {
                                                format!("{x:.2}, {y:.2}")
                                            }
                                        }
                                });

                                if c == 0 {
                                    mean_plot = mean_plot.y_axis_label("Mean [keV]");
                                }
                                if r == nrows - 1 {
                                    mean_plot = mean_plot.x_axis_label("θ [°]");
                                }

                                mean_plot.show(ui, |pui| {
                                    for &(ang, _y, _dy, field, ref params, col, markershape) in pts {

                                        if let Some(mean) = params.mean.calibrated_value {
                                            let mean_unc = params.mean.calibrated_uncertainty.unwrap_or(0.0);
                                            let name = format!(
                                                "Mean Check\nUUID {}\nAngle: {ang:.1}°\nCalibrated Mean: {mean:.2} ± {mean_unc:.2} keV\nMagnetic Field: {field:.2} kG",
                                                params.uuid
                                            );
                                            let mut mean_point = EguiPoints::new_cross_section(
                                                &name,
                                                ang,
                                                mean,
                                                mean_unc,
                                                col,
                                            );
                                            mean_point.log_y = false;
                                            mean_point.radius = self.settings.markersize;
                                            mean_point.shape = Some(markershape);
                                            mean_point.draw(pui, false);
                                        }
                                    }

                                    // horizontal line at the average calibrated mean
                                    if let Some(avg) = avg_cal_mean {
                                        let avg_cal_unc = avg_cal_mean_unc.unwrap_or(0.0);
                                        let upper_value = avg + avg_cal_unc;
                                        let lower_value = avg - avg_cal_unc;

                                        let points = vec![
                                            [xmin_lin, avg],
                                            [xmax_lin, avg],
                                        ];
                                        let mut upper_line = EguiLine::new_with_points(points.clone());
                                        let mut lower_line = EguiLine::new_with_points(points);

                                        upper_line.color = mean_color;
                                        upper_line.reference_fill = true;
                                        upper_line.fill = upper_value as f32;

                                        lower_line.color = mean_color;
                                        lower_line.reference_fill = true;
                                        lower_line.fill = lower_value as f32;

                                        upper_line.draw(pui, None);
                                        lower_line.draw(pui, None);

                                    }

                                    if let Some(e) = energy_val {
                                        let assigned_energy_unc = energy_unc.unwrap_or(0.0);
                                        let lower_energy = e - assigned_energy_unc;
                                        let upper_energy = e + assigned_energy_unc;

                                        let points = vec![
                                            [xmin_lin, e],
                                            [xmax_lin, e],
                                        ];
                                        let mut energy_line_upper = EguiLine::new_with_points(points.clone());
                                        let mut energy_line_lower = EguiLine::new_with_points(points);
                                        energy_line_lower.color = assigned_energy_color;
                                        energy_line_lower.reference_fill = true;
                                        energy_line_lower.fill = lower_energy as f32;

                                        energy_line_upper.color = assigned_energy_color;
                                        energy_line_upper.reference_fill = true;
                                        energy_line_upper.fill = upper_energy as f32;
                                        energy_line_lower.draw(pui, None);
                                        energy_line_upper.draw(pui, None);
                                    }

                                    pui.set_plot_bounds_x(xmin_lin..=xmax_lin);
                                    pui.set_plot_bounds_y(min_calibrated_mean..=max_calibrated_mean);

                                });
                            }
                        });
                    }
                });
            }
        });
    }
}
