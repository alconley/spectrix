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
}

impl Default for CustomConfigs {
    fn default() -> Self {
        Self {
            sps: SPSConfig::new(),
            cebra: CeBrAConfig::default(),
        }
    }
}

impl CustomConfigs {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Custom: ");
            ui.checkbox(&mut self.sps.active, "SPS");
            ui.checkbox(&mut self.cebra.active, "CeBrA");
        });

        if self.sps.active {
            ui.collapsing("SE-SPS", |ui| {
                self.sps.ui(ui);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() - 40.0);
                    if ui.button("Reset").clicked() {
                        self.sps = SPSConfig::new();
                        self.sps.active = true;
                    }
                });
            });
        }

        if self.cebra.active {
            ui.collapsing("CeBrA", |ui| {
                self.cebra.ui(ui);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() - 40.0);
                    if ui.button("Reset").clicked() {
                        self.cebra = CeBrAConfig::default();
                        self.cebra.active = true;
                    }
                });
            });
        }
    }

    pub fn merge_active_configs(&self) -> Configs {
        let mut configs = Configs::default();

        if self.sps.active {
            // get the updated configs from sps
            let sps_configs = self.sps.update_configs_with_cuts();
            configs.merge(sps_configs.clone()); // Ensure `merge` handles in-place modifications
        }

        if self.cebra.active {
            // get the updated configs from cebra
            let cebra_configs = self.cebra.get_configs();
            configs.merge(cebra_configs.clone()); // Ensure `merge` handles in-place modifications
        }

        configs
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

impl Calibration {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.active {
                ui.add(egui::DragValue::new(&mut self.a).speed(0.01).prefix("a: "));
                ui.add(egui::DragValue::new(&mut self.b).speed(0.01).prefix("b: "));
                ui.add(egui::DragValue::new(&mut self.c).speed(0.01).prefix("c: "));

                ui.separator();
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

                ui.label(format!(
                    "keV/bin: {:.2}",
                    (self.range.1 - self.range.0) / self.bins as f64
                ));
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
    pub active: bool,
}

impl TimeCut {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Time Cut: ");
            if self.active {
                ui.add(
                    egui::DragValue::new(&mut self.mean)
                        .speed(0.01)
                        .prefix("Mean: "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.low)
                        .speed(0.01)
                        .prefix("Low: "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.high)
                        .speed(0.01)
                        .prefix("High: "),
                );
            }
            ui.checkbox(&mut self.active, "Active");
        });
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Cebr3 {
    pub number: usize,
    pub sps: bool,
    pub timecut: TimeCut,
    pub gainmatch: Calibration,
    pub energy_calibration: Calibration,
}

impl Cebr3 {
    pub fn new(number: usize, sps: bool) -> Self {
        Self {
            number,
            sps,
            timecut: TimeCut {
                mean: 0.0,
                low: -3000.0,
                high: 3000.0,
                active: sps,
            },
            gainmatch: Calibration {
                a: 0.0,
                b: 1.0,
                c: 0.0,
                bins: 512,
                range: (0.0, 4096.0),
                active: false,
            },
            energy_calibration: Calibration {
                a: 0.0,
                b: 1.0,
                c: 0.0,
                bins: 512,
                range: (0.0, 4096.0),
                active: false,
            },
        }
    }

    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub fn config(&self) -> Configs {

        let mut configs = Configs::default();

        let range = (0.0, 4096.0);
        let bins = 512;

        configs.hist1d(&format!("CeBrA/Cebra{}/Cebra{}Energy", self.number, self.number), &format!("Cebra{}Energy", self.number), range, bins, None);


        if self.gainmatch.active {
            configs.columns.push(self.gainmatch.new_column(&format!("Cebra{}Energy", self.number),&format!("Cebra{}GainMatched", self.number)));
            configs.hist1d(&format!("CeBrA/Cebra{}/Cebra{} Gain Matched", self.number, self.number), &format!("Cebra{}GainMatched", self.number), self.gainmatch.range, self.gainmatch.bins, None); 
            configs.hist1d(&"CeBrA/Gain Matched", &format!("Cebra{}GainMatched", self.number),  self.gainmatch.range, self.gainmatch.bins, None); 
        }

        if self.energy_calibration.active {
            if self.gainmatch.active {
                configs.columns.push(self.energy_calibration.new_column(&format!("Cebra{}GainMatched", self.number),&format!("Cebra{}EnergyCalibrated", self.number)));
            } else {
                configs.columns.push(self.energy_calibration.new_column(&format!("Cebra{}Energy", self.number),&format!("Cebra{}EnergyCalibrated", self.number)));
            }
            configs.hist1d(&format!("CeBrA/Cebra{}/Cebra{} Energy Calibrated", self.number, self.number), &format!("Cebra{}EnergyCalibrated", self.number), self.energy_calibration.range, self.energy_calibration.bins, None);
            configs.hist1d(&"CeBrA/Energy Calibrated", &format!("Cebra{}EnergyCalibrated", self.number), self.energy_calibration.range, self.energy_calibration.bins, None);
        }

        if self.sps {
            configs.hist1d(&format!("CeBrA/Cebra{}/Cebra{}RelTime", self.number, self.number), &format!("Cebra{}RelTime", self.number), (-3200.0, 3200.0), 6400, None);
            configs.hist2d(&format!("CeBrA/Cebra{}/Cebra{}Energy v Xavg", self.number, self.number), &format!("Xavg"), &format!("Cebra{}Energy", self.number), (-300.0, 300.0), (0.0, 4096.0), (600, 512), None);
            configs.hist2d(&format!("CeBrA/Cebra{}/Cebra{}RelTime v Xavg", self.number, self.number), &format!("Xavg"), &format!("Cebra{}RelTime", self.number), (-300.0, 300.0), (-3200.0, 3200.0), (600, 6400), None);
            configs.hist2d(&format!("CeBrA/Cebra{}/Theta v Cebra{}RelTime", self.number, self.number), &format!("Cebra{}RelTime", self.number), "Theta", (-3200.0, 3200.0), (0.0, PI), (6400, 300), None);

            if self.timecut.active {
                // columns with 2 minus do not work
                configs.columns.push((format!("Cebra{}RelTime - {}", self.number, self.timecut.mean), format!("Cebra{}RelTimeShifted", self.number)));
                configs.hist1d(&format!("CeBrA/Cebra{}/Cebra{}RelTimeShifted", self.number, self.number), &format!("Cebra{}RelTimeShifted", self.number), (-3200.0, 3200.0), 6400, None);


                let cebra_time_cut = Cut::new_1d(&format!("Cebra{} Time Cut", self.number), &format!("Cebra{}RelTime >= {} && Cebra{}RelTime <= {}", self.number, self.timecut.low, self.number, self.timecut.high));
                configs.cuts.add_cut(cebra_time_cut.clone());
                let tcut = Some(Cuts::new(vec![cebra_time_cut.clone()]));

                configs.hist1d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{}RelTime", self.number, self.number), &format!("Cebra{}RelTime", self.number), (-3200.0, 3200.0), 6400, tcut.clone());
                configs.hist1d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{}RelTimeShifted", self.number, self.number), &format!("Cebra{}RelTimeShifted", self.number), (-50.0, 50.0), 100, tcut.clone());

                configs.hist1d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{}Energy", self.number, self.number), &format!("Cebra{}Energy", self.number), range, bins, tcut.clone());
                configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{}Energy v Xavg", self.number, self.number), &format!("Xavg"), &format!("Cebra{}Energy", self.number), (-300.0, 300.0), (0.0, 4096.0), (600, 512), tcut.clone());
                configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{}Energy v X1", self.number, self.number), &format!("X1"), &format!("Cebra{}Energy", self.number), (-300.0, 300.0), (0.0, 4096.0), (600, 512), tcut.clone());

                if self.gainmatch.active {
                    configs.hist1d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} Gain Matched", self.number, self.number), &format!("Cebra{}GainMatched", self.number), self.gainmatch.range, self.gainmatch.bins, tcut.clone());
                    configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} Gain Matched v Xavg", self.number, self.number), &format!("Xavg"), &format!("Cebra{}GainMatched", self.number), (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), tcut.clone());
                    configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} Gain Matched v X1", self.number, self.number), &format!("X1"), &format!("Cebra{}GainMatched", self.number), (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), tcut.clone());

                    configs.hist2d(&format!("CeBrA/Time Cut/CeBrA Gain Matched v Xavg"), &format!("Xavg"), &format!("Cebra{}Energy", self.number), (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), tcut.clone());
                    configs.hist2d(&format!("CeBrA/Time Cut/CeBrA Gain Matched v X1"), &format!("X1"), &format!("Cebra{}Energy", self.number), (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), tcut.clone());   
                }
                if self.energy_calibration.active {
                    configs.hist1d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} Energy Calibrated", self.number, self.number), &format!("Cebra{}EnergyCalibrated", self.number), self.energy_calibration.range, self.energy_calibration.bins, tcut.clone());
                    configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} Energy Calibrated v Xavg", self.number, self.number), &format!("Xavg"), &format!("Cebra{}EnergyCalibrated", self.number), (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), tcut.clone());
                    configs.hist2d(&format!("CeBrA/Cebra{}/Time Cut/Cebra{} Energy Calibrated v X1", self.number, self.number), &format!("X1"), &format!("Cebra{}EnergyCalibrated", self.number), (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), tcut.clone());

                    configs.hist2d(&format!("CeBrA/Time Cut/CeBrA Energy Calibrated v Xavg"), &format!("Xavg"), &format!("Cebra{}EnergyCalibrated", self.number), (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), tcut.clone());
                    configs.hist2d(&format!("CeBrA/Time Cut/CeBrA Energy Calibrated v X1"), &format!("X1"), &format!("Cebra{}EnergyCalibrated", self.number), (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), tcut.clone());
                }
            }
        }

        configs
    }
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct CeBrAConfig {
    pub active: bool,
    pub detectors: Vec<Cebr3>,
    pub sps: bool,
}

impl CeBrAConfig {
    pub fn add_detector(&mut self, number: usize) {
        self.detectors.push(Cebr3::new(number, self.sps));
    }

    pub fn time_cut_ui(&mut self, ui: &mut egui::Ui) {
        let mut indices_to_remove = Vec::new();

        ui.label("Time Cuts");
        // Create the table
        TableBuilder::new(ui)
            .id_salt("cebra_timecuts") // Unique identifier for the table
            .column(Column::auto()) // Detector Number
            .column(Column::auto()) // Mean
            .column(Column::auto()) // Low
            .column(Column::auto()) // High
            .column(Column::auto()) // Active
            .column(Column::remainder()) // Actions (Remove)
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
                header.col(|ui| {
                    ui.label("Actions");
                });
            })
            .body(|mut body| {
                for (index, detector) in self.detectors.iter_mut().enumerate() {
                    body.row(18.0, |mut row| {
                        // Detector Number
                        row.col(|ui| {
                            ui.label(format!("#{}", detector.number));
                        });

                        // Mean
                        row.col(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut detector.timecut.mean)
                                    .speed(0.01)
                                    .prefix(" "),
                            );
                        });

                        // Low
                        row.col(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut detector.timecut.low)
                                    .speed(0.01)
                                    .prefix(" "),
                            );
                        });

                        // High
                        row.col(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut detector.timecut.high)
                                    .speed(0.01)
                                    .prefix(" "),
                            );
                        });

                        // Active
                        row.col(|ui| {
                            ui.checkbox(&mut detector.timecut.active, "");
                        });

                        // Actions
                        row.col(|ui| {
                            if ui.button("Remove").clicked() {
                                indices_to_remove.push(index);
                            }
                        });
                    });
                }
            });

        // Remove detectors marked for deletion
        for &index in indices_to_remove.iter().rev() {
            self.detectors.remove(index);
        }
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

                // Common Range
                ui.label("Range:");
                ui.add(egui::DragValue::new(&mut range.0).speed(1.0).prefix("("));
                ui.add(egui::DragValue::new(&mut range.1).speed(1.0).suffix(")"));

                // Common Bins
                ui.label("Bins:");
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
                                egui::DragValue::new(&mut detector.gainmatch.a)
                                    .speed(0.01)
                                    .prefix("A: "),
                            );
                        });

                        // Coefficient B
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.gainmatch.active,
                                egui::DragValue::new(&mut detector.gainmatch.b)
                                    .speed(0.01)
                                    .prefix("B: "),
                            );
                        });

                        // Coefficient C
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.gainmatch.active,
                                egui::DragValue::new(&mut detector.gainmatch.c)
                                    .speed(0.01)
                                    .prefix("C: "),
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

                // Common Range
                ui.label("Range:");
                ui.add(egui::DragValue::new(&mut range.0).speed(1.0).prefix("("));
                ui.add(egui::DragValue::new(&mut range.1).speed(1.0).suffix(")"));

                // Common Bins
                ui.label("Bins:");
                ui.add(egui::DragValue::new(&mut bins).speed(1).prefix("Bins: "));
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
                                    .speed(0.01)
                                    .prefix("A: "),
                            );
                        });

                        // Coefficient B
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.energy_calibration.active,
                                egui::DragValue::new(&mut detector.energy_calibration.b)
                                    .speed(0.01)
                                    .prefix("B: "),
                            );
                        });

                        // Coefficient C
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.energy_calibration.active,
                                egui::DragValue::new(&mut detector.energy_calibration.c)
                                    .speed(0.01)
                                    .prefix("C: "),
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

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Add Detector").clicked() {
                self.add_detector(self.detectors.len());
            }
            ui.checkbox(&mut self.sps, "SE-SPS");
        });

        self.time_cut_ui(ui);
        self.gain_matching_ui(ui);
        self.energy_calibration_ui(ui);

        for detector in &mut self.detectors {
            detector.sps = self.sps;
        }
    }

    pub fn get_configs(&self) -> Configs {
        let mut configs = Configs::default();

        for detector in &self.detectors {
            configs.merge(detector.config());
        }

        configs
    }
}
/*************************** SE-SPS Custom Struct ***************************/
#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct SPSConfig {
    active: bool,
    xavg: Calibration,
    cuts: Cuts,
}

impl Default for SPSConfig {
    fn default() -> Self {
        Self {
            active: false,
            xavg: Calibration {
                a: 0.0,
                b: 1.0,
                c: 0.0,
                bins: 512,
                range: (0.0, 4096.0),
                active: false,
            },
            cuts: Cuts::default(),
        }
    }
}

impl SPSConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();

        ui.label("Calibration");

        self.xavg.ui(ui);

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

        // If there are no cuts, return the sps configs
        if active_cuts.is_empty() {
            self.sps_configs()
        } else {
            let mut updated_configs = Configs::default();

            let original_configs = self.sps_configs();

            for config in &original_configs.configs {
                match config {
                    Config::Hist1D(hist) => {
                        let mut cuts = hist.cuts.clone();

                        updated_configs.hist1d(
                            &hist.name.replacen("SE-SPS", "SE-SPS/No Cuts", 1),
                            &hist.column_name,
                            hist.range,
                            hist.bins,
                            Some(cuts.clone()),
                        );

                        cuts.merge(&active_cuts.clone());

                        updated_configs.hist1d(
                            &hist.name.replacen("SE-SPS", "SE-SPS/Cuts", 1),
                            &hist.column_name,
                            hist.range,
                            hist.bins,
                            Some(cuts.clone()),
                        );
                    }
                    Config::Hist2D(hist) => {
                        let mut cuts = hist.cuts.clone();

                        updated_configs.hist2d(
                            &hist.name.replacen("SE-SPS", "SE-SPS/No Cuts", 1),
                            &hist.x_column_name,
                            &hist.y_column_name,
                            hist.x_range,
                            hist.y_range,
                            hist.bins,
                            Some(cuts.clone()),
                        );

                        cuts.merge(&active_cuts.clone());

                        updated_configs.hist2d(
                            &hist.name.replacen("SE-SPS", "SE-SPS/Cuts", 1),
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

            updated_configs.columns = original_configs.columns.clone();

            updated_configs.cuts = original_configs.cuts.clone();
            updated_configs.cuts.merge(&active_cuts);

            updated_configs
        }
    }
}

/*

#[rustfmt::skip]
#[allow(clippy::all)]
pub fn catrina(h: &mut Histogrammer, lf: LazyFrame, detector_number: usize) {
    let i = detector_number;

    h.add_fill_hist1d(&format!("Catrina/CATRINA{i}/Energy"), &lf, &format!("CATRINA{i}Energy"), 4096, range);
    h.add_fill_hist2d(&format!("Catrina/CATRINA{i}/PSD vs Energy"), &lf, &format!("CATRINA{i}Energy"), &format!("CATRINA{i}PSD"), (512, 500), (range, (0.0, 1.0)));
}


*/

// pips1000(h, lf);
//....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....

// sps_histograms(h, lf.clone());

// For 52Cr(d,pg)53Cr
// let det_0_timecut = TimeCut { mean: -1155.6, low: -1158.0, high: -1152.0};
// let det_1_timecut = TimeCut { mean: -1153.9, low: -1159.0, high: -1147.0};
// let det_2_timecut = TimeCut { mean: -1154.0, low: -1158.0, high: -1151.0};
// let det_3_timecut = TimeCut { mean: -1152.0, low: -1158.0, high: -1148.0};
// let det_4_timecut = TimeCut { mean: -1123.1, low: -1127.0, high: -1118.0};

// // These values were gain match to detector 0
// // let det_0_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};
// // let det_1_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};
// // let det_2_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};
// // let det_3_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};
// // let det_4_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};

// let det_0_energy_calibration = EnergyCalibration { a: 0.0, b: 1.7551059351549314, c: -12.273506897222896, bins: 1024, range: (0.0, 16384.0) };
// let det_1_energy_calibration = EnergyCalibration { a: 0.0, b: 1.9510278378962256, c: -16.0245754973971, bins: 1024, range: (0.0, 16384.0) };
// let det_2_energy_calibration = EnergyCalibration { a: 0.0, b: 1.917190081718234, c: 16.430212777833802, bins: 1024, range: (0.0, 16384.0) };
// let det_3_energy_calibration = EnergyCalibration { a: 0.0, b: 1.6931918955746692, c: 12.021258506937766, bins: 1024, range: (0.0, 16384.0) };
// let det_4_energy_calibration = EnergyCalibration { a: 0.0, b: 1.6373533248536343, c: 13.091030061910748, bins: 1024, range: (0.0, 16384.0) };

// cebra(h, lf.clone(), 0, Some(det_0_timecut), None, Some(det_0_energy_calibration));
// cebra(h, lf.clone(), 1, Some(det_1_timecut), None, Some(det_1_energy_calibration));
// cebra(h, lf.clone(), 2, Some(det_2_timecut), None, Some(det_2_energy_calibration));
// cebra(h, lf.clone(), 3, Some(det_3_timecut), None, Some(det_3_energy_calibration));
// cebra(h, lf.clone(), 4, Some(det_4_timecut), None, Some(det_4_energy_calibration));

// #[rustfmt::skip]
// #[allow(clippy::all)]
// pub fn pips1000(h: &mut Histogrammer, lf: LazyFrame) {
//     let lf_pips = lf.with_columns( vec![
//         // ( ( col("PIPS1000Energy") - col("PIPS1000Short") )/ col("PIPS1000Energy") ).alias("PIPS1000PSD"),
//         (lit(-1.77049e-06)*col("PIPS1000Energy")*col("PIPS1000Energy") + lit(0.544755003513083)*col("PIPS1000Energy") + lit(-1.36822594543883)).alias("PIPS1000EnergyCalibrated") ]
//     );

//     h.add_fill_hist1d("PIPS1000/Energy", &lf_pips, "PIPS1000Energy", 16384, (0.0, 16384.0));
//     // h.add_fill_hist2d("PIPS1000: PSD", &lf_pips, "PIPS1000Energy", "PIPS1000PSD", (512, 500), (range, (0.0, 1.0)));
//     h.add_fill_hist1d("PIPS1000/Energy Calibrated", &lf_pips, "PIPS1000EnergyCalibrated", 600, (0.0, 1200.0));

// }
