use super::cuts::Cut;

use egui_extras::{Column, TableBuilder, TableRow};

// Enum to encapsulate 1D and 2D histogram configurations
#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub enum Config {
    Hist1D(Hist1DConfig),
    Hist2D(Hist2DConfig),
}

impl Config {
    /// Create a new 1D histogram configuration.
    pub fn new_1d(
        name: &str,
        column_name: &str,
        range: (f64, f64),
        bins: usize,
        cuts: Option<Vec<Cut>>, // Accept owned cuts directly
    ) -> Self {
        let mut config = Hist1DConfig::new(name, column_name, range, bins);
        if let Some(cuts) = cuts {
            config.cuts = cuts;
        }
        Config::Hist1D(config)
    }

    /// Create a new 2D histogram configuration.
    pub fn new_2d(
        name: &str,
        x_column_name: &str,
        y_column_name: &str,
        x_range: (f64, f64),
        y_range: (f64, f64),
        bins: (usize, usize),
        cuts: Option<Vec<Cut>>,
    ) -> Self {
        let mut config =
            Hist2DConfig::new(name, x_column_name, y_column_name, x_range, y_range, bins);
        if let Some(cuts) = cuts {
            config.cuts = cuts;
        }
        Config::Hist2D(config)
    }

    pub fn table_row(&mut self, row: &mut TableRow<'_, '_>, cuts: &mut Vec<Cut>) {
        match self {
            Config::Hist1D(config) => config.table_row(row, cuts),
            Config::Hist2D(config) => config.table_row(row, cuts),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Configs {
    pub configs: Vec<Config>,
}

impl Configs {
    // pub fn validate_and_filter_columns(&mut self, lf_columns: Vec<String>) -> Vec<String> {

    // }

    pub fn ui(&mut self, ui: &mut egui::Ui, cuts: &mut Vec<Cut>) {
        ui.horizontal(|ui| {
            ui.heading("Histograms");

            if ui.button("+1D").clicked() {
                self.configs.push(Config::Hist1D(Hist1DConfig {
                    name: "".to_string(),
                    column_name: "".to_string(),
                    range: (0.0, 4096.0),
                    bins: 512,
                    cuts: vec![],
                    calculate: true,
                    enabled: true,
                }));
            }

            if ui.button("+2D").clicked() {
                self.configs.push(Config::Hist2D(Hist2DConfig {
                    name: "".to_string(),
                    x_column_name: "".to_string(),
                    y_column_name: "".to_string(),
                    x_range: (0.0, 4096.0),
                    y_range: (0.0, 4096.0),
                    bins: (512, 512),
                    cuts: vec![],
                    calculate: true,
                    enabled: true,
                }));
            }

            ui.separator();

            if ui.button("Remove All").clicked() {
                self.configs.clear();
            }
        });

        let mut indices_to_remove = Vec::new();

        // Create the table
        TableBuilder::new(ui)
            .id_salt("hist_configs")
            .column(Column::auto()) // Type
            .column(Column::auto()) // Name
            .column(Column::auto()) // Columns
            .column(Column::auto()) // Ranges
            .column(Column::auto()) // Bins
            .column(Column::auto()) // cuts
            .column(Column::auto()) // Actions
            .column(Column::remainder()) // remove
            .striped(true)
            .vscroll(false)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label(" # ");
                });
                header.col(|ui| {
                    ui.label("Name");
                });
                header.col(|ui| {
                    ui.label("Column(s)");
                });
                header.col(|ui| {
                    ui.label("Range(s)");
                });
                header.col(|ui| {
                    ui.label("Bins");
                });
                header.col(|ui| {
                    ui.label("Cuts");
                });
            })
            .body(|mut body| {
                for (index, config) in self.configs.iter_mut().enumerate() {
                    body.row(18.0, |mut row| {
                        row.col(|ui| match config {
                            Config::Hist1D(_) => {
                                ui.label(format!("{index}"));
                            }
                            Config::Hist2D(_) => {
                                ui.label(format!("{index}"));
                            }
                        });

                        config.table_row(&mut row, cuts);

                        row.col(|ui| {
                            if ui.button("X").clicked() {
                                indices_to_remove.push(index);
                            }
                        });
                    });
                }
            });

        // Remove indices in reverse order to prevent shifting issues
        for &index in indices_to_remove.iter().rev() {
            self.configs.remove(index);
        }
    }
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Hist1DConfig {
    pub name: String,        // Histogram display name
    pub column_name: String, // Data column to fill from
    pub range: (f64, f64),   // Range for the histogram
    pub bins: usize,         // Number of bins
    pub cuts: Vec<Cut>,      // Cuts for the histogram
    pub calculate: bool,     // Whether to calculate the histogram
    pub enabled: bool,       // Whether to let the user interact with the histogram
}

impl Hist1DConfig {
    pub fn new(name: &str, column_name: &str, range: (f64, f64), bins: usize) -> Self {
        Self {
            name: name.to_string(),
            column_name: column_name.to_string(),
            range,
            bins,
            cuts: Vec::new(),
            calculate: true,
            enabled: true,
        }
    }

    pub fn table_row(&mut self, row: &mut egui_extras::TableRow<'_, '_>, cuts: &mut Vec<Cut>) {
        row.col(|ui| {
            ui.add_enabled(
                self.enabled,
                egui::TextEdit::singleline(&mut self.name)
                    .hint_text("Name")
                    .clip_text(false),
            );
        });

        row.col(|ui| {
            ui.add_enabled(
                self.enabled,
                egui::TextEdit::singleline(&mut self.column_name)
                    .hint_text("Column Name")
                    .clip_text(false),
            );
        });

        row.col(|ui| {
            ui.horizontal(|ui| {
                ui.add_enabled(
                    self.enabled,
                    egui::DragValue::new(&mut self.range.0)
                        .speed(0.1)
                        .prefix("(")
                        .suffix(","),
                );
                ui.add_enabled(
                    self.enabled,
                    egui::DragValue::new(&mut self.range.1)
                        .speed(0.1)
                        .prefix(" ")
                        .suffix(")"),
                );
            });
        });

        row.col(|ui| {
            ui.add_enabled(self.enabled, egui::DragValue::new(&mut self.bins).speed(1));
        });

        row.col(|ui| {
            egui::ComboBox::from_id_salt(format!("cut_select_1d_{}", self.name))
                .selected_text("Select cuts")
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for cut in cuts {
                        let mut is_selected =
                            self.cuts.iter().any(|selected_cut| selected_cut == cut);
                        match cut {
                            Cut::Cut1D(cut1d) => {
                                if ui.checkbox(&mut is_selected, &cut1d.name).clicked() {
                                    if is_selected && !self.cuts.contains(cut) {
                                        self.cuts.push(cut.clone());
                                    } else if !is_selected {
                                        self.cuts.retain(|selected_cut| selected_cut != cut);
                                    }
                                }
                            }
                            Cut::Cut2D(cut2d) => {
                                if ui.checkbox(&mut is_selected, &cut2d.polygon.name).clicked() {
                                    if is_selected && !self.cuts.contains(cut) {
                                        self.cuts.push(cut.clone());
                                    } else if !is_selected {
                                        self.cuts.retain(|selected_cut| selected_cut != cut);
                                    }
                                }
                            }
                        }
                    }
                });
        });

        row.col(|ui| {
            ui.add_enabled(self.enabled, egui::Checkbox::new(&mut self.calculate, ""));
        });
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Hist2DConfig {
    pub name: String,          // Histogram display name
    pub x_column_name: String, // Data column for X-axis
    pub y_column_name: String, // Data column for Y-axis
    pub x_range: (f64, f64),   // Range for X-axis
    pub y_range: (f64, f64),   // Range for Y-axis
    pub bins: (usize, usize),  // Number of bins for X and Y axes
    pub cuts: Vec<Cut>,        // Cuts for the histogram
    pub calculate: bool,       // Whether to calculate the histogram
    pub enabled: bool,         // Whether to let the user interact with the histogram
}

impl Hist2DConfig {
    pub fn new(
        name: &str,
        x_column_name: &str,
        y_column_name: &str,
        x_range: (f64, f64),
        y_range: (f64, f64),
        bins: (usize, usize),
    ) -> Self {
        Self {
            name: name.to_string(),
            x_column_name: x_column_name.to_string(),
            y_column_name: y_column_name.to_string(),
            x_range,
            y_range,
            bins,
            cuts: Vec::new(),
            calculate: true,
            enabled: true,
        }
    }

    pub fn table_row(&mut self, row: &mut egui_extras::TableRow<'_, '_>, cuts: &mut Vec<Cut>) {
        row.col(|ui| {
            ui.add_enabled(
                self.enabled,
                egui::TextEdit::singleline(&mut self.name)
                    .hint_text("Name")
                    .clip_text(false),
            );
        });

        row.col(|ui| {
            ui.vertical(|ui| {
                ui.add_enabled(
                    self.enabled,
                    egui::TextEdit::singleline(&mut self.x_column_name)
                        .hint_text("X Column Name")
                        .clip_text(false),
                );
                ui.add_enabled(
                    self.enabled,
                    egui::TextEdit::singleline(&mut self.y_column_name)
                        .hint_text("Y Column Name")
                        .clip_text(false),
                );
            });
        });

        row.col(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.add_enabled(
                        self.enabled,
                        egui::DragValue::new(&mut self.x_range.0)
                            .speed(0.1)
                            .prefix("(")
                            .suffix(","),
                    );
                    ui.add_enabled(
                        self.enabled,
                        egui::DragValue::new(&mut self.x_range.1)
                            .speed(0.1)
                            .prefix(" ")
                            .suffix(")"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.add_enabled(
                        self.enabled,
                        egui::DragValue::new(&mut self.y_range.0)
                            .speed(0.1)
                            .prefix("(")
                            .suffix(","),
                    );
                    ui.add_enabled(
                        self.enabled,
                        egui::DragValue::new(&mut self.y_range.1)
                            .speed(0.1)
                            .prefix(" ")
                            .suffix(")"),
                    );
                });
            });
        });

        row.col(|ui| {
            ui.vertical(|ui| {
                ui.add_enabled(
                    self.enabled,
                    egui::DragValue::new(&mut self.bins.0).speed(1),
                );
                ui.add_enabled(
                    self.enabled,
                    egui::DragValue::new(&mut self.bins.1).speed(1),
                );
            });
        });

        row.col(|ui| {
            egui::ComboBox::from_id_salt(format!("cut_select_1d_{}", self.name))
                .selected_text("Select cuts")
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for cut in cuts {
                        let mut is_selected =
                            self.cuts.iter().any(|selected_cut| selected_cut == cut);
                        match cut {
                            Cut::Cut1D(cut1d) => {
                                if ui.checkbox(&mut is_selected, &cut1d.name).clicked() {
                                    if is_selected && !self.cuts.contains(cut) {
                                        self.cuts.push(cut.clone());
                                    } else if !is_selected {
                                        self.cuts.retain(|selected_cut| selected_cut != cut);
                                    }
                                }
                            }
                            Cut::Cut2D(cut2d) => {
                                if ui.checkbox(&mut is_selected, &cut2d.polygon.name).clicked() {
                                    if is_selected && !self.cuts.contains(cut) {
                                        self.cuts.push(cut.clone());
                                    } else if !is_selected {
                                        self.cuts.retain(|selected_cut| selected_cut != cut);
                                    }
                                }
                            }
                        }
                    }
                });
        });

        row.col(|ui| {
            ui.add_enabled(self.enabled, egui::Checkbox::new(&mut self.calculate, ""));
        });
    }
}
