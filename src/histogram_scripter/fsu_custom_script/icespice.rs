use super::general::{Calibration, TimeCut};

use crate::histoer::configs::Configs;
use crate::histoer::cuts::{Cut, Cuts};

use super::cebra::CeBrAConfig;
use super::se_sps::SPSConfig;

/*************************** ICESPICE Custom Struct ***************************/
#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct PIPS {
    pub name: String, // Naming convention is either 1000, 500, 300, 100 for now
    pub sps_timecut: TimeCut,
    pub energy_calibration: Calibration,
    pub active: bool,
    pub range: (f64, f64),
    pub bins: usize,
}

impl Default for PIPS {
    fn default() -> Self {
        Self {
            name: "1000".to_string(),
            sps_timecut: TimeCut::default(),
            energy_calibration: Calibration {
                a: 0.0,
                b: 1.0,
                c: 0.0,
                active: false,
                range: (0.0, 2000.0),
                bins: 2000,
            },
            active: false,
            range: (0.0, 4096.0),
            bins: 512,
        }
    }
}

impl PIPS {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            sps_timecut: TimeCut::default(),
            energy_calibration: Calibration::default(),
            active: false,
            range: (0.0, 4096.0),
            bins: 512,
        }
    }

    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub fn configs(&self, cebra_config: CeBrAConfig, sps_config: SPSConfig, main_cuts: Option<Cuts>) -> Configs {
        let mut configs = Configs::default();
        let base_path = if main_cuts.is_none() { "No Cuts/ICESPICE" } else { "Cuts/ICESPICE" };

        let pips_energy_column = format!("PIPS{}Energy", self.name);
        let pips_energy_calibrated_column = format!("PIPS{}EnergyCalibrated", self.name);
        let pips_time_column = format!("PIPS{}Time", self.name);

        let pips_range = self.range;
        let pips_bins = self.bins;

        let det = self.name.clone();

        configs.hist1d(&format!("{base_path}/PIPS{det}/{pips_energy_column}"), &pips_energy_column, pips_range, pips_bins, &main_cuts);

        if self.energy_calibration.active {
            configs.columns.push(self.energy_calibration.new_column(&pips_energy_column, &pips_energy_calibrated_column));
            configs.hist1d(&format!("{base_path}/PIPS{det}/{pips_energy_calibrated_column}"), &pips_energy_calibrated_column, self.energy_calibration.range, self.energy_calibration.bins, &main_cuts);
            configs.hist1d(&format!("{base_path}/PIPS/PIPSEnergyCalibrated"), &pips_energy_calibrated_column, self.energy_calibration.range, self.energy_calibration.bins, &main_cuts);
        }

        if cebra_config.active {
            for cebr3 in cebra_config.detectors.iter() {
                if cebr3.active {

                    let cebra_det_number = cebr3.number;

                    let cebra_base_path = format!("{base_path}/PIPS{det}/Cebra{cebra_det_number}");

                    let cebr3_energy_column = format!("Cebra{}Energy", cebr3.number);
                    let cebr3_energy_calibrated_column = format!("Cebra{}EnergyCalibrated", cebr3.number);
                    let cebr3_time_column = format!("Cebra{}Time", cebr3.number);
                    let pips_rel_time_to_cebra_column = format!("PIPS{}TimeRelToCebra{}Time", self.name, cebr3.number);

                    let cebr3_range = (0.0, 4096.0);
                    let cebr3_bins = 4096;

                    // create the time difference column
                    configs.columns.push((format!("{cebr3_time_column} - {pips_time_column}"), pips_rel_time_to_cebra_column.clone()));

                    let valid_cebra_time_cut = Cut::new_1d(&format!("Valid PIPS{} Cebra{} Time Cut", self.name, cebr3.number), &format!("PIPS{}Energy > 0.0 && Cebra{}Energy > 0.0", self.name, cebr3.number));
                    configs.cuts.add_cut(valid_cebra_time_cut.clone());

                    let cebra_tcut: Option<Cuts> = if let Some(mut main_cuts) = main_cuts.clone() {
                        main_cuts.add_cut(valid_cebra_time_cut);
                        Some(main_cuts)
                    } else {
                        Some(Cuts::new(vec![valid_cebra_time_cut.clone()]))
                    };


                    configs.hist1d(&format!("{cebra_base_path}/{pips_rel_time_to_cebra_column}"), &pips_rel_time_to_cebra_column, cebr3.pips_timecuts.pips1000.no_cut_range, cebr3.pips_timecuts.pips1000.no_cut_bins, &cebra_tcut);
                    configs.hist2d(&format!("{}/PIPS{}Energy v Cebra{}Energy",base_path, self.name, cebr3.number), &cebr3_energy_column, &pips_energy_column, cebr3_range, pips_range, (cebr3_bins, pips_bins), &cebra_tcut);

                    // energy calibrated histograms
                    if self.energy_calibration.active {
                        configs.hist2d(&format!("{cebra_base_path}/Energy Calibrated/{pips_energy_calibrated_column} v {cebr3_energy_column}"), &cebr3_energy_column, &pips_energy_calibrated_column, cebr3_range, self.energy_calibration.range, (cebr3_bins, self.energy_calibration.bins), &main_cuts);
                    }
                    if cebr3.energy_calibration.active & self.energy_calibration.active {
                        configs.hist2d(&format!("{cebra_base_path}/Energy Calibrated/{pips_energy_calibrated_column} v {cebr3_energy_calibrated_column}"), &cebr3_energy_calibrated_column, &pips_energy_calibrated_column, cebr3.energy_calibration.range, self.energy_calibration.range, (cebr3.energy_calibration.bins, self.energy_calibration.bins), &main_cuts);
                    }
                    // if cebr3.energy_calibration.active {
                        // configs.hist2d(&format!("{}/Energy Calibrated/PIPS{}TimeRelToCebra{}Time v Cebra{}EnergyCalibrated", base_path, self.name, cebr3.number, cebr3.number), &format!("PIPS{}TimeRelToCebra{}Time", self.name, cebr3.number), &cebr3_energy_calibrated, cebr3.pips_timecuts.pips1000.no_cut_range, cebr3.energy_calibration.range, (cebr3.pips_timecuts.pips1000.no_cut_bins, cebr3.energy_calibration.bins), None);
                    // }

                    // check time cuts for the correct detector
                    // ONLY PIPS1000 is used for now for testing purposes
                    // if cebr3.pips_timecuts.pips1000.active {
                    //     let cebra_min = cebr3.pips_timecuts.pips1000.low;
                    //     let cebra_max = cebr3.pips_timecuts.pips1000.high;
                    //     let cebra_mean = cebr3.pips_timecuts.pips1000.mean;
                    //     let cebra_time_range = cebr3.pips_timecuts.pips1000.range;
                    //     let cebra_time_bins = cebr3.pips_timecuts.pips1000.bins;

                    //     // add column for the time cut to shift the time
                    //     configs.columns.push((format!("{cebr3_time_column} - {pips_time_column} - {cebra_mean}"), format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number)));

                    //     // add the time cut

                    //     let pips_cebra_tcut = Cut::new_1d(&format!("PIPS{}+Cebra{} Time Cut", self.name, cebr3.number), &format!("PIPS{}TimeRelToCebra{}Time >= {} && PIPS{}TimeRelToCebra{}Time <= {}", self.name, cebr3.number, min, self.name, cebr3.number, max));
                    //     configs.cuts.add_cut(pips_cebra_tcut.clone());
                    //     let tcut = Some(Cuts::new(vec![pips_cebra_tcut.clone(), valid_time_cut.clone()]));

                    //     configs.hist1d(&format!("{}/Time Cut/PIPS{}TimeRelToCebra{}TimeShifted", base_path, self.name, cebr3.number), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), time_range, time_bins, tcut.clone());
                    //     configs.hist1d(&format!("ICESPICE/PIPS{}/CeBrA/Time Cut - PIPS{}TimeRelToCeBrA", self.name, self.name), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), time_range, time_bins, tcut.clone());

                    //     configs.hist2d(&format!("{}/Time Cut/PIPS{}Energy v Cebra{}Energy", base_path, self.name, cebr3.number), &cebr3_energy, &energy, cebr3_range, pips_range, (cebr3_bins, pips_bins), tcut.clone());
                    //     configs.hist2d(&format!("{}/Time Cut/PIPS{}RelToCebra{}Shifted v Cebra{}Energy", base_path, self.name, cebr3.number, cebr3.number), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), &cebr3_energy, time_range, cebr3_range, (time_bins, cebr3_bins), tcut.clone());
                    //     // energy calibrated histograms
                    //     if self.energy_calibration.active & !cebr3.energy_calibration.active {
                    //         configs.hist2d(&format!("{}/Time Cut/Energy Calibrated/PIPS{}EnergyCalibrated v Cebra{}Energy", base_path, self.name, cebr3.number), &cebr3_energy, &energy_calibrated, cebr3_range, self.energy_calibration.range, (cebr3_bins, self.energy_calibration.bins), tcut.clone());
                    //     } else if cebr3.energy_calibration.active & self.energy_calibration.active {
                    //         configs.hist2d(&format!("{}/Time Cut/Energy Calibrated/PIPS{}EnergyCalibrated v Cebra{}EnergyCalibrated", base_path, self.name, cebr3.number), &cebr3_energy_calibrated, &energy_calibrated, cebr3.energy_calibration.range, self.energy_calibration.range, (cebr3.energy_calibration.bins, self.energy_calibration.bins), tcut.clone());
                    //         configs.hist2d(&format!("ICESPICE/PIPS{}/CeBrA/Time Cut- PIPS{}EnergyCalibrated v CeBrA", self.name, self.name), &cebr3_energy_calibrated, &energy_calibrated, cebr3.energy_calibration.range, self.energy_calibration.range, (cebr3.energy_calibration.bins, self.energy_calibration.bins), tcut.clone());
                    //     }
                    //     if cebr3.energy_calibration.active {
                    //         configs.hist2d(&format!("{}/Time Cut/Energy Calibrated/PIPS{}TimeRelToCebra{}TimeShifted v Cebra{}EnergyCalibrated", base_path, self.name, cebr3.number, cebr3.number), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), &cebr3_energy_calibrated, time_range, cebr3.energy_calibration.range, (time_bins, cebr3.energy_calibration.bins), tcut.clone());
                    //         configs.hist2d(&format!("ICESPICE/PIPS{}/CeBrA/Time Cut - PIPS{}TimeRelToCeBrA v CeBrA", self.name, self.name), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), &cebr3_energy_calibrated, time_range, cebr3.energy_calibration.range, (time_bins, cebr3.energy_calibration.bins), tcut.clone());
                    //     }
                    // }
                }
            }

        }

        if sps_config.active {
            let pips_rel_time_to_sps_column = format!("PIPS{}RelTime", self.name); // PIPS{}Time - ScintLeftTime -> Column is made in the eventbuilder with the anode condition too
            let sps_tcut_min = self.sps_timecut.low;
            let sps_tcut_max = self.sps_timecut.high;
            let sps_tcut_mean = self.sps_timecut.mean;
            let sps_tcut_time_range = self.sps_timecut.range;
            let sps_tcut_time_bins = self.sps_timecut.bins;
            let no_sps_tcut_time_range = self.sps_timecut.no_cut_range;
            let no_sps_tcut_time_bins = self.sps_timecut.no_cut_bins;

            configs.hist1d(&format!("{base_path}/PIPS{det}/SPS/{pips_rel_time_to_sps_column}"), &pips_rel_time_to_sps_column, no_sps_tcut_time_range, no_sps_tcut_time_bins, &main_cuts);
            configs.hist2d(&format!("{base_path}/PIPS{det}/SPS/{pips_rel_time_to_sps_column} v Xavg"), &pips_rel_time_to_sps_column, &format!("Xavg"), no_sps_tcut_time_range, (-300.0, 300.0), (no_sps_tcut_time_bins, 600), &main_cuts);
            configs.hist2d(&format!("{base_path}/PIPS{det}/SPS/{pips_energy_column} v Xavg"), &format!("Xavg"), &pips_energy_column,(-300.0, 300.0), pips_range, (pips_bins, 600), &main_cuts);

            if sps_config.xavg.active {
                configs.hist2d(&format!("{base_path}/PIPS{det}/SPS/{pips_rel_time_to_sps_column} v XavgEnergyCalibrated"), &pips_rel_time_to_sps_column, &format!("XavgEnergyCalibrated"), no_sps_tcut_time_range, sps_config.xavg.range, (no_sps_tcut_time_bins, sps_config.xavg.bins), &main_cuts);
                configs.hist2d(&format!("{base_path}/PIPS{det}/SPS/{pips_energy_column} v XavgEnergyCalibrated"), &format!("XavgEnergyCalibrated"), &pips_energy_column, sps_config.xavg.range, pips_range, (pips_bins, sps_config.xavg.bins), &main_cuts);
            }

            if self.sps_timecut.active {

                // add column for the time cut to shift the time
                let pips_rel_time_to_sps_column_shifted = format!("{pips_rel_time_to_sps_column}Shifted");

                configs.columns.push((format!("{pips_rel_time_to_sps_column} - {sps_tcut_mean}"), pips_rel_time_to_sps_column_shifted.clone()));

                // add the time cut
                let pips_sps_tcut = Cut::new_1d(&format!("PIPS{} Time Cut", self.name), &format!("{pips_rel_time_to_sps_column} >= {sps_tcut_min} && {pips_rel_time_to_sps_column} <= {sps_tcut_max}"));
                configs.cuts.add_cut(pips_sps_tcut.clone());

                let sps_tcut: Option<Cuts> = if let Some(mut main_cuts) = main_cuts.clone() {
                    main_cuts.add_cut(pips_sps_tcut);
                    Some(main_cuts)
                } else {
                    Some(Cuts::new(vec![pips_sps_tcut.clone()]))
                };

                // configs.hist1d(&format!("{base_path}/PIPS{det}/SPS/Time Cut/{pips_rel_time_to_sps_column_shifted}"), &pips_rel_time_to_sps_column_shifted, sps_tcut_time_range, sps_tcut_time_bins, &sps_tcut);
                configs.hist1d(&format!("{base_path}/PIPS{det}/SPS/Time Cut/{pips_energy_column}"), &pips_energy_column, pips_range, pips_bins, &sps_tcut);
                configs.hist2d(&format!("{base_path}/PIPS{det}/SPS/Time Cut/{pips_energy_column} v Xavg"), &format!("Xavg"), &pips_energy_column, (-300.0, 300.0), pips_range, (600, pips_bins), &sps_tcut);
                configs.hist2d(&format!("{base_path}/PIPS{det}/SPS/Time Cut/{pips_rel_time_to_sps_column_shifted} v Xavg"), &format!("Xavg"), &pips_rel_time_to_sps_column_shifted, (-300.0, 300.0), sps_tcut_time_range, (600, sps_tcut_time_bins), &sps_tcut);
            }
        }

        configs
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        cebra_config: &mut CeBrAConfig,
        sps_config: &mut SPSConfig,
    ) {
        if self.active {
            // Collapsing header for PIPS detector
            ui.collapsing(format!("PIPS{}", self.name), |ui| {
                // Energy Calibration UI
                ui.horizontal(|ui| {
                    ui.label("Energy Calibration: ");
                    self.energy_calibration.ui(ui, false);
                });

                // SPS Time Cut UI
                if sps_config.active {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("SE-SPS Time Cut: ");
                        ui.add_enabled(
                            self.sps_timecut.active,
                            egui::DragValue::new(&mut self.sps_timecut.mean)
                                .speed(1)
                                .prefix("Mean: "),
                        );
                        ui.add_enabled(
                            self.sps_timecut.active,
                            egui::DragValue::new(&mut self.sps_timecut.low)
                                .speed(1)
                                .prefix("Low: "),
                        );
                        ui.add_enabled(
                            self.sps_timecut.active,
                            egui::DragValue::new(&mut self.sps_timecut.high)
                                .speed(1)
                                .prefix("High: "),
                        );
                        ui.checkbox(&mut self.sps_timecut.active, "Active");
                    });
                    ui.separator();
                }

                // CeBrA Detector Configurations
                if cebra_config.active {
                    for cebr3 in &mut cebra_config.detectors {
                        if cebr3.active {
                            ui.horizontal(|ui| {
                                ui.label(format!("Cebra{} Time Cut: ", cebr3.number));

                                match self.name.as_str() {
                                    "1000" => {
                                        ui.add_enabled(
                                            cebr3.pips_timecuts.pips1000.active,
                                            egui::DragValue::new(
                                                &mut cebr3.pips_timecuts.pips1000.mean,
                                            )
                                            .speed(1)
                                            .prefix("Mean: "),
                                        );
                                        ui.add_enabled(
                                            cebr3.pips_timecuts.pips1000.active,
                                            egui::DragValue::new(
                                                &mut cebr3.pips_timecuts.pips1000.low,
                                            )
                                            .speed(1)
                                            .prefix("Low: "),
                                        );
                                        ui.add_enabled(
                                            cebr3.pips_timecuts.pips1000.active,
                                            egui::DragValue::new(
                                                &mut cebr3.pips_timecuts.pips1000.high,
                                            )
                                            .speed(1)
                                            .prefix("High: "),
                                        );
                                        ui.checkbox(
                                            &mut cebr3.pips_timecuts.pips1000.active,
                                            "Active",
                                        );
                                    }
                                    "500" => {
                                        ui.add_enabled(
                                            cebr3.pips_timecuts.pips500.active,
                                            egui::DragValue::new(
                                                &mut cebr3.pips_timecuts.pips500.mean,
                                            )
                                            .speed(1)
                                            .prefix("Mean: "),
                                        );
                                        ui.add_enabled(
                                            cebr3.pips_timecuts.pips500.active,
                                            egui::DragValue::new(
                                                &mut cebr3.pips_timecuts.pips500.low,
                                            )
                                            .speed(1)
                                            .prefix("Low: "),
                                        );
                                        ui.add_enabled(
                                            cebr3.pips_timecuts.pips500.active,
                                            egui::DragValue::new(
                                                &mut cebr3.pips_timecuts.pips500.high,
                                            )
                                            .speed(1)
                                            .prefix("High: "),
                                        );
                                        ui.checkbox(
                                            &mut cebr3.pips_timecuts.pips500.active,
                                            "Active",
                                        );
                                    }
                                    "300" => {
                                        ui.add_enabled(
                                            cebr3.pips_timecuts.pips300.active,
                                            egui::DragValue::new(
                                                &mut cebr3.pips_timecuts.pips300.mean,
                                            )
                                            .speed(1)
                                            .prefix("Mean: "),
                                        );
                                        ui.add_enabled(
                                            cebr3.pips_timecuts.pips300.active,
                                            egui::DragValue::new(
                                                &mut cebr3.pips_timecuts.pips300.low,
                                            )
                                            .speed(1)
                                            .prefix("Low: "),
                                        );
                                        ui.add_enabled(
                                            cebr3.pips_timecuts.pips300.active,
                                            egui::DragValue::new(
                                                &mut cebr3.pips_timecuts.pips300.high,
                                            )
                                            .speed(1)
                                            .prefix("High: "),
                                        );
                                        ui.checkbox(
                                            &mut cebr3.pips_timecuts.pips300.active,
                                            "Active",
                                        );
                                    }
                                    "100" => {
                                        ui.add_enabled(
                                            cebr3.pips_timecuts.pips100.active,
                                            egui::DragValue::new(
                                                &mut cebr3.pips_timecuts.pips100.mean,
                                            )
                                            .speed(1)
                                            .prefix("Mean: "),
                                        );
                                        ui.add_enabled(
                                            cebr3.pips_timecuts.pips100.active,
                                            egui::DragValue::new(
                                                &mut cebr3.pips_timecuts.pips100.low,
                                            )
                                            .speed(1)
                                            .prefix("Low: "),
                                        );
                                        ui.add_enabled(
                                            cebr3.pips_timecuts.pips100.active,
                                            egui::DragValue::new(
                                                &mut cebr3.pips_timecuts.pips100.high,
                                            )
                                            .speed(1)
                                            .prefix("High: "),
                                        );
                                        ui.checkbox(
                                            &mut cebr3.pips_timecuts.pips100.active,
                                            "Active",
                                        );
                                    }
                                    _ => {}
                                }
                            });
                        }
                    }
                }
            });
        }
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct ICESPICEConfig {
    pub pips1000: PIPS,
    pub pips500: PIPS,
    pub pips300: PIPS,
    pub pips100: PIPS,
    pub active: bool,
}

impl Default for ICESPICEConfig {
    fn default() -> Self {
        Self {
            pips1000: PIPS::new("1000"),
            pips500: PIPS::new("500"),
            pips300: PIPS::new("300"),
            pips100: PIPS::new("100"),
            active: false,
        }
    }
}

impl ICESPICEConfig {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        cebra_config: &mut CeBrAConfig,
        sps_config: &mut SPSConfig,
    ) {
        if !self.active {
            return;
        }

        self.sync_common_values();

        ui.separator();

        ui.label("PIPS Detectors");

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.pips1000.active, "PIPS1000");
            ui.checkbox(&mut self.pips500.active, "PIPS500");
            ui.checkbox(&mut self.pips300.active, "PIPS300");
            ui.checkbox(&mut self.pips100.active, "PIPS100");
        });

        egui::Grid::new("icespice_pips")
            .striped(true)
            .show(ui, |ui| {
                ui.label("Histogram Settings");
                ui.label("Range");
                ui.label("Bins");

                ui.end_row();

                // Default
                ui.label("Default:");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.pips1000.range.0)
                            .speed(1.0)
                            .prefix("("),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.pips1000.range.1)
                            .speed(1.0)
                            .suffix(")"),
                    );
                });
                ui.add(egui::DragValue::new(&mut self.pips1000.bins).speed(1));

                ui.end_row();

                // Energy Calibrated
                ui.label("Energy Calibrated");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.pips1000.energy_calibration.range.0)
                            .speed(1.0)
                            .prefix("("),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.pips1000.energy_calibration.range.1)
                            .speed(1.0)
                            .suffix(")"),
                    );
                });
                ui.add(egui::DragValue::new(&mut self.pips1000.energy_calibration.bins).speed(1));
                ui.end_row();

                // SPS Time Cut Values
                if sps_config.active {
                    ui.label("SE-SPS: No Time Cut");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.pips1000.sps_timecut.no_cut_range.0)
                                .speed(1.0)
                                .prefix("("),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.pips1000.sps_timecut.no_cut_range.1)
                                .speed(1.0)
                                .suffix(")"),
                        );
                    });
                    ui.add(
                        egui::DragValue::new(&mut self.pips1000.sps_timecut.no_cut_bins).speed(1),
                    );
                    ui.end_row();

                    ui.label("SE-SPS: Time Cut:");

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.pips1000.sps_timecut.range.0)
                                .speed(1.0)
                                .prefix("("),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.pips1000.sps_timecut.range.1)
                                .speed(1.0)
                                .suffix(")"),
                        );
                    });
                    ui.add(egui::DragValue::new(&mut self.pips1000.sps_timecut.bins).speed(1));
                    ui.end_row();
                }

                // CeBrA Time Cut Values
                if cebra_config.active && !cebra_config.detectors.is_empty() {
                    ui.label("CeBrA: No Time Cut:");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(
                                &mut cebra_config.detectors[0]
                                    .pips_timecuts
                                    .pips1000
                                    .no_cut_range
                                    .0,
                            )
                            .speed(1.0)
                            .prefix("("),
                        );
                        ui.add(
                            egui::DragValue::new(
                                &mut cebra_config.detectors[0]
                                    .pips_timecuts
                                    .pips1000
                                    .no_cut_range
                                    .1,
                            )
                            .speed(1.0)
                            .suffix(")"),
                        );
                    });
                    ui.add(
                        egui::DragValue::new(
                            &mut cebra_config.detectors[0].pips_timecuts.pips1000.no_cut_bins,
                        )
                        .speed(1),
                    );
                    ui.end_row();

                    ui.label("CeBrA: Time Cut:");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(
                                &mut cebra_config.detectors[0].pips_timecuts.pips1000.range.0,
                            )
                            .speed(1.0)
                            .prefix("("),
                        );
                        ui.add(
                            egui::DragValue::new(
                                &mut cebra_config.detectors[0].pips_timecuts.pips1000.range.1,
                            )
                            .speed(1.0)
                            .suffix(")"),
                        );
                    });
                    ui.add(
                        egui::DragValue::new(
                            &mut cebra_config.detectors[0].pips_timecuts.pips1000.bins,
                        )
                        .speed(1),
                    );
                    ui.end_row();
                }
            });

        ui.separator();

        self.pips1000.ui(ui, cebra_config, sps_config);
        self.pips500.ui(ui, cebra_config, sps_config);
        self.pips300.ui(ui, cebra_config, sps_config);
        self.pips100.ui(ui, cebra_config, sps_config);
    }

    pub fn sync_common_values(&mut self) {
        // Default range/bins
        self.pips500.range = self.pips1000.range;
        self.pips500.bins = self.pips1000.bins;

        self.pips300.range = self.pips1000.range;
        self.pips300.bins = self.pips1000.bins;

        self.pips100.range = self.pips1000.range;
        self.pips100.bins = self.pips1000.bins;

        // Energy Calibration range/bins
        self.pips500.energy_calibration.range = self.pips1000.energy_calibration.range;
        self.pips500.energy_calibration.bins = self.pips1000.energy_calibration.bins;

        self.pips300.energy_calibration.range = self.pips1000.energy_calibration.range;
        self.pips300.energy_calibration.bins = self.pips1000.energy_calibration.bins;

        self.pips100.energy_calibration.range = self.pips1000.energy_calibration.range;
        self.pips100.energy_calibration.bins = self.pips1000.energy_calibration.bins;

        // SE-SPS Time Cuts
        self.pips500.sps_timecut.range = self.pips1000.sps_timecut.range;
        self.pips500.sps_timecut.bins = self.pips1000.sps_timecut.bins;
        self.pips500.sps_timecut.no_cut_range = self.pips1000.sps_timecut.no_cut_range;
        self.pips500.sps_timecut.no_cut_bins = self.pips1000.sps_timecut.no_cut_bins;

        self.pips300.sps_timecut.range = self.pips1000.sps_timecut.range;
        self.pips300.sps_timecut.bins = self.pips1000.sps_timecut.bins;
        self.pips300.sps_timecut.no_cut_range = self.pips1000.sps_timecut.no_cut_range;
        self.pips300.sps_timecut.no_cut_bins = self.pips1000.sps_timecut.no_cut_bins;

        self.pips100.sps_timecut.range = self.pips1000.sps_timecut.range;
        self.pips100.sps_timecut.bins = self.pips1000.sps_timecut.bins;
        self.pips100.sps_timecut.no_cut_range = self.pips1000.sps_timecut.no_cut_range;
        self.pips100.sps_timecut.no_cut_bins = self.pips1000.sps_timecut.no_cut_bins;

        // CeBrA Time Cuts
        // self.
    }

    pub fn icespice_configs(
        &self,
        cebra_config: &CeBrAConfig,
        sps_config: &SPSConfig,
        main_cuts: Option<Cuts>,
    ) -> Configs {
        let mut configs = Configs::default();

        if self.pips1000.active {
            configs.merge(self.pips1000.configs(
                cebra_config.clone(),
                sps_config.clone(),
                main_cuts.clone(),
            ));
        }

        if self.pips500.active {
            configs.merge(self.pips500.configs(
                cebra_config.clone(),
                sps_config.clone(),
                main_cuts.clone(),
            ));
        }

        if self.pips300.active {
            configs.merge(self.pips300.configs(
                cebra_config.clone(),
                sps_config.clone(),
                main_cuts.clone(),
            ));
        }

        if self.pips100.active {
            configs.merge(self.pips100.configs(
                cebra_config.clone(),
                sps_config.clone(),
                main_cuts.clone(),
            ));
        }

        configs
    }
}
