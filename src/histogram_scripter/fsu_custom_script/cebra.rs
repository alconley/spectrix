use super::general::{Calibration, TimeCut};
use super::se_sps::SPSConfig;

use crate::histoer::configs::{ColumnGroup, Configs};
use crate::histoer::cuts::{Cut, Cuts};
use crate::histoer::ui_helpers::precise_drag_value;

use std::collections::BTreeSet;
use std::f64::consts::PI;

use egui_extras::{Column, TableBuilder};

const CEBRA_ENERGY_RANGE: (f64, f64) = (0.0, 4096.0);
const CEBRA_ENERGY_BINS: usize = 4096;
const CEBRA_PSD_RANGE: (f64, f64) = (-1.0, 1.0);
const CEBRA_PSD_BINS: usize = 512;
const CEBRA_ARRAY_ENERGY_ALIAS: &str = "CeBrAArrayEnergy";
const CEBRA_ARRAY_PSD_ALIAS: &str = "CeBrAArrayPsd";
const CEBRA_ARRAY_GAIN_MATCHED_ALIAS: &str = "CeBrAArrayGainMatched";
const CEBRA_ARRAY_ENERGY_CALIBRATED_ALIAS: &str = "CeBrAArrayEnergyCalibrated";
const CEBRA_ARRAY_REL_TIME_ALIAS: &str = "CeBrAArrayRelTime";
const CEBRA_ARRAY_REL_TIME_SHIFTED_ALIAS: &str = "CeBrAArrayRelTimeShifted";

#[derive(Clone, serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct CeBrAConfig {
    pub active: bool,
    pub detectors: Vec<Cebr3>,
}

impl CeBrAConfig {
    fn detected_numbers(column_names: &[String]) -> BTreeSet<usize> {
        column_names
            .iter()
            .filter_map(|column_name| {
                let suffix = column_name.strip_prefix("Cebra")?;
                let digits: String = suffix
                    .chars()
                    .take_while(|character| character.is_ascii_digit())
                    .collect();

                if digits.is_empty() {
                    None
                } else {
                    digits.parse::<usize>().ok()
                }
            })
            .collect()
    }

    fn has_column(column_names: &[String], column_name: &str) -> bool {
        column_names.iter().any(|name| name == column_name)
    }

    fn detector_available(detector: &Cebr3, column_names: &[String]) -> bool {
        Self::has_column(column_names, &detector.energy_column())
    }

    fn common_detector_index(&self, column_names: &[String]) -> Option<usize> {
        self.detectors
            .iter()
            .position(|detector| {
                detector.active && Self::detector_available(detector, column_names)
            })
            .or_else(|| {
                self.detectors
                    .iter()
                    .position(|detector| Self::detector_available(detector, column_names))
            })
    }

    fn ensure_detectors_from_columns(&mut self, column_names: &[String]) {
        let detector_numbers = Self::detected_numbers(column_names);

        if detector_numbers.is_empty() {
            return;
        }

        for detector_number in detector_numbers {
            if self
                .detectors
                .iter()
                .any(|detector| detector.number == detector_number)
            {
                continue;
            }

            let mut detector = Cebr3::new(detector_number);
            detector.active = true;
            self.detectors.push(detector);
        }

        self.detectors.sort_by_key(|detector| detector.number);
    }

    fn detector_management_ui(&mut self, ui: &mut egui::Ui, column_names: &[String]) {
        self.ensure_detectors_from_columns(column_names);

        ui.label("Detectors");
        ui.label("Loaded columns automatically define the CeBrA detector list. Uncheck a detector to skip its histograms.");

        if self.detectors.is_empty() {
            ui.label("No CeBrA detector columns were found in the loaded column list.");
        } else {
            ui.horizontal_wrapped(|ui| {
                for detector in &mut self.detectors {
                    let available = Self::detector_available(detector, column_names);
                    let label = if available {
                        format!("Cebra{}", detector.number)
                    } else {
                        format!("Cebra{} (not in loaded columns)", detector.number)
                    };

                    ui.add_enabled(available, egui::Checkbox::new(&mut detector.active, label))
                        .on_hover_text("Show or hide this detector's settings tables.");
                }
            });
        }
    }

    pub fn sps_time_cut_ui(&mut self, ui: &mut egui::Ui, column_names: &[String]) {
        let Some(common_detector_index) = self.common_detector_index(column_names) else {
            return;
        };

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("SE-SPS No Time Cuts");

            ui.separator();

            ui.add(
                egui::DragValue::new(
                    &mut self.detectors[common_detector_index]
                        .sps_timecut
                        .no_cut_range
                        .0,
                )
                .speed(1.0)
                .prefix("Range: ("),
            );
            ui.add(
                egui::DragValue::new(
                    &mut self.detectors[common_detector_index]
                        .sps_timecut
                        .no_cut_range
                        .1,
                )
                .speed(1.0)
                .suffix(")"),
            );

            ui.add(
                egui::DragValue::new(
                    &mut self.detectors[common_detector_index]
                        .sps_timecut
                        .no_cut_bins,
                )
                .speed(1)
                .prefix("Bins: "),
            );
        });

        ui.horizontal(|ui| {
            ui.label("SE-SPS Time Cuts");

            ui.separator();

            ui.add(
                egui::DragValue::new(
                    &mut self.detectors[common_detector_index].sps_timecut.range.0,
                )
                .speed(1.0)
                .prefix("Range: ("),
            );
            ui.add(
                egui::DragValue::new(
                    &mut self.detectors[common_detector_index].sps_timecut.range.1,
                )
                .speed(1.0)
                .suffix(")"),
            );

            ui.add(
                egui::DragValue::new(&mut self.detectors[common_detector_index].sps_timecut.bins)
                    .speed(1)
                    .prefix("Bins: "),
            );
        });

        //sync the time cut range and bins for all detectors
        let sps_timecut_range = self.detectors[common_detector_index].sps_timecut.range;
        let sps_timecut_bins = self.detectors[common_detector_index].sps_timecut.bins;

        let sps_no_timecut_range = self.detectors[common_detector_index]
            .sps_timecut
            .no_cut_range;
        let sps_no_timecut_bins = self.detectors[common_detector_index]
            .sps_timecut
            .no_cut_bins;

        for (index, detector) in self.detectors.iter_mut().enumerate() {
            if index == common_detector_index || !Self::detector_available(detector, column_names) {
                continue;
            }
            detector.sps_timecut.range = sps_timecut_range;
            detector.sps_timecut.bins = sps_timecut_bins;
            detector.sps_timecut.no_cut_range = sps_no_timecut_range;
            detector.sps_timecut.no_cut_bins = sps_no_timecut_bins;
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
                    if !detector.active || !Self::detector_available(detector, column_names) {
                        continue;
                    }

                    body.row(18.0, |mut row| {
                        // Detector Number
                        row.col(|ui| {
                            ui.label(format!("#{}", detector.number));
                        });

                        // Mean
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.sps_timecut.active,
                                precise_drag_value(&mut detector.sps_timecut.mean).speed(0.01),
                            );
                        });

                        // Low
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.sps_timecut.active,
                                precise_drag_value(&mut detector.sps_timecut.low).speed(0.01),
                            );
                        });

                        // High
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.sps_timecut.active,
                                precise_drag_value(&mut detector.sps_timecut.high).speed(0.01),
                            );
                        });

                        // Active
                        row.col(|ui| {
                            ui.checkbox(&mut detector.sps_timecut.active, "");
                        });
                    });
                }
            });
    }

    pub fn gain_matching_ui(&mut self, ui: &mut egui::Ui, column_names: &[String]) {
        // Temporarily store the range and bins to avoid conflicting borrows
        let (common_range, common_bins) =
            if let Some(common_detector_index) = self.common_detector_index(column_names) {
                ui.separator();

                let mut range = self.detectors[common_detector_index].gainmatch.range;
                let mut bins = self.detectors[common_detector_index].gainmatch.bins;

                ui.horizontal(|ui| {
                    ui.label("Gain Matching");

                    ui.separator();

                    ui.add(
                        precise_drag_value(&mut range.0)
                            .speed(1.0)
                            .prefix("Range: ("),
                    );
                    ui.add(precise_drag_value(&mut range.1).speed(1.0).suffix(")"));
                    ui.add(egui::DragValue::new(&mut bins).speed(1).prefix("Bins: "));
                });

                // Update the reference detector with the new range and bins
                self.detectors[common_detector_index].gainmatch.range = range;
                self.detectors[common_detector_index].gainmatch.bins = bins;

                (range, bins)
            } else {
                return; // No detectors to configure
            };

        // Update all other detectors with the common range and bins
        for detector in &mut self.detectors {
            if !Self::detector_available(detector, column_names) {
                continue;
            }
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
                    if !detector.active || !Self::detector_available(detector, column_names) {
                        continue;
                    }

                    body.row(18.0, |mut row| {
                        // Detector Number
                        row.col(|ui| {
                            ui.label(format!("#{}", detector.number));
                        });

                        // Coefficient A
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.gainmatch.active,
                                precise_drag_value(&mut detector.gainmatch.a).speed(0.01),
                            );
                        });

                        // Coefficient B
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.gainmatch.active,
                                precise_drag_value(&mut detector.gainmatch.b).speed(0.01),
                            );
                        });

                        // Coefficient C
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.gainmatch.active,
                                precise_drag_value(&mut detector.gainmatch.c).speed(0.01),
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

    pub fn energy_calibration_ui(&mut self, ui: &mut egui::Ui, column_names: &[String]) {
        // Temporarily store the range and bins to avoid conflicting borrows
        let (common_range, common_bins) =
            if let Some(common_detector_index) = self.common_detector_index(column_names) {
                ui.separator();

                let mut range = self.detectors[common_detector_index]
                    .energy_calibration
                    .range;
                let mut bins = self.detectors[common_detector_index]
                    .energy_calibration
                    .bins;

                ui.horizontal(|ui| {
                    ui.label("Energy Calibration");

                    ui.separator();

                    ui.add(
                        precise_drag_value(&mut range.0)
                            .speed(1.0)
                            .prefix(" Range: ("),
                    );
                    ui.add(precise_drag_value(&mut range.1).speed(1.0).suffix(")"));
                    ui.add(egui::DragValue::new(&mut bins).speed(1).prefix("Bins: "));

                    ui.label(format!("keV/bin: {:.2}", (range.1 - range.0) / bins as f64));
                });

                // Update the reference detector with the new range and bins
                self.detectors[common_detector_index]
                    .energy_calibration
                    .range = range;
                self.detectors[common_detector_index]
                    .energy_calibration
                    .bins = bins;

                (range, bins)
            } else {
                return; // No detectors to configure
            };

        // Update all other detectors with the common range and bins
        for detector in &mut self.detectors {
            if !Self::detector_available(detector, column_names) {
                continue;
            }
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
                    if !detector.active || !Self::detector_available(detector, column_names) {
                        continue;
                    }

                    body.row(18.0, |mut row| {
                        // Detector Number
                        row.col(|ui| {
                            ui.label(format!("#{}", detector.number));
                        });

                        // Coefficient A
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.energy_calibration.active,
                                precise_drag_value(&mut detector.energy_calibration.a).speed(0.01),
                            );
                        });

                        // Coefficient B
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.energy_calibration.active,
                                precise_drag_value(&mut detector.energy_calibration.b).speed(0.01),
                            );
                        });

                        // Coefficient C
                        row.col(|ui| {
                            ui.add_enabled(
                                detector.energy_calibration.active,
                                precise_drag_value(&mut detector.energy_calibration.c).speed(0.01),
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

    pub fn ui(&mut self, ui: &mut egui::Ui, sps_config: &SPSConfig, column_names: &[String]) {
        self.detector_management_ui(ui, column_names);

        if self.detectors.is_empty() {
            return;
        }

        if !self
            .detectors
            .iter()
            .any(|detector| detector.active && Self::detector_available(detector, column_names))
        {
            ui.label("Check a detector above to show its settings.");
            return;
        }

        if sps_config.active {
            self.sps_time_cut_ui(ui, column_names);
        }
        self.gain_matching_ui(ui, column_names);
        self.energy_calibration_ui(ui, column_names);
    }

    fn active_available_detectors<'a>(&'a self, column_names: &[String]) -> Vec<&'a Cebr3> {
        self.detectors
            .iter()
            .filter(|detector| detector.active && Self::detector_available(detector, column_names))
            .collect()
    }

    fn add_column_group(configs: &mut Configs, alias: &str, column_names: Vec<String>) {
        if column_names.is_empty() {
            return;
        }

        configs.column_groups.push(ColumnGroup {
            alias: alias.to_owned(),
            column_names,
        });
    }

    fn add_array_column_groups(
        configs: &mut Configs,
        active_detectors: &[&Cebr3],
        column_names: &[String],
        sps_config: &SPSConfig,
    ) {
        Self::add_column_group(
            configs,
            CEBRA_ARRAY_ENERGY_ALIAS,
            active_detectors
                .iter()
                .map(|detector| detector.energy_column())
                .collect(),
        );

        let psd_columns = active_detectors
            .iter()
            .filter(|detector| Self::has_column(column_names, &detector.short_column()))
            .map(|detector| detector.psd_column())
            .collect();
        Self::add_column_group(configs, CEBRA_ARRAY_PSD_ALIAS, psd_columns);

        let gain_matched_columns = active_detectors
            .iter()
            .filter(|detector| detector.gainmatch.active)
            .map(|detector| detector.gain_matched_column())
            .collect();
        Self::add_column_group(
            configs,
            CEBRA_ARRAY_GAIN_MATCHED_ALIAS,
            gain_matched_columns,
        );

        let energy_calibrated_columns = active_detectors
            .iter()
            .filter(|detector| detector.energy_calibration.active)
            .map(|detector| detector.energy_calibrated_column())
            .collect();
        Self::add_column_group(
            configs,
            CEBRA_ARRAY_ENERGY_CALIBRATED_ALIAS,
            energy_calibrated_columns,
        );

        if sps_config.active {
            let rel_time_columns = active_detectors
                .iter()
                .filter(|detector| {
                    Self::has_column(column_names, &detector.time_column())
                        && Self::has_column(column_names, "ScintLeftTime")
                })
                .map(|detector| detector.rel_time_column())
                .collect();
            Self::add_column_group(configs, CEBRA_ARRAY_REL_TIME_ALIAS, rel_time_columns);

            let rel_time_shifted_columns = active_detectors
                .iter()
                .filter(|detector| {
                    detector.sps_timecut.active
                        && Self::has_column(column_names, &detector.time_column())
                        && Self::has_column(column_names, "ScintLeftTime")
                })
                .map(|detector| detector.rel_time_shifted_column())
                .collect();
            Self::add_column_group(
                configs,
                CEBRA_ARRAY_REL_TIME_SHIFTED_ALIAS,
                rel_time_shifted_columns,
            );
        }
    }

    pub fn configs(
        &mut self,
        column_names: &[String],
        sps_config: &SPSConfig,
        main_cuts: &Option<Cuts>,
    ) -> Configs {
        self.ensure_detectors_from_columns(column_names);

        let active_detectors = self.active_available_detectors(column_names);
        if active_detectors.is_empty() {
            return Configs::default();
        }

        let mut configs = Configs::default();
        Self::add_array_column_groups(&mut configs, &active_detectors, column_names, sps_config);

        for detector in active_detectors {
            configs.merge(detector.cebr3_configs(column_names, sps_config, main_cuts));
        }

        configs
    }

    pub fn cr52dp_experiment(&mut self) {
        self.detectors.clear();

        let mut detector_0 = Cebr3::new(0);

        detector_0.active = true;
        detector_0.energy_calibration.active = true;
        detector_0.energy_calibration.a = 0.0;
        detector_0.energy_calibration.b = 1.7551059351549314;
        detector_0.energy_calibration.c = -12.273506897222896;

        detector_0.sps_timecut.active = true;
        detector_0.sps_timecut.mean = -1155.6;
        detector_0.sps_timecut.low = -1158.0;
        detector_0.sps_timecut.high = -1152.0;

        self.detectors.push(detector_0);

        let mut detector_1 = Cebr3::new(1);

        detector_1.active = true;
        detector_1.energy_calibration.active = true;
        detector_1.energy_calibration.a = 0.0;
        detector_1.energy_calibration.b = 1.9510278378962256;
        detector_1.energy_calibration.c = -16.0245754973971;

        detector_1.sps_timecut.active = true;
        detector_1.sps_timecut.mean = -1153.9;
        detector_1.sps_timecut.low = -1159.0;
        detector_1.sps_timecut.high = -1147.0;

        self.detectors.push(detector_1);

        let mut detector_2 = Cebr3::new(2);

        detector_2.active = true;
        detector_2.energy_calibration.active = true;
        detector_2.energy_calibration.a = 0.0;
        detector_2.energy_calibration.b = 1.917190081718234;
        detector_2.energy_calibration.c = 16.430212777833802;

        detector_2.sps_timecut.active = true;
        detector_2.sps_timecut.mean = -1154.0;
        detector_2.sps_timecut.low = -1158.0;
        detector_2.sps_timecut.high = -1151.0;

        self.detectors.push(detector_2);

        let mut detector_3 = Cebr3::new(3);

        detector_3.active = true;
        detector_3.energy_calibration.active = true;
        detector_3.energy_calibration.a = 0.0;
        detector_3.energy_calibration.b = 1.6931918955746692;
        detector_3.energy_calibration.c = 12.021258506937766;

        detector_3.sps_timecut.active = true;
        detector_3.sps_timecut.mean = -1152.0;
        detector_3.sps_timecut.low = -1158.0;
        detector_3.sps_timecut.high = -1148.0;

        self.detectors.push(detector_3);

        let mut detector_4 = Cebr3::new(4);

        detector_4.active = true;
        detector_4.energy_calibration.active = true;
        detector_4.energy_calibration.a = 0.0;
        detector_4.energy_calibration.b = 1.6373533248536343;
        detector_4.energy_calibration.c = 13.091030061910748;

        detector_4.sps_timecut.active = true;
        detector_4.sps_timecut.mean = -1123.1;
        detector_4.sps_timecut.low = -1127.0;
        detector_4.sps_timecut.high = -1118.0;

        self.detectors.push(detector_4);

        for detector in &mut self.detectors {
            detector.gainmatch.active = false;
            detector.energy_calibration.bins = 500;
            detector.energy_calibration.range = (0.0, 5500.0);
        }
    }
}

#[derive(Default, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct PIPSTimeCuts {
    pub pips1000: TimeCut,
    pub pips500: TimeCut,
    pub pips300: TimeCut,
    pub pips100: TimeCut,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Cebr3 {
    pub number: usize,
    pub sps_timecut: TimeCut,
    pub gainmatch: Calibration,
    pub energy_calibration: Calibration,
    pub pips_timecuts: PIPSTimeCuts,
    pub active: bool,
}

impl Default for Cebr3 {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Cebr3 {
    pub fn new(number: usize) -> Self {
        Self {
            number,
            sps_timecut: TimeCut::default(),
            gainmatch: Calibration::default(),
            energy_calibration: Calibration::default(),
            pips_timecuts: PIPSTimeCuts::default(),
            active: false,
        }
    }

    fn energy_column(&self) -> String {
        format!("Cebra{}Energy", self.number)
    }

    fn short_column(&self) -> String {
        format!("Cebra{}Short", self.number)
    }

    fn psd_column(&self) -> String {
        format!("Cebra{}PSD", self.number)
    }

    fn time_column(&self) -> String {
        format!("Cebra{}Time", self.number)
    }

    fn gain_matched_column(&self) -> String {
        format!("Cebra{}GainMatched", self.number)
    }

    fn energy_calibrated_column(&self) -> String {
        format!("Cebra{}EnergyCalibrated", self.number)
    }

    fn rel_time_column(&self) -> String {
        format!("Cebra{}RelTime", self.number)
    }

    fn rel_time_shifted_column(&self) -> String {
        format!("Cebra{}RelTimeShifted", self.number)
    }

    #[rustfmt::skip]
    #[expect(clippy::all)]
    pub fn cebr3_configs(&self, column_names: &[String], sps_config: &SPSConfig, main_cuts: &Option<Cuts>) -> Configs {

        if !self.active {
            return Configs::default();
        }

        let mut configs = Configs::default();

        let base_path = if main_cuts.is_none() {
            "CeBrA/Histograms"
        } else {
            "CeBrA/SE-SPS Cuts"
        };


        let energy_column = self.energy_column();
        if !column_names.iter().any(|column_name| column_name == &energy_column) {
            return configs;
        }

        let short_column = self.short_column();
        let psd_column = self.psd_column();
        let time_column = self.time_column();
        let gain_matched_column = self.gain_matched_column();
        let energy_calibrated_column = self.energy_calibrated_column();
        let rel_time_column = self.rel_time_column();
        let rel_time_shifted_column = self.rel_time_shifted_column();

        let detector_folder = format!("Detector {}", self.number);

        let number = self.number;

        configs.hist1d(
            &format!("{base_path}/CeBrA/Energy"),
            &energy_column,
            CEBRA_ENERGY_RANGE,
            CEBRA_ENERGY_BINS,
            &main_cuts,
        );

        configs.hist1d(
            &format!("{base_path}/{detector_folder}/Energy"),
            &energy_column,
            CEBRA_ENERGY_RANGE,
            CEBRA_ENERGY_BINS,
            &main_cuts,
        );


        if column_names.iter().any(|column_name| column_name == &short_column) {
            configs.columns.push((
                format!("({energy_column} - {short_column}) / {energy_column}"),
                psd_column.clone(),
            ));
            configs.hist2d(
                &format!("{base_path}/{detector_folder}/PSD v Energy"),
                &energy_column,
                &psd_column,
                CEBRA_ENERGY_RANGE,
                CEBRA_PSD_RANGE,
                (CEBRA_ENERGY_BINS, CEBRA_PSD_BINS),
                &main_cuts,
            );
            configs.hist2d(
                &format!("{base_path}/CeBrA/PSD v Energy"),
                &energy_column,
                &psd_column,
                CEBRA_ENERGY_RANGE,
                CEBRA_PSD_RANGE,
                (CEBRA_ENERGY_BINS, CEBRA_PSD_BINS),
                &main_cuts,
            );
        }


        if self.gainmatch.active {
            configs.columns.push(self.gainmatch.new_column(&energy_column, &gain_matched_column));
            configs.hist1d(
                &format!("{base_path}/{detector_folder}/Gain Matched/Energy"),
                &gain_matched_column,
                self.gainmatch.range,
                self.gainmatch.bins,
                &main_cuts,
            );
            configs.hist1d(
                &format!("{base_path}/CeBrA/Gain Matched/Energy"),
                &gain_matched_column,
                self.gainmatch.range,
                self.gainmatch.bins,
                &main_cuts,
            );
        }

        if self.energy_calibration.active {
            if self.gainmatch.active {
                configs.columns.push(self.energy_calibration.new_column(&gain_matched_column, &energy_calibrated_column));
            } else {
                configs.columns.push(self.energy_calibration.new_column(&energy_column, &energy_calibrated_column));
            }
            configs.hist1d(
                &format!("{base_path}/{detector_folder}/Energy Calibrated/Energy"),
                &energy_calibrated_column,
                self.energy_calibration.range,
                self.energy_calibration.bins,
                &main_cuts,
            );
            configs.hist1d(
                &format!("{base_path}/CeBrA/Energy Calibrated/Energy"),
                &energy_calibrated_column,
                self.energy_calibration.range,
                self.energy_calibration.bins,
                &main_cuts,
            );
        }

        if sps_config.active
            && column_names.iter().any(|column_name| column_name == &time_column)
            && column_names.iter().any(|column_name| column_name == "ScintLeftTime")
        {
            let sps_tcut_mean = self.sps_timecut.mean;
            let sps_tcut_low = self.sps_timecut.low;
            let sps_tcut_high = self.sps_timecut.high;

            let sps_no_tcut_range = self.sps_timecut.no_cut_range;
            let sps_no_tcut_bins = self.sps_timecut.no_cut_bins;

            let sps_tcut_range = self.sps_timecut.range;
            let sps_tcut_bins = self.sps_timecut.bins;

            configs.columns.push((
                format!("{time_column} - ScintLeftTime"),
                rel_time_column.clone(),
            ));

            configs.hist1d(&format!("{base_path}/{detector_folder}/RelTime"), &rel_time_column, sps_no_tcut_range, sps_no_tcut_bins, &main_cuts);
            configs.hist2d(&format!("{base_path}/{detector_folder}/Energy v Xavg"), "Xavg", &energy_column, (-300.0, 300.0), CEBRA_ENERGY_RANGE, (600, 512), &main_cuts);
            configs.hist2d(&format!("{base_path}/{detector_folder}/Energy v X1"), "X1", &energy_column, (-300.0, 300.0), CEBRA_ENERGY_RANGE, (600, 512), &main_cuts);
            configs.hist2d(&format!("{base_path}/{detector_folder}/RelTime v Xavg"), "Xavg", &rel_time_column, (-300.0, 300.0), sps_no_tcut_range, (600, sps_no_tcut_bins), &main_cuts);
            configs.hist2d(&format!("{base_path}/{detector_folder}/Theta v RelTime"), &rel_time_column, "Theta", sps_no_tcut_range, (0.0, PI/2.0), (sps_no_tcut_bins, 300), &main_cuts);

            configs.hist1d(&format!("{base_path}/CeBrA/RelTime"), &rel_time_column, sps_no_tcut_range, sps_no_tcut_bins, &main_cuts);
            configs.hist2d(&format!("{base_path}/CeBrA/Energy v Xavg"), "Xavg", &energy_column, (-300.0, 300.0), CEBRA_ENERGY_RANGE, (600, 512), &main_cuts);
            configs.hist2d(&format!("{base_path}/CeBrA/Energy v X1"), "X1", &energy_column, (-300.0, 300.0), CEBRA_ENERGY_RANGE, (600, 512), &main_cuts);
            configs.hist2d(&format!("{base_path}/CeBrA/RelTime v Xavg"), "Xavg", &rel_time_column, (-300.0, 300.0), sps_no_tcut_range, (600, sps_no_tcut_bins), &main_cuts);
            configs.hist2d(&format!("{base_path}/CeBrA/Theta v RelTime"), &rel_time_column, "Theta", sps_no_tcut_range, (0.0, PI/2.0), (sps_no_tcut_bins, 300), &main_cuts);

            if self.gainmatch.active {
                configs.hist2d(&format!("{base_path}/{detector_folder}/Gain Matched/Energy v Xavg"), "Xavg", &gain_matched_column, (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), &main_cuts);
                configs.hist2d(&format!("{base_path}/{detector_folder}/Gain Matched/Energy v X1"), "X1", &gain_matched_column, (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), &main_cuts);

                configs.hist2d(&format!("{base_path}/CeBrA/Gain Matched/Energy v Xavg"), "Xavg", &gain_matched_column, (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), &main_cuts);
                configs.hist2d(&format!("{base_path}/CeBrA/Gain Matched/Energy v X1"), "X1", &gain_matched_column, (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), &main_cuts);
            }

            if self.energy_calibration.active {
                configs.hist2d(&format!("{base_path}/{detector_folder}/Energy Calibrated/Energy v Xavg"), "Xavg", &energy_calibrated_column, (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), &main_cuts);
                configs.hist2d(&format!("{base_path}/{detector_folder}/Energy Calibrated/Energy v X1"), "X1", &energy_calibrated_column, (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), &main_cuts);
                configs.hist2d(&format!("{base_path}/CeBrA/Energy Calibrated/Energy v Xavg"), "Xavg", &energy_calibrated_column, (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), &main_cuts);
                configs.hist2d(&format!("{base_path}/CeBrA/Energy Calibrated/Energy v X1"), "X1", &energy_calibrated_column, (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), &main_cuts);
            }

            if self.sps_timecut.active {

                configs.columns.push((format!("{rel_time_column} - {sps_tcut_mean}"), rel_time_shifted_column.clone()));
                configs.hist1d(&format!("{base_path}/{detector_folder}/RelTimeShifted"), &rel_time_shifted_column, sps_tcut_range, sps_tcut_bins, &main_cuts);
                configs.hist1d(&format!("{base_path}/CeBrA/RelTimeShifted"), &rel_time_shifted_column, sps_tcut_range, sps_tcut_bins, &main_cuts);

                let cebra_time_cut = Cut::new_1d(&format!("Cebra{number} Time Cut"), &format!("{rel_time_column} >= {sps_tcut_low} & {rel_time_column} <= {sps_tcut_high}"));
                configs.cuts.add_cut(cebra_time_cut.clone());

                let tcut: Option<Cuts> = if let Some(mut main_cuts) = main_cuts.clone() {
                    main_cuts.add_cut(cebra_time_cut);
                    Some(main_cuts)
                } else {
                    Some(Cuts::new(vec![cebra_time_cut.clone()]))
                };

                configs.hist1d(&format!("{base_path}/{detector_folder}/Time Cut/RelTime"), &rel_time_column, sps_tcut_range, sps_tcut_bins, &tcut);
                configs.hist1d(&format!("{base_path}/CeBrA/Time Cut/RelTime"), &rel_time_column, sps_tcut_range, sps_tcut_bins, &tcut);
                configs.hist1d(&format!("{base_path}/{detector_folder}/Time Cut/RelTimeShifted"), &rel_time_shifted_column, (-50.0, 50.0), 100, &tcut);
                configs.hist1d(&format!("{base_path}/CeBrA/Time Cut/RelTimeShifted"), &rel_time_shifted_column, (-50.0, 50.0), 100, &tcut);

                configs.hist1d(&format!("{base_path}/{detector_folder}/Time Cut/Energy"), &energy_column, CEBRA_ENERGY_RANGE, CEBRA_ENERGY_BINS, &tcut);
                configs.hist1d(&format!("{base_path}/CeBrA/Time Cut/Energy"), &energy_column, CEBRA_ENERGY_RANGE, CEBRA_ENERGY_BINS, &tcut);
                configs.hist2d(&format!("{base_path}/{detector_folder}/Time Cut/Energy v Xavg"), "Xavg", &energy_column, (-300.0, 300.0), CEBRA_ENERGY_RANGE, (600, 512), &tcut);
                configs.hist2d(&format!("{base_path}/CeBrA/Time Cut/Energy v Xavg"), "Xavg", &energy_column, (-300.0, 300.0), CEBRA_ENERGY_RANGE, (600, 512), &tcut);
                configs.hist2d(&format!("{base_path}/{detector_folder}/Time Cut/Energy v X1"), "X1", &energy_column, (-300.0, 300.0), CEBRA_ENERGY_RANGE, (600, 512), &tcut);
                configs.hist2d(&format!("{base_path}/CeBrA/Time Cut/Energy v X1"), "X1", &energy_column, (-300.0, 300.0), CEBRA_ENERGY_RANGE, (600, 512), &tcut);

                if self.gainmatch.active {
                    configs.hist1d(&format!("{base_path}/{detector_folder}/Gain Matched/Time Cut/Energy"), &gain_matched_column, self.gainmatch.range, self.gainmatch.bins, &tcut);
                    configs.hist2d(&format!("{base_path}/{detector_folder}/Gain Matched/Time Cut/Energy v Xavg"), "Xavg", &gain_matched_column, (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), &tcut);
                    configs.hist2d(&format!("{base_path}/{detector_folder}/Gain Matched/Time Cut/Energy v X1"), "X1", &gain_matched_column, (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), &tcut);

                    configs.hist1d(&format!("{base_path}/CeBrA/Gain Matched/Time Cut/Energy"), &gain_matched_column, self.gainmatch.range, self.gainmatch.bins, &tcut);
                    configs.hist2d(&format!("{base_path}/CeBrA/Gain Matched/Time Cut/Energy v Xavg"), "Xavg", &gain_matched_column, (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), &tcut);
                    configs.hist2d(&format!("{base_path}/CeBrA/Gain Matched/Time Cut/Energy v X1"), "X1", &gain_matched_column, (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), &tcut);
                }
                if self.energy_calibration.active {
                    configs.hist1d(&format!("{base_path}/{detector_folder}/Energy Calibrated/Time Cut/Energy"), &energy_calibrated_column, self.energy_calibration.range, self.energy_calibration.bins, &tcut);
                    configs.hist2d(&format!("{base_path}/{detector_folder}/Energy Calibrated/Time Cut/Energy v Xavg"), "Xavg", &energy_calibrated_column, (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), &tcut);
                    configs.hist2d(&format!("{base_path}/{detector_folder}/Energy Calibrated/Time Cut/Energy v X1"), "X1", &energy_calibrated_column, (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), &tcut);

                    configs.hist1d(&format!("{base_path}/CeBrA/Energy Calibrated/Time Cut/Energy"), &energy_calibrated_column, self.energy_calibration.range, self.energy_calibration.bins, &tcut);
                    configs.hist2d(&format!("{base_path}/CeBrA/Energy Calibrated/Time Cut/Energy v Xavg"), "Xavg", &energy_calibrated_column, (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), &tcut);
                    configs.hist2d(&format!("{base_path}/CeBrA/Energy Calibrated/Time Cut/Energy v X1"), "X1", &energy_calibrated_column, (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), &tcut);
                }
            }
        }

        configs
    }
}
