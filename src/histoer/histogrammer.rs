use super::histo1d::histogram1d::Histogram;
use super::histo2d::histogram2d::Histogram2D;
use super::pane::Pane;
use super::tree::TreeBehavior;
use polars::prelude::*;

use egui_tiles::TileId;

use indicatif::ProgressBar;
use indicatif::ProgressStyle;

pub enum ContainerType {
    Grid,
    Tabs,
    Vertical,
    Horizontal,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Histogrammer {
    pub tree: egui_tiles::Tree<Pane>,
    pub behavior: TreeBehavior,
}

impl Default for Histogrammer {
    fn default() -> Self {
        Self {
            tree: egui_tiles::Tree::empty("Empty tree"),
            behavior: Default::default(),
        }
    }
}

impl Histogrammer {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.tree.ui(&mut self.behavior, ui);
    }

    pub fn side_panel_ui(&mut self, ui: &mut egui::Ui) {
        self.behavior.ui(ui);

        ui.separator();

        if let Some(root) = self.tree.root() {
            tree_ui(ui, &mut self.behavior, &mut self.tree.tiles, root);
        }
    }

    pub fn create_grid(&mut self, tab_name: String) -> egui_tiles::TileId {
        // Create a new grid container
        let grid = egui_tiles::Grid::new(vec![]);
        let grid_container = egui_tiles::Container::Grid(grid);
        let grid_id = self.tree.tiles.insert_new(grid_container.into());

        // Create a new tab and place the grid inside it
        let tab = egui_tiles::Tabs::new(vec![grid_id]);
        let tab_id =
            self.tree
                .tiles
                .insert_new(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(
                    tab,
                )));

        // Set the tab name in the behavior's tile_map
        self.behavior.set_tile_tab_mapping(grid_id, tab_name);

        // If the tree is empty, set this new tab as the root
        if self.tree.is_empty() {
            self.tree.root = Some(tab_id);
        } else if let Some(root_id) = self.tree.root {
            // Access the container at the root
            if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) =
                self.tree.tiles.get_mut(root_id)
            {
                // Add the new tab to the existing tab container
                tabs.add_child(tab_id);
            } else {
                // If the root is not a tabs container, create a new tabs container
                let new_tabs = egui_tiles::Tabs::new(vec![root_id, tab_id]);
                let new_root_id = self.tree.tiles.insert_new(egui_tiles::Tile::Container(
                    egui_tiles::Container::Tabs(new_tabs),
                ));
                self.tree.root = Some(new_root_id);
            }
        }

        grid_id
    }

    pub fn add_hist1d(&mut self, name: &str, bins: usize, range: (f64, f64), grid: Option<TileId>) {
        let mut pane_id_to_update = None;

        for (id, tile) in self.tree.tiles.iter_mut() {
            if let egui_tiles::Tile::Pane(Pane::Histogram(hist)) = tile {
                if hist.name == name {
                    hist.reset();
                    pane_id_to_update = Some(*id);
                    break;
                }
            }
        }

        if pane_id_to_update.is_none() {
            let hist = Histogram::new(name, bins, range);
            let pane = Pane::Histogram(Box::new(hist));
            let pane_id = self.tree.tiles.insert_pane(pane);

            if let Some(grid_id) = grid {
                if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Grid(grid))) =
                    self.tree.tiles.get_mut(grid_id)
                {
                    grid.add_child(pane_id);
                } else {
                    log::error!("Invalid grid ID provided");
                }
            } else if self.tree.is_empty() {
                self.tree.root = Some(pane_id);
            } else if let Some(root_id) = self.tree.root {
                let new_grid = egui_tiles::Grid::new(vec![root_id, pane_id]);
                let new_root_id = self.tree.tiles.insert_new(egui_tiles::Tile::Container(
                    egui_tiles::Container::Grid(new_grid),
                ));
                self.tree.root = Some(new_root_id);
            }
        }
    }

    pub fn fill_hist1d(&mut self, name: &str, lf: &LazyFrame, column_name: &str) -> bool {
        // Find the histogram pane by name and fill it
        if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram(hist)))) = self
            .tree
            .tiles
            .iter_mut()
            .find(|(_id, tile)| matches!(tile, egui_tiles::Tile::Pane(Pane::Histogram(_))))
        {
            let hist_range = hist.range;
            let filter_expr = col(column_name)
                .gt(lit(hist_range.0))
                .and(col(column_name).lt(lit(hist_range.1)));

            match lf
                .clone()
                .select([col(column_name)])
                .filter(filter_expr)
                .collect()
            {
                Ok(df) => {
                    let series = df.column(column_name).unwrap();
                    let values = series.f64().unwrap();
                    let pb = ProgressBar::new(values.len() as u64);
                    pb.set_style(
                        ProgressStyle::default_bar()
                            .template(&format!(
                                "{} [{{bar:40.cyan/blue}}] {{pos}}/{{len}} ({{eta}})",
                                name
                            ))
                            .expect("Failed to create progress style")
                            .progress_chars("#>-"),
                    );

                    for value in values {
                        if let Some(v) = value {
                            hist.fill(v);
                        }
                        pb.inc(1);
                    }

                    pb.finish();
                    return true;
                }
                Err(e) => {
                    log::error!("Failed to collect LazyFrame: {}", e);
                    return false;
                }
            }
        }

        false
    }

    pub fn add_fill_hist1d(
        &mut self,
        name: &str,
        lf: &LazyFrame,
        column_name: &str,
        bins: usize,
        range: (f64, f64),
        grid: Option<TileId>,
    ) {
        self.add_hist1d(name, bins, range, grid); // Add the histogram.
        self.fill_hist1d(name, lf, column_name); // Fill it with data.
    }

    pub fn add_hist2d(
        &mut self,
        name: &str,
        bins: (usize, usize),
        range: ((f64, f64), (f64, f64)),
        grid: Option<TileId>,
    ) {
        let mut pane_id_to_update = None;

        // Check if the histogram already exists in the tree
        for (id, tile) in self.tree.tiles.iter_mut() {
            if let egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) = tile {
                if hist.name == name {
                    hist.reset();
                    pane_id_to_update = Some(*id);
                    break;
                }
            }
        }

        // If the histogram doesn't exist, create a new one
        if pane_id_to_update.is_none() {
            let hist = Histogram2D::new(name, bins, range);
            let pane = Pane::Histogram2D(Box::new(hist));
            let pane_id = self.tree.tiles.insert_pane(pane);

            // Add the pane to the specified grid or create a new grid if no grid_id is provided
            if let Some(grid_id) = grid {
                if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Grid(grid))) =
                    self.tree.tiles.get_mut(grid_id)
                {
                    grid.add_child(pane_id);
                } else {
                    log::error!("Invalid grid ID provided");
                }
            } else {
                // If no grid is specified, create a new grid or set as root
                if self.tree.is_empty() {
                    self.tree.root = Some(pane_id);
                } else if let Some(root_id) = self.tree.root {
                    let new_grid = egui_tiles::Grid::new(vec![root_id, pane_id]);
                    let new_root_id = self.tree.tiles.insert_new(egui_tiles::Tile::Container(
                        egui_tiles::Container::Grid(new_grid),
                    ));
                    self.tree.root = Some(new_root_id);
                }
            }
        }
    }

    pub fn fill_hist2d(
        &mut self,
        name: &str,
        lf: &LazyFrame,
        x_column_name: &str,
        y_column_name: &str,
    ) -> bool {
        // Find the 2D histogram pane by name and fill it
        if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram2D(hist)))) = self
            .tree
            .tiles
            .iter_mut()
            .find(|(_id, tile)| matches!(tile, egui_tiles::Tile::Pane(Pane::Histogram2D(_))))
        {
            let hist_range = hist.range.clone();
            let filter_expr = col(x_column_name)
                .gt(lit(hist_range.x.min))
                .and(col(x_column_name).lt(lit(hist_range.x.max)))
                .and(col(y_column_name).gt(lit(hist_range.y.min)))
                .and(col(y_column_name).lt(lit(hist_range.y.max)));
            match lf
                .clone()
                .select([col(x_column_name), col(y_column_name)])
                .filter(filter_expr)
                .collect()
            {
                Ok(df) => {
                    let x_values = df.column(x_column_name).unwrap().f64().unwrap();
                    let y_values = df.column(y_column_name).unwrap().f64().unwrap();
                    let pb = ProgressBar::new(x_values.len() as u64);
                    pb.set_style(
                        ProgressStyle::default_bar()
                            .template(&format!(
                                "{} [{{bar:40.cyan/blue}}] {{pos}}/{{len}} ({{eta}})",
                                name
                            ))
                            .expect("Failed to create progress style")
                            .progress_chars("#>-"),
                    );
                    for (x_value, y_value) in x_values.into_iter().zip(y_values) {
                        if let (Some(x), Some(y)) = (x_value, y_value) {
                            hist.fill(x, y);
                        }
                        pb.inc(1);
                    }
                    pb.finish();
                    return true;
                }
                Err(e) => {
                    log::error!("Failed to collect LazyFrame: {}", e);
                    return false;
                }
            }
        }

        false
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_fill_hist2d(
        &mut self,
        name: &str,
        lf: &LazyFrame,
        x_column_name: &str,
        y_column_name: &str,
        bins: (usize, usize),
        range: ((f64, f64), (f64, f64)),
        grid: Option<TileId>,
    ) {
        self.add_hist2d(name, bins, range, grid); // Add the histogram.
        self.fill_hist2d(name, lf, x_column_name, y_column_name); // Fill it with data.
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
