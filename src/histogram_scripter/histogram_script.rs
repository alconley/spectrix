// use super::configure_auxillary_detectors::AuxillaryDetectors;
use super::manual_histogram_scripts::sps_histograms;

use crate::cutter::cuts::Cut;
use crate::histoer::histogrammer::{Histo1DConfig, Histo2DConfig, Histogrammer};
use egui_extras::{Column, TableBuilder};
use polars::prelude::*;
use rfd::FileDialog;
use serde_json;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};

// Enum for sorting options
#[derive(Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, Default)]
pub enum SortOrder {
    #[default]
    Name,
    Column,
    Type, // 1D or 2D
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct HistogramScript {
    pub hist_configs: Vec<HistoConfig>, // Unified vector for both 1D and 2D configurations
    pub new_columns: Vec<(String, String)>,
    pub cuts: Vec<Cut>,
    #[serde(skip)]
    pub sort_order: SortOrder,
    #[serde(skip)]
    pub reverse_sort: bool,
}

// Enum to encapsulate 1D and 2D histogram configurations
#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub enum HistoConfig {
    Histo1D(Histo1DConfig),
    Histo2D(Histo2DConfig),
}

impl HistogramScript {
    pub fn new() -> Self {
        Self {
            hist_configs: vec![],
            new_columns: vec![],
            cuts: vec![],
            sort_order: SortOrder::default(),
            reverse_sort: false,
        }
    }

    // Helper function to check if a histogram with the given name already exists
    fn histogram_exists(&self, name: &str) -> bool {
        self.hist_configs.iter().any(|config| match config {
            HistoConfig::Histo1D(hist) => hist.name == name,
            HistoConfig::Histo2D(hist) => hist.name == name,
        })
    }

    // Helper function to check if a column with the given alias already exists
    fn column_exists(&self, alias: &str) -> bool {
        self.new_columns
            .iter()
            .any(|(_, col_alias)| col_alias == alias)
    }

    // Sorting logic based on the selected SortOrder and reverse_sort flag
    fn sort_histograms(&mut self) {
        match self.sort_order {
            SortOrder::Name => {
                self.hist_configs.sort_by(|a, b| match (a, b) {
                    (HistoConfig::Histo1D(h1), HistoConfig::Histo1D(h2)) => h1.name.cmp(&h2.name),
                    (HistoConfig::Histo2D(h1), HistoConfig::Histo2D(h2)) => h1.name.cmp(&h2.name),
                    _ => std::cmp::Ordering::Equal,
                });
            }
            SortOrder::Column => {
                self.hist_configs.sort_by(|a, b| match (a, b) {
                    (HistoConfig::Histo1D(h1), HistoConfig::Histo1D(h2)) => {
                        h1.column_name.cmp(&h2.column_name)
                    }
                    (HistoConfig::Histo2D(h1), HistoConfig::Histo2D(h2)) => h1
                        .x_column_name
                        .cmp(&h2.x_column_name)
                        .then_with(|| h1.y_column_name.cmp(&h2.y_column_name)),
                    _ => std::cmp::Ordering::Equal,
                });
            }
            SortOrder::Type => {
                self.hist_configs.sort_by(|a, b| match (a, b) {
                    (HistoConfig::Histo1D(_), HistoConfig::Histo2D(_)) => std::cmp::Ordering::Less,
                    (HistoConfig::Histo2D(_), HistoConfig::Histo1D(_)) => {
                        std::cmp::Ordering::Greater
                    }
                    _ => std::cmp::Ordering::Equal,
                });
            }
        }

        // Reverse the order if reverse_sort is true
        if self.reverse_sort {
            self.hist_configs.reverse();
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Custom Histogram Scripts");
        ui.horizontal(|ui| {
            if ui.button("SE-SPS").clicked() {
                let (columns, histograms) = sps_histograms();
                for histogram in histograms {
                    match &histogram {
                        HistoConfig::Histo1D(histo1d) => {
                            if !self.histogram_exists(&histo1d.name) {
                                self.hist_configs
                                    .push(HistoConfig::Histo1D(histo1d.clone()));
                            }
                        }
                        HistoConfig::Histo2D(histo2d) => {
                            if !self.histogram_exists(&histo2d.name) {
                                self.hist_configs
                                    .push(HistoConfig::Histo2D(histo2d.clone()));
                            }
                        }
                    }
                }

                // Only add columns if the alias is unique
                for (expression, alias) in columns {
                    if !self.column_exists(&alias) {
                        self.new_columns.push((expression, alias));
                    }
                }
            }

            ui.separator();

            if ui.button("Save Script").clicked() {
                if let Err(e) = self.save_histogram_script() {
                    log::error!("Failed to save script: {}", e);
                }
            }
            if ui.button("Load Script").clicked() {
                if let Err(e) = self.load_histogram_script() {
                    log::error!("Failed to load script: {}", e);
                }
            }
        });

        ui.separator();

        // Add header controls
        ui.horizontal(|ui| {
            if ui.button("Add 1D Histogram").clicked() {
                self.hist_configs.push(HistoConfig::Histo1D(Histo1DConfig {
                    name: "".to_string(),
                    column_name: "".to_string(),
                    range: (0.0, 4096.0),
                    bins: 512,
                    cuts: vec![],
                    calculate: true,
                }));
            }

            if ui.button("Add 2D Histogram").clicked() {
                self.hist_configs.push(HistoConfig::Histo2D(Histo2DConfig {
                    name: "".to_string(),
                    x_column_name: "".to_string(),
                    y_column_name: "".to_string(),
                    x_range: (0.0, 4096.0),
                    y_range: (0.0, 4096.0),
                    bins: (512, 512),
                    cuts: vec![],
                    calculate: true,
                }));
            }

            ui.separator();

            if ui.button("Remove All").clicked() {
                self.hist_configs.clear();
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Cuts");

                if ui.button("+").clicked() {
                    let mut new_cut = Cut::default();
                    if let Err(e) = new_cut.load_cut_from_json() {
                        log::error!("Failed to load cut: {:?}", e);
                    } else {
                        self.cuts.push(new_cut);
                    }
                }
            });

            if self.cuts.is_empty() {
                ui.label("No cuts loaded");
            } else {
                // Table for Cuts with similar layout to Column Creation and Histograms
                let mut indices_to_remove_cut = Vec::new();

                TableBuilder::new(ui)
                    .id_salt("cuts_table")
                    .column(Column::auto()) // Name
                    .column(Column::auto()) // X Column
                    .column(Column::auto()) // Y Column
                    .column(Column::remainder()) // Actions
                    .striped(true)
                    .vscroll(false)
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.label("Name");
                        });
                        header.col(|ui| {
                            ui.label("X Column");
                        });
                        header.col(|ui| {
                            ui.label("Y Column");
                        });
                    })
                    .body(|mut body| {
                        for (index, cut) in self.cuts.iter_mut().enumerate() {
                            body.row(18.0, |mut row| {
                                row.col(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut cut.polygon.name)
                                            .hint_text("Cut Name")
                                            .clip_text(false),
                                    );
                                });
                                row.col(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut cut.x_column)
                                            .hint_text("X Column")
                                            .clip_text(false),
                                    );
                                });
                                row.col(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut cut.y_column)
                                            .hint_text("Y Column")
                                            .clip_text(false),
                                    );
                                });
                                row.col(|ui| {
                                    if ui.button("X").clicked() {
                                        indices_to_remove_cut.push(index);
                                    }
                                });
                            });
                        }
                    });

                // Remove indices in reverse order to prevent shifting issues
                for &index in indices_to_remove_cut.iter().rev() {
                    self.cuts.remove(index);
                }

                // Buttons to apply cuts to all histograms or remove all cuts from histograms
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        for config in &mut self.hist_configs {
                            match config {
                                HistoConfig::Histo1D(hist) => {
                                    hist.cuts = self.cuts.clone(); // Apply all cuts to 1D histograms
                                }
                                HistoConfig::Histo2D(hist) => {
                                    hist.cuts = self.cuts.clone(); // Apply all cuts to 2D histograms
                                }
                            }
                        }
                    }
                    ui.label("/");

                    if ui.button("Remove").clicked() {
                        for config in &mut self.hist_configs {
                            match config {
                                HistoConfig::Histo1D(hist) => {
                                    hist.cuts.clear(); // Clear all cuts from 1D histograms
                                }
                                HistoConfig::Histo2D(hist) => {
                                    hist.cuts.clear(); // Clear all cuts from 2D histograms
                                }
                            }
                        }
                    }

                    ui.label(" all cuts on histograms");
                });
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.heading("Column Creation");

                if ui.button("+").clicked() {
                    self.new_columns.push(("".to_string(), "".to_string()));
                }
            });

            if !self.new_columns.is_empty() {
                let mut indices_to_remove_column = Vec::new();

                TableBuilder::new(ui)
                    .id_salt("new_columns")
                    .column(Column::auto()) // expression
                    .column(Column::auto()) // alias
                    .column(Column::remainder()) // Actions
                    .striped(true)
                    .vscroll(false)
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.label("Expression");
                        });
                        header.col(|ui| {
                            ui.label("Alias");
                        });
                    })
                    .body(|mut body| {
                        for (index, (expression, alias)) in self.new_columns.iter_mut().enumerate()
                        {
                            body.row(18.0, |mut row| {
                                row.col(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(expression)
                                            .hint_text("Expression")
                                            .clip_text(false),
                                    );
                                });

                                row.col(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(alias)
                                            .hint_text("Alias")
                                            .clip_text(false),
                                    );
                                });

                                row.col(|ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("X").clicked() {
                                            indices_to_remove_column.push(index);
                                        }
                                    });
                                });
                            });
                        }
                    });

                // Remove indices in reverse order to prevent shifting issues
                for &index in indices_to_remove_column.iter().rev() {
                    self.new_columns.remove(index);
                }
            }

            ui.separator();

            ui.heading("Histograms");

            // Sorting controls
            ui.horizontal(|ui| {
                ui.label("Sort by:");
                if ui.button("Name").clicked() {
                    self.sort_order = SortOrder::Name;
                    self.reverse_sort = !self.reverse_sort;
                    self.sort_histograms();
                }
                if ui.button("Column").clicked() {
                    self.sort_order = SortOrder::Column;
                    self.reverse_sort = !self.reverse_sort;
                    self.sort_histograms();
                }
                if ui.button("Type").clicked() {
                    self.sort_order = SortOrder::Type;
                    self.reverse_sort = !self.reverse_sort;
                    self.sort_histograms();
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
                .column(Column::remainder()) // Actions
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
                    header.col(|ui| {
                        ui.label("Actions");
                    });
                })
                .body(|mut body| {
                    for (index, config) in self.hist_configs.iter_mut().enumerate() {
                        body.row(18.0, |mut row| {
                            row.col(|ui| match config {
                                HistoConfig::Histo1D(_) => {
                                    ui.label(format!("{index}"));
                                }
                                HistoConfig::Histo2D(_) => {
                                    ui.label(format!("{index}"));
                                }
                            });

                            row.col(|ui| match config {
                                HistoConfig::Histo1D(hist) => {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut hist.name)
                                            .hint_text("Name")
                                            .clip_text(false),
                                    );
                                }

                                HistoConfig::Histo2D(hist) => {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut hist.name)
                                            .hint_text("Name")
                                            .clip_text(false),
                                    );
                                }
                            });

                            row.col(|ui| match config {
                                HistoConfig::Histo1D(hist) => {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut hist.column_name)
                                            .hint_text("Column Name")
                                            .clip_text(false),
                                    );
                                }
                                HistoConfig::Histo2D(hist) => {
                                    ui.vertical(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut hist.x_column_name)
                                                .hint_text("X Column Name")
                                                .clip_text(false),
                                        );
                                        ui.add(
                                            egui::TextEdit::singleline(&mut hist.y_column_name)
                                                .hint_text("Y Column Name")
                                                .clip_text(false),
                                        );
                                    });
                                }
                            });

                            row.col(|ui| match config {
                                HistoConfig::Histo1D(hist) => {
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::DragValue::new(&mut hist.range.0)
                                                .speed(0.1)
                                                .prefix("(")
                                                .suffix(","),
                                        );
                                        ui.add(
                                            egui::DragValue::new(&mut hist.range.1)
                                                .speed(0.1)
                                                .prefix(" ")
                                                .suffix(")"),
                                        );
                                    });
                                }
                                HistoConfig::Histo2D(hist) => {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::DragValue::new(&mut hist.x_range.0)
                                                    .speed(1.0)
                                                    .prefix("(")
                                                    .suffix(","),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut hist.x_range.1)
                                                    .speed(1.0)
                                                    .prefix(" ")
                                                    .suffix(")"),
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::DragValue::new(&mut hist.y_range.0)
                                                    .speed(1.0)
                                                    .prefix("(")
                                                    .suffix(","),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut hist.y_range.1)
                                                    .speed(1.0)
                                                    .prefix(" ")
                                                    .suffix(")"),
                                            );
                                        });
                                    });
                                }
                            });

                            row.col(|ui| match config {
                                HistoConfig::Histo1D(hist) => {
                                    ui.add(egui::DragValue::new(&mut hist.bins).speed(1));
                                }
                                HistoConfig::Histo2D(hist) => {
                                    ui.vertical(|ui| {
                                        ui.add(egui::DragValue::new(&mut hist.bins.0).speed(1));
                                        ui.add(egui::DragValue::new(&mut hist.bins.1).speed(1));
                                    });
                                }
                            });

                            row.col(|ui| match config {
                                HistoConfig::Histo1D(hist) => {
                                    egui::ComboBox::from_id_salt(format!(
                                        "cut_select_1d_{}",
                                        index
                                    ))
                                    .selected_text("Select cuts")
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for cut in &self.cuts {
                                            let mut is_selected = hist
                                                .cuts
                                                .iter()
                                                .any(|selected_cut| selected_cut == cut);
                                            if ui
                                                .checkbox(&mut is_selected, &cut.polygon.name)
                                                .clicked()
                                            {
                                                if is_selected && !hist.cuts.contains(cut) {
                                                    hist.cuts.push(cut.clone());
                                                } else if !is_selected {
                                                    hist.cuts
                                                        .retain(|selected_cut| selected_cut != cut);
                                                }
                                            }
                                        }
                                    });
                                }
                                HistoConfig::Histo2D(hist) => {
                                    egui::ComboBox::from_id_salt(format!(
                                        "cut_select_2d_{}",
                                        index
                                    ))
                                    .selected_text("Select cuts")
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for cut in &self.cuts {
                                            let mut is_selected = hist
                                                .cuts
                                                .iter()
                                                .any(|selected_cut| selected_cut == cut);
                                            if ui
                                                .checkbox(&mut is_selected, &cut.polygon.name)
                                                .clicked()
                                            {
                                                if is_selected && !hist.cuts.contains(cut) {
                                                    hist.cuts.push(cut.clone());
                                                } else if !is_selected {
                                                    hist.cuts
                                                        .retain(|selected_cut| selected_cut != cut);
                                                }
                                            }
                                        }
                                    });
                                }
                            });

                            row.col(|ui| {
                                ui.horizontal(|ui| {
                                    match config {
                                        HistoConfig::Histo1D(hist) => {
                                            ui.checkbox(&mut hist.calculate, "");
                                        }
                                        HistoConfig::Histo2D(hist) => {
                                            ui.checkbox(&mut hist.calculate, "");
                                        }
                                    }

                                    ui.separator();

                                    if ui.button("X").clicked() {
                                        indices_to_remove.push(index);
                                    }
                                });
                            });
                        });
                    }
                });

            // Remove indices in reverse order to prevent shifting issues
            for &index in indices_to_remove.iter().rev() {
                self.hist_configs.remove(index);
            }
        });
    }

    pub fn save_histogram_script(&self) -> io::Result<()> {
        if let Some(path) = FileDialog::new()
            .set_title("Save Histogram Script")
            .save_file()
        {
            let file = File::create(path)?;
            let writer = BufWriter::new(file);
            serde_json::to_writer(writer, &self)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        } else {
            Ok(()) // User canceled, return Ok
        }
    }

    // Function to load histogram configuration from a JSON file
    pub fn load_histogram_script(&mut self) -> io::Result<()> {
        if let Some(path) = FileDialog::new()
            .set_title("Load Histogram Script")
            .pick_file()
        {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            *self = serde_json::from_reader(reader)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }
        Ok(())
    }

    pub fn add_histograms(&mut self, h: &mut Histogrammer, lf: LazyFrame) {
        let mut histo1d_configs = Vec::new();
        let mut histo2d_configs = Vec::new();

        let range_re = regex::Regex::new(r"\{(\d+)-(\d+)\}").unwrap();
        // Regex for range pattern `{start-end}`

        let list_re = regex::Regex::new(r"\{([\d,]+)\}").unwrap();
        // Regex for discrete comma-separated values `{val1,val2,...}`

        for config in &self.hist_configs {
            match config {
                // 1D Histogram Configuration
                HistoConfig::Histo1D(histo1d) => {
                    if histo1d.calculate {
                        if histo1d.name.contains("{}") {
                            // name has {} and column_name has a range pattern
                            if let Some(caps) = range_re.captures(&histo1d.column_name) {
                                let start: usize = caps[1].parse().unwrap();
                                let end: usize = caps[2].parse().unwrap();

                                // Loop through start and end values
                                for i in start..=end {
                                    let mut new_config = histo1d.clone();
                                    new_config.name =
                                        histo1d.name.replace("{}", &i.to_string()).to_string();
                                    new_config.column_name = range_re
                                        .replace(&histo1d.column_name, i.to_string())
                                        .to_string();
                                    histo1d_configs.push(new_config);
                                }
                            }
                            // name has {} and column_name has a list pattern
                            else if let Some(caps) = list_re.captures(&histo1d.column_name) {
                                // Split comma-separated values and loop over them
                                let values: Vec<&str> = caps[1].split(',').collect();
                                for val in values {
                                    let mut new_config = histo1d.clone();
                                    new_config.name = histo1d.name.replace("{}", val).to_string();
                                    new_config.column_name =
                                        list_re.replace(&histo1d.column_name, val).to_string();
                                    histo1d_configs.push(new_config);
                                }
                            // Unsupported pattern
                            } else {
                                log::error!(
                                    "Warning: Unsupported pattern for 1D histogram with name '{}', column '{}'",
                                    histo1d.name, histo1d.column_name
                                );
                            }
                        } else {
                            // No {} in name, but column_name has a range pattern
                            if let Some(caps) = range_re.captures(&histo1d.column_name) {
                                let start: usize = caps[1].parse().unwrap();
                                let end: usize = caps[2].parse().unwrap();

                                for i in start..=end {
                                    let mut new_config = histo1d.clone();
                                    new_config.column_name = range_re
                                        .replace(&histo1d.column_name, i.to_string())
                                        .to_string();
                                    histo1d_configs.push(new_config);
                                }
                            }
                            // No {} in name, but column_name has a list pattern
                            else if let Some(caps) = list_re.captures(&histo1d.column_name) {
                                let values: Vec<&str> = caps[1].split(',').collect();
                                for val in values {
                                    let mut new_config = histo1d.clone();
                                    new_config.column_name =
                                        list_re.replace(&histo1d.column_name, val).to_string();
                                    histo1d_configs.push(new_config);
                                }
                            // No {} in name or column_name i.e. a normal configuration
                            } else {
                                histo1d_configs.push(histo1d.clone());
                            }
                        }
                    }
                }

                // 2D Histogram Configuration
                HistoConfig::Histo2D(histo2d) => {
                    if histo2d.calculate {
                        if histo2d.name.contains("{}") {
                            // Case 1: `{}` in `name`, `x_column_name` has a pattern
                            if let Some(caps) = range_re.captures(&histo2d.x_column_name) {
                                let start: usize = caps[1].parse().unwrap();
                                let end: usize = caps[2].parse().unwrap();
                                for i in start..=end {
                                    let mut new_config = histo2d.clone();
                                    new_config.name = histo2d.name.replace("{}", &i.to_string());
                                    new_config.x_column_name = range_re
                                        .replace(&histo2d.x_column_name, i.to_string())
                                        .to_string();
                                    new_config.y_column_name = histo2d.y_column_name.clone();
                                    histo2d_configs.push(new_config);
                                }
                            } else if let Some(caps) = list_re.captures(&histo2d.x_column_name) {
                                let values: Vec<&str> = caps[1].split(',').collect();
                                for val in values {
                                    let mut new_config = histo2d.clone();
                                    new_config.name = histo2d.name.replace("{}", val);
                                    new_config.x_column_name =
                                        list_re.replace(&histo2d.x_column_name, val).to_string();
                                    new_config.y_column_name = histo2d.y_column_name.clone();
                                    histo2d_configs.push(new_config);
                                }
                            }
                            // Case 2: `{}` in `name`, `y_column_name` has a pattern
                            else if let Some(caps) = range_re.captures(&histo2d.y_column_name) {
                                let start: usize = caps[1].parse().unwrap();
                                let end: usize = caps[2].parse().unwrap();
                                for i in start..=end {
                                    let mut new_config = histo2d.clone();
                                    new_config.name = histo2d.name.replace("{}", &i.to_string());
                                    new_config.x_column_name = histo2d.x_column_name.clone();
                                    new_config.y_column_name = range_re
                                        .replace(&histo2d.y_column_name, i.to_string())
                                        .to_string();
                                    histo2d_configs.push(new_config);
                                }
                            } else if let Some(caps) = list_re.captures(&histo2d.y_column_name) {
                                let values: Vec<&str> = caps[1].split(',').collect();
                                for val in values {
                                    let mut new_config = histo2d.clone();
                                    new_config.name = histo2d.name.replace("{}", val);
                                    new_config.x_column_name = histo2d.x_column_name.clone();
                                    new_config.y_column_name =
                                        list_re.replace(&histo2d.y_column_name, val).to_string();
                                    histo2d_configs.push(new_config);
                                }
                            } else {
                                log::error!(
                                "Warning: Unsupported pattern for 2D histogram with name '{}', x_column '{}', y_column '{}'",
                                histo2d.name, histo2d.x_column_name, histo2d.y_column_name
                            );
                            }
                        } else {
                            // Static `name`, expand `x_column_name` or `y_column_name` with range or list patterns
                            if let Some(caps) = range_re.captures(&histo2d.x_column_name) {
                                let start: usize = caps[1].parse().unwrap();
                                let end: usize = caps[2].parse().unwrap();
                                for i in start..=end {
                                    let mut new_config = histo2d.clone();
                                    new_config.x_column_name = range_re
                                        .replace(&histo2d.x_column_name, i.to_string())
                                        .to_string();
                                    histo2d_configs.push(new_config);
                                }
                            } else if let Some(caps) = list_re.captures(&histo2d.x_column_name) {
                                let values: Vec<&str> = caps[1].split(',').collect();
                                for val in values {
                                    let mut new_config = histo2d.clone();
                                    new_config.x_column_name =
                                        list_re.replace(&histo2d.x_column_name, val).to_string();
                                    histo2d_configs.push(new_config);
                                }
                            } else if let Some(caps) = range_re.captures(&histo2d.y_column_name) {
                                let start: usize = caps[1].parse().unwrap();
                                let end: usize = caps[2].parse().unwrap();
                                for i in start..=end {
                                    let mut new_config = histo2d.clone();
                                    new_config.y_column_name = range_re
                                        .replace(&histo2d.y_column_name, i.to_string())
                                        .to_string();
                                    histo2d_configs.push(new_config);
                                }
                            } else if let Some(caps) = list_re.captures(&histo2d.y_column_name) {
                                let values: Vec<&str> = caps[1].split(',').collect();
                                for val in values {
                                    let mut new_config = histo2d.clone();
                                    new_config.y_column_name =
                                        list_re.replace(&histo2d.y_column_name, val).to_string();
                                    histo2d_configs.push(new_config);
                                }
                            } else {
                                histo2d_configs.push(histo2d.clone());
                            }
                        }
                    }
                }
            }
        }

        // Pass expanded configurations to fill_histograms
        h.fill_histograms(
            histo1d_configs,
            histo2d_configs,
            &lf,
            self.new_columns.clone(),
            10000000,
        );
    }
}
