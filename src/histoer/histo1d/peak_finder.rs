use find_peaks::Peak;
use find_peaks::PeakFinder;

use super::histogram1d::Histogram;
use crate::fitter::main_fitter::BackgroundResult;

impl Histogram {
    fn active_background_result(&self) -> Option<BackgroundResult> {
        self.fits
            .temp_fit
            .as_ref()
            .and_then(|fit| fit.background_result.clone())
            .or_else(|| {
                self.fits
                    .stored_fits
                    .iter()
                    .rev()
                    .find_map(|fit| fit.background_result.clone())
            })
    }

    pub fn find_peaks(&mut self) {
        let region_marker_positions = self.plot_settings.markers.get_region_marker_positions();
        let (x_data, mut y_data) = if region_marker_positions.len() == 2 {
            let start_x = region_marker_positions[0];
            let end_x = region_marker_positions[1];

            (
                self.get_bin_centers_between(start_x, end_x),
                self.get_bin_counts_between(start_x, end_x),
            )
        } else {
            (
                self.get_bin_centers(),
                self.bins.iter().map(|&count| count as f64).collect(),
            )
        };

        if let Some(background_result) = self.active_background_result() {
            for (x, y) in x_data.iter().zip(&mut y_data) {
                let background = background_result.evaluate(*x);
                if background.is_finite() {
                    *y -= background;
                }
            }
        }

        self.plot_settings.markers.clear_peak_markers();

        for peak in self.plot_settings.find_peaks_settings.find_peaks(&y_data) {
            if let Some(x) = x_data.get(peak.middle_position()) {
                self.plot_settings.markers.add_peak_marker(*x);
            }
        }
    }
}

fn float_setting_row(
    ui: &mut egui::Ui,
    enabled: &mut bool,
    value: &mut f64,
    label: &str,
    hover_text: &str,
) {
    ui.horizontal(|ui| {
        ui.checkbox(enabled, label).on_hover_text(hover_text);
        if *enabled {
            ui.add(
                egui::DragValue::new(value)
                    .speed(1.0)
                    .range(0.0..=f64::INFINITY),
            )
            .on_hover_text(hover_text);
        }
    });
}

fn usize_setting_row(
    ui: &mut egui::Ui,
    enabled: &mut bool,
    value: &mut usize,
    label: &str,
    hover_text: &str,
) {
    ui.horizontal(|ui| {
        ui.checkbox(enabled, label).on_hover_text(hover_text);
        if *enabled {
            ui.add(egui::DragValue::new(value).speed(1.0).range(0..=usize::MAX))
                .on_hover_text(hover_text);
        }
    });
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeakFindingSettings {
    min_height: f64,
    max_height: f64,
    min_prominence: f64,
    max_prominence: f64,
    min_difference: f64,
    max_difference: f64,
    min_plateau_size: usize,
    max_plateau_size: usize,
    min_distance: usize,
    max_distance: usize,

    enable_min_height: bool,
    enable_max_height: bool,
    enable_min_prominence: bool,
    enable_max_prominence: bool,
    enable_min_difference: bool,
    enable_max_difference: bool,
    enable_min_plateau_size: bool,
    enable_max_plateau_size: bool,
    enable_min_distance: bool,
    enable_max_distance: bool,
}

impl Default for PeakFindingSettings {
    fn default() -> Self {
        Self {
            min_height: 10.0,
            max_height: 0.0,
            min_prominence: 8.0,
            max_prominence: 0.0,
            min_difference: 2.0,
            max_difference: 0.0,
            min_plateau_size: 1,
            max_plateau_size: 1,
            min_distance: 4,
            max_distance: 0,

            enable_min_height: false,
            enable_max_height: false,
            enable_min_prominence: true,
            enable_max_prominence: false,
            enable_min_difference: true,
            enable_max_difference: false,
            enable_min_plateau_size: false,
            enable_max_plateau_size: false,
            enable_min_distance: true,
            enable_max_distance: false,
        }
    }
}

impl PeakFindingSettings {
    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
        ui.heading("Peak Finder Settings");

        if ui.button("Reset").clicked() {
            *self = Self::default();
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            float_setting_row(
                ui,
                &mut self.enable_min_height,
                &mut self.min_height,
                "Min Height",
                "Minimum peak height after optional background subtraction. Raise this to ignore smaller peaks.",
            );

            float_setting_row(
                ui,
                &mut self.enable_max_height,
                &mut self.max_height,
                "Max Height",
                "Maximum peak height after optional background subtraction. Useful for excluding very tall or saturated peaks.",
            );

            float_setting_row(
                ui,
                &mut self.enable_min_prominence,
                &mut self.min_prominence,
                "Min Prominence",
                "Minimum prominence. Higher values keep only peaks that stand out clearly above nearby valleys.",
            );

            float_setting_row(
                ui,
                &mut self.enable_max_prominence,
                &mut self.max_prominence,
                "Max Prominence",
                "Maximum prominence. Useful if you want to ignore very dominant peaks.",
            );

            float_setting_row(
                ui,
                &mut self.enable_min_difference,
                &mut self.min_difference,
                "Min Difference",
                "Minimum absolute drop to the nearest neighboring bins on each side. Helps reject tiny wiggles and noise.",
            );

            float_setting_row(
                ui,
                &mut self.enable_max_difference,
                &mut self.max_difference,
                "Max Difference",
                "Maximum absolute drop to the nearest neighboring bins on each side.",
            );

            usize_setting_row(
                ui,
                &mut self.enable_min_plateau_size,
                &mut self.min_plateau_size,
                "Min Plateau Size",
                "Minimum number of bins allowed in a flat-topped peak.",
            );

            usize_setting_row(
                ui,
                &mut self.enable_max_plateau_size,
                &mut self.max_plateau_size,
                "Max Plateau Size",
                "Maximum number of bins allowed in a flat-topped peak.",
            );

            usize_setting_row(
                ui,
                &mut self.enable_min_distance,
                &mut self.min_distance,
                "Min Distance",
                "Minimum separation in bins between accepted peaks. If peaks are too close, the taller one wins.",
            );

            usize_setting_row(
                ui,
                &mut self.enable_max_distance,
                &mut self.max_distance,
                "Max Distance",
                "Maximum separation in bins between accepted peaks.",
            );
        });
    }

    pub fn find_peaks(&self, y_data: &[f64]) -> Vec<Peak<f64>> {
        let mut peak_finder = PeakFinder::new(y_data);

        if self.enable_min_height {
            peak_finder.with_min_height(self.min_height);
        }

        if self.enable_max_height {
            peak_finder.with_max_height(self.max_height);
        }

        if self.enable_min_prominence {
            peak_finder.with_min_prominence(self.min_prominence);
        }

        if self.enable_max_prominence {
            peak_finder.with_max_prominence(self.max_prominence);
        }

        if self.enable_min_difference {
            peak_finder.with_min_difference(self.min_difference);
        }

        if self.enable_max_difference {
            peak_finder.with_max_difference(self.max_difference);
        }

        if self.enable_min_plateau_size {
            peak_finder.with_min_plateau_size(self.min_plateau_size);
        }

        if self.enable_max_plateau_size {
            peak_finder.with_max_plateau_size(self.max_plateau_size);
        }

        if self.enable_min_distance {
            peak_finder.with_min_distance(self.min_distance);
        }

        if self.enable_max_distance {
            peak_finder.with_max_distance(self.max_distance);
        }

        peak_finder.find_peaks()
    }
}
