use super::pane::Pane;
use super::tree::TreeBehavior;
use crate::util::processer::Processer;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct GNATApp {
    tree: egui_tiles::Tree<Pane>,
    processer: Processer,
    behavior: TreeBehavior,
    left_side_panel_open: bool,
    right_side_panel_open: bool,
}

impl Default for GNATApp {
    fn default() -> Self {
        let tree = egui_tiles::Tree::empty("Empty tree");

        Self {
            tree,
            processer: Processer::new(),
            behavior: Default::default(),
            left_side_panel_open: true,
            right_side_panel_open: true,
        }
    }
}

impl GNATApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn add_histograms_to_tree_from_processer(&mut self) {
        self.tree = self.processer.histogrammer.histogrammer_tree();
        self.behavior
            .tile_map
            .clone_from(&self.processer.histogrammer.tile_map);
    }
}

impl eframe::App for GNATApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("nat_top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.label("Show: ");
                ui.checkbox(&mut self.left_side_panel_open, "Info Panel");
                ui.checkbox(&mut self.right_side_panel_open, "Histogram Script");
            });
        });

        egui::SidePanel::left("nat_left_panel").show_animated(
            ctx,
            self.left_side_panel_open,
            |ui| {
                ui.horizontal(|ui| {
                    ui.heading("gNAT");

                    egui::global_dark_light_mode_switch(ui);

                    if ui.button("Reset").clicked() {
                        self.tree = egui_tiles::Tree::empty("Empty tree");
                        self.processer.reset();
                    }
                });

                egui::ScrollArea::vertical()
                    .id_source("LeftPanel")
                    .show(ui, |ui| {
                        ui.separator();

                        self.processer.ui(ui);

                        // check to see if processer.is_tree_ready -> get the tree from the histogrammer
                        if self.processer.is_tree_ready {
                            self.add_histograms_to_tree_from_processer();
                            self.processer.is_tree_ready = false;
                        }

                        self.behavior.ui(ui);

                        ui.separator();

                        if let Some(root) = self.tree.root() {
                            tree_ui(ui, &mut self.behavior, &mut self.tree.tiles, root);
                        }
                    });
            },
        );

        egui::SidePanel::right("nat_right_panel").show_animated(
            ctx,
            self.right_side_panel_open,
            |ui| {
                self.processer.histogram_script_ui(ui);
            },
        );

        egui::CentralPanel::default().show(ctx, |ui| {
            self.tree.ui(&mut self.behavior, ui);
        });
    }
}

fn tree_ui(
    ui: &mut egui::Ui,
    behavior: &mut dyn egui_tiles::Behavior<Pane>,
    tiles: &mut egui_tiles::Tiles<Pane>,
    tile_id: egui_tiles::TileId,
) {
    // Get the name BEFORE we remove the tile below!
    let text = format!(
        "{} - {tile_id:?}",
        behavior.tab_title_for_tile(tiles, tile_id).text()
    );

    // Temporarily remove the tile to circumvent the borrowchecker
    let Some(mut tile) = tiles.remove(tile_id) else {
        log::debug!("Missing tile {tile_id:?}");
        return;
    };

    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        egui::Id::new((tile_id, "tree")),
        false,
    )
    .show_header(ui, |ui| {
        ui.label(text);
        let mut visible = tiles.is_visible(tile_id);
        ui.checkbox(&mut visible, "Visible");
        tiles.set_visible(tile_id, visible);
    })
    .body(|ui| match &mut tile {
        egui_tiles::Tile::Pane(_) => {}
        egui_tiles::Tile::Container(container) => {
            for &child in container.children() {
                tree_ui(ui, behavior, tiles, child);
            }
        }
    });

    // Put the tile back
    tiles.insert(tile_id, tile);
}
