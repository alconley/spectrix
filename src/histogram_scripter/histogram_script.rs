// use super::configure_auxillary_detectors::AuxillaryDetectors;
use super::configure_lazyframes::{LazyFrameInfo, LazyFrames};
use super::manual_histogram_scripts::sps_histograms;

use crate::histoer::histogrammer::{Histo1DConfig, Histo2DConfig, Histogrammer};
use egui_extras::{Column, TableBuilder};
use polars::prelude::*;

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct HistogramScript {
    pub lazyframe_info: LazyFrameInfo,
    pub hist_configs: Vec<HistoConfig>, // Unified vector for both 1D and 2D configurations
    pub new_columns: Vec<(String, String)>,
}

// Enum to encapsulate 1D and 2D histogram configurations
#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub enum HistoConfig {
    Histo1D(Histo1DConfig),
    Histo2D(Histo2DConfig),
}

impl HistogramScript {
    pub fn new() -> Self {
        Self {
            lazyframe_info: LazyFrameInfo::default(),
            hist_configs: vec![],
            new_columns: vec![],
        }
    }

    pub fn get_lazyframe_info(&mut self) {
        let mut lazyframe_info = LazyFrameInfo::default();

        let lazyframes = LazyFrames::new();
        let main_columns = lazyframes.main_column_names();
        let main_lf_names = lazyframes.main_lfs_names();

        lazyframe_info.lfs = main_lf_names;
        lazyframe_info.columns = main_columns;

        self.lazyframe_info = lazyframe_info;
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Custom Histogram Scripts");
        ui.horizontal(|ui| {
            if ui.button("SE-SPS").clicked() {
                let (columns, histograms) = sps_histograms();
                self.hist_configs = histograms;
                self.new_columns = columns;
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
            let mut indices_to_remove_column = Vec::new();

            ui.heading("Column Creation");

            TableBuilder::new(ui)
                .id_salt("new_columns")
                .column(Column::auto()) // expression
                .column(Column::auto()) // alias
                .column(Column::auto()) // Actions
                .striped(true)
                .vscroll(false)
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.label("Expression");
                    });
                    header.col(|ui| {
                        ui.label("Alias");
                    });
                    header.col(|ui| {
                        if ui.button("Add Column").clicked() {
                            self.new_columns.push(("".to_string(), "".to_string()));
                        }
                    });
                })
                .body(|mut body| {
                    for (index, (expression, alias)) in self.new_columns.iter_mut().enumerate() {
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

            ui.separator();

            ui.heading("Histograms");

            let mut indices_to_remove = Vec::new();

            // Create the table
            TableBuilder::new(ui)
                .id_salt("hist_configs")
                .column(Column::auto()) // Type
                .column(Column::auto()) // Name
                .column(Column::auto()) // Columns
                .column(Column::auto()) // Ranges
                .column(Column::auto()) // Bins
                .column(Column::auto()) // Actions
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
                                    ui.add_sized(
                                        ui.available_size(),
                                        egui::TextEdit::singleline(&mut hist.name)
                                            .hint_text("Name")
                                            .clip_text(false),
                                    );
                                }

                                HistoConfig::Histo2D(hist) => {
                                    ui.add_sized(
                                        ui.available_size(),
                                        egui::TextEdit::singleline(&mut hist.name)
                                            .hint_text("Name")
                                            .clip_text(false),
                                    );
                                }
                            });

                            row.col(|ui| match config {
                                HistoConfig::Histo1D(hist) => {
                                    ui.add_sized(
                                        ui.available_size(),
                                        egui::TextEdit::singleline(&mut hist.column_name)
                                            .hint_text("Column Name")
                                            .clip_text(false),
                                    );
                                }
                                HistoConfig::Histo2D(hist) => {
                                    ui.vertical(|ui| {
                                        ui.add_sized(
                                            ui.available_size(),
                                            egui::TextEdit::singleline(&mut hist.x_column_name)
                                                .hint_text("X Column Name")
                                                .clip_text(false),
                                        );
                                        ui.add_sized(
                                            ui.available_size(),
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

    pub fn add_histograms(&mut self, h: &mut Histogrammer, lf: LazyFrame) {
        // form the 1d and 2d histo congifurations vecs
        let mut histo1d_configs = Vec::new();
        let mut histo2d_configs = Vec::new();

        for config in self.hist_configs.iter() {
            match config {
                HistoConfig::Histo1D(histo1d) => {
                    histo1d_configs.push(histo1d.clone());
                }
                HistoConfig::Histo2D(histo2d) => {
                    histo2d_configs.push(histo2d.clone());
                }
            }
        }

        h.fill_histograms(
            histo1d_configs,
            histo2d_configs,
            &lf,
            self.new_columns.clone(),
            10000000,
        );
    }
}
