use super::histogram1d::Histogram;

use crate::fitter::common::Data;
use crate::fitter::main_fitter::{FitModel, Fitter};

impl Histogram {
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

        fitter.background_model = background_model;
        fitter.background_result = background_result;

        fitter.fit_model = FitModel::Gaussian(
            region_markers.clone(),
            peak_positions.clone(),
            background_markers.clone(),
            equal_stdev,
            free_position,
        );

        fitter.fit();

        self.plot_settings.markers.clear_peak_markers();
        let updated_markers = fitter.get_peak_markers();
        for marker in updated_markers {
            self.plot_settings.markers.add_peak_marker(marker);
        }

        fitter.set_name(self.name.clone());
        self.fits.temp_fit = Some(fitter);

        // calibrate temp fit if calibration is enabled
        if self.fits.settings.calibrated {
            if let Some(temp_fit) = &mut self.fits.temp_fit {
                temp_fit.calibrate(&self.fits.calibration);
            }
        }
    }
}
