// use super::configure_auxillary_detectors::AuxillaryDetectors;
use super::configure_lazyframes::{LazyFrameInfo, LazyFrames};
use super::histogram_ui_elements::{AddHisto1d, AddHisto2d, FillHisto1d, FillHisto2d, HistoConfig};
use super::manual_histogram_script::manual_add_histograms;

use crate::histoer::histogrammer::Histogrammer;
use polars::prelude::*;

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct HistogramScript {
    pub lazyframe_info: LazyFrameInfo,
    pub add_histograms: Vec<HistoConfig>,
    pub fill_histograms: Vec<HistoConfig>,
    pub grids: Vec<String>,
    pub manual_histogram_script: bool,
}

impl HistogramScript {
    pub fn new() -> Self {
        Self {
            lazyframe_info: LazyFrameInfo::default(),
            add_histograms: vec![],
            fill_histograms: vec![],
            grids: vec![],
            // auxillary_detectors: None,
            manual_histogram_script: true,
        }
    }

    pub fn get_lazyframe_info(&mut self) {
        let mut lazyframe_info = LazyFrameInfo::default();

        let lazyframes = LazyFrames::new();
        let main_columns = lazyframes.main_column_names();
        let main_lf_names = lazyframes.main_lfs_names();

        lazyframe_info.lfs = main_lf_names;
        lazyframe_info.columns = main_columns;

        // if self.add_auxillary_detectors {
        //     if let Some(auxillary_detectors) = &self.auxillary_detectors {
        //         let aux_columns = auxillary_detectors.get_column_names();
        //         let aux_lf_names = auxillary_detectors.get_lf_names();

        //         lazyframe_info.lfs.extend(aux_lf_names);
        //         lazyframe_info.columns.extend(aux_columns);
        //     }
        // }

        self.lazyframe_info = lazyframe_info;
    }

    pub fn get_hist_names(&self) -> Vec<String> {
        self.add_histograms.iter().map(|hist| hist.name()).collect()
    }

    pub fn add_histogram1d(&mut self, config: AddHisto1d) {
        self.add_histograms.push(HistoConfig::AddHisto1d(config));
    }

    pub fn add_histogram2d(&mut self, config: AddHisto2d) {
        self.add_histograms.push(HistoConfig::AddHisto2d(config));
    }

    pub fn fill_histogram1d(&mut self, config: FillHisto1d) {
        self.fill_histograms.push(HistoConfig::FillHisto1d(config));
    }

    pub fn add_fill_histogram2d(&mut self, config: FillHisto2d) {
        self.fill_histograms.push(HistoConfig::FillHisto2d(config));
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.manual_histogram_script, "Manual Histogram Script");
        if self.manual_histogram_script {
            ui.label("Manual Histogram Script Enabled");
            ui.label(
                "Create your custom script in src/histogram_scripter/manual_histogram_script.rs",
            );
        } else {
            self.get_lazyframe_info();

            ui.separator();

            // UI for Auxillary Detectors
            // ui.horizontal(|ui| {
            //     ui.label("Auxillary Detectors");
            //     ui.checkbox(&mut self.add_auxillary_detectors, "Add Auxillary Detectors");
            // });

            // if self.add_auxillary_detectors {
            //     if let Some(auxillary_detectors) = &mut self.auxillary_detectors {
            //         auxillary_detectors.ui(ui);
            //     } else {
            //         self.auxillary_detectors = Some(AuxillaryDetectors::default());
            //     }
            //     ui.separator();
            // }

            ui.heading("Grids");
            ui.horizontal(|ui| {
                ui.label("Grids");
                if ui.button("Add Grid").clicked() {
                    let name = format!("Grid {}", self.grids.len());
                    self.grids.push(name);
                }
            });

            if !self.grids.is_empty() {
                let mut to_remove: Option<usize> = None;
                egui::Grid::new("Grids")
                    .striped(true)
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("");
                        ui.label("Name");
                        ui.end_row();
                        for (i, grid) in &mut self.grids.iter_mut().enumerate() {
                            if ui.button("X").clicked() {
                                to_remove = Some(i);
                            }
                            ui.text_edit_singleline(grid);
                            ui.end_row();
                        }
                    });

                if let Some(index) = to_remove {
                    self.grids.remove(index);
                }
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.heading("Add Histograms");
                if ui.button("1d").clicked() {
                    self.add_histogram1d(AddHisto1d::new(self.add_histograms.len()));
                }
                if ui.button("2d").clicked() {
                    self.add_histogram2d(AddHisto2d::new(self.add_histograms.len()));
                }
            });

            let mut to_remove: Option<usize> = None;
            egui::Grid::new("Add Histogram Config")
                .striped(true)
                .num_columns(5)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name                                             ");
                    });
                    ui.label("Bins");
                    ui.label("Range");
                    ui.label("Grid");
                    ui.label("Remove");
                    ui.end_row();
                    for (i, config) in &mut self.add_histograms.iter_mut().enumerate() {
                        config.add_ui(ui, self.grids.clone());

                        // Remove button
                        if ui.button("X").clicked() {
                            to_remove = Some(i);
                        }
                        ui.end_row();
                    }
                });

            if let Some(index) = to_remove {
                self.add_histograms.remove(index);
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.heading("Fill Histograms");
                if ui.button("1d").clicked() {
                    self.fill_histogram1d(FillHisto1d::new(self.fill_histograms.len()));
                }
                if ui.button("2d").clicked() {
                    self.add_fill_histogram2d(FillHisto2d::new(self.fill_histograms.len()));
                }
            });

            let mut to_remove: Option<usize> = None;

            let histogram_names = self.get_hist_names();

            egui::Grid::new("Histogram Config")
                .striped(true)
                .num_columns(5)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Histogram");
                    });
                    ui.label("LazyFrame");
                    ui.label("Column");
                    ui.label("Calculate");
                    ui.label("Remove");
                    ui.end_row();
                    for (i, config) in &mut self.fill_histograms.iter_mut().enumerate() {
                        config.fill_ui(ui, self.lazyframe_info.clone(), histogram_names.clone());

                        // Remove button
                        if ui.button("X").clicked() {
                            to_remove = Some(i);
                        }
                        ui.end_row();
                    }
                });

            if let Some(index) = to_remove {
                self.fill_histograms.remove(index);
            }

            ui.separator();
        }
    }

    pub fn add_histograms(&mut self, h: &mut Histogrammer, lf: LazyFrame) {
        if self.manual_histogram_script {
            manual_add_histograms(h, lf);
        } else {
            let mut lazyframes = LazyFrames::new();

            let mut lf = lf;
            // add the main extra columns to the raw lazyframe
            lf = lazyframes.add_columns_to_lazyframe(&lf);

            // // add auxillary detectors columns to the raw lazyframe
            // if self.add_auxillary_detectors {
            //     if let Some(auxillary_detectors) = &self.auxillary_detectors {
            //         lf = auxillary_detectors.add_columns_to_lazyframe(&lf);
            //     }
            // }

            // add the main lfs to the lazyframes
            lazyframes.lfs = lazyframes.filtered_lfs(lf.clone());

            // // add auxillary detectors lfs to the lazyframes
            // if self.add_auxillary_detectors {
            //     if let Some(auxillary_detectors) = &self.auxillary_detectors {
            //         let aux_filtered_lfs = auxillary_detectors.filterd_lazyframes(lf.clone());
            //         for (name, lf) in aux_filtered_lfs {
            //             lazyframes.lfs.insert(name, lf);
            //         }
            //     }
            // }

            // add histograms to histogrammer

            for hist in self.add_histograms.iter_mut() {
                match hist {
                    HistoConfig::AddHisto1d(config) => {
                        let name = config.name.clone();
                        let bins = config.bins;
                        let range = config.range;
                        let grid = config.grid.as_deref();
                        h.add_hist1d(&name, bins, range, grid);
                    }
                    HistoConfig::AddHisto2d(config) => {
                        let name = config.name.clone();
                        let bins = config.bins;
                        let range = config.range;
                        let grid = config.grid.as_deref();
                        h.add_hist2d(&name, bins, range, grid);
                    }
                    _ => {}
                }
            }

            // fill histograms
            for hist in self.fill_histograms.iter_mut() {
                match hist {
                    HistoConfig::FillHisto1d(config) => {
                        if let Some(lf) = lazyframes.get_lf(&config.lazyframe) {
                            let name = config.name.clone();
                            let column = config.column.clone();
                            h.fill_hist1d(&name, lf, &column);
                        }
                    }
                    HistoConfig::FillHisto2d(config) => {
                        if let Some(lf) = lazyframes.get_lf(&config.lazyframe) {
                            let name = config.name.clone();
                            let x_column = config.x_column.clone();
                            let y_column = config.y_column.clone();
                            h.fill_hist2d(&name, lf, &x_column, &y_column);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
