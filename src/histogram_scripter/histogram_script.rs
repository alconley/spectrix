use super::custom_scripts::CustomConfigs;

use crate::histoer::configs::Configs;
use crate::histoer::cuts::ActiveCut2D;
use crate::histoer::histogrammer::Histogrammer;
use polars::prelude::*;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write as _};

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct HistogramScript {
    pub configs: Configs,
    pub custom_scripts: CustomConfigs,
    pub active_cut_states: HashMap<String, bool>,
}

impl HistogramScript {
    pub fn new() -> Self {
        Self {
            configs: Configs::default(),
            custom_scripts: CustomConfigs::default(),
            active_cut_states: HashMap::new(),
        }
    }

    pub fn save_configs_dialog(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON Config", &["json"])
            .set_title("Save Configurations")
            .save_file()
            && let Ok(serialized) = serde_json::to_string_pretty(&self)
            && let Ok(mut file) = File::create(path)
        {
            let _file = file.write_all(serialized.as_bytes());
        }
    }

    pub fn load_configs_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON Config", &["json"])
            .set_title("Load Configurations")
            .pick_file()
            && let Ok(file) = File::open(path)
        {
            let reader = BufReader::new(file);
            if let Ok(loaded) = serde_json::from_reader(reader) {
                *self = loaded;
            }
        }
    }

    fn apply_active_cut_states(&mut self, active_cuts: &mut [ActiveCut2D]) {
        self.active_cut_states.retain(|cut_name, _| {
            active_cuts
                .iter()
                .any(|active_cut| &active_cut.cut.polygon.name == cut_name)
        });

        for active_cut in active_cuts {
            let enabled = self
                .active_cut_states
                .entry(active_cut.cut.polygon.name.clone())
                .or_insert(true);
            active_cut.enabled = *enabled;
        }
    }

    fn store_active_cut_states(&mut self, active_cuts: &[ActiveCut2D]) {
        for active_cut in active_cuts {
            self.active_cut_states
                .insert(active_cut.cut.polygon.name.clone(), active_cut.enabled);
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, histogrammer: &Histogrammer) {
        let mut active_cuts = histogrammer.retrieve_active_2d_cuts();
        self.apply_active_cut_states(&mut active_cuts);

        ui.horizontal(|ui| {
            ui.heading("Histogram Script");

            ui.separator();

            if ui.button("Save Configs").clicked() {
                self.save_configs_dialog();
            }
            if ui.button("Load Configs").clicked() {
                self.load_configs_dialog();
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::CollapsingHeader::new("General")
                .default_open(false)
                .show(ui, |ui| {
                    self.configs.ui(ui, Some(active_cuts.as_mut_slice()));
                });

            ui.separator();

            egui::CollapsingHeader::new("Custom")
                .default_open(false)
                .show(ui, |ui| {
                    self.custom_scripts.ui(ui, Some(active_cuts.as_mut_slice()));
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
        let mut active_cuts = h.retrieve_active_2d_cuts();
        self.apply_active_cut_states(&mut active_cuts);
        let active_custom_configs = self
            .custom_scripts
            .merge_active_configs(Some(active_cuts.as_slice()));

        let mut cloned_configs = self.configs.clone();
        let merged_general_cuts = cloned_configs
            .cuts
            .merged_with_active_cuts(Some(active_cuts.as_slice()));
        cloned_configs.sync_histogram_cuts(&merged_general_cuts);
        cloned_configs.merge(active_custom_configs);
        let mut merged_configs = cloned_configs;

        if let Some(prefix) = prefix {
            merged_configs.set_prefix(&prefix);
        }

        self.store_active_cut_states(&active_cuts);
        h.fill_histograms(merged_configs.clone(), lf, estimated_memory);
    }
}
