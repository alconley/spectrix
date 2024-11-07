use super::histo1d::histogram1d::Histogram;
use super::histo2d::histogram2d::Histogram2D;
use super::pane::Pane;
use super::tree::TreeBehavior;
use crate::cutter::cut_handler::CutHandler;
use egui_tiles::TileId;
use fnv::FnvHashMap;
use polars::prelude::col;
use polars::prelude::*;
use rayon::prelude::*;
use std::thread::JoinHandle;

use std::convert::TryInto;

use std::sync::{Arc, Mutex};

use std::collections::HashMap;

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

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Histo1DConfig {
    pub name: String,        // Histogram display name
    pub column_name: String, // Data column to fill from
    pub range: (f64, f64),   // Range for the histogram
    pub bins: usize,         // Number of bins
    pub calculate: bool,     // Whether to calculate the histogram
}

impl Histo1DConfig {
    pub fn new(name: &str, column_name: &str, range: (f64, f64), bins: usize) -> Self {
        Self {
            name: name.to_string(),
            column_name: column_name.to_string(),
            range,
            bins,
            calculate: true,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Histo2DConfig {
    pub name: String,          // Histogram display name
    pub x_column_name: String, // Data column for X-axis
    pub y_column_name: String, // Data column for Y-axis
    pub x_range: (f64, f64),   // Range for X-axis
    pub y_range: (f64, f64),   // Range for Y-axis
    pub bins: (usize, usize),  // Number of bins for X and Y axes
    pub calculate: bool,       // Whether to calculate the histogram
}

impl Histo2DConfig {
    pub fn new(
        name: &str,
        x_column_name: &str,
        y_column_name: &str,
        x_range: (f64, f64),
        y_range: (f64, f64),
        bins: (usize, usize),
    ) -> Self {
        Self {
            name: name.to_string(),
            x_column_name: x_column_name.to_string(),
            y_column_name: y_column_name.to_string(),
            x_range,
            y_range,
            bins,
            calculate: true,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Histogrammer {
    pub name: String,
    pub tree: egui_tiles::Tree<Pane>,
    pub behavior: TreeBehavior,
    #[serde(skip)]
    pub handles: Vec<JoinHandle<()>>, // Multiple thread handles
    pub histogram_map: HashMap<String, ContainerInfo>, // Map full path to TabInfo
}

impl Default for Histogrammer {
    fn default() -> Self {
        Self {
            name: "Histogrammer".to_string(),
            tree: egui_tiles::Tree::empty("Empty tree"),
            behavior: Default::default(),
            handles: vec![],
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

    // pub fn fill_histograms(
    //     &mut self,
    //     hist1d_specs: Vec<Histo1DConfig>,
    //     hist2d_specs: Vec<Histo2DConfig>,
    //     lf: &LazyFrame,
    // ) {
    //     // Join any previous threads
    //     for handle in self.handles.drain(..) {
    //         if let Err(e) = handle.join() {
    //             println!("Failed to join thread: {:?}", e);
    //         }
    //     }

    //     let start_time = std::time::Instant::now();

    //     // Collect available column names
    //     let available_columns = self.get_column_names_from_lazyframe(lf);

    //     // Filter hist1d_specs and hist2d_specs to include only those marked for calculation and with existing columns
    //     let hist1d_specs: Vec<_> = hist1d_specs
    //         .into_iter()
    //         .filter(|h| {
    //             if h.calculate {
    //                 if available_columns.contains(&h.column_name) {
    //                     true
    //                 } else {
    //                     println!(
    //                         "Warning: Column '{}' does not exist for 1D histogram: '{}'. Skipping.",
    //                         h.column_name, h.name
    //                     );
    //                     false
    //                 }
    //             } else {
    //                 false
    //             }
    //         })
    //         .collect();

    //     let hist2d_specs: Vec<_> = hist2d_specs
    //         .into_iter()
    //         .filter(|h| {
    //             if h.calculate {
    //                 let missing_columns = !available_columns.contains(&h.x_column_name) || !available_columns.contains(&h.y_column_name);
    //                 if missing_columns {
    //                     println!("Warning: Columns '{}' or '{}' do not exist for 2D histogram '{}'. Skipping.", h.x_column_name, h.y_column_name, h.name);
    //                     false
    //                 } else {
    //                     true
    //                 }
    //             } else {
    //                 false
    //             }
    //         })
    //         .collect();

    //     // Collect column names for the remaining histograms
    //     let mut column_names: Vec<&str> = hist1d_specs
    //         .iter()
    //         .map(|h| h.column_name.as_str())
    //         .collect();
    //     for h in &hist2d_specs {
    //         column_names.push(h.x_column_name.as_str());
    //         column_names.push(h.y_column_name.as_str());
    //     }
    //     column_names.sort_unstable();
    //     column_names.dedup();

    //     // Map column names to expressions for LazyFrame selection
    //     let selected_columns: Vec<_> = column_names.iter().map(|&col_name| col(col_name)).collect();

    //     // Prepare collections for histograms
    //     let mut hist1d_map = Vec::new();
    //     let mut hist2d_map = Vec::new();
    //     let mut missing_1d = Vec::new();
    //     let mut missing_2d = Vec::new();

    //     // Identify and reset existing histograms, collect missing ones
    //     for h in &hist1d_specs {
    //         if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram(hist)))) =
    //             self.tree.tiles.iter_mut().find(|(_id, tile)| {
    //                 if let egui_tiles::Tile::Pane(Pane::Histogram(hist)) = tile {
    //                     hist.lock().unwrap().name == h.name
    //                 } else {
    //                     false
    //                 }
    //             })
    //         {
    //             hist.lock().unwrap().reset(); // Reset histogram counts
    //             hist1d_map.push((Arc::clone(hist), h.clone()));
    //         } else {
    //             missing_1d.push(h.clone());
    //         }
    //     }

    //     for h in &hist2d_specs {
    //         if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram2D(hist)))) =
    //             self.tree.tiles.iter_mut().find(|(_id, tile)| {
    //                 if let egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) = tile {
    //                     hist.lock().unwrap().name == h.name
    //                 } else {
    //                     false
    //                 }
    //             })
    //         {
    //             hist.lock().unwrap().reset(); // Reset histogram counts
    //             hist2d_map.push((Arc::clone(hist), h.clone()));
    //         } else {
    //             missing_2d.push(h.clone());
    //         }
    //     }

    //     // Add missing 1D histograms outside of the mutable borrow loop
    //     for h in missing_1d {
    //         self.add_hist1d(&h.name, h.bins, h.range);
    //         if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram(hist)))) =
    //             self.tree.tiles.iter_mut().find(|(_id, tile)| {
    //                 if let egui_tiles::Tile::Pane(Pane::Histogram(hist)) = tile {
    //                     hist.lock().unwrap().name == h.name
    //                 } else {
    //                     false
    //                 }
    //             })
    //         {
    //             hist1d_map.push((Arc::clone(hist), h));
    //         }
    //     }

    //     // Add missing 2D histograms outside of the mutable borrow loop
    //     for h in missing_2d {
    //         self.add_hist2d(&h.name, h.bins, (h.x_range, h.y_range));
    //         if let Some((_id, egui_tiles::Tile::Pane(Pane::Histogram2D(hist)))) =
    //             self.tree.tiles.iter_mut().find(|(_id, tile)| {
    //                 if let egui_tiles::Tile::Pane(Pane::Histogram2D(hist)) = tile {
    //                     hist.lock().unwrap().name == h.name
    //                 } else {
    //                     false
    //                 }
    //             })
    //         {
    //             hist2d_map.push((Arc::clone(hist), h));
    //         }
    //     }

    //     // Filter and select necessary columns from LazyFrame
    //     let lf_selected = lf.clone().select(selected_columns);

    //     // Spawn a thread for processing histograms
    //     let handle = std::thread::spawn(move || {
    //         if let Ok(df) = lf_selected.collect() {
    //             let height = df.height();

    //             for (hist, _) in &hist1d_map {
    //                 hist.lock().unwrap().reset();
    //             }

    //             for (hist, _) in &hist2d_map {
    //                 hist.lock().unwrap().reset();
    //             }

    //             // Process 1D histograms
    //             hist1d_map.par_iter().for_each(|(hist, meta)| {
    //                 if let Ok(col_idx) = df.column(&meta.column_name) {
    //                     if let Ok(col_values) = col_idx.f64() {
    //                         let mut hist = hist.lock().unwrap();
    //                         for (row_idx, value) in col_values.into_no_null_iter().enumerate() {
    //                             if value == -1e6 {
    //                                 continue;
    //                             }
    //                             if value < meta.range.0 {
    //                                 hist.underflow += 1;
    //                             } else if value > meta.range.1 {
    //                                 hist.overflow += 1;
    //                             } else {
    //                                 hist.fill(value, row_idx, height);
    //                             }
    //                         }
    //                     }
    //                 }
    //             });

    //             // Process 2D histograms
    //             hist2d_map.par_iter().for_each(|(hist, meta)| {
    //                 if let (Ok(x_values), Ok(y_values)) = (
    //                     df.column(&meta.x_column_name).and_then(|c| c.f64()),
    //                     df.column(&meta.y_column_name).and_then(|c| c.f64()),
    //                 ) {
    //                     let mut hist = hist.lock().unwrap();
    //                     for (row_idx, (x, y)) in x_values
    //                         .into_no_null_iter()
    //                         .zip(y_values.into_no_null_iter())
    //                         .enumerate()
    //                     {
    //                         if x == -1e6 || y == -1e6 {
    //                             continue;
    //                         }
    //                         if x < meta.x_range.0 {
    //                             hist.underflow.0 += 1;
    //                         } else if x > meta.x_range.1 {
    //                             hist.overflow.0 += 1;
    //                         }
    //                         if y < meta.y_range.0 {
    //                             hist.underflow.1 += 1;
    //                         } else if y > meta.y_range.1 {
    //                             hist.overflow.1 += 1;
    //                         }
    //                         if x > meta.x_range.0
    //                             && x < meta.x_range.1
    //                             && y > meta.y_range.0
    //                             && y < meta.y_range.1
    //                         {
    //                             hist.fill(x, y, row_idx, height);
    //                         }
    //                     }
    //                 }
    //             });

    //             // Reset progress and recalculate for 2D histograms
    //             for (hist, _) in &hist1d_map {
    //                 hist.lock().unwrap().plot_settings.progress = None;
    //             }
    //             for (hist, _) in &hist2d_map {
    //                 let mut hist = hist.lock().unwrap();
    //                 hist.plot_settings.progress = None;
    //                 hist.plot_settings.recalculate_image = true;
    //             }

    //             let elapsed_time = start_time.elapsed();
    //             println!("\nFilling histograms took: {:?}\n", elapsed_time);
    //         } else {
    //             println!("Failed to collect data for filling histograms.");
    //         }
    //     });

    //     self.handles.push(handle);
    // }

    pub fn fill_histograms(
        &mut self,
        hist1d_specs: Vec<Histo1DConfig>,
        hist2d_specs: Vec<Histo2DConfig>,
        lf: &LazyFrame,
        max_rows_per_batch: usize,
    ) {
        // Join any previous threads
        for handle in self.handles.drain(..) {
            if let Err(e) = handle.join() {
                println!("Failed to join thread: {:?}", e);
            }
        }

        let start_time = std::time::Instant::now();

        // Collect available column names
        let available_columns = self.get_column_names_from_lazyframe(lf);

        // Filter hist1d_specs and hist2d_specs to include only those marked for calculation and with existing columns
        let hist1d_specs: Vec<_> = hist1d_specs
            .into_iter()
            .filter(|h| {
                if h.calculate {
                    if available_columns.contains(&h.column_name) {
                        true
                    } else {
                        println!(
                            "Warning: Column '{}' does not exist for 1D histogram: '{}'. Skipping.",
                            h.column_name, h.name
                        );
                        false
                    }
                } else {
                    false
                }
            })
            .collect();

        let hist2d_specs: Vec<_> = hist2d_specs
            .into_iter()
            .filter(|h| {
                if h.calculate {
                    let missing_columns = !available_columns.contains(&h.x_column_name) || !available_columns.contains(&h.y_column_name);
                    if missing_columns {
                        println!("Warning: Columns '{}' or '{}' do not exist for 2D histogram '{}'. Skipping.", h.x_column_name, h.y_column_name, h.name);
                        false
                    } else {
                        true
                    }
                } else {
                    false
                }
            })
            .collect();

        // Collect column names for the remaining histograms
        let mut column_names: Vec<&str> = hist1d_specs
            .iter()
            .map(|h| h.column_name.as_str())
            .collect();
        for h in &hist2d_specs {
            column_names.push(h.x_column_name.as_str());
            column_names.push(h.y_column_name.as_str());
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

        let mut row_start = 0;
        loop {
            let batch_lf = lf
                .clone()
                .slice(row_start as i64, max_rows_per_batch.try_into().unwrap());
            let lf_selected = batch_lf.select(selected_columns.clone());

            if lf_selected.clone().limit(1).collect().unwrap().height() == 0 {
                break;
            }

            let hist1d_map = hist1d_map.clone();
            let hist2d_map = hist2d_map.clone();

            let handle = std::thread::spawn(move || {
                if let Ok(df) = lf_selected.collect() {
                    let height = df.height();

                    for (hist, _) in &*hist1d_map {
                        hist.lock().unwrap().reset();
                    }

                    for (hist, _) in &*hist2d_map {
                        hist.lock().unwrap().reset();
                    }

                    hist1d_map.par_iter().for_each(|(hist, meta)| {
                        if let Ok(col_idx) = df.column(&meta.column_name) {
                            if let Ok(col_values) = col_idx.f64() {
                                let mut hist = hist.lock().unwrap();
                                for (row_idx, value) in col_values.into_no_null_iter().enumerate() {
                                    if value == -1e6 {
                                        continue;
                                    }
                                    if value < meta.range.0 {
                                        hist.underflow += 1;
                                    } else if value > meta.range.1 {
                                        hist.overflow += 1;
                                    } else {
                                        hist.fill(value, row_idx, height);
                                    }
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
                            for (row_idx, (x, y)) in x_values
                                .into_no_null_iter()
                                .zip(y_values.into_no_null_iter())
                                .enumerate()
                            {
                                if x == -1e6 || y == -1e6 {
                                    continue;
                                }
                                if x < meta.x_range.0 {
                                    hist.underflow.0 += 1;
                                } else if x > meta.x_range.1 {
                                    hist.overflow.0 += 1;
                                }
                                if y < meta.y_range.0 {
                                    hist.underflow.1 += 1;
                                } else if y > meta.y_range.1 {
                                    hist.overflow.1 += 1;
                                }
                                if x > meta.x_range.0
                                    && x < meta.x_range.1
                                    && y > meta.y_range.0
                                    && y < meta.y_range.1
                                {
                                    hist.fill(x, y, row_idx, height);
                                }
                            }
                        }
                    });

                    println!("Processed rows {} to {}", row_start, row_start + height);

                    // Reset progress and recalculate for 2D histograms
                    for (hist, _) in &hist1d_map {
                        hist.lock().unwrap().plot_settings.progress = None;
                    }
                    for (hist, _) in &hist2d_map {
                        let mut hist = hist.lock().unwrap();
                        hist.plot_settings.progress = None;
                        hist.plot_settings.recalculate_image = true;
                    }
                }
            });

            self.handles.push(handle);
            row_start += max_rows_per_batch;
        }

        for handle in self.handles.drain(..) {
            if let Err(e) = handle.join() {
                println!("Failed to join thread: {:?}", e);
            }
        }

        let elapsed_time = start_time.elapsed();
        println!("\nTotal time to fill histograms: {:?}\n", elapsed_time);
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
                if ui.button("Reorganize").clicked() {
                    self.reorganize();
                }
                tree_ui(ui, &mut self.behavior, &mut self.tree.tiles, root);
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

/*

        // // Start filling histograms in a new thread
        // let handle = std::thread::spawn(move || {
        //     if let Ok(df) = lf_selected.collect() {
        //         for row_idx in 0..df.height() {
        //             // Fill 1D histograms
        //             for (hist, col, range, _) in &hist1d_map {
        //                 if let Ok(col_idx) = df.column(col) {
        //                     let col_values = col_idx.f64().unwrap();
        //                     if let Some(value) = col_values.get(row_idx) {
        //                         if value == -1e6 {
        //                             continue;
        //                         }
        //                         let mut hist = hist.lock().unwrap();
        //                         if value < range.0 {
        //                             hist.underflow += 1;
        //                         } else if value > range.1 {
        //                             hist.overflow += 1;
        //                         } else {
        //                             hist.fill(value, row_idx, df.height());
        //                         }
        //                     }
        //                 }
        //             }

        //             // Fill 2D histograms
        //             for (hist, x_col, y_col, (x_range, y_range), _) in &hist2d_map {
        //                 if let (Ok(x_values), Ok(y_values)) = (df.column(x_col), df.column(y_col)) {
        //                     let x_vals = x_values.f64().unwrap();
        //                     let y_vals = y_values.f64().unwrap();
        //                     if let (Some(x), Some(y)) = (x_vals.get(row_idx), y_vals.get(row_idx)) {
        //                         if x == -1e6 || y == -1e6 {
        //                             continue;
        //                         }
        //                         let mut hist = hist.lock().unwrap();
        //                         if x < x_range.0 {
        //                             hist.underflow.0 += 1;
        //                         } else if x > x_range.1 {
        //                             hist.overflow.0 += 1;
        //                         }
        //                         if y < y_range.0 {
        //                             hist.underflow.1 += 1;
        //                         } else if y > y_range.1 {
        //                             hist.overflow.1 += 1;
        //                         }
        //                         if x > x_range.0 && x < x_range.1 && y > y_range.0 && y < y_range.1 {
        //                             hist.fill(x, y, row_idx, df.height());
        //                         }
        //                     }
        //                 }
        //             }
        //         }
        //         log::info!("Completed filling all 1D and 2D histograms row-by-row");

        //         // Reset progress to None for all histograms
        //         for (hist, _, _, _) in &hist1d_map {
        //             hist.lock().unwrap().plot_settings.progress = None;
        //         }
        //         for (hist, _, _, _, _) in &hist2d_map {
        //             hist.lock().unwrap().plot_settings.recalculate_image = true;
        //             hist.lock().unwrap().plot_settings.progress = None;
        //         }

        //     } else {
        //         log::error!("Failed to collect data for filling histograms");
        //     }
        // });

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

*/
