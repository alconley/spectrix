use crate::pane::Pane;
// use egui_tiles::{Tile, TileId, Tiles};
use egui_tiles::Tile;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TreeBehavior {
    #[serde(skip)]
    simplification_options: egui_tiles::SimplificationOptions,
    tab_bar_height: f32,
    gap_width: f32,
    min_size: f32,
    preview_dragged_panes: bool,
    pub tile_map: std::collections::HashMap<egui_tiles::TileId, String>,
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
            min_size: 50.0,
            preview_dragged_panes: true,
            tile_map: std::collections::HashMap::new(),
        }
    }
}

impl TreeBehavior {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Behavior", |ui| {
            egui::Grid::new("behavior_ui")
                .num_columns(2)
                .show(ui, |ui| {
                    // ui.label("All panes must have tabs:");
                    // ui.checkbox(&mut self.simplification_options.all_panes_must_have_tabs, "");
                    // ui.end_row();

                    ui.label("Join nested containers:");
                    ui.checkbox(
                        &mut self.simplification_options.join_nested_linear_containers,
                        "",
                    );
                    ui.end_row();

                    ui.label("Tab bar height:");
                    ui.add(
                        egui::DragValue::new(&mut self.tab_bar_height)
                            .range(0.0..=100.0)
                            .speed(1.0),
                    );
                    ui.end_row();

                    ui.label("Gap width:");
                    ui.add(
                        egui::DragValue::new(&mut self.gap_width)
                            .range(0.0..=20.0)
                            .speed(1.0),
                    );
                    ui.end_row();

                    ui.label("Min size:");
                    ui.add(
                        egui::DragValue::new(&mut self.min_size)
                            .range(0.0..=f32::INFINITY)
                            .speed(1.0),
                    );
                    ui.end_row();

                    ui.label("Preview dragged panes:");
                    ui.checkbox(&mut self.preview_dragged_panes, "");
                    ui.end_row();
                });
        });

        ui.separator();
    }

    pub fn set_tile_tab_mapping(&mut self, tile_id: egui_tiles::TileId, tab_name: String) {
        self.tile_map.insert(tile_id, tab_name);
    }

    pub fn get_tab_name(&self, tile_id: &egui_tiles::TileId) -> Option<&String> {
        self.tile_map.get(tile_id)
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
        let mut options = self.simplification_options;

        if !options.all_panes_must_have_tabs {
            options.all_panes_must_have_tabs = true;
        }

        options
    }

    fn min_size(&self) -> f32 {
        self.min_size
    }

    fn preview_dragged_panes(&self) -> bool {
        self.preview_dragged_panes
    }

    /*
    fn is_tab_closable(&self, _tiles: &Tiles<Pane>, _tile_id: TileId) -> bool {
        true
    }

    fn on_tab_close(&mut self, tiles: &mut Tiles<Pane>, tile_id: TileId) -> bool {
        if let Some(tile) = tiles.get(tile_id) {
            match tile {
                Tile::Pane(pane) => {
                    // Single pane removal
                    let tab_title = self.tab_title_for_pane(pane);
                    log::debug!("Closing tab: {}, tile ID: {tile_id:?}", tab_title.text());
                }
                Tile::Container(container) => {
                    // Container removal
                    log::debug!("Closing container: {:?}", container.kind());
                    let children_ids = container.children();
                    for child_id in children_ids {
                        if let Some(Tile::Pane(pane)) = tiles.get(*child_id) {
                            let tab_title = self.tab_title_for_pane(pane);
                            log::debug!("Closing tab: {}, tile ID: {tile_id:?}", tab_title.text());
                        }
                    }
                }
            }
        }

        // Proceed to removing the tab
        true
    }
    */

    fn tab_title_for_tile(
        &mut self,
        tiles: &egui_tiles::Tiles<Pane>,
        tile_id: egui_tiles::TileId,
    ) -> egui::WidgetText {
        if let Some(tab_name) = self.get_tab_name(&tile_id) {
            tab_name.clone().into()
        } else {
            match tiles.get(tile_id) {
                Some(Tile::Pane(pane)) => self.tab_title_for_pane(pane),
                Some(Tile::Container(_container)) => "Container".into(),
                _ => "Unknown".into(),
            }
        }
    }
}
