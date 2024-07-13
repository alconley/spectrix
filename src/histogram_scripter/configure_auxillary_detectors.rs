use polars::prelude::*;
use std::collections::HashMap;

use super::configure_histograms::{Histo1dConfig, Histo2dConfig, HistoConfig};

// This with be an example with CeBrA
#[derive(Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct AuxillaryDetectors {
    pub cebra: Vec<CeBr3>,
}

impl AuxillaryDetectors {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if ui.button("Add CeBr3").clicked() {
            self.cebra.push(CeBr3::new(self.cebra.len()));
        }

        let mut to_remove = None;

        if self.cebra.is_empty() {
            ui.label("No Detectors Added");
        } else {
            egui::Grid::new("cebra_grid").striped(true).show(ui, |ui| {
                ui.label("Number");
                ui.label("Gain Match").on_hover_text("y = a*x^2 + b*x + c");
                ui.label("Energy Calibration")
                    .on_hover_text("y = a*x^2 + b*x + c");
                ui.label("Time Cuts").on_hover_text("min, max, centroid");
                ui.end_row();

                for (index, cebra) in self.cebra.iter_mut().enumerate() {
                    ui.label(cebra.number.to_string());
                    cebra.gain_match.ui(ui);
                    cebra.energy_calibration.ui(ui);
                    cebra.timecuts.ui(ui);

                    if ui.button("X").clicked() {
                        to_remove = Some(index);
                    }

                    ui.end_row();
                }
            });

            if let Some(to_remove) = to_remove {
                self.cebra.remove(to_remove);
            }
        }
    }

    pub fn get_column_names(&self) -> Vec<String> {
        let mut columns = vec![];
        for cebra in &self.cebra {
            columns.extend(cebra.columns());
        }

        columns
    }

    pub fn get_lf_names(&self) -> Vec<String> {
        let mut lfs = vec![];
        for cebra in &self.cebra {
            lfs.push(format!("Cebra{}TimeCut", cebra.number));
        }

        lfs
    }

    pub fn add_columns_to_lazyframe(&self, lazyframe: &LazyFrame) -> LazyFrame {
        let mut lazyframe = lazyframe.clone();
        for cebra in &self.cebra {
            lazyframe = cebra.add_columns_to_lazyframe(&lazyframe);
        }

        lazyframe
    }

    pub fn time_filterd_lazyframes(&self, lazyframe: LazyFrame) -> HashMap<String, LazyFrame> {
        let mut lfs = HashMap::new();
        for cebra in &self.cebra {
            lfs.insert(
                format!("Cebra{}TimeCut", cebra.number),
                cebra.time_filterd_lazyframe(lazyframe.clone()),
            );
        }

        lfs
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Calibration {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub bins: usize,
    pub range: (f64, f64),
}
impl Default for Calibration {
    fn default() -> Self {
        Self {
            a: 0.0,
            b: 1.0,
            c: 0.0,
            bins: 512,
            range: (0.0, 4096.0),
        }
    }
}
impl Calibration {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.a).speed(1.0));
            ui.add(egui::DragValue::new(&mut self.b).speed(1.0));
            ui.add(egui::DragValue::new(&mut self.c).speed(1.0));
        });
    }

    #[allow(clippy::all)]
    pub fn lazyframe_column(
        &self,
        lazyframe: LazyFrame,
        column: String,
        alias: String,
    ) -> LazyFrame {
        let lazyframe = lazyframe.with_column(
            (lit(self.a) * col(&column) * col(&column) + lit(self.b) * col(&column) + lit(self.c))
                .alias(&alias),
        );

        lazyframe
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct CeBr3TimeCut {
    pub min: f64,
    pub max: f64,
    pub centroid: f64,
}
impl Default for CeBr3TimeCut {
    fn default() -> Self {
        Self {
            min: -3000.0,
            max: 3000.0,
            centroid: 0.0,
        }
    }
}
impl CeBr3TimeCut {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.min).speed(1.0));
            ui.add(egui::DragValue::new(&mut self.max).speed(1.0));
            ui.add(egui::DragValue::new(&mut self.centroid).speed(1.0));
        });
    }
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct CeBr3 {
    pub number: usize,
    pub timecuts: CeBr3TimeCut,
    pub gain_match: Calibration,
    pub energy_calibration: Calibration,
}
impl CeBr3 {
    pub fn new(number: usize) -> Self {
        Self {
            number,
            timecuts: CeBr3TimeCut::default(),
            gain_match: Calibration::default(),
            energy_calibration: Calibration::default(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.gain_match.ui(ui);
        self.energy_calibration.ui(ui);
        self.timecuts.ui(ui);
    }

    pub fn columns(&self) -> Vec<String> {
        vec![
            format!("Cebra{}Energy", self.number),
            format!("Cebra{}Short", self.number),
            format!("Cebra{}Time", self.number),
            format!("Cebra{}Time_toScint", self.number),
            format!("Cebra{}Time_toScint_Shifted", self.number),
            format!("Cebra{}EnergyGainMatched", self.number),
            format!("Cebra{}EnergyCalibrated", self.number),
        ]
    }

    #[allow(clippy::all)]
    pub fn add_columns_to_lazyframe(&self, lazyframe: &LazyFrame) -> LazyFrame {
        let lazyframe = lazyframe.clone().with_columns(vec![
            (col(&format!("Cebra{}Time", self.number)) - col("ScintLeftTime"))
                .alias(&format!("Cebra{}Time_toScint", self.number)),
            (lit(self.gain_match.a)
                * col(&format!("Cebra{}Energy", self.number))
                * col(&format!("Cebra{}Energy", self.number))
                + lit(self.gain_match.b) * col(&format!("Cebra{}Energy", self.number))
                + lit(self.gain_match.c))
            .alias(&format!("Cebra{}EnergyGainMatched", self.number)),
        ]);

        let lazyframe = lazyframe.clone().with_columns(vec![
            (col(&format!("Cebra{}Time_toScint", self.number)) - lit(self.timecuts.centroid))
                .alias(&format!("Cebra{}Time_toScint_Shifted", self.number)),
            (lit(self.energy_calibration.a)
                * col(&format!("Cebra{}EnergyGainMatched", self.number))
                * col(&format!("Cebra{}EnergyGainMatched", self.number))
                + lit(self.energy_calibration.b)
                    * col(&format!("Cebra{}EnergyGainMatched", self.number))
                + lit(self.energy_calibration.c))
            .alias(&format!("Cebra{}EnergyCalibrated", self.number)),
        ]);

        lazyframe
    }

    #[allow(clippy::all)]
    pub fn time_filterd_lazyframe(&self, lazyframe: LazyFrame) -> LazyFrame {
        // time cut column name
        let column = format!("Cebra{}Time_toScint", self.number);

        let cebr3_time_filtered_lf = lazyframe
            .clone()
            .filter(col("ScintLeftTime").neq(lit(-1e6)))
            .filter(col("AnodeBackTime").neq(lit(-1e6)))
            .filter(col(&column).gt_eq(lit(self.timecuts.min)))
            .filter(col(&column).lt_eq(lit(self.timecuts.max)));

        cebr3_time_filtered_lf
    }

    #[allow(clippy::all)]
    pub fn histograms(&self) -> Vec<HistoConfig> {
        vec![
            // Raw lazyframes
            HistoConfig::Histo1d(Histo1dConfig {
                name: format!("Cebra{}Energy", self.number),
                lazyframe: "Raw".to_string(),
                column: format!("Cebra{}Energy", self.number),
                bins: 512,
                range: (0.0, 4096.0),
                calculate: true,
            }),
            HistoConfig::Histo1d(Histo1dConfig {
                name: format!("Cebra{}EnergyGainMatched: Raw", self.number),
                lazyframe: "Raw".to_string(),
                column: format!("Cebra{}EnergyGainMatched", self.number),
                bins: self.gain_match.bins,
                range: self.gain_match.range,
                calculate: true,
            }),
            HistoConfig::Histo1d(Histo1dConfig {
                name: format!("Cebra{}EnergyCalibrated: Raw", self.number),
                lazyframe: "Raw".to_string(),
                column: format!("Cebra{}EnergyCalibrated", self.number),
                bins: self.energy_calibration.bins,
                range: self.energy_calibration.range,
                calculate: true,
            }),
            HistoConfig::Histo1d(Histo1dConfig {
                name: format!("Cebra{}Time - ScintLeftTime: Raw", self.number),
                lazyframe: "Raw".to_string(),
                column: format!("Cebra{}Time_toScint", self.number),
                bins: 6400,
                range: (-3200.0, 3200.0),
                calculate: true,
            }),
            HistoConfig::Histo1d(Histo1dConfig {
                name: format!("Cebra{}Time - ScintLeftTime Shifted: Raw", self.number),
                lazyframe: "Raw".to_string(),
                column: format!("Cebra{}Time_toScint_Shifted", self.number),
                bins: 6400,
                range: (-3200.0, 3200.0),
                calculate: true,
            }),
            HistoConfig::Histo2d(Histo2dConfig {
                name: format!("Cebra{}Energy vs Time: Raw", self.number),
                lazyframe: "Raw".to_string(),
                x_column: format!("Cebra{}Time", self.number),
                y_column: format!("Cebra{}Energy", self.number),
                bins: (3600, 512),
                range: ((0.0, 1e9), (0.0, 4096.0)),
                calculate: false,
            }),
            HistoConfig::Histo2d(Histo2dConfig {
                name: format!("Cebra{}Energy vs Xavg: Raw", self.number),
                lazyframe: "Raw".to_string(),
                x_column: "Xavg".to_string(),
                y_column: format!("Cebra{}Energy", self.number),
                bins: (600, 512),
                range: ((-300.0, 300.0), (0.0, 4096.0)),
                calculate: false,
            }),
            // Time cut lazyframes
            HistoConfig::Histo1d(Histo1dConfig {
                name: format!("Cebra{}Energy: TimeCut", self.number),
                lazyframe: format!("Cebra{}TimeCut", self.number),
                column: format!("Cebra{}Energy", self.number),
                bins: 512,
                range: (0.0, 4096.0),
                calculate: true,
            }),
            HistoConfig::Histo1d(Histo1dConfig {
                name: format!("Cebra{}EnergyGainMatched: TimeCut", self.number),
                lazyframe: format!("Cebra{}TimeCut", self.number),
                column: format!("Cebra{}EnergyGainMatched", self.number),
                bins: self.gain_match.bins,
                range: self.gain_match.range,
                calculate: true,
            }),
            HistoConfig::Histo1d(Histo1dConfig {
                name: format!("Cebra{}EnergyCalibrated: TimeCut", self.number),
                lazyframe: format!("Cebra{}TimeCut", self.number),
                column: format!("Cebra{}EnergyCalibrated", self.number),
                bins: self.energy_calibration.bins,
                range: self.energy_calibration.range,
                calculate: true,
            }),
            HistoConfig::Histo1d(Histo1dConfig {
                name: format!("Cebra{}Time - ScintLeftTime: TimeCut", self.number),
                lazyframe: format!("Cebra{}TimeCut", self.number),
                column: format!("Cebra{}Time_toScint", self.number),
                bins: 6400,
                range: (-3200.0, 3200.0),
                calculate: false,
            }),
            HistoConfig::Histo1d(Histo1dConfig {
                name: format!("Cebra{}Time - ScintLeftTime Shifted: TimeCut", self.number),
                lazyframe: format!("Cebra{}TimeCut", self.number),
                column: format!("Cebra{}Time_toScint_Shifted", self.number),
                bins: 100,
                range: (-50.0, 50.0),
                calculate: true,
            }),
            HistoConfig::Histo2d(Histo2dConfig {
                name: format!("Cebra{}Energy vs Xavg: TimeCut", self.number),
                lazyframe: format!("Cebra{}TimeCut", self.number),
                x_column: "Xavg".to_string(),
                y_column: format!("Cebra{}Energy", self.number),
                bins: (600, 512),
                range: ((-300.0, 300.0), (0.0, 4096.0)),
                calculate: true,
            }),
            HistoConfig::Histo2d(Histo2dConfig {
                name: format!("Cebra{}EnergyGainMatched vs Xavg: TimeCut", self.number),
                lazyframe: format!("Cebra{}TimeCut", self.number),
                x_column: "Xavg".to_string(),
                y_column: format!("Cebra{}EnergyGainMatched", self.number),
                bins: (600, self.gain_match.bins),
                range: ((-300.0, 300.0), self.gain_match.range),
                calculate: true,
            }),
            HistoConfig::Histo2d(Histo2dConfig {
                name: format!("Cebra{}EnergyCalibrated vs Xavg: TimeCut", self.number),
                lazyframe: format!("Cebra{}TimeCut", self.number),
                x_column: "Xavg".to_string(),
                y_column: format!("Cebra{}EnergyCalibrated", self.number),
                bins: (600, self.energy_calibration.bins),
                range: ((-300.0, 300.0), self.energy_calibration.range),
                calculate: true,
            }),
        ]
    }
}
