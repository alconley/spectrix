use super::calibration::CalibrationScript;
use super::custom_scripts::CustomConfigs;

use crate::histoer::configs::{Configs, get_column_names_from_lazyframe};
use crate::histoer::cuts::{ActiveHistogramCut, Cut, Cuts};
use crate::histoer::histogrammer::Histogrammer;
use polars::prelude::*;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write as _};

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct HistogramScript {
    pub configs: Configs,
    pub custom_scripts: CustomConfigs,
    pub calibration: CalibrationScript,
    pub active_cut_states: HashMap<String, bool>,
}

impl HistogramScript {
    pub fn new() -> Self {
        Self {
            configs: Configs::default(),
            custom_scripts: CustomConfigs::default(),
            calibration: CalibrationScript::default(),
            active_cut_states: HashMap::new(),
        }
    }

    // pub fn save_configs_dialog(&self) {
    //     if let Some(path) = rfd::FileDialog::new()
    //         .add_filter("JSON Config", &["json"])
    //         .set_title("Save Configurations")
    //         .save_file()
    //         && let Ok(serialized) = serde_json::to_string_pretty(&self)
    //         && let Ok(mut file) = File::create(path)
    //     {
    //         let _file = file.write_all(serialized.as_bytes());
    //     }
    // }

    pub fn save_general_configs_dialog(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON Config", &["json"])
            .set_title("Save General Histogram Config")
            .save_file()
            && let Ok(serialized) = serde_json::to_string_pretty(&self.configs)
            && let Ok(mut file) = File::create(path)
        {
            let _file = file.write_all(serialized.as_bytes());
        }
    }

    // pub fn load_configs_dialog(&mut self) {
    //     if let Some(path) = rfd::FileDialog::new()
    //         .add_filter("JSON Config", &["json"])
    //         .set_title("Load Configurations")
    //         .pick_file()
    //         && let Ok(file) = File::open(path)
    //     {
    //         let reader = BufReader::new(file);
    //         if let Ok(loaded) = serde_json::from_reader(reader) {
    //             *self = loaded;
    //         }
    //     }
    // }

    pub fn load_general_configs_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON Config", &["json"])
            .set_title("Load General Histogram Config")
            .pick_file()
            && let Ok(file) = File::open(path)
        {
            let reader = BufReader::new(file);
            if let Ok(loaded) = serde_json::from_reader(reader) {
                self.configs = loaded;
            }
        }
    }

    fn apply_active_cut_states(&mut self, active_cuts: &mut [ActiveHistogramCut]) {
        self.active_cut_states.retain(|cut_name, _| {
            active_cuts
                .iter()
                .any(|active_cut| active_cut.cut.name() == cut_name)
        });

        for active_cut in active_cuts {
            let enabled = self
                .active_cut_states
                .entry(active_cut.cut.name().to_owned())
                .or_insert(true);
            active_cut.enabled = *enabled;
        }
    }

    fn store_active_cut_states(&mut self, active_cuts: &[ActiveHistogramCut]) {
        for active_cut in active_cuts {
            self.active_cut_states
                .insert(active_cut.cut.name().to_owned(), active_cut.enabled);
        }
    }

    fn upsert_cut(cuts: &mut Vec<Cut>, cut: Cut) {
        if let Some(existing_cut) = cuts
            .iter_mut()
            .find(|existing_cut| existing_cut.name() == cut.name())
        {
            *existing_cut = cut;
        } else {
            cuts.push(cut);
        }
    }

    fn resolved_active_histogram_cuts(
        &self,
        histogrammer: &Histogrammer,
    ) -> Vec<ActiveHistogramCut> {
        let mut active_cuts = histogrammer.retrieve_active_histogram_cuts();

        for active_cut in &mut active_cuts {
            active_cut.enabled = self
                .active_cut_states
                .get(active_cut.cut.name())
                .copied()
                .unwrap_or(true);
        }

        active_cuts
    }

    pub fn active_filter_cuts(&self, histogrammer: &Histogrammer) -> Cuts {
        let mut merged_cuts = Cuts::default();

        for cut in self.configs.cuts.get_active_cuts().cuts {
            Self::upsert_cut(&mut merged_cuts.cuts, cut);
        }

        for cut in self.custom_scripts.active_filter_cuts().cuts {
            Self::upsert_cut(&mut merged_cuts.cuts, cut);
        }

        for active_cut in self
            .resolved_active_histogram_cuts(histogrammer)
            .into_iter()
            .filter(|active_cut| active_cut.enabled)
        {
            Self::upsert_cut(&mut merged_cuts.cuts, active_cut.cut);
        }

        merged_cuts
    }

    pub fn active_filter_cut_count(&self, histogrammer: &Histogrammer) -> usize {
        self.active_filter_cuts(histogrammer).cuts.len()
    }

    pub fn available_column_names(&self, column_names: &[String]) -> Vec<String> {
        let mut available_columns = column_names.to_vec();
        available_columns.extend(self.calibration.output_columns(column_names));
        available_columns.sort();
        available_columns.dedup();
        available_columns
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, histogrammer: &Histogrammer, column_names: &[String]) {
        let mut active_cuts = histogrammer.retrieve_active_histogram_cuts();
        self.apply_active_cut_states(&mut active_cuts);

        ui.horizontal(|ui| {
            ui.heading("Histogram Script");

            // ui.separator();

            // if ui.button("Save Configs").clicked() {
            //     self.save_configs_dialog();
            // }
            // if ui.button("Load Configs").clicked() {
            //     self.load_configs_dialog();
            // }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::CollapsingHeader::new("Calibration")
                .default_open(false)
                .show(ui, |ui| {
                    self.calibration.ui(ui, column_names);
                });

            ui.separator();

            let available_columns = self.available_column_names(column_names);

            egui::CollapsingHeader::new("General")
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Save General").clicked() {
                            self.save_general_configs_dialog();
                        }
                        if ui.button("Load General").clicked() {
                            self.load_general_configs_dialog();
                        }
                    });

                    ui.separator();
                    self.configs
                        .ui(ui, Some(active_cuts.as_mut_slice()), &available_columns);
                });

            ui.separator();

            egui::CollapsingHeader::new("Custom")
                .default_open(false)
                .show(ui, |ui| {
                    let merged_general_cuts = self
                        .configs
                        .cuts
                        .merged_with_active_cuts(Some(active_cuts.as_slice()));
                    self.custom_scripts
                        .ui(ui, &merged_general_cuts, &available_columns);
                });
        });

        self.store_active_cut_states(&active_cuts);
    }

    pub fn add_histograms(
        &mut self,
        h: &mut Histogrammer,
        lf: &LazyFrame,
        estimated_memory: f64,
        prefix: Option<String>,
    ) {
        let mut active_cuts = h.retrieve_active_histogram_cuts();
        self.apply_active_cut_states(&mut active_cuts);
        let column_names = get_column_names_from_lazyframe(lf).unwrap_or_else(|error| {
            log::error!("Failed to retrieve column names for custom configs: {error}");
            Vec::new()
        });
        let mut cloned_configs = self.configs.clone();
        let merged_general_cuts = cloned_configs
            .cuts
            .merged_with_active_cuts(Some(active_cuts.as_slice()));
        let available_columns = self.available_column_names(&column_names);
        let active_custom_configs = self
            .custom_scripts
            .merge_active_configs(&merged_general_cuts, &available_columns);
        cloned_configs.sync_histogram_cuts(&merged_general_cuts);
        cloned_configs.merge(active_custom_configs);
        cloned_configs.prepend_computed_columns(self.calibration.computed_columns(&column_names));
        let mut merged_configs = cloned_configs;

        if let Some(prefix) = prefix {
            merged_configs.set_prefix(&prefix);
        }

        self.store_active_cut_states(&active_cuts);
        h.fill_histograms(merged_configs.clone(), lf, estimated_memory);
    }
}
