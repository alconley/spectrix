use std::collections::HashMap;
use super::histogrammer::Histogrammer;
use polars::prelude::*;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct LazyFrameInfo {
    pub name: String,
    pub columns: Vec<String>,
}

pub struct LazyFrames {
    pub lfs: HashMap<String, LazyFrame>,
}

impl LazyFrames {
    pub fn new() -> Self {
        Self { lfs: HashMap::new() }
    }

    pub fn create_lfs(&mut self, lf: LazyFrame) -> &Self {
        // Declare all the lazyframes that will be used
        let lf = lf.with_columns(vec![
            (col("DelayFrontRightEnergy") + col("DelayFrontLeftEnergy") / lit(2.0)).alias("DelayFrontAverageEnergy"),
            (col("DelayBackRightEnergy") + col("DelayBackLeftEnergy") / lit(2.0)).alias("DelayBackAverageEnergy"),
            (col("DelayFrontLeftTime") - col("AnodeFrontTime")).alias("DelayFrontLeftTime_AnodeFrontTime"),
            (col("DelayFrontRightTime") - col("AnodeFrontTime")).alias("DelayFrontRightTime_AnodeFrontTime"),
            (col("DelayBackLeftTime") - col("AnodeFrontTime")).alias("DelayBackLeftTime_AnodeFrontTime"),
            (col("DelayBackRightTime") - col("AnodeFrontTime")).alias("DelayBackRightTime_AnodeFrontTime"),
            (col("DelayFrontLeftTime") - col("AnodeBackTime")).alias("DelayFrontLeftTime_AnodeBackTime"),
            (col("DelayFrontRightTime") - col("AnodeBackTime")).alias("DelayFrontRightTime_AnodeBackTime"),
            (col("DelayBackLeftTime") - col("AnodeBackTime")).alias("DelayBackLeftTime_AnodeBackTime"),
            (col("DelayBackRightTime") - col("AnodeBackTime")).alias("DelayBackRightTime_AnodeBackTime"),
            (col("AnodeFrontTime") - col("AnodeBackTime")).alias("AnodeFrontTime_AnodeBackTime"),
            (col("AnodeBackTime") - col("AnodeFrontTime")).alias("AnodeBackTime_AnodeFrontTime"),
            (col("AnodeFrontTime") - col("ScintLeftTime")).alias("AnodeFrontTime_ScintLeftTime"),
            (col("AnodeBackTime") - col("ScintLeftTime")).alias("AnodeBackTime_ScintLeftTime"),
            (col("DelayFrontLeftTime") - col("ScintLeftTime")).alias("DelayFrontLeftTime_ScintLeftTime"),
            (col("DelayFrontRightTime") - col("ScintLeftTime")).alias("DelayFrontRightTime_ScintLeftTime"),
            (col("DelayBackLeftTime") - col("ScintLeftTime")).alias("DelayBackLeftTime_ScintLeftTime"),
            (col("DelayBackRightTime") - col("ScintLeftTime")).alias("DelayBackRightTime_ScintLeftTime"),
            (col("ScintRightTime") - col("ScintLeftTime")).alias("ScintRightTime_ScintLeftTime"),
            (lit(-0.013139237615) * col("Xavg") * col("Xavg") + lit(-13.80004977) * col("Xavg") + lit(9790.048149635)).alias("XavgEnergyCalibrated"),
        ]);

        let lf_bothplanes = lf.clone().filter(col("X1").neq(lit(-1e6))).filter(col("X2").neq(lit(-1e6)));
        let lf_only_x1_plane = lf.clone().filter(col("X1").neq(lit(-1e6))).filter(col("X2").eq(lit(-1e6)));
        let lf_only_x2_plane = lf.clone().filter(col("X2").neq(lit(-1e6))).filter(col("X1").eq(lit(-1e6)));

        self.lfs.insert("Raw".to_string(), lf);
        self.lfs.insert("BothPlanes".to_string(), lf_bothplanes);
        self.lfs.insert("OnlyX1Plane".to_string(), lf_only_x1_plane);
        self.lfs.insert("OnlyX2Plane".to_string(), lf_only_x2_plane);

        self
    }

    pub fn get_lf(&self, key: &str) -> Option<&LazyFrame> {
        self.lfs.get(key)
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
    pub fn ui(&mut self, ui: &mut egui::Ui, lazyframe_keys: &[String]) {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.name);
        });
        ui.horizontal(|ui| {
            ui.label("LazyFrame:");
            egui::ComboBox::from_label("LazyFrame")
                .selected_text(&self.lazyframe)
                .show_ui(ui, |ui| {
                    for key in lazyframe_keys {
                        ui.selectable_value(&mut self.lazyframe, key.clone(), key);
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("Column:");
            ui.text_edit_singleline(&mut self.column);
        });

        ui.horizontal(|ui| {
            ui.label("Bins:");
            ui.add(
                egui::DragValue::new(&mut self.bins)
                    .speed(1.0)
                    .range(1..=usize::MAX),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Range:");
            ui.add(
                egui::DragValue::new(&mut self.range.0)
                    .speed(1.0)
                    .prefix("min:")
            );
            ui.add(
                egui::DragValue::new(&mut self.range.1)
                    .speed(1.0)
                    .prefix("max:")
            );
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.calculate, "Calculate");
        });
    }
}


#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct GridConfig {
    pub name: String,
    pub histograms: Vec<String>,
}

impl GridConfig {
    pub fn ui(&mut self, ui: &mut egui::Ui, histo_keys: &[String]) {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.name);
        });

        ui.group(|ui| {
            ui.label("Histograms:");
            for name in &self.histograms {
                ui.label(name);
            }
        });
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct HistogramScript {
    pub lazyframe_info: Vec<LazyFrameInfo>,
    pub histograms: HashMap<String, Histo1dConfig>,
    pub grids: Vec<GridConfig>,
}

impl HistogramScript {
    pub fn new() -> Self {
        Self { lazyframe_info: Vec::new(), histograms: HashMap::new(), grids: Vec::new() }
    }

    pub fn add_histogram(&mut self, config: Histo1dConfig) {
        self.histograms.insert(config.name.clone(), config);
    }

    pub fn config(&self, key: &str) -> Option<&Histo1dConfig> {
        self.histograms.get(key)
    }

    pub fn config_mut(&mut self, key: &str) -> Option<&mut Histo1dConfig> {
        self.histograms.get_mut(key)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let lazyframe_keys: Vec<String> = self.histograms.keys().cloned().collect();

        for config in self.histograms.values_mut() {
            config.ui(ui, &lazyframe_keys);
        }

        if ui.button("Add Histogram").clicked() {
            self.add_histogram(Histo1dConfig::default());
        }

        ui.separator();

    }

    pub fn add_histograms(&self, lf: LazyFrame, show_progress: bool) -> Result<Histogrammer, PolarsError> {
        let mut lazyframes = LazyFrames::new();
        lazyframes.create_lfs(lf);

        let mut histogrammer = Histogrammer::new();
        histogrammer.show_progress = show_progress;

        for config in self.histograms.values() {
            if config.calculate {
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
        }

        // for (grid_name, hist_names) in &self.grids {
        //     let pane_names: Vec<&str> = hist_names.iter().map(|s| s.as_str()).collect();
        //     let panes = histogrammer.get_panes(pane_names);
        //     histogrammer.tabs.insert(grid_name.clone(), panes);
        // }

        Ok(histogrammer)
    }
}
