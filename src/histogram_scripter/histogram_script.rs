use super::custom_scripts::CustomConfigs;

use crate::histoer::configs::Configs;
use crate::histoer::histogrammer::Histogrammer;
use polars::prelude::*;

use std::fs::File;
use std::io::{BufReader, Write as _};

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct HistogramScript {
    pub configs: Configs,
    pub custom_scripts: CustomConfigs,
}

impl HistogramScript {
    pub fn new() -> Self {
        Self {
            configs: Configs::default(),
            custom_scripts: CustomConfigs::default(),
        }
    }

    pub fn save_configs_dialog(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON Config", &["json"])
            .set_title("Save Configurations")
            .save_file()
        {
            if let Ok(serialized) = serde_json::to_string_pretty(&self) {
                if let Ok(mut file) = File::create(path) {
                    let _file = file.write_all(serialized.as_bytes());
                }
            }
        }
    }

    pub fn load_configs_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON Config", &["json"])
            .set_title("Load Configurations")
            .pick_file()
        {
            if let Ok(file) = File::open(path) {
                let reader = BufReader::new(file);
                if let Ok(loaded) = serde_json::from_reader(reader) {
                    *self = loaded;
                }
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
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
                    self.configs.ui(ui);
                });

            ui.separator();

            egui::CollapsingHeader::new("Custom")
                .default_open(false)
                .show(ui, |ui| {
                    self.custom_scripts.ui(ui);
                });
        });
    }

    pub fn add_histograms(
        &mut self,
        h: &mut Histogrammer,
        lf: &LazyFrame,
        estimated_memory: f64,
        prefix: Option<String>,
    ) {
        let active_custom_configs = self.custom_scripts.merge_active_configs();

        let mut cloned_configs = self.configs.clone();
        cloned_configs.merge(active_custom_configs);
        let mut merged_configs = cloned_configs;

        if let Some(prefix) = prefix {
            merged_configs.set_prefix(&prefix);
        }

        h.fill_histograms(merged_configs.clone(), lf, estimated_memory);
    }
}
