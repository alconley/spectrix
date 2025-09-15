use super::markers::FitMarkers;
use super::peak_finder::PeakFindingSettings;
use crate::egui_plot_stuff::egui_plot_settings::EguiPlotSettings;
use crate::fitter::common::Calibration;

use egui::PopupCloseBehavior;
use egui::containers::menu::{MenuConfig, SubMenuButton};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotSettings {
    #[serde(skip)]
    pub cursor_position: Option<egui_plot::PlotPoint>,
    pub egui_settings: EguiPlotSettings,
    pub stats_info: bool,
    pub markers: FitMarkers,
    pub rebin_factor: usize,
    pub find_peaks_settings: PeakFindingSettings,

    #[serde(skip)] // Skip serialization for progress
    pub progress: Option<f32>, // Optional progress tracking
}
impl Default for PlotSettings {
    fn default() -> Self {
        Self {
            cursor_position: None,
            egui_settings: EguiPlotSettings::default(),
            stats_info: false,
            markers: FitMarkers::new(),
            rebin_factor: 1,
            find_peaks_settings: PeakFindingSettings::default(),
            progress: None,
        }
    }
}
impl PlotSettings {
    pub fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.stats_info, "Show Statistics");

        SubMenuButton::new("Markers")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                self.markers.menu_button(ui);
            });

        SubMenuButton::new("Visual Settings")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                self.egui_settings.menu_button(ui);
            });
    }

    pub fn interactive_response(
        &mut self,
        response: &egui_plot::PlotResponse<()>,
        calibration: Option<&Calibration>,
    ) {
        self.markers.interactive_dragging(response, calibration);
    }
}
