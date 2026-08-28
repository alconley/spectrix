use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::egui_plot_stuff::egui_plot_settings::EguiPlotSettings;
use crate::egui_plot_stuff::line_style::SerializableLineStyle;
use crate::fitter::fit_settings::FitSettings;
use crate::histoer::histo2d::colormaps::{ColorMap, ColormapOptions};
use crate::util::processer::FileSortState;

pub const DEFAULTS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct SpectrixDefaults {
    pub histogram_1d: Histogram1DDefaults,
    pub histogram_2d: Histogram2DDefaults,
    pub histogrammer: TileLayoutDefaults,
    pub general: GeneralDefaults,
    pub ai: AiDefaults,
    pub new_sessions: NewSessionDefaults,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct GeneralDefaults {
    pub estimated_memory_gb: f64,
}

impl Default for GeneralDefaults {
    fn default() -> Self {
        Self {
            estimated_memory_gb: 0.1,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct AiDefaults {
    pub base_url: String,
    pub model: String,
}

impl Default for AiDefaults {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:11434".to_owned(),
            model: "qwen3-coder:480b-cloud".to_owned(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct NewSessionDefaults {
    pub files_panel_open: bool,
    pub histogram_script_open: bool,
    pub ai_open: bool,
    pub calculate_histograms_separately: bool,
    pub file_sort: FileSortState,
}

impl Default for NewSessionDefaults {
    fn default() -> Self {
        Self {
            files_panel_open: true,
            histogram_script_open: true,
            ai_open: false,
            calculate_histograms_separately: false,
            file_sort: FileSortState::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TileLayoutDefaults {
    pub tab_bar_height: f32,
    pub gap_width: f32,
    pub min_size: f32,
    pub preview_dragged_panes: bool,
    pub prune_empty_tabs: bool,
    pub prune_empty_containers: bool,
    pub prune_single_child_tabs: bool,
    pub prune_single_child_containers: bool,
    pub all_panes_must_have_tabs: bool,
    pub join_nested_linear_containers: bool,
    pub flatten_tabs_in_tabs: bool,
}

impl Default for TileLayoutDefaults {
    fn default() -> Self {
        Self {
            tab_bar_height: 24.0,
            gap_width: 2.0,
            min_size: 50.0,
            preview_dragged_panes: true,
            prune_empty_tabs: true,
            prune_empty_containers: true,
            prune_single_child_tabs: false,
            prune_single_child_containers: false,
            all_panes_must_have_tabs: false,
            join_nested_linear_containers: false,
            flatten_tabs_in_tabs: false,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LineDefaults {
    pub draw: bool,
    pub name_in_legend: bool,
    pub highlighted: bool,
    pub width: f32,
    pub color: egui::Color32,
    pub reference_fill: bool,
    pub fill: f32,
    pub fill_alpha: f32,
    pub style: SerializableLineStyle,
    pub style_length: f32,
}

impl Default for LineDefaults {
    fn default() -> Self {
        Self::from_color(egui::Color32::LIGHT_BLUE)
    }
}

impl LineDefaults {
    pub fn from_color(color: egui::Color32) -> Self {
        Self {
            draw: true,
            name_in_legend: false,
            highlighted: false,
            width: 1.0,
            color,
            reference_fill: false,
            fill: 0.0,
            fill_alpha: 0.3,
            style: SerializableLineStyle::Solid,
            style_length: 15.0,
        }
    }

    pub fn apply_to(&self, line: &mut EguiLine) {
        line.draw = self.draw;
        line.name_in_legend = self.name_in_legend;
        line.highlighted = self.highlighted;
        line.width = self.width;
        line.set_color(self.color);
        line.reference_fill = self.reference_fill;
        line.fill = self.fill;
        line.fill_alpha = self.fill_alpha;
        line.style = self.style;
        line.style_length = self.style_length;
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct MarkerStyleDefaults {
    pub color: egui::Color32,
    pub width: f32,
    pub style: SerializableLineStyle,
    pub style_length: f32,
    pub midpoint_radius: f32,
}

impl MarkerStyleDefaults {
    pub fn new(color: egui::Color32) -> Self {
        Self {
            color,
            width: 0.5,
            style: SerializableLineStyle::Solid,
            style_length: 15.0,
            midpoint_radius: 3.0,
        }
    }
}

impl Default for MarkerStyleDefaults {
    fn default() -> Self {
        Self::new(egui::Color32::BLUE)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct MarkerDefaults {
    pub region: MarkerStyleDefaults,
    pub peak: MarkerStyleDefaults,
    pub background: MarkerStyleDefaults,
    pub background_fill_alpha: f32,
}

impl Default for MarkerDefaults {
    fn default() -> Self {
        Self {
            region: MarkerStyleDefaults::new(egui::Color32::BLUE),
            peak: MarkerStyleDefaults::new(egui::Color32::from_rgb(225, 0, 255)),
            background: MarkerStyleDefaults::new(egui::Color32::from_rgb(0, 200, 0)),
            background_fill_alpha: 0.05,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct FitPaletteDefaults {
    pub temporary_background: LineDefaults,
    pub temporary_composition: LineDefaults,
    pub temporary_decomposition: LineDefaults,
    pub stored_background: LineDefaults,
    pub stored_composition: LineDefaults,
    pub stored_decomposition: LineDefaults,
}

impl Default for FitPaletteDefaults {
    fn default() -> Self {
        Self {
            temporary_background: LineDefaults::from_color(egui::Color32::GREEN),
            temporary_composition: LineDefaults::from_color(egui::Color32::BLUE),
            temporary_decomposition: LineDefaults::from_color(egui::Color32::from_rgb(150, 0, 255)),
            stored_background: LineDefaults::from_color(egui::Color32::DARK_GREEN),
            stored_composition: LineDefaults::from_color(egui::Color32::DARK_BLUE),
            stored_decomposition: LineDefaults::from_color(egui::Color32::from_rgb(150, 0, 255)),
        }
    }
}

pub const DEFAULT_CUT_COLORS: [egui::Color32; 6] = [
    egui::Color32::RED,
    egui::Color32::GREEN,
    egui::Color32::BLUE,
    egui::Color32::YELLOW,
    egui::Color32::from_rgb(255, 0, 255),
    egui::Color32::from_rgb(0, 255, 255),
];

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Cut1DDefaults {
    pub palette: [egui::Color32; 6],
    pub line_width: f32,
    pub line_style: SerializableLineStyle,
    pub style_length: f32,
    pub midpoint_radius: f32,
    pub fill_alpha: f32,
    pub initial_inset_fraction: f64,
    pub active: bool,
}

impl Default for Cut1DDefaults {
    fn default() -> Self {
        Self {
            palette: DEFAULT_CUT_COLORS,
            line_width: 2.0,
            line_style: SerializableLineStyle::Solid,
            style_length: 15.0,
            midpoint_radius: 5.0,
            fill_alpha: 0.12,
            initial_inset_fraction: 0.05,
            active: true,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Cut2DDefaults {
    pub palette: [egui::Color32; 6],
    pub draw: bool,
    pub name_in_legend: bool,
    pub line_width: f32,
    pub line_style: SerializableLineStyle,
    pub style_length: f32,
    pub fill_color: egui::Color32,
    pub interactive_dragging: bool,
    pub active: bool,
}

impl Default for Cut2DDefaults {
    fn default() -> Self {
        Self {
            palette: DEFAULT_CUT_COLORS,
            draw: true,
            name_in_legend: false,
            line_width: 2.0,
            line_style: SerializableLineStyle::Solid,
            style_length: 15.0,
            fill_color: egui::Color32::TRANSPARENT,
            interactive_dragging: true,
            active: true,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Histogram1DDefaults {
    pub light_theme_color: egui::Color32,
    pub dark_theme_color: egui::Color32,
    pub line: LineDefaults,
    pub plot: EguiPlotSettings,
    pub show_statistics: bool,
    pub auto_fit_y_to_visible_range: bool,
    pub auto_fit_y_multiplier_linear: f64,
    pub auto_fit_y_multiplier_log: f64,
    pub markers: MarkerDefaults,
    pub fit: FitSettings,
    pub fit_palette: FitPaletteDefaults,
    pub cuts: Cut1DDefaults,
}

impl Default for Histogram1DDefaults {
    fn default() -> Self {
        Self {
            light_theme_color: egui::Color32::BLACK,
            dark_theme_color: egui::Color32::LIGHT_BLUE,
            line: LineDefaults::default(),
            plot: EguiPlotSettings::default(),
            show_statistics: false,
            auto_fit_y_to_visible_range: true,
            auto_fit_y_multiplier_linear: 1.15,
            auto_fit_y_multiplier_log: 1.15,
            markers: MarkerDefaults::default(),
            fit: FitSettings::default(),
            fit_palette: FitPaletteDefaults::default(),
            cuts: Cut1DDefaults::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ImageDefaults {
    pub draw: bool,
    pub name_in_legend: bool,
    pub highlighted: bool,
    pub add_background: bool,
    pub background_color: egui::Color32,
    pub tint: egui::Color32,
    pub rotate_degrees: f64,
    pub magnification: egui::TextureFilter,
    pub minification: egui::TextureFilter,
}

impl Default for ImageDefaults {
    fn default() -> Self {
        Self {
            draw: true,
            name_in_legend: false,
            highlighted: false,
            add_background: false,
            background_color: egui::Color32::TRANSPARENT,
            tint: egui::Color32::WHITE,
            rotate_degrees: 0.0,
            magnification: egui::TextureFilter::Nearest,
            minification: egui::TextureFilter::Nearest,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ProjectionDefaults {
    pub add_x_projection: bool,
    pub add_y_projection: bool,
    pub x_color: egui::Color32,
    pub y_color: egui::Color32,
    pub line_width: f32,
    pub line_style: SerializableLineStyle,
    pub style_length: f32,
    pub midpoint_radius: f32,
    pub fill_alpha: f32,
    pub initial_inset_fraction: f64,
}

impl Default for ProjectionDefaults {
    fn default() -> Self {
        Self {
            add_x_projection: false,
            add_y_projection: false,
            x_color: egui::Color32::from_rgb(0, 0, 255),
            y_color: egui::Color32::from_rgb(255, 0, 0),
            line_width: 2.0,
            line_style: SerializableLineStyle::Solid,
            style_length: 15.0,
            midpoint_radius: 5.0,
            fill_alpha: 0.3,
            initial_inset_fraction: 0.05,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Histogram2DDefaults {
    pub plot: EguiPlotSettings,
    pub show_statistics: bool,
    pub colormap: ColorMap,
    pub colormap_options: ColormapOptions,
    pub image: ImageDefaults,
    pub projections: ProjectionDefaults,
    pub cuts: Cut2DDefaults,
}

pub fn apply_plot_defaults(defaults: &EguiPlotSettings, target: &mut EguiPlotSettings) {
    let x_label = std::mem::take(&mut target.x_label);
    let y_label = std::mem::take(&mut target.y_label);
    let reset_axis = target.reset_axis;
    *target = defaults.clone();
    target.x_label = x_label;
    target.y_label = y_label;
    target.reset_axis = reset_axis;
}

pub fn search_matches(query: &str, label: &str, keywords: &[&str]) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }

    let haystack = std::iter::once(label)
        .chain(keywords.iter().copied())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    query
        .split_whitespace()
        .all(|token| haystack.contains(token))
}

#[cfg(test)]
mod tests {
    use super::{SpectrixDefaults, search_matches};

    #[test]
    fn defaults_search_matches_labels_and_aliases() {
        assert!(search_matches(
            "marker",
            "Peak Markers",
            &["region", "peak"]
        ));
        assert!(search_matches("line", "Line Width", &[]));
        assert!(!search_matches("ollama", "Line Width", &["stroke"]));
    }

    #[test]
    fn defaults_round_trip_through_ron() {
        let mut defaults = SpectrixDefaults::default();
        defaults.histogram_1d.auto_fit_y_multiplier_linear = 1.75;
        let serialized = ron::to_string(&defaults).expect("serialize defaults");
        let restored: SpectrixDefaults = ron::from_str(&serialized).expect("deserialize defaults");
        assert_eq!(restored.histogram_1d.auto_fit_y_multiplier_linear, 1.75);
    }
}
