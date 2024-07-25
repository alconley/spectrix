use crate::cutter::cuts::HistogramCuts;
use crate::egui_plot_stuff::egui_plot_settings::EguiPlotSettings;

use super::colormaps::{ColorMap, ColormapOptions};
use super::projections::Projections;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotSettings {
    #[serde(skip)]
    pub cursor_position: Option<egui_plot::PlotPoint>,
    pub egui_settings: EguiPlotSettings,
    pub cuts: HistogramCuts,
    pub stats_info: bool,
    pub colormap: ColorMap,
    pub colormap_options: ColormapOptions,
    pub projections: Projections,
    #[serde(skip)]
    pub recalculate_image: bool,
}
impl Default for PlotSettings {
    fn default() -> Self {
        PlotSettings {
            cursor_position: None,
            egui_settings: EguiPlotSettings::default(),
            cuts: HistogramCuts::default(),
            stats_info: false,
            colormap: ColorMap::default(),
            colormap_options: ColormapOptions::default(),
            projections: Projections::new(),
            recalculate_image: false,
        }
    }
}
impl PlotSettings {
    pub fn settings_ui(&mut self, ui: &mut egui::Ui, max_z_range: u64) {
        ui.menu_button("Colormaps", |ui| {
            self.colormap_options
                .ui(ui, &mut self.recalculate_image, max_z_range);
            ui.separator();
            self.colormap.color_maps_ui(ui, &mut self.recalculate_image);
        });

        ui.separator();

        ui.checkbox(&mut self.stats_info, "Show Statitics");
        self.egui_settings.menu_button(ui);

        ui.separator();

        self.projections.menu_button(ui);

        ui.separator();

        self.cuts.menu_button(ui);

        // if any cuts are active temp disable double clicking to reset
        self.egui_settings.allow_double_click_reset = !self
            .cuts
            .cuts
            .iter()
            .any(|cut| cut.polygon.interactive_clicking);
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi) {
        self.cuts.draw(plot_ui);
        self.projections.draw(plot_ui);
    }

    pub fn interactive_response(&mut self, plot_response: &egui_plot::PlotResponse<()>) {
        self.projections.interactive_dragging(plot_response);
        self.cuts.interactive_response(plot_response);
    }

    // pub fn keybinds(&mut self, ui: &mut egui::Ui) {}
}
