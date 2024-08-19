use super::configure_auxillary_detectors::AuxillaryDetectors;
use super::configure_lazyframes::{LazyFrameInfo, LazyFrames};
use super::histogram_ui_elements::{Histo1dConfig, Histo2dConfig, HistoConfig};
use super::manual_histogram_script::manual_add_histograms;

use crate::histoer::histogrammer::Histogrammer;
use polars::prelude::*;

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct HistogramScript {
    pub lazyframe_info: LazyFrameInfo,
    pub histograms: Vec<HistoConfig>,
    pub manual_histogram_script: bool,

    pub add_auxillary_detectors: bool,
    pub auxillary_detectors: Option<AuxillaryDetectors>,
}

impl HistogramScript {
    pub fn new() -> Self {
        Self {
            lazyframe_info: LazyFrameInfo::default(),
            histograms: Vec::new(),
            add_auxillary_detectors: false,
            auxillary_detectors: None,
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

        if self.add_auxillary_detectors {
            if let Some(auxillary_detectors) = &self.auxillary_detectors {
                let aux_columns = auxillary_detectors.get_column_names();
                let aux_lf_names = auxillary_detectors.get_lf_names();

                lazyframe_info.lfs.extend(aux_lf_names);
                lazyframe_info.columns.extend(aux_columns);
            }
        }

        self.lazyframe_info = lazyframe_info;
    }

    pub fn get_hist_names(&self) -> Vec<String> {
        self.histograms.iter().map(|hist| hist.name()).collect()
    }

    pub fn add_histogram1d(&mut self, config: Histo1dConfig) {
        self.histograms.push(HistoConfig::Histo1d(config));
    }

    pub fn add_histogram2d(&mut self, config: Histo2dConfig) {
        self.histograms.push(HistoConfig::Histo2d(config));
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let max_height = ui.min_rect().height();

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
            ui.horizontal(|ui| {
                ui.label("Auxillary Detectors");
                ui.checkbox(&mut self.add_auxillary_detectors, "Add Auxillary Detectors");
            });

            if self.add_auxillary_detectors {
                if let Some(auxillary_detectors) = &mut self.auxillary_detectors {
                    auxillary_detectors.ui(ui);
                } else {
                    self.auxillary_detectors = Some(AuxillaryDetectors::default());
                }
                ui.separator();
            }

            ui.horizontal(|ui| {
                ui.label("Add Histogram");
                if ui.button("1d").clicked() {
                    self.add_histogram1d(Histo1dConfig::default());
                }
                if ui.button("2d").clicked() {
                    self.add_histogram2d(Histo2dConfig::default());
                }
            });

            ui.separator();

            let mut to_remove: Option<usize> = None;

            ui.heading("Histograms");
            egui::ScrollArea::vertical()
                .id_source("HistogramScriptScrollArea")
                .max_height(max_height * 0.6)
                .show(ui, |ui| {
                    egui::Grid::new("Histogram Config")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Name                                             ");
                            });
                            ui.label("LazyFrame");
                            ui.label("Column");
                            ui.label("Bins");
                            ui.label("Range");
                            ui.label("");
                            ui.label("Grids");
                            ui.label("");
                            ui.end_row();
                            for (i, config) in &mut self.histograms.iter_mut().enumerate() {
                                config.ui(ui, self.lazyframe_info.clone());

                                // Remove button
                                if ui.button("X").clicked() {
                                    to_remove = Some(i);
                                }
                                ui.end_row();
                            }
                        });
                });

            if let Some(index) = to_remove {
                self.histograms.remove(index);
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

            // add auxillary detectors columns to the raw lazyframe
            if self.add_auxillary_detectors {
                if let Some(auxillary_detectors) = &self.auxillary_detectors {
                    lf = auxillary_detectors.add_columns_to_lazyframe(&lf);
                }
            }

            // add the main lfs to the lazyframes
            lazyframes.lfs = lazyframes.filtered_lfs(lf.clone());

            // add auxillary detectors lfs to the lazyframes
            if self.add_auxillary_detectors {
                if let Some(auxillary_detectors) = &self.auxillary_detectors {
                    let aux_filtered_lfs = auxillary_detectors.filterd_lazyframes(lf.clone());
                    for (name, lf) in aux_filtered_lfs {
                        lazyframes.lfs.insert(name, lf);
                    }
                }
            }

            for hist in self.histograms.iter_mut() {
                match hist {
                    HistoConfig::Histo1d(config) => {
                        if config.calculate {
                            if let Some(lf) = lazyframes.get_lf(&config.lazyframe) {
                                let name = config.name.clone();
                                let column = config.column.clone();
                                let bins = config.bins;
                                let range = config.range;
                                h.add_fill_hist1d(&name, lf, &column, bins, range, None);
                            } else {
                                log::error!("LazyFrame not found: {}", config.lazyframe);
                            }
                        }
                    }
                    HistoConfig::Histo2d(config) => {
                        if config.calculate {
                            if let Some(lf) = lazyframes.get_lf(&config.lazyframe) {
                                let name = config.name.clone();
                                let x_column = config.x_column.clone();
                                let y_column = config.y_column.clone();
                                let bins = config.bins;
                                let range = config.range;
                                h.add_fill_hist2d(
                                    &name, lf, &x_column, &y_column, bins, range, None,
                                );
                            } else {
                                log::error!("LazyFrame not found: {}", config.lazyframe);
                            }
                        }
                    }
                }
            }
        }
    }
}
