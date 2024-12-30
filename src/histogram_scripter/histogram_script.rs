use super::custom_scripts::CustomConfigs;

use crate::histoer::configs::Configs;
use crate::histoer::histogrammer::Histogrammer;
use polars::prelude::*;
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

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Histogram Script");

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::CollapsingHeader::new("General")
                .default_open(false)
                .show(ui, |ui| {
                    self.configs.ui(ui);
                });

            ui.separator();

            self.custom_scripts.ui(ui);
        });
    }

    pub fn add_histograms(&mut self, h: &mut Histogrammer, lf: LazyFrame, estimated_memory: f64) {
        let active_custom_configs = self.custom_scripts.merge_active_configs();

        let mut cloned_configs = self.configs.clone();
        cloned_configs.merge(active_custom_configs);
        let merged_configs = cloned_configs;

        h.fill_histograms(merged_configs.clone(), &lf, estimated_memory);
    }
}
