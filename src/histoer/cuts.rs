use geo::Contains;
use regex::Regex;
use std::fs::File;
use std::io::{BufReader, Write};
use std::ops::BitAnd;

use polars::prelude::*;

use crate::egui_plot_stuff::egui_polygon::EguiPolygon;
use egui_extras::{Column, TableBuilder};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum Cut {
    Cut1D(Cut1D),
    Cut2D(Cut2D),
}

impl Default for Cut {
    fn default() -> Self {
        Cut::Cut2D(Cut2D::default())
    }
}

impl Cut {
    // Method to check if a cut is valid for a specific row in the DataFrame
    pub fn valid(&self, df: &DataFrame, row_idx: usize) -> bool {
        match self {
            Cut::Cut1D(cut1d) => cut1d.valid(df, row_idx),
            Cut::Cut2D(cut2d) => cut2d.valid(df, row_idx),
        }
    }

    pub fn table_row(&mut self, row: &mut egui_extras::TableRow<'_, '_>) {
        match self {
            Cut::Cut1D(cut1d) => cut1d.table_row(row),
            Cut::Cut2D(cut2d) => cut2d.table_row(row),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Cut::Cut1D(cut1d) => &cut1d.name,
            Cut::Cut2D(cut2d) => &cut2d.polygon.name,
        }
    }

    /// Returns the column(s) required by the cut
    pub fn required_columns(&self) -> Vec<String> {
        match self {
            Cut::Cut1D(cut1d) => cut1d.required_columns(),
            Cut::Cut2D(cut2d) => cut2d.required_columns(),
        }
    }

    pub fn new_1d(name: &str, expression: &str) -> Self {
        Cut::Cut1D(Cut1D::new(name, expression))
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default)]
pub struct Cuts {
    pub cuts: Vec<Cut>,
}

impl Cuts {
    pub fn new(cuts: Vec<Cut>) -> Self {
        Self { cuts }
    }

    pub fn get_active_cuts(&self) -> Cuts {
        let active_cuts = self
            .cuts
            .iter()
            .filter(|cut| match cut {
                Cut::Cut1D(cut1d) => cut1d.active,
                Cut::Cut2D(cut2d) => cut2d.active,
            })
            .cloned()
            .collect();
        Cuts::new(active_cuts)
    }

    pub fn is_empty(&self) -> bool {
        self.cuts.is_empty()
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
            log::error!("No cut with name '{}' found.", name);
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

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Cuts");

            if ui.button("+1D").clicked() {
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
                    .column(Column::auto()) // Active
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
                                    if ui.button("X").clicked() {
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
                    .column(Column::auto()) // X Column
                    .column(Column::auto()) // Y Column
                    .column(Column::auto()) // Active
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
            }
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
            self.cuts.iter().map(|cut| cut.name().to_string()).collect();
        cut_names.sort(); // Ensure consistent ordering
        cut_names.join(",") // Create a comma-separated key
    }

    pub fn filter_df_and_save(
        &self,
        df: &DataFrame,
        file_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mask = self.create_combined_mask(df, &self.cuts.iter().collect::<Vec<_>>())?;
        let mut filtered_df = df.filter(&mask)?; // Make filtered_df mutable

        let file = std::fs::File::create(file_path)?;
        ParquetWriter::new(file).finish(&mut filtered_df)?; // Pass as mutable reference

        Ok(())
    }

    pub fn filter_lazyframe_in_batches(
        &self,
        lf: &LazyFrame,
        batch_size: usize,
    ) -> Result<LazyFrame, PolarsError> {
        let mut offset = 0;
        let mut filtered_batches = vec![];

        loop {
            let batch = lf.clone().slice(offset as i64, batch_size as u32);
            let df = batch.collect()?;
            if df.height() == 0 {
                break;
            }

            // get valid cuts
            let valid_cuts: Vec<&Cut> = self
                .cuts
                .iter()
                .filter(|cut| match cut {
                    Cut::Cut1D(cut1d) => cut1d.active,
                    Cut::Cut2D(cut2d) => cut2d.active,
                })
                .collect();

            let mask = self.create_combined_mask(&df, &valid_cuts)?;
            let filtered = df.filter(&mask)?;

            filtered_batches.push(filtered);
            offset += batch_size;
        }

        let lazy_batches: Vec<LazyFrame> =
            filtered_batches.into_iter().map(DataFrame::lazy).collect();
        let concatenated = concat(lazy_batches, UnionArgs::default())?;
        Ok(concatenated)
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Cut2D {
    pub polygon: EguiPolygon,
    pub x_column: String,
    pub y_column: String,
    pub active: bool,
}

impl Default for Cut2D {
    fn default() -> Self {
        Cut2D {
            polygon: EguiPolygon::default(),
            x_column: "".to_string(),
            y_column: "".to_string(),
            active: true,
        }
    }
}

impl Cut2D {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if ui.button("Load").clicked() {
            if let Err(e) = self.load_cut_from_json() {
                log::error!("Error loading cut: {:?}", e);
            }
        }

        if ui.button("Save").clicked() {
            if let Err(e) = self.save_cut_to_json() {
                log::error!("Error saving cut: {:?}", e);
            }
        }
        self.polygon.menu_button(ui);
    }

    pub fn single_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("2D Cut");
            if ui.button("Load").clicked() {
                if let Err(e) = self.load_cut_from_json() {
                    log::error!("Error loading cut: {:?}", e);
                }
            }

            ui.add(
                egui::TextEdit::singleline(&mut self.polygon.name)
                    .hint_text("Cut Name")
                    .clip_text(false),
            );
        });
    }

    pub fn table_row(&mut self, row: &mut egui_extras::TableRow<'_, '_>) {
        row.col(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.polygon.name)
                    .hint_text("Cut Name")
                    .clip_text(false),
            );
        });
        row.col(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.x_column)
                    .hint_text("X Column")
                    .clip_text(false),
            );
        });
        row.col(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.y_column)
                    .hint_text("Y Column")
                    .clip_text(false),
            );
        });
        row.col(|ui| {
            ui.add(egui::Checkbox::new(&mut self.active, ""));
        });
    }

    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
        if ui.button("Load").clicked() {
            if let Err(e) = self.load_cut_from_json() {
                log::error!("Error loading cut: {:?}", e);
            }
        }

        if ui.button("Save").clicked() {
            if let Err(e) = self.save_cut_to_json() {
                log::error!("Error saving cut: {:?}", e);
            }
        }

        self.polygon.menu_button(ui);
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

    pub fn save_cut_to_json(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file_path) = rfd::FileDialog::new()
            .add_filter("JSON Files", &["json"]) // Add a filter for json files
            .save_file()
        {
            let serialized = serde_json::to_string(self)?;
            let mut file = File::create(file_path)?;
            file.write_all(serialized.as_bytes())?;
        }
        Ok(())
    }

    pub fn load_cut_from_json(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file_path) = rfd::FileDialog::new()
            .set_file_name("cut.json") // Suggest a default file name for convenience
            .add_filter("JSON Files", &["json"]) // Filter for json files
            .pick_file()
        {
            let file = File::open(file_path)?;
            let reader = BufReader::new(file);
            let cut: Cut2D = serde_json::from_reader(reader)?;
            *self = cut;
        }
        Ok(())
    }

    fn to_geo_polygon(&self) -> geo::Polygon<f64> {
        let exterior_coords: Vec<_> = self.polygon.vertices.iter().map(|&[x, y]| (x, y)).collect();
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
        self.polygon.handle_interactions(plot_response);
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

    pub fn filter_df_and_save(
        &self,
        df: &DataFrame,
        file_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mask = self.create_mask(df)?;
        let mut filtered_df = df.filter(&mask)?; // Make filtered_df mutable

        let file = std::fs::File::create(file_path)?;
        ParquetWriter::new(file).finish(&mut filtered_df)?; // Pass as mutable reference

        Ok(())
    }
}

// Struct to hold each parsed condition
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCondition {
    pub column_name: String,
    pub operator: String,
    pub literal_value: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Cut1D {
    pub name: String,
    pub expression: String, // Logical expression to evaluate, e.g., "X1 != -1e6 & X2 == -1e6"
    pub active: bool,
    #[serde(skip)] // Skip during serialization
    pub parsed_conditions: Option<Vec<ParsedCondition>>, // Cache parsed conditions
}

impl Cut1D {
    pub fn new(name: &str, expression: &str) -> Self {
        Self {
            name: name.to_string(),
            expression: expression.to_string(),
            active: true,
            parsed_conditions: None,
        }
    }

    pub fn table_row(&mut self, row: &mut egui_extras::TableRow<'_, '_>) {
        row.col(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.name)
                    .hint_text("Name")
                    .clip_text(false),
            );
        });
        row.col(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.expression)
                    .hint_text("Expression")
                    .clip_text(false),
            );
        });
        row.col(|ui| {
            ui.add(egui::Checkbox::new(&mut self.active, ""));
        });
    }

    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::TextEdit::singleline(&mut self.name)
                .hint_text("Name")
                .clip_text(false),
        );

        ui.add(
            egui::TextEdit::singleline(&mut self.expression)
                .hint_text("Expression")
                .clip_text(false),
        );
    }

    pub fn required_columns(&self) -> Vec<String> {
        self.parsed_conditions
            .as_ref()
            .map_or(vec![], |conditions| {
                conditions
                    .iter()
                    .map(|cond| cond.column_name.clone())
                    .collect()
            })
    }

    // // Parse and cache conditions
    // pub fn parse_conditions(&mut self) {
    //     self.parsed_conditions = None; // Reset parsed conditions

    //     let condition_re = Regex::new(
    //         r"(?P<column>\w+)\s*(?P<op>>=|<=|!=|==|>|<)\s*(?P<value>-?\d+(\.\d+)?(e-?\d+)?|nan|inf)"
    //     ).unwrap();

    //     let mut conditions = Vec::new();
    //     for caps in condition_re.captures_iter(&self.expression) {
    //         let column_name = caps["column"].to_string();
    //         let operator = caps["op"].to_string();
    //         let literal_value: f64 = caps["value"].parse().unwrap();

    //         conditions.push(ParsedCondition {
    //             column_name,
    //             operator,
    //             literal_value,
    //         });
    //     }
    //     self.parsed_conditions = Some(conditions);

    //     log::info!("Parsed conditions: {:?}", self.parsed_conditions);
    // }

    pub fn parse_conditions(&mut self) {
        // self.parsed_conditions = None; // Reset
        if self.expression.trim().is_empty() {
            log::error!("Empty expression for cut '{}'", self.name);
            self.parsed_conditions = None;
            return;
        }

        let condition_re = Regex::new(
            r"(?P<column>\w+)\s*(?P<op>>=|<=|!=|==|>|<)\s*(?P<value>-?\d+(?:\.\d+)?(?:e-?\d+)?|nan|inf)"
        ).unwrap();

        let mut conditions = Vec::new();

        // Split on '&' to allow multiple conditions in one expression
        for expr in self.expression.split('&') {
            let expr = expr.trim();
            if let Some(caps) = condition_re.captures(expr) {
                let column_name = caps["column"].to_string();
                let operator = caps["op"].to_string();
                let literal_value: f64 = match caps["value"].parse() {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!(
                            "Invalid numeric literal in cut '{}': {} ({})",
                            self.name,
                            expr,
                            e
                        );
                        continue;
                    }
                };

                conditions.push(ParsedCondition {
                    column_name,
                    operator,
                    literal_value,
                });
            } else {
                log::error!(
                    "Failed to parse expression '{}' in cut '{}'",
                    expr,
                    self.name
                );
            }
        }

        if conditions.is_empty() {
            log::error!("No valid conditions parsed in cut '{}'", self.name);
        }

        self.parsed_conditions = Some(conditions);
    }

    // Validate a row using cached conditions
    pub fn valid(&self, df: &DataFrame, row_idx: usize) -> bool {
        if !self.active {
            return false; // If the cut is not active, it is not valid
        }
        if let Some(conditions) = &self.parsed_conditions {
            // Iterate through all parsed conditions
            for condition in conditions {
                if let Ok(column) = df.column(&condition.column_name).and_then(|c| c.f64()) {
                    if let Some(value) = column.get(row_idx) {
                        match condition.operator.as_str() {
                            ">" => {
                                if value.partial_cmp(&condition.literal_value)
                                    != Some(std::cmp::Ordering::Greater)
                                {
                                    return false;
                                }
                            }
                            "<" => {
                                if value.partial_cmp(&condition.literal_value)
                                    != Some(std::cmp::Ordering::Less)
                                {
                                    return false;
                                }
                            }
                            ">=" => {
                                if value.partial_cmp(&condition.literal_value)
                                    == Some(std::cmp::Ordering::Less)
                                {
                                    return false;
                                }
                            }
                            "<=" => {
                                if value.partial_cmp(&condition.literal_value)
                                    == Some(std::cmp::Ordering::Greater)
                                {
                                    return false;
                                }
                            }
                            "==" => {
                                if value.partial_cmp(&condition.literal_value)
                                    != Some(std::cmp::Ordering::Equal)
                                {
                                    return false;
                                }
                            }
                            "!=" => {
                                if value.partial_cmp(&condition.literal_value)
                                    == Some(std::cmp::Ordering::Equal)
                                {
                                    return false;
                                }
                            }
                            _ => {
                                log::error!("Unknown operator: {}", condition.operator);
                                return false;
                            }
                        }
                    } else {
                        return false; // Missing value for row
                    }
                } else {
                    log::error!("Column not found: {}", condition.column_name);
                    return false; // Missing column
                }
            }
            true // All conditions passed
        } else {
            log::error!("No parsed conditions for Cut1D '{}'", self.name);
            false // Parsing failed or was not performed
        }
    }

    pub fn create_mask(&self, df: &DataFrame) -> Result<BooleanChunked, PolarsError> {
        if let Some(conditions) = &self.parsed_conditions {
            let mut masks = Vec::new();
            for condition in conditions {
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
                        ))
                    }
                };
                masks.push(mask);
            }

            // Combine all masks with a logical AND
            let combined_mask = masks
                .into_iter()
                .reduce(|a, b| a.bitand(b))
                .unwrap_or_else(|| BooleanChunked::from_slice("".into(), &[]));
            Ok(combined_mask)
        } else {
            Err(PolarsError::ComputeError(
                "Conditions not parsed for Cut1D".into(),
            ))
        }
    }
}
