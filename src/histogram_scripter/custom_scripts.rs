use crate::histoer::{configs::Configs, cuts::Cuts};

use super::fsu_custom_script::cebra::CeBrAConfig;
use super::fsu_custom_script::general::Calibration;
use super::fsu_custom_script::icespice::ICESPICEConfig;
use super::fsu_custom_script::se_sps::{SPSConfig, SPSOptions};

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Options {
    pub calculate_cut_histograms: bool,
    pub calculate_no_cut_histograms: bool,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct CustomConfigs {
    pub sps: SPSConfig,
    pub cebra: CeBrAConfig,
    pub icespice: ICESPICEConfig,
    pub cuts: Cuts,
    pub options: Options,
}

impl Default for CustomConfigs {
    fn default() -> Self {
        Self {
            sps: SPSConfig::new(),
            cebra: CeBrAConfig::default(),
            icespice: ICESPICEConfig::default(),
            cuts: Cuts::default(),
            options: Options {
                calculate_cut_histograms: true,
                calculate_no_cut_histograms: true,
            },
        }
    }
}

impl CustomConfigs {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Custom Configs: ");
            ui.checkbox(&mut self.sps.active, "SPS");
            ui.checkbox(&mut self.cebra.active, "CeBrA");
            ui.checkbox(&mut self.icespice.active, "ICESPICE");
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Previous Experiments: ");
            if ui.button("52Cr(d,p)53Cr").clicked() {
                self.cr52dp_experiment();
            }
        });

        ui.separator();

        self.cuts.ui(ui);

        ui.separator();

        if !self.cuts.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Options: ");
                ui.checkbox(
                    &mut self.options.calculate_cut_histograms,
                    "Calculate Cut Histograms",
                );
                ui.checkbox(
                    &mut self.options.calculate_no_cut_histograms,
                    "Calculate No Cut Histograms",
                );
            });
        }

        if self.sps.active {
            ui.collapsing("SE-SPS", |ui| {
                self.sps.ui(ui);
                ui.horizontal(|ui| {
                    if ui.button("Reset").clicked() {
                        self.sps = SPSConfig::new();
                        self.sps.active = true;
                    }
                });
            });
        }

        if self.cebra.active {
            ui.collapsing("CeBrA", |ui| {
                self.cebra.ui(ui, &self.sps);
                ui.horizontal(|ui| {
                    if ui.button("Reset").clicked() {
                        self.cebra = CeBrAConfig::default();
                        self.cebra.active = true;
                    }
                });
            });
        }

        if self.icespice.active {
            ui.collapsing("ICESPICE", |ui| {
                self.icespice.ui(ui, &mut self.cebra, &mut self.sps);
                ui.horizontal(|ui| {
                    if ui.button("Reset").clicked() {
                        self.icespice = ICESPICEConfig::default();
                        self.icespice.active = true;
                    }
                });
            });
        }
    }

    pub fn merge_active_configs(&mut self) -> Configs {
        let mut configs = Configs::default();

        if self.sps.active {
            if self.options.calculate_cut_histograms && !self.cuts.is_empty() {
                let cuts = self.cuts.clone();
                let sps_configs = self.sps.sps_configs(Some(cuts));
                configs.merge(sps_configs.clone()); // Ensure `merge` handles in-place modifications
            }

            if self.options.calculate_no_cut_histograms {
                let sps_configs = self.sps.sps_configs(None);
                configs.merge(sps_configs.clone()); // Ensure `merge` handles in-place modifications
            }
        }

        if self.cebra.active {
            for det in self.cebra.detectors.iter_mut() {
                if det.active {
                    let sps_config = self.sps.clone();

                    if self.options.calculate_cut_histograms && !self.cuts.is_empty() {
                        let cuts = self.cuts.clone();
                        let cebr3_configs =
                            det.cebr3_configs(sps_config.clone(), Some(cuts.clone()));
                        configs.merge(cebr3_configs.clone()); // Ensure `merge` handles in-place modifications
                    }

                    if self.options.calculate_no_cut_histograms {
                        let cebr3_configs = det.cebr3_configs(sps_config.clone(), None);
                        configs.merge(cebr3_configs.clone()); // Ensure `merge` handles in-place modifications
                    }
                }
            }
        }

        if self.icespice.active {
            let sps_config = self.sps.clone();
            let cebr_config = self.cebra.clone();

            if self.options.calculate_cut_histograms && !self.cuts.is_empty() {
                let cuts = self.cuts.clone();
                let icespice_configs =
                    self.icespice
                        .icespice_configs(&cebr_config, &sps_config, Some(cuts));
                configs.merge(icespice_configs.clone()); // Ensure `merge` handles in-place modifications
            }

            if self.options.calculate_no_cut_histograms {
                let icespice_configs =
                    self.icespice
                        .icespice_configs(&cebr_config, &sps_config, None);
                configs.merge(icespice_configs.clone()); // Ensure `merge` handles in-place modifications
            }
        }

        configs
    }

    pub fn cr52dp_experiment(&mut self) {
        self.sps = SPSConfig {
            active: true,
            xavg: Calibration {
                a: -0.0023904378617156377,
                b: -18.49776562220117,
                c: 1357.4874219091237,
                bins: 500,
                range: (-100.0, 5500.0),
                active: true,
            },
            options: SPSOptions::default(),
        };

        self.cebra.active = true;
        self.cebra.cr52dp_experiment();
    }
}
