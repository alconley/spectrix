use super::pane::Pane;
use super::tree::TreeBehavior;

use super::processer::Processer;
use super::workspacer::Workspacer;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct NATApp {
    tree: egui_tiles::Tree<Pane>,

    workspacer: Workspacer,
    processer: Processer,

    #[serde(skip)] // This how you opt-out of serialization of a field
    behavior: TreeBehavior,

    side_panel_open: bool,
}

impl Default for NATApp {
    fn default() -> Self {
        let mut tiles = egui_tiles::Tiles::default();

        let workspacer = Workspacer::new();
        let processer = Processer::new();

        let mut tabs = vec![];
        tabs.push(tiles.insert_pane(Pane::Workspace(workspacer.clone())));

        let root = tiles.insert_tab_tile(tabs);

        let tree = egui_tiles::Tree::new("my_tree", root, tiles);

        Self {
            tree,
            workspacer,
            processer,
            behavior: Default::default(),
            side_panel_open: false,
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

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.tree.ui(&mut self.behavior, ui);
    }

    pub fn add_histograms_to_tree(&mut self) {
        // let mut panes = self.processer.histogrammer.get_histogram1d_panes();

        // panes.push(Pane::Workspace(self.workspacer.clone()));

        // let tree = egui_tiles::Tree::new_grid("histograms", panes);

        // self.tree = tree;

        self.tree = self.processer.histogrammer.histogrammer_tree();

        // let tabs: Vec<TileId> = vec![tiles.insert_pane(Pane { }), tiles.insert_pane(Pane { })];

        // self.tree.tiles.insert_container(container);
    }
}

impl eframe::App for NATApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("muc_top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.checkbox(&mut self.side_panel_open, "Side Panel");

                ui.separator();

                if !self.workspacer.selected_files.borrow().is_empty() {
                    // Properly clone the shared state for processing
                    self.processer.files = self.workspacer.selected_files.borrow().clone();
                    // self.processer.calculation_ui(ui);

                    if ui.button("Calculate Histograms").clicked() {
                        self.processer.calculate_histograms();
                        self.add_histograms_to_tree();
                    }
                }
            });
        });

        egui::SidePanel::left("tree")
            .max_width(300.0)
            .show_animated(ctx, self.side_panel_open, |ui| {
                egui::global_dark_light_mode_buttons(ui);

                ui.separator();

                if ui.button("Reset").clicked() {
                    *self = Default::default();
                }
                self.behavior.ui(ui);

                ui.separator();

                // ui.collapsing("Tree", |ui| {
                //     ui.style_mut().wrap = Some(false);
                //     let tree_debug = format!("{:#?}", self.tree);
                //     ui.monospace(&tree_debug);
                // });

                ui.separator();

                ui.collapsing("Active tiles", |ui| {
                    let active = self.tree.active_tiles();
                    for tile_id in active {
                        use egui_tiles::Behavior as _;
                        let name = self.behavior.tab_title_for_tile(&self.tree.tiles, tile_id);
                        ui.label(format!("{} - {tile_id:?}", name.text()));
                    }
                });

                ui.separator();

                if let Some(root) = self.tree.root() {
                    tree_ui(ui, &mut self.behavior, &mut self.tree.tiles, root);
                }
            });

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
            let mut kind = container.kind();
            egui::ComboBox::from_label("Kind")
                .selected_text(format!("{kind:?}"))
                .show_ui(ui, |ui| {
                    for typ in egui_tiles::ContainerKind::ALL {
                        ui.selectable_value(&mut kind, typ, format!("{typ:?}"))
                            .clicked();
                    }
                });
            if kind != container.kind() {
                container.set_kind(kind);
            }

            for &child in container.children() {
                tree_ui(ui, behavior, tiles, child);
            }
        }
    });

    // Put the tile back
    tiles.insert(tile_id, tile);
}
