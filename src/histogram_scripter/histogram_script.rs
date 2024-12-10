// use super::configure_auxillary_detectors::AuxillaryDetectors;
use super::manual_histogram_scripts::sps_histograms;

use crate::histoer::configs::{Configs, Hist1DConfig, Hist2DConfig};
use crate::histoer::cuts::{Cut, Cut1D, Cut2D};
use crate::histoer::histogrammer::Histogrammer;
use egui_extras::{Column, TableBuilder};
use polars::prelude::*;
use rfd::FileDialog;
use serde_json;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct HistogramScript {
    pub hist_configs: Vec<Configs>, // Unified vector for both 1D and 2D configurations
    pub new_columns: Vec<(String, String)>,
    pub cuts: Vec<Cut>,
}

impl HistogramScript {
    pub fn new() -> Self {
        Self {
            hist_configs: vec![],
            new_columns: vec![],
            cuts: vec![],
        }
    }

    fn histogram_exists(&self, name: &str) -> bool {
        self.hist_configs.iter().any(|config| match config {
            Configs::Hist1D(hist) => hist.name == name,
            Configs::Hist2D(hist) => hist.name == name,
        })
    }

    fn column_exists(&self, alias: &str) -> bool {
        self.new_columns
            .iter()
            .any(|(_, col_alias)| col_alias == alias)
    }

    fn column_creation_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Column Creation");

            if ui.button("+").clicked() {
                self.new_columns.push(("".to_string(), "".to_string()));
            }

            ui.separator();

            if ui.button("Remove All").clicked() {
                self.new_columns.clear();
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
                        ui.label("Alias");
                    });
                    header.col(|ui| {
                        ui.label("Expression");
                    });
                })
                .body(|mut body| {
                    for (index, (expression, alias)) in self.new_columns.iter_mut().enumerate() {
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(alias)
                                        .hint_text("Alias")
                                        .clip_text(false),
                                );
                            });

                            row.col(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(expression)
                                        .hint_text("Expression")
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
    }

    fn histogram_script_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Custom Histogram Scripts");
        ui.horizontal(|ui| {
            if ui.button("SE-SPS").clicked() {
                let (columns, histograms, cuts) = sps_histograms();
                for histogram in histograms {
                    match &histogram {
                        Configs::Hist1D(histo1d) => {
                            if !self.histogram_exists(&histo1d.name) {
                                self.hist_configs.push(Configs::Hist1D(histo1d.clone()));
                            }
                        }
                        Configs::Hist2D(histo2d) => {
                            if !self.histogram_exists(&histo2d.name) {
                                self.hist_configs.push(Configs::Hist2D(histo2d.clone()));
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

                self.cuts = cuts;
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
    }

    fn cut_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Cuts");

            if ui.button("+1D").clicked() {
                // Add logic to create a new 1D cut and add it to `self.cuts`
                // For example:
                self.cuts.push(Cut::Cut1D(Cut1D::new("", "")));
            }

            if ui.button("+2D").clicked() {
                // Create a new instance of Cut2D and attempt to load it from a JSON file
                let mut new_cut2d = Cut2D::default();
                if new_cut2d.load_cut_from_json().is_ok() {
                    // If successfully loaded, add it to the cuts vector as a Cuts::Cut2D variant
                    self.cuts.push(Cut::Cut2D(new_cut2d));
                } else {
                    log::error!("Failed to load 2D cut from file.");
                }
            }

            ui.separator();

            if ui.button("Remove All").clicked() {
                self.cuts.clear();
                for hist_config in &mut self.hist_configs {
                    match hist_config {
                        Configs::Hist1D(hist1d) => {
                            hist1d.cuts.clear();
                        }
                        Configs::Hist2D(hist2d) => {
                            hist2d.cuts.clear();
                        }
                    }
                }
            }
        });

        if !self.cuts.is_empty() {
            let mut indices_to_remove_cut = Vec::new();

            let mut cuts_1d = Vec::new();
            let mut cuts_2d = Vec::new();

            self.cuts
                .iter_mut()
                .enumerate()
                .for_each(|(i, cut)| match cut {
                    Cut::Cut1D(_) => cuts_1d.push((i, cut)),
                    Cut::Cut2D(_) => cuts_2d.push((i, cut)),
                });

            // Render 1D Cuts Table
            if !cuts_1d.is_empty() {
                ui.label("1D Cuts");
                TableBuilder::new(ui)
                    .id_salt("cuts_1d_table")
                    .column(Column::auto()) // Name
                    .column(Column::auto()) // Expression
                    .column(Column::remainder()) // Actions
                    .striped(true)
                    .vscroll(false)
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.label("Name");
                        });
                        header.col(|ui| {
                            ui.label("Operation(s)");
                        });
                    })
                    .body(|mut body| {
                        for (index, cut1d) in cuts_1d {
                            body.row(18.0, |mut row| {
                                cut1d.table_row(&mut row);

                                row.col(|ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("Apply to All").clicked() {
                                            for hist_config in &mut self.hist_configs {
                                                match hist_config {
                                                    Configs::Hist1D(hist1d) => {
                                                        if !hist1d.cuts.contains(cut1d) {
                                                            hist1d.cuts.push(cut1d.clone());
                                                        }
                                                    }
                                                    Configs::Hist2D(hist2d) => {
                                                        if !hist2d.cuts.contains(cut1d) {
                                                            hist2d.cuts.push(cut1d.clone());
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if ui.button("Remove from All").clicked() {
                                            for hist_config in &mut self.hist_configs {
                                                match hist_config {
                                                    Configs::Hist1D(hist1d) => {
                                                        hist1d.cuts.retain(|cut| cut != cut1d);
                                                    }
                                                    Configs::Hist2D(hist2d) => {
                                                        hist2d.cuts.retain(|cut| cut != cut1d);
                                                    }
                                                }
                                            }
                                        }

                                        if ui.button("X").clicked() {
                                            indices_to_remove_cut.push(index);
                                        }
                                    });
                                });
                            });
                        }
                    });
            }

            if !cuts_2d.is_empty() {
                ui.label("2D Cuts");
                TableBuilder::new(ui)
                    .id_salt("cuts_2d_table")
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
                        for (index, cut2d) in cuts_2d {
                            body.row(18.0, |mut row| {
                                cut2d.table_row(&mut row);
                                row.col(|ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("Apply to All").clicked() {
                                            for hist_config in &mut self.hist_configs {
                                                match hist_config {
                                                    Configs::Hist1D(hist1d) => {
                                                        if !hist1d.cuts.contains(cut2d) {
                                                            hist1d.cuts.push(cut2d.clone());
                                                        }
                                                    }
                                                    Configs::Hist2D(hist2d) => {
                                                        if !hist2d.cuts.contains(cut2d) {
                                                            hist2d.cuts.push(cut2d.clone());
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if ui.button("Remove from All").clicked() {
                                            for hist_config in &mut self.hist_configs {
                                                match hist_config {
                                                    Configs::Hist1D(hist1d) => {
                                                        hist1d.cuts.retain(|cut| cut != cut2d);
                                                    }
                                                    Configs::Hist2D(hist2d) => {
                                                        hist2d.cuts.retain(|cut| cut != cut2d);
                                                    }
                                                }
                                            }
                                        }

                                        if ui.button("X").clicked() {
                                            indices_to_remove_cut.push(index);
                                        }
                                    });
                                });
                            });
                        }
                    });
            }

            for &index in indices_to_remove_cut.iter().rev() {
                self.cuts.remove(index);
            }
        }
    }

    fn histogram_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Histograms");

            if ui.button("+1D").clicked() {
                self.hist_configs.push(Configs::Hist1D(Hist1DConfig {
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
                self.hist_configs.push(Configs::Hist2D(Hist2DConfig {
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
                self.hist_configs.clear();
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
                for (index, config) in self.hist_configs.iter_mut().enumerate() {
                    body.row(18.0, |mut row| {
                        row.col(|ui| match config {
                            Configs::Hist1D(_) => {
                                ui.label(format!("{index}"));
                            }
                            Configs::Hist2D(_) => {
                                ui.label(format!("{index}"));
                            }
                        });

                        config.table_row(&mut row, &mut self.cuts);

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
            self.hist_configs.remove(index);
        }
    }

    fn verify_cuts(&mut self) {
        // Synchronize cuts after all UI interactions
        for hist_config in &mut self.hist_configs {
            match hist_config {
                Configs::Hist1D(hist1d) => {
                    for hist_cut in &mut hist1d.cuts {
                        if let Some(updated_cut) =
                            self.cuts.iter().find(|cut| cut.name() == hist_cut.name())
                        {
                            // Replace the cut if the operation or content has changed
                            *hist_cut = updated_cut.clone();
                        }
                    }

                    // Remove cuts that no longer exist in `self.cuts`
                    hist1d
                        .cuts
                        .retain(|cut| self.cuts.iter().any(|c| c.name() == cut.name()));
                }
                Configs::Hist2D(hist2d) => {
                    for hist_cut in &mut hist2d.cuts {
                        if let Some(updated_cut) =
                            self.cuts.iter().find(|cut| cut.name() == hist_cut.name())
                        {
                            // Replace the cut if the operation or content has changed
                            *hist_cut = updated_cut.clone();
                        }
                    }

                    // Remove cuts that no longer exist in `self.cuts`
                    hist2d
                        .cuts
                        .retain(|cut| self.cuts.iter().any(|c| c.name() == cut.name()));
                }
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.histogram_script_ui(ui);

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            self.column_creation_ui(ui);

            ui.separator();

            self.cut_ui(ui);

            ui.separator();

            self.histogram_ui(ui);
        });

        self.verify_cuts();
    }

    fn save_histogram_script(&self) -> io::Result<()> {
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

    fn load_histogram_script(&mut self) -> io::Result<()> {
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
                Configs::Hist1D(histo1d) => {
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
                Configs::Hist2D(histo2d) => {
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

        // Parse all conditions in histo1d_configs
        for config in &mut histo1d_configs {
            for cut in &mut config.cuts {
                if let Cut::Cut1D(cut1d) = cut {
                    cut1d.parse_conditions(); // Pre-parse the conditions
                }
            }
        }

        // Parse all conditions in histo2d_configs
        for config in &mut histo2d_configs {
            for cut in &mut config.cuts {
                if let Cut::Cut1D(cut1d) = cut {
                    cut1d.parse_conditions(); // Pre-parse the conditions
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
