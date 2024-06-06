use crate::pane::Pane;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TreeBehavior {
    #[serde(skip)]
    simplification_options: egui_tiles::SimplificationOptions,
    tab_bar_height: f32,
    gap_width: f32,
}

impl Default for TreeBehavior {
    fn default() -> Self {
        Self {
            simplification_options: egui_tiles::SimplificationOptions {
                prune_empty_tabs: true,
                prune_empty_containers: true,
                prune_single_child_tabs: true,
                prune_single_child_containers: true,
                all_panes_must_have_tabs: true,
                join_nested_linear_containers: true,
            },
            tab_bar_height: 24.0,
            gap_width: 2.0,
        }
    }
}

impl TreeBehavior {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            simplification_options,
            tab_bar_height,
            gap_width,
        } = self;

        egui::Grid::new("behavior_ui")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("All panes must have tabs:");
                ui.checkbox(&mut simplification_options.all_panes_must_have_tabs, "");
                ui.end_row();

                ui.label("Join nested containers:");
                ui.checkbox(
                    &mut simplification_options.join_nested_linear_containers,
                    "",
                );
                ui.end_row();

                ui.label("Tab bar height:");
                ui.add(
                    egui::DragValue::new(tab_bar_height)
                        .clamp_range(0.0..=100.0)
                        .speed(1.0),
                );
                ui.end_row();

                ui.label("Gap width:");
                ui.add(
                    egui::DragValue::new(gap_width)
                        .clamp_range(0.0..=20.0)
                        .speed(1.0),
                );
                ui.end_row();
            });
    }
}

impl egui_tiles::Behavior<Pane> for TreeBehavior {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        pane.ui(ui)
    }

    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        match pane {
            Pane::Workspace(_workspace) => "Workspace".into(),
            Pane::Histogram(hist) => hist.name.clone().into(),
            Pane::Histogram2D(hist) => hist.name.clone().into(),
        }
    }

    // ---
    // Settings:

    fn tab_bar_height(&self, _style: &egui::Style) -> f32 {
        self.tab_bar_height
    }

    fn gap_width(&self, _style: &egui::Style) -> f32 {
        self.gap_width
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        self.simplification_options
    }
}
