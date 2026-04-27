use crate::histoer::configs::Configs;
use crate::histoer::cuts::{Cut, Cuts};
use crate::histoer::ui_helpers::precise_drag_value;

use std::collections::BTreeSet;

fn default_energy_range() -> (f64, f64) {
    (0.0, 4096.0)
}

fn default_energy_bins() -> usize {
    4096
}

fn default_psd_bins() -> usize {
    512
}

fn default_tof_range() -> (f64, f64) {
    (-3200.0, 3200.0)
}

fn default_tof_bins() -> usize {
    6400
}

#[derive(Clone, Copy)]
struct CATRiNAHistogramSettings {
    energy_range: (f64, f64),
    energy_bins: usize,
    psd_bins: usize,
    tof_range: (f64, f64),
    tof_bins: usize,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct CATRiNAConfig {
    pub active: bool,
    #[serde(default = "default_energy_range")]
    pub energy_range: (f64, f64),
    #[serde(default = "default_energy_bins")]
    pub energy_bins: usize,
    #[serde(default = "default_psd_bins")]
    pub psd_bins: usize,
    #[serde(default = "default_tof_range")]
    pub tof_range: (f64, f64),
    #[serde(default = "default_tof_bins")]
    pub tof_bins: usize,
    pub detectors: Vec<CATRiNADetector>,
}

impl CATRiNAConfig {
    fn parse_detector_number(column_name: &str) -> Option<usize> {
        let suffix = column_name
            .strip_prefix("CATRINA")
            .or_else(|| column_name.strip_prefix("CATRiNA"))?;

        let digits: String = suffix
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect();

        if digits.is_empty() {
            None
        } else {
            digits.parse::<usize>().ok()
        }
    }

    fn add_detectors_from_columns(&mut self, column_names: &[String]) {
        let detector_numbers: BTreeSet<usize> = column_names
            .iter()
            .filter_map(|column_name| Self::parse_detector_number(column_name))
            .collect();

        if detector_numbers.is_empty() {
            log::warn!("No CATRiNA detector columns found in the loaded selected-file columns.");
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

            let mut detector = CATRiNADetector::new(detector_number);
            detector.active = true;
            self.detectors.push(detector);
        }

        self.detectors.sort_by_key(|detector| detector.number);
    }

    fn detector_management_ui(&mut self, ui: &mut egui::Ui, column_names: &[String]) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Detectors");

            let discover_response = ui
                .add_enabled(!column_names.is_empty(), egui::Button::new("Get Detectors"))
                .on_hover_text(
                    "Scan the loaded selected-file column names and add any missing CATRiNA detectors inferred from columns like CATRINA0Energy or CATRINA3PSD.",
                )
                .on_disabled_hover_text(
                    "Load the selected file columns first with 'Get Column Names' in the Processor menu.",
                );

            if discover_response.clicked() {
                self.add_detectors_from_columns(column_names);
            }
        });

        ui.horizontal_wrapped(|ui| {
            ui.label("Checked detectors stay visible below. Remove deletes the CATRiNA detector configuration.");
            if self.detectors.is_empty() {
                ui.label("No CATRiNA detectors loaded yet.");
            } else {
                for detector in &mut self.detectors {
                    ui.checkbox(&mut detector.active, format!("CATRINA{}", detector.number))
                        .on_hover_text("Show or hide this detector's histogram generation.");
                }
            }
        });
    }

    fn histogram_settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Energy");
            ui.add(
                precise_drag_value(&mut self.energy_range.0)
                    .speed(1.0)
                    .prefix("Range: ("),
            );
            ui.add(
                precise_drag_value(&mut self.energy_range.1)
                    .speed(1.0)
                    .suffix(")"),
            );
            ui.add(
                egui::DragValue::new(&mut self.energy_bins)
                    .speed(1)
                    .prefix("Bins: "),
            );
        });

        ui.horizontal(|ui| {
            ui.label("PSD");
            ui.label("Range: (-1, 1)");
            ui.add(
                egui::DragValue::new(&mut self.psd_bins)
                    .speed(1)
                    .prefix("Bins: "),
            );
        });

        ui.horizontal(|ui| {
            ui.label("ToF");
            ui.add(
                precise_drag_value(&mut self.tof_range.0)
                    .speed(1.0)
                    .prefix("Range: ("),
            );
            ui.add(
                precise_drag_value(&mut self.tof_range.1)
                    .speed(1.0)
                    .suffix(")"),
            );
            ui.add(
                egui::DragValue::new(&mut self.tof_bins)
                    .speed(1)
                    .prefix("Bins: "),
            );
        });
    }

    fn histogram_settings(&self) -> CATRiNAHistogramSettings {
        CATRiNAHistogramSettings {
            energy_range: self.energy_range,
            energy_bins: self.energy_bins,
            psd_bins: self.psd_bins,
            tof_range: self.tof_range,
            tof_bins: self.tof_bins,
        }
    }

    pub fn configs(&self, column_names: &[String], main_cuts: &Option<Cuts>) -> Configs {
        let mut configs = Configs::default();
        let settings = self.histogram_settings();

        for detector in &self.detectors {
            if detector.active {
                configs.merge(detector.catrina_configs(column_names, &settings, main_cuts));
            }
        }

        configs
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, column_names: &[String]) {
        self.detector_management_ui(ui, column_names);

        if self.detectors.is_empty() {
            ui.label("Load CATRiNA detector columns to configure histogram generation.");
            return;
        }

        self.histogram_settings_ui(ui);

        if !self.detectors.iter().any(|detector| detector.active) {
            ui.label("Check a detector above to generate its histograms.");
        }
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct CATRiNADetector {
    pub number: usize,
    pub active: bool,
}

impl CATRiNADetector {
    pub fn new(number: usize) -> Self {
        Self {
            number,
            active: false,
        }
    }

    fn has_column(column_names: &[String], column_name: &str) -> bool {
        column_names.iter().any(|existing| existing == column_name)
    }

    fn catrina_configs(
        &self,
        column_names: &[String],
        settings: &CATRiNAHistogramSettings,
        main_cuts: &Option<Cuts>,
    ) -> Configs {
        if !self.active {
            return Configs::default();
        }

        let mut configs = Configs::default();
        let base_path = if main_cuts.is_none() {
            "No Cuts/CATRiNA"
        } else {
            "Cuts/CATRiNA"
        };

        let detector_folder = format!("{base_path}/Detectors/CATRiNA {}", self.number);
        let tof_folder = format!("{base_path}/ToF");
        let energy_column = format!("CATRINA{}Energy", self.number);
        let psd_column = format!("CATRINA{}PSD", self.number);
        let time_column = format!("CATRINA{}Time", self.number);
        let tof_column = format!("ToF_{}", self.number);

        if Self::has_column(column_names, &energy_column) {
            configs.hist1d(
                &format!("{detector_folder}/Energy"),
                &energy_column,
                settings.energy_range,
                settings.energy_bins,
                main_cuts,
            );
        }

        if Self::has_column(column_names, &energy_column)
            && Self::has_column(column_names, &psd_column)
        {
            configs.hist2d(
                &format!("{detector_folder}/PSD v Energy"),
                &energy_column,
                &psd_column,
                settings.energy_range,
                (-1.0, 1.0),
                (settings.energy_bins, settings.psd_bins),
                main_cuts,
            );
        }

        if Self::has_column(column_names, &energy_column)
            && Self::has_column(column_names, &time_column)
            && Self::has_column(column_names, "RF")
        {
            configs
                .columns
                .push((format!("{time_column} - RF"), tof_column.clone()));

            let valid_tof_cut = Cut::new_1d(
                &format!("Valid ToF {}", self.number),
                &format!("{energy_column} > 0.0"),
            );
            configs.cuts.add_cut(valid_tof_cut.clone());

            let tof_cuts = if let Some(mut main_cuts) = main_cuts.clone() {
                main_cuts.add_cut(valid_tof_cut);
                Some(main_cuts)
            } else {
                Some(Cuts::new(vec![valid_tof_cut.clone()]))
            };

            configs.hist1d(
                &format!("{detector_folder}/{tof_column}"),
                &tof_column,
                settings.tof_range,
                settings.tof_bins,
                &tof_cuts,
            );
            configs.hist1d(
                &format!("{tof_folder}/{tof_column}"),
                &tof_column,
                settings.tof_range,
                settings.tof_bins,
                &tof_cuts,
            );
        }

        configs
    }
}

impl Default for CATRiNADetector {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Default for CATRiNAConfig {
    fn default() -> Self {
        Self {
            active: false,
            energy_range: default_energy_range(),
            energy_bins: default_energy_bins(),
            psd_bins: default_psd_bins(),
            tof_range: default_tof_range(),
            tof_bins: default_tof_bins(),
            detectors: Vec::new(),
        }
    }
}
