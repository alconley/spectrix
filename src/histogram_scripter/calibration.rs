use crate::histoer::ui_helpers::{precise_drag_value, searchable_column_picker_ui};

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::PathBuf;

const DEFAULT_A: f64 = 0.0;
const DEFAULT_B: f64 = 1.0;
const DEFAULT_C: f64 = 0.0;
const DEFAULT_FILE_NAME: &str = "histogram_calibration.csv";
const EXPECTED_HEADER: &str = "column_name,a,b,c,output_column_name";

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct CalibrationRow {
    pub column_name: String,
    pub output_column_name: String,
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

impl CalibrationRow {
    fn new(column_name: &str) -> Self {
        Self {
            column_name: column_name.to_owned(),
            output_column_name: column_name.to_owned(),
            a: DEFAULT_A,
            b: DEFAULT_B,
            c: DEFAULT_C,
        }
    }

    fn reset_to_default(&mut self) {
        self.output_column_name = self.column_name.clone();
        self.a = DEFAULT_A;
        self.b = DEFAULT_B;
        self.c = DEFAULT_C;
    }

    fn is_active(&self) -> bool {
        (self.a - DEFAULT_A).abs() > f64::EPSILON
            || (self.b - DEFAULT_B).abs() > f64::EPSILON
            || (self.c - DEFAULT_C).abs() > f64::EPSILON
    }

    fn effective_output_column_name(&self) -> String {
        sanitize_output_column_name(&self.output_column_name, &self.column_name)
    }

    fn computed_column(&self) -> Option<(String, String)> {
        if !self.is_active() {
            return None;
        }

        let column_name = self.column_name.trim();
        if column_name.is_empty() {
            return None;
        }

        Some((
            format!(
                "({})*{}**2 + ({})*{} + ({})",
                format_number(self.a),
                column_name,
                format_number(self.b),
                column_name,
                format_number(self.c)
            ),
            self.effective_output_column_name(),
        ))
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct CalibrationScript {
    pub rows: Vec<CalibrationRow>,
    pub sort_ascending: bool,
    #[serde(skip)]
    add_column_candidate: String,
    #[serde(skip)]
    search_query: String,
    #[serde(skip)]
    status_message: Option<String>,
    #[serde(skip)]
    status_is_warning: bool,
    loaded_csv_path: Option<PathBuf>,
    loaded_csv_columns: Vec<String>,
}

impl Default for CalibrationScript {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            sort_ascending: true,
            add_column_candidate: String::new(),
            search_query: String::new(),
            status_message: None,
            status_is_warning: false,
            loaded_csv_path: None,
            loaded_csv_columns: Vec::new(),
        }
    }
}

impl CalibrationScript {
    pub fn sync_columns(&mut self, column_names: &[String]) {
        let remaining_rows = self
            .rows
            .iter()
            .map(|row| row.column_name.as_str())
            .collect::<HashSet<_>>();
        self.loaded_csv_columns
            .retain(|column| remaining_rows.contains(column.as_str()));
        if self.loaded_csv_columns.is_empty() {
            self.loaded_csv_path = None;
        }

        if !column_names
            .iter()
            .any(|column| column == &self.add_column_candidate)
            || self
                .rows
                .iter()
                .any(|row| row.column_name == self.add_column_candidate)
        {
            self.add_column_candidate.clear();
        }
    }

    pub fn computed_columns(&self, column_names: &[String]) -> Vec<(String, String)> {
        let row_lookup = self
            .rows
            .iter()
            .map(|row| (row.column_name.as_str(), row))
            .collect::<HashMap<_, _>>();

        column_names
            .iter()
            .filter_map(|column_name| row_lookup.get(column_name.as_str()))
            .filter_map(|row| row.computed_column())
            .collect()
    }

    pub fn output_columns(&self, column_names: &[String]) -> Vec<String> {
        self.computed_columns(column_names)
            .into_iter()
            .map(|(_, alias)| alias)
            .collect()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, column_names: &[String]) {
        self.sync_columns(column_names);
        let loaded_columns = column_names
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();

        ui.horizontal_wrapped(|ui| {
            if ui
                .button("Defaults")
                .on_hover_text(
                    "Reset all calibration rows to A=0, B=1, C=0 and set each output column name back to the source column name.",
                )
                .clicked()
            {
                self.reset_to_defaults();
            }

            let clear_all_response = ui.add_enabled(
                !self.rows.is_empty() || self.loaded_csv_path.is_some(),
                egui::Button::new("Clear All"),
            );
            let clear_all_response = if !self.rows.is_empty() || self.loaded_csv_path.is_some() {
                clear_all_response.on_hover_text(
                    "Remove every calibration row and clear the loaded calibration CSV association.",
                )
            } else {
                clear_all_response
                    .on_disabled_hover_text("There is no calibration state to clear.")
            };
            if clear_all_response.clicked() {
                self.clear_all();
            }

            if ui
                .button("Import [.csv]")
                .on_hover_text(
                    "Load calibration values from a CSV. Rows are kept even if the current file schema does not contain those columns yet.",
                )
                .clicked()
            {
                self.load_csv(column_names);
            }

            let export_enabled = !self.rows.is_empty();
            let export_response =
                ui.add_enabled(export_enabled, egui::Button::new("Export [.csv]"));
            let export_response = if export_enabled {
                export_response.on_hover_text(
                    "Export all calibration rows to a CSV file.",
                )
            } else {
                export_response.on_disabled_hover_text("There are no calibration rows to export.")
            };

            if export_response.clicked() {
                self.save_csv();
            }

            ui.separator();
            ui.horizontal( |ui| {
                ui.label("Search");
                ui.add(egui::TextEdit::singleline(&mut self.search_query).hint_text("Column name"));    
            });

            if ui
                .button(if self.sort_ascending {
                    "Sort A-Z"
                } else {
                    "Sort Z-A"
                })
                .on_hover_text("Toggle column-name sort order")
                .clicked()
            {
                self.sort_ascending = !self.sort_ascending;
            }
        });

        ui.horizontal_wrapped(|ui| {
            ui.label("Equation:");
            ui.monospace("output = A*input^2 + B*input + C");
        });

        ui.horizontal_wrapped(|ui| {
            let loaded_file_label = self
                .loaded_csv_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "No calibration CSV loaded".to_owned());
            let has_loaded_csv = self.loaded_csv_path.is_some();

            ui.label("Calibration file:");
            let response = ui
                .add(egui::Label::new(egui::RichText::new(&loaded_file_label).monospace()).wrap());
            response.on_hover_text(loaded_file_label);

            if ui
                .add_enabled(has_loaded_csv, egui::Button::new("X"))
                .on_disabled_hover_text("No calibration CSV is currently loaded.")
                .on_hover_text("Remove the loaded calibration CSV and clear its imported values.")
                .clicked()
            {
                self.clear_loaded_csv(column_names);
            }
        });

        ui.label(
            egui::RichText::new(
                "Matching an existing output column name overwrites that column when A/B/C are not 0, 1, 0.",
            )
            .weak()
            .small(),
        );

        if let Some(message) = &self.status_message {
            let rich_text = egui::RichText::new(message).small();
            if self.status_is_warning {
                ui.label(rich_text);
            } else {
                ui.label(rich_text.weak());
            }
        }

        let unavailable_row_count = self.unavailable_row_count(&loaded_columns);
        if column_names.is_empty() {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                "No file columns are currently loaded, so calibration rows are shown in red and skipped during calculation.",
            );
        } else if unavailable_row_count > 0 {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                "Red rows are not present in the currently loaded file columns and will not be calculated.",
            );
        }

        let addable_columns = self.addable_columns(column_names);
        ui.horizontal_wrapped(|ui| {
            let add_response = ui.add_enabled(
                !self.add_column_candidate.trim().is_empty(),
                egui::Button::new("+"),
            );
            let add_response = if !self.add_column_candidate.trim().is_empty() {
                add_response.on_hover_text("Add the selected column to the calibration table.")
            } else {
                add_response.on_disabled_hover_text("Choose a current file column to add.")
            };
            if add_response.clicked() {
                self.add_candidate_row();
            }

            searchable_column_picker_ui(
                ui,
                "calibration_add_column_picker",
                &mut self.add_column_candidate,
                &addable_columns,
                "Add column",
                !addable_columns.is_empty(),
            );
        });

        let filtered_indices = self.filtered_row_indices();
        let active_row_count = self.rows.iter().filter(|row| row.is_active()).count();
        let unavailable_row_count = self.unavailable_row_count(&loaded_columns);
        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "Showing {} of {}",
                filtered_indices.len(),
                self.rows.len()
            ));
            ui.label(format!("Active: {active_row_count}"));
            if unavailable_row_count > 0 {
                ui.colored_label(
                    egui::Color32::LIGHT_RED,
                    format!("Missing: {unavailable_row_count}"),
                );
            }
        });

        ui.separator();

        egui::ScrollArea::both()
            .id_salt("calibration_grid_scroll")
            .show(ui, |ui| {
                let mut rows_to_remove = Vec::new();
                egui::Grid::new("calibration_grid")
                    .num_columns(5)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Column Name");
                        ui.strong("A");
                        ui.strong("B");
                        ui.strong("C");
                        ui.strong("Output Column Name");
                        ui.end_row();

                        for &index in &filtered_indices {
                            let calibration = &mut self.rows[index];
                            if calibration.output_column_name.trim().is_empty() {
                                calibration.output_column_name = calibration.column_name.clone();
                            }
                            let row_is_available =
                                loaded_columns.contains(calibration.column_name.as_str());
                            let mut remove_row = false;

                            calibration_cell(ui, row_is_available, |ui| {
                                let text = if calibration.is_active() {
                                    egui::RichText::new(&calibration.column_name).strong()
                                } else {
                                    egui::RichText::new(&calibration.column_name)
                                };
                                let response = ui.label(text);
                                if row_is_available {
                                    response.on_hover_text(&calibration.column_name);
                                } else {
                                    response.on_hover_text(format!(
                                        "{}\nMissing from the currently loaded file columns. This row will be skipped.",
                                        calibration.column_name
                                    ));
                                }
                            });

                            calibration_cell(ui, row_is_available, |ui| {
                                ui.add(precise_drag_value(&mut calibration.a).speed(0.01));
                            });

                            calibration_cell(ui, row_is_available, |ui| {
                                ui.add(precise_drag_value(&mut calibration.b).speed(0.01));
                            });

                            calibration_cell(ui, row_is_available, |ui| {
                                ui.add(precise_drag_value(&mut calibration.c).speed(0.01));
                            });

                            calibration_cell(ui, row_is_available, |ui| {
                                let response = ui
                                    .with_layout(
                                        egui::Layout::right_to_left(egui::Align::Min),
                                        |ui| {
                                            if ui.button("X").clicked() {
                                                remove_row = true;
                                            }

                                            ui.add_sized(
                                                [ui.available_width().max(0.0), 0.0],
                                                egui::TextEdit::singleline(
                                                    &mut calibration.output_column_name,
                                                )
                                                .desired_width(0.0)
                                                .clip_text(true),
                                            )
                                        },
                                    )
                                    .inner;

                                if response.changed() {
                                    calibration.output_column_name = sanitize_output_column_name(
                                        &calibration.output_column_name,
                                        &calibration.column_name,
                                    );
                                }
                            });

                            if remove_row {
                                rows_to_remove.push(index);
                            }

                            ui.end_row();
                        }
                    });

                if !rows_to_remove.is_empty() {
                    rows_to_remove.sort_unstable();
                    rows_to_remove.dedup();
                    for index in rows_to_remove.into_iter().rev() {
                        self.remove_row(index);
                    }
                }
            });
    }

    fn addable_columns(&self, column_names: &[String]) -> Vec<String> {
        let existing = self
            .rows
            .iter()
            .map(|row| row.column_name.as_str())
            .collect::<HashSet<_>>();

        let mut addable = column_names
            .iter()
            .filter(|column_name| !existing.contains(column_name.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        addable.sort();
        addable
    }

    fn unavailable_row_count(&self, loaded_columns: &HashSet<&str>) -> usize {
        self.rows
            .iter()
            .filter(|row| !loaded_columns.contains(row.column_name.as_str()))
            .count()
    }

    fn filtered_row_indices(&self) -> Vec<usize> {
        let search_query = self.search_query.trim();
        if search_query.is_empty() {
            self.row_indices(None)
        } else {
            self.row_indices(Some(search_query))
        }
    }

    fn row_indices(&self, search_query: Option<&str>) -> Vec<usize> {
        let normalized_query = search_query.unwrap_or_default().trim().to_ascii_lowercase();

        let mut indices = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                normalized_query.is_empty()
                    || row
                        .column_name
                        .to_ascii_lowercase()
                        .contains(&normalized_query)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        indices.sort_by(|left, right| {
            let left_name = self.rows[*left].column_name.to_ascii_lowercase();
            let right_name = self.rows[*right].column_name.to_ascii_lowercase();
            let ordering = left_name.cmp(&right_name).then_with(|| {
                self.rows[*left]
                    .column_name
                    .cmp(&self.rows[*right].column_name)
            });
            if self.sort_ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });

        indices
    }

    fn save_csv(&mut self) {
        if self.rows.is_empty() {
            self.set_status("No calibration rows are available to export.", true);
            return;
        }

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name(DEFAULT_FILE_NAME)
            .save_file()
        {
            let mut file = match File::create(&path) {
                Ok(file) => file,
                Err(error) => {
                    let message = format!("Failed to create calibration CSV: {error}");
                    log::error!("{message}");
                    self.set_status(message, true);
                    return;
                }
            };

            if let Err(error) = writeln!(file, "{EXPECTED_HEADER}") {
                let message = format!("Failed to write calibration CSV header: {error}");
                log::error!("{message}");
                self.set_status(message, true);
                return;
            }

            for row in &self.rows {
                if let Err(error) = writeln!(
                    file,
                    "{},{},{},{},{}",
                    escape_csv(&row.column_name),
                    format_number(row.a),
                    format_number(row.b),
                    format_number(row.c),
                    escape_csv(&row.effective_output_column_name()),
                ) {
                    let message = format!(
                        "Failed to write calibration row for '{}': {error}",
                        row.column_name
                    );
                    log::error!("{message}");
                    self.set_status(message, true);
                    return;
                }
            }

            let message = format!(
                "Exported {} calibration row(s) to {}.",
                self.rows.len(),
                path.display()
            );
            log::info!("{message}");
            self.set_status(message, false);
        }
    }

    fn load_csv(&mut self, column_names: &[String]) {
        self.sync_columns(column_names);

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .pick_file()
        {
            let file = match File::open(&path) {
                Ok(file) => file,
                Err(error) => {
                    let message = format!("Failed to open calibration CSV: {error}");
                    log::error!("{message}");
                    self.set_status(message, true);
                    return;
                }
            };

            let mut reader = BufReader::new(file);
            let mut header = String::new();
            if let Err(error) = reader.read_line(&mut header) {
                let message = format!("Failed to read calibration CSV header: {error}");
                log::error!("{message}");
                self.set_status(message, true);
                return;
            }

            let header_columns = split_csv(header.trim());
            let (layout, header_warning) = CalibrationCsvLayout::from_header(&header_columns);

            let loaded_columns = column_names
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>();
            let mut row_indices = self
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| (row.column_name.clone(), index))
                .collect::<HashMap<_, _>>();

            let mut matched_rows = 0usize;
            let mut created_rows = 0usize;
            let mut missing_column_rows = 0usize;
            let mut invalid_numeric_values = 0usize;
            let mut sanitized_output_names = 0usize;
            let mut rows_missing_from_loaded_schema = HashSet::new();
            let mut loaded_csv_columns = Vec::new();

            for (line_offset, line_result) in reader.lines().enumerate() {
                let line_number = line_offset + 2;
                let line = match line_result {
                    Ok(line) => line.trim().to_owned(),
                    Err(error) => {
                        log::warn!("Failed to read calibration CSV line {line_number}: {error}");
                        missing_column_rows += 1;
                        continue;
                    }
                };

                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                let cells = split_csv(&line);
                let Some(column_name) = layout
                    .column_name
                    .and_then(|index| cells.get(index))
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                else {
                    log::warn!(
                        "Skipping calibration CSV line {line_number}: no column name was found."
                    );
                    missing_column_rows += 1;
                    continue;
                };

                let row_index = if let Some(&row_index) = row_indices.get(column_name) {
                    row_index
                } else {
                    self.rows.push(CalibrationRow::new(column_name));
                    let row_index = self.rows.len() - 1;
                    row_indices.insert(column_name.to_owned(), row_index);
                    created_rows += 1;
                    row_index
                };

                let row = &mut self.rows[row_index];
                matched_rows += 1;
                loaded_csv_columns.push(row.column_name.clone());
                if !loaded_columns.contains(row.column_name.as_str()) {
                    rows_missing_from_loaded_schema.insert(row.column_name.clone());
                }

                for (index, target) in [
                    (layout.a, &mut row.a),
                    (layout.b, &mut row.b),
                    (layout.c, &mut row.c),
                ] {
                    if let Some(value) = index.and_then(|field_index| cells.get(field_index))
                        && !value.trim().is_empty()
                    {
                        match value.trim().parse::<f64>() {
                            Ok(parsed) => *target = parsed,
                            Err(_) => invalid_numeric_values += 1,
                        }
                    }
                }

                if let Some(output_index) = layout.output_column_name
                    && let Some(value) = cells.get(output_index)
                {
                    let sanitized = sanitize_output_column_name(value, &row.column_name);
                    if !value.trim().is_empty() && sanitized != value.trim() {
                        sanitized_output_names += 1;
                    }
                    row.output_column_name = sanitized;
                }
            }

            let mut issues = Vec::new();
            if header_warning {
                issues.push(
                    "the header did not fully match the expected format, so Spectrix used whatever columns it could recognize"
                        .to_owned(),
                );
            }
            if created_rows > 0 {
                issues.push(format!(
                    "{created_rows} new calibration row(s) were added because they were not already in the table"
                ));
            }
            if missing_column_rows > 0 {
                issues.push(format!(
                    "{missing_column_rows} row(s) were missing a usable column name"
                ));
            }
            if invalid_numeric_values > 0 {
                issues.push(format!(
                    "{invalid_numeric_values} coefficient value(s) were invalid and kept their previous values"
                ));
            }
            if sanitized_output_names > 0 {
                issues.push(format!(
                    "{sanitized_output_names} output column name(s) were sanitized to valid identifiers"
                ));
            }
            if !rows_missing_from_loaded_schema.is_empty() {
                issues.push(format!(
                    "{} row(s) are not in the currently loaded file columns and are shown in red until a matching file schema is loaded",
                    rows_missing_from_loaded_schema.len()
                ));
            }

            let mut message = format!(
                "Loaded {} calibration row(s) from {}.",
                matched_rows,
                path.display()
            );
            if !issues.is_empty() {
                message.push(' ');
                message.push_str("Warning: ");
                message.push_str(&issues.join("; "));
            }

            if issues.is_empty() {
                log::info!("{message}");
            } else {
                log::warn!("{message}");
            }
            self.loaded_csv_path = Some(path);
            loaded_csv_columns.sort();
            loaded_csv_columns.dedup();
            self.loaded_csv_columns = loaded_csv_columns;
            self.sync_columns(column_names);
            self.set_status(message, !issues.is_empty());
        }
    }

    fn set_status(&mut self, message: impl Into<String>, is_warning: bool) {
        self.status_message = Some(message.into());
        self.status_is_warning = is_warning;
    }

    fn add_candidate_row(&mut self) {
        let column_name = self.add_column_candidate.trim();
        if column_name.is_empty() || self.rows.iter().any(|row| row.column_name == column_name) {
            return;
        }

        self.rows.push(CalibrationRow::new(column_name));
        self.add_column_candidate.clear();
        self.set_status("Added a calibration row.", false);
    }

    fn reset_to_defaults(&mut self) {
        for row in &mut self.rows {
            row.reset_to_default();
        }
        self.set_status("Reset all calibration rows to their default values.", false);
    }

    fn clear_all(&mut self) {
        self.rows.clear();
        self.loaded_csv_path = None;
        self.loaded_csv_columns.clear();
        self.add_column_candidate.clear();
        self.search_query.clear();
        self.set_status(
            "Cleared all calibration rows and the loaded calibration CSV.",
            false,
        );
    }

    fn remove_row(&mut self, index: usize) {
        if index >= self.rows.len() {
            return;
        }

        let removed_column = self.rows[index].column_name.clone();
        self.rows.remove(index);
        self.loaded_csv_columns
            .retain(|column| column != &removed_column);
        if self.loaded_csv_columns.is_empty() {
            self.loaded_csv_path = None;
        }
        self.set_status(
            format!("Removed calibration row '{removed_column}'."),
            false,
        );
    }

    fn clear_loaded_csv(&mut self, column_names: &[String]) {
        if self.loaded_csv_path.is_none() && self.loaded_csv_columns.is_empty() {
            self.set_status("No calibration CSV is currently loaded.", true);
            return;
        }

        let loaded_csv_columns = self
            .loaded_csv_columns
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let current_columns = column_names
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();

        for row in &mut self.rows {
            if loaded_csv_columns.contains(row.column_name.as_str()) {
                row.reset_to_default();
            }
        }

        self.rows.retain(|row| {
            current_columns.contains(row.column_name.as_str())
                || !loaded_csv_columns.contains(row.column_name.as_str())
        });

        self.loaded_csv_path = None;
        self.loaded_csv_columns.clear();
        self.set_status(
            "Removed the loaded calibration CSV and cleared its imported calibration values.",
            false,
        );
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct CalibrationCsvLayout {
    column_name: Option<usize>,
    a: Option<usize>,
    b: Option<usize>,
    c: Option<usize>,
    output_column_name: Option<usize>,
}

impl CalibrationCsvLayout {
    fn from_header(header: &[String]) -> (Self, bool) {
        let mut layout = Self::default();

        for (index, value) in header.iter().enumerate() {
            match normalize_header_name(value).as_str() {
                "columnname" => layout.column_name = Some(index),
                "a" => layout.a = Some(index),
                "b" => layout.b = Some(index),
                "c" => layout.c = Some(index),
                "outputcolumnname" | "output" | "alias" => {
                    layout.output_column_name = Some(index);
                }
                _ => {}
            }
        }

        let recognized_fields = [
            layout.column_name,
            layout.a,
            layout.b,
            layout.c,
            layout.output_column_name,
        ]
        .into_iter()
        .flatten()
        .count();

        let mut warning = false;
        if recognized_fields == 0 {
            layout.column_name = Some(0);
            layout.a = Some(1);
            layout.b = Some(2);
            layout.c = Some(3);
            layout.output_column_name = Some(4);
            warning = true;
        } else {
            if layout.column_name.is_none() {
                layout.column_name = Some(0);
                warning = true;
            }

            if layout.a.is_none() || layout.b.is_none() || layout.c.is_none() {
                warning = true;
            }
        }

        if header.is_empty() {
            warning = true;
        }

        (layout, warning)
    }
}

fn format_number(value: f64) -> String {
    let mut formatted = format!("{value:.15}");
    if formatted.contains('.') {
        formatted = formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned();
    }

    if formatted == "-0" {
        "0".to_owned()
    } else {
        formatted
    }
}

fn normalize_header_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn escape_csv(value: &str) -> String {
    if value.contains(',') || value.contains('"') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn split_csv(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut characters = line.chars().peekable();

    while let Some(character) = characters.next() {
        match character {
            '"' => {
                if in_quotes && characters.peek() == Some(&'"') {
                    current.push('"');
                    _ = characters.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                values.push(current.trim().to_owned());
                current.clear();
            }
            _ => current.push(character),
        }
    }

    values.push(current.trim().to_owned());
    values
}

fn sanitize_output_column_name(output_name: &str, default_name: &str) -> String {
    let trimmed = output_name.trim();
    if trimmed.is_empty() || trimmed == default_name {
        return default_name.to_owned();
    }

    let mut sanitized = String::with_capacity(trimmed.len());
    let mut previous_was_underscore = false;

    for character in trimmed.chars() {
        let mapped = if character.is_ascii_alphanumeric() || character == '_' {
            character
        } else {
            '_'
        };

        if mapped == '_' {
            if !previous_was_underscore {
                sanitized.push(mapped);
            }
            previous_was_underscore = true;
        } else {
            sanitized.push(mapped);
            previous_was_underscore = false;
        }
    }

    if sanitized.is_empty() {
        return default_name.to_owned();
    }

    if sanitized
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        sanitized.insert(0, '_');
    }

    sanitized
}

fn calibration_cell(
    ui: &mut egui::Ui,
    row_is_available: bool,
    add_cell: impl FnOnce(&mut egui::Ui),
) {
    ui.scope(|ui| {
        if !row_is_available {
            ui.visuals_mut().override_text_color = Some(egui::Color32::LIGHT_RED);
        }
        add_cell(ui);
    });
}
