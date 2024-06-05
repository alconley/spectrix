use egui_plot::Polygon as EguiPolygon;
use egui_plot::{PlotPoints, PlotUi, Points};

use eframe::egui::{Color32, Stroke};

use std::fs::File;
use std::io::{BufReader, Write};

use serde::{Deserialize, Serialize};
use serde_json;

use rfd::FileDialog;

use geo::{algorithm::contains::Contains, LineString, Point, Polygon};

use polars::prelude::*;

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct EditableEguiPolygon {
    pub vertices: Vec<[f64; 2]>,          // List of vertex coordinates
    selected_vertex_index: Option<usize>, // Index of the selected vertex (if any)
    pub selected_x_column: Option<String>,
    pub selected_y_column: Option<String>,
    pub column_names: Vec<String>,
}

impl EditableEguiPolygon {
    /// Creates a new `EditablePolygon` with default vertices.
    /// Current Cut Binds:
    ///     Right click to add verticies
    ///     Left click to remove verticies
    ///     Middle click to remove all verticies
    pub fn new(column_names: Vec<String>) -> Self {
        Self {
            vertices: Vec::new(),        // Initialize with an empty set of vertices
            selected_vertex_index: None, // Initially, no vertex is selected
            selected_x_column: None,
            selected_y_column: None,
            column_names, // Initialize with the provided column names
        }
    }

    pub fn draw(&mut self, plot_ui: &mut PlotUi) {
        self.handle_mouse_interactions(plot_ui); // Handle mouse interactions
        self.draw_vertices_and_polygon(plot_ui); // Draw vertices and polygon
    }

    fn handle_mouse_interactions(&mut self, plot_ui: &mut PlotUi) {
        let response = plot_ui.response();

        if response.clicked() {
            let pointer_pos = plot_ui.pointer_coordinate().unwrap();
            self.add_new_vertex([pointer_pos.x, pointer_pos.y]); // Add a new vertex on left-click
        }

        if response.secondary_clicked() {
            let pointer_pos = plot_ui.pointer_coordinate().unwrap();
            self.selected_vertex_index =
                self.get_closest_vertex_index([pointer_pos.x, pointer_pos.y]); // Select and remove on right-click
            self.remove_vertex();
        }

        if response.middle_clicked() {
            self.remove_all_vertices(); // Remove all vertices on middle-click
        }
    }

    fn add_new_vertex(&mut self, coordinates: [f64; 2]) {
        self.vertices.push(coordinates); // Add a new vertex to the list
    }

    fn remove_vertex(&mut self) {
        if let Some(index) = self.selected_vertex_index {
            self.vertices.remove(index); // Remove the selected vertex
            self.clear_selection(); // Clear the selection
        }
    }

    fn get_closest_vertex_index(&self, pointer_pos: [f64; 2]) -> Option<usize> {
        let mut closest_vertex_index: Option<usize> = None;
        let mut closest_distance: f64 = 0.0;

        for (index, vertex) in self.vertices.iter().enumerate() {
            let distance =
                (vertex[0] - pointer_pos[0]).powi(2) + (vertex[1] - pointer_pos[1]).powi(2);
            if closest_vertex_index.is_none() || distance < closest_distance {
                closest_vertex_index = Some(index);
                closest_distance = distance;
            }
        }

        closest_vertex_index
    }

    fn clear_selection(&mut self) {
        self.selected_vertex_index = None; // Clear the selected vertex
    }

    fn remove_all_vertices(&mut self) {
        self.vertices.clear(); // Remove all vertices
        self.clear_selection(); // Clear the selection
    }

    fn draw_vertices_and_polygon(&mut self, plot_ui: &mut PlotUi) {
        if !self.vertices.is_empty() {
            let color = Color32::RED;
            let plot_points = PlotPoints::new(self.vertices.clone());
            let polygon_points = EguiPolygon::new(plot_points)
                .fill_color(Color32::TRANSPARENT)
                .stroke(Stroke::new(1.0, color));
            plot_ui.polygon(polygon_points); // Draw the polygon

            let vertices = Points::new(self.vertices.clone()).radius(5.0).color(color);
            plot_ui.points(vertices); // Draw the vertices
        }
    }

    pub fn save_cut_to_json(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create a default file name based on the selected columns
        let default_name = match (&self.selected_x_column, &self.selected_y_column) {
            (Some(x), Some(y)) => format!("{}_{}_cut.json", y, x),
            _ => "cut.json".to_string(),
        };

        if let Some(file_path) = FileDialog::new()
            .set_file_name(default_name)
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
        if let Some(file_path) = FileDialog::new()
            .set_file_name("cut.json") // Suggest a default file name for convenience
            .add_filter("JSON Files", &["json"]) // Filter for json files
            .pick_file()
        {
            let file = File::open(file_path)?;
            let reader = BufReader::new(file);
            let loaded_polygon: EditableEguiPolygon = serde_json::from_reader(reader)?;
            *self = loaded_polygon;
        }
        Ok(())
    }

    fn to_geo_polygon(&self) -> Polygon<f64> {
        let exterior_coords: Vec<_> = self.vertices.iter().map(|&[x, y]| (x, y)).collect();
        let exterior_line_string = LineString::from(exterior_coords);
        Polygon::new(exterior_line_string, vec![])
    }

    pub fn is_inside(&self, x: f64, y: f64) -> bool {
        let point = Point::new(x, y);
        let polygon = self.to_geo_polygon();
        polygon.contains(&point)
    }

    pub fn cut_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.separator();

            // Y Column ComboBox
            egui::ComboBox::from_label("Y Column")
                .selected_text(self.selected_y_column.as_deref().unwrap_or(""))
                .show_ui(ui, |ui| {
                    // Use self.column_names instead of CUT_COLUMN_NAMES
                    for column in &self.column_names {
                        if ui
                            .selectable_label(
                                self.selected_y_column.as_deref() == Some(column),
                                column,
                            )
                            .clicked()
                        {
                            self.selected_y_column = Some(column.to_string());
                        }
                    }
                });

            ui.separator();

            // X Column ComboBox
            egui::ComboBox::from_label("X Column")
                .selected_text(self.selected_x_column.as_deref().unwrap_or(""))
                .show_ui(ui, |ui| {
                    // Use self.column_names instead of CUT_COLUMN_NAMES
                    for column in &self.column_names {
                        if ui
                            .selectable_label(
                                self.selected_x_column.as_deref() == Some(column),
                                column,
                            )
                            .clicked()
                        {
                            self.selected_x_column = Some(column.to_string());
                        }
                    }
                });

            ui.separator();

            // Load Cut button
            if ui.button("Load Cut").clicked() {
                if let Err(e) = self.load_cut_from_json() {
                    log::error!("Error loading cut: {:?}", e);
                }
            }

            // Save Cut button
            let can_save: bool =
                self.selected_x_column.is_some() && self.selected_y_column.is_some();
            if ui
                .add_enabled(can_save, egui::Button::new("Save Cut"))
                .clicked()
            {
                if let Err(e) = self.save_cut_to_json() {
                    log::error!("Error saving cut: {:?}", e);
                }
            }

            ui.separator();
        });
    }

    pub fn filter_lf_with_cut(&self, lf: &LazyFrame) -> Result<LazyFrame, PolarsError> {
        // lots of clones... maybe there is a better way to do this

        let current_lf = lf.clone();

        if let (Some(x_col_name), Some(y_col_name)) =
            (&self.selected_x_column, &self.selected_y_column)
        {
            // get the min and max values for the x and y data points in the cuts
            let x_min = self
                .vertices
                .iter()
                .map(|&[x, _]| x)
                .fold(f64::INFINITY, |a, b| a.min(b));
            let x_max = self
                .vertices
                .iter()
                .map(|&[x, _]| x)
                .fold(f64::NEG_INFINITY, |a, b| a.max(b));
            let y_min = self
                .vertices
                .iter()
                .map(|&[_, y]| y)
                .fold(f64::INFINITY, |a, b| a.min(b));
            let y_max = self
                .vertices
                .iter()
                .map(|&[_, y]| y)
                .fold(f64::NEG_INFINITY, |a, b| a.max(b));

            let current_lf = current_lf
                .filter(col(x_col_name).gt_eq(lit(x_min)))
                .filter(col(x_col_name).lt_eq(lit(x_max)))
                .filter(col(y_col_name).gt_eq(lit(y_min)))
                .filter(col(y_col_name).lt_eq(lit(y_max)))
                .filter(col(x_col_name).neq(lit(-1e6)))
                .filter(col(y_col_name).neq(lit(-1e6)));

            let mask_creation_df = current_lf
                .clone()
                .select([col(x_col_name), col(y_col_name)])
                .collect()?;

            let ndarray_mask_creation_df =
                mask_creation_df.to_ndarray::<Float64Type>(IndexOrder::Fortran)?;
            let rows = ndarray_mask_creation_df.shape()[0];
            let mut boolean_chunked_builder = BooleanChunkedBuilder::new("mask", rows);

            for i in 0..rows {
                let x_value = ndarray_mask_creation_df[[i, 0]];
                let y_value = ndarray_mask_creation_df[[i, 1]];
                let point = self.is_inside(x_value, y_value);
                boolean_chunked_builder.append_value(point);
            }

            let boolean_chunked_series = boolean_chunked_builder.finish();
            let filtered_df = current_lf.clone().collect()?;

            let filtered_lf = filtered_df.filter(&boolean_chunked_series)?.lazy();

            return Ok(filtered_lf);
        }

        Ok(current_lf)
    }
}
