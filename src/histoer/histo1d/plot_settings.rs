use super::interactive_cuts::InteractiveCut1D;
use super::markers::FitMarkers;
use super::peak_finder::PeakFindingSettings;
use crate::egui_plot_stuff::egui_plot_settings::EguiPlotSettings;
use crate::fitter::common::Calibration;

use egui::PopupCloseBehavior;
use egui::containers::menu::{MenuConfig, SubMenuButton};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PlotSettings {
    #[serde(skip)]
    pub cursor_position: Option<egui_plot::PlotPoint>,
    #[serde(skip)]
    pub current_plot_bounds: Option<(f64, f64)>,
    pub egui_settings: EguiPlotSettings,
    pub column_name: String,
    pub cuts: Vec<InteractiveCut1D>,
    pub stats_info: bool,
    pub auto_fit_y_to_visible_range: bool,
    #[serde(alias = "auto_fit_y_max_multiplier")]
    pub auto_fit_y_max_multiplier_linear: f64,
    pub auto_fit_y_max_multiplier_log: f64,
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
            current_plot_bounds: None,
            egui_settings: EguiPlotSettings::default(),
            column_name: String::new(),
            cuts: vec![],
            stats_info: false,
            auto_fit_y_to_visible_range: true,
            auto_fit_y_max_multiplier_linear: 1.15,
            auto_fit_y_max_multiplier_log: 1.15,
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

        let mut cuts_dragging = false;
        for cut in &mut self.cuts {
            cut.interactive_dragging(response, calibration, self.current_plot_bounds);
            cuts_dragging |= cut.is_dragging();
        }

        self.egui_settings.allow_drag = !self.markers.is_dragging() && !cuts_dragging;
        self.egui_settings.allow_double_click_reset = !cuts_dragging;
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>, calibration: Option<&Calibration>) {
        for cut in &mut self.cuts {
            cut.draw(plot_ui, calibration);
        }
    }
}
