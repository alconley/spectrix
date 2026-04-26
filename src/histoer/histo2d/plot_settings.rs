use crate::histoer::cuts::Cut2D;

use crate::egui_plot_stuff::egui_plot_settings::EguiPlotSettings;

use super::colormaps::{ColorMap, ColormapOptions};
use super::projections::{ProjectionAxisSettings, Projections};

use egui::PopupCloseBehavior;
use egui::containers::menu::{MenuConfig, SubMenuButton};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotSettings {
    #[serde(skip)]
    pub cursor_position: Option<egui_plot::PlotPoint>,
    pub egui_settings: EguiPlotSettings,
    pub x_column: String,
    pub y_column: String,
    #[serde(default)]
    pub source_pairs: Vec<(String, String)>,
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
            source_pairs: Vec::new(),
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
    pub fn cut_source_pairs(&self) -> Vec<(String, String)> {
        if !self.source_pairs.is_empty() {
            self.source_pairs.clone()
        } else if self.x_column.trim().is_empty() || self.y_column.trim().is_empty() {
            Vec::new()
        } else {
            vec![(
                self.x_column.trim().to_owned(),
                self.y_column.trim().to_owned(),
            )]
        }
    }

    pub fn cuts_available(&self) -> bool {
        !self.cut_source_pairs().is_empty()
    }

    pub fn cuts_unavailable_reason(&self) -> &'static str {
        "No X/Y source pairs are available for this histogram."
    }

    pub fn settings_ui(
        &mut self,
        ui: &mut egui::Ui,
        max_z_range: u64,
        x_axis_settings: ProjectionAxisSettings,
        y_axis_settings: ProjectionAxisSettings,
    ) {
        SubMenuButton::new("Colormaps")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                self.colormap_options
                    .ui(ui, &mut self.recalculate_image, max_z_range);
                ui.separator();
                self.colormap.color_maps_ui(ui, &mut self.recalculate_image);
            });

        SubMenuButton::new("Settings")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                ui.checkbox(&mut self.stats_info, "Show Statitics");
                ui.separator();
                self.egui_settings.menu_button(ui);
            });

        SubMenuButton::new("Projections")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                self.projections
                    .menu_button(ui, x_axis_settings, y_axis_settings);
            });
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        if self.cuts_available() {
            for cut in &mut self.cuts {
                cut.draw(plot_ui);
            }
        }
        self.projections.draw(plot_ui);
    }

    pub fn interactive_response(&mut self, plot_response: &egui_plot::PlotResponse<()>) {
        self.projections.interactive_dragging(plot_response);

        if self.cuts_available() {
            for cut in &mut self.cuts {
                self.egui_settings.allow_drag = !cut.is_dragging();
                self.egui_settings.allow_double_click_reset = !cut.is_clicking();
                cut.interactions(plot_response);
            }
        }
    }
}
