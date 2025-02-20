use crate::histoer::{configs::Configs, cuts::Cuts};

use super::fsu_custom_script::cebra::CeBrAConfig;
use super::fsu_custom_script::general::Calibration;
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
    // pub icespice: ICESPICEConfig,
    pub cuts: Cuts,
    pub options: Options,
}

impl Default for CustomConfigs {
    fn default() -> Self {
        Self {
            sps: SPSConfig::new(),
            cebra: CeBrAConfig::default(),
            // icespice: ICESPICEConfig::default(),
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
            // ui.checkbox(&mut self.icespice.active, "ICESPICE");
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

        // if self.icespice.active {
        //     ui.collapsing("ICESPICE", |ui| {
        //         self.icespice.ui(ui, &mut self.cebra, &mut self.sps);
        //         ui.horizontal(|ui| {
        //             if ui.button("Reset").clicked() {
        //                 self.icespice = ICESPICEConfig::default();
        //                 self.icespice.active = true;
        //             }
        //         });
        //     });
        // }
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

        // if self.icespice.active {
        //     let icespice_configs = self.icespice.get_configs(&mut self.cebra, &mut self.sps);
        //     configs.merge(icespice_configs.clone()); // Ensure `merge` handles in-place modifications
        // }

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
                original_column: "Xavg".to_string(),
            },
            options: SPSOptions::default(),
        };

        self.cebra.active = true;
        self.cebra.cr52dp_experiment();
    }
}

// /*************************** ICESPICE Custom Struct ***************************/
// #[derive(Clone, serde::Deserialize, serde::Serialize)]
// pub struct PIPS {
//     pub name: String, // Naming convention is either 1000, 500, 300, 100 for now
//     pub sps_timecut: TimeCut,
//     pub energy_calibration: Calibration,
//     pub active: bool,
//     pub range: (f64, f64),
//     pub bins: usize,
// }

// impl Default for PIPS {
//     fn default() -> Self {
//         Self {
//             name: "1000".to_string(),
//             sps_timecut: TimeCut::default(),
//             energy_calibration: Calibration {
//                 a: 0.0,
//                 b: 1.0,
//                 c: 0.0,
//                 active: false,
//                 range: (0.0, 2000.0),
//                 bins: 2000,
//             },
//             active: false,
//             range: (0.0, 4096.0),
//             bins: 512,
//         }
//     }
// }

// impl PIPS {
//     pub fn new(name: &str) -> Self {
//         Self {
//             name: name.to_string(),
//             sps_timecut: TimeCut::default(),
//             energy_calibration: Calibration::default(),
//             active: false,
//             range: (0.0, 4096.0),
//             bins: 512,
//         }
//     }

//     #[rustfmt::skip]
//     #[allow(clippy::all)]
//     pub fn configs(&self, cebra_config: CeBrAConfig, sps_config: SPSConfig, cuts: Option<Cuts>) -> Configs {
//         let base_path = if cuts.is_none() { "No Cuts/ICESPICE" } else { "Cuts/ICESPICE" };
//         let pips_column = format!("PIPS{}Energy", self.name);
//         let pips_time_column = format!("PIPS{}Time", self.name);
//         let pips_rel_time_column = format!("PIPS{}RelTime", self.name); // PIPS{#}Time - ScintLeftTime

//         let mut configs = Configs::default();

//         let pips_range = self.range;
//         let pips_bins = self.bins;

//         let energy = format!("PIPS{}Energy", self.name);
//         let energy_calibrated = format!("PIPS{}EnergyCalibrated", self.name);
//         let time = format!("PIPS{}Time", self.name);

//         configs.hist1d(&format!("ICESPICE/PIPS{}/PIPS{}Energy", self.name, self.name), &energy, pips_range, pips_bins, None);

//         if self.energy_calibration.active {
//             configs.columns.push(self.energy_calibration.new_column(&energy, &format!("PIPS{}EnergyCalibrated", self.name)));
//             configs.hist1d(&format!("ICESPICE/PIPS{}/PIPS{}EnergyCalibrated", self.name, self.name), &format!("PIPS{}EnergyCalibrated", self.name), self.energy_calibration.range, self.energy_calibration.bins, None);
//             configs.hist1d(&format!("ICESPICE/PIPS/PIPSEnergyCalibrated"), &format!("PIPS{}EnergyCalibrated", self.name), self.energy_calibration.range, self.energy_calibration.bins, None);
//         }

//         if cebra_config.active {
//             for cebr3 in cebra_config.detectors.iter() {
//                 if cebr3.active {

//                     let base_path = format!("ICESPICE/PIPS{}/Cebra{}", self.name, cebr3.number);

//                     let cebr3_energy = format!("Cebra{}Energy", cebr3.number);
//                     let cebr3_energy_calibrated = format!("Cebra{}EnergyCalibrated", cebr3.number);
//                     let cebr3_time = format!("Cebra{}Time", cebr3.number);

//                     let cebr3_range = (0.0, 4096.0);
//                     let cebr3_bins = 4096;

//                     // create the time difference column
//                     configs.columns.push((format!("{} - {}", cebr3_time, time), format!("PIPS{}TimeRelToCebra{}Time", self.name, cebr3.number)));

//                     let valid_time_cut = Cut::new_1d(&format!("Valid PIPS{} Cebra{} Time Cut", self.name, cebr3.number), &format!("PIPS{}Energy > 0.0 && Cebra{}Energy > 0.0", self.name, cebr3.number));
//                     configs.cuts.add_cut(valid_time_cut.clone());
//                     let tcut = Some(Cuts::new(vec![valid_time_cut.clone()]));

//                     configs.hist1d(&format!("{}/PIPS{}TimeRelToCebra{}Time", base_path, self.name, cebr3.number), &format!("PIPS{}TimeRelToCebra{}Time", self.name, cebr3.number), cebr3.pips_timecuts.pips1000.no_cut_range, cebr3.pips_timecuts.pips1000.no_cut_bins, tcut.clone());
//                     configs.hist2d(&format!("{}/PIPS{}Energy v Cebra{}Energy",base_path, self.name, cebr3.number), &cebr3_energy, &energy, cebr3_range, pips_range, (cebr3_bins, pips_bins), tcut.clone());
//                     // configs.hist2d(&format!("{}/PIPS{}RelToCebra{} v Cebra{}Energy", base_path, self.name, cebr3.number, cebr3.number), &format!("PIPS{}TimeRelToCebra{}Time", self.name, cebr3.number), &cebr3_energy, cebr3.pips_timecuts.pips1000.no_cut_range, cebr3_range, (cebr3.pips_timecuts.pips1000.no_cut_bins, cebr3_bins), tcut.clone());

//                     // energy calibrated histograms
//                     if self.energy_calibration.active {
//                         configs.hist2d(&format!("{}/Energy Calibrated/PIPS{}EnergyCalibrated v Cebra{}Energy", base_path, self.name, cebr3.number), &cebr3_energy, &energy_calibrated, cebr3_range, self.energy_calibration.range, (cebr3_bins, self.energy_calibration.bins), None);
//                     }
//                     if cebr3.energy_calibration.active & self.energy_calibration.active {
//                         configs.hist2d(&format!("{}/Energy Calibrated/PIPS{}EnergyCalibrated v Cebra{}EnergyCalibrated", base_path, self.name, cebr3.number), &cebr3_energy_calibrated, &energy_calibrated, cebr3.energy_calibration.range, self.energy_calibration.range, (cebr3.energy_calibration.bins, self.energy_calibration.bins), None);
//                     }
//                     // if cebr3.energy_calibration.active {
//                         // configs.hist2d(&format!("{}/Energy Calibrated/PIPS{}TimeRelToCebra{}Time v Cebra{}EnergyCalibrated", base_path, self.name, cebr3.number, cebr3.number), &format!("PIPS{}TimeRelToCebra{}Time", self.name, cebr3.number), &cebr3_energy_calibrated, cebr3.pips_timecuts.pips1000.no_cut_range, cebr3.energy_calibration.range, (cebr3.pips_timecuts.pips1000.no_cut_bins, cebr3.energy_calibration.bins), None);
//                     // }

//                     // check time cuts for the correct detector
//                     // ONLY PIPS1000 is used for now for testing purposes
//                     if cebr3.pips_timecuts.pips1000.active {
//                         let min = cebr3.pips_timecuts.pips1000.low;
//                         let max = cebr3.pips_timecuts.pips1000.high;
//                         let mean = cebr3.pips_timecuts.pips1000.mean;
//                         let time_range = cebr3.pips_timecuts.pips1000.range;
//                         let time_bins = cebr3.pips_timecuts.pips1000.bins;

//                         // add column for the time cut to shift the time
//                         configs.columns.push((format!("{} - {} - {}", cebr3_time, time, mean), format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number)));

//                         // add the time cut

//                         let pips_cebra_tcut = Cut::new_1d(&format!("PIPS{}+Cebra{} Time Cut", self.name, cebr3.number), &format!("PIPS{}TimeRelToCebra{}Time >= {} && PIPS{}TimeRelToCebra{}Time <= {}", self.name, cebr3.number, min, self.name, cebr3.number, max));
//                         configs.cuts.add_cut(pips_cebra_tcut.clone());
//                         let tcut = Some(Cuts::new(vec![pips_cebra_tcut.clone(), valid_time_cut.clone()]));

//                         configs.hist1d(&format!("{}/Time Cut/PIPS{}TimeRelToCebra{}TimeShifted", base_path, self.name, cebr3.number), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), time_range, time_bins, tcut.clone());
//                         configs.hist1d(&format!("ICESPICE/PIPS{}/CeBrA/Time Cut - PIPS{}TimeRelToCeBrA", self.name, self.name), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), time_range, time_bins, tcut.clone());

//                         configs.hist2d(&format!("{}/Time Cut/PIPS{}Energy v Cebra{}Energy", base_path, self.name, cebr3.number), &cebr3_energy, &energy, cebr3_range, pips_range, (cebr3_bins, pips_bins), tcut.clone());
//                         configs.hist2d(&format!("{}/Time Cut/PIPS{}RelToCebra{}Shifted v Cebra{}Energy", base_path, self.name, cebr3.number, cebr3.number), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), &cebr3_energy, time_range, cebr3_range, (time_bins, cebr3_bins), tcut.clone());
//                         // energy calibrated histograms
//                         if self.energy_calibration.active & !cebr3.energy_calibration.active {
//                             configs.hist2d(&format!("{}/Time Cut/Energy Calibrated/PIPS{}EnergyCalibrated v Cebra{}Energy", base_path, self.name, cebr3.number), &cebr3_energy, &energy_calibrated, cebr3_range, self.energy_calibration.range, (cebr3_bins, self.energy_calibration.bins), tcut.clone());
//                         } else if cebr3.energy_calibration.active & self.energy_calibration.active {
//                             configs.hist2d(&format!("{}/Time Cut/Energy Calibrated/PIPS{}EnergyCalibrated v Cebra{}EnergyCalibrated", base_path, self.name, cebr3.number), &cebr3_energy_calibrated, &energy_calibrated, cebr3.energy_calibration.range, self.energy_calibration.range, (cebr3.energy_calibration.bins, self.energy_calibration.bins), tcut.clone());
//                             configs.hist2d(&format!("ICESPICE/PIPS{}/CeBrA/Time Cut- PIPS{}EnergyCalibrated v CeBrA", self.name, self.name), &cebr3_energy_calibrated, &energy_calibrated, cebr3.energy_calibration.range, self.energy_calibration.range, (cebr3.energy_calibration.bins, self.energy_calibration.bins), tcut.clone());
//                         }
//                         if cebr3.energy_calibration.active {
//                             configs.hist2d(&format!("{}/Time Cut/Energy Calibrated/PIPS{}TimeRelToCebra{}TimeShifted v Cebra{}EnergyCalibrated", base_path, self.name, cebr3.number, cebr3.number), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), &cebr3_energy_calibrated, time_range, cebr3.energy_calibration.range, (time_bins, cebr3.energy_calibration.bins), tcut.clone());
//                             configs.hist2d(&format!("ICESPICE/PIPS{}/CeBrA/Time Cut - PIPS{}TimeRelToCeBrA v CeBrA", self.name, self.name), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), &cebr3_energy_calibrated, time_range, cebr3.energy_calibration.range, (time_bins, cebr3.energy_calibration.bins), tcut.clone());
//                         }
//                     }
//                 }
//             }

//         }

//         if sps_config.active {
//             let rel_time_column = format!("PIPS{}RelTime", self.name); // PIPS{}Time - ScintLeftTime -> Column is made in the eventbuilder with the anode condition too
//             let pips_energy = format!("PIPS{}Energy", self.name);

//             // rel time histogram
//             configs.hist1d(&format!("ICESPICE/PIPS{}/SPS/PIPS{}RelTime", self.name, self.name), &rel_time_column, self.sps_timecut.no_cut_range, self.sps_timecut.no_cut_bins, None);

//             // rel time vs xavg
//             configs.hist2d(&format!("ICESPICE/PIPS{}/SPS/PIPS{}RelTime v Xavg", self.name, self.name), &rel_time_column, &format!("Xavg"), self.sps_timecut.no_cut_range, (-300.0, 300.0), (self.sps_timecut.no_cut_bins, 600), None);

//             // pips energy vs xavg
//             configs.hist2d(&format!("ICESPICE/PIPS{}/SPS/PIPS{}Energy v Xavg", self.name, self.name), &format!("Xavg"), &pips_energy,(-300.0, 300.0), pips_range, (pips_bins, 600), None);

//             if sps_config.xavg.active {
//                 // rel time vs xavg energy calibrated
//                 configs.hist2d(&format!("ICESPICE/PIPS{}/SPS/PIPS{}RelTime v XavgEnergyCalibrated", self.name, self.name), &rel_time_column, &format!("XavgEnergyCalibrated"), self.sps_timecut.no_cut_range, sps_config.xavg.range, (self.sps_timecut.no_cut_bins, sps_config.xavg.bins), None);

//                 // pips energy vs xavg energy calibrated
//                 configs.hist2d(&format!("ICESPICE/PIPS{}/SPS/PIPS{}Energy v XavgEnergyCalibrated", self.name, self.name), &format!("XavgEnergyCalibrated"), &pips_energy, sps_config.xavg.range, pips_range, (pips_bins, sps_config.xavg.bins), None);
//             }

//             if self.sps_timecut.active {
//                 let min = self.sps_timecut.low;
//                 let max = self.sps_timecut.high;
//                 let mean = self.sps_timecut.mean;
//                 let time_range = self.sps_timecut.range;
//                 let time_bins = self.sps_timecut.bins;

//                 // add column for the time cut to shift the time
//                 configs.columns.push((format!("{} - {}", rel_time_column, mean), format!("PIPS{}RelTimeShifted", self.name)));

//                 // add the time cut
//                 let pips_tcut = Cut::new_1d(&format!("PIPS{} Time Cut", self.name), &format!("PIPS{}RelTime >= {} && PIPS{}RelTime <= {}", self.name, min, self.name, max));
//                 configs.cuts.add_cut(pips_tcut.clone());
//                 let tcut = Some(Cuts::new(vec![pips_tcut.clone()]));

//                 configs.hist1d(&format!("ICESPICE/PIPS{}/SPS/Time Cut/PIPS{}RelTimeShifted", self.name, self.name), &format!("PIPS{}RelTimeShifted", self.name), time_range, time_bins, tcut.clone());
//                 configs.hist1d(&format!("ICESPICE/PIPS{}/SPS/Time Cut/PIPS{}Energy", self.name, self.name), &pips_energy, pips_range, pips_bins, tcut.clone());
//                 configs.hist2d(&format!("ICESPICE/PIPS{}/SPS/Time Cut/PIPS{}Energy v Xavg", self.name, self.name), &format!("Xavg"), &pips_energy, (-300.0, 300.0), pips_range, (600, pips_bins), tcut.clone());
//                 configs.hist2d(&format!("ICESPICE/PIPS{}/SPS/Time Cut/PIPS{}RelTimeShifted v Xavg", self.name, self.name), &format!("Xavg"), &format!("PIPS{}RelTimeShifted", self.name), (-300.0, 300.0), time_range, (600, time_bins), tcut.clone());
//             }
//         }

//         configs
//     }

//     pub fn ui(
//         &mut self,
//         ui: &mut egui::Ui,
//         cebra_config: &mut CeBrAConfig,
//         sps_config: &mut SPSConfig,
//     ) {
//         if self.active {
//             // Collapsing header for PIPS detector
//             ui.collapsing(format!("PIPS{}", self.name), |ui| {
//                 // Energy Calibration UI
//                 ui.horizontal(|ui| {
//                     ui.label("Energy Calibration: ");
//                     self.energy_calibration.ui(ui, false);
//                 });

//                 // SPS Time Cut UI
//                 if sps_config.active {
//                     ui.separator();
//                     ui.horizontal(|ui| {
//                         ui.label("SE-SPS Time Cut: ");
//                         ui.add_enabled(
//                             self.sps_timecut.active,
//                             egui::DragValue::new(&mut self.sps_timecut.mean)
//                                 .speed(1)
//                                 .prefix("Mean: "),
//                         );
//                         ui.add_enabled(
//                             self.sps_timecut.active,
//                             egui::DragValue::new(&mut self.sps_timecut.low)
//                                 .speed(1)
//                                 .prefix("Low: "),
//                         );
//                         ui.add_enabled(
//                             self.sps_timecut.active,
//                             egui::DragValue::new(&mut self.sps_timecut.high)
//                                 .speed(1)
//                                 .prefix("High: "),
//                         );
//                         ui.checkbox(&mut self.sps_timecut.active, "Active");
//                     });
//                     ui.separator();
//                 }

//                 // CeBrA Detector Configurations
//                 if cebra_config.active {
//                     for cebr3 in &mut cebra_config.detectors {
//                         if cebr3.active {
//                             ui.horizontal(|ui| {
//                                 ui.label(format!("Cebra{} Time Cut: ", cebr3.number));

//                                 match self.name.as_str() {
//                                     "1000" => {
//                                         ui.add_enabled(
//                                             cebr3.pips_timecuts.pips1000.active,
//                                             egui::DragValue::new(
//                                                 &mut cebr3.pips_timecuts.pips1000.mean,
//                                             )
//                                             .speed(1)
//                                             .prefix("Mean: "),
//                                         );
//                                         ui.add_enabled(
//                                             cebr3.pips_timecuts.pips1000.active,
//                                             egui::DragValue::new(
//                                                 &mut cebr3.pips_timecuts.pips1000.low,
//                                             )
//                                             .speed(1)
//                                             .prefix("Low: "),
//                                         );
//                                         ui.add_enabled(
//                                             cebr3.pips_timecuts.pips1000.active,
//                                             egui::DragValue::new(
//                                                 &mut cebr3.pips_timecuts.pips1000.high,
//                                             )
//                                             .speed(1)
//                                             .prefix("High: "),
//                                         );
//                                         ui.checkbox(
//                                             &mut cebr3.pips_timecuts.pips1000.active,
//                                             "Active",
//                                         );
//                                     }
//                                     "500" => {
//                                         ui.add_enabled(
//                                             cebr3.pips_timecuts.pips500.active,
//                                             egui::DragValue::new(
//                                                 &mut cebr3.pips_timecuts.pips500.mean,
//                                             )
//                                             .speed(1)
//                                             .prefix("Mean: "),
//                                         );
//                                         ui.add_enabled(
//                                             cebr3.pips_timecuts.pips500.active,
//                                             egui::DragValue::new(
//                                                 &mut cebr3.pips_timecuts.pips500.low,
//                                             )
//                                             .speed(1)
//                                             .prefix("Low: "),
//                                         );
//                                         ui.add_enabled(
//                                             cebr3.pips_timecuts.pips500.active,
//                                             egui::DragValue::new(
//                                                 &mut cebr3.pips_timecuts.pips500.high,
//                                             )
//                                             .speed(1)
//                                             .prefix("High: "),
//                                         );
//                                         ui.checkbox(
//                                             &mut cebr3.pips_timecuts.pips500.active,
//                                             "Active",
//                                         );
//                                     }
//                                     "300" => {
//                                         ui.add_enabled(
//                                             cebr3.pips_timecuts.pips300.active,
//                                             egui::DragValue::new(
//                                                 &mut cebr3.pips_timecuts.pips300.mean,
//                                             )
//                                             .speed(1)
//                                             .prefix("Mean: "),
//                                         );
//                                         ui.add_enabled(
//                                             cebr3.pips_timecuts.pips300.active,
//                                             egui::DragValue::new(
//                                                 &mut cebr3.pips_timecuts.pips300.low,
//                                             )
//                                             .speed(1)
//                                             .prefix("Low: "),
//                                         );
//                                         ui.add_enabled(
//                                             cebr3.pips_timecuts.pips300.active,
//                                             egui::DragValue::new(
//                                                 &mut cebr3.pips_timecuts.pips300.high,
//                                             )
//                                             .speed(1)
//                                             .prefix("High: "),
//                                         );
//                                         ui.checkbox(
//                                             &mut cebr3.pips_timecuts.pips300.active,
//                                             "Active",
//                                         );
//                                     }
//                                     "100" => {
//                                         ui.add_enabled(
//                                             cebr3.pips_timecuts.pips100.active,
//                                             egui::DragValue::new(
//                                                 &mut cebr3.pips_timecuts.pips100.mean,
//                                             )
//                                             .speed(1)
//                                             .prefix("Mean: "),
//                                         );
//                                         ui.add_enabled(
//                                             cebr3.pips_timecuts.pips100.active,
//                                             egui::DragValue::new(
//                                                 &mut cebr3.pips_timecuts.pips100.low,
//                                             )
//                                             .speed(1)
//                                             .prefix("Low: "),
//                                         );
//                                         ui.add_enabled(
//                                             cebr3.pips_timecuts.pips100.active,
//                                             egui::DragValue::new(
//                                                 &mut cebr3.pips_timecuts.pips100.high,
//                                             )
//                                             .speed(1)
//                                             .prefix("High: "),
//                                         );
//                                         ui.checkbox(
//                                             &mut cebr3.pips_timecuts.pips100.active,
//                                             "Active",
//                                         );
//                                     }
//                                     _ => {}
//                                 }
//                             });
//                         }
//                     }
//                 }
//             });
//         }
//     }
// }

// #[derive(Clone, serde::Deserialize, serde::Serialize)]
// pub struct ICESPICEConfig {
//     pub pips1000: PIPS,
//     pub pips500: PIPS,
//     pub pips300: PIPS,
//     pub pips100: PIPS,
//     pub active: bool,
// }

// impl Default for ICESPICEConfig {
//     fn default() -> Self {
//         Self {
//             pips1000: PIPS::new("1000"),
//             pips500: PIPS::new("500"),
//             pips300: PIPS::new("300"),
//             pips100: PIPS::new("100"),
//             active: false,
//         }
//     }
// }

// impl ICESPICEConfig {
//     pub fn ui(
//         &mut self,
//         ui: &mut egui::Ui,
//         cebra_config: &mut CeBrAConfig,
//         sps_config: &mut SPSConfig,
//     ) {
//         if !self.active {
//             return;
//         }

//         self.sync_common_values();

//         ui.separator();

//         ui.label("PIPS Detectors");

//         ui.horizontal(|ui| {
//             ui.checkbox(&mut self.pips1000.active, "PIPS1000");
//             ui.checkbox(&mut self.pips500.active, "PIPS500");
//             ui.checkbox(&mut self.pips300.active, "PIPS300");
//             ui.checkbox(&mut self.pips100.active, "PIPS100");
//         });

//         egui::Grid::new("icespice_pips")
//             .striped(true)
//             .show(ui, |ui| {
//                 ui.label("Histogram Settings");
//                 ui.label("Range");
//                 ui.label("Bins");

//                 ui.end_row();

//                 // Default
//                 ui.label("Default:");
//                 ui.horizontal(|ui| {
//                     ui.add(
//                         egui::DragValue::new(&mut self.pips1000.range.0)
//                             .speed(1.0)
//                             .prefix("("),
//                     );
//                     ui.add(
//                         egui::DragValue::new(&mut self.pips1000.range.1)
//                             .speed(1.0)
//                             .suffix(")"),
//                     );
//                 });
//                 ui.add(egui::DragValue::new(&mut self.pips1000.bins).speed(1));

//                 ui.end_row();

//                 // Energy Calibrated
//                 ui.label("Energy Calibrated");
//                 ui.horizontal(|ui| {
//                     ui.add(
//                         egui::DragValue::new(&mut self.pips1000.energy_calibration.range.0)
//                             .speed(1.0)
//                             .prefix("("),
//                     );
//                     ui.add(
//                         egui::DragValue::new(&mut self.pips1000.energy_calibration.range.1)
//                             .speed(1.0)
//                             .suffix(")"),
//                     );
//                 });
//                 ui.add(egui::DragValue::new(&mut self.pips1000.energy_calibration.bins).speed(1));
//                 ui.end_row();

//                 // SPS Time Cut Values
//                 if sps_config.active {
//                     ui.label("SE-SPS: No Time Cut");
//                     ui.horizontal(|ui| {
//                         ui.add(
//                             egui::DragValue::new(&mut self.pips1000.sps_timecut.no_cut_range.0)
//                                 .speed(1.0)
//                                 .prefix("("),
//                         );
//                         ui.add(
//                             egui::DragValue::new(&mut self.pips1000.sps_timecut.no_cut_range.1)
//                                 .speed(1.0)
//                                 .suffix(")"),
//                         );
//                     });
//                     ui.add(
//                         egui::DragValue::new(&mut self.pips1000.sps_timecut.no_cut_bins).speed(1),
//                     );
//                     ui.end_row();

//                     ui.label("SE-SPS: Time Cut:");

//                     ui.horizontal(|ui| {
//                         ui.add(
//                             egui::DragValue::new(&mut self.pips1000.sps_timecut.range.0)
//                                 .speed(1.0)
//                                 .prefix("("),
//                         );
//                         ui.add(
//                             egui::DragValue::new(&mut self.pips1000.sps_timecut.range.1)
//                                 .speed(1.0)
//                                 .suffix(")"),
//                         );
//                     });
//                     ui.add(egui::DragValue::new(&mut self.pips1000.sps_timecut.bins).speed(1));
//                     ui.end_row();
//                 }

//                 // CeBrA Time Cut Values
//                 if cebra_config.active {
//                     if cebra_config.detectors.len() > 0 {
//                         ui.label("CeBrA: No Time Cut:");
//                         ui.horizontal(|ui| {
//                             ui.add(
//                                 egui::DragValue::new(
//                                     &mut cebra_config.detectors[0]
//                                         .pips_timecuts
//                                         .pips1000
//                                         .no_cut_range
//                                         .0,
//                                 )
//                                 .speed(1.0)
//                                 .prefix("("),
//                             );
//                             ui.add(
//                                 egui::DragValue::new(
//                                     &mut cebra_config.detectors[0]
//                                         .pips_timecuts
//                                         .pips1000
//                                         .no_cut_range
//                                         .1,
//                                 )
//                                 .speed(1.0)
//                                 .suffix(")"),
//                             );
//                         });
//                         ui.add(
//                             egui::DragValue::new(
//                                 &mut cebra_config.detectors[0].pips_timecuts.pips1000.no_cut_bins,
//                             )
//                             .speed(1),
//                         );
//                         ui.end_row();

//                         ui.label("CeBrA: Time Cut:");
//                         ui.horizontal(|ui| {
//                             ui.add(
//                                 egui::DragValue::new(
//                                     &mut cebra_config.detectors[0].pips_timecuts.pips1000.range.0,
//                                 )
//                                 .speed(1.0)
//                                 .prefix("("),
//                             );
//                             ui.add(
//                                 egui::DragValue::new(
//                                     &mut cebra_config.detectors[0].pips_timecuts.pips1000.range.1,
//                                 )
//                                 .speed(1.0)
//                                 .suffix(")"),
//                             );
//                         });
//                         ui.add(
//                             egui::DragValue::new(
//                                 &mut cebra_config.detectors[0].pips_timecuts.pips1000.bins,
//                             )
//                             .speed(1),
//                         );
//                         ui.end_row();
//                     }
//                 }
//             });

//         ui.separator();

//         self.pips1000.ui(ui, cebra_config, sps_config);
//         self.pips500.ui(ui, cebra_config, sps_config);
//         self.pips300.ui(ui, cebra_config, sps_config);
//         self.pips100.ui(ui, cebra_config, sps_config);
//     }

//     pub fn sync_common_values(&mut self) {
//         // Default range/bins
//         self.pips500.range = self.pips1000.range;
//         self.pips500.bins = self.pips1000.bins;

//         self.pips300.range = self.pips1000.range;
//         self.pips300.bins = self.pips1000.bins;

//         self.pips100.range = self.pips1000.range;
//         self.pips100.bins = self.pips1000.bins;

//         // Energy Calibration range/bins
//         self.pips500.energy_calibration.range = self.pips1000.energy_calibration.range;
//         self.pips500.energy_calibration.bins = self.pips1000.energy_calibration.bins;

//         self.pips300.energy_calibration.range = self.pips1000.energy_calibration.range;
//         self.pips300.energy_calibration.bins = self.pips1000.energy_calibration.bins;

//         self.pips100.energy_calibration.range = self.pips1000.energy_calibration.range;
//         self.pips100.energy_calibration.bins = self.pips1000.energy_calibration.bins;

//         // SE-SPS Time Cuts
//         self.pips500.sps_timecut.range = self.pips1000.sps_timecut.range;
//         self.pips500.sps_timecut.bins = self.pips1000.sps_timecut.bins;
//         self.pips500.sps_timecut.no_cut_range = self.pips1000.sps_timecut.no_cut_range;
//         self.pips500.sps_timecut.no_cut_bins = self.pips1000.sps_timecut.no_cut_bins;

//         self.pips300.sps_timecut.range = self.pips1000.sps_timecut.range;
//         self.pips300.sps_timecut.bins = self.pips1000.sps_timecut.bins;
//         self.pips300.sps_timecut.no_cut_range = self.pips1000.sps_timecut.no_cut_range;
//         self.pips300.sps_timecut.no_cut_bins = self.pips1000.sps_timecut.no_cut_bins;

//         self.pips100.sps_timecut.range = self.pips1000.sps_timecut.range;
//         self.pips100.sps_timecut.bins = self.pips1000.sps_timecut.bins;
//         self.pips100.sps_timecut.no_cut_range = self.pips1000.sps_timecut.no_cut_range;
//         self.pips100.sps_timecut.no_cut_bins = self.pips1000.sps_timecut.no_cut_bins;

//         // CeBrA Time Cuts
//         // self.
//     }

//     pub fn get_configs(
//         &self,
//         cebra_config: &mut CeBrAConfig,
//         sps_config: &mut SPSConfig,
//     ) -> Configs {
//         let mut configs = Configs::default();

//         // if self.pips1000.active {
//         //     configs.merge(self.pips1000.configs(cebra_config.clone(), sps_config));
//         // }

//         // if self.pips500.active {
//         //     configs.merge(self.pips500.configs(cebra_config.clone(), sps_config));
//         // }

//         // if self.pips300.active {
//         //     configs.merge(self.pips300.configs(cebra_config.clone(), sps_config));
//         // }

//         // if self.pips100.active {
//         //     configs.merge(self.pips100.configs(cebra_config.clone(), sps_config));
//         // }

//         configs
//     }
// }
