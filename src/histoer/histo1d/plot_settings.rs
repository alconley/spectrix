use super::markers::FitMarkers;
use super::peak_finder::PeakFindingSettings;
use crate::egui_plot_stuff::egui_plot_settings::EguiPlotSettings;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotSettings {
    #[serde(skip)]
    pub cursor_position: Option<egui_plot::PlotPoint>,
    pub egui_settings: EguiPlotSettings,
    pub stats_info: bool,
    pub markers: FitMarkers,
    pub rebin_factor: usize,
    pub find_peaks_settings: PeakFindingSettings,
}
impl Default for PlotSettings {
    fn default() -> Self {
        PlotSettings {
            cursor_position: None,
            egui_settings: EguiPlotSettings::default(),
            stats_info: false,
            markers: FitMarkers::new(),
            rebin_factor: 1,
            find_peaks_settings: PeakFindingSettings::default(),
        }
    }
}
impl PlotSettings {
    pub fn settings_ui(&mut self, ui: &mut egui::Ui) {
        self.egui_settings.menu_button(ui);
        ui.checkbox(&mut self.stats_info, "Show Statistics");
        self.markers.menu_button(ui);
    }

    pub fn interactive_response(&mut self, response: &egui_plot::PlotResponse<()>) {
        self.markers.interactive_dragging(response);
    }
}