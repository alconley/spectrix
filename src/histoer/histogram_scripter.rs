use super::histogrammer::Histogrammer;
use polars::prelude::*;
use std::collections::HashMap;

#[derive(Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct LazyFrameInfo {
    pub name: String,
    pub columns: Vec<String>,
}

pub struct LazyFrames {
    pub lfs: HashMap<String, LazyFrame>,
}

impl Default for LazyFrames {
    fn default() -> Self {
        Self::new()
    }
}

impl LazyFrames {
    pub fn new() -> Self {
        Self {
            lfs: HashMap::new(),
        }
    }

    pub fn create_lfs(&mut self, lf: LazyFrame) -> &Self {
        let lf = lf.with_columns(vec![
            (col("DelayFrontRightEnergy") + col("DelayFrontLeftEnergy") / lit(2.0))
                .alias("DelayFrontAverageEnergy"),
            (col("DelayBackRightEnergy") + col("DelayBackLeftEnergy") / lit(2.0))
                .alias("DelayBackAverageEnergy"),
            (col("DelayFrontLeftTime") - col("AnodeFrontTime"))
                .alias("DelayFrontLeftTime_AnodeFrontTime"),
            (col("DelayFrontRightTime") - col("AnodeFrontTime"))
                .alias("DelayFrontRightTime_AnodeFrontTime"),
            (col("DelayBackLeftTime") - col("AnodeFrontTime"))
                .alias("DelayBackLeftTime_AnodeFrontTime"),
            (col("DelayBackRightTime") - col("AnodeFrontTime"))
                .alias("DelayBackRightTime_AnodeFrontTime"),
            (col("DelayFrontLeftTime") - col("AnodeBackTime"))
                .alias("DelayFrontLeftTime_AnodeBackTime"),
            (col("DelayFrontRightTime") - col("AnodeBackTime"))
                .alias("DelayFrontRightTime_AnodeBackTime"),
            (col("DelayBackLeftTime") - col("AnodeBackTime"))
                .alias("DelayBackLeftTime_AnodeBackTime"),
            (col("DelayBackRightTime") - col("AnodeBackTime"))
                .alias("DelayBackRightTime_AnodeBackTime"),
            (col("AnodeFrontTime") - col("AnodeBackTime")).alias("AnodeFrontTime_AnodeBackTime"),
            (col("AnodeBackTime") - col("AnodeFrontTime")).alias("AnodeBackTime_AnodeFrontTime"),
            (col("AnodeFrontTime") - col("ScintLeftTime")).alias("AnodeFrontTime_ScintLeftTime"),
            (col("AnodeBackTime") - col("ScintLeftTime")).alias("AnodeBackTime_ScintLeftTime"),
            (col("DelayFrontLeftTime") - col("ScintLeftTime"))
                .alias("DelayFrontLeftTime_ScintLeftTime"),
            (col("DelayFrontRightTime") - col("ScintLeftTime"))
                .alias("DelayFrontRightTime_ScintLeftTime"),
            (col("DelayBackLeftTime") - col("ScintLeftTime"))
                .alias("DelayBackLeftTime_ScintLeftTime"),
            (col("DelayBackRightTime") - col("ScintLeftTime"))
                .alias("DelayBackRightTime_ScintLeftTime"),
            (col("ScintRightTime") - col("ScintLeftTime")).alias("ScintRightTime_ScintLeftTime"),
        ]);

        let lf_bothplanes = lf
            .clone()
            .filter(col("X1").neq(lit(-1e6)))
            .filter(col("X2").neq(lit(-1e6)));
        let lf_only_x1_plane = lf
            .clone()
            .filter(col("X1").neq(lit(-1e6)))
            .filter(col("X2").eq(lit(-1e6)));
        let lf_only_x2_plane = lf
            .clone()
            .filter(col("X2").neq(lit(-1e6)))
            .filter(col("X1").eq(lit(-1e6)));

        self.lfs.insert("Raw".to_string(), lf);
        self.lfs.insert("BothPlanes".to_string(), lf_bothplanes);
        self.lfs.insert("OnlyX1Plane".to_string(), lf_only_x1_plane);
        self.lfs.insert("OnlyX2Plane".to_string(), lf_only_x2_plane);

        self
    }

    pub fn columns(&self, number_of_cebr3: usize) -> Vec<String> {
        let main_sps_columns = vec![
            "AnodeFrontEnergy",
            "AnodeFrontShort",
            "AnodeFrontTime",
            "AnodeBackEnergy",
            "AnodeBackShort",
            "AnodeBackTime",
            "ScintLeftEnergy",
            "ScintLeftShort",
            "ScintLeftTime",
            "ScintRightEnergy",
            "ScintRightShort",
            "ScintRightTime",
            "CathodeEnergy",
            "CathodeShort",
            "CathodeTime",
            "DelayFrontLeftEnergy",
            "DelayFrontLeftShort",
            "DelayFrontLeftTime",
            "DelayFrontRightEnergy",
            "DelayFrontRightShort",
            "DelayFrontRightTime",
            "DelayBackLeftEnergy",
            "DelayBackLeftShort",
            "DelayBackLeftTime",
            "DelayBackRightEnergy",
            "DelayBackRightShort",
            "DelayBackRightTime",
            "X1",
            "X2",
            "Xavg",
            "Theta",
        ];

        let extra_sps_columns = vec![
            "DelayFrontAverageEnergy",
            "DelayBackAverageEnergy",
            "DelayFrontLeftTime_AnodeFrontTime",
            "DelayFrontRightTime_AnodeFrontTime",
            "DelayBackLeftTime_AnodeFrontTime",
            "DelayBackRightTime_AnodeFrontTime",
            "DelayFrontLeftTime_AnodeBackTime",
            "DelayFrontRightTime_AnodeBackTime",
            "DelayBackLeftTime_AnodeBackTime",
            "DelayBackRightTime_AnodeBackTime",
            "AnodeFrontTime_AnodeBackTime",
            "AnodeBackTime_AnodeFrontTime",
            "AnodeFrontTime_ScintLeftTime",
            "AnodeBackTime_ScintLeftTime",
            "DelayFrontLeftTime_ScintLeftTime",
            "DelayFrontRightTime_ScintLeftTime",
            "DelayBackLeftTime_ScintLeftTime",
            "DelayBackRightTime_ScintLeftTime",
            "ScintRightTime_ScintLeftTime",
        ];

        let mut cebra_columns = vec![];
        for i in 0..number_of_cebr3 {
            cebra_columns.push(format!("Cebra{}Energy", i));
            cebra_columns.push(format!("Cebra{}Short", i));
            cebra_columns.push(format!("Cebra{}Time", i));
            cebra_columns.push(format!("Cebra{}Time_toScint", i));
        }

        let mut columns = main_sps_columns
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();
        columns.extend(extra_sps_columns.iter().map(|&s| s.to_string()));
        columns.extend(cebra_columns);

        columns
    }

    pub fn get_lf(&self, key: &str) -> Option<&LazyFrame> {
        self.lfs.get(key)
    }

    fn get_column_names_from_lazyframe(lazyframe: &LazyFrame) -> Vec<String> {
        let lf: LazyFrame = lazyframe.clone().limit(1);
        let df: DataFrame = lf.collect().unwrap();
        let columns: Vec<String> = df
            .get_column_names_owned()
            .into_iter()
            .map(|name| name.to_string())
            .collect();

        columns
    }

    pub fn get_lazyframe_info(&self) -> Vec<LazyFrameInfo> {
        let mut lazyframe_info = Vec::new();
        for (name, lf) in &self.lfs {
            let columns = Self::get_column_names_from_lazyframe(lf);
            lazyframe_info.push(LazyFrameInfo {
                name: name.clone(),
                columns,
            });
        }

        lazyframe_info
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Histo1dConfig {
    pub name: String,
    pub lazyframe: String,
    pub column: String,
    pub bins: usize,
    pub range: (f64, f64),
    pub calculate: bool,
}

impl Default for Histo1dConfig {
    fn default() -> Self {
        Self {
            name: "Xavg".to_string(),
            lazyframe: "Raw".to_string(),
            column: "Xavg".to_string(),
            bins: 600,
            range: (-300.0, 300.0),
            calculate: true,
        }
    }
}

impl Histo1dConfig {
    pub fn ui(&mut self, ui: &mut egui::Ui, lazyframe_info: Vec<LazyFrameInfo>) {
        ui.text_edit_singleline(&mut self.name);

        egui::ComboBox::from_id_source(self.name.clone())
            .selected_text(&self.lazyframe)
            .show_ui(ui, |ui| {
                for lf in &lazyframe_info {
                    ui.selectable_value(&mut self.lazyframe, lf.name.clone(), lf.name.clone());
                }
            });

        ui.text_edit_singleline(&mut self.column);

        ui.label("");

        ui.add(
            egui::DragValue::new(&mut self.bins)
                .speed(1.0)
                .range(1..=usize::MAX),
        );

        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.range.0)
                    .speed(1.0)
                    .prefix("(")
                    .suffix(","),
            );
            ui.add(
                egui::DragValue::new(&mut self.range.1)
                    .speed(1.0)
                    .suffix(")"),
            );
        });

        ui.checkbox(&mut self.calculate, "");
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Histo2dConfig {
    pub name: String,
    pub lazyframe: String,
    pub x_column: String,
    pub y_column: String,
    pub bins: (usize, usize),
    pub range: ((f64, f64), (f64, f64)),
    pub calculate: bool,
}

impl Default for Histo2dConfig {
    fn default() -> Self {
        Self {
            name: "PID".to_string(),
            lazyframe: "Raw".to_string(),
            x_column: "ScintLeftEnergy".to_string(),
            y_column: "AnodeBackEnergy".to_string(),
            bins: (512, 512),
            range: ((0.0, 4096.0), (0.0, 4096.0)),
            calculate: true,
        }
    }
}

impl Histo2dConfig {
    pub fn ui(&mut self, ui: &mut egui::Ui, lazyframe_info: Vec<LazyFrameInfo>) {
        ui.text_edit_singleline(&mut self.name);

        egui::ComboBox::from_id_source(self.name.clone())
            .selected_text(&self.lazyframe)
            .show_ui(ui, |ui| {
                for lf in &lazyframe_info {
                    ui.selectable_value(&mut self.lazyframe, lf.name.clone(), lf.name.clone());
                }
            });

        ui.text_edit_singleline(&mut self.x_column);

        ui.text_edit_singleline(&mut self.y_column);

        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.bins.0)
                    .speed(1.0)
                    .range(1..=usize::MAX),
            );

            ui.add(
                egui::DragValue::new(&mut self.bins.1)
                    .speed(1.0)
                    .range(1..=usize::MAX),
            );
        });

        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.range.0 .0)
                    .speed(1.0)
                    .prefix("( (")
                    .suffix(","),
            );
            ui.add(
                egui::DragValue::new(&mut self.range.0 .1)
                    .speed(1.0)
                    .suffix(") ,"),
            );

            ui.add(
                egui::DragValue::new(&mut self.range.1 .0)
                    .speed(1.0)
                    .prefix("(")
                    .suffix(","),
            );

            ui.add(
                egui::DragValue::new(&mut self.range.1 .1)
                    .speed(1.0)
                    .suffix(") )"),
            );
        });

        ui.checkbox(&mut self.calculate, "");
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub enum HistoConfig {
    Histo1d(Histo1dConfig),
    Histo2d(Histo2dConfig),
}

impl HistoConfig {
    pub fn ui(&mut self, ui: &mut egui::Ui, lazyframe_info: Vec<LazyFrameInfo>) {
        match self {
            HistoConfig::Histo1d(config) => {
                config.ui(ui, lazyframe_info);
            }
            HistoConfig::Histo2d(config) => {
                config.ui(ui, lazyframe_info);
            }
        }
    }

    pub fn name(&self) -> String {
        match self {
            HistoConfig::Histo1d(config) => config.name.clone(),
            HistoConfig::Histo2d(config) => config.name.clone(),
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct HistogramScript {
    pub lazyframe_info: Vec<LazyFrameInfo>,
    pub histograms: Vec<HistoConfig>,
    pub progress: f32,
}

impl HistogramScript {
    pub fn new() -> Self {
        Self {
            lazyframe_info: Vec::new(),
            histograms: Vec::new(),
            progress: 0.0,
        }
    }

    pub fn get_lazyframe_info(&mut self) {
        let lazyframes = LazyFrames::new();
        self.lazyframe_info = lazyframes.get_lazyframe_info();
    }

    pub fn get_hist_names(&self) -> Vec<String> {
        self.histograms.iter().map(|hist| hist.name()).collect()
    }

    pub fn add_histogram1d(&mut self, config: Histo1dConfig) {
        self.histograms.push(HistoConfig::Histo1d(config));
    }

    pub fn add_histogram2d(&mut self, config: Histo2dConfig) {
        self.histograms.push(HistoConfig::Histo2d(config));
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // UI for progress bar
        ui.horizontal(|ui| {
            ui.label("Progress:");
            ui.add(egui::ProgressBar::new(self.progress).show_percentage());
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Add Histogram");
            if ui.button("1d").clicked() {
                self.add_histogram1d(Histo1dConfig::default());
            }
            if ui.button("2d").clicked() {
                self.add_histogram2d(Histo2dConfig::default());
            }
        });

        egui::ScrollArea::vertical()
            .id_source("HistogramScriptScrollArea")
            .show(ui, |ui| {
                egui::Grid::new("Histogram Config")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Name               ");
                        ui.label("LazyFrame");
                        ui.label("Column               ");
                        ui.label("                                        ");
                        ui.label("Bins");
                        ui.label("Range");
                        ui.label("Calculate");
                        ui.end_row();
                        for config in &mut self.histograms {
                            config.ui(ui, self.lazyframe_info.clone());
                            ui.end_row();
                        }
                    });
            });
    }

    pub fn add_histograms(&mut self, lf: LazyFrame) -> Result<Histogrammer, PolarsError> {
        let mut lazyframes = LazyFrames::new();
        lazyframes.create_lfs(lf);

        let mut histogrammer = Histogrammer::new();

        let total_histograms = self.histograms.len() as f32;
        for (i, hist) in self.histograms.iter_mut().enumerate() {
            match hist {
                HistoConfig::Histo1d(config) => {
                    if let Some(lf) = lazyframes.get_lf(&config.lazyframe) {
                        let name = config.name.clone();
                        let column = config.column.clone();
                        let bins = config.bins;
                        let range = config.range;
                        histogrammer.add_fill_hist1d(&name, lf, &column, bins, range);
                    } else {
                        log::error!("LazyFrame not found: {}", config.lazyframe);
                    }
                }
                HistoConfig::Histo2d(config) => {
                    if let Some(lf) = lazyframes.get_lf(&config.lazyframe) {
                        let name = config.name.clone();
                        let x_column = config.x_column.clone();
                        let y_column = config.y_column.clone();
                        let bins = config.bins;
                        let range = config.range;
                        histogrammer.add_fill_hist2d(&name, lf, &x_column, &y_column, bins, range);
                    } else {
                        log::error!("LazyFrame not found: {}", config.lazyframe);
                    }
                }
            }
            // Update progress
            self.progress = (i as f32 + 1.0) / total_histograms;
        }

        let hist_names = self.get_hist_names();
        let pane_names: Vec<&str> = hist_names.iter().map(|s| s.as_str()).collect();
        let panes = histogrammer.get_panes(pane_names);
        histogrammer.tabs.insert("All".to_string(), panes);

        Ok(histogrammer)
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct GridConfig {
    pub name: String,
    pub histograms: Vec<String>,
    pub selected_histogram: String,
}

impl GridConfig {
    pub fn ui(&mut self, ui: &mut egui::Ui, histo_keys: &[String]) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.name);
            });

            ui.separator();

            ui.label("Histograms:");
            for name in &self.histograms {
                ui.label(name);
            }

            ui.separator();

            egui::ComboBox::from_label("Histogram")
                .selected_text(&self.selected_histogram)
                .show_ui(ui, |ui| {
                    for key in histo_keys {
                        ui.selectable_value(&mut self.selected_histogram, key.clone(), key);
                    }
                });
        });
    }
}
