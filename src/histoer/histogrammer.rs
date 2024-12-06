use super::configs::{Hist1DConfig, Hist2DConfig};
use super::cuts::Cut;
use super::histo1d::histogram1d::Histogram;
use super::histo2d::histogram2d::Histogram2D;
use super::pane::Pane;
use super::tree::TreeBehavior;
use pyo3::{prelude::*, types::PyModule};

use egui_tiles::TileId;
use fnv::FnvHashMap;

use polars::prelude::col;
use polars::prelude::*;

use rayon::prelude::*;

use std::convert::TryInto;

use std::sync::{Arc, Mutex};

use std::collections::HashMap;

use std::sync::atomic::{AtomicBool, Ordering};

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug)]
pub enum ContainerType {
    Grid,
    Tabs,
    Vertical,
    Horizontal,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct ContainerInfo {
    container_type: ContainerType,
    parent_id: Option<TileId>, // None if it's a top-level tab
    children: Vec<TileId>,     // Child tile IDs
    display_name: String,      // Display name for the tab
    tab_id: TileId,            // ID for this tab
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Histogrammer {
    pub name: String,
    pub tree: egui_tiles::Tree<Pane>,
    pub behavior: TreeBehavior,
    #[serde(skip)]
    pub calculating: Arc<AtomicBool>, // Use AtomicBool for thread-safe status tracking
    pub histogram_map: HashMap<String, ContainerInfo>, // Map full path to TabInfo
}

impl Default for Histogrammer {
    fn default() -> Self {
        Self {
            name: "Histogrammer".to_string(),
            tree: egui_tiles::Tree::empty("Empty tree"),
            behavior: Default::default(),
            calculating: Arc::new(AtomicBool::new(false)),
            histogram_map: HashMap::new(),
        }
    }
}

impl Histogrammer {
    pub fn add_hist1d(&mut self, name: &str, bins: usize, range: (f64, f64)) {
        log::debug!("Creating or updating 1D histogram '{}'", name);

        let mut pane_id_to_update = None;
        for (id, tile) in self.tree.tiles.iter_mut() {
            if let egui_tiles::Tile::Pane(Pane::Histogram(hist)) = tile {
                if hist.lock().unwrap().name == name {
                    log::debug!("Resetting existing 1D histogram '{}'", name);
                    hist.lock().unwrap().reset();
                    pane_id_to_update = Some(*id);
                    break;
                }
            }
        }

        if pane_id_to_update.is_none() {
            log::debug!(
                "No existing histogram found, creating new pane for '{}'",
                name
            );
            let hist = Histogram::new(name, bins, range);
            let pane = Pane::Histogram(Arc::new(Mutex::new(Box::new(hist))));
            let pane_id = self.tree.tiles.insert_pane(pane);

            // Pass the full name directly to create_tabs to parse into tabs and grids
            let grid_id = self.create_tabs(name.to_string());

            if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Grid(grid))) =
                self.tree.tiles.get_mut(grid_id)
            {
                log::debug!(
                    "Adding histogram '{}' to grid container ID {:?}",
                    name,
                    grid_id
                );
                grid.add_child(pane_id);

                self.histogram_map
                    .entry(name.to_string())
                    .and_modify(|tab_info| tab_info.children.push(pane_id));
            } else {
                log::error!("Failed to retrieve grid container for '{}'", name);
            }
        }
    }

    pub fn add_hist2d(
        &mut self,
        name: &str,
        bins: (usize, usize),
        range: ((f64, f64), (f64, f64)),
    ) {
        log::debug!("Creating or updating 2D histogram '{}'", name);

        let mut pane_id_to_update = None;
        for (id, tile) in self.tree.tiles.iter_mut() {
            if let egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) = tile {
                if hist.lock().unwrap().name == name {
                    log::debug!("Resetting existing 2D histogram '{}'", name);
                    hist.lock().unwrap().reset();
                    pane_id_to_update = Some(*id);
                    break;
                }
            }
        }

        if pane_id_to_update.is_none() {
            log::debug!(
                "No existing histogram found, creating new pane for '{}'",
                name
            );
            let hist = Histogram2D::new(name, bins, range);
            let pane = Pane::Histogram2D(Arc::new(Mutex::new(Box::new(hist))));
            let pane_id = self.tree.tiles.insert_pane(pane);

            // Pass the full name directly to create_tabs to parse into tabs and grids
            let grid_id = self.create_tabs(name.to_string());

            if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Grid(grid))) =
                self.tree.tiles.get_mut(grid_id)
            {
                log::debug!(
                    "Adding 2D histogram '{}' to grid container ID {:?}",
                    name,
                    grid_id
                );
                grid.add_child(pane_id);

                self.histogram_map
                    .entry(name.to_string())
                    .and_modify(|tab_info| tab_info.children.push(pane_id));
            } else {
                log::error!("Failed to retrieve grid container for '{}'", name);
            }
        }
    }

    pub fn fill_histograms(
        &mut self,
        hist1d_specs: Vec<Hist1DConfig>,
        hist2d_specs: Vec<Hist2DConfig>,
        lf: &LazyFrame,
        new_columns: Vec<(String, String)>,
        max_rows_per_batch: usize,
    ) {
        let mut lf = lf.clone();
        for (expression, alias) in new_columns {
            if let Err(e) = Self::add_computed_column(&mut lf, &expression, &alias) {
                log::error!("Error adding computed column '{}': {}", alias, e);
            }
        }
        // Wrap `lf` in an `Arc` to safely share it between threads
        let lf = Arc::new(lf.clone());
        let calculating = Arc::clone(&self.calculating);

        // Set calculating to true at the start
        calculating.store(true, Ordering::SeqCst);

        // Collect available column names
        let available_columns = self.get_column_names_from_lazyframe(&lf);

        // Filter and validate histograms based on existing columns
        let hist1d_specs: Vec<_> = hist1d_specs.into_iter().filter(|h| {
            if h.calculate {
                let column_exists = available_columns.contains(&h.column_name);
                if !column_exists {
                    log::error!("Warning: Column '{}' does not exist for 1D histogram '{}'. Skipping.", h.column_name, h.name);
                    return false;
                }
                for cut in &h.cuts {
                    match cut {
                        Cut::Cut1D(cut1d) => {
                            if let Some(conditions) = &cut1d.parsed_conditions {
                                for condition in conditions {
                                    if !available_columns.contains(&condition.column_name) {
                                        log::error!(
                                            "Warning: Cut column '{}' does not exist for 1D histogram '{}'. Skipping cut.",
                                            condition.column_name, h.name
                                        );
                                        return false;
                                    }
                                }
                            } else {
                                log::error!("Warning: Failed to parse conditions for 1D cut '{}'. Skipping cut.", h.name);
                                return false;
                            }
                        }
                        Cut::Cut2D(cut2d) => {
                            if !available_columns.contains(&cut2d.x_column) {
                                log::error!("Warning: Cut column '{}' does not exist for 1D histogram '{}'. Skipping cut.", cut2d.x_column, h.name);
                                return false;
                            }
                        }
                    }
                }
                true
            } else {
                false
            }
        }).collect();

        let hist2d_specs: Vec<_> = hist2d_specs.into_iter().filter(|h| {
            if h.calculate {
                let columns_exist = available_columns.contains(&h.x_column_name) && available_columns.contains(&h.y_column_name);
                if !columns_exist {
                    log::error!("Warning: Columns '{}' or '{}' do not exist for 2D histogram '{}'. Skipping.", h.x_column_name, h.y_column_name, h.name);
                    return false;
                }
                for cut in &h.cuts {
                    match cut {
                        Cut::Cut1D(cut1d) => {
                            if let Some(conditions) = &cut1d.parsed_conditions {
                                for condition in conditions {
                                    if !available_columns.contains(&condition.column_name) {
                                        log::error!(
                                            "Warning: Cut column '{}' does not exist for 1D histogram '{}'. Skipping cut.",
                                            condition.column_name, h.name
                                        );
                                        return false;
                                    }
                                }
                            } else {
                                log::error!("Warning: Failed to parse conditions for 1D cut '{}'. Skipping cut.", h.name);
                                return false;
                            }
                        }
                        Cut::Cut2D(cut) => {
                            if !available_columns.contains(&cut.x_column) || !available_columns.contains(&cut.y_column) {
                                log::error!("Warning: Cut columns '{}' or '{}' do not exist for 2D histogram '{}'. Skipping cut.", cut.x_column, cut.y_column, h.name);
                                return false;
                            }
                        }
                    }
                }
                true
            } else {
                false
            }
        }).collect();

        // Collect column names for the remaining histograms
        let mut column_names: Vec<&str> = hist1d_specs
            .iter()
            .map(|h| h.column_name.as_str())
            .collect();

        // Include columns required for cuts in both 1D and 2D histograms
        for h in &hist1d_specs {
            for cut in &h.cuts {
                match cut {
                    Cut::Cut1D(cut) => {
                        for condition in cut.parsed_conditions.as_ref().unwrap() {
                            column_names.push(condition.column_name.as_str());
                        }
                    }
                    Cut::Cut2D(cut) => {
                        column_names.push(cut.x_column.as_str());
                    }
                }
            }
        }

        for h in &hist2d_specs {
            column_names.push(h.x_column_name.as_str());
            column_names.push(h.y_column_name.as_str());

            for cut in &h.cuts {
                match cut {
                    Cut::Cut1D(_) => {}
                    Cut::Cut2D(cut) => {
                        column_names.push(cut.x_column.as_str());
                        column_names.push(cut.y_column.as_str());
                    }
                }
            }
        }

        column_names.sort_unstable();
        column_names.dedup();

        // Map column names to expressions for LazyFrame selection
        let selected_columns: Vec<_> = column_names.iter().map(|&col_name| col(col_name)).collect();

        // Prepare collections for histograms
        let mut hist1d_map = Vec::new();
        let mut hist2d_map = Vec::new();
        let mut missing_1d = Vec::new();
        let mut missing_2d = Vec::new();

        // Identify and reset existing histograms, collect missing ones
        for h in &hist1d_specs {
            if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram(hist)))) =
                self.tree.tiles.iter_mut().find(|(_id, tile)| {
                    if let egui_tiles::Tile::Pane(Pane::Histogram(hist)) = tile {
                        hist.lock().unwrap().name == h.name
                    } else {
                        false
                    }
                })
            {
                hist.lock().unwrap().reset(); // Reset histogram counts
                hist1d_map.push((Arc::clone(hist), h.clone()));
            } else {
                missing_1d.push(h.clone());
            }
        }

        for h in &hist2d_specs {
            if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram2D(hist)))) =
                self.tree.tiles.iter_mut().find(|(_id, tile)| {
                    if let egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) = tile {
                        hist.lock().unwrap().name == h.name
                    } else {
                        false
                    }
                })
            {
                hist.lock().unwrap().reset(); // Reset histogram counts
                hist2d_map.push((Arc::clone(hist), h.clone()));
            } else {
                missing_2d.push(h.clone());
            }
        }

        // Add missing 1D histograms outside of the mutable borrow loop
        for h in missing_1d {
            self.add_hist1d(&h.name, h.bins, h.range);
            if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram(hist)))) =
                self.tree.tiles.iter_mut().find(|(_id, tile)| {
                    if let egui_tiles::Tile::Pane(Pane::Histogram(hist)) = tile {
                        hist.lock().unwrap().name == h.name
                    } else {
                        false
                    }
                })
            {
                hist1d_map.push((Arc::clone(hist), h));
            }
        }

        // Add missing 2D histograms outside of the mutable borrow loop
        for h in missing_2d {
            self.add_hist2d(&h.name, h.bins, (h.x_range, h.y_range));
            if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram2D(hist)))) =
                self.tree.tiles.iter_mut().find(|(_id, tile)| {
                    if let egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) = tile {
                        hist.lock().unwrap().name == h.name
                    } else {
                        false
                    }
                })
            {
                hist2d_map.push((Arc::clone(hist), h));
            }
        }

        // Spawn the batch processing task asynchronously
        rayon::spawn({
            let calculating = Arc::clone(&calculating);
            let lf = Arc::clone(&lf); // Clone lf to move into the spawn closure
            move || {
                let mut row_start = 0;
                loop {
                    // Borrow the `LazyFrame` inside the Arc without moving it
                    let batch_lf = lf
                        .as_ref()
                        .clone()
                        .slice(row_start as i64, max_rows_per_batch.try_into().unwrap());
                    let lf_selected = batch_lf.select(selected_columns.clone());

                    // Break if no more rows are left to process
                    if lf_selected.clone().limit(1).collect().unwrap().height() == 0 {
                        break;
                    }

                    // No need for an inner handle; use `par_iter` for parallel execution within this batch
                    if let Ok(df) = lf_selected.collect() {
                        let height = df.height();

                        // Parallel filling of 1D histograms
                        hist1d_map.par_iter().for_each(|(hist, meta)| {
                            if let Ok(col_idx) = df.column(&meta.column_name) {
                                if let Ok(col_values) = col_idx.f64() {
                                    let mut hist = hist.lock().unwrap();

                                    // Loop over each row, matching the row index in the DataFrame with the cut columns
                                    for (index, value) in col_values.into_no_null_iter().enumerate()
                                    {
                                        if value == -1e6 {
                                            continue;
                                        }

                                        // Track if this point passes all cuts for this histogram
                                        let mut passes_all_cuts = true;

                                        // Evaluate each cut for the current point
                                        for cut in &meta.cuts {
                                            // Call point_is_inside to check if this point satisfies the cut
                                            if !cut.valid(&df, index) {
                                                passes_all_cuts = false;
                                                break;
                                            }
                                        }

                                        // Fill histogram only if all cuts pass for this point
                                        if passes_all_cuts {
                                            hist.fill(value);
                                        }

                                        hist.plot_settings.progress = Some(
                                            (row_start as f32 + index as f32)
                                                / (df.height() as f32),
                                        );
                                    }
                                }
                            }
                        });

                        hist2d_map.par_iter().for_each(|(hist, meta)| {
                            if let (Ok(x_values), Ok(y_values)) = (
                                df.column(&meta.x_column_name).and_then(|c| c.f64()),
                                df.column(&meta.y_column_name).and_then(|c| c.f64()),
                            ) {
                                let mut hist = hist.lock().unwrap();

                                // Loop over each row, matching the row index in the DataFrame with the cut columns
                                for (index, (x, y)) in x_values
                                    .into_no_null_iter()
                                    .zip(y_values.into_no_null_iter())
                                    .enumerate()
                                {
                                    if x == -1e6 || y == -1e6 {
                                        continue;
                                    }

                                    // Track if this point passes all cuts for this histogram
                                    let mut passes_all_cuts = true;

                                    // Evaluate each cut for the current point
                                    for cut in &meta.cuts {
                                        // Call point_is_inside to check if this point satisfies the cut
                                        if !cut.valid(&df, index) {
                                            passes_all_cuts = false;
                                            break;
                                        }
                                    }

                                    // Fill histogram only if all cuts pass for this point
                                    if passes_all_cuts {
                                        hist.fill(x, y);
                                    }
                                }
                            }
                        });

                        for (hist, meta) in &hist2d_map {
                            let mut hist = hist.lock().unwrap();
                            hist.plot_settings.x_column = meta.x_column_name.clone();
                            hist.plot_settings.y_column = meta.y_column_name.clone();
                            hist.plot_settings.recalculate_image = true;
                        }

                        for hist in &hist1d_map {
                            let mut hist = hist.0.lock().unwrap();
                            hist.plot_settings.progress = None;
                        }

                        println!("\tProcessed rows {} to {}", row_start, row_start + height);
                    }

                    row_start += max_rows_per_batch;
                }
                println!("Finished processing all rows\n");

                // Set calculating to false when processing is complete
                calculating.store(false, Ordering::SeqCst);
            }
        });
    }

    fn add_computed_column(
        lf: &mut LazyFrame,
        expression: &str,
        alias: &str,
    ) -> Result<(), PolarsError> {
        let computed_expr = expr_from_string(expression)?;
        *lf = lf.clone().with_column(computed_expr.alias(alias)); // Use alias for the new column name
        Ok(())
    }

    pub fn get_column_names_from_lazyframe(&self, lazyframe: &LazyFrame) -> Vec<String> {
        let lf: LazyFrame = lazyframe.clone().limit(1);
        let df: DataFrame = lf.collect().unwrap();
        let columns: Vec<String> = df
            .get_column_names_owned()
            .into_iter()
            .map(|name| name.to_string())
            .collect();

        columns
    }

    pub fn add_hist1d_with_bin_values(
        &mut self,
        name: &str,
        bins: Vec<u64>,
        underflow: u64,
        overflow: u64,
        range: (f64, f64),
    ) {
        self.add_hist1d(name, bins.len(), range);

        // set the bin values for the histogram
        if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram(hist)))) =
            self.tree.tiles.iter_mut().find(|(_id, tile)| {
                if let egui_tiles::Tile::Pane(Pane::Histogram(hist)) = tile {
                    hist.lock().unwrap().name == name
                } else {
                    false
                }
            })
        {
            hist.lock().unwrap().bins = bins.clone();
            hist.lock().unwrap().original_bins = bins;
            hist.lock().unwrap().underflow = underflow;
            hist.lock().unwrap().overflow = overflow;
        }
    }

    pub fn add_hist2d_with_bin_values(
        &mut self,
        name: &str,
        bins: Vec<Vec<u64>>,
        range: ((f64, f64), (f64, f64)),
    ) {
        // First, add the 2D histogram (with the bin size and range)
        self.add_hist2d(name, (bins.len(), bins[0].len()), range);

        let mut min_value = u64::MAX;
        let mut max_value = u64::MIN;

        // Convert the 2D bins to the FnvHashMap and update min/max counts dynamically
        let mut bin_map = FnvHashMap::default();
        for (i, row) in bins.iter().enumerate() {
            for (j, &bin) in row.iter().enumerate() {
                if bin != 0 {
                    bin_map.insert((i, j), bin);

                    // Update min and max values
                    if bin < min_value {
                        min_value = bin;
                    }
                    if bin > max_value {
                        max_value = bin;
                    }
                }
            }
        }

        // Handle the case when all bins are 0
        if min_value == u64::MAX {
            min_value = 0;
        }

        // Set the bin values for the histogram and update min/max count
        if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram2D(hist)))) =
            self.tree.tiles.iter_mut().find(|(_id, tile)| {
                if let egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) = tile {
                    hist.lock().unwrap().name == name
                } else {
                    false
                }
            })
        {
            let mut hist = hist.lock().unwrap();
            hist.bins.counts = bin_map;
            hist.bins.min_count = min_value;
            hist.bins.max_count = max_value;

            // Flag the image to be recalculated due to new bin values
            hist.plot_settings.recalculate_image = true;
        } else {
            log::error!("2D histogram '{}' not found in the tree", name);
        }
    }

    pub fn hist1d_data(&self) -> Vec<(String, String, Vec<u64>, u64, u64, (f64, f64))> {
        let mut data = Vec::new();
        for (_id, tile) in self.tree.tiles.iter() {
            if let egui_tiles::Tile::Pane(Pane::Histogram(hist)) = tile {
                let hist = hist.lock().unwrap();

                // strip the last part of the name for the title
                let name_parts: Vec<&str> = hist.name.split('/').collect();
                let title = name_parts.last().unwrap().to_string();

                data.push((
                    hist.name.clone(),
                    title,
                    hist.bins.clone(),
                    hist.underflow,
                    hist.overflow,
                    hist.range,
                ));
            }
        }
        data
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.tree.ui(&mut self.behavior, ui);
    }

    pub fn menu_ui(&mut self, ui: &mut egui::Ui) {
        // self.behavior.ui(ui);

        ui.menu_button("Histogrammer", |ui| {
            if let Some(root) = self.tree.root() {
                ui.horizontal(|ui| {
                    ui.heading("Tree");

                    ui.separator();

                    if ui.button("Reorganize").clicked() {
                        self.reorganize();
                    }
                });
                ui.separator();

                self.behavior.ui(ui);

                ui.separator();

                tree_ui(ui, &mut self.behavior, &mut self.tree.tiles, root);

                ui.separator();

                if ui.button("Create ROOT File").clicked() {
                    let _ = self.histograms_to_root();
                }
            }
        });
    }

    fn ensure_root(&mut self) -> TileId {
        // Ensure that `self.tree.root` has been initialized
        if let Some(root_id) = self.tree.root {
            root_id
        } else {
            // Initialize the root as the main tab container if it's not set
            let main_tab = egui_tiles::Container::new_tabs(vec![]);
            let main_container_id = self.tree.tiles.insert_new(main_tab.into());
            self.tree.root = Some(main_container_id);

            // Insert into histogram_map
            self.histogram_map.insert(
                self.name.clone(),
                ContainerInfo {
                    container_type: ContainerType::Tabs,
                    parent_id: None,
                    children: vec![],
                    display_name: self.name.clone(),
                    tab_id: main_container_id,
                },
            );

            main_container_id
        }
    }

    fn create_tabs(&mut self, name: String) -> TileId {
        // Ensure root container (main tab) exists
        let mut current_container_id = self.ensure_root();
        let path_components: Vec<&str> = name.split('/').collect();
        let mut accumulated_path = String::new();

        // Traverse each component in the name to build the tab structure
        for (i, component) in path_components.iter().enumerate() {
            if i > 0 {
                accumulated_path.push('/');
            }
            accumulated_path.push_str(component);

            if let Some(container_info) = self.histogram_map.get(&accumulated_path) {
                // Use the existing container's tab_id if found
                current_container_id = container_info.tab_id;
            } else {
                // Create a new tab container for intermediate components and a grid for the last component
                let new_id = if i < path_components.len() - 1 {
                    // Intermediate components get a new tab container
                    let tab_id = self.add_tab_container(current_container_id, component);
                    self.histogram_map.insert(
                        accumulated_path.clone(),
                        ContainerInfo {
                            container_type: ContainerType::Tabs,
                            parent_id: Some(current_container_id),
                            children: vec![],
                            display_name: component.to_string(),
                            tab_id,
                        },
                    );
                    tab_id
                } else {
                    // For the last component, create a unique grid for histograms
                    let grid_id = self.add_histograms_grid(current_container_id); // Ensure a unique grid ID
                    self.histogram_map.insert(
                        accumulated_path.clone(),
                        ContainerInfo {
                            container_type: ContainerType::Grid,
                            parent_id: Some(current_container_id),
                            children: vec![],
                            display_name: "Histograms".to_string(), // Set display name to "Histograms"
                            tab_id: grid_id,
                        },
                    );
                    grid_id
                };

                // Update the parent's children, whether it’s `Histogrammer` or an intermediate tab
                if let Some(parent_path) =
                    accumulated_path.rsplit_once('/').map(|(prefix, _)| prefix)
                {
                    // If a parent exists, add the new tab/grid as a child
                    if let Some(parent_info) = self.histogram_map.get_mut(parent_path) {
                        if !parent_info.children.contains(&new_id) {
                            parent_info.children.push(new_id);
                        }
                    }
                } else {
                    // If no parent path (i.e., root level), add to main tab
                    let main_tab = self.histogram_map.get_mut("Histogrammer").unwrap();
                    if !main_tab.children.contains(&new_id) {
                        main_tab.children.push(new_id);
                    }
                }

                // Update the current container ID
                current_container_id = new_id;
            }
        }

        current_container_id // Return the final tab or grid ID
    }

    fn add_tab_container(&mut self, parent_id: TileId, name: &str) -> TileId {
        // Create a new Tabs container
        let new_tab = egui_tiles::Container::new_tabs(vec![]);
        let new_tab_id = self.tree.tiles.insert_new(new_tab.into());
        self.behavior
            .set_tile_tab_mapping(new_tab_id, name.to_string());

        // Attach the new tab to its parent container
        if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(parent_tabs))) =
            self.tree.tiles.get_mut(parent_id)
        {
            parent_tabs.add_child(new_tab_id);
        } else {
            log::error!(
                "Parent container ID {:?} is not a Tabs container",
                parent_id
            );
        }

        new_tab_id
    }

    fn add_histograms_grid(&mut self, parent_tab_id: TileId) -> TileId {
        // Construct the key for lookup in histogram_map
        let histograms_key = format!("{:?}/Histograms", parent_tab_id);

        // Check if there's already a "Histograms" grid under the parent
        if let Some(container_info) = self.histogram_map.get(&histograms_key) {
            log::debug!(
                "Reusing existing Histograms grid with ID {:?}",
                container_info.tab_id
            );
            return container_info.tab_id; // Reuse the existing grid container
        }

        // Otherwise, create a new grid container
        let grid_container = egui_tiles::Container::new_grid(vec![]);
        let grid_id = self.tree.tiles.insert_new(grid_container.into());

        // Set the display name for the grid as "Histograms"
        self.behavior
            .set_tile_tab_mapping(grid_id, "Histograms".to_string());

        // Attach this grid to the specified parent tab container
        if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) =
            self.tree.tiles.get_mut(parent_tab_id)
        {
            tabs.add_child(grid_id);
        } else {
            log::error!(
                "Parent container ID {:?} is not a Tabs container",
                parent_tab_id
            );
        }

        // Add the new grid to histogram_map with the correct name and ID
        self.histogram_map.insert(
            histograms_key.clone(),
            ContainerInfo {
                container_type: ContainerType::Grid,
                parent_id: Some(parent_tab_id),
                children: vec![],
                display_name: "Histograms".to_string(),
                tab_id: grid_id,
            },
        );

        log::debug!("Created new Histograms grid with ID {:?}", grid_id);

        grid_id
    }

    fn reorganize(&mut self) {
        log::info!("histogram_map: {:#?}", self.histogram_map);

        // Step 1: Find the main tab using the root tile
        let main_tab_id = self.tree.root.expect("Main root tile (tab) not found");
        log::info!("Main tab found with ID: {:?}", main_tab_id);

        // Step 2: Locate the main tab in histogram_map and prepare to reorganize its children
        if let Some(main_tab_info) = self
            .histogram_map
            .values_mut()
            .find(|info| info.tab_id == main_tab_id)
        {
            if let Some(egui_tiles::Tile::Container(container)) =
                self.tree.tiles.get_mut(main_tab_id)
            {
                match main_tab_info.container_type {
                    ContainerType::Tabs => {
                        container.set_kind(egui_tiles::ContainerKind::Tabs);
                    }
                    ContainerType::Grid => {
                        container.set_kind(egui_tiles::ContainerKind::Grid);
                    }
                    ContainerType::Vertical => {
                        container.set_kind(egui_tiles::ContainerKind::Vertical);
                    }
                    ContainerType::Horizontal => {
                        container.set_kind(egui_tiles::ContainerKind::Horizontal);
                    }
                }
            }

            for (index, &child_id) in main_tab_info.children.iter().enumerate() {
                // Step 3: Check if the child is in the tree and change its container kind to `Tabs`
                if let Some(egui_tiles::Tile::Container(_container)) =
                    self.tree.tiles.get_mut(child_id)
                {
                    // let kind = container.kind();

                    // Change to `Tabs` if not already of this type
                    // if container.kind() != egui_tiles::ContainerKind::Tabs {
                    //     log::info!("Setting container type of {:?} to Tabs", child_id);
                    //     container.set_kind(egui_tiles::ContainerKind::Tabs);
                    // }

                    // Move the child to the main tab as a tabbed container
                    self.tree
                        .move_tile_to_container(child_id, main_tab_id, index, true);
                }
            }
        }

        // Step 4: Identify and move orphan histogram panes (those not found as keys in `histogram_map`)
        let mut orphans_to_move = Vec::new();
        for info in self.histogram_map.values() {
            for &child_id in &info.children {
                if !self.histogram_map.contains_key(&format!("{:?}", child_id)) {
                    orphans_to_move.push((child_id, info.tab_id));
                }
            }
        }

        for (orphan_id, destination_id) in orphans_to_move {
            log::info!(
                "Moving orphan ID {:?} to destination container ID {:?}",
                orphan_id,
                destination_id
            );
            self.tree
                .move_tile_to_container(orphan_id, destination_id, 0, true);
        }

        // Step 5: Identify and remove unreferenced (extraneous) containers
        let mut referenced_ids = std::collections::HashSet::new();
        for info in self.histogram_map.values() {
            referenced_ids.insert(info.tab_id);
            if let Some(parent_id) = info.parent_id {
                referenced_ids.insert(parent_id);
            }
            for &child_id in &info.children {
                referenced_ids.insert(child_id);
            }
        }

        let unreferenced_ids: Vec<TileId> = self
            .tree
            .tiles
            .iter()
            .filter(|(id, _)| !referenced_ids.contains(id))
            .map(|(id, _)| *id)
            .collect();

        for tile_id in unreferenced_ids {
            log::info!("Removing extraneous container with ID {:?}", tile_id);
            self.tree.remove_recursively(tile_id);
        }

        log::info!("Reorganization complete.");
    }

    pub fn retrieve_active_2d_cuts(&self) {
        let mut active_cuts = Vec::new();
        for (_id, tile) in self.tree.tiles.iter() {
            if let egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) = tile {
                let hist = hist.lock().unwrap();
                for cut in &hist.plot_settings.cuts {
                    active_cuts.push(cut.clone());
                }
            }
        }
    }

    pub fn histograms_to_root(&mut self) -> PyResult<()> {
        // python3 -m venv .venv
        // source .venv/bin/activate
        // export PYO3_PYTHON=$(pwd)/.venv/bin/python
        // export PYTHONPATH=$(pwd)/.venv/lib/python3.12/site-packages
        // cargo run --release

        Python::with_gil(|py| {
            let sys = py.import_bound("sys")?;
            let version: String = sys.getattr("version")?.extract()?;
            let executable: String = sys.getattr("executable")?.extract()?;
            println!("Using Python version: {}", version);
            println!("Python executable: {}", executable);

            // Check if the `uproot` module can be imported
            match py.import_bound("uproot") {
                Ok(_) => {
                    println!("Successfully imported `uproot` module.");
                }
                Err(_) => {
                    eprintln!("Error: `uproot` module could not be found. Make sure you are using the correct Python environment with `uproot` installed.");
                    return Err(PyErr::new::<pyo3::exceptions::PyImportError, _>(
                        "`uproot` module not available",
                    ));
                }
            }

            // Define the Python code as a module
            let code = r#"
import numpy as np
import uproot

def write_histograms(output_file, hist1d_data):
    """
    Writes 1D and 2D histograms to a ROOT file.

    Parameters:
        output_file (str): Path to the output ROOT file.
        hist1d_data (list): List of tuples for 1D histograms. Each tuple contains:
            - name (str): Histogram name.
            - title (str): Histogram title.
            - bins (list of int): Bin counts.
            - underflow (int): Underflow count.
            - overflow (int): Overflow count.
            - range (tuple): Range of the histogram as (min, max).
        hist2d_data (list): List of tuples for 2D histograms. Each tuple contains:
            - name (str): Histogram name.
            - title (str): Histogram title.
            - bins (list of list of int): Bin counts (2D array).
            - range_x (tuple): Range of the X-axis as (min, max).
            - range_y (tuple): Range of the Y-axis as (min, max).
    """
    with uproot.recreate(output_file) as file:
        for name, title, bins, underflow, overflow, range in hist1d_data:
            # Create bin edges for the histogram
            bin_edges = np.linspace(range[0], range[1], len(bins) + 1)
            
            # Include underflow and overflow in the data array
            data = np.array([underflow] + bins + [overflow], dtype=np.float32)
            bins_array = np.array(bins, dtype=np.float32)  # Convert bins to numpy array

            # Define fXaxis using to_TAxis with positional arguments
            fXaxis = uproot.writing.identify.to_TAxis(
                fName="xaxis",         # Temporary name for the X-axis
                fTitle="",       # Title of the X-axis
                fNbins=len(bins),      # Number of bins
                fXmin=range[0],       # Minimum X-axis value
                fXmax=range[1],       # Maximum X-axis value
                fXbins=bin_edges       # Bin edges
            )

            # Calculate metadata
            fEntries = float(np.sum(bins))
            fTsumw = float(np.sum(bins))
            fTsumw2 = float(np.sum(bins_array**2))
            fTsumwx = float(np.sum(bins_array * bin_edges[:-1]))
            fTsumwx2 = float(np.sum(bins_array * bin_edges[:-1]**2))
            fSumw2 = None

            # Write the histogram using uproot.writing.identify.to_TH1x
            file[name] = uproot.writing.identify.to_TH1x(
                fName=None,
                fTitle=title,
                data=data,
                fEntries=fEntries,
                fTsumw=fTsumw,
                fTsumw2=fTsumw2,
                fTsumwx=fTsumwx,
                fTsumwx2=fTsumwx2,
                fSumw2=fSumw2,
                fXaxis=fXaxis
            )

            print(f"1D Histogram '{name}' written successfully.")
            

            
    print(f"All histograms written to '{output_file}'.")
"#;

            // Compile the Python code into a module
            let module =
                PyModule::from_code_bound(py, code, "write_histograms.py", "write_histograms")?;

            let output_file = "output.root";

            let hist1d_data = self.hist1d_data();

            let _result = module
                .getattr("get_1d_histograms")?
                .call1((output_file, hist1d_data))?;

            Ok(())
        })
    }

    // # Write 2D histograms
    // for name, title, bins, range_x, range_y in hist2d_data:
    //     bins = np.array(bins, dtype=np.float32)
    //     # Flatten the 2D array with added underflow/overflow bins
    //     bins_with_overflow = np.zeros((bins.shape[0] + 2, bins.shape[1] + 2), dtype=np.float32)
    //     bins_with_overflow[1:-1, 1:-1] = bins
    //     data = bins_with_overflow.flatten()

    //     x_bin_edges = np.linspace(range_x[0], range_x[1], bins.shape[1] + 1)
    //     y_bin_edges = np.linspace(range_y[0], range_y[1], bins.shape[0] + 1)

    //     fXaxis = uproot.writing.identify.to_TAxis(
    //         fName="xaxis",
    //         fTitle="X-axis",
    //         fNbins=bins.shape[1],
    //         fXmin=range_x[0],
    //         fXmax=range_x[1],
    //         fXbins=x_bin_edges
    //     )

    //     fYaxis = uproot.writing.identify.to_TAxis(
    //         fName="yaxis",
    //         fTitle="Y-axis",
    //         fNbins=bins.shape[0],
    //         fXmin=range_y[0],
    //         fXmax=range_y[1],
    //         fXbins=y_bin_edges
    //     )

    //     # Compute required statistical sums
    //     x_centers = (x_bin_edges[:-1] + x_bin_edges[1:]) / 2
    //     y_centers = (y_bin_edges[:-1] + y_bin_edges[1:]) / 2

    //     fTsumw = np.sum(bins)
    //     fTsumw2 = np.sum(bins**2)
    //     fTsumwx = np.sum(bins * x_centers[np.newaxis, :])
    //     fTsumwx2 = np.sum(bins * (x_centers[np.newaxis, :]**2))
    //     fTsumwy = np.sum(bins * y_centers[:, np.newaxis])
    //     fTsumwy2 = np.sum(bins * (y_centers[:, np.newaxis]**2))
    //     fTsumwxy = np.sum(bins * x_centers[np.newaxis, :] * y_centers[:, np.newaxis])

    //     file[name] = uproot.writing.identify.to_TH2x(
    //         fName=None,
    //         fTitle=title,
    //         data=data,
    //         fEntries=fTsumw,
    //         fTsumw=fTsumw,
    //         fTsumw2=fTsumw2,
    //         fTsumwx=fTsumwx,
    //         fTsumwx2=fTsumwx2,
    //         fTsumwy=fTsumwy,
    //         fTsumwy2=fTsumwy2,
    //         fTsumwxy=fTsumwxy,
    //         fSumw2=None,
    //         fXaxis=fXaxis,
    //         fYaxis=fYaxis
    //     )

    //     print(f"2D Histogram '{name}' written successfully.")
}

use regex::Regex;

fn expr_from_string(expression: &str) -> Result<Expr, PolarsError> {
    let re = Regex::new(r"(-?\d+\.?\d*|\w+|\*\*|[+*/()-])").unwrap();
    let tokens: Vec<String> = re
        .find_iter(expression)
        .map(|m| m.as_str().to_string())
        .collect();

    let mut expr_stack: Vec<Expr> = Vec::new();
    let mut op_stack: Vec<String> = Vec::new();
    let mut is_first_token = true;

    log::debug!("Starting evaluation of expression: '{}'", expression);
    log::debug!("Tokens: {:?}", tokens);

    for token in tokens {
        match token.as_str() {
            "+" | "-" | "*" | "/" | "**" => {
                while let Some(op) = op_stack.last() {
                    // Pop operators with higher or equal precedence
                    if precedence(op) > precedence(&token)
                        || (precedence(op) == precedence(&token) && is_left_associative(&token))
                    {
                        apply_op(&mut expr_stack, op_stack.pop().unwrap().as_str());
                    } else {
                        break;
                    }
                }
                op_stack.push(token);
                is_first_token = false;
            }
            "(" => {
                op_stack.push(token);
                is_first_token = false;
            }
            ")" => {
                while let Some(op) = op_stack.pop() {
                    if op == "(" {
                        break;
                    }
                    apply_op(&mut expr_stack, &op);
                }
            }
            _ if token.parse::<f64>().is_ok() => {
                let number = token.parse::<f64>().unwrap();
                if number < 0.0 && !is_first_token {
                    op_stack.push("+".to_string());
                }
                expr_stack.push(lit(number));
                is_first_token = false;
            }
            _ => {
                expr_stack.push(col(&token));
                is_first_token = false;
            }
        }
    }

    while let Some(op) = op_stack.pop() {
        apply_op(&mut expr_stack, &op);
    }

    if expr_stack.len() == 1 {
        Ok(expr_stack.pop().unwrap())
    } else {
        log::error!("Error: Stack ended with more than one expression, invalid expression");
        Err(PolarsError::ComputeError("Invalid expression".into()))
    }
}

fn precedence(op: &str) -> i32 {
    match op {
        "+" | "-" => 1,
        "*" | "/" => 2,
        "**" => 3,
        _ => 0,
    }
}

fn is_left_associative(op: &str) -> bool {
    match op {
        "+" | "-" | "*" | "/" => true,
        "**" => false, // Exponentiation is right-associative
        _ => false,
    }
}

fn apply_op(expr_stack: &mut Vec<Expr>, operator: &str) {
    if expr_stack.len() < 2 {
        log::warn!("Error: Not enough operands for '{}'", operator);
        return;
    }

    let right = expr_stack.pop().unwrap();
    let left = expr_stack.pop().unwrap();

    let result = match operator {
        "+" => left + right,
        "-" => left - right,
        "*" => left * right,
        "/" => left / right,
        "**" => left.pow(right),
        _ => {
            log::error!("Unknown operator: '{}'", operator);
            return;
        }
    };

    expr_stack.push(result);
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
            // let mut kind = container.kind();
            // egui::ComboBox::from_label("Kind")
            //     .selected_text(format!("{kind:?}"))
            //     .show_ui(ui, |ui| {
            //         for typ in egui_tiles::ContainerKind::ALL {
            //             ui.selectable_value(&mut kind, typ, format!("{typ:?}"))
            //                 .clicked();
            //         }
            //     });
            // if kind != container.kind() {
            //     container.set_kind(kind);
            // }

            for &child in container.children() {
                tree_ui(ui, behavior, tiles, child);
            }
        }
    });

    // Put the tile back
    tiles.insert(tile_id, tile);
}
