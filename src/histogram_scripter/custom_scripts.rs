use crate::histoer::{
    configs::{Config, Configs},
    cuts::{Cut, Cuts},
};
use egui_extras::{Column, TableBuilder};
use std::f64::consts::PI;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct CustomConfigs {
    pub sps: SPSConfig,
    pub cebra: CeBrAConfig,
    pub icespice: ICESPICEConfig,
}

impl Default for CustomConfigs {
    fn default() -> Self {
        Self {
            sps: SPSConfig::new(),
            cebra: CeBrAConfig::default(),
            icespice: ICESPICEConfig::default(),
        }
    }
}

impl CustomConfigs {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Custom: ");
            ui.checkbox(&mut self.sps.active, "SPS");
            ui.checkbox(&mut self.cebra.active, "CeBrA");
            ui.checkbox(&mut self.icespice.active, "ICESPICE");
        });

        ui.horizontal(|ui| {
            ui.label("Previous Experiments: ");
            if ui.button("52Cr(d,p)53Cr").clicked() {
                self.cr52dp_experiment();
            }
        });

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
            // get the updated configs from sps
            let sps_configs = self.sps.update_configs_with_cuts();
            configs.merge(sps_configs.clone()); // Ensure `merge` handles in-place modifications
        }

        if self.cebra.active {
            let cebra_configs = self.cebra.get_configs(&self.sps);
            configs.merge(cebra_configs.clone()); // Ensure `merge` handles in-place modifications
        }

        if self.icespice.active {
            let icespice_configs = self.icespice.get_configs(&mut self.cebra, &mut self.sps);
            configs.merge(icespice_configs.clone()); // Ensure `merge` handles in-place modifications
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
            cuts: Cuts::default(),
        };

        self.cebra.active = true;
        self.cebra.cr52dp_experiment();
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Calibration {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub bins: usize,
    pub range: (f64, f64),
    pub active: bool,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            a: 0.0,
            b: 1.0,
            c: 0.0,
            bins: 512,
            range: (0.0, 4096.0),
            active: false,
        }
    }
}

impl Calibration {
    pub fn ui(&mut self, ui: &mut egui::Ui, bins: bool) {
        ui.horizontal(|ui| {
            if self.active {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.a).speed(0.01).prefix("a: "));
                    ui.add(egui::DragValue::new(&mut self.b).speed(0.01).prefix("b: "));
                    ui.add(egui::DragValue::new(&mut self.c).speed(0.01).prefix("c: "));
                });

                if bins {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.bins)
                                .speed(1)
                                .prefix("Bins: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.range.0)
                                .speed(1)
                                .prefix("Range: (")
                                .suffix(", "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.range.1)
                                .speed(1)
                                .suffix(") [keV]"),
                        );
                    });
                }

                // ui.label(format!(
                //     "keV/bin: {:.2}",
                //     (self.range.1 - self.range.0) / self.bins as f64
                // ));
            }
            ui.checkbox(&mut self.active, "Active");
        });
    }

    pub fn new_column(&self, column: &str, alias: &str) -> (String, String) {
        (
            format!(
                "({})*{}**2 + ({})*{} + ({})",
                self.a, column, self.b, column, self.c
            ),
            alias.to_string(),
        )
    }
}

/*************************** CeBrA Custom Struct ***************************/

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct TimeCut {
    pub mean: f64,
    pub low: f64,
    pub high: f64,
    pub bins: usize,
    pub range: (f64, f64),
    pub no_cut_range: (f64, f64),
    pub no_cut_bins: usize,
    pub active: bool,
}

impl Default for TimeCut {
    fn default() -> Self {
        Self {
            mean: 0.0,
            low: -3000.0,
            high: 3000.0,
            bins: 6400,
            range: (-3200.0, 3200.0),
            no_cut_range: (-3200.0, 3200.0),
            no_cut_bins: 6400,
            active: false,
        }
    }
}

impl TimeCut {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("timecut_grid")
            .striped(true)
            .show(ui, |ui| {
                ui.checkbox(&mut self.active, "Active");
                ui.label("Range");
                ui.label("Bins");

                if self.active {
                    ui.label("Mean");
                    ui.label("Low");
                    ui.label("High");
                }

                ui.end_row();

                if self.active {
                    ui.label("Cut");
                    ui.horizontal(|ui| {
                        ui.add_enabled(
                            self.active,
                            egui::DragValue::new(&mut self.range.0)
                                .speed(1)
                                .prefix("(")
                                .suffix(", "),
                        );
                        ui.add_enabled(
                            self.active,
                            egui::DragValue::new(&mut self.range.1).speed(1).suffix(")"),
                        );
                    });

                    ui.add_enabled(self.active, egui::DragValue::new(&mut self.bins).speed(1));
                    ui.add_enabled(self.active, egui::DragValue::new(&mut self.mean).speed(1));
                    ui.add_enabled(self.active, egui::DragValue::new(&mut self.low).speed(1));
                    ui.add_enabled(self.active, egui::DragValue::new(&mut self.high).speed(1));

                    ui.end_row();
                }

                ui.label("No Cut");

                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.no_cut_range.0)
                            .speed(1)
                            .prefix("(")
                            .suffix(", "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.no_cut_range.1)
                            .speed(1)
                            .suffix(")"),
                    );
                });
                ui.add(egui::DragValue::new(&mut self.no_cut_bins).speed(1));
                ui.end_row();
            });
    }
}
#[derive(Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct PIPSTimeCuts {
    pub pips1000: TimeCut,
    pub pips500: TimeCut,
    pub pips300: TimeCut,
    pub pips100: TimeCut,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct GammaGates {
    pub name: String,
    pub low: f64,
    pub high: f64,
}

impl GammaGates {
    pub fn create_1d_cut(&self, detector: usize) -> Cut {
        Cut::new_1d(
            &self.name,
            &format!(
                "Cebra{}EnergyCalibrated >= {} && Cebra{}EnergyCalibrated <= {}",
                detector, self.low, detector, self.high
            ),
        )
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Cebr3 {
    pub number: usize,
    pub timecut: TimeCut,
    pub gainmatch: Calibration,
    pub energy_calibration: Calibration,
    pub pips_timecuts: PIPSTimeCuts,
    pub gamma_gates: Vec<GammaGates>,
    pub active: bool,
}

impl Cebr3 {
    pub fn new(number: usize) -> Self {
        Self {
            number,
            timecut: TimeCut::default(),
            gainmatch: Calibration::default(),
            energy_calibration: Calibration::default(),
            pips_timecuts: PIPSTimeCuts::default(),
            gamma_gates: vec![],
            active: false,
        }
    }

    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub fn config(&self, sps_config: SPSConfig) -> Configs {

        if !self.active {
            return Configs::default();
        }

        let mut configs = Configs::default();

        let range = (0.0, 4096.0);
        let bins = 4096;

        configs.hist1d(&format!("CeBrA/Cebra{}/Cebra{}Energy", self.number, self.number), &format!("Cebra{}Energy", self.number), range, bins, None);


        if self.gainmatch.active {
            configs.columns.push(self.gainmatch.new_column(&format!("Cebra{}Energy", self.number),&format!("Cebra{}GainMatched", self.number)));
            configs.hist1d(&format!("CeBrA/Cebra{}/Cebra{} Gain Matched", self.number, self.number), &format!("Cebra{}GainMatched", self.number), self.gainmatch.range, self.gainmatch.bins, None); 
            configs.hist1d(&"CeBrA/CeBrA/Gain Matched", &format!("Cebra{}GainMatched", self.number),  self.gainmatch.range, self.gainmatch.bins, None); 
        }

        if self.energy_calibration.active {
            if self.gainmatch.active {
                configs.columns.push(self.energy_calibration.new_column(&format!("Cebra{}GainMatched", self.number),&format!("Cebra{}EnergyCalibrated", self.number)));
            } else {
                configs.columns.push(self.energy_calibration.new_column(&format!("Cebra{}Energy", self.number),&format!("Cebra{}EnergyCalibrated", self.number)));
            }
            configs.hist1d(&format!("CeBrA/Cebra{}/Cebra{} Energy Calibrated", self.number, self.number), &format!("Cebra{}EnergyCalibrated", self.number), self.energy_calibration.range, self.energy_calibration.bins, None);
            configs.hist1d(&"CeBrA/CeBrA/Energy Calibrated", &format!("Cebra{}EnergyCalibrated", self.number), self.energy_calibration.range, self.energy_calibration.bins, None);

            for gate in &self.gamma_gates {
                // needs further devolopment
                let cut = gate.create_1d_cut(self.number);
                configs.cuts.add_cut(cut.clone());
                // configs.hist1d(&format!("CeBrA/Cebra{}/Cebra{} Energy Calibrated {}", self.number, self.number, gate.name), &format!("Cebra{}EnergyCalibrated", self.number), self.energy_calibration.range, self.energy_calibration.bins, Some(Cuts::new(vec![cut])));
            }
        }

        if sps_config.active {
            configs.hist1d(&format!("CeBrA/Cebra{}/Cebra{}RelTime", self.number, self.number), &format!("Cebra{}RelTime", self.number), self.timecut.range, self.timecut.bins, None);
            configs.hist2d(&format!("CeBrA/Cebra{}/Cebra{}Energy v Xavg", self.number, self.number), &format!("Xavg"), &format!("Cebra{}Energy", self.number), (-300.0, 300.0), (0.0, 4096.0), (600, 512), None);
            configs.hist2d(&format!("CeBrA/Cebra{}/Cebra{}RelTime v Xavg", self.number, self.number), &format!("Xavg"), &format!("Cebra{}RelTime", self.number), (-300.0, 300.0), self.timecut.range, (600, self.timecut.bins), None);
            configs.hist2d(&format!("CeBrA/Cebra{}/Theta v Cebra{}RelTime", self.number, self.number), &format!("Cebra{}RelTime", self.number), "Theta", self.timecut.range, (0.0, PI/2.0), (self.timecut.bins, 300), None);

            if self.timecut.active {
                // columns with 2 minus do not work
                configs.columns.push((format!("Cebra{}RelTime - {}", self.number, self.timecut.mean), format!("Cebra{}RelTimeShifted", self.number)));
                configs.hist1d(&format!("CeBrA/Cebra{}/Cebra{}RelTimeShifted", self.number, self.number), &format!("Cebra{}RelTimeShifted", self.number), self.timecut.range, self.timecut.bins, None);
                configs.hist1d(&format!("CeBrA/CeBrA/CeBrARelTimeShifted"), &format!("Cebra{}RelTimeShifted", self.number), self.timecut.range, self.timecut.bins, None);


                let cebra_time_cut = Cut::new_1d(&format!("Cebra{} Time Cut", self.number), &format!("Cebra{}RelTime >= {} && Cebra{}RelTime <= {}", self.number, self.timecut.low, self.number, self.timecut.high));
                configs.cuts.add_cut(cebra_time_cut.clone());
                let tcut = Some(Cuts::new(vec![cebra_time_cut.clone()]));

                configs.hist1d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{}RelTime", self.number, self.number), &format!("Cebra{}RelTime", self.number), self.timecut.range, self.timecut.bins, tcut.clone());
                configs.hist1d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{}RelTimeShifted", self.number, self.number), &format!("Cebra{}RelTimeShifted", self.number), (-50.0, 50.0), 100, tcut.clone());

                configs.hist1d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{}Energy", self.number, self.number), &format!("Cebra{}Energy", self.number), range, bins, tcut.clone());
                configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{}Energy v Xavg", self.number, self.number), &format!("Xavg"), &format!("Cebra{}Energy", self.number), (-300.0, 300.0), (0.0, 4096.0), (600, 512), tcut.clone());
                configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{}Energy v X1", self.number, self.number), &format!("X1"), &format!("Cebra{}Energy", self.number), (-300.0, 300.0), (0.0, 4096.0), (600, 512), tcut.clone());

                configs.hist1d(&format!("CeBrA/Cebra{}/Time Cut/Xavg", self.number), &format!("Xavg"), (-300.0, 300.0), 600, tcut.clone());

                if self.gainmatch.active {
                    configs.hist1d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} Gain Matched", self.number, self.number), &format!("Cebra{}GainMatched", self.number), self.gainmatch.range, self.gainmatch.bins, tcut.clone());
                    configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} Gain Matched v Xavg", self.number, self.number), &format!("Xavg"), &format!("Cebra{}GainMatched", self.number), (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), tcut.clone());
                    configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} Gain Matched v X1", self.number, self.number), &format!("X1"), &format!("Cebra{}GainMatched", self.number), (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), tcut.clone());

                    configs.hist1d(&format!("CeBrA/CeBrA/Time Cut/CeBrA Gain Matched"), &format!("Cebra{}GainMatched", self.number), self.gainmatch.range, self.gainmatch.bins, tcut.clone());
                    configs.hist2d(&format!("CeBrA/CeBrA/Time Cut/CeBrA Gain Matched v Xavg"), &format!("Xavg"), &format!("Cebra{}Energy", self.number), (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), tcut.clone());
                    configs.hist2d(&format!("CeBrA/CeBrA/Time Cut/CeBrA Gain Matched v X1"), &format!("X1"), &format!("Cebra{}Energy", self.number), (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), tcut.clone());   

                    if sps_config.xavg.active {
                        configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} v Xavg- Gain Matched", self.number, self.number), &format!("XavgEnergyCalibrated"), &format!("Cebra{}GainMatched", self.number), sps_config.xavg.range, self.gainmatch.range, (sps_config.xavg.bins, self.gainmatch.bins), tcut.clone());
                        configs.hist2d(&format!("CeBrA/CeBrA/Time Cut/CeBrA v Xavg- Gain Matched"), &format!("XavgEnergyCalibrated"), &format!("Cebra{}GainMatched", self.number), sps_config.xavg.range, self.gainmatch.range, (sps_config.xavg.bins, self.gainmatch.bins), tcut.clone());
                    }
                }
                if self.energy_calibration.active {
                    configs.hist1d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} Energy Calibrated", self.number, self.number), &format!("Cebra{}EnergyCalibrated", self.number), self.energy_calibration.range, self.energy_calibration.bins, tcut.clone());
                    configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} Energy Calibrated v Xavg", self.number, self.number), &format!("Xavg"), &format!("Cebra{}EnergyCalibrated", self.number), (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), tcut.clone());
                    configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} Energy Calibrated v X1", self.number, self.number), &format!("X1"), &format!("Cebra{}EnergyCalibrated", self.number), (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), tcut.clone());

                    if sps_config.xavg.active {
                        configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} v Xavg- Energy Calibrated", self.number, self.number), &format!("XavgEnergyCalibrated"), &format!("Cebra{}EnergyCalibrated", self.number), sps_config.xavg.range, self.energy_calibration.range, (sps_config.xavg.bins, self.energy_calibration.bins), tcut.clone());
                        configs.hist2d(&format!("CeBrA/CeBrA/Time Cut/CeBrA v Xavg- Energy Calibrated"), &format!("XavgEnergyCalibrated"), &format!("Cebra{}EnergyCalibrated", self.number), sps_config.xavg.range, self.energy_calibration.range, (sps_config.xavg.bins, self.energy_calibration.bins), tcut.clone());
                    }

                    configs.hist1d(&format!("CeBrA/CeBrA/Time Cut/CeBrA Energy Calibrated"), &format!("Cebra{}EnergyCalibrated", self.number), self.energy_calibration.range, self.energy_calibration.bins, tcut.clone());
                    configs.hist2d(&format!("CeBrA/CeBrA/Time Cut/CeBrA Energy Calibrated v Xavg"), &format!("Xavg"), &format!("Cebra{}EnergyCalibrated", self.number), (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), tcut.clone());
                    configs.hist2d(&format!("CeBrA/CeBrA/Time Cut/CeBrA Energy Calibrated v X1"), &format!("X1"), &format!("Cebra{}EnergyCalibrated", self.number), (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), tcut.clone());
                }
            }
        }

        configs
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct CeBrAConfig {
    pub active: bool,
    pub detectors: Vec<Cebr3>,
}

impl Default for CeBrAConfig {
    fn default() -> Self {
        Self {
            active: false,
            // 9 detectors
            detectors: (0..9).map(Cebr3::new).collect(),
        }
    }
}

impl CeBrAConfig {
    pub fn time_cut_ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("SE-SPS Time Cuts");

            ui.separator();

            ui.add(
                egui::DragValue::new(&mut self.detectors[0].timecut.range.0)
                    .speed(1.0)
                    .prefix("Range: ("),
            );
            ui.add(
                egui::DragValue::new(&mut self.detectors[0].timecut.range.1)
                    .speed(1.0)
                    .suffix(")"),
            );

            ui.add(
                egui::DragValue::new(&mut self.detectors[0].timecut.bins)
                    .speed(1)
                    .prefix("Bins: "),
            );
        });

        //sync the time cut range and bins for all detectors
        let timecut_range = self.detectors[0].timecut.range;
        let timecut_bins = self.detectors[0].timecut.bins;

        for detector in &mut self.detectors[1..] {
            detector.timecut.range = timecut_range;
            detector.timecut.bins = timecut_bins;
        }

        // Create the table
        TableBuilder::new(ui)
            .id_salt("cebra_timecuts") // Unique identifier for the table
            .column(Column::auto()) // Detector Number
            .column(Column::auto()) // Mean
            .column(Column::auto()) // Low
            .column(Column::auto()) // High
            .column(Column::remainder()) // Active
            .striped(true) // Optional for better readability
            .vscroll(false) // Disable vertical scrolling for compact tables
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("Detector");
                });
                header.col(|ui| {
                    ui.label("Mean (ns)");
                });
                header.col(|ui| {
                    ui.label("Low (ns)");
                });
                header.col(|ui| {
                    ui.label("High (ns)");
                });
                header.col(|ui| {
                    ui.label("Active");
                });
            })
            .body(|mut body| {
                for detector in &mut self.detectors {
                    body.row(18.0, |mut row| {
                        // Detector Number
                        row.col(|ui| {
                            ui.label(format!("#{}", detector.number));
                        });

                        // Mean
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.timecut.active,
                                egui::DragValue::new(&mut detector.timecut.mean).speed(0.01),
                            );
                        });

                        // Low
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.timecut.active,
                                egui::DragValue::new(&mut detector.timecut.low).speed(0.01),
                            );
                        });

                        // High
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.timecut.active,
                                egui::DragValue::new(&mut detector.timecut.high).speed(0.01),
                            );
                        });

                        // Active
                        row.col(|ui| {
                            ui.checkbox(&mut detector.timecut.active, "");
                        });
                    });
                }
            });
    }

    pub fn gain_matching_ui(&mut self, ui: &mut egui::Ui) {
        // Temporarily store the range and bins to avoid conflicting borrows
        let (common_range, common_bins) = if let Some(first_detector) = self.detectors.get_mut(0) {
            ui.separator();

            let mut range = first_detector.gainmatch.range;
            let mut bins = first_detector.gainmatch.bins;

            ui.horizontal(|ui| {
                ui.label("Gain Matching");

                ui.separator();

                ui.add(
                    egui::DragValue::new(&mut range.0)
                        .speed(1.0)
                        .prefix("Range: ("),
                );
                ui.add(egui::DragValue::new(&mut range.1).speed(1.0).suffix(")"));
                ui.add(egui::DragValue::new(&mut bins).speed(1).prefix("Bins: "));
            });

            // Update the first detector with the new range and bins
            first_detector.gainmatch.range = range;
            first_detector.gainmatch.bins = bins;

            (range, bins)
        } else {
            return; // No detectors to configure
        };

        // Update all other detectors with the common range and bins
        for detector in &mut self.detectors[1..] {
            detector.gainmatch.range = common_range;
            detector.gainmatch.bins = common_bins;
        }

        TableBuilder::new(ui)
            .id_salt("cebra_gain_matching") // Unique identifier for the table
            .column(Column::auto()) // Detector Number
            .column(Column::auto()) // Coefficient A
            .column(Column::auto()) // Coefficient B
            .column(Column::auto()) // Coefficient C
            .column(Column::remainder()) // Active
            .striped(true) // Optional for better readability
            .vscroll(false) // Disable vertical scrolling for compact tables
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("Detector");
                });
                header.col(|ui| {
                    ui.label("A");
                });
                header.col(|ui| {
                    ui.label("B");
                });
                header.col(|ui| {
                    ui.label("C");
                });
                header.col(|ui| {
                    ui.label("Active");
                });
            })
            .body(|mut body| {
                for detector in &mut self.detectors {
                    body.row(18.0, |mut row| {
                        // Detector Number
                        row.col(|ui| {
                            ui.label(format!("#{}", detector.number));
                        });

                        // Coefficient A
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.gainmatch.active,
                                egui::DragValue::new(&mut detector.gainmatch.a).speed(0.01),
                            );
                        });

                        // Coefficient B
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.gainmatch.active,
                                egui::DragValue::new(&mut detector.gainmatch.b).speed(0.01),
                            );
                        });

                        // Coefficient C
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.gainmatch.active,
                                egui::DragValue::new(&mut detector.gainmatch.c).speed(0.01),
                            );
                        });

                        // Active
                        row.col(|ui| {
                            ui.checkbox(&mut detector.gainmatch.active, "");
                        });
                    });
                }
            });
    }

    pub fn energy_calibration_ui(&mut self, ui: &mut egui::Ui) {
        // Temporarily store the range and bins to avoid conflicting borrows
        let (common_range, common_bins) = if let Some(first_detector) = self.detectors.get_mut(0) {
            ui.separator();

            let mut range = first_detector.energy_calibration.range;
            let mut bins = first_detector.energy_calibration.bins;

            ui.horizontal(|ui| {
                ui.label("Energy Calibration");

                ui.separator();

                ui.add(
                    egui::DragValue::new(&mut range.0)
                        .speed(1.0)
                        .prefix(" Range: ("),
                );
                ui.add(egui::DragValue::new(&mut range.1).speed(1.0).suffix(")"));
                ui.add(egui::DragValue::new(&mut bins).speed(1).prefix("Bins: "));

                ui.label(format!("keV/bin: {:.2}", (range.1 - range.0) / bins as f64));
            });

            // Update the first detector with the new range and bins
            first_detector.energy_calibration.range = range;
            first_detector.energy_calibration.bins = bins;

            (range, bins)
        } else {
            return; // No detectors to configure
        };

        // Update all other detectors with the common range and bins
        for detector in &mut self.detectors[1..] {
            detector.energy_calibration.range = common_range;
            detector.energy_calibration.bins = common_bins;
        }

        TableBuilder::new(ui)
            .id_salt("cebra_energy_calibration") // Unique identifier for the table
            .column(Column::auto()) // Detector Number
            .column(Column::auto()) // Coefficient A
            .column(Column::auto()) // Coefficient B
            .column(Column::auto()) // Coefficient C
            .column(Column::remainder()) // Active
            .striped(true) // Optional for better readability
            .vscroll(false) // Disable vertical scrolling for compact tables
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("Detector");
                });
                header.col(|ui| {
                    ui.label("A");
                });
                header.col(|ui| {
                    ui.label("B");
                });
                header.col(|ui| {
                    ui.label("C");
                });
                header.col(|ui| {
                    ui.label("Active");
                });
            })
            .body(|mut body| {
                for detector in &mut self.detectors {
                    body.row(18.0, |mut row| {
                        // Detector Number
                        row.col(|ui| {
                            ui.label(format!("#{}", detector.number));
                        });

                        // Coefficient A
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.energy_calibration.active,
                                egui::DragValue::new(&mut detector.energy_calibration.a)
                                    .speed(0.01),
                            );
                        });

                        // Coefficient B
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.energy_calibration.active,
                                egui::DragValue::new(&mut detector.energy_calibration.b)
                                    .speed(0.01),
                            );
                        });

                        // Coefficient C
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.energy_calibration.active,
                                egui::DragValue::new(&mut detector.energy_calibration.c)
                                    .speed(0.01),
                            );
                        });

                        // Active
                        row.col(|ui| {
                            ui.checkbox(&mut detector.energy_calibration.active, "");
                        });
                    });
                }
            });
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, sps_config: &SPSConfig) {
        ui.horizontal_wrapped(|ui| {
            for detector in &mut self.detectors {
                ui.checkbox(&mut detector.active, format!("Cebra{}", detector.number));
            }
        });

        // check if there are detectors to configure
        if self.detectors.is_empty() {
            return;
        }

        if sps_config.active {
            self.time_cut_ui(ui);
        }
        self.gain_matching_ui(ui);
        self.energy_calibration_ui(ui);
    }

    pub fn get_configs(&self, sps_config: &SPSConfig) -> Configs {
        let mut configs = Configs::default();

        for detector in &self.detectors {
            configs.merge(detector.config(sps_config.clone()));
        }

        if !sps_config.active {
            configs
        } else {
            let mut updated_configs = Configs::default();
            let sps_cuts = sps_config.cuts.get_active_cuts();

            for config in &mut configs.configs {
                match config {
                    Config::Hist1D(hist) => {
                        let mut cuts = hist.cuts.clone();

                        updated_configs.hist1d(
                            &format!("No Cuts/{}", hist.name),
                            &hist.column_name,
                            hist.range,
                            hist.bins,
                            Some(cuts.clone()),
                        );

                        cuts.merge(&sps_cuts.clone());

                        if !sps_cuts.is_empty() {
                            updated_configs.hist1d(
                                &format!("Cuts/{}", hist.name),
                                &hist.column_name,
                                hist.range,
                                hist.bins,
                                Some(cuts.clone()),
                            );
                        }
                    }
                    Config::Hist2D(hist) => {
                        let mut cuts = hist.cuts.clone();

                        updated_configs.hist2d(
                            &format!("No Cuts/{}", hist.name),
                            &hist.x_column_name,
                            &hist.y_column_name,
                            hist.x_range,
                            hist.y_range,
                            hist.bins,
                            Some(cuts.clone()),
                        );

                        cuts.merge(&sps_cuts.clone());

                        if !sps_cuts.is_empty() {
                            updated_configs.hist2d(
                                &format!("Cuts/{}", hist.name),
                                &hist.x_column_name,
                                &hist.y_column_name,
                                hist.x_range,
                                hist.y_range,
                                hist.bins,
                                Some(cuts.clone()),
                            );
                        }
                    }
                }
            }

            updated_configs.columns = configs.columns.clone();
            updated_configs.cuts = configs.cuts.clone();

            if !sps_cuts.is_empty() {
                updated_configs.cuts.merge(&sps_cuts.clone());
            }

            updated_configs
        }
    }

    pub fn cr52dp_experiment(&mut self) {
        self.detectors.clear();

        let mut detector_0 = Cebr3::new(0);

        detector_0.active = true;
        detector_0.energy_calibration.active = true;
        detector_0.energy_calibration.a = 0.0;
        detector_0.energy_calibration.b = 1.7551059351549314;
        detector_0.energy_calibration.c = -12.273506897222896;

        detector_0.timecut.active = true;
        detector_0.timecut.mean = -1155.6;
        detector_0.timecut.low = -1158.0;
        detector_0.timecut.high = -1152.0;

        self.detectors.push(detector_0);

        let mut detector_1 = Cebr3::new(1);

        detector_1.active = true;
        detector_1.energy_calibration.active = true;
        detector_1.energy_calibration.a = 0.0;
        detector_1.energy_calibration.b = 1.9510278378962256;
        detector_1.energy_calibration.c = -16.0245754973971;

        detector_1.timecut.active = true;
        detector_1.timecut.mean = -1153.9;
        detector_1.timecut.low = -1159.0;
        detector_1.timecut.high = -1147.0;

        self.detectors.push(detector_1);

        let mut detector_2 = Cebr3::new(2);

        detector_2.active = true;
        detector_2.energy_calibration.active = true;
        detector_2.energy_calibration.a = 0.0;
        detector_2.energy_calibration.b = 1.917190081718234;
        detector_2.energy_calibration.c = 16.430212777833802;

        detector_2.timecut.active = true;
        detector_2.timecut.mean = -1154.0;
        detector_2.timecut.low = -1158.0;
        detector_2.timecut.high = -1151.0;

        self.detectors.push(detector_2);

        let mut detector_3 = Cebr3::new(3);

        detector_3.active = true;
        detector_3.energy_calibration.active = true;
        detector_3.energy_calibration.a = 0.0;
        detector_3.energy_calibration.b = 1.6931918955746692;
        detector_3.energy_calibration.c = 12.021258506937766;

        detector_3.timecut.active = true;
        detector_3.timecut.mean = -1152.0;
        detector_3.timecut.low = -1158.0;
        detector_3.timecut.high = -1148.0;

        self.detectors.push(detector_3);

        let mut detector_4 = Cebr3::new(4);

        detector_4.active = true;
        detector_4.energy_calibration.active = true;
        detector_4.energy_calibration.a = 0.0;
        detector_4.energy_calibration.b = 1.6373533248536343;
        detector_4.energy_calibration.c = 13.091030061910748;

        detector_4.timecut.active = true;
        detector_4.timecut.mean = -1123.1;
        detector_4.timecut.low = -1127.0;
        detector_4.timecut.high = -1118.0;

        self.detectors.push(detector_4);

        for detector in &mut self.detectors {
            detector.gainmatch.active = false;
            detector.energy_calibration.bins = 500;
            detector.energy_calibration.range = (0.0, 5500.0);
        }
    }
}

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
    pub fn configs(&self, cebra_config: CeBrAConfig, sps_config: &mut SPSConfig) -> Configs {
        let mut configs = Configs::default();

        let pips_range = self.range;
        let pips_bins = self.bins;

        let energy = format!("PIPS{}Energy", self.name);
        let energy_calibrated = format!("PIPS{}EnergyCalibrated", self.name);
        let time = format!("PIPS{}Time", self.name);

        configs.hist1d(&format!("ICESPICE/PIPS{}/PIPS{}Energy", self.name, self.name), &energy, pips_range, pips_bins, None);

        if self.energy_calibration.active {
            configs.columns.push(self.energy_calibration.new_column(&energy, &format!("PIPS{}EnergyCalibrated", self.name)));
            configs.hist1d(&format!("ICESPICE/PIPS{}/PIPS{}EnergyCalibrated", self.name, self.name), &format!("PIPS{}EnergyCalibrated", self.name), self.energy_calibration.range, self.energy_calibration.bins, None);
            configs.hist1d(&format!("ICESPICE/PIPS/PIPSEnergyCalibrated"), &format!("PIPS{}EnergyCalibrated", self.name), self.energy_calibration.range, self.energy_calibration.bins, None);
        }

        if cebra_config.active {
            for cebr3 in cebra_config.detectors.iter() {
                if cebr3.active {

                    let base_path = format!("ICESPICE/PIPS{}/Cebra{}", self.name, cebr3.number);

                    let cebr3_energy = format!("Cebra{}Energy", cebr3.number);
                    let cebr3_energy_calibrated = format!("Cebra{}EnergyCalibrated", cebr3.number);
                    let cebr3_time = format!("Cebra{}Time", cebr3.number);

                    let cebr3_range = (0.0, 4096.0);
                    let cebr3_bins = 4096;

                    // create the time difference column
                    configs.columns.push((format!("{} - {}", cebr3_time, time), format!("PIPS{}TimeRelToCebra{}Time", self.name, cebr3.number)));

                    let valid_time_cut = Cut::new_1d(&format!("Valid PIPS{} Cebra{} Time Cut", self.name, cebr3.number), &format!("PIPS{}Energy > 0.0 && Cebra{}Energy > 0.0", self.name, cebr3.number));
                    configs.cuts.add_cut(valid_time_cut.clone());
                    let tcut = Some(Cuts::new(vec![valid_time_cut.clone()]));

                    configs.hist1d(&format!("{}/PIPS{}TimeRelToCebra{}Time", base_path, self.name, cebr3.number), &format!("PIPS{}TimeRelToCebra{}Time", self.name, cebr3.number), cebr3.pips_timecuts.pips1000.no_cut_range, cebr3.pips_timecuts.pips1000.no_cut_bins, tcut.clone());
                    configs.hist2d(&format!("{}/PIPS{}Energy v Cebra{}Energy",base_path, self.name, cebr3.number), &cebr3_energy, &energy, cebr3_range, pips_range, (cebr3_bins, pips_bins), tcut.clone());
                    // configs.hist2d(&format!("{}/PIPS{}RelToCebra{} v Cebra{}Energy", base_path, self.name, cebr3.number, cebr3.number), &format!("PIPS{}TimeRelToCebra{}Time", self.name, cebr3.number), &cebr3_energy, cebr3.pips_timecuts.pips1000.no_cut_range, cebr3_range, (cebr3.pips_timecuts.pips1000.no_cut_bins, cebr3_bins), tcut.clone());


                    // energy calibrated histograms
                    if self.energy_calibration.active {
                        configs.hist2d(&format!("{}/Energy Calibrated/PIPS{}EnergyCalibrated v Cebra{}Energy", base_path, self.name, cebr3.number), &cebr3_energy, &energy_calibrated, cebr3_range, self.energy_calibration.range, (cebr3_bins, self.energy_calibration.bins), None);
                    }
                    if cebr3.energy_calibration.active & self.energy_calibration.active {
                        configs.hist2d(&format!("{}/Energy Calibrated/PIPS{}EnergyCalibrated v Cebra{}EnergyCalibrated", base_path, self.name, cebr3.number), &cebr3_energy_calibrated, &energy_calibrated, cebr3.energy_calibration.range, self.energy_calibration.range, (cebr3.energy_calibration.bins, self.energy_calibration.bins), None);
                    }
                    // if cebr3.energy_calibration.active {
                        // configs.hist2d(&format!("{}/Energy Calibrated/PIPS{}TimeRelToCebra{}Time v Cebra{}EnergyCalibrated", base_path, self.name, cebr3.number, cebr3.number), &format!("PIPS{}TimeRelToCebra{}Time", self.name, cebr3.number), &cebr3_energy_calibrated, cebr3.pips_timecuts.pips1000.no_cut_range, cebr3.energy_calibration.range, (cebr3.pips_timecuts.pips1000.no_cut_bins, cebr3.energy_calibration.bins), None);
                    // }

                    // check time cuts for the correct detector
                    if cebr3.pips_timecuts.pips1000.active {
                        let min = cebr3.pips_timecuts.pips1000.low;
                        let max = cebr3.pips_timecuts.pips1000.high;
                        let mean = cebr3.pips_timecuts.pips1000.mean;
                        let time_range = cebr3.pips_timecuts.pips1000.range;
                        let time_bins = cebr3.pips_timecuts.pips1000.bins;

                        // add column for the time cut to shift the time
                        configs.columns.push((format!("{} - {} - {}", cebr3_time, time, mean), format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number)));

                        // add the time cut

                        let pips_cebra_tcut = Cut::new_1d(&format!("PIPS{}+Cebra{} Time Cut", self.name, cebr3.number), &format!("PIPS{}TimeRelToCebra{}Time >= {} && PIPS{}TimeRelToCebra{}Time <= {}", self.name, cebr3.number, min, self.name, cebr3.number, max));
                        configs.cuts.add_cut(pips_cebra_tcut.clone());
                        let tcut = Some(Cuts::new(vec![pips_cebra_tcut.clone(), valid_time_cut.clone()]));

                        configs.hist1d(&format!("{}/Time Cut/PIPS{}TimeRelToCebra{}TimeShifted", base_path, self.name, cebr3.number), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), time_range, time_bins, tcut.clone());
                        configs.hist1d(&format!("ICESPICE/PIPS{}/CeBrA/Time Cut - PIPS{}TimeRelToCeBrA", self.name, self.name), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), time_range, time_bins, tcut.clone());

                        configs.hist2d(&format!("{}/Time Cut/PIPS{}Energy v Cebra{}Energy", base_path, self.name, cebr3.number), &cebr3_energy, &energy, cebr3_range, pips_range, (cebr3_bins, pips_bins), tcut.clone());
                        configs.hist2d(&format!("{}/Time Cut/PIPS{}RelToCebra{}Shifted v Cebra{}Energy", base_path, self.name, cebr3.number, cebr3.number), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), &cebr3_energy, time_range, cebr3_range, (time_bins, cebr3_bins), tcut.clone());
                        // energy calibrated histograms
                        if self.energy_calibration.active & !cebr3.energy_calibration.active {
                            configs.hist2d(&format!("{}/Time Cut/Energy Calibrated/PIPS{}EnergyCalibrated v Cebra{}Energy", base_path, self.name, cebr3.number), &cebr3_energy, &energy_calibrated, cebr3_range, self.energy_calibration.range, (cebr3_bins, self.energy_calibration.bins), tcut.clone());
                        } else if cebr3.energy_calibration.active & self.energy_calibration.active {
                            configs.hist2d(&format!("{}/Time Cut/Energy Calibrated/PIPS{}EnergyCalibrated v Cebra{}EnergyCalibrated", base_path, self.name, cebr3.number), &cebr3_energy_calibrated, &energy_calibrated, cebr3.energy_calibration.range, self.energy_calibration.range, (cebr3.energy_calibration.bins, self.energy_calibration.bins), tcut.clone());
                            configs.hist2d(&format!("ICESPICE/PIPS{}/CeBrA/Time Cut- PIPS{}EnergyCalibrated v CeBrA", self.name, self.name), &cebr3_energy_calibrated, &energy_calibrated, cebr3.energy_calibration.range, self.energy_calibration.range, (cebr3.energy_calibration.bins, self.energy_calibration.bins), tcut.clone());
                        }
                        if cebr3.energy_calibration.active {
                            configs.hist2d(&format!("{}/Time Cut/Energy Calibrated/PIPS{}TimeRelToCebra{}TimeShifted v Cebra{}EnergyCalibrated", base_path, self.name, cebr3.number, cebr3.number), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), &cebr3_energy_calibrated, time_range, cebr3.energy_calibration.range, (time_bins, cebr3.energy_calibration.bins), tcut.clone());
                            configs.hist2d(&format!("ICESPICE/PIPS{}/CeBrA/Time Cut - PIPS{}TimeRelToCeBrA v CeBrA", self.name, self.name), &format!("PIPS{}TimeRelToCebra{}TimeShifted", self.name, cebr3.number), &cebr3_energy_calibrated, time_range, cebr3.energy_calibration.range, (time_bins, cebr3.energy_calibration.bins), tcut.clone());
                        }
                    }
                }
            }

        }

        if sps_config.active {
            let rel_time_column = format!("PIPS{}RelTime", self.name); // PIPS{}Time - ScintLeftTime -> Column is made in the eventbuilder with the anode condition too
            let pips_energy = format!("PIPS{}Energy", self.name);

            // rel time histogram
            configs.hist1d(&format!("ICESPICE/PIPS{}/SPS/PIPS{}RelTime", self.name, self.name), &rel_time_column, self.sps_timecut.no_cut_range, self.sps_timecut.no_cut_bins, None);

            // rel time vs xavg
            configs.hist2d(&format!("ICESPICE/PIPS{}/SPS/PIPS{}RelTime v Xavg", self.name, self.name), &rel_time_column, &format!("Xavg"), self.sps_timecut.no_cut_range, (-300.0, 300.0), (self.sps_timecut.no_cut_bins, 600), None);

            // pips energy vs xavg
            configs.hist2d(&format!("ICESPICE/PIPS{}/SPS/PIPS{}Energy v Xavg", self.name, self.name), &format!("Xavg"), &pips_energy,(-300.0, 300.0), pips_range, (pips_bins, 600), None);

            if sps_config.xavg.active {
                // rel time vs xavg energy calibrated
                configs.hist2d(&format!("ICESPICE/PIPS{}/SPS/PIPS{}RelTime v XavgEnergyCalibrated", self.name, self.name), &rel_time_column, &format!("XavgEnergyCalibrated"), self.sps_timecut.no_cut_range, sps_config.xavg.range, (self.sps_timecut.no_cut_bins, sps_config.xavg.bins), None);

                // pips energy vs xavg energy calibrated
                configs.hist2d(&format!("ICESPICE/PIPS{}/SPS/PIPS{}Energy v XavgEnergyCalibrated", self.name, self.name), &format!("XavgEnergyCalibrated"), &pips_energy, sps_config.xavg.range, pips_range, (pips_bins, sps_config.xavg.bins), None);
            }

            // if self.energy_calibration.active {

            // }
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
            // collapsing header
            ui.collapsing(format!("PIPS{}", self.name), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Energy Calibration: ");
                    self.energy_calibration.ui(ui, false);
                });

                if sps_config.active {
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("SE-SPS Time Cut: ");
                        self.sps_timecut.ui(ui);
                    });
                    ui.separator();
                }

                if cebra_config.active {
                    for cebr3 in &mut cebra_config.detectors {
                        if cebr3.active {
                            ui.horizontal(|ui| {
                                ui.label(format!("Cebra{} Time Cut: ", cebr3.number));
                                if self.name == "1000" {
                                    cebr3.pips_timecuts.pips1000.ui(ui);
                                } else if self.name == "500" {
                                    cebr3.pips_timecuts.pips500.ui(ui);
                                } else if self.name == "300" {
                                    cebr3.pips_timecuts.pips300.ui(ui);
                                } else if self.name == "100" {
                                    cebr3.pips_timecuts.pips100.ui(ui);
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

        ui.separator();

        ui.label("PIPS Detectors");

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.pips1000.active, "PIPS1000");
            ui.checkbox(&mut self.pips500.active, "PIPS500");
            ui.checkbox(&mut self.pips300.active, "PIPS300");
            ui.checkbox(&mut self.pips100.active, "PIPS100");
        });

        // Default Histogram Values
        ui.horizontal(|ui| {
            ui.label("Default Histogram Values:");
            ui.add(
                egui::DragValue::new(&mut self.pips1000.range.0)
                    .speed(1.0)
                    .prefix("Range: ("),
            );
            ui.add(
                egui::DragValue::new(&mut self.pips1000.range.1)
                    .speed(1.0)
                    .suffix(")"),
            );

            ui.add(
                egui::DragValue::new(&mut self.pips1000.bins)
                    .speed(1)
                    .prefix("Bins: "),
            );

            // set the other pips detectors to the same range and bins
            self.pips500.range = self.pips1000.range;
            self.pips500.bins = self.pips1000.bins;

            self.pips300.range = self.pips1000.range;
            self.pips300.bins = self.pips1000.bins;

            self.pips100.range = self.pips1000.range;
            self.pips100.bins = self.pips1000.bins;
        });

        // Energy Calibration Values
        ui.horizontal(|ui| {
            ui.label("Energy Calibration Values:");
            ui.add(
                egui::DragValue::new(&mut self.pips1000.energy_calibration.range.0)
                    .speed(1.0)
                    .prefix("Range: ("),
            );
            ui.add(
                egui::DragValue::new(&mut self.pips1000.energy_calibration.range.1)
                    .speed(1.0)
                    .suffix(")"),
            );

            ui.add(
                egui::DragValue::new(&mut self.pips1000.energy_calibration.bins)
                    .speed(1)
                    .prefix("Bins: "),
            );

            // set the other pips detectors to the same range and bins
            self.pips500.energy_calibration.range = self.pips1000.energy_calibration.range;
            self.pips500.energy_calibration.bins = self.pips1000.energy_calibration.bins;

            self.pips300.energy_calibration.range = self.pips1000.energy_calibration.range;
            self.pips300.energy_calibration.bins = self.pips1000.energy_calibration.bins;

            self.pips100.energy_calibration.range = self.pips1000.energy_calibration.range;
            self.pips100.energy_calibration.bins = self.pips1000.energy_calibration.bins;
        });

        // CeBrA Time Cut Values
        if cebra_config.active {
            for cebr3 in &mut cebra_config.detectors {
                if cebr3.active {
                    ui.horizontal(|ui| {
                        ui.label(format!("Cebra{} Time Cut Values:", cebr3.number));
                        if self.pips1000.active {
                            cebr3.pips_timecuts.pips1000.ui(ui);
                        } else if self.pips500.active {
                            cebr3.pips_timecuts.pips500.ui(ui);
                        } else if self.pips300.active {
                            cebr3.pips_timecuts.pips300.ui(ui);
                        } else if self.pips100.active {
                            cebr3.pips_timecuts.pips100.ui(ui);
                        }
                    });
                }
            }
        }

        ui.separator();

        self.pips1000.ui(ui, cebra_config, sps_config);
        self.pips500.ui(ui, cebra_config, sps_config);
        self.pips300.ui(ui, cebra_config, sps_config);
        self.pips100.ui(ui, cebra_config, sps_config);
    }

    pub fn get_configs(
        &self,
        cebra_config: &mut CeBrAConfig,
        sps_config: &mut SPSConfig,
    ) -> Configs {
        let mut configs = Configs::default();

        if self.pips1000.active {
            configs.merge(self.pips1000.configs(cebra_config.clone(), sps_config));
        }

        if self.pips500.active {
            configs.merge(self.pips500.configs(cebra_config.clone(), sps_config));
        }

        if self.pips300.active {
            configs.merge(self.pips300.configs(cebra_config.clone(), sps_config));
        }

        if self.pips100.active {
            configs.merge(self.pips100.configs(cebra_config.clone(), sps_config));
        }

        configs
    }
}

/*************************** SE-SPS Custom Struct ***************************/
#[derive(Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct SPSConfig {
    active: bool,
    xavg: Calibration,
    cuts: Cuts,
}

impl SPSConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();

        ui.label("Calibration");

        ui.horizontal(|ui| {
            ui.label("Xavg: ");
            self.xavg.ui(ui, true);
        });
        ui.separator();

        self.cuts.ui(ui);
    }

    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub fn sps_configs(&self) -> Configs {
        let mut configs = Configs::default();

        configs.columns.push(("( DelayFrontRightEnergy + DelayFrontLeftEnergy ) / 2.0".into(), "DelayFrontAverageEnergy".into()));
        configs.columns.push(("( DelayBackRightEnergy + DelayBackLeftEnergy ) / 2.0".into(), "DelayBackAverageEnergy".into()));
        configs.columns.push(("DelayFrontLeftTime - AnodeFrontTime".into(), "DelayFrontLeftTime_AnodeFrontTime".into()));
        configs.columns.push(("DelayFrontRightTime - AnodeFrontTime".into(), "DelayFrontRightTime_AnodeFrontTime".into()));
        configs.columns.push(("DelayBackLeftTime - AnodeFrontTime".into(), "DelayBackLeftTime_AnodeFrontTime".into()));
        configs.columns.push(("DelayBackRightTime - AnodeFrontTime".into(), "DelayBackRightTime_AnodeFrontTime".into()));
        configs.columns.push(("DelayFrontLeftTime - AnodeBackTime".into(), "DelayFrontLeftTime_AnodeBackTime".into()));
        configs.columns.push(("DelayFrontRightTime - AnodeBackTime".into(), "DelayFrontRightTime_AnodeBackTime".into()));
        configs.columns.push(("DelayBackLeftTime - AnodeBackTime".into(), "DelayBackLeftTime_AnodeBackTime".into()));
        configs.columns.push(("DelayBackRightTime - AnodeBackTime".into(), "DelayBackRightTime_AnodeBackTime".into()));
        configs.columns.push(("AnodeFrontTime - AnodeBackTime".into(), "AnodeFrontTime_AnodeBackTime".into()));
        configs.columns.push(("AnodeBackTime - AnodeFrontTime".into(), "AnodeBackTime_AnodeFrontTime".into()));
        configs.columns.push(("AnodeFrontTime - ScintLeftTime".into(), "AnodeFrontTime_ScintLeftTime".into()));
        configs.columns.push(("AnodeBackTime - ScintLeftTime".into(), "AnodeBackTime_ScintLeftTime".into()));
        configs.columns.push(("DelayFrontLeftTime - ScintLeftTime".into(), "DelayFrontLeftTime_ScintLeftTime".into()));
        configs.columns.push(("DelayFrontRightTime - ScintLeftTime".into(), "DelayFrontRightTime_ScintLeftTime".into()));
        configs.columns.push(("DelayBackLeftTime - ScintLeftTime".into(), "DelayBackLeftTime_ScintLeftTime".into()));
        configs.columns.push(("DelayBackRightTime - ScintLeftTime".into(), "DelayBackRightTime_ScintLeftTime".into()));
        configs.columns.push(("ScintRightTime - ScintLeftTime".into(), "ScintRightTime_ScintLeftTime".into()));

        if self.xavg.active {
            configs.columns.push(self.xavg.new_column("Xavg", "XavgEnergyCalibrated"));
        }

        let mut cuts = Cuts::default();

        let bothplanes_cut = Cut::new_1d("Both Planes", "X2 != -1e6 && X1 != -1e6");
        let only_x1_plane_cut = Cut::new_1d("Only X1 Plane", "X1 != -1e6 && X2 == -1e6");
        let only_x2_plane_cut = Cut::new_1d("Only X2 Plane", "X2 != -1e6 && X1 == -1e6");

        cuts.add_cut(bothplanes_cut.clone());
        cuts.add_cut(only_x1_plane_cut.clone());
        cuts.add_cut(only_x2_plane_cut.clone());

        let fp_range = (-300.0, 300.0);
        let fp_bins = 600;

        let range = (0.0, 4096.0);
        let bins = 512;

        // Focal plane histograms
        configs.hist1d("SE-SPS/Focal Plane/X1", "X1", fp_range, fp_bins, None);
        configs.hist1d("SE-SPS/Focal Plane/X2", "X2", fp_range, fp_bins, None);
        configs.hist1d("SE-SPS/Focal Plane/Xavg", "Xavg", fp_range, fp_bins, None);
        if self.xavg.active {
            configs.hist1d("SE-SPS/Focal Plane/Xavg Energy Calibrated", "XavgEnergyCalibrated", self.xavg.range, self.xavg.bins, None);
        }
        configs.hist2d("SE-SPS/Focal Plane/X2 v X1", "X1", "X2", fp_range, fp_range, (fp_bins, fp_bins), None);
        configs.hist2d("SE-SPS/Focal Plane/Theta v Xavg", "Xavg", "Theta", fp_range, (0.0, PI), (fp_bins, fp_bins), None);
        // configs.hist2d("SE-SPS/Focal Plane/Rays", "X", "Z", fp_range, (-50.0, 50.0), (fp_bins, 100), None);

        let cut_bothplanes = Some(Cuts::new(vec![bothplanes_cut.clone()]));
        let cut_only_x1_plane = Some(Cuts::new(vec![only_x1_plane_cut]));
        let cut_only_x2_plane = Some(Cuts::new(vec![only_x2_plane_cut]));

        configs.hist1d("SE-SPS/Focal Plane/Checks/Xavg", "Xavg", fp_range, fp_bins, None);
        configs.hist1d("SE-SPS/Focal Plane/Checks/Raw- X1", "X1", fp_range, fp_bins, None);
        configs.hist1d("SE-SPS/Focal Plane/Checks/Both Planes- X1", "X1", fp_range, fp_bins, cut_bothplanes.clone());
        configs.hist1d("SE-SPS/Focal Plane/Checks/Only 1 Plane- X1", "X1", fp_range, fp_bins, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Focal Plane/Checks/Raw- X2", "X2", fp_range, fp_bins, None);
        configs.hist1d("SE-SPS/Focal Plane/Checks/Both Planes- X2", "X2", fp_range, fp_bins, cut_bothplanes.clone());
        configs.hist1d("SE-SPS/Focal Plane/Checks/Only 1 Plane- X2", "X2", fp_range, fp_bins, cut_only_x2_plane.clone());

        // Particle Identification histograms
        configs.hist2d("SE-SPS/Particle Identification/AnodeBack v ScintLeft", "ScintLeftEnergy", "AnodeBackEnergy", range, range, (bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification/AnodeFront v ScintLeft", "ScintLeftEnergy", "AnodeFrontEnergy", range, range, (bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification/Cathode v ScintLeft", "ScintLeftEnergy", "CathodeEnergy", range, range, (bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification/AnodeBack v ScintRight", "ScintRightEnergy", "AnodeBackEnergy", range, range, (bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification/AnodeFront v ScintRight", "ScintRightEnergy", "AnodeFrontEnergy", range, range, (bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification/Cathode v ScintRight", "ScintRightEnergy", "CathodeEnergy", range, range, (bins,bins), None);

        // Particle Identification vs Focal plane histograms
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/ScintLeft v X1", "X1", "ScintLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/ScintLeft v X2", "X2", "ScintLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/ScintLeft v Xavg", "Xavg", "ScintLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/ScintRight v X1", "X1", "ScintRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/ScintRight v X2", "X2", "ScintRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/ScintRight v Xavg", "Xavg", "ScintRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeBack v X1", "X1", "AnodeBackEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeBack v X2", "X2", "AnodeBackEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeBack v Xavg", "Xavg", "AnodeBackEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeFront v X1", "X1", "AnodeFrontEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeFront v X2", "X2", "AnodeFrontEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeFront v Xavg", "Xavg", "AnodeFrontEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/Cathode v X1", "X1", "CathodeEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/Cathode v X2", "X2", "CathodeEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/Cathode v Xavg", "Xavg", "CathodeEnergy", fp_range, range, (fp_bins,bins), None);

        // Delay lines vs Focal plane histograms
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v X1", "X1", "DelayFrontRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v X1", "X1", "DelayFrontLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v X2", "X2", "DelayBackRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v X2", "X2", "DelayBackLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v Xavg", "Xavg", "DelayBackRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v Xavg", "Xavg", "DelayBackLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v Xavg", "Xavg", "DelayFrontRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v Xavg", "Xavg", "DelayFrontLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v X1", "X1", "DelayBackRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v X1", "X1", "DelayBackLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v X2", "X2", "DelayFrontRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v X2", "X2", "DelayFrontLeftEnergy", fp_range, range, (fp_bins,bins), None);

        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayFrontAverage v X1", "X1", "DelayFrontAverageEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayBackAverage v X1", "X1", "DelayBackAverageEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayFrontAverage v X2", "X2", "DelayFrontAverageEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayBackAverage v X2", "X2", "DelayBackAverageEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayFrontAverage v Xavg", "Xavg", "DelayFrontAverageEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayBackAverage v Xavg", "Xavg", "DelayBackAverageEnergy", fp_range, range, (fp_bins,bins), None);


        // Delay timing relative to anodes histograms
        let valid_sps_timing = Cut::new_1d("Valid SPS Timing", "AnodeBackTime != -1e6 && ScintLeftTime != -1e6");
        cuts.add_cut(valid_sps_timing.clone());

        let cut_timing = Some(Cuts::new(vec![valid_sps_timing.clone()]));

        configs.hist1d("SE-SPS/Timing/AnodeFrontTime-AnodeBackTime", "AnodeFrontTime_AnodeBackTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/AnodeBackTime-AnodeFrontTime", "AnodeBackTime_AnodeFrontTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/AnodeFrontTime-ScintLeftTime", "AnodeFrontTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/AnodeBackTime-ScintLeftTime", "AnodeBackTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/DelayFrontLeftTime-ScintLeftTime", "DelayFrontLeftTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/DelayFrontRightTime-ScintLeftTime", "DelayFrontRightTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/DelayBackLeftTime-ScintLeftTime", "DelayBackLeftTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/DelayBackRightTime-ScintLeftTime", "DelayBackRightTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/ScintRightTime-ScintLeftTime", "ScintRightTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist2d("SE-SPS/Timing/ScintTimeDif v Xavg", "Xavg", "ScintRightTime_ScintLeftTime", fp_range, (-3200.0, 3200.0), (fp_bins, 12800), cut_timing.clone());


        configs.hist1d("SE-SPS/Timing/Both Planes/DelayFrontLeftTime-AnodeFrontTime", "DelayFrontLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_bothplanes.clone());
        configs.hist1d("SE-SPS/Timing/Both Planes/DelayFrontRightTime-AnodeFrontTime", "DelayFrontRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_bothplanes.clone());
        configs.hist1d("SE-SPS/Timing/Both Planes/DelayBackLeftTime-AnodeBackTime", "DelayBackLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_bothplanes.clone());
        configs.hist1d("SE-SPS/Timing/Both Planes/DelayBackRightTime-AnodeBackTime", "DelayBackRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_bothplanes.clone());

        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayFrontLeftTime-AnodeFrontTime", "DelayFrontLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayFrontRightTime-AnodeFrontTime", "DelayFrontRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayBackLeftTime-AnodeFrontTime", "DelayBackLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayBackRightTime-AnodeFrontTime", "DelayBackRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayFrontLeftTime-AnodeBackTime", "DelayFrontLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayFrontRightTime-AnodeBackTime", "DelayFrontRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayBackLeftTime-AnodeBackTime", "DelayBackLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayBackRightTime-AnodeBackTime", "DelayBackRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());

        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayFrontLeftTime-AnodeFrontTime", "DelayFrontLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayFrontRightTime-AnodeFrontTime", "DelayFrontRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayBackLeftTime-AnodeFrontTime", "DelayBackLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayBackRightTime-AnodeFrontTime", "DelayBackRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayFrontLeftTime-AnodeBackTime", "DelayFrontLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayFrontRightTime-AnodeBackTime", "DelayFrontRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayBackLeftTime-AnodeBackTime", "DelayBackLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayBackRightTime-AnodeBackTime", "DelayBackRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());

        configs.cuts = cuts;

        configs
    }

    pub fn update_configs_with_cuts(&self) -> Configs {
        // Get the active cuts
        let active_cuts = self.cuts.get_active_cuts();

        let mut updated_configs = Configs::default();

        let original_configs = self.sps_configs();

        for config in &original_configs.configs {
            match config {
                Config::Hist1D(hist) => {
                    let mut cuts = hist.cuts.clone();

                    updated_configs.hist1d(
                        &format!("No Cuts/{}", hist.name),
                        &hist.column_name,
                        hist.range,
                        hist.bins,
                        Some(cuts.clone()),
                    );

                    cuts.merge(&active_cuts.clone());

                    if !active_cuts.is_empty() {
                        updated_configs.hist1d(
                            &format!("Cuts/{}", hist.name),
                            &hist.column_name,
                            hist.range,
                            hist.bins,
                            Some(cuts.clone()),
                        );
                    }
                }
                Config::Hist2D(hist) => {
                    let mut cuts = hist.cuts.clone();

                    updated_configs.hist2d(
                        &format!("No Cuts/{}", hist.name),
                        &hist.x_column_name,
                        &hist.y_column_name,
                        hist.x_range,
                        hist.y_range,
                        hist.bins,
                        Some(cuts.clone()),
                    );

                    cuts.merge(&active_cuts.clone());

                    if !active_cuts.is_empty() {
                        updated_configs.hist2d(
                            &format!("Cuts/{}", hist.name),
                            &hist.x_column_name,
                            &hist.y_column_name,
                            hist.x_range,
                            hist.y_range,
                            hist.bins,
                            Some(cuts.clone()),
                        );
                    }
                }
            }
        }

        updated_configs.columns = original_configs.columns.clone();

        updated_configs.cuts = original_configs.cuts.clone();
        updated_configs.cuts.merge(&active_cuts);

        updated_configs
    }
}
