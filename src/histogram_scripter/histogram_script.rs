use super::configure_auxillary_detectors::AuxillaryDetectors;
use super::configure_histograms::{Histo1dConfig, Histo2dConfig, HistoConfig};
use super::configure_lazyframes::{LazyFrameInfo, LazyFrames};
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

    pub progress: f32,
}

impl HistogramScript {
    pub fn new() -> Self {
        Self {
            lazyframe_info: LazyFrameInfo::default(),
            histograms: Vec::new(),
            add_auxillary_detectors: false,
            auxillary_detectors: None,
            manual_histogram_script: false,
            progress: 0.0,
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
        ui.checkbox(&mut self.manual_histogram_script, "Manual Histogram Script");
        if self.manual_histogram_script {
            ui.label("Manual Histogram Script Enabled");
            ui.label(
                "Create your custom script in src/histogram_scripter/manual_histogram_script.rs",
            );
        } else {
            self.get_lazyframe_info();

            // UI for progress bar
            ui.horizontal(|ui| {
                ui.label("Progress:");
                ui.add(egui::ProgressBar::new(self.progress).show_percentage());
            });

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

            egui::ScrollArea::vertical()
                .id_source("HistogramScriptScrollArea")
                .show(ui, |ui| {
                    egui::Grid::new("Histogram Config")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Name                                        ");
                            });
                            ui.label("LazyFrame");
                            ui.label("Column");
                            ui.label("Bins");
                            ui.label("Range");
                            ui.label("");
                            ui.label("");
                            ui.end_row();
                            for (i, config) in &mut self.histograms.iter_mut().enumerate() {
                                config.ui(ui, self.lazyframe_info.clone());

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
        }
    }

    pub fn add_histograms(&mut self, lf: LazyFrame) -> Result<Histogrammer, PolarsError> {
        if self.manual_histogram_script {
            match manual_add_histograms(lf.clone()) {
                Ok(h) => Ok(h),
                Err(e) => {
                    log::error!("Failed to create histograms: {}", e);
                    Err(e)
                }
            }
        } else {
            self.progress = 0.0;

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
                    let aux_filtered_lfs = auxillary_detectors.time_filterd_lazyframes(lf.clone());
                    for (name, lf) in aux_filtered_lfs {
                        lazyframes.lfs.insert(name, lf);
                    }
                }
            }

            let mut histogrammer = Histogrammer::new();

            let total_histograms = self.histograms.len() as f32;
            for (i, hist) in self.histograms.iter_mut().enumerate() {
                match hist {
                    HistoConfig::Histo1d(config) => {
                        if let Some(lf) = lazyframes.get_lf(&config.lazyframe) {
                            let name = config.name.clone();
                            let column = config.column.clone();
                            let bins = config.bins;
                            let range = config.range;
                            histogrammer.add_fill_hist1d(&name, lf, &column, bins, range);
                        } else {
                            log::error!("LazyFrame not found: {}", config.lazyframe);
                        }
                    }
                    HistoConfig::Histo2d(config) => {
                        if let Some(lf) = lazyframes.get_lf(&config.lazyframe) {
                            let name = config.name.clone();
                            let x_column = config.x_column.clone();
                            let y_column = config.y_column.clone();
                            let bins = config.bins;
                            let range = config.range;
                            histogrammer
                                .add_fill_hist2d(&name, lf, &x_column, &y_column, bins, range);
                        } else {
                            log::error!("LazyFrame not found: {}", config.lazyframe);
                        }
                    }
                }
                // Update progress
                self.progress = (i as f32 + 1.0) / total_histograms;
            }

            let hist_names = self.get_hist_names();
            let pane_names: Vec<&str> = hist_names.iter().map(|s| s.as_str()).collect();
            let panes = histogrammer.get_panes(pane_names);
            histogrammer.tabs.insert("All".to_string(), panes);

            Ok(histogrammer)
        }
    }
}
