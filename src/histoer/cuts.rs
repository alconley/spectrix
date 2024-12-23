use geo::Contains;
use regex::Regex;
use std::fs::File;
use std::io::{BufReader, Write};

use polars::prelude::*;

use crate::egui_plot_stuff::egui_polygon::EguiPolygon;

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

    pub fn new_1d(name: &str, expression: &str) -> Self {
        Cut::Cut1D(Cut1D::new(name, expression))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Cut2D {
    pub polygon: EguiPolygon,
    pub x_column: String,
    pub y_column: String,
}

impl Default for Cut2D {
    fn default() -> Self {
        Cut2D {
            polygon: EguiPolygon::default(),
            x_column: "".to_string(),
            y_column: "".to_string(),
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

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi) {
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
    #[serde(skip)] // Skip during serialization
    pub parsed_conditions: Option<Vec<ParsedCondition>>, // Cache parsed conditions
}

impl Cut1D {
    pub fn new(name: &str, expression: &str) -> Self {
        Self {
            name: name.to_string(),
            expression: expression.to_string(),
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

    // Parse and cache conditions
    pub fn parse_conditions(&mut self) {
        self.parsed_conditions = None; // Reset parsed conditions

        let condition_re = Regex::new(
            r"(?P<column>\w+)\s*(?P<op>>=|<=|!=|==|>|<)\s*(?P<value>-?\d+(\.\d+)?(e-?\d+)?|nan|inf)"
        ).unwrap();

        let mut conditions = Vec::new();
        for caps in condition_re.captures_iter(&self.expression) {
            let column_name = caps["column"].to_string();
            let operator = caps["op"].to_string();
            let literal_value: f64 = caps["value"].parse().unwrap();

            conditions.push(ParsedCondition {
                column_name,
                operator,
                literal_value,
            });
        }
        self.parsed_conditions = Some(conditions);

        log::info!("Parsed conditions: {:?}", self.parsed_conditions);
    }

    // Validate a row using cached conditions
    pub fn valid(&self, df: &DataFrame, row_idx: usize) -> bool {
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
            false // Parsing failed or was not performed
        }
    }
}
