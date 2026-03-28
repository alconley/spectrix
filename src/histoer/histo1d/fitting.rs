use super::histogram1d::Histogram;

use crate::fitter::common::Data;
use crate::fitter::main_fitter::{BackgroundModel, FitModel, FitResult, Fitter};

impl Histogram {
    pub fn apply_refit_all_request(&mut self) {
        if !self.fits.take_pending_refit_all() {
            return;
        }

        let fit_count = self.fits.stored_fits.len();
        for _ in 0..fit_count {
            self.fits.pending_modify_fit = Some(0);
            self.apply_modify_fit_request();
            self.fit_gaussians();
            self.fits.store_temp_fit();
        }
    }

    pub fn apply_modify_fit_request(&mut self) {
        let Some(fit_idx) = self.fits.take_pending_modify_fit() else {
            return;
        };

        let Some((metadata, metadata_found, fallback_background_model, mut moved_fit)) =
            self.fits.stored_fits.get(fit_idx).and_then(|stored_fit| {
                if let Some(FitResult::Gaussian(gaussian)) = &stored_fit.fit_result {
                    let (metadata, metadata_found) = gaussian.fit_metadata_with_fallback();
                    Some((
                        metadata,
                        metadata_found,
                        stored_fit.background_model.clone(),
                        stored_fit.clone(),
                    ))
                } else {
                    None
                }
            })
        else {
            log::warn!("Modify fit requested for non-Gaussian fit.");
            return;
        };

        if !metadata_found {
            log::warn!(
                "Fit metadata was not found; using fallback marker data derived from Gaussian parameters."
            );
        }

        self.plot_settings.markers.clear_background_markers();
        self.plot_settings.markers.clear_peak_markers();
        self.plot_settings.markers.clear_region_markers();

        for marker in metadata.region_markers {
            self.plot_settings.markers.add_region_marker(marker);
        }
        for marker in metadata.peak_markers {
            self.plot_settings.markers.add_peak_marker(marker);
        }

        self.fits.settings.background_model = match metadata.background_model.as_str() {
            "linear" => BackgroundModel::Linear(Default::default()),
            "quadratic" => BackgroundModel::Quadratic(Default::default()),
            "exponential" => BackgroundModel::Exponential(Default::default()),
            "powerlaw" => BackgroundModel::PowerLaw(Default::default()),
            "None" => BackgroundModel::None,
            _ => fallback_background_model,
        };

        if matches!(self.fits.settings.background_model, BackgroundModel::None) {
            self.plot_settings.markers.clear_background_markers();
        } else {
            self.plot_settings
                .markers
                .set_background_marker_positions(&metadata.background_markers);
            self.update_background_pair_lines();
        }

        if fit_idx < self.fits.stored_fits.len() {
            self.fits.stored_fits.remove(fit_idx);
        }

        moved_fit.name = format!("{} (Temp)", moved_fit.name);
        self.fits.temp_fit = Some(moved_fit);
    }

    pub fn fit_background(&mut self) {
        log::info!("Fitting background for histogram: {}", self.name);
        self.fits.temp_fit = None;

        let marker_positions = self.plot_settings.markers.get_background_marker_positions();
        if marker_positions.is_empty() {
            log::error!("Need to set at least one background marker pair to fit the histogram");
            return;
        }

        let mut x_data = Vec::new();
        let mut y_data = Vec::new();

        for (start_x, end_x) in marker_positions {
            let bin_centers = self.get_bin_centers_between(start_x, end_x);
            let bin_counts = self.get_bin_counts_between(start_x, end_x);

            x_data.extend(bin_centers);
            y_data.extend(bin_counts);
        }

        if x_data.is_empty() || y_data.is_empty() {
            log::error!("No valid data points found between background markers.");
            return;
        }

        let mut fitter = Fitter::new(Data {
            x: x_data,
            y: y_data,
        });

        fitter.background_model = self.fits.settings.background_model.clone();
        fitter.fit_background();

        fitter.name = format!("{} Temp Fit", self.name);
        fitter.set_name(self.name.clone());

        self.fits.temp_fit = Some(fitter);
    }

    pub fn fit_gaussians(&mut self) {
        let previous_peak_assignments = self
            .fits
            .temp_fit
            .as_ref()
            .and_then(|temp_fit| match &temp_fit.fit_result {
                Some(FitResult::Gaussian(g)) => Some(
                    g.fit_result
                        .iter()
                        .filter_map(|p| {
                            p.mean.value.map(|m| {
                                (
                                    m,
                                    p.uuid,
                                    p.energy.value.unwrap_or(-1.0),
                                    p.energy.uncertainty.unwrap_or(0.0),
                                )
                            })
                        })
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .unwrap_or_default();

        let region_markers = self.plot_settings.markers.get_region_marker_positions();
        let peak_positions = self.plot_settings.markers.get_peak_marker_positions();
        let background_markers = self.plot_settings.markers.get_background_marker_positions();

        let centers = self.get_bin_centers();
        let counts = self.bins.clone();

        let data = Data {
            x: centers,
            y: counts.iter().map(|&c| c as f64).collect(),
        };

        let mut fitter = Fitter::new(data);

        let background_model = self.fits.settings.background_model.clone();

        let background_result = if let Some(temp_fit) = &self.fits.temp_fit {
            fitter.background_line = temp_fit.background_line.clone();
            temp_fit.background_result.clone()
        } else {
            None
        };

        let equal_stdev = self.fits.settings.equal_stddev;
        let free_position = self.fits.settings.free_position;

        fitter.calibration = self.fits.calibration.clone();

        fitter.background_model = background_model;
        fitter.background_result = background_result;

        // build optional σ-bounds from UI; when UI is “calibrated”, these are energy-bounds
        let sigma_bounds_ui = if self.fits.settings.constrain_sigma {
            Some((self.fits.settings.sigma_min, self.fits.settings.sigma_max))
        } else {
            None
        };
        let bounds_are_calibrated = self.fits.settings.calibrated;

        fitter.fit_model = FitModel::Gaussian(
            region_markers.clone(),
            peak_positions.clone(),
            background_markers.clone(),
            equal_stdev,
            free_position,
            sigma_bounds_ui,       // <- NEW: (min,max) from UI if enabled
            bounds_are_calibrated, // <- NEW: interpret bounds as energy if true
        );

        fitter.fit();

        self.plot_settings.markers.clear_peak_markers();
        let updated_markers = if let Some(FitResult::Gaussian(g)) = &fitter.fit_result {
            g.fit_result
                .iter()
                .filter_map(|p| p.mean.value)
                .collect::<Vec<_>>()
        } else {
            fitter.get_peak_markers()
        };
        for marker in updated_markers {
            self.plot_settings.markers.add_peak_marker(marker);
        }

        fitter.set_name(self.name.clone());
        self.fits.temp_fit = Some(fitter);
        if matches!(
            self.fits
                .temp_fit
                .as_ref()
                .and_then(|temp_fit| temp_fit.fit_result.as_ref()),
            Some(FitResult::Gaussian(_))
        ) {
            self.fits.settings.show_fit_stats = true;
        }

        // Preserve UUID and energy assignments across modify -> refit workflows.
        if !previous_peak_assignments.is_empty()
            && let Some(temp_fit) = &mut self.fits.temp_fit
            && let Some(FitResult::Gaussian(g)) = &mut temp_fit.fit_result
        {
            let mut prev_sorted = previous_peak_assignments.clone();
            prev_sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

            let mut new_sorted: Vec<(usize, f64)> = g
                .fit_result
                .iter()
                .enumerate()
                .filter_map(|(idx, p)| p.mean.value.map(|m| (idx, m)))
                .collect();
            new_sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            for ((new_idx, _), (_, uuid, energy, energy_unc)) in
                new_sorted.into_iter().zip(prev_sorted.into_iter())
            {
                if let Err(e) = g.update_uuid_for_peak(new_idx, uuid) {
                    log::warn!("Failed to preserve UUID for peak {new_idx}: {e}");
                }
                if let Err(e) = g.update_energy_for_peak(new_idx, energy, energy_unc) {
                    log::warn!("Failed to preserve energy for peak {new_idx}: {e}");
                }
            }
        }

        // calibrate temp fit if calibration is enabled
        if self.fits.settings.calibrated
            && let Some(temp_fit) = &mut self.fits.temp_fit
        {
            temp_fit.calibrate(&self.fits.calibration);
        }
    }
}
