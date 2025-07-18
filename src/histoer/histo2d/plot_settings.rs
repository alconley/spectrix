use crate::histoer::cuts::Cut2D;

use crate::egui_plot_stuff::egui_plot_settings::EguiPlotSettings;

use super::colormaps::{ColorMap, ColormapOptions};
use super::projections::Projections;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotSettings {
    #[serde(skip)]
    pub cursor_position: Option<egui_plot::PlotPoint>,
    pub egui_settings: EguiPlotSettings,
    pub x_column: String,
    pub y_column: String,
    pub cuts: Vec<Cut2D>,
    pub stats_info: bool,
    pub colormap: ColorMap,
    pub colormap_options: ColormapOptions,
    pub projections: Projections,
    pub rebin_x_factor: usize,
    pub rebin_y_factor: usize,
    #[serde(skip)]
    pub recalculate_image: bool,
}
impl Default for PlotSettings {
    fn default() -> Self {
        Self {
            cursor_position: None,
            egui_settings: EguiPlotSettings::default(),
            x_column: String::new(),
            y_column: String::new(),
            cuts: vec![],
            stats_info: false,
            colormap: ColorMap::default(),
            colormap_options: ColormapOptions::default(),
            projections: Projections::new(),
            rebin_x_factor: 1,
            rebin_y_factor: 1,
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
        // self.egui_settings.menu_button(ui);

        ui.separator();

        self.projections.menu_button(ui);

        ui.separator();
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        for cut in &mut self.cuts {
            cut.draw(plot_ui);
        }
        self.projections.draw(plot_ui);
    }

    pub fn interactive_response(&mut self, plot_response: &egui_plot::PlotResponse<()>) {
        self.projections.interactive_dragging(plot_response);

        for cut in &mut self.cuts {
            self.egui_settings.allow_drag = !cut.is_dragging();
            self.egui_settings.allow_double_click_reset = !cut.is_clicking();
            cut.interactions(plot_response);
        }
    }
}
