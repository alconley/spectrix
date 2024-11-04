use super::histo1d::histogram1d::Histogram;
use super::histo2d::histogram2d::Histogram2D;
use super::pane::Pane;
use super::tree::TreeBehavior;
use crate::cutter::cut_handler::CutHandler;
use egui_tiles::TileId;
use fnv::FnvHashMap;
use polars::prelude::*;
use std::thread::JoinHandle;

use std::sync::{Arc, Mutex};

use std::collections::{HashMap, HashSet};

#[derive(serde::Deserialize, serde::Serialize, PartialEq)]
pub enum ContainerType {
    Grid,
    Tabs,
    Vertical,
    Horizontal,
}

#[derive(serde::Deserialize, serde::Serialize)]
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
    pub handles: Vec<JoinHandle<()>>, // Multiple thread handles
    pub grid_histogram_map: HashMap<String, ContainerInfo>, // Map full path to TabInfo
    #[serde(skip)]
    pub selected_histograms: HashSet<String>, // Track selected histograms with unique identifiers
    pub grid_container: Option<Vec<TileId>>,                // Vector of grid container IDs
}

impl Default for Histogrammer {
    fn default() -> Self {
        Self {
            name: "Histogrammer".to_string(),
            tree: egui_tiles::Tree::empty("Empty tree"),
            behavior: Default::default(),
            handles: vec![],
            grid_histogram_map: HashMap::new(),
            selected_histograms: HashSet::new(),
            grid_container: None,
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

                self.grid_histogram_map
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

                self.grid_histogram_map
                    .entry(name.to_string())
                    .and_modify(|tab_info| tab_info.children.push(pane_id));
            } else {
                log::error!("Failed to retrieve grid container for '{}'", name);
            }
        }
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

            // Insert into grid_histogram_map
            self.grid_histogram_map.insert(
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

    pub fn fill_hist1d(&mut self, name: &str, lf: &LazyFrame, column_name: &str) -> bool {
        if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram(hist)))) =
            self.tree.tiles.iter_mut().find(|(_id, tile)| {
                if let egui_tiles::Tile::Pane(Pane::Histogram(hist)) = tile {
                    hist.lock().unwrap().name == name
                } else {
                    false
                }
            })
        {
            let hist = Arc::clone(hist); // Clone the Arc to share ownership
            let hist_range = hist.lock().unwrap().range; // Access the range safely
            let filter_expr = col(column_name)
                .gt(lit(hist_range.0))
                .and(col(column_name).lt(lit(hist_range.1)));

            // let overflow_filter_expr = col(column_name).gt(lit(hist_range.1));
            // // get the overflow values
            // let overflow_df = lf
            //     .clone()
            //     .select([col(column_name)])
            //     .filter(overflow_filter_expr)
            //     .sum()
            //     .collect()
            //     .unwrap();

            // let overflow_value = overflow_df.column(column_name).unwrap().get(0).unwrap(); // Now you can access the first value safely

            // let overflow_as_u64 = match overflow_value {
            //     AnyValue::Int64(val) => val as u64,   // Cast if it's an Int64
            //     AnyValue::Float64(val) => val as u64, // Cast if it's a Float64
            //     _ => panic!("Unexpected value type!"),
            // };

            // let underflow_filter_expr = col(column_name).lt(lit(hist_range.0));
            // // get the underflow values
            // let underflow_df = lf
            //     .clone()
            //     .select([col(column_name)])
            //     .filter(underflow_filter_expr)
            //     .sum()
            //     .collect()
            //     .unwrap();

            // let underflow_value = underflow_df.column(column_name).unwrap().get(0).unwrap(); // Now you can access the first value safely

            // let underflow_as_u64 = match underflow_value {
            //     AnyValue::Int64(val) => val as u64,   // Cast if it's an Int64
            //     AnyValue::Float64(val) => val as u64, // Cast if it's a Float64
            //     _ => panic!("Unexpected value type!"),
            // };

            // hist.lock().unwrap().overflow = overflow_as_u64;
            // hist.lock().unwrap().underflow = underflow_as_u64;

            let lf = lf.clone();
            let name = name.to_string();
            let column_name = column_name.to_string();

            log::info!(
                "Starting to fill histogram '{}' with data from column '{}'",
                name,
                column_name
            );

            // Spawn a new thread for the filling operation
            let handle = std::thread::spawn(move || {
                log::info!("Thread started for filling histogram '{}'", name);

                if let Ok(df) = lf
                    .select([col(&column_name)])
                    .filter(filter_expr.clone()) // Clone for logging purposes
                    .collect()
                {
                    log::info!("Data collected for histogram '{}'", name);

                    let series = df.column(&column_name).unwrap();
                    let values = series.f64().unwrap();
                    let total_steps = values.len();

                    log::info!(
                        "Histogram '{}' will be filled with {} values from column '{}'",
                        name,
                        total_steps,
                        column_name
                    );

                    for (i, value) in values.iter().enumerate() {
                        if let Some(v) = value {
                            let mut hist = hist.lock().unwrap(); // Lock the mutex to access the correct Histogram
                            hist.fill(v, i, total_steps); // Pass the progress to the fill method
                        }
                    }

                    log::info!("Completed filling histogram '{}'", name);

                    // Optionally: Set progress to None or trigger any final updates here
                    hist.lock().unwrap().plot_settings.progress = None;
                } else {
                    log::error!("Failed to collect LazyFrame for histogram '{}'", name);
                }
            });

            // Store the thread handle in the vector
            self.handles.push(handle);

            return true;
        }

        log::error!("Histogram '{}' not found in the tree", name);
        false
    }

    pub fn add_fill_hist1d(
        &mut self,
        name: &str,
        lf: &LazyFrame,
        column_name: &str,
        bins: usize,
        range: (f64, f64),
    ) {
        self.add_hist1d(name, bins, range); // Add the histogram.
        self.fill_hist1d(name, lf, column_name); // Fill it with data.
    }

    pub fn fill_hist2d(
        &mut self,
        name: &str,
        lf: &LazyFrame,
        x_column_name: &str,
        y_column_name: &str,
    ) -> bool {
        if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram2D(hist)))) =
            self.tree.tiles.iter_mut().find(|(_id, tile)| {
                if let egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) = tile {
                    hist.lock().unwrap().name == name
                } else {
                    false
                }
            })
        {
            let hist = Arc::clone(hist); // Clone the Arc to share ownership
            let hist_range = hist.lock().unwrap().range.clone(); // Access the range safely
            let filter_expr = col(x_column_name)
                .gt(lit(hist_range.x.min))
                .and(col(x_column_name).lt(lit(hist_range.x.max)))
                .and(col(y_column_name).gt(lit(hist_range.y.min)))
                .and(col(y_column_name).lt(lit(hist_range.y.max)));

            // let overflow_expr = col(x_column_name)
            //     .gt(lit(hist_range.x.max))
            //     .or(col(y_column_name).gt(lit(hist_range.y.max)));

            // let underflow_expr = col(x_column_name)
            //     .lt(lit(hist_range.x.min))
            //     .or(col(y_column_name).lt(lit(hist_range.y.min)));

            // let overflow_df = lf
            //     .clone()
            //     .select([col(x_column_name), col(y_column_name)])
            //     .filter(overflow_expr)
            //     .sum()
            //     .collect()
            //     .unwrap();

            // let underflow_df = lf
            //     .clone()
            //     .select([col(x_column_name), col(y_column_name)])
            //     .filter(underflow_expr)
            //     .sum()
            //     .collect()
            //     .unwrap();

            // let overflow_x_value = overflow_df.column(x_column_name).unwrap().get(0).unwrap();

            // let overflow_y_value = overflow_df.column(y_column_name).unwrap().get(0).unwrap();

            // let underflow_x_value = underflow_df.column(x_column_name).unwrap().get(0).unwrap();

            // let underflow_y_value = underflow_df.column(y_column_name).unwrap().get(0).unwrap();

            // let overflow_x_as_u64 = match overflow_x_value {
            //     AnyValue::Int64(val) => val as u64,   // Cast if it's an Int64
            //     AnyValue::Float64(val) => val as u64, // Cast if it's a Float64
            //     _ => panic!("Unexpected value type!"),
            // };

            // let overflow_y_as_u64 = match overflow_y_value {
            //     AnyValue::Int64(val) => val as u64,   // Cast if it's an Int64
            //     AnyValue::Float64(val) => val as u64, // Cast if it's a Float64
            //     _ => panic!("Unexpected value type!"),
            // };

            // let underflow_x_as_u64 = match underflow_x_value {
            //     AnyValue::Int64(val) => val as u64,   // Cast if it's an Int64
            //     AnyValue::Float64(val) => val as u64, // Cast if it's a Float64
            //     _ => panic!("Unexpected value type!"),
            // };

            // let underflow_y_as_u64 = match underflow_y_value {
            //     AnyValue::Int64(val) => val as u64,   // Cast if it's an Int64
            //     AnyValue::Float64(val) => val as u64, // Cast if it's a Float64
            //     _ => panic!("Unexpected value type!"),
            // };

            // hist.lock().unwrap().overflow = (overflow_x_as_u64, overflow_y_as_u64);
            // hist.lock().unwrap().underflow = (underflow_x_as_u64, underflow_y_as_u64);

            let lf = lf.clone();
            let name = name.to_string();
            let x_column_name = x_column_name.to_string();
            let y_column_name = y_column_name.to_string();

            hist.lock().unwrap().plot_settings.cuts.x_column = x_column_name.clone();
            hist.lock().unwrap().plot_settings.cuts.y_column = y_column_name.clone();

            log::info!(
                "Starting to fill 2D histogram '{}' with data from columns '{}' and '{}'",
                name,
                x_column_name,
                y_column_name
            );

            // Spawn a new thread for the filling operation
            let handle = std::thread::spawn(move || {
                log::info!("Thread started for filling 2D histogram '{}'", name);

                if let Ok(df) = lf
                    .select([col(&x_column_name), col(&y_column_name)])
                    .filter(filter_expr.clone()) // Clone for logging purposes
                    .collect()
                {
                    log::info!("Data collected for 2D histogram '{}'", name);

                    let x_values = df.column(&x_column_name).unwrap().f64().unwrap();
                    let y_values = df.column(&y_column_name).unwrap().f64().unwrap();
                    let total_steps = x_values.len();

                    log::info!(
                        "2D Histogram '{}' will be filled with {} value pairs from columns '{}' and '{}'",
                        name,
                        total_steps,
                        x_column_name,
                        y_column_name
                    );

                    for (i, (x_value, y_value)) in x_values.iter().zip(y_values.iter()).enumerate()
                    {
                        if let (Some(x), Some(y)) = (x_value, y_value) {
                            let mut hist = hist.lock().unwrap(); // Lock the mutex to access the correct Histogram2D
                            hist.fill(x, y, i, total_steps); // Pass the progress to the fill method
                        }
                    }

                    log::info!("Completed filling 2D histogram '{}'", name);

                    // Optionally: Set progress to None or trigger any final updates here
                    hist.lock().unwrap().plot_settings.progress = None;
                } else {
                    log::error!("Failed to collect LazyFrame for 2D histogram '{}'", name);
                }
            });

            // Store the thread handle in the vector
            self.handles.push(handle);

            return true;
        }

        log::error!("2D Histogram '{}' not found in the tree", name);
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
    ) {
        self.add_hist2d(name, bins, range); // Add the histogram.
        self.fill_hist2d(name, lf, x_column_name, y_column_name); // Fill it with data.
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

    pub fn check_and_join_finished_threads(&mut self) {
        // Only proceed if there are threads to check
        if self.handles.is_empty() {
            return;
        }

        let mut finished_indices = Vec::new();

        // First, identify all the threads that have finished
        for (i, handle) in self.handles.iter().enumerate() {
            if handle.is_finished() {
                finished_indices.push(i);
            }
        }

        // Then, remove and join the finished threads
        for &i in finished_indices.iter().rev() {
            let handle = self.handles.swap_remove(i);
            match handle.join() {
                Ok(_) => log::info!("A thread completed successfully."),
                Err(e) => log::error!("A thread encountered an error: {:?}", e),
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Check and join finished threads
        self.check_and_join_finished_threads();

        self.tree.ui(&mut self.behavior, ui);
    }

    /// Main UI function to display the histogram selection panel and other components
    pub fn side_panel_ui(&mut self, ui: &mut egui::Ui) {
        self.behavior.ui(ui);
        ui.separator();

        ui.collapsing("Histogrammer", |ui| {
            if let Some(root) = self.tree.root() {
                // if ui.button("Reorganize").clicked() {
                //     self.reorganize();
                // }
                tree_ui(ui, &mut self.behavior, &mut self.tree.tiles, root);
            }
        });
    }

    pub fn create_tabs(&mut self, name: String) -> TileId {
        // Ensure root container exists
        let mut current_container_id = self.ensure_root();
        let path_components: Vec<&str> = name.split('/').collect();
        let mut accumulated_path = String::new();

        // Traverse each component in the name to build the tab structure
        for (i, component) in path_components.iter().enumerate() {
            if i > 0 {
                accumulated_path.push('/');
            }
            accumulated_path.push_str(component);

            if let Some(container_info) = self.grid_histogram_map.get(&accumulated_path) {
                current_container_id = container_info.tab_id;
            } else {
                log::debug!(
                    "Creating tab '{}' for path '{}'",
                    component,
                    accumulated_path
                );

                // Create a new tab container at each level except the last
                if i < path_components.len() - 1 {
                    let new_tab_id = self.add_tab_container(current_container_id, component);

                    self.grid_histogram_map.insert(
                        accumulated_path.clone(),
                        ContainerInfo {
                            container_type: ContainerType::Tabs,
                            parent_id: Some(current_container_id),
                            children: vec![],
                            display_name: component.to_string(),
                            tab_id: new_tab_id,
                        },
                    );
                    current_container_id = new_tab_id;
                }
            }
        }

        // The last component should have a grid for holding histograms
        let histograms_grid_path = format!("{}/Histograms", accumulated_path);
        if let Some(container_info) = self.grid_histogram_map.get(&histograms_grid_path) {
            log::debug!(
                "Reusing existing Histograms grid for path: {}",
                histograms_grid_path
            );
            container_info.tab_id
        } else {
            let grid_id = self.add_histograms_grid(current_container_id);
            log::debug!(
                "Creating new Histograms grid for path: {}",
                histograms_grid_path
            );

            self.grid_histogram_map.insert(
                histograms_grid_path.clone(),
                ContainerInfo {
                    container_type: ContainerType::Grid,
                    parent_id: Some(current_container_id),
                    children: vec![],
                    display_name: "Histograms".to_string(),
                    tab_id: grid_id,
                },
            );
            grid_id
        }
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
        // Construct the key for lookup in grid_histogram_map
        let histograms_key = format!("{:?}/Histograms", parent_tab_id);

        // Check if there's already a "Histograms" grid under the parent
        if let Some(container_info) = self.grid_histogram_map.get(&histograms_key) {
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

        // Add the new grid to grid_histogram_map with the correct name and ID
        self.grid_histogram_map.insert(
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

    // pub fn reorganize(&mut self) {
    // use std::collections::HashSet;
    // let mut processed_tabs: HashSet<TileId> = HashSet::new();

    // for (_path, tab_info) in &self.grid_histogram_map {
    //     // Verify the tab itself exists in the tiles
    //     if !self.tree.tiles.contains_key(tab_info.tab_id) {
    //         log::debug!("Skipping missing tab: {:?}", tab_info.tab_id);
    //         continue;
    //     }

    //     // Handle parent-child relationships if parent exists
    //     if let Some(parent_id) = tab_info.parent_id {
    //         if self.tree.tiles.contains_key(parent_id) {
    //             if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(parent_tabs))) =
    //                 self.tree.tiles.get_mut(parent_id)
    //             {
    //                 if !parent_tabs.children.contains(&tab_info.tab_id) {
    //                     parent_tabs.add_child(tab_info.tab_id);
    //                 }
    //             }
    //         } else {
    //             log::debug!("Skipping missing parent: {:?}", parent_id);
    //             continue;
    //         }
    //     }

    //     // Validate and collect existing children for the tab
    //     let valid_children: Vec<TileId> = tab_info
    //         .children
    //         .iter()
    //         .copied()
    //         .filter(|&child_id| self.tree.tiles.contains_key(child_id))
    //         .collect();

    //     // Add each valid child to the tab container
    //     if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tab_container))) =
    //         self.tree.tiles.get_mut(tab_info.tab_id)
    //     {
    //         for child_id in valid_children {
    //             if !tab_container.children.contains(&child_id) && child_id != tab_info.tab_id {
    //                 tab_container.add_child(child_id);
    //             }
    //         }
    //     }

    //     // Mark this tab as processed
    //     processed_tabs.insert(tab_info.tab_id);
    // }
    // }

    pub fn retrieve_active_cuts(&self, cut_handler: &mut CutHandler) {
        for (_id, tile) in self.tree.tiles.iter() {
            if let egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) = tile {
                let hist = hist.lock().unwrap();
                let active_cuts = hist.plot_settings.cuts.clone();

                // Update cuts with correct column names and avoid duplicates
                for mut new_cut in active_cuts.cuts {
                    // Set the correct column names in the Cut struct
                    new_cut.x_column = hist.plot_settings.cuts.x_column.clone();
                    new_cut.y_column = hist.plot_settings.cuts.y_column.clone();

                    cut_handler.cuts.push(new_cut);
                }
            }
        }
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
