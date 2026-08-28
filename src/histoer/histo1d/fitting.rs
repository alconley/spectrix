use super::histogram1d::Histogram;

use crate::fitter::common::Data;
use crate::fitter::main_fitter::{BackgroundModel, FitModel, FitResult, Fitter};
use spectrix_fitting::{
    BackgroundCoupling, FitOptions as NativeFitOptions, ManualSeedEstimateRequest,
    estimate_manual_peak_seeds,
};
use std::hash::{Hash, Hasher};

fn background_coupling_for_fit(
    background_model: &BackgroundModel,
    lock_manual_background: bool,
) -> BackgroundCoupling {
    if lock_manual_background && !matches!(background_model, BackgroundModel::None) {
        BackgroundCoupling::PrefitFrozen
    } else {
        BackgroundCoupling::PrefitJoint
    }
}

fn reusable_manual_background(temp_fit: Option<&Fitter>) -> Option<&Fitter> {
    temp_fit.filter(|fit| fit.background_was_fit_manually && fit.background_result.is_some())
}

impl Histogram {
    fn resolved_objective(&self, range: Option<(f64, f64)>) -> spectrix_fitting::ObjectiveKind {
        let counts = range.map_or_else(
            || self.bins.iter().map(|count| *count as f64).collect(),
            |(start, end)| self.get_bin_counts_between(start, end),
        );
        self.fits.settings.objective.resolve(counts)
    }

    fn manual_estimate_signature(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.bin_width.to_bits().hash(&mut hasher);
        self.bins.hash(&mut hasher);
        format!("{:?}", self.fits.settings.background_model).hash(&mut hasher);
        format!("{:?}", self.fits.settings.objective).hash(&mut hasher);
        self.fits.settings.equal_stddev.hash(&mut hasher);
        for value in self
            .plot_settings
            .markers
            .get_region_marker_positions()
            .into_iter()
            .chain(self.plot_settings.markers.get_peak_marker_positions())
        {
            value.to_bits().hash(&mut hasher);
        }
        for (start, end) in self.plot_settings.markers.get_background_marker_positions() {
            start.to_bits().hash(&mut hasher);
            end.to_bits().hash(&mut hasher);
        }
        hasher.finish().max(1)
    }

    pub fn refresh_manual_peak_guesses(&mut self) {
        if matches!(
            self.fits.settings.background_model,
            BackgroundModel::LegacyAuto
        ) {
            self.fits.settings.background_model = BackgroundModel::None;
        }
        self.plot_settings.markers.ensure_peak_ids();
        if self.plot_settings.markers.peak_markers.is_empty() {
            self.plot_settings.markers.preview_background.clear();
            self.plot_settings.markers.estimate_error = None;
            return;
        }
        let signature = self.manual_estimate_signature();
        if self.plot_settings.markers.estimate_signature == signature {
            return;
        }

        let region_markers = self.plot_settings.markers.get_region_marker_positions();
        if region_markers.len() != 2 {
            self.plot_settings.markers.estimate_error =
                Some("Place exactly two region markers to estimate peak guesses.".to_owned());
            self.plot_settings.markers.preview_background.clear();
            self.plot_settings.markers.estimate_signature = signature;
            return;
        }
        let background_markers = self.plot_settings.markers.get_background_marker_positions();
        if !matches!(self.fits.settings.background_model, BackgroundModel::None)
            && background_markers.is_empty()
        {
            self.plot_settings.markers.estimate_error = Some(
                "Place at least one background marker window for the selected background model."
                    .to_owned(),
            );
            self.plot_settings.markers.preview_background.clear();
            self.plot_settings.markers.estimate_signature = signature;
            return;
        }

        let equal_sigma = self.fits.settings.equal_stddev;
        let displayed_widths_disagree =
            self.plot_settings
                .markers
                .peak_markers
                .windows(2)
                .any(|pair| {
                    let scale = pair[0].fwhm.abs().max(pair[1].fwhm.abs()).max(1.0);
                    (pair[0].fwhm - pair[1].fwhm).abs() > f64::EPSILON * scale * 16.0
                });
        let force_shared_width = equal_sigma
            && (self.plot_settings.markers.last_equal_sigma == Some(false)
                || displayed_widths_disagree);
        let request = ManualSeedEstimateRequest {
            x: self.get_bin_centers(),
            y: self.bins.iter().map(|count| *count as f64).collect(),
            bin_width: self.bin_width,
            region: [region_markers[0], region_markers[1]],
            peak_markers: self.plot_settings.markers.get_peak_marker_positions(),
            background_markers,
            background: crate::fitter::native::background_kind(
                &self.fits.settings.background_model,
            ),
            background_seed: Some(crate::fitter::native::background_seed(
                &self.fits.settings.background_model,
                None,
            )),
            equal_sigma,
        };
        let options = NativeFitOptions {
            objective: self.resolved_objective(Some((region_markers[0], region_markers[1]))),
            ..NativeFitOptions::default()
        };
        match estimate_manual_peak_seeds(&request, &options) {
            Ok(estimate) => {
                self.plot_settings.markers.preview_background = estimate
                    .background_prefit
                    .evaluation_x
                    .iter()
                    .copied()
                    .zip(estimate.background_prefit.best_fit.iter().copied())
                    .map(Into::into)
                    .collect();
                for (guess, estimated) in self
                    .plot_settings
                    .markers
                    .peak_markers
                    .iter_mut()
                    .zip(estimate.peaks)
                {
                    if force_shared_width
                        || guess.width_source
                            == crate::histoer::histo1d::markers::GuessSource::Estimated
                    {
                        let fwhm = 2.354_82 * estimated.seed.sigma;
                        if guess.amplitude_source
                            != crate::histoer::histo1d::markers::GuessSource::Estimated
                        {
                            guess.set_fwhm_preserving_height(fwhm, self.bin_width);
                        } else {
                            guess.fwhm = fwhm;
                        }
                        guess.width_source =
                            crate::histoer::histo1d::markers::GuessSource::Estimated;
                    }
                    if guess.amplitude_source
                        == crate::histoer::histo1d::markers::GuessSource::Estimated
                    {
                        guess.amplitude = estimated.seed.amplitude;
                    }
                    if guess.bounds_source
                        == crate::histoer::histo1d::markers::GuessSource::Estimated
                    {
                        guess.center_min = estimated.bounds.center[0];
                        guess.center_max = estimated.bounds.center[1];
                        guess.fwhm_min = 2.354_82 * estimated.bounds.sigma[0];
                        guess.fwhm_max = 2.354_82 * estimated.bounds.sigma[1];
                        guess.net_height_min = estimated.bounds.net_height[0];
                        guess.net_height_max = estimated.bounds.net_height[1];
                    }
                    guess.net_height = guess.model_height(self.bin_width);
                    guess.clean_width = estimated.clean_width;
                    guess.valid = guess.fwhm.is_finite()
                        && guess.fwhm > 0.0
                        && guess.amplitude.is_finite()
                        && guess.amplitude > 0.0
                        && guess.bounds_valid();
                }
                self.plot_settings.markers.estimate_error = None;
            }
            Err(error) => {
                self.plot_settings.markers.preview_background.clear();
                self.plot_settings.markers.estimate_error = Some(error.to_string());
                for guess in &mut self.plot_settings.markers.peak_markers {
                    if guess.width_source
                        == crate::histoer::histo1d::markers::GuessSource::Estimated
                        || guess.amplitude_source
                            == crate::histoer::histo1d::markers::GuessSource::Estimated
                    {
                        guess.valid = false;
                    }
                }
            }
        }
        self.plot_settings.markers.last_equal_sigma = Some(equal_sigma);
        self.plot_settings.markers.estimate_signature = signature;
    }

    pub fn invalidate_manual_gaussian_preview(&mut self) {
        self.plot_settings.markers.estimate_signature = 0;
        if self
            .fits
            .temp_fit
            .as_ref()
            .is_some_and(|fit| matches!(&fit.fit_result, Some(FitResult::Gaussian(_))))
        {
            self.fits.temp_fit = None;
        }
    }

    fn manual_fit_readiness(&self) -> Result<(), String> {
        let markers = &self.plot_settings.markers;
        if markers.region_markers.len() != 2 {
            return Err("Place exactly two region markers before fitting.".to_owned());
        }
        if markers.peak_markers.is_empty() {
            return Err("Place at least one peak marker before fitting.".to_owned());
        }
        if !matches!(self.fits.settings.background_model, BackgroundModel::None)
            && markers.background_markers.is_empty()
        {
            return Err(
                "Place at least one background marker window for this background model.".to_owned(),
            );
        }
        if let Some(error) = &markers.estimate_error {
            return Err(error.clone());
        }
        if markers.peak_markers.iter().any(|guess| !guess.valid) {
            return Err(
                "Every peak needs a positive width and amplitude; move it or re-estimate it."
                    .to_owned(),
            );
        }
        Ok(())
    }

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

        let Some((
            metadata,
            metadata_found,
            fallback_background_model,
            background_coupling,
            mut moved_fit,
        )) = self.fits.stored_fits.get(fit_idx).and_then(|stored_fit| {
            if let Some(FitResult::Gaussian(gaussian)) = &stored_fit.fit_result {
                let (metadata, metadata_found) = gaussian.fit_metadata_with_fallback();
                Some((
                    metadata,
                    metadata_found,
                    stored_fit.background_model.clone(),
                    gaussian.background_coupling,
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
        if metadata.peak_seeds.is_empty() {
            for marker in metadata.peak_markers {
                self.plot_settings.markers.add_peak_marker(marker);
            }
        } else {
            for (index, seed) in metadata.peak_seeds.iter().copied().enumerate() {
                self.plot_settings.markers.add_peak_seed(
                    seed,
                    crate::histoer::histo1d::markers::GuessSource::Fitted,
                    self.bin_width,
                    metadata.peak_bounds.get(index).copied(),
                );
            }
        }

        self.fits.settings.background_model = match metadata.background_model.as_str() {
            "auto" | "Auto" => BackgroundModel::None,
            "constant" => crate::fitter::native::concrete_background_model(
                spectrix_fitting::BackgroundKind::Constant,
            ),
            "linear" => BackgroundModel::Linear(Default::default()),
            "quadratic" => BackgroundModel::Quadratic(Default::default()),
            "exponential" => BackgroundModel::Exponential(Default::default()),
            "powerlaw" => BackgroundModel::PowerLaw(Default::default()),
            "None" => BackgroundModel::None,
            _ => fallback_background_model,
        };
        self.fits.settings.background_coupling = background_coupling;
        self.fits.settings.lock_background = background_coupling
            == BackgroundCoupling::PrefitFrozen
            && moved_fit.background_was_fit_manually;

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

        let objective = self.fits.settings.objective.resolve(y_data.iter().copied());
        let mut fitter = Fitter::new(Data {
            x: x_data,
            y: y_data,
        });

        fitter.background_model = self.fits.settings.background_model.clone();
        fitter.objective = objective;
        fitter.fit_background();
        // Background markers choose the samples used by the fit, but the fitted model should
        // remain visible across the histogram instead of only inside those narrow windows.
        fitter.set_background_display_range(self.range);
        if let Some(background) = &fitter.background_result {
            self.fits.settings.apply_background_fit(background);
        }

        fitter.name = format!("{} Temp Fit", self.name);
        fitter.set_name(self.name.clone());
        self.fits.style_temporary_fit(&mut fitter);

        self.fits.temp_fit = Some(fitter);
    }

    pub fn fit_gaussians(&mut self) {
        self.refresh_manual_peak_guesses();
        if let Err(error) = self.manual_fit_readiness() {
            log::error!("Fit aborted: {error}");
            return;
        }
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
        let peak_seeds = self.plot_settings.markers.get_peak_seeds();
        let peak_bounds = self.plot_settings.markers.get_peak_bounds();
        let background_markers = self.plot_settings.markers.get_background_marker_positions();

        let centers = self.get_bin_centers();
        let counts = self.bins.clone();

        let data = Data {
            x: centers,
            y: counts.iter().map(|&c| c as f64).collect(),
        };

        let mut fitter = Fitter::new(data);

        let background_model = self.fits.settings.background_model.clone();

        let manual_background = reusable_manual_background(self.fits.temp_fit.as_ref());
        let background_result = if let Some(temp_fit) = manual_background {
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
        fitter.background_was_fit_manually = manual_background.is_some();
        let lock_manual_background =
            self.fits.settings.lock_background && manual_background.is_some();
        // Background parameters vary with the peaks unless the user explicitly locks a manual
        // background fit. The legacy coupling field records the effective persisted behavior.
        fitter.background_coupling =
            background_coupling_for_fit(&fitter.background_model, lock_manual_background);
        fitter.objective = self.resolved_objective(Some((region_markers[0], region_markers[1])));
        fitter.manual_peak_bounds = Some(peak_bounds);

        // build optional σ-bounds from UI; when UI is “calibrated”, these are energy-bounds
        let sigma_bounds_ui = if self.fits.settings.constrain_sigma {
            Some((self.fits.settings.sigma_min, self.fits.settings.sigma_max))
        } else {
            None
        };
        let bounds_are_calibrated = self.fits.settings.calibrated;

        fitter.fit_model = FitModel::Gaussian(
            region_markers.clone(),
            peak_seeds,
            background_markers.clone(),
            equal_stdev,
            free_position,
            sigma_bounds_ui,       // <- NEW: (min,max) from UI if enabled
            bounds_are_calibrated, // <- NEW: interpret bounds as energy if true
        );

        fitter.fit();
        if fitter.background_was_fit_manually
            && let Some(background) = &fitter.background_result
        {
            self.fits.settings.apply_background_fit(background);
        }

        if let Some(FitResult::Gaussian(gaussian)) = &fitter.fit_result {
            for (guess, fitted) in self
                .plot_settings
                .markers
                .peak_markers
                .iter_mut()
                .zip(&gaussian.fit_result)
            {
                if let (Some(center), Some(fwhm), Some(amplitude)) =
                    (fitted.mean.value, fitted.fwhm.value, fitted.amplitude.value)
                {
                    guess.center.x_value = center;
                    guess.fwhm = fwhm;
                    guess.amplitude = amplitude;
                    guess.net_height = guess.model_height(self.bin_width);
                    guess.width_source = crate::histoer::histo1d::markers::GuessSource::Fitted;
                    guess.amplitude_source = crate::histoer::histo1d::markers::GuessSource::Fitted;
                    guess.valid = center.is_finite()
                        && fwhm.is_finite()
                        && fwhm > 0.0
                        && amplitude.is_finite()
                        && amplitude > 0.0;
                }
            }
            self.plot_settings
                .markers
                .peak_markers
                .sort_by(|left, right| left.center.x_value.total_cmp(&right.center.x_value));
        }

        fitter.set_name(self.name.clone());
        self.fits.style_temporary_fit(&mut fitter);
        self.fits.temp_fit = Some(fitter);
        self.fits.settings.show_fit_stats = true;

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
                new_sorted.into_iter().zip(prev_sorted)
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

#[cfg(test)]
mod tests {
    use super::{background_coupling_for_fit, reusable_manual_background};
    use crate::fitter::{
        main_fitter::{BackgroundModel, BackgroundResult, Fitter},
        models::linear::LinearFitter,
    };
    use spectrix_fitting::BackgroundCoupling;

    #[test]
    fn every_unlocked_background_is_refined_with_the_peak_model() {
        for model in [
            BackgroundModel::Linear(Default::default()),
            BackgroundModel::Quadratic(Default::default()),
            BackgroundModel::PowerLaw(Default::default()),
            BackgroundModel::Exponential(Default::default()),
        ] {
            assert_eq!(
                background_coupling_for_fit(&model, false),
                BackgroundCoupling::PrefitJoint
            );
        }
        assert_eq!(
            background_coupling_for_fit(&BackgroundModel::Quadratic(Default::default()), true,),
            BackgroundCoupling::PrefitFrozen
        );
    }

    #[test]
    fn only_explicit_background_fits_are_reused() {
        let mut fitter =
            Fitter {
                background_result: Some(BackgroundResult::Linear(
                    LinearFitter::new_from_parameters((0.2, 0.01), (3.0, 0.1), 0.0, 10.0),
                )),
                ..Fitter::default()
            };

        assert!(reusable_manual_background(Some(&fitter)).is_none());
        fitter.background_was_fit_manually = true;
        assert!(reusable_manual_background(Some(&fitter)).is_some());
    }
}
