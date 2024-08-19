use super::configure_lazyframes::LazyFrameInfo;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub enum HistoConfig {
    Histo1d(Histo1dConfig),
    Histo2d(Histo2dConfig),
}

impl HistoConfig {
    pub fn ui(&mut self, ui: &mut egui::Ui, lazyframe_info: LazyFrameInfo) {
        match self {
            HistoConfig::Histo1d(config) => {
                config.ui(ui, lazyframe_info);
            }
            HistoConfig::Histo2d(config) => {
                config.ui(ui, lazyframe_info);
            }
        }
    }

    pub fn set_name(&mut self, new_name: &str) {
        match self {
            HistoConfig::Histo1d(config) => config.name = new_name.to_string(),
            HistoConfig::Histo2d(config) => config.name = new_name.to_string(),
        }
    }

    pub fn name(&self) -> String {
        match self {
            HistoConfig::Histo1d(config) => config.name.clone(),
            HistoConfig::Histo2d(config) => config.name.clone(),
        }
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
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
    pub fn ui(&mut self, ui: &mut egui::Ui, lazyframe_info: LazyFrameInfo) {
        ui.text_edit_singleline(&mut self.name);

        egui::ComboBox::from_id_source(format!("{}-{}", self.name, self.lazyframe))
            .selected_text(&self.lazyframe)
            .show_ui(ui, |ui| {
                for lf in &lazyframe_info.lfs {
                    ui.selectable_value(&mut self.lazyframe, lf.clone(), lf.clone());
                }
            });

        ui.horizontal(|ui| {
            ui.label("X:");
            egui::ComboBox::from_id_source(format!("{}-{}", self.name, self.column))
                .selected_text(&self.column)
                .show_ui(ui, |ui| {
                    for column in &lazyframe_info.columns {
                        ui.selectable_value(&mut self.column, column.clone(), column.clone());
                    }
                });
        });

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

#[derive(Clone, serde::Deserialize, serde::Serialize)]
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
    pub fn ui(&mut self, ui: &mut egui::Ui, lazyframe_info: LazyFrameInfo) {
        ui.text_edit_singleline(&mut self.name);

        egui::ComboBox::from_id_source(format!("{}-{}", self.name, self.lazyframe))
            .selected_text(&self.lazyframe)
            .show_ui(ui, |ui| {
                for lf in &lazyframe_info.lfs {
                    ui.selectable_value(&mut self.lazyframe, lf.clone(), lf.clone());
                }
            });

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("X:");
                egui::ComboBox::from_id_source(format!("{}-X Column Combobox", self.name))
                    .selected_text(&self.x_column)
                    .show_ui(ui, |ui| {
                        for column in &lazyframe_info.columns {
                            ui.selectable_value(&mut self.x_column, column.clone(), column.clone());
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Y:");
                egui::ComboBox::from_id_source(format!("{}-Y Column ComboBox", self.name))
                    .selected_text(&self.y_column)
                    .show_ui(ui, |ui| {
                        for column in &lazyframe_info.columns {
                            ui.selectable_value(&mut self.y_column, column.clone(), column.clone());
                        }
                    });
            });
        });

        ui.vertical(|ui| {
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

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut self.range.0 .0)
                        .speed(1.0)
                        .prefix("(")
                        .suffix(","),
                );
                ui.add(
                    egui::DragValue::new(&mut self.range.0 .1)
                        .speed(1.0)
                        .suffix(")"),
                );
            });

            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut self.range.1 .0)
                        .speed(1.0)
                        .prefix("(")
                        .suffix(","),
                );
                ui.add(
                    egui::DragValue::new(&mut self.range.1 .1)
                        .speed(1.0)
                        .suffix(")"),
                );
            });
        });

        ui.checkbox(&mut self.calculate, "");
    }
}
