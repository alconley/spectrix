use geo::Contains as _;
use regex::Regex;
use std::fs::File;
use std::io::{BufReader, Write as _};
use std::ops::{BitAnd as _, BitOr as _};
use std::path::{Path, PathBuf};

use polars::prelude::*;

use crate::egui_plot_stuff::egui_polygon::EguiPolygon;
use crate::histoer::ui_helpers::{
    SearchableColumnPickerSize, precise_drag_value, searchable_column_picker_with_width_ui,
};
use egui_extras::{Column, TableBuilder};

pub(crate) fn sanitize_cut_file_name_component(name: &str, fallback: &str) -> String {
    let trimmed_name = name.trim();
    let base_name = if trimmed_name.is_empty() {
        fallback
    } else {
        trimmed_name
    };

    let mut collapsed = String::with_capacity(base_name.len());
    let mut previous_was_underscore = false;

    for character in base_name.chars() {
        let mapped = if character.is_ascii_alphanumeric() || character == '-' {
            character
        } else {
            '_'
        };

        if mapped == '_' {
            if !previous_was_underscore {
                collapsed.push('_');
            }
            previous_was_underscore = true;
        } else {
            collapsed.push(mapped);
            previous_was_underscore = false;
        }
    }

    let trimmed = collapsed.trim_matches('_');
    if trimmed.is_empty() {
        fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum Cut {
    Cut1D(Cut1D),
    Cut2D(Cut2D),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ActiveHistogramCut {
    pub histogram_name: String,
    pub enabled: bool,
    pub cut: Cut,
}

impl Default for Cut {
    fn default() -> Self {
        Self::Cut2D(Cut2D::default())
    }
}

impl Cut {
    // Method to check if a cut is valid for a specific row in the DataFrame
    pub fn valid(&self, df: &DataFrame, row_idx: usize) -> bool {
        match self {
            Self::Cut1D(cut1d) => cut1d.valid(df, row_idx),
            Self::Cut2D(cut2d) => cut2d.valid(df, row_idx),
        }
    }

    pub fn table_row(
        &mut self,
        row: &mut egui_extras::TableRow<'_, '_>,
        row_index: usize,
        is_selected: bool,
    ) -> bool {
        match self {
            Self::Cut1D(cut1d) => cut1d.table_row(row, row_index, is_selected),
            Self::Cut2D(cut2d) => {
                cut2d.table_row(row);
                false
            }
        }
    }

    pub fn table_row_height(&self) -> f32 {
        match self {
            Self::Cut1D(_) => Cut1D::table_row_height(),
            Self::Cut2D(_) => 28.0,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Cut1D(cut1d) => &cut1d.name,
            Self::Cut2D(cut2d) => &cut2d.polygon.name,
        }
    }

    /// Returns the column(s) required by the cut
    pub fn required_columns(&self) -> Vec<String> {
        match self {
            Self::Cut1D(cut1d) => cut1d.required_columns(),
            Self::Cut2D(cut2d) => cut2d.required_columns(),
        }
    }

    pub fn new_1d(name: &str, expression: &str) -> Self {
        Self::Cut1D(Cut1D::new(name, expression))
    }

    pub fn save_button(&mut self, ui: &mut egui::Ui) {
        match self {
            Self::Cut1D(cut1d) => cut1d.save_button(ui),
            Self::Cut2D(cut2d) => cut2d.save_button(ui),
        }
    }

    pub fn info_button(&self, ui: &mut egui::Ui, histogram_description: Option<String>) {
        match self {
            Self::Cut1D(cut1d) => cut1d.info_button(ui, histogram_description),
            Self::Cut2D(cut2d) => cut2d.info_button(ui, histogram_description),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default)]
pub struct Cuts {
    pub cuts: Vec<Cut>,
    pub cut_folder: Option<PathBuf>,
    #[serde(skip)]
    pub selected_cut_index: Option<usize>,
}

impl Cuts {
    fn active_cut_rows(ui: &mut egui::Ui, active_cuts: &mut [ActiveHistogramCut], id_suffix: &str) {
        if active_cuts.is_empty() {
            return;
        }

        ui.separator();

        ui.label("Active Histogram Cuts");
        TableBuilder::new(ui)
            .id_salt(format!("active_histogram_cuts_{id_suffix}"))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .striped(true)
            .vscroll(false)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("Use");
                });
                header.col(|ui| {
                    ui.label("Name");
                });
                header.col(|ui| {
                    ui.label("Save");
                });
                header.col(|ui| {
                    ui.label("Info");
                });
            })
            .body(|mut body| {
                for active_cut in active_cuts {
                    body.row(18.0, |mut row| {
                        row.col(|ui| {
                            ui.checkbox(&mut active_cut.enabled, "");
                        });
                        row.col(|ui| {
                            ui.add(
                                egui::Label::new(active_cut.cut.name())
                                    .wrap_mode(egui::TextWrapMode::Extend),
                            );
                        });
                        row.col(|ui| {
                            active_cut.cut.save_button(ui);
                        });
                        row.col(|ui| {
                            active_cut.cut.info_button(
                                ui,
                                Some(format!("Histogram: {}", active_cut.histogram_name)),
                            );
                        });
                    });
                }
            });
    }

    pub fn merged_with_active_cuts(&self, active_cuts: Option<&[ActiveHistogramCut]>) -> Self {
        let mut merged = self.clone();

        if let Some(active_cuts) = active_cuts {
            for active_cut in active_cuts.iter().filter(|active_cut| active_cut.enabled) {
                if let Some(existing_cut) = merged
                    .cuts
                    .iter_mut()
                    .find(|existing_cut| existing_cut.name() == active_cut.cut.name())
                {
                    *existing_cut = active_cut.cut.clone();
                } else {
                    merged.cuts.push(active_cut.cut.clone());
                }
            }
        }

        merged
    }

    pub fn new(cuts: Vec<Cut>) -> Self {
        Self {
            cuts,
            cut_folder: None,
            selected_cut_index: None,
        }
    }

    pub fn get_active_cuts(&self) -> Self {
        let active_cuts = self
            .cuts
            .iter()
            .filter(|cut| match cut {
                Cut::Cut1D(cut1d) => cut1d.active,
                Cut::Cut2D(cut2d) => cut2d.active,
            })
            .cloned()
            .collect();
        Self::new(active_cuts)
    }

    pub fn is_empty(&self) -> bool {
        self.cuts.is_empty()
    }

    pub fn get_cuts_in_folder(&self) -> Vec<Cut> {
        let mut cuts = Vec::new();

        if let Some(folder) = &self.cut_folder
            && folder.exists()
            && folder.is_dir()
            && let Ok(entries) = std::fs::read_dir(folder)
        {
            for entry in entries {
                let path = match entry {
                    Ok(entry) => entry.path(),
                    Err(e) => {
                        log::error!("Failed to read cut entry in {}: {e}", folder.display());
                        continue;
                    }
                };
                if let Some(ext) = path.extension()
                    && ext == "json"
                {
                    let content = match std::fs::read_to_string(&path) {
                        Ok(content) => content,
                        Err(e) => {
                            log::error!("Failed to read cut file {}: {e}", path.display());
                            continue;
                        }
                    };
                    let cut1d: Result<Cut1D, _> = serde_json::from_str(&content);
                    if let Ok(mut cut) = cut1d {
                        cut.active = false; // Set active to false by default
                        cut.normalize_after_load();
                        cut.saved_path = Some(path.clone());
                        cuts.push(Cut::Cut1D(cut));
                        continue;
                    }

                    let cut2d: Result<Cut2D, _> = serde_json::from_str(&content);
                    if let Ok(mut cut) = cut2d {
                        cut.active = false; // Set active to false by default
                        cut.saved_path = Some(path.clone());
                        cut.normalize_after_load();
                        cuts.push(Cut::Cut2D(cut));
                        continue;
                    }

                    log::error!("Invalid cut file: {}. Skipping...", path.display());
                }
            }
        }

        cuts
    }
    // Add a new cut
    pub fn add_cut(&mut self, cut: Cut) {
        if self.cuts.iter().any(|c| c.name() == cut.name()) {
            log::error!("Cut with name '{}' already exists.", cut.name());
        } else {
            self.cuts.push(cut);
        }
    }

    // Remove a cut by name
    pub fn remove_cut(&mut self, name: &str) {
        if let Some(pos) = self.cuts.iter().position(|cut| cut.name() == name) {
            self.cuts.remove(pos);
        } else {
            log::error!("No cut with name '{name}' found.");
        }
    }

    pub fn merge(&mut self, other: &Self) {
        for cut in &other.cuts {
            if !self.cuts.iter().any(|c| c.name() == cut.name()) {
                self.cuts.push(cut.clone());
            }
        }
    }

    pub fn parse_conditions(&mut self) {
        for cut in &mut self.cuts {
            match cut {
                Cut::Cut1D(cut1d) => cut1d.parse_conditions(),
                Cut::Cut2D(_) => {}
            }
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        active_cuts: Option<&mut [ActiveHistogramCut]>,
        available_columns: &[String],
        id_suffix: &str,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Cuts");

            if ui.button("+1D Manual").clicked() {
                self.cuts.push(Cut::Cut1D(Cut1D::builder_default()));
                self.selected_cut_index = Some(self.cuts.len() - 1);
            }

            if ui.button("+1D Load").clicked() {
                let mut new_cut1d = Cut1D::default();
                if new_cut1d.load_cut_from_json().is_ok() {
                    self.cuts.push(Cut::Cut1D(new_cut1d));
                    self.selected_cut_index = Some(self.cuts.len() - 1);
                } else {
                    log::error!("Failed to load 1D cut from file.");
                }
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

            if ui.button("Add Cut Folder").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_file_name("cuts")
                    .set_directory(self.cut_folder.clone().unwrap_or_default())
                    .pick_folder()
            {
                self.cut_folder = Some(path);

                self.cuts = self.get_cuts_in_folder();
            }

            // Display an X button to clear the cut folder if it exists
            if self.cut_folder.is_some() {
                // Add a refresh button (logo) to reload cuts from the folder
                if ui.button("🔄").clicked()
                    && let Some(folder) = &self.cut_folder
                {
                    if folder.exists() && folder.is_dir() {
                        self.cuts = self.get_cuts_in_folder();
                    } else {
                        log::error!("Cut folder is invalid: {}", folder.display());
                    }
                }

                if ui.button("❌").clicked() {
                    self.cut_folder = None;
                    self.cuts.clear(); // Clear cuts when folder is removed
                }
            }

            ui.separator();

            if ui.button("Remove All").clicked() {
                self.cuts.clear();
            }
        });

        ui.horizontal_wrapped(|ui| {
            // Display the path of the cut folder if it exists
            if let Some(ref path) = self.cut_folder {
                ui.label(format!("Cut Folder: {}", path.display()));
            }
        });

        if self.cut_folder.is_some() {
            ui.separator();
        }

        if self
            .selected_cut_index
            .is_some_and(|index| index >= self.cuts.len())
        {
            self.selected_cut_index = None;
        }

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
                if self.selected_cut_index.is_none() {
                    ui.label(
                        egui::RichText::new("Click Builder on a 1D cut to edit its clauses.")
                            .weak()
                            .small(),
                    );
                }
                TableBuilder::new(ui)
                    .id_salt("cuts_1d_table")
                    .column(Column::auto()) // Name
                    .column(Column::remainder()) // Expression
                    .column(Column::auto()) // Active
                    .column(Column::auto()) // Info
                    .column(Column::auto()) // Actions
                    .striped(true)
                    .vscroll(false)
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.label("Name");
                        });
                        header.col(|ui| {
                            ui.label("Expression");
                        });
                        header.col(|ui| {
                            ui.label("Active");
                        });
                        header.col(|ui| {
                            ui.label("Info");
                        });
                        header.col(|ui| {
                            ui.label("");
                        });
                    })
                    .body(|mut body| {
                        for (index, cut1d) in cuts_1d {
                            let row_height = Cut1D::table_row_height();
                            body.row(row_height, |mut row| {
                                let is_selected = self.selected_cut_index == Some(index);
                                let builder_clicked = cut1d.table_row(&mut row, index, is_selected);

                                row.col(|ui| {
                                    if builder_clicked {
                                        self.selected_cut_index =
                                            if is_selected { None } else { Some(index) };
                                    }

                                    if ui.small_button("X").clicked() {
                                        indices_to_remove_cut.push(index);
                                    }
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
                    .column(Column::auto()) // Active
                    .column(Column::auto()) // Info
                    .column(Column::remainder()) // Actions
                    .striped(true)
                    .vscroll(false)
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.label("Name");
                        });
                        header.col(|ui| {
                            ui.label("Active");
                        });
                        header.col(|ui| {
                            ui.label("Info");
                        });
                    })
                    .body(|mut body| {
                        for (index, cut2d) in cuts_2d {
                            body.row(28.0, |mut row| {
                                cut2d.table_row(&mut row, index, false);
                                row.col(|ui| {
                                    if ui.button("X").clicked() {
                                        indices_to_remove_cut.push(index);
                                    }
                                });
                            });
                        }
                    });
            }

            for &index in indices_to_remove_cut.iter().rev() {
                self.cuts.remove(index);
                match self.selected_cut_index {
                    Some(selected) if selected == index => self.selected_cut_index = None,
                    Some(selected) if selected > index => {
                        self.selected_cut_index = Some(selected - 1);
                    }
                    _ => {}
                }
            }
        }

        if let Some(selected_index) = self.selected_cut_index
            && let Some(Cut::Cut1D(cut1d)) = self.cuts.get_mut(selected_index)
        {
            ui.separator();
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "1D Cut Builder: {}",
                            if cut1d.name.trim().is_empty() {
                                "Unnamed Cut"
                            } else {
                                cut1d.name.trim()
                            }
                        ))
                        .strong(),
                    );
                    if ui.small_button("Close").clicked() {
                        self.selected_cut_index = None;
                    }
                });
                ui.label(
                    egui::RichText::new(
                        "Build clauses here. The expression preview updates automatically.",
                    )
                    .weak()
                    .small(),
                );
                ui.separator();
                cut1d.builder_editor_ui(ui, available_columns, selected_index);
            });
        }

        if let Some(active_cuts) = active_cuts {
            Self::active_cut_rows(ui, active_cuts, id_suffix);
        }
    }

    pub fn valid(&self, df: &DataFrame, row_idx: usize) -> bool {
        self.cuts.iter().all(|cut| cut.valid(df, row_idx))
    }

    pub fn create_combined_mask(
        &self,
        df: &DataFrame,
        cuts: &[&Cut],
    ) -> Result<BooleanChunked, PolarsError> {
        let masks: Vec<BooleanChunked> = cuts
            .iter()
            .map(|cut| match cut {
                Cut::Cut1D(cut1d) => cut1d.create_mask(df),
                Cut::Cut2D(cut2d) => cut2d.create_mask(df),
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Combine all masks with a logical AND
        let combined_mask = masks
            .into_iter()
            .reduce(|a, b| a.bitand(b))
            .unwrap_or_else(|| BooleanChunked::from_slice("".into(), &[]));

        Ok(combined_mask)
    }

    pub fn required_columns(&self) -> Vec<String> {
        self.cuts
            .iter()
            .flat_map(|cut| cut.required_columns())
            .collect()
    }

    pub fn generate_key(&self) -> String {
        let mut cut_names: Vec<String> =
            self.cuts.iter().map(|cut| cut.name().to_owned()).collect();
        cut_names.sort(); // Ensure consistent ordering
        cut_names.join(",") // Create a comma-separated key
    }

    pub fn filter_df_and_save(
        &self,
        df: &DataFrame,
        file_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // get only valid cuts
        let valid_cuts: Vec<&Cut> = self
            .cuts
            .iter()
            .filter(|cut| match cut {
                Cut::Cut1D(cut1d) => cut1d.active,
                Cut::Cut2D(cut2d) => cut2d.active,
            })
            .collect();
        let mask = self.create_combined_mask(df, &valid_cuts)?;
        let mut filtered_df = df.filter(&mask)?; // Make filtered_df mutable

        let file = std::fs::File::create(file_path)?;
        ParquetWriter::new(file).finish(&mut filtered_df)?; // Pass as mutable reference

        Ok(())
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Cut2D {
    pub polygon: EguiPolygon,
    pub x_column: String,
    pub y_column: String,
    pub active: bool,
    #[serde(skip)]
    pub saved_path: Option<PathBuf>,
}

impl PartialEq for Cut2D {
    fn eq(&self, other: &Self) -> bool {
        self.polygon.draw == other.polygon.draw
            && self.polygon.name_in_legend == other.polygon.name_in_legend
            && self.polygon.name == other.polygon.name
            && self.polygon.highlighted == other.polygon.highlighted
            && self.polygon.stroke == other.polygon.stroke
            && self.polygon.width == other.polygon.width
            && self.polygon.fill_color == other.polygon.fill_color
            && self.polygon.style == other.polygon.style
            && self.polygon.style_length == other.polygon.style_length
            && self.polygon.vertices == other.polygon.vertices
            && self.polygon.color_rgb == other.polygon.color_rgb
            && self.polygon.stroke_rgb == other.polygon.stroke_rgb
            && self.polygon.interactive_clicking == other.polygon.interactive_clicking
            && self.polygon.interactive_dragging == other.polygon.interactive_dragging
            && self.x_column == other.x_column
            && self.y_column == other.y_column
            && self.active == other.active
    }
}

impl Default for Cut2D {
    fn default() -> Self {
        Self {
            polygon: EguiPolygon::default(),
            x_column: String::new(),
            y_column: String::new(),
            active: true,
            saved_path: None,
        }
    }
}

impl Cut2D {
    fn normalize_after_load(&mut self) {
        self.polygon.interactive_clicking = false;
    }

    pub fn default_name(x_column: &str, y_column: &str) -> String {
        format!("{y_column} v {x_column} Cut")
    }

    pub fn sanitized_file_name(&self) -> String {
        let trimmed_name = self.polygon.name.trim();
        let fallback = if trimmed_name.is_empty() {
            Self::default_name(&self.x_column, &self.y_column)
        } else {
            "cut".to_owned()
        };

        sanitize_cut_file_name_component(&self.polygon.name, &fallback)
    }

    fn save_to_path(&self, file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let serialized = serde_json::to_string(self)?;
        let mut file = File::create(file_path)?;
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }

    pub(crate) fn autosave_to_saved_path(&self) {
        if let Some(path) = &self.saved_path
            && let Err(error) = self.save_to_path(path)
        {
            log::error!("Error autosaving 2D cut '{}': {error:?}", self.polygon.name);
        }
    }

    fn changed_for_autosave(&self, previous: &Self) -> bool {
        self.polygon.name != previous.polygon.name
            || self.x_column != previous.x_column
            || self.y_column != previous.y_column
            || self.polygon.vertices != previous.polygon.vertices
    }

    fn saved_path_display(&self) -> String {
        self.saved_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "Not saved".to_owned())
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let previous = self.clone();

        if ui.button("Load").clicked()
            && let Err(e) = self.load_cut_from_json()
        {
            log::error!("Error loading cut: {e:?}");
        }

        if ui.button("Save").clicked()
            && let Err(e) = self.save_cut_to_json()
        {
            log::error!("Error saving cut: {e:?}");
        }
        self.polygon.menu_button(ui);

        if self.changed_for_autosave(&previous) {
            self.autosave_to_saved_path();
        }
    }

    pub fn single_ui(&mut self, ui: &mut egui::Ui) {
        let previous_name = self.polygon.name.clone();
        ui.horizontal(|ui| {
            ui.label("2D Cut");
            if ui.button("Load").clicked()
                && let Err(e) = self.load_cut_from_json()
            {
                log::error!("Error loading cut: {e:?}");
            }

            ui.add(
                egui::TextEdit::singleline(&mut self.polygon.name)
                    .hint_text("Cut Name")
                    .clip_text(false),
            );
        });

        if self.polygon.name != previous_name {
            self.autosave_to_saved_path();
        }
    }

    pub fn table_row(&mut self, row: &mut egui_extras::TableRow<'_, '_>) {
        let previous_name = self.polygon.name.clone();
        row.col(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.polygon.name)
                    .hint_text("Cut Name")
                    .clip_text(false),
            );
        });
        row.col(|ui| {
            ui.add(egui::Checkbox::new(&mut self.active, ""));
        });
        row.col(|ui| {
            self.info_button(ui, None);
        });

        if self.polygon.name != previous_name {
            self.autosave_to_saved_path();
        }
    }

    fn info_ui(&self, ui: &mut egui::Ui, histogram_description: Option<String>) {
        if let Some(histogram_description) = histogram_description {
            ui.label(histogram_description);
            ui.separator();
        }
        ui.label(format!("X Column: {}", self.x_column));
        ui.label(format!("Y Column: {}", self.y_column));
        ui.label(format!("Vertices: {}", self.polygon.vertices.len()));
        ui.label("Saved Path:");
        ui.monospace(self.saved_path_display());
    }

    pub fn info_button(&self, ui: &mut egui::Ui, histogram_description: Option<String>) {
        ui.menu_button("?", |ui| {
            self.info_ui(ui, histogram_description);
        });
    }

    pub fn save_button(&mut self, ui: &mut egui::Ui) {
        if ui.button("Save").clicked()
            && let Err(e) = self.save_cut_to_json()
        {
            log::error!("Error saving cut: {e:?}");
        }
    }

    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
        let previous = self.clone();

        if ui.button("Load").clicked()
            && let Err(e) = self.load_cut_from_json()
        {
            log::error!("Error loading cut: {e:?}");
        }

        if ui.button("Save").clicked()
            && let Err(e) = self.save_cut_to_json()
        {
            log::error!("Error saving cut: {e:?}");
        }

        self.polygon.menu_button(ui);

        if self.changed_for_autosave(&previous) {
            self.autosave_to_saved_path();
        }
    }

    pub fn valid(&self, df: &DataFrame, row_idx: usize) -> bool {
        if !self.active {
            return false; // If the cut is not active, it is not valid
        }
        // Attempt to retrieve the x and y column values for the specified row
        if let (Ok(cut_x_values), Ok(cut_y_values)) = (
            df.column(&self.x_column).and_then(|c| c.f64()),
            df.column(&self.y_column).and_then(|c| c.f64()),
        ) {
            // Retrieve the x and y values for the given row index
            if let (Some(cut_x), Some(cut_y)) =
                (cut_x_values.get(row_idx), cut_y_values.get(row_idx))
            {
                // Check if the point (cut_x, cut_y) is inside the polygon
                return self.is_inside(cut_x, cut_y);
            }
        }
        // Return false if columns or row data were not found or if point is not inside polygon
        false
    }

    pub fn create_mask(&self, df: &DataFrame) -> Result<BooleanChunked, PolarsError> {
        let polygon = self.to_geo_polygon();
        let x_col = df.column(&self.x_column)?.f64()?;
        let y_col = df.column(&self.y_column)?.f64()?;

        // Create mask by checking if each point is inside the polygon
        let mask = x_col
            .into_no_null_iter()
            .zip(y_col.into_no_null_iter())
            .map(|(x, y)| polygon.contains(&geo::Point::new(x, y)))
            .collect::<BooleanChunked>();

        Ok(mask)
    }

    pub fn save_cut_to_json(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file_path) = rfd::FileDialog::new()
            .set_file_name(format!("{}.json", self.sanitized_file_name()))
            .add_filter("JSON Files", &["json"]) // Add a filter for json files
            .save_file()
        {
            self.save_to_path(&file_path)?;
            self.saved_path = Some(file_path);
        }
        Ok(())
    }

    pub fn load_cut_from_json(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file_path) = rfd::FileDialog::new()
            .set_file_name("cut.json") // Suggest a default file name for convenience
            .add_filter("JSON Files", &["json"]) // Filter for json files
            .pick_file()
        {
            let file = File::open(&file_path)?;
            let reader = BufReader::new(file);
            let mut cut: Self = serde_json::from_reader(reader)?;
            cut.saved_path = Some(file_path);
            cut.normalize_after_load();
            *self = cut;
        }
        Ok(())
    }

    fn to_geo_polygon(&self) -> geo::Polygon<f64> {
        let exterior_coords: Vec<(f64, f64)> = self
            .polygon
            .vertices
            .iter()
            .map(|&arr| arr.into())
            .collect();
        let exterior_line_string = geo::LineString::from(exterior_coords);
        geo::Polygon::new(exterior_line_string, vec![])
    }

    pub fn is_inside(&self, x: f64, y: f64) -> bool {
        let point = geo::Point::new(x, y);
        let polygon = self.to_geo_polygon();
        polygon.contains(&point)
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        self.polygon.draw(plot_ui);
    }

    pub fn interactions(&mut self, plot_response: &egui_plot::PlotResponse<()>) {
        let previous = self.clone();
        self.polygon.handle_interactions(plot_response);
        if self.changed_for_autosave(&previous) {
            self.autosave_to_saved_path();
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.polygon.is_dragging
    }

    pub fn is_clicking(&self) -> bool {
        self.polygon.interactive_clicking
    }

    pub fn required_columns(&self) -> Vec<String> {
        vec![self.x_column.clone(), self.y_column.clone()]
    }

    // pub fn filter_df_and_save(
    //     &self,
    //     df: &DataFrame,
    //     file_path: &str,
    // ) -> Result<(), Box<dyn std::error::Error>> {
    //     let mask = self.create_mask(df)?;
    //     let mut filtered_df = df.filter(&mask)?; // Make filtered_df mutable

    //     let file = std::fs::File::create(file_path)?;
    //     ParquetWriter::new(file).finish(&mut filtered_df)?; // Pass as mutable reference

    //     Ok(())
    // }
}

#[cfg(test)]
mod tests {
    use super::{Cut1D, Cut2D, sanitize_cut_file_name_component};

    #[test]
    fn loading_cut_disables_interactive_vertex_adding() {
        let mut cut = Cut2D::default();
        cut.polygon.interactive_clicking = true;

        cut.normalize_after_load();

        assert!(!cut.polygon.interactive_clicking);
    }

    #[test]
    fn cut_file_name_components_replace_collapse_and_trim_invalid_characters() {
        assert_eq!(
            sanitize_cut_file_name_component(" alpha beta!! gamma. ", "cut"),
            "alpha_beta_gamma"
        );
    }

    #[test]
    fn cut2d_sanitized_file_name_keeps_existing_fallback_behavior() {
        let mut cut = Cut2D::default();
        cut.polygon.name = "!!!".to_owned();

        assert_eq!(cut.sanitized_file_name(), "cut");
    }

    #[test]
    fn parse_conditions_accepts_legacy_parenthesized_literals() {
        let mut cut = Cut1D::new("Legacy", "(X1 >= (50.749586467606804)) & (X1 <= 100)");

        cut.parse_conditions();

        let parsed_groups = cut
            .parsed_groups
            .as_ref()
            .expect("legacy expression should parse");
        assert_eq!(parsed_groups.len(), 1);
        assert_eq!(parsed_groups[0].conditions.len(), 2);
        assert_eq!(parsed_groups[0].conditions[0].column_name, "X1");
        assert_eq!(parsed_groups[0].conditions[0].operator, ">=");
        assert!((parsed_groups[0].conditions[0].literal_value - 50.749586467606804).abs() < 1e-12);
        assert_eq!(parsed_groups[0].conditions[1].column_name, "X1");
        assert_eq!(parsed_groups[0].conditions[1].operator, "<=");
        assert!((parsed_groups[0].conditions[1].literal_value - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_conditions_accepts_builder_style_grouped_ranges() {
        let mut cut = Cut1D::new("Grouped", "((X1 >= 50) & (X1 <= 100))");

        cut.parse_conditions();

        let parsed_groups = cut
            .parsed_groups
            .as_ref()
            .expect("grouped expression should parse");
        assert_eq!(parsed_groups.len(), 1);
        assert_eq!(parsed_groups[0].conditions.len(), 2);
    }
}

// Struct to hold each parsed condition
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCondition {
    pub column_name: String,
    pub operator: String,
    pub literal_value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedConditionGroup {
    pub conditions: Vec<ParsedCondition>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
pub enum Cut1DEditMode {
    #[default]
    Builder,
    Advanced,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
pub enum Cut1DOperator {
    #[default]
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

impl Cut1DOperator {
    fn symbol(self) -> &'static str {
        match self {
            Self::GreaterThan => ">",
            Self::GreaterThanOrEqual => ">=",
            Self::LessThan => "<",
            Self::LessThanOrEqual => "<=",
            Self::Equal => "==",
            Self::NotEqual => "!=",
        }
    }

    fn from_symbol(symbol: &str) -> Option<Self> {
        match symbol {
            ">" => Some(Self::GreaterThan),
            ">=" => Some(Self::GreaterThanOrEqual),
            "<" => Some(Self::LessThan),
            "<=" => Some(Self::LessThanOrEqual),
            "==" => Some(Self::Equal),
            "!=" => Some(Self::NotEqual),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Cut1DBuilderCondition {
    pub column_name: String,
    pub operator: Cut1DOperator,
    pub literal_value: f64,
}

impl Default for Cut1DBuilderCondition {
    fn default() -> Self {
        Self {
            column_name: String::new(),
            operator: Cut1DOperator::GreaterThanOrEqual,
            literal_value: 0.0,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub struct Cut1DBuilderGroup {
    pub conditions: Vec<Cut1DBuilderCondition>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Cut1D {
    pub name: String,
    pub expression: String, // Logical expression to evaluate, e.g., "X1 != -1e6 & X2 == -1e6"
    pub active: bool,
    #[serde(default)]
    pub edit_mode: Cut1DEditMode,
    #[serde(default)]
    pub builder_groups: Vec<Cut1DBuilderGroup>,
    #[serde(skip)] // Skip during serialization
    pub parsed_groups: Option<Vec<ParsedConditionGroup>>, // Cache parsed conditions
    #[serde(skip)]
    pub saved_path: Option<PathBuf>,
}

impl PartialEq for Cut1D {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.expression == other.expression
            && self.active == other.active
    }
}

impl Cut1D {
    pub fn new(name: &str, expression: &str) -> Self {
        Self {
            name: name.to_owned(),
            expression: expression.to_owned(),
            active: true,
            edit_mode: Cut1DEditMode::Builder,
            builder_groups: vec![Cut1DBuilderGroup {
                conditions: vec![Cut1DBuilderCondition::default()],
            }],
            parsed_groups: None,
            saved_path: None,
        }
    }

    pub fn builder_default() -> Self {
        Self::new("", "")
    }

    fn normalize_after_load(&mut self) {
        self.edit_mode = Cut1DEditMode::Builder;
        self.parse_conditions();
        self.ensure_builder_groups();
    }

    fn format_value(value: f64) -> String {
        if value.is_nan() {
            return "nan".to_owned();
        }

        if value.is_infinite() {
            return if value.is_sign_positive() {
                "inf".to_owned()
            } else {
                "-inf".to_owned()
            };
        }

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

    fn range_editor_fields(&self) -> (String, String, String) {
        let Some(groups) = &self.parsed_groups else {
            return (String::new(), String::new(), String::new());
        };

        if groups.len() != 1 {
            return (String::new(), String::new(), String::new());
        }

        let Some(first_group) = groups.first() else {
            return (String::new(), String::new(), String::new());
        };

        let Some(first_condition) = first_group.conditions.first() else {
            return (String::new(), String::new(), String::new());
        };

        let column_name = first_condition.column_name.clone();
        if !first_group
            .conditions
            .iter()
            .all(|condition| condition.column_name == column_name)
        {
            return (String::new(), String::new(), String::new());
        }

        let mut lower_bound: Option<f64> = None;
        let mut upper_bound: Option<f64> = None;

        for condition in &first_group.conditions {
            match condition.operator.as_str() {
                ">" | ">=" => {
                    lower_bound = Some(lower_bound.map_or(condition.literal_value, |current| {
                        current.max(condition.literal_value)
                    }));
                }
                "<" | "<=" => {
                    upper_bound = Some(upper_bound.map_or(condition.literal_value, |current| {
                        current.min(condition.literal_value)
                    }));
                }
                _ => {}
            }
        }

        (
            column_name,
            lower_bound.map(Self::format_value).unwrap_or_default(),
            upper_bound.map(Self::format_value).unwrap_or_default(),
        )
    }

    fn table_row_height() -> f32 {
        44.0
    }

    fn ensure_builder_groups(&mut self) {
        if self
            .builder_groups
            .iter()
            .any(|group| !group.conditions.is_empty())
        {
            return;
        }

        if !self.expression.trim().is_empty() {
            if self.parsed_groups.is_none() {
                self.parse_conditions();
            }
            if self
                .builder_groups
                .iter()
                .any(|group| !group.conditions.is_empty())
            {
                return;
            }
        }

        self.builder_groups.push(Cut1DBuilderGroup {
            conditions: vec![Cut1DBuilderCondition::default()],
        });
    }

    fn sync_expression_from_builder(&mut self) {
        let parsed_groups = self
            .builder_groups
            .iter()
            .filter_map(|group| {
                let conditions = group
                    .conditions
                    .iter()
                    .filter(|condition| !condition.column_name.trim().is_empty())
                    .map(|condition| ParsedCondition {
                        column_name: condition.column_name.trim().to_owned(),
                        operator: condition.operator.symbol().to_owned(),
                        literal_value: condition.literal_value,
                    })
                    .collect::<Vec<_>>();

                if conditions.is_empty() {
                    None
                } else {
                    Some(ParsedConditionGroup { conditions })
                }
            })
            .collect::<Vec<_>>();

        let expression = parsed_groups
            .iter()
            .map(|group| {
                let group_expression = group
                    .conditions
                    .iter()
                    .map(|condition| {
                        format!(
                            "({} {} {})",
                            condition.column_name,
                            condition.operator,
                            Self::format_value(condition.literal_value)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" & ");
                format!("({group_expression})")
            })
            .collect::<Vec<_>>()
            .join(" | ");

        self.expression = expression;
        if parsed_groups.is_empty() {
            self.parsed_groups = None;
        } else {
            self.parsed_groups = Some(parsed_groups);
        }
    }

    fn builder_editor_ui(
        &mut self,
        ui: &mut egui::Ui,
        available_columns: &[String],
        row_index: usize,
    ) {
        self.ensure_builder_groups();

        let mut changed = false;
        let mut group_indices_to_remove = Vec::new();
        let total_group_count = self.builder_groups.len();
        let total_condition_count = self
            .builder_groups
            .iter()
            .map(|group| group.conditions.len())
            .sum::<usize>();

        ui.label(format!(
            "Expressions: {total_group_count}  Conditions: {total_condition_count}"
        ));
        ui.separator();

        for (group_index, group) in self.builder_groups.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Expression {}", group_index + 1)).strong(),
                    );
                    ui.label(
                        egui::RichText::new("(conditions inside here use AND)")
                            .weak()
                            .small(),
                    );
                    if total_group_count > 1 && ui.small_button("Remove Expression").clicked() {
                        group_indices_to_remove.push(group_index);
                    }
                });

                let mut condition_indices_to_remove = Vec::new();

                for (condition_index, condition) in group.conditions.iter_mut().enumerate() {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(format!("Condition {}", condition_index + 1));
                        if ui.small_button("Remove").clicked() {
                            condition_indices_to_remove.push(condition_index);
                        }
                    });

                    ui.horizontal_wrapped(|ui| {
                        ui.label("Column");
                        changed |= searchable_column_picker_with_width_ui(
                            ui,
                            format!(
                                "cut1d_builder_column_{row_index}_{group_index}_{condition_index}"
                            ),
                            &mut condition.column_name,
                            available_columns,
                            "Select column",
                            true,
                            SearchableColumnPickerSize {
                                closed_width: 220.0,
                                open_width: 420.0,
                            },
                        );

                        ui.label("Operator");
                        egui::ComboBox::from_id_salt(format!(
                            "cut1d_builder_operator_{row_index}_{group_index}_{condition_index}"
                        ))
                        .selected_text(condition.operator.symbol())
                        .show_ui(ui, |ui| {
                            for operator in [
                                Cut1DOperator::GreaterThan,
                                Cut1DOperator::GreaterThanOrEqual,
                                Cut1DOperator::LessThan,
                                Cut1DOperator::LessThanOrEqual,
                                Cut1DOperator::Equal,
                                Cut1DOperator::NotEqual,
                            ] {
                                changed |= ui
                                    .selectable_value(
                                        &mut condition.operator,
                                        operator,
                                        operator.symbol(),
                                    )
                                    .changed();
                            }
                        });

                        ui.label("Value");
                        changed |= ui
                            .add(precise_drag_value(&mut condition.literal_value).speed(0.1))
                            .changed();
                    });
                }

                for &index in condition_indices_to_remove.iter().rev() {
                    group.conditions.remove(index);
                    changed = true;
                }

                if group.conditions.is_empty() {
                    group.conditions.push(Cut1DBuilderCondition::default());
                }

                if ui.button("Add Condition").clicked() {
                    group.conditions.push(Cut1DBuilderCondition::default());
                    changed = true;
                    ui.ctx().request_repaint();
                }
            });

            if group_index + 1 < total_group_count {
                ui.label(egui::RichText::new("OR").strong());
            }
        }

        for &index in group_indices_to_remove.iter().rev() {
            self.builder_groups.remove(index);
            changed = true;
        }

        ui.horizontal(|ui| {
            if ui.button("Add OR Expression").clicked() {
                self.builder_groups.push(Cut1DBuilderGroup {
                    conditions: vec![Cut1DBuilderCondition::default()],
                });
                changed = true;
                ui.ctx().request_repaint();
            }

            ui.label("Expressions are combined with OR.");
        });

        if changed {
            self.sync_expression_from_builder();
        }
    }

    pub fn table_row(
        &mut self,
        row: &mut egui_extras::TableRow<'_, '_>,
        _row_index: usize,
        is_selected: bool,
    ) -> bool {
        let previous = self.clone();
        let mut builder_clicked = false;

        row.col(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.name)
                    .hint_text("Name")
                    .clip_text(false),
            );
        });
        row.col(|ui| {
            ui.vertical(|ui| {
                self.edit_mode = Cut1DEditMode::Builder;
                let condition_count = self
                    .builder_groups
                    .iter()
                    .map(|group| {
                        group
                            .conditions
                            .iter()
                            .filter(|condition| !condition.column_name.trim().is_empty())
                            .count()
                    })
                    .sum::<usize>();
                let expression_count = self
                    .builder_groups
                    .iter()
                    .filter(|group| {
                        group
                            .conditions
                            .iter()
                            .any(|condition| !condition.column_name.trim().is_empty())
                    })
                    .count();

                ui.horizontal(|ui| {
                    if ui
                        .small_button(if is_selected {
                            "Hide Builder"
                        } else {
                            "Builder"
                        })
                        .clicked()
                    {
                        builder_clicked = true;
                    }

                    ui.label(format!(
                        "{condition_count} condition(s) in {expression_count} expression(s)"
                    ));
                });
            });
        });
        row.col(|ui| {
            ui.add(egui::Checkbox::new(&mut self.active, ""));
        });
        row.col(|ui| {
            self.info_button(ui, None);
        });

        if self.name != previous.name || self.expression != previous.expression {
            self.autosave_to_saved_path();
        }

        builder_clicked
    }

    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
        let previous = self.clone();
        self.edit_mode = Cut1DEditMode::Builder;
        ui.add(
            egui::TextEdit::singleline(&mut self.name)
                .hint_text("Name")
                .clip_text(false),
        );
        ui.label("Use the 1D cut builder to edit conditions.");

        if self.name != previous.name || self.expression != previous.expression {
            self.autosave_to_saved_path();
        }
    }

    pub fn sanitized_file_name(&self) -> String {
        sanitize_cut_file_name_component(&self.name, "1d_cut")
    }

    fn save_to_path(&self, file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let serialized = serde_json::to_string(self)?;
        let mut file = File::create(file_path)?;
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }

    pub(crate) fn autosave_to_saved_path(&self) {
        if let Some(path) = &self.saved_path
            && let Err(error) = self.save_to_path(path)
        {
            log::error!("Error autosaving 1D cut '{}': {error:?}", self.name);
        }
    }

    fn saved_path_display(&self) -> String {
        self.saved_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "Not saved".to_owned())
    }

    pub fn save_cut_to_json(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file_path) = rfd::FileDialog::new()
            .set_file_name(format!("{}.json", self.sanitized_file_name()))
            .add_filter("JSON Files", &["json"])
            .save_file()
        {
            self.save_to_path(&file_path)?;
            self.saved_path = Some(file_path);
        }
        Ok(())
    }

    pub fn load_cut_from_json(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file_path) = rfd::FileDialog::new()
            .set_file_name("cut.json")
            .add_filter("JSON Files", &["json"])
            .pick_file()
        {
            let file = File::open(&file_path)?;
            let reader = BufReader::new(file);
            let mut cut: Self = serde_json::from_reader(reader)?;
            cut.normalize_after_load();
            cut.saved_path = Some(file_path);
            *self = cut;
        }
        Ok(())
    }

    fn info_ui(&self, ui: &mut egui::Ui, histogram_description: Option<String>) {
        if let Some(histogram_description) = histogram_description {
            ui.label(histogram_description);
            ui.separator();
        }

        let (column_name, lower_bound, upper_bound) = self.range_editor_fields();
        if !column_name.is_empty() {
            ui.label(format!("Column: {column_name}"));
            ui.label(format!(
                ">= {}",
                if lower_bound.is_empty() {
                    "N/A"
                } else {
                    &lower_bound
                }
            ));
            ui.label(format!(
                "<= {}",
                if upper_bound.is_empty() {
                    "N/A"
                } else {
                    &upper_bound
                }
            ));
            ui.separator();
        }

        ui.label("Expression:");
        ui.monospace(&self.expression);
        ui.separator();
        ui.label("Saved Path:");
        ui.monospace(self.saved_path_display());
    }

    pub fn info_button(&self, ui: &mut egui::Ui, histogram_description: Option<String>) {
        ui.menu_button("?", |ui| {
            self.info_ui(ui, histogram_description);
        });
    }

    pub fn save_button(&mut self, ui: &mut egui::Ui) {
        if ui.button("Save").clicked()
            && let Err(error) = self.save_cut_to_json()
        {
            log::error!("Error saving 1D cut: {error:?}");
        }
    }

    pub fn required_columns(&self) -> Vec<String> {
        self.parsed_groups.as_ref().map_or(vec![], |groups| {
            groups
                .iter()
                .flat_map(|group| group.conditions.iter())
                .map(|cond| cond.column_name.clone())
                .collect()
        })
    }

    fn trim_wrapping_parentheses(expression: &str) -> String {
        let mut trimmed = expression.trim().to_owned();

        loop {
            if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
                break;
            }

            let mut depth = 0usize;
            let mut wraps_entire_expression = true;

            for (index, character) in trimmed.char_indices() {
                match character {
                    '(' => depth += 1,
                    ')' => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 && index + character.len_utf8() < trimmed.len() {
                            wraps_entire_expression = false;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if wraps_entire_expression && depth == 0 {
                trimmed = trimmed[1..trimmed.len() - 1].trim().to_owned();
            } else {
                break;
            }
        }

        trimmed
    }

    fn split_top_level(expression: &str, separator: char) -> Vec<String> {
        let mut parts = Vec::new();
        let mut depth = 0usize;
        let mut start = 0usize;

        for (index, character) in expression.char_indices() {
            match character {
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                _ => {}
            }

            if character == separator && depth == 0 {
                parts.push(expression[start..index].trim().to_owned());
                start = index + character.len_utf8();
            }
        }

        parts.push(expression[start..].trim().to_owned());
        parts.into_iter().filter(|part| !part.is_empty()).collect()
    }

    pub fn parse_conditions(&mut self) {
        if self.expression.trim().is_empty() {
            log::error!("Empty expression for cut '{}'", self.name);
            self.parsed_groups = None;
            return;
        }

        let condition_re = Regex::new(
            r"(?P<column>\w+)\s*(?P<op>>=|<=|!=|==|>|<)\s*\(?(?P<value>[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?|[+-]?inf|nan)\)?"
        )
        .expect("Failed to create regex");

        let expression = Self::trim_wrapping_parentheses(&self.expression);
        let group_expressions = Self::split_top_level(&expression, '|');
        let mut parsed_groups = Vec::new();

        for group_expression in group_expressions {
            let group_expression = Self::trim_wrapping_parentheses(&group_expression);
            let condition_expressions = Self::split_top_level(&group_expression, '&');
            let mut group_conditions = Vec::new();

            for expression in condition_expressions {
                let expression = Self::trim_wrapping_parentheses(&expression);
                let expression = expression.trim();

                if let Some(caps) = condition_re.captures(expression) {
                    let column_name = caps["column"].to_string();
                    let operator = caps["op"].to_string();
                    let literal_value: f64 = match caps["value"].parse() {
                        Ok(v) => v,
                        Err(error) => {
                            log::error!(
                                "Invalid numeric literal in cut '{}': {} ({})",
                                self.name,
                                expression,
                                error
                            );
                            continue;
                        }
                    };

                    group_conditions.push(ParsedCondition {
                        column_name,
                        operator,
                        literal_value,
                    });
                } else {
                    log::error!(
                        "Failed to parse expression '{}' in cut '{}'",
                        expression,
                        self.name
                    );
                }
            }

            if !group_conditions.is_empty() {
                parsed_groups.push(ParsedConditionGroup {
                    conditions: group_conditions,
                });
            }
        }

        if parsed_groups.is_empty() {
            log::error!("No valid conditions parsed in cut '{}'", self.name);
            self.parsed_groups = None;
            return;
        }

        self.builder_groups = parsed_groups
            .iter()
            .map(|group| Cut1DBuilderGroup {
                conditions: group
                    .conditions
                    .iter()
                    .filter_map(|condition| {
                        Some(Cut1DBuilderCondition {
                            column_name: condition.column_name.clone(),
                            operator: Cut1DOperator::from_symbol(&condition.operator)?,
                            literal_value: condition.literal_value,
                        })
                    })
                    .collect(),
            })
            .collect();

        self.parsed_groups = Some(parsed_groups);
    }

    fn condition_valid_for_row(
        df: &DataFrame,
        row_idx: usize,
        condition: &ParsedCondition,
    ) -> bool {
        if let Ok(column) = df.column(&condition.column_name).and_then(|c| c.f64()) {
            if let Some(value) = column.get(row_idx) {
                match condition.operator.as_str() {
                    ">" => {
                        value.partial_cmp(&condition.literal_value)
                            == Some(std::cmp::Ordering::Greater)
                    }
                    "<" => {
                        value.partial_cmp(&condition.literal_value)
                            == Some(std::cmp::Ordering::Less)
                    }
                    ">=" => {
                        value.partial_cmp(&condition.literal_value)
                            != Some(std::cmp::Ordering::Less)
                    }
                    "<=" => {
                        value.partial_cmp(&condition.literal_value)
                            != Some(std::cmp::Ordering::Greater)
                    }
                    "==" => {
                        value.partial_cmp(&condition.literal_value)
                            == Some(std::cmp::Ordering::Equal)
                    }
                    "!=" => {
                        value.partial_cmp(&condition.literal_value)
                            != Some(std::cmp::Ordering::Equal)
                    }
                    _ => {
                        log::error!("Unknown operator: {}", condition.operator);
                        false
                    }
                }
            } else {
                false
            }
        } else {
            log::error!("Column not found: {}", condition.column_name);
            false
        }
    }

    fn mask_for_condition(
        df: &DataFrame,
        condition: &ParsedCondition,
    ) -> Result<BooleanChunked, PolarsError> {
        let column = df.column(&condition.column_name)?.f64()?;
        let mask = match condition.operator.as_str() {
            ">" => column.gt(condition.literal_value),
            "<" => column.lt(condition.literal_value),
            ">=" => column.gt_eq(condition.literal_value),
            "<=" => column.lt_eq(condition.literal_value),
            "==" => column.equal(condition.literal_value),
            "!=" => column.not_equal(condition.literal_value),
            _ => {
                return Err(PolarsError::ComputeError(
                    format!("Unknown operator: {}", condition.operator).into(),
                ));
            }
        };
        Ok(mask)
    }

    // Validate a row using cached conditions
    pub fn valid(&self, df: &DataFrame, row_idx: usize) -> bool {
        if !self.active {
            return false; // If the cut is not active, it is not valid
        }
        if let Some(groups) = &self.parsed_groups {
            groups.iter().any(|group| {
                group
                    .conditions
                    .iter()
                    .all(|condition| Self::condition_valid_for_row(df, row_idx, condition))
            })
        } else {
            log::error!("No parsed conditions for Cut1D '{}'", self.name);
            false
        }
    }

    pub fn create_mask(&self, df: &DataFrame) -> Result<BooleanChunked, PolarsError> {
        if let Some(groups) = &self.parsed_groups {
            let mut group_masks = Vec::new();

            for group in groups {
                let mut condition_masks = Vec::new();
                for condition in &group.conditions {
                    condition_masks.push(Self::mask_for_condition(df, condition)?);
                }

                let group_mask = condition_masks
                    .into_iter()
                    .reduce(|a, b| a.bitand(b))
                    .unwrap_or_else(|| BooleanChunked::from_slice("".into(), &[]));
                group_masks.push(group_mask);
            }

            let combined_mask = group_masks
                .into_iter()
                .reduce(|a, b| a.bitor(b))
                .unwrap_or_else(|| BooleanChunked::from_slice("".into(), &[]));
            Ok(combined_mask)
        } else {
            Err(PolarsError::ComputeError(
                "Conditions not parsed for Cut1D".into(),
            ))
        }
    }
}
