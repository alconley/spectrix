use super::general::{Calibration, TimeCut};
use super::se_sps::SPSConfig;

use crate::histoer::configs::Configs;
use crate::histoer::cuts::{Cut, Cuts};
use crate::histoer::ui_helpers::precise_drag_value;

use std::collections::BTreeSet;
use std::f64::consts::PI;

use egui_extras::{Column, TableBuilder};

#[derive(Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct CeBrAConfig {
    pub active: bool,
    pub detectors: Vec<Cebr3>,
}

impl CeBrAConfig {
    fn common_detector_index(&self) -> Option<usize> {
        self.detectors
            .iter()
            .position(|detector| detector.active)
            .or_else(|| (!self.detectors.is_empty()).then_some(0))
    }

    fn add_detectors_from_columns(&mut self, column_names: &[String]) {
        let detector_numbers: BTreeSet<usize> = column_names
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
            .collect();

        if detector_numbers.is_empty() {
            log::warn!("No CeBrA detector columns found in the loaded selected-file columns.");
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
        let mut detector_to_remove = None;

        ui.horizontal(|ui| {
            ui.label("Detectors");

            let discover_response = ui
                .add_enabled(!column_names.is_empty(), egui::Button::new("Get Detectors"))
                .on_hover_text(
                    "Scan the loaded selected-file column names and add any missing CeBrA detectors inferred from columns like Cebra0Energy or Cebra3Time.",
                )
                .on_disabled_hover_text(
                    "Load the selected file columns first with 'Get Column Names' in the Processor menu.",
                );

            if discover_response.clicked() {
                self.add_detectors_from_columns(column_names);
            }
        });

        ui.label("Checked detectors stay visible below. Remove deletes the CeBrA detector configuration.");

        if self.detectors.is_empty() {
            ui.label("No CeBrA detectors loaded yet.");
        } else {
            // ui.horizontal_wrapped(|ui| {
            for (index, detector) in self.detectors.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut detector.active, format!("Cebra{}", detector.number))
                        .on_hover_text("Show or hide this detector's settings tables.");

                    let remove_response = ui.small_button("X").on_hover_text(
                        "Remove this detector and its saved CeBrA-specific settings.",
                    );

                    if remove_response.clicked() {
                        detector_to_remove = Some(index);
                    }
                });
            }
            // });
        }

        if let Some(index) = detector_to_remove {
            self.detectors.remove(index);
        }
    }

    pub fn sps_time_cut_ui(&mut self, ui: &mut egui::Ui) {
        let Some(common_detector_index) = self.common_detector_index() else {
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
            if index == common_detector_index {
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
                    if !detector.active {
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

    pub fn gain_matching_ui(&mut self, ui: &mut egui::Ui) {
        // Temporarily store the range and bins to avoid conflicting borrows
        let (common_range, common_bins) =
            if let Some(common_detector_index) = self.common_detector_index() {
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
                    if !detector.active {
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

    pub fn energy_calibration_ui(&mut self, ui: &mut egui::Ui) {
        // Temporarily store the range and bins to avoid conflicting borrows
        let (common_range, common_bins) =
            if let Some(common_detector_index) = self.common_detector_index() {
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
                    if !detector.active {
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
            ui.label("Add a CeBrA detector number to configure it.");
            return;
        }

        if !self.detectors.iter().any(|detector| detector.active) {
            ui.label("Check a detector above to show its settings.");
            return;
        }

        if sps_config.active {
            self.sps_time_cut_ui(ui);
        }
        self.gain_matching_ui(ui);
        self.energy_calibration_ui(ui);
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
pub struct PIPSTimeCuts {
    pub pips1000: TimeCut,
    pub pips500: TimeCut,
    pub pips300: TimeCut,
    pub pips100: TimeCut,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Cebr3 {
    pub number: usize,
    pub sps_timecut: TimeCut,
    pub gainmatch: Calibration,
    pub energy_calibration: Calibration,
    pub pips_timecuts: PIPSTimeCuts,
    pub active: bool,
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

    #[rustfmt::skip]
    #[expect(clippy::all)]
    pub fn cebr3_configs(&self, sps_config: &SPSConfig, main_cuts: &Option<Cuts>) -> Configs {

        if !self.active {
            return Configs::default();
        }

        let mut configs = Configs::default();

        let base_path = if main_cuts.is_none() { "No Cuts/CeBrA" } else { "Cuts/CeBrA" };

        let range = (0.0, 4096.0);
        let bins = 4096;

        let number = self.number;

        configs.hist1d(&format!("{base_path}/Cebra{number}/Cebra{number}Energy"), &format!("Cebra{number}Energy"), range, bins, &main_cuts);


        if self.gainmatch.active {
            configs.columns.push(self.gainmatch.new_column(&format!("Cebra{number}Energy"),&format!("Cebra{number}GainMatched")));
            configs.hist1d(&format!("{base_path}/Cebra{number}/Cebra{number} Gain Matched"), &format!("Cebra{number}GainMatched"), self.gainmatch.range, self.gainmatch.bins, &main_cuts); 
            configs.hist1d(&format!("{base_path}/CeBrA/Gain Matched"), &format!("Cebra{number}GainMatched"),  self.gainmatch.range, self.gainmatch.bins, &main_cuts); 
        }

        if self.energy_calibration.active {
            if self.gainmatch.active {
                configs.columns.push(self.energy_calibration.new_column(&format!("Cebra{number}GainMatched"),&format!("Cebra{number}EnergyCalibrated")));
            } else {
                configs.columns.push(self.energy_calibration.new_column(&format!("Cebra{number}Energy"),&format!("Cebra{number}EnergyCalibrated")));
            }
            configs.hist1d(&format!("{base_path}/Cebra{number}/Cebra{number} Energy Calibrated"), &format!("Cebra{number}EnergyCalibrated"), self.energy_calibration.range, self.energy_calibration.bins, &main_cuts);
            configs.hist1d(&format!("{base_path}/CeBrA/Energy Calibrated"), &format!("Cebra{number}EnergyCalibrated"), self.energy_calibration.range, self.energy_calibration.bins, &main_cuts);
        }

        if sps_config.active {
            let sps_tcut_mean = self.sps_timecut.mean;
            let sps_tcut_low = self.sps_timecut.low;
            let sps_tcut_high = self.sps_timecut.high;

            let sps_no_tcut_range = self.sps_timecut.no_cut_range;
            let sps_no_tcut_bins = self.sps_timecut.no_cut_bins;

            let sps_tcut_range = self.sps_timecut.range;
            let sps_tcut_bins = self.sps_timecut.bins;

            configs.columns.push((
                format!("Cebra{number}Time - ScintLeftTime"),
                format!("Cebra{number}RelTime"),
            ));

            configs.hist1d(&format!("{base_path}/Cebra{number}/Cebra{number}RelTime"), &format!("Cebra{number}RelTime"), sps_no_tcut_range, sps_no_tcut_bins, &main_cuts);
            configs.hist2d(&format!("{base_path}/Cebra{number}/Cebra{number}Energy v Xavg"), &format!("Xavg"), &format!("Cebra{number}Energy"), (-300.0, 300.0), (0.0, 4096.0), (600, 512), &main_cuts);
            configs.hist2d(&format!("{base_path}/Cebra{number}/Cebra{number}RelTime v Xavg"), &format!("Xavg"), &format!("Cebra{number}RelTime"), (-300.0, 300.0), sps_no_tcut_range, (600, sps_no_tcut_bins), &main_cuts);
            configs.hist2d(&format!("{base_path}/Cebra{number}/Theta v Cebra{number}RelTime"), &format!("Cebra{number}RelTime"), "Theta", sps_no_tcut_range, (0.0, PI/2.0), (sps_no_tcut_bins, 300), &main_cuts);

            if self.sps_timecut.active {

                configs.columns.push((format!("Cebra{number}RelTime - {sps_tcut_mean}"), format!("Cebra{number}RelTimeShifted")));
                configs.hist1d(&format!("{base_path}/Cebra{number}/Cebra{number}RelTimeShifted"), &format!("Cebra{number}RelTimeShifted"), sps_tcut_range, sps_tcut_bins, &main_cuts);
                configs.hist1d(&format!("{base_path}/CeBrA/CeBrARelTimeShifted"), &format!("Cebra{number}RelTimeShifted"), sps_tcut_range, sps_tcut_bins, &main_cuts);

                let cebra_time_cut = Cut::new_1d(&format!("Cebra{number} Time Cut"), &format!("Cebra{number}RelTime >= {sps_tcut_low} & Cebra{number}RelTime <= {sps_tcut_high}"));
                configs.cuts.add_cut(cebra_time_cut.clone());

                let tcut: Option<Cuts> = if let Some(mut main_cuts) = main_cuts.clone() {
                    main_cuts.add_cut(cebra_time_cut);
                    Some(main_cuts)
                } else {
                    Some(Cuts::new(vec![cebra_time_cut.clone()]))
                };

                configs.hist1d(&format!("{base_path}/Cebra{number}/Time Cut/Cebra{number}RelTime"), &format!("Cebra{number}RelTime"), sps_tcut_range, sps_tcut_bins, &tcut);
                configs.hist1d(&format!("{base_path}/Cebra{number}/Time Cut/Cebra{number}RelTimeShifted"), &format!("Cebra{number}RelTimeShifted"), (-50.0, 50.0), 100, &tcut);

                configs.hist1d(&format!("{base_path}/Cebra{number}/Time Cut/Cebra{number}Energy"), &format!("Cebra{number}Energy"), range, bins, &tcut);
                configs.hist2d(&format!("{base_path}/Cebra{number}/Time Cut/Cebra{number}Energy v Xavg"), &format!("Xavg"), &format!("Cebra{number}Energy"), (-300.0, 300.0), (0.0, 4096.0), (600, 512), &tcut);
                configs.hist2d(&format!("{base_path}/Cebra{number}/Time Cut/Cebra{number}Energy v X1"), &format!("X1"), &format!("Cebra{number}Energy"), (-300.0, 300.0), (0.0, 4096.0), (600, 512), &tcut);

                configs.hist1d(&format!("{base_path}/Cebra{number}/Time Cut/Xavg"), &format!("Xavg"), (-300.0, 300.0), 600, &tcut);

                if self.gainmatch.active {
                    configs.hist1d(&format!("{base_path}/Cebra{number}/Time Cut/Cebra{number} Gain Matched"), &format!("Cebra{number}GainMatched"), self.gainmatch.range, self.gainmatch.bins, &tcut);
                    configs.hist2d(&format!("{base_path}/Cebra{number}/Time Cut/Cebra{number} Gain Matched v Xavg"), &format!("Xavg"), &format!("Cebra{number}GainMatched"), (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), &tcut);
                    configs.hist2d(&format!("{base_path}/Cebra{number}/Time Cut/Cebra{number} Gain Matched v X1"), &format!("X1"), &format!("Cebra{number}GainMatched"), (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), &tcut);

                    configs.hist1d(&format!("{base_path}/CeBrA/Time Cut/CeBrA Gain Matched"), &format!("Cebra{number}GainMatched"), self.gainmatch.range, self.gainmatch.bins, &tcut);
                    configs.hist2d(&format!("{base_path}/CeBrA/Time Cut/CeBrA Gain Matched v Xavg"), &format!("Xavg"), &format!("Cebra{number}Energy"), (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), &tcut);
                    configs.hist2d(&format!("{base_path}/CeBrA/Time Cut/CeBrA Gain Matched v X1"), &format!("X1"), &format!("Cebra{number}Energy"), (-300.0, 300.0),  self.gainmatch.range, (600, self.gainmatch.bins), &tcut);   

                    if sps_config.xavg.active {
                        configs.hist2d(&format!("{base_path}/Cebra{number}/Time Cut/Cebra{number} v Xavg- Gain Matched"), &format!("XavgEnergyCalibrated"), &format!("Cebra{number}GainMatched"), sps_config.xavg.range, self.gainmatch.range, (sps_config.xavg.bins, self.gainmatch.bins), &tcut);
                        configs.hist2d(&format!("{base_path}/CeBrA/Time Cut/CeBrA v Xavg- Gain Matched"), &format!("XavgEnergyCalibrated"), &format!("Cebra{number}GainMatched"), sps_config.xavg.range, self.gainmatch.range, (sps_config.xavg.bins, self.gainmatch.bins), &tcut);
                    }
                }
                if self.energy_calibration.active {
                    configs.hist1d(&format!("{base_path}/Cebra{number}/Time Cut/Cebra{number} Energy Calibrated"), &format!("Cebra{number}EnergyCalibrated"), self.energy_calibration.range, self.energy_calibration.bins, &tcut);
                    configs.hist2d(&format!("{base_path}/Cebra{number}/Time Cut/Cebra{number} Energy Calibrated v Xavg"), &format!("Xavg"), &format!("Cebra{number}EnergyCalibrated"), (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), &tcut);
                    configs.hist2d(&format!("{base_path}/Cebra{number}/Time Cut/Cebra{number} Energy Calibrated v X1"), &format!("X1"), &format!("Cebra{number}EnergyCalibrated"), (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), &tcut);

                    if sps_config.xavg.active {
                        configs.hist2d(&format!("{base_path}/Cebra{number}/Time Cut/Cebra{number} v Xavg- Energy Calibrated"), &format!("XavgEnergyCalibrated"), &format!("Cebra{number}EnergyCalibrated"), sps_config.xavg.range, self.energy_calibration.range, (sps_config.xavg.bins, self.energy_calibration.bins), &tcut);
                        configs.hist2d(&format!("{base_path}/CeBrA/Time Cut/CeBrA v Xavg- Energy Calibrated"), &format!("XavgEnergyCalibrated"), &format!("Cebra{number}EnergyCalibrated"), sps_config.xavg.range, self.energy_calibration.range, (sps_config.xavg.bins, self.energy_calibration.bins), &tcut);
                    }

                    configs.hist1d(&format!("{base_path}/CeBrA/Time Cut/CeBrA Energy Calibrated"), &format!("Cebra{number}EnergyCalibrated"), self.energy_calibration.range, self.energy_calibration.bins, &tcut);
                    configs.hist2d(&format!("{base_path}/CeBrA/Time Cut/CeBrA Energy Calibrated v Xavg"), &format!("Xavg"), &format!("Cebra{number}EnergyCalibrated"), (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), &tcut);
                    configs.hist2d(&format!("{base_path}/CeBrA/Time Cut/CeBrA Energy Calibrated v X1"), &format!("X1"), &format!("Cebra{number}EnergyCalibrated"), (-300.0, 300.0), self.energy_calibration.range, (600, self.energy_calibration.bins), &tcut);
                }
            }
        }

        configs
    }
}
