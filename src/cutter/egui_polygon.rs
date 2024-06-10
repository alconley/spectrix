use std::fs::File;
use std::io::{BufReader, Write};

use serde::{Deserialize, Serialize};
use serde_json;

use rfd::FileDialog;

use geo::Contains;

use polars::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EguiPolygon {
    pub vertices: Vec<[f64; 2]>,          // List of vertex coordinates
    selected_vertex_index: Option<usize>, // Index of the selected vertex (if any)
    pub draw: bool,
    #[serde(skip)]
    pub cursor_position: Option<egui_plot::PlotPoint>,
}

impl EguiPolygon {
    /// Creates a new `EditablePolygon` with default vertices.
    /// Current Cut Binds:
    ///     Right click to add verticies
    ///     Left click to remove verticies
    ///     Middle click to remove all verticies
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            selected_vertex_index: None,
            draw: true,
            cursor_position: None,
        }
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi) {
        self.handle_mouse_interactions(plot_ui); // Handle mouse interactions

        if self.draw {
            self.draw_vertices_and_polygon(plot_ui); // Draw vertices and polygon
        }
    }

    fn handle_mouse_interactions(&mut self, plot_ui: &mut egui_plot::PlotUi) {
        let response = plot_ui.response();

        if response.clicked() {
            let pointer_pos = plot_ui.pointer_coordinate().unwrap();
            self.add_new_vertex([pointer_pos.x, pointer_pos.y]); // Add a new vertex on left-click
        }
    }

    // Handles the interactive elements of the histogram
    pub fn keybinds(&mut self, ui: &mut egui::Ui) {
        if let Some(cursor_position) = self.cursor_position {
            if ui.input(|i| i.key_pressed(egui::Key::C)) {
                self.draw = !self.draw;
            }

            if ui.input(|i| i.key_pressed(egui::Key::Delete))
                || ui.input(|i| i.key_pressed(egui::Key::Backspace))
            {
                self.remove_all_vertices();
            }
        }
    }

    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Polygon", |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.draw, "Draw");
                if self.draw {
                    ui.separator();
                    ui.label("Polygon Settings:");
                    // self.egui_polygon.settings_ui(ui);
                }
            });
        });
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

    fn draw_vertices_and_polygon(&mut self, plot_ui: &mut egui_plot::PlotUi) {
        if !self.vertices.is_empty() {
            let color = egui::Color32::RED;
            let plot_points = egui_plot::PlotPoints::new(self.vertices.clone());
            let polygon_points = egui_plot::Polygon::new(plot_points)
                .fill_color(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::new(1.0, color));
            plot_ui.polygon(polygon_points); // Draw the polygon

            let vertices = egui_plot::Points::new(self.vertices.clone())
                .radius(5.0)
                .color(color);
            plot_ui.points(vertices); // Draw the vertices
        }
    }

    pub fn save_cut_to_json(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file_path) = FileDialog::new()
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
            let loaded_polygon: EguiPolygon = serde_json::from_reader(reader)?;
            *self = loaded_polygon;
        }
        Ok(())
    }

    fn to_geo_polygon(&self) -> geo::Polygon<f64> {
        let exterior_coords: Vec<_> = self.vertices.iter().map(|&[x, y]| (x, y)).collect();
        let exterior_line_string = geo::LineString::from(exterior_coords);
        geo::Polygon::new(exterior_line_string, vec![])
    }

    pub fn is_inside(&self, x: f64, y: f64) -> bool {
        let point = geo::Point::new(x, y);
        let polygon = self.to_geo_polygon();
        polygon.contains(&point)
    }

    pub fn filter_lf_with_cut(
        &self,
        lf: &LazyFrame,
        x_column: &str,
        y_column: &str,
    ) -> Result<LazyFrame, PolarsError> {
        // lots of clones... maybe there is a better way to do this

        let current_lf = lf.clone();

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
            .filter(col(x_column).gt_eq(lit(x_min)))
            .filter(col(x_column).lt_eq(lit(x_max)))
            .filter(col(y_column).gt_eq(lit(y_min)))
            .filter(col(y_column).lt_eq(lit(y_max)))
            .filter(col(x_column).neq(lit(-1e6)))
            .filter(col(y_column).neq(lit(-1e6)));

        let mask_creation_df = current_lf
            .clone()
            .select([col(x_column), col(y_column)])
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

        Ok(current_lf)
    }
}

// pub fn cut_ui(&mut self, ui: &mut egui::Ui) {
//     ui.horizontal(|ui| {
//         ui.separator();

//         // Y Column ComboBox
//         egui::ComboBox::from_label("Y Column")
//             .selected_text(self.selected_y_column.as_deref().unwrap_or(""))
//             .show_ui(ui, |ui| {
//                 // Use self.column_names instead of CUT_COLUMN_NAMES
//                 for column in &self.column_names {
//                     if ui
//                         .selectable_label(
//                             self.selected_y_column.as_deref() == Some(column),
//                             column,
//                         )
//                         .clicked()
//                     {
//                         self.selected_y_column = Some(column.to_string());
//                     }
//                 }
//             });

//         ui.separator();

//         // X Column ComboBox
//         egui::ComboBox::from_label("X Column")
//             .selected_text(self.selected_x_column.as_deref().unwrap_or(""))
//             .show_ui(ui, |ui| {
//                 // Use self.column_names instead of CUT_COLUMN_NAMES
//                 for column in &self.column_names {
//                     if ui
//                         .selectable_label(
//                             self.selected_x_column.as_deref() == Some(column),
//                             column,
//                         )
//                         .clicked()
//                     {
//                         self.selected_x_column = Some(column.to_string());
//                     }
//                 }
//             });

//         ui.separator();

//         // Load Cut button
//         if ui.button("Load Cut").clicked() {
//             if let Err(e) = self.load_cut_from_json() {
//                 log::error!("Error loading cut: {:?}", e);
//             }
//         }

//         // Save Cut button
//         let can_save: bool =
//             self.selected_x_column.is_some() && self.selected_y_column.is_some();
//         if ui
//             .add_enabled(can_save, egui::Button::new("Save Cut"))
//             .clicked()
//         {
//             if let Err(e) = self.save_cut_to_json() {
//                 log::error!("Error saving cut: {:?}", e);
//             }
//         }

//         ui.separator();
//     });
// }
