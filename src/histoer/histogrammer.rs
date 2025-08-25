// External crates
use egui_tiles::TileId;
use fnv::FnvHashMap;
use indicatif::{ProgressBar, ProgressStyle};
use polars::prelude::*;
use pyo3::ffi::c_str;
use pyo3::{prelude::*, types::PyModule};

// Standard library
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

// Project modules
use super::configs::{Config, Configs};
use super::histo1d::histogram1d::Histogram;
use super::histo2d::histogram2d::Histogram2D;
use super::pane::Pane;
use super::tree::TreeBehavior;

use super::cuts::Cuts;

use crate::histoer::configs::Hist1DConfig;
use crate::histoer::configs::Hist2DConfig;

// ADD:
use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::fitter::main_fitter::{FitResult, Fitter};
use crate::fitter::models::gaussian::GaussianFitter;
use std::path::Path;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq, Debug)]
pub enum ContainerType {
    Grid,
    Tabs,
    Vertical,
    Horizontal,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct ContainerInfo {
    container_type: ContainerType,
    parent_id: Option<TileId>,
    children: Vec<TileId>,
    display_name: String,
    tab_id: TileId,
}
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Histogrammer {
    pub name: String,
    pub tree: egui_tiles::Tree<Pane>,
    pub behavior: TreeBehavior,
    #[serde(skip)]
    pub calculating: Arc<AtomicBool>, // Use AtomicBool for thread-safe status tracking
    #[serde(skip)]
    pub abort_flag: Arc<AtomicBool>, // Use AtomicBool for thread-safe abort flag
    #[serde(skip)]
    pub progress: Arc<Mutex<f32>>,
    pub histogram_map: HashMap<String, ContainerInfo>, // Map full path to TabInfo

    #[serde(skip)]
    pub fill_column_wise: bool,
}

impl Default for Histogrammer {
    fn default() -> Self {
        Self {
            name: "Histogrammer".to_owned(),
            tree: egui_tiles::Tree::empty("Empty tree"),
            behavior: Default::default(),
            calculating: Arc::new(AtomicBool::new(false)),
            abort_flag: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(Mutex::new(0.0)),
            histogram_map: HashMap::new(),
            fill_column_wise: true,
        }
    }
}

impl Histogrammer {
    pub fn find_existing_histogram(&self, name: &str) -> Option<TileId> {
        self.tree.tiles.iter().find_map(|(id, tile)| {
            match tile {
                egui_tiles::Tile::Pane(Pane::Histogram(hist)) => {
                    if hist.lock().expect("Failed to lock histogram").name == name {
                        return Some(*id);
                    }
                }
                egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) => {
                    if hist.lock().expect("Failed to lock 2D histogram").name == name {
                        return Some(*id);
                    }
                }
                _ => {}
            }
            None
        })
    }

    pub fn reset_histograms(&mut self) {
        for (_id, tile) in self.tree.tiles.iter_mut() {
            match tile {
                egui_tiles::Tile::Pane(Pane::Histogram(hist)) => {
                    hist.lock().expect("Failed to lock histogram").reset();
                }
                egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) => {
                    hist.lock().expect("Failed to lock 2D histogram").reset();
                }
                _ => {}
            }
        }
    }

    pub fn reset_histogram(&mut self, pane_id: TileId) {
        if let Some((_id, tile)) = self.tree.tiles.iter_mut().find(|(id, _)| **id == pane_id) {
            match tile {
                egui_tiles::Tile::Pane(Pane::Histogram(hist)) => {
                    hist.lock().expect("Failed to lock histogram").reset();
                }
                egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) => {
                    hist.lock().expect("Failed to lock 2D histogram").reset();
                }
                _ => {}
            }
        }
    }

    fn create_1d_pane(&mut self, name: &str, bins: usize, range: (f64, f64)) -> TileId {
        let hist = Histogram::new(name, bins, range);
        let pane = Pane::Histogram(Arc::new(Mutex::new(Box::new(hist))));
        let pane_id = self.tree.tiles.insert_pane(pane);
        self.format_pane_in_containers(name, pane_id);

        pane_id
    }

    fn create_2d_pane(
        &mut self,
        name: &str,
        bins: (usize, usize),
        range: ((f64, f64), (f64, f64)),
    ) -> TileId {
        let hist = Histogram2D::new(name, bins, range);
        let pane = Pane::Histogram2D(Arc::new(Mutex::new(Box::new(hist))));
        let pane_id = self.tree.tiles.insert_pane(pane);
        self.format_pane_in_containers(name, pane_id);

        pane_id
    }

    fn format_pane_in_containers(&mut self, name: &str, pane_id: TileId) {
        // Parse the name to determine its hierarchical structure (e.g., "Tab1/Tab2/Histogram")
        let grid_id = self.create_tabs(name);

        if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Grid(grid))) =
            self.tree.tiles.get_mut(grid_id)
        {
            log::debug!("Adding pane '{name}' to grid container ID {grid_id:?}");
            grid.add_child(pane_id);

            self.histogram_map
                .entry(name.to_owned())
                .and_modify(|container_info| container_info.children.push(pane_id));
        } else {
            log::error!("Failed to retrieve grid container for '{name}'");
        }
    }

    pub fn add_hist1d(&mut self, name: &str, bins: usize, range: (f64, f64)) {
        log::debug!("Creating or updating 1D histogram '{name}'");

        if let Some(pane_id) = self.find_existing_histogram(name) {
            log::debug!("Resetting existing 1D histogram '{name}'");
            self.reset_histogram(pane_id);
        } else {
            log::debug!("No existing histogram found; creating new 1D histogram '{name}'");
            self.create_1d_pane(name, bins, range);
        }
    }

    pub fn add_hist2d(
        &mut self,
        name: &str,
        bins: (usize, usize),
        range: ((f64, f64), (f64, f64)),
    ) {
        log::debug!("Creating or updating 2D histogram '{name}'");

        if let Some(pane_id) = self.find_existing_histogram(name) {
            log::debug!("Resetting existing 2D histogram '{name}'");
            self.reset_histogram(pane_id);
        } else {
            log::debug!("No existing histogram found; creating new 2D histogram '{name}'");
            self.create_2d_pane(name, bins, range);
        }
    }

    pub fn fill_histograms_column_wise(
        &mut self,
        mut configs: Configs,
        lf: &LazyFrame,
        estimated_memory: f64,
    ) {
        let calculating = Arc::clone(&self.calculating);
        let abort_flag = Arc::clone(&self.abort_flag);
        let progress = Arc::clone(&self.progress);

        // Set calculating to true at the start
        calculating.store(true, Ordering::SeqCst);
        abort_flag.store(false, Ordering::SeqCst);

        let mut lf = lf.clone();

        // Validate configurations and prepare histograms
        let valid_configs = configs.valid_configs(&mut lf);
        valid_configs.check_and_add_panes(self);

        // if valid configs is empty, return early
        if valid_configs.is_empty() {
            calculating.store(false, Ordering::SeqCst);
            log::error!("No valid configurations found for histograms.");
            return;
        }

        // Select required columns from the LazyFrame
        let used_columns = valid_configs.get_used_columns();

        let columns = used_columns.len() as u64;

        // Select required columns from the LazyFrame
        let used_columns = valid_configs.get_used_columns();
        let selected_columns: Vec<_> = used_columns.iter().map(col).collect();

        // Estimate rows per chunk
        let bytes_per_row = columns as f64 * 8.0; // Each f64 is 8 bytes
        let chunk_size_bytes = estimated_memory * 1_073_741_824.0;
        let rows_per_chunk = (chunk_size_bytes / bytes_per_row).floor() as usize;

        // Apply the selection to the LazyFrame
        let lf = Arc::new(lf.clone().select(selected_columns.clone()));

        // Initialize histogram maps
        let hist1d_map: Vec<_> = valid_configs
            .configs
            .iter()
            .filter_map(|config| {
                if let Config::Hist1D(hist1d) = config {
                    self.tree.tiles.iter().find_map(|(_id, tile)| match tile {
                        egui_tiles::Tile::Pane(Pane::Histogram(hist))
                            if hist.lock().expect("Failed to lock histogram").name
                                == hist1d.name =>
                        {
                            Some((Arc::clone(hist), hist1d.clone()))
                        }
                        _ => None,
                    })
                } else {
                    None
                }
            })
            .collect();

        let hist2d_map: Vec<_> = valid_configs
            .configs
            .iter()
            .filter_map(|config| {
                if let Config::Hist2D(hist2d) = config {
                    self.tree.tiles.iter().find_map(|(_id, tile)| match tile {
                        egui_tiles::Tile::Pane(Pane::Histogram2D(hist))
                            if hist.lock().expect("Failed to lock 2D histogram").name
                                == hist2d.name =>
                        {
                            Some((Arc::clone(hist), hist2d.clone()))
                        }
                        _ => None,
                    })
                } else {
                    None
                }
            })
            .collect();

        #[expect(clippy::type_complexity)]
        let mut cut_groups_1d: HashMap<
            String,
            (Cuts, Vec<(Arc<Mutex<Box<Histogram>>>, Hist1DConfig)>),
        > = HashMap::new();

        #[expect(clippy::type_complexity)]
        let mut cut_groups_2d: HashMap<
            String,
            (Cuts, Vec<(Arc<Mutex<Box<Histogram2D>>>, Hist2DConfig)>),
        > = HashMap::new();

        for config in &valid_configs.configs {
            match config {
                Config::Hist1D(hist1d) => {
                    if let Some(hist) = self.tree.tiles.iter().find_map(|(_id, tile)| match tile {
                        egui_tiles::Tile::Pane(Pane::Histogram(hist))
                            if hist.lock().expect("Failed to lock histogram").name
                                == hist1d.name =>
                        {
                            Some(Arc::clone(hist))
                        }
                        _ => None,
                    }) {
                        let key = hist1d.cuts.generate_key();
                        cut_groups_1d
                            .entry(key.clone())
                            .or_insert_with(|| (hist1d.cuts.clone(), vec![]))
                            .1
                            .push((hist, hist1d.clone()));
                    }
                }
                Config::Hist2D(hist2d) => {
                    if let Some(hist) = self.tree.tiles.iter().find_map(|(_id, tile)| match tile {
                        egui_tiles::Tile::Pane(Pane::Histogram2D(hist))
                            if hist.lock().expect("Failed to lock 2D histogram").name
                                == hist2d.name =>
                        {
                            Some(Arc::clone(hist))
                        }
                        _ => None,
                    }) {
                        let key = hist2d.cuts.generate_key();
                        cut_groups_2d
                            .entry(key.clone())
                            .or_insert_with(|| (hist2d.cuts.clone(), vec![]))
                            .1
                            .push((hist, hist2d.clone()));
                    }
                }
            }
        }
        std::thread::spawn({
            let calculating = Arc::clone(&calculating);
            let progress = Arc::clone(&progress);
            let lf = Arc::clone(&lf);
            let total_histos = (hist1d_map.len() + hist2d_map.len()) as u64;

            let progress_bar = ProgressBar::new(total_histos);
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar:40.cyan/blue} {percent}% ({pos}/{len})")
                    .expect("progress bar template")
                    .progress_chars("#>-"),
            );

            move || {
                let mut completed = 0.0;
                let total = (cut_groups_1d
                    .values()
                    .map(|(_, list)| list.len())
                    .sum::<usize>()
                    + cut_groups_2d
                        .values()
                        .map(|(_, list)| list.len())
                        .sum::<usize>()) as f32;

                let mut values: Vec<_> = cut_groups_1d.values().collect();
                values.sort_by_key(|(cuts, _)| format!("{cuts:?}")); // Stable deterministic ordering

                for (cuts, grouped) in values {
                    let filtered_lf = if cuts.is_empty() {
                        Ok((*lf).clone())
                    } else {
                        cuts.filter_lazyframe_in_batches(&(*lf).clone(), rows_per_chunk)
                    };

                    if let Ok(filtered_lf) = filtered_lf {
                        for (hist, config) in grouped {
                            let mut guard = hist.lock().expect("Failed to lock histogram");
                            if let Err(e) = guard.fill_from_lazyframe(
                                filtered_lf.clone(),
                                &config.column_name,
                                -1e6,
                            ) {
                                log::error!("Failed to fill hist1d '{}': {:?}", config.name, e);
                            }
                            guard.plot_settings.egui_settings.reset_axis = true;

                            completed += 1.0;
                            *progress.lock().expect("Failed to lock progress") = completed / total;
                            progress_bar.inc(1);
                            if abort_flag.load(Ordering::SeqCst) {
                                println!("Processing aborted by user.");
                                return;
                            }
                        }
                    }
                }

                // for (cuts, grouped) in cut_groups_1d.values() {
                //     let filtered_lf = if cuts.is_empty() {
                //         Ok((*lf).clone())
                //     } else {
                //         cuts.filter_lazyframe_in_batches(&(*lf).clone(), rows_per_chunk)
                //     };

                //     if let Ok(filtered_lf) = filtered_lf {
                //         for (hist, config) in grouped {
                //             let mut guard = hist.lock().expect("Failed to lock histogram");
                //             if let Err(e) = guard.fill_from_lazyframe(
                //                 filtered_lf.clone(),
                //                 &config.column_name,
                //                 -1e6,
                //             ) {
                //                 log::error!("Failed to fill hist1d '{}': {:?}", config.name, e);
                //             }
                //             guard.plot_settings.egui_settings.reset_axis = true;

                //             completed += 1.0;
                //             *progress.lock().expect("Failed to lock progress") = completed / total;
                //             progress_bar.inc(1);
                //             if abort_flag.load(Ordering::SeqCst) {
                //                 println!("Processing aborted by user.");
                //                 return;
                //             }
                //         }
                //     }
                // }

                // for (cuts, grouped) in cut_groups_2d.values() {
                //     let filtered_lf = if cuts.is_empty() {
                //         Ok((*lf).clone())
                //     } else {
                //         cuts.filter_lazyframe_in_batches(&(*lf).clone(), rows_per_chunk)
                //     };

                //     if let Ok(filtered_lf) = filtered_lf {
                //         for (hist, config) in grouped {
                //             let mut guard = hist.lock().expect("Failed to lock 2D histogram");
                //             if let Err(e) = guard.fill_from_lazyframe(
                //                 filtered_lf.clone(),
                //                 &config.x_column_name,
                //                 &config.y_column_name,
                //                 -1e6,
                //             ) {
                //                 log::error!("Failed to fill hist2d '{}': {:?}", config.name, e);
                //             }

                //             // reset
                //             completed += 1.0;
                //             *progress.lock().expect("Failed to lock progress") = completed / total;
                //             progress_bar.inc(1);
                //             if abort_flag.load(Ordering::SeqCst) {
                //                 println!("Processing aborted by user.");
                //                 return;
                //             }
                //         }
                //     }
                // }

                let mut values: Vec<_> = cut_groups_2d.values().collect();
                values.sort_by_key(|(cuts, _)| format!("{cuts:?}")); // sort by cuts debug string (stable)

                for (cuts, grouped) in values {
                    let filtered_lf = if cuts.is_empty() {
                        Ok((*lf).clone())
                    } else {
                        cuts.filter_lazyframe_in_batches(&(*lf).clone(), rows_per_chunk)
                    };

                    if let Ok(filtered_lf) = filtered_lf {
                        for (hist, config) in grouped {
                            let mut guard = hist.lock().expect("Failed to lock 2D histogram");
                            if let Err(e) = guard.fill_from_lazyframe(
                                filtered_lf.clone(),
                                &config.x_column_name,
                                &config.y_column_name,
                                -1e6,
                            ) {
                                log::error!("Failed to fill hist2d '{}': {:?}", config.name, e);
                            }

                            completed += 1.0;
                            *progress.lock().expect("Failed to lock progress") = completed / total;
                            progress_bar.inc(1);
                            if abort_flag.load(Ordering::SeqCst) {
                                println!("Processing aborted by user.");
                                return;
                            }
                        }
                    }
                }

                progress_bar.finish_with_message("Column-wise histogram fill complete.");
                *progress.lock().expect("Failed to lock progress") = 1.0;
                calculating.store(false, Ordering::SeqCst);
            }
        });
    }

    pub fn fill_histograms(
        &mut self,
        configs: Configs,
        lf: &LazyFrame,
        estimated_memory: f64, // chunk size in GB
    ) {
        // if self.fill_column_wise {
        self.fill_histograms_column_wise(configs, lf, estimated_memory);
        // } else {
        //     self.fill_histograms_row_wise(configs, lf, estimated_memory);
        // }
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
                    hist.lock().expect("Failed to lock histogram").name == name
                } else {
                    false
                }
            })
        {
            hist.lock().expect("Failed to lock histogram").bins = bins.clone();
            hist.lock().expect("Failed to lock histogram").original_bins = bins;
            hist.lock().expect("Failed to lock histogram").underflow = underflow;
            hist.lock().expect("Failed to lock histogram").overflow = overflow;
        }
    }

    pub fn add_hist2d_with_bin_values(
        &mut self,
        name: &str,
        bins: &[Vec<u64>],
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
                    hist.lock().expect("Failed to lock 2D histogram").name == name
                } else {
                    false
                }
            })
        {
            let mut hist = hist.lock().expect("Failed to lock 2D histogram");
            hist.bins.counts = bin_map;
            hist.bins.min_count = min_value;
            hist.bins.max_count = max_value;

            // Flag the image to be recalculated due to new bin values
            hist.plot_settings.recalculate_image = true;
        } else {
            log::error!("2D histogram '{name}' not found in the tree");
        }
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

                    if ui.button("Single Grid").clicked() {
                        self.reorganize_to_single_grid();
                    }

                    ui.separator();

                    if ui.button("Reset").clicked() {
                        *self = Default::default();
                    }
                });
                ui.separator();

                self.behavior.ui(ui);

                ui.separator();

                tree_ui(ui, &mut self.behavior, &mut self.tree.tiles, root);

                ui.separator();

                if ui.button("Create ROOT File").clicked() {
                    // Use rfd to open a file save dialog
                    let file_dialog = rfd::FileDialog::new()
                        .set_title("Save ROOT File")
                        .set_file_name("output.root")
                        .add_filter("ROOT file", &["root"])
                        .save_file();

                    if let Some(path) = file_dialog {
                        // Convert path to a string and call the function
                        if let Some(output_file) = path.to_str() {
                            match self.histograms_to_root(output_file) {
                                Ok(_) => println!("ROOT file created at: {output_file}"),
                                Err(e) => eprintln!("Error creating ROOT file: {e:?}"),
                            }
                        } else {
                            eprintln!("Invalid file path selected.");
                        }
                    } else {
                        println!("File save dialog canceled.");
                    }
                }

                if ui.button("Export All lmfit Fits").clicked()
                    && let Some(dir_path) = rfd::FileDialog::new().pick_folder()
                {
                    let dir_path = dir_path.clone();

                    for (_id, tile) in self.tree.tiles.iter() {
                        if let egui_tiles::Tile::Pane(Pane::Histogram(hist)) = tile {
                            let hist = hist.lock().expect("Failed to lock histogram");
                            let fits = &hist.fits;

                            fits.export_lmfit(&dir_path);
                        }

                        log::info!("All lmfit results exported.");
                    }
                }

                // ADD directly after the export button block:
                if ui.button("Import All lmfit Fits to Histograms").clicked() {
                    self.import_all_lmfit_to_histograms_from_folder();
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

    fn create_tabs(&mut self, name: &str) -> TileId {
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
                            display_name: (*component).to_owned(),
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
                            display_name: "Histograms".to_owned(), // Set display name to "Histograms"
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
                    if let Some(parent_info) = self.histogram_map.get_mut(parent_path)
                        && !parent_info.children.contains(&new_id)
                    {
                        parent_info.children.push(new_id);
                    }
                } else {
                    // If no parent path (i.e., root level), add to main tab
                    let main_tab = self
                        .histogram_map
                        .get_mut("Histogrammer")
                        .expect("Main tab not found");
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
            .set_tile_tab_mapping(new_tab_id, name.to_owned());

        // Attach the new tab to its parent container
        if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(parent_tabs))) =
            self.tree.tiles.get_mut(parent_id)
        {
            parent_tabs.add_child(new_tab_id);
        } else {
            log::error!("Parent container ID {parent_id:?} is not a Tabs container");
        }

        new_tab_id
    }

    fn add_histograms_grid(&mut self, parent_tab_id: TileId) -> TileId {
        // Construct the key for lookup in histogram_map
        let histograms_key = format!("{parent_tab_id:?}/Histograms");

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
            .set_tile_tab_mapping(grid_id, "Histograms".to_owned());

        // Attach this grid to the specified parent tab container
        if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) =
            self.tree.tiles.get_mut(parent_tab_id)
        {
            tabs.add_child(grid_id);
        } else {
            log::error!("Parent container ID {parent_tab_id:?} is not a Tabs container");
        }

        // Add the new grid to histogram_map with the correct name and ID
        self.histogram_map.insert(
            histograms_key.clone(),
            ContainerInfo {
                container_type: ContainerType::Grid,
                parent_id: Some(parent_tab_id),
                children: vec![],
                display_name: "Histograms".to_owned(),
                tab_id: grid_id,
            },
        );

        log::debug!("Created new Histograms grid with ID {grid_id:?}");

        grid_id
    }

    fn reorganize(&mut self) {
        log::info!("histogram_map: {:#?}", self.histogram_map);

        // Step 1: Find the main tab using the root tile
        let main_tab_id = self.tree.root.expect("Main root tile (tab) not found");
        log::info!("Main tab found with ID: {main_tab_id:?}");

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

        // for info in self.histogram_map.values() {
        //     for &child_id in &info.children {
        //         if !self.histogram_map.contains_key(&format!("{child_id:?}")) {
        //             orphans_to_move.push((child_id, info.tab_id));
        //         }
        //     }
        // }

        let mut orphans_to_move = Vec::new();
        // Step 4: Identify and move orphan histogram panes (those not found as keys in `histogram_map`)
        let mut values: Vec<_> = self.histogram_map.values().collect();
        values.sort_by_key(|info| format!("{:?}", info.tab_id));

        for info in values {
            for &child_id in &info.children {
                if !self.histogram_map.contains_key(&format!("{child_id:?}")) {
                    orphans_to_move.push((child_id, info.tab_id));
                }
            }
        }

        for (orphan_id, destination_id) in orphans_to_move {
            log::info!(
                "Moving orphan ID {orphan_id:?} to destination container ID {destination_id:?}"
            );
            self.tree
                .move_tile_to_container(orphan_id, destination_id, 0, true);
        }

        // Step 5: Identify and remove unreferenced (extraneous) containers
        // let mut referenced_ids = std::collections::HashSet::new();
        // for info in self.histogram_map.values() {
        //     referenced_ids.insert(info.tab_id);
        //     if let Some(parent_id) = info.parent_id {
        //         referenced_ids.insert(parent_id);
        //     }
        //     for &child_id in &info.children {
        //         referenced_ids.insert(child_id);
        //     }
        // }

        let mut referenced_ids = std::collections::HashSet::new();

        let mut values: Vec<_> = self.histogram_map.values().collect();
        values.sort_by_key(|info| format!("{:?}", info.tab_id)); // stable ordering

        for info in values {
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
            log::info!("Removing extraneous container with ID {tile_id:?}");
            self.tree.remove_recursively(tile_id);
        }

        log::info!("Reorganization complete.");
    }

    pub fn reorganize_to_single_grid(&mut self) {
        let root_id = self.ensure_root();
        let grid_id = self.add_histograms_grid(root_id);

        // Remove existing children from the grid
        if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Grid(grid))) =
            self.tree.tiles.get_mut(grid_id)
        {
            let old_children = grid.children().copied().collect::<Vec<_>>();
            for child in old_children {
                self.tree.tiles.remove(child);
            }
        }

        // Move all histogram panes into this grid
        for tile_id in self
            .tree
            .tiles
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>()
        {
            if let Some(tile) = self.tree.tiles.get(tile_id)
                && matches!(
                    tile,
                    egui_tiles::Tile::Pane(Pane::Histogram(_) | Pane::Histogram2D(_))
                )
            {
                self.tree.move_tile_to_container(tile_id, grid_id, 0, true);
            }
        }

        log::info!("All histograms moved to a single grid container.");
    }

    pub fn retrieve_active_2d_cuts(&self) {
        let mut active_cuts = Vec::new();
        for (_id, tile) in self.tree.tiles.iter() {
            if let egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) = tile {
                let hist = hist.lock().expect("Failed to lock 2D histogram");
                for cut in &hist.plot_settings.cuts {
                    active_cuts.push(cut.clone());
                }
            }
        }
    }

    pub fn histograms_to_root(&mut self, output_file: &str) -> PyResult<()> {
        // python3 -m venv .venv
        // source .venv/bin/activate
        // export PYO3_PYTHON=$(pwd)/.venv/bin/python
        // export PYTHONPATH=$(pwd)/.venv/lib/python3.12/site-packages
        // cargo run --release

        Python::with_gil(|py| {
            let sys = py.import("sys")?;
            let version: String = sys.getattr("version")?.extract()?;
            let executable: String = sys.getattr("executable")?.extract()?;
            println!("Using Python version: {version}");
            println!("Python executable: {executable}");

            // Check if the `uproot` module can be imported
            if py.import("uproot").is_ok() {
                println!("Successfully imported `uproot` module.");
            } else {
                eprintln!(
                    "Error: `uproot` module could not be found. Make sure you are using the correct Python environment with `uproot` installed."
                );
                return Err(PyErr::new::<pyo3::exceptions::PyImportError, _>(
                    "`uproot` module not available",
                ));
            }

            // Define the Python code as a module
            let code = c_str!(
                "
import numpy as np
import uproot

def write_histograms(output_file, hist1d_data, hist2d_data):
    with uproot.recreate(output_file) as file:
        for name, title, bins, underflow, overflow, range in hist1d_data:
            name = name.replace(' ', '_')
            # Create bin edges for the histogram
            bin_edges = np.linspace(range[0], range[1], len(bins) + 1)
            
            # Include underflow and overflow in the data array
            data = np.array([underflow] + bins + [overflow], dtype=np.float32)
            bins_array = np.array(bins, dtype=np.float32)  # Convert bins to numpy array

            # Define fXaxis using to_TAxis with positional arguments
            fXaxis = uproot.writing.identify.to_TAxis(
                fName='xaxis',         # Temporary name for the X-axis
                fTitle='',       # Title of the X-axis
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
            
        # Write 2D histograms
        for name, title, bins, range_x, range_y in hist2d_data:
            name = name.replace(' ', '_')
            bins = np.array(bins, dtype=np.float32)
            # Flatten the 2D array with added underflow/overflow bins
            bins_with_overflow = np.zeros((bins.shape[0] + 2, bins.shape[1] + 2), dtype=np.float32)
            bins_with_overflow[1:-1, 1:-1] = bins
            data = bins_with_overflow.flatten()

            x_bin_edges = np.linspace(range_x[0], range_x[1], bins.shape[1] + 1)
            y_bin_edges = np.linspace(range_y[0], range_y[1], bins.shape[0] + 1)

            fXaxis = uproot.writing.identify.to_TAxis(
                fName='xaxis',
                fTitle='',
                fNbins=bins.shape[1],
                fXmin=range_x[0],
                fXmax=range_x[1],
                fXbins=x_bin_edges
            )

            fYaxis = uproot.writing.identify.to_TAxis(
                fName='yaxis',
                fTitle='',
                fNbins=bins.shape[0],
                fXmin=range_y[0],
                fXmax=range_y[1],
                fXbins=y_bin_edges
            )

            # Compute required statistical sums
            x_centers = (x_bin_edges[:-1] + x_bin_edges[1:]) / 2
            y_centers = (y_bin_edges[:-1] + y_bin_edges[1:]) / 2

            fTsumw = np.sum(bins)
            fTsumw2 = np.sum(bins**2)
            fTsumwx = np.sum(bins * x_centers[np.newaxis, :])
            fTsumwx2 = np.sum(bins * (x_centers[np.newaxis, :]**2))
            fTsumwy = np.sum(bins * y_centers[:, np.newaxis])
            fTsumwy2 = np.sum(bins * (y_centers[:, np.newaxis]**2))
            fTsumwxy = np.sum(bins * x_centers[np.newaxis, :] * y_centers[:, np.newaxis])

            file[name] = uproot.writing.identify.to_TH2x(
                fName=None,
                fTitle=title,
                data=data,
                fEntries=fTsumw,
                fTsumw=fTsumw,
                fTsumw2=fTsumw2,
                fTsumwx=fTsumwx,
                fTsumwx2=fTsumwx2,
                fTsumwy=fTsumwy,
                fTsumwy2=fTsumwy2,
                fTsumwxy=fTsumwxy,
                fSumw2=None,
                fXaxis=fXaxis,
                fYaxis=fYaxis
            )

    print(f'All histograms written to {output_file}.')
"
            );

            // Compile the Python code into a module
            let module = PyModule::from_code(
                py,
                code,
                c_str!("write_histograms.py"),
                c_str!("write_histograms"),
            )?;

            let mut hist1d_data = Vec::new();
            for (_id, tile) in self.tree.tiles.iter() {
                if let egui_tiles::Tile::Pane(Pane::Histogram(hist)) = tile {
                    let hist = hist.lock().expect("Failed to lock histogram");

                    // strip the last part of the name for the title
                    let name_parts: Vec<&str> = hist.name.split('/').collect();
                    let title =
                        (*name_parts.last().expect("Failed to get last name part")).to_owned();

                    hist1d_data.push((
                        hist.name.clone(),
                        title,
                        hist.bins.clone(),
                        hist.underflow,
                        hist.overflow,
                        hist.range,
                    ));
                }
            }

            let mut hist2d_data = Vec::new();
            for (_id, tile) in self.tree.tiles.iter() {
                if let egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) = tile {
                    let hist = hist.lock().expect("Failed to lock 2D histogram");

                    // Use backup bins if available
                    let bins = hist.backup_bins.as_ref().unwrap_or(&hist.bins);

                    let mut counts_2d = vec![vec![0; bins.x]; bins.y];

                    let mut entries: Vec<_> = bins.counts.iter().collect();
                    entries.sort_by_key(|&(&(x, y), _)| (y, x)); // row-major order, adjust as needed

                    for ((x_idx, y_idx), &count) in entries {
                        if *x_idx < bins.x && *y_idx < bins.y {
                            counts_2d[*y_idx][*x_idx] = count;
                        }
                    }

                    // Extract the range for x and y axes
                    let range_x = (hist.range.x.min, hist.range.x.max);
                    let range_y = (hist.range.y.min, hist.range.y.max);

                    // Create a human-readable title from the histogram name
                    let name_parts: Vec<&str> = hist.name.split('/').collect();
                    let title = (*name_parts.last().unwrap_or(&"")).to_owned();

                    // Add to the data vector
                    hist2d_data.push((
                        hist.name.clone(), // Full histogram name
                        title,             // Human-readable title
                        counts_2d,         // 2D bin counts
                        range_x,           // Range for x-axis
                        range_y,           // Range for y-axis
                    ));
                }
            }

            match module
                .getattr("write_histograms")?
                .call1((output_file, hist1d_data, hist2d_data))
            {
                Ok(_) => println!("Histograms written successfully."),
                Err(e) => eprintln!("Error in Python code: {e:?}"),
            }

            Ok(())
        })
    }

    pub fn import_all_lmfit_to_histograms_from_folder_path(&mut self, folder: &Path) {
        fn sanitize_name_for_filename(s: &str) -> String {
            s.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
        }
        fn stem_before_fit(stem: &str) -> &str {
            stem.split_once("_fit").map(|(lhs, _)| lhs).unwrap_or(stem)
        }

        // Build a lookup of sanitized histogram names -> Arc<Mutex<Box<Histogram>>>
        use std::collections::HashMap;
        let mut lookup: HashMap<String, Arc<Mutex<Box<Histogram>>>> = HashMap::new();
        for (_id, tile) in self.tree.tiles.iter_mut() {
            if let egui_tiles::Tile::Pane(Pane::Histogram(h)) = tile {
                let name = {
                    let guard = h.lock().expect("Failed to lock histogram");
                    guard.name.clone()
                };
                let key = sanitize_name_for_filename(&name);
                lookup.insert(key, Arc::clone(h));
            }
        }

        let read_dir = match std::fs::read_dir(folder) {
            Ok(it) => it,
            Err(e) => {
                log::error!("Failed to read folder {}: {:?}", folder.display(), e);
                return;
            }
        };

        // Collect & sort .sav files for stable ordering
        let mut sav_paths: Vec<_> = read_dir
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("sav"))
            })
            .collect();
        sav_paths.sort();

        let mut imported = 0usize;

        for path in sav_paths {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let base = stem_before_fit(stem).to_owned();

            // primary: sanitized match; fallback: exact (unsanitized) match if export never sanitized
            let target_hist = lookup
                .get(&base)
                .or_else(|| lookup.get(&sanitize_name_for_filename(&base)));

            let Some(hist_arc) = target_hist.cloned() else {
                log::warn!(
                    "No matching histogram for '{}'; file '{}'",
                    base,
                    path.display()
                );
                continue;
            };

            let mut gaussian_fitter = GaussianFitter::default();
            match gaussian_fitter.lmfit(Some(path.clone())) {
                Ok(_) => {
                    let mut new_fitter = Fitter::default();
                    new_fitter.set_name(base.clone());

                    // Composition (total fit curve)
                    new_fitter.composition_line.points = gaussian_fitter.fit_points.clone();

                    // Decomposition (components)
                    for (i, fit) in gaussian_fitter.fit_result.iter().enumerate() {
                        let mut line = EguiLine::new(egui::Color32::from_rgb(150, 0, 255));
                        line.points = fit.fit_points.clone();
                        line.name = format!("{base} Decomposition {i}");
                        new_fitter.decomposition_lines.push(line);
                    }

                    // Background
                    if let Some(background_result) = &gaussian_fitter.background_result {
                        new_fitter.background_result = Some(background_result.clone());
                        new_fitter.background_line.points = background_result.get_fit_points();
                    }

                    new_fitter.fit_result = Some(FitResult::Gaussian(gaussian_fitter.clone()));

                    {
                        let mut hist = hist_arc.lock().expect("Failed to lock histogram");
                        hist.fits.stored_fits.push(new_fitter);
                        hist.plot_settings.egui_settings.reset_axis = true;
                    }

                    imported += 1;
                    log::info!("Imported lmfit for '{}' from {}", base, path.display());
                }
                Err(e) => {
                    log::error!("Failed to import {}: {:?}", path.display(), e);
                }
            }
        }

        if imported == 0 {
            log::warn!("No .sav files were imported; check filename ↔ histogram name mapping.");
        } else {
            log::info!("Imported {imported} lmfit file(s).");
        }
    }

    // ADD: convenience wrapper that opens a folder picker
    pub fn import_all_lmfit_to_histograms_from_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.import_all_lmfit_to_histograms_from_folder_path(&folder);
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

// fn estimate_gb(rows: u64, columns: u64) -> f64 {
//     // Each f64 takes 8 bytes
//     let total_bytes = rows * columns * 8;
//     // Convert bytes to gigabytes
//     total_bytes as f64 / 1024.0 / 1024.0 / 1024.0
// }

// pub fn fill_histograms_row_wise(
//     &mut self,
//     mut configs: Configs,
//     lf: &LazyFrame,
//     estimated_memory: f64, // chunk size in GB
// ) {
//     let calculating = Arc::clone(&self.calculating);
//     let abort_flag = Arc::clone(&self.abort_flag);
//     let progress = Arc::clone(&self.progress);

//     // Set calculating to true at the start
//     calculating.store(true, Ordering::SeqCst);
//     abort_flag.store(false, Ordering::SeqCst);

//     let mut lf = lf.clone();

//     let row_count = lf
//         .clone()
//         .select([len().alias("count")])
//         .collect()
//         .unwrap()
//         .column("count")
//         .unwrap()
//         .u32()
//         .unwrap()
//         .get(0)
//         .unwrap();

//     // Validate configurations and prepare histograms
//     let valid_configs = configs.valid_configs(&mut lf);
//     valid_configs.check_and_add_panes(self);

//     // if valid configs is empty, return early
//     if valid_configs.is_empty() {
//         calculating.store(false, Ordering::SeqCst);
//         log::error!("No valid configurations found for histograms.");
//         return;
//     }

//     // Select required columns from the LazyFrame
//     let used_columns = valid_configs.get_used_columns();
//     let selected_columns: Vec<_> = used_columns.iter().map(col).collect();

//     let columns = used_columns.len() as u64;
//     let rows = row_count as u64;
//     let estimated_gb = estimate_gb(rows, columns);

//     // Estimate rows per chunk
//     let bytes_per_row = columns as f64 * 8.0; // Each f64 is 8 bytes
//     let chunk_size_bytes = estimated_memory * 1_073_741_824.0;
//     let rows_per_chunk = (chunk_size_bytes / bytes_per_row).floor() as usize;

//     let progress_bar = ProgressBar::new(row_count as u64);
//     progress_bar.set_style(
//         ProgressStyle::default_bar()
//             .template(
//                 "[{elapsed_precise}] {bar:40.cyan/blue} {percent}% ({pos}/{len}) ETA: {eta}",
//             )
//             .expect("Failed to set progress bar template")
//             .progress_chars("#>-"),
//     );
//     progress_bar.println(format!("Processing ~{:.2} GB of raw data", estimated_gb));

//     // Apply the selection to the LazyFrame
//     let lf = Arc::new(lf.clone().select(selected_columns.clone()));

//     // Initialize histogram maps
//     let hist1d_map: Vec<_> = valid_configs
//         .configs
//         .iter()
//         .filter_map(|config| {
//             if let Config::Hist1D(hist1d) = config {
//                 self.tree.tiles.iter().find_map(|(_id, tile)| match tile {
//                     egui_tiles::Tile::Pane(Pane::Histogram(hist))
//                         if hist.lock().unwrap().name == hist1d.name =>
//                     {
//                         Some((Arc::clone(hist), hist1d.clone()))
//                     }
//                     _ => None,
//                 })
//             } else {
//                 None
//             }
//         })
//         .collect();

//     let hist2d_map: Vec<_> = valid_configs
//         .configs
//         .iter()
//         .filter_map(|config| {
//             if let Config::Hist2D(hist2d) = config {
//                 self.tree.tiles.iter().find_map(|(_id, tile)| match tile {
//                     egui_tiles::Tile::Pane(Pane::Histogram2D(hist))
//                         if hist.lock().unwrap().name == hist2d.name =>
//                     {
//                         Some((Arc::clone(hist), hist2d.clone()))
//                     }
//                     _ => None,
//                 })
//             } else {
//                 None
//             }
//         })
//         .collect();

//     // let pool = rayon::ThreadPoolBuilder::new()
//     //     .num_threads(4) // choose a number lower than the total core count
//     //     .build()
//     //     .unwrap();

//     // Spawn the batch processing task asynchronously
//     rayon::spawn({
//         // pool.spawn({
//         let calculating = Arc::clone(&calculating);
//         let lf = Arc::clone(&lf); // Clone lf to move into the spawn closure
//         let progress_bar = progress_bar.clone();
//         let total_rows = row_count as f32;

//         move || {
//             let mut row_start = 0;
//             loop {
//                 if abort_flag.load(Ordering::SeqCst) {
//                     println!("Processing aborted by user.");
//                     break;
//                 }
//                 // Slice the LazyFrame into batches
//                 let batch_lf = lf
//                     .as_ref()
//                     .clone()
//                     .slice(row_start as i64, rows_per_chunk.try_into().unwrap());

//                 // Break if no rows are left to process
//                 if batch_lf.clone().limit(1).collect().unwrap().height() == 0 {
//                     break;
//                 }

//                 if let Ok(df) = batch_lf.collect() {
//                     let height = df.height();

//                     // --- Process 1D histograms ---
//                     hist1d_map.par_iter().for_each(|(hist, meta)| {
//                         if let Ok(column) = df.column(&meta.column_name).and_then(|c| c.f64()) {
//                             // Buffer valid updates outside the lock:
//                             let valid_values: Vec<f64> = column
//                                 .into_no_null_iter()
//                                 .enumerate()
//                                 .filter_map(|(index, value)| {
//                                     if value != -1e6 && meta.cuts.valid(&df, index) {
//                                         Some(value)
//                                     } else {
//                                         None
//                                     }
//                                 })
//                                 .collect();

//                             // Lock only once to update the histogram:
//                             {
//                                 let mut hist_guard = hist.lock().unwrap();
//                                 for value in valid_values {
//                                     hist_guard.fill(value);
//                                 }
//                                 // Update the plot settings once after processing the batch.
//                                 hist_guard.plot_settings.egui_settings.reset_axis = true;
//                             }
//                         }
//                     });

//                     // --- Process 2D histograms ---
//                     hist2d_map.par_iter().for_each(|(hist, meta)| {
//                         if let (Ok(x_col), Ok(y_col)) = (
//                             df.column(&meta.x_column_name).and_then(|c| c.f64()),
//                             df.column(&meta.y_column_name).and_then(|c| c.f64()),
//                         ) {
//                             // Buffer valid (x, y) pairs outside the lock:
//                             let valid_pairs: Vec<(f64, f64)> = x_col
//                                 .into_no_null_iter()
//                                 .zip(y_col.into_no_null_iter())
//                                 .enumerate()
//                                 .filter_map(|(index, (x, y))| {
//                                     if x != -1e6 && y != -1e6 && meta.cuts.valid(&df, index) {
//                                         Some((x, y))
//                                     } else {
//                                         None
//                                     }
//                                 })
//                                 .collect();

//                             // Lock once to update the 2D histogram:
//                             {
//                                 let mut hist_guard = hist.lock().unwrap();
//                                 for (x, y) in valid_pairs {
//                                     hist_guard.fill(x, y);
//                                 }
//                                 // Update plot settings after processing the batch.
//                                 hist_guard.plot_settings.recalculate_image = true;
//                                 hist_guard.plot_settings.egui_settings.reset_axis = true;
//                                 hist_guard.plot_settings.x_column = meta.x_column_name.clone();
//                                 hist_guard.plot_settings.y_column = meta.y_column_name.clone();
//                             }
//                         }
//                     });

//                     progress_bar.inc(height as u64);

//                     // Update progress as a percentage
//                     let completed_rows = row_start as f32 + height as f32;
//                     let percentage = completed_rows / total_rows;
//                     {
//                         let mut progress_lock = progress.lock().unwrap();
//                         *progress_lock = percentage;
//                     }
//                 }

//                 row_start += rows_per_chunk;
//             }

//             let mut progress_lock = progress.lock().unwrap();
//             *progress_lock = 1.0;

//             progress_bar.finish_with_message("Processing complete.");
//             // Set calculating to false when processing is complete
//             calculating.store(false, Ordering::SeqCst);
//         }
//     });
// }
