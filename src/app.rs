use super::pane::Pane;
use super::processer::Processer;
use super::tree::TreeBehavior;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct NATApp {
    tree: egui_tiles::Tree<Pane>,
    processer: Processer,
    behavior: TreeBehavior,
    side_panel_open: bool,
}

impl Default for NATApp {
    fn default() -> Self {
        let tree = egui_tiles::Tree::empty("Empty tree");

        Self {
            tree,
            processer: Processer::new(),
            behavior: Default::default(),
            side_panel_open: true,
        }
    }
}

impl NATApp {
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

impl eframe::App for NATApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("nat_top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.checkbox(&mut self.side_panel_open, "Side Panel");
            });
        });

        egui::SidePanel::left("nat_left_panel")
            // .max_width(200.0)
            .show_animated(ctx, self.side_panel_open, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("NAT");

                    egui::global_dark_light_mode_switch(ui);

                    if ui.button("Reset").clicked() {
                        *self = Default::default();
                    }
                });

                ui.separator();

                self.processer.ui(ui);

                // check to see if processer.is_ready -> get the tree from the histogrammer
                if self.processer.is_ready {
                    self.add_histograms_to_tree_from_processer();
                    self.processer.is_ready = false;
                }

                egui::ScrollArea::vertical()
                    .id_source("LeftPanel")
                    .show(ui, |ui| {
                        self.behavior.ui(ui);

                        // ui.collapsing("Tree", |ui| {
                        //     ui.style_mut().wrap = Some(false);
                        //     let tree_debug = format!("{:#?}", self.tree);
                        //     ui.monospace(&tree_debug);
                        // });

                        // ui.separator();

                        // ui.collapsing("Active tiles", |ui| {
                        //     let active = self.tree.active_tiles();
                        //     for tile_id in active {
                        //         use egui_tiles::Behavior as _;
                        //         let name = self.behavior.tab_title_for_tile(&self.tree.tiles, tile_id);
                        //         ui.label(format!("{} - {tile_id:?}", name.text()));
                        //     }
                        // });

                        if let Some(root) = self.tree.root() {
                            tree_ui(ui, &mut self.behavior, &mut self.tree.tiles, root);
                        }
                    });
            });

        // egui::SidePanel::right("nat_right_panel")
        // // .max_width(200.0)
        // .show_animated(ctx, self.side_panel_open, |ui| {
        //     self.processer.histogrammer_ui(ui);
        // });

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

    // check to see if the container only has one child
    // if let egui_tiles::Tile::Container(container) = &mut tile {
    //     if let Some(child) = container.only_child() {
    //         tree_ui(ui, behavior, tiles, child);

    //         return;
    //     }
    // }

    let default_open = false;
    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        egui::Id::new((tile_id, "tree")),
        default_open,
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
            //     // let mut kind = container.kind();
            //     // egui::ComboBox::from_label("Kind")
            //     //     .selected_text(format!("{kind:?}"))
            //     //     .show_ui(ui, |ui| {
            //     //         for typ in egui_tiles::ContainerKind::ALL {
            //     //             ui.selectable_value(&mut kind, typ, format!("{typ:?}"))
            //     //                 .clicked();
            //     //         }
            //     //     });
            //     // if kind != container.kind() {
            //     //     container.set_kind(kind);
            //     // }

            for &child in container.children() {
                tree_ui(ui, behavior, tiles, child);
            }
        }
    });

    // Put the tile back
    tiles.insert(tile_id, tile);
}
