use crate::histoer::{configs::Configs, cuts::Cuts};
use crate::histogram_scripter::fsu_custom_script::se_sps::SPSOptions;

use super::fsu_custom_script::catrina::CATRiNAConfig;
use super::fsu_custom_script::cebra::CeBrAConfig;
// use super::fsu_custom_script::general::Calibration;
use super::fsu_custom_script::icespice::ICESPICEConfig;
// use super::fsu_custom_script::se_sps::{SPSConfig, SPSOptions};
use super::fsu_custom_script::se_sps::SPSConfig;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct CustomConfigs {
    pub sps: SPSConfig,
    pub cebra: CeBrAConfig,
    pub catrina: CATRiNAConfig,
    pub icespice: ICESPICEConfig,
}

impl Default for CustomConfigs {
    fn default() -> Self {
        Self {
            sps: SPSConfig::new(),
            cebra: CeBrAConfig::default(),
            catrina: CATRiNAConfig::default(),
            icespice: ICESPICEConfig::default(),
        }
    }
}

impl CustomConfigs {
    pub fn active_filter_cuts(&self) -> Cuts {
        let mut cuts = Cuts::default();

        if self.sps.active {
            cuts.merge(&self.sps.sps_cuts.get_active_cuts());
        }

        cuts
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, available_cuts: &Cuts, column_names: &[String]) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Custom Configs: ");
            ui.checkbox(&mut self.sps.active, "SPS");
            ui.checkbox(&mut self.cebra.active, "CeBrA");
            ui.checkbox(&mut self.catrina.active, "CATRiNA");
            // ui.checkbox(&mut self.icespice.active, "ICESPICE");
        });

        ui.separator();

        // ui.horizontal(|ui| {
        //     ui.label("Previous Experiments: ");
        //     if ui.button("52Cr(d,p)53Cr").clicked() {
        //         self.cr52dp_experiment();
        //     }
        // });

        // ui.separator();

        if self.sps.active {
            ui.collapsing("SE-SPS", |ui| {
                self.sps.ui(ui, available_cuts);
            });
        }

        if self.cebra.active {
            ui.collapsing("CeBrA", |ui| {
                self.cebra.ui(ui, &self.sps, column_names);
            });
        }

        if self.catrina.active {
            ui.collapsing("CATRiNA", |ui| {
                self.catrina.ui(ui, column_names);
            });
        }

        // if self.icespice.active {
        //     ui.collapsing("ICESPICE", |ui| {
        //         self.icespice.ui(ui, &mut self.cebra, &mut self.sps);
        //         ui.horizontal_wrapped(|ui| {
        //             if ui.button("Reset").clicked() {
        //                 self.icespice = ICESPICEConfig::default();
        //                 self.icespice.active = true;
        //             }
        //         });
        //     });
        // }
    }

    pub fn merge_active_configs(
        &mut self,
        available_cuts: &Cuts,
        column_names: &[String],
    ) -> Configs {
        let mut configs = Configs::default();
        let sps_selected_cuts = if self.sps.active {
            let selected_cuts = self
                .sps
                .resolved_selected_cuts(available_cuts, column_names);
            (!selected_cuts.is_empty()).then_some(selected_cuts)
        } else {
            None
        };

        if self.sps.active {
            configs.merge(self.sps.configs(available_cuts, column_names));
        }

        if self.cebra.active {
            configs.merge(self.cebra.configs(column_names, &self.sps, &None));

            if let Some(selected_cuts) = sps_selected_cuts {
                configs.merge(
                    self.cebra
                        .configs(column_names, &self.sps, &Some(selected_cuts)),
                );
            }
        }

        if self.catrina.active {
            configs.merge(self.catrina.configs(column_names, &None));
        }

        // if self.icespice.active {
        //     let sps_config = self.sps.clone();
        //     let cebr_config = self.cebra.clone();

        //     if should_calculate_cut_histograms {
        //         let cuts = merged_sps_cuts.clone();
        //         let icespice_configs =
        //             self.icespice
        //                 .icespice_configs(&cebr_config, &sps_config, &Some(cuts));
        //         configs.merge(icespice_configs.clone()); // Ensure `merge` handles in-place modifications
        //     }
        // }

        configs
    }

    pub fn cr52dp_experiment(&mut self) {
        self.sps = SPSConfig {
            active: true,
            options: SPSOptions::default(),
            sps_cuts: Cuts::default(),
        };

        self.cebra.active = true;
        self.cebra.cr52dp_experiment();
    }
}
