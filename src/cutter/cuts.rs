use geo::Contains;
use polars::prelude::*;
use std::fs::File;
use std::io::{BufReader, Write};

use crate::egui_plot_stuff::egui_polygon::EguiPolygon;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Cut {
    pub polygon: EguiPolygon,
    pub x_column: String,
    pub y_column: String,
    #[serde(skip)]
    pub selected: bool,
}

impl Cut {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // putting this in a grid
        ui.text_edit_singleline(&mut self.x_column);

        ui.text_edit_singleline(&mut self.y_column);

        self.polygon.polygon_info_menu_button(ui);
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
            let cut: Cut = serde_json::from_reader(reader)?;
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

    pub fn filter_lf_with_cut(&self, lf: &LazyFrame) -> Result<LazyFrame, PolarsError> {
        let x_column = &self.x_column;
        let y_column = &self.y_column;

        let current_lf = lf.clone();

        // check to see if the column exists
        let check_lf = current_lf.clone().limit(1);
        let df = check_lf.collect().unwrap();
        let columns: Vec<String> = df
            .get_column_names_owned()
            .into_iter()
            .map(|name| name.to_string())
            .collect();

        if !columns.contains(&x_column.to_string()) {
            log::error!("Column {} does not exist", x_column);
            return Err(PolarsError::ColumnNotFound(x_column.to_string().into()));
        }

        if !columns.contains(&y_column.to_string()) {
            log::error!("Column {} does not exist", y_column);
            return Err(PolarsError::ColumnNotFound(y_column.to_string().into()));
        }

        // get the min and max values for the x and y data points in the cuts
        let x_min = self
            .polygon
            .vertices
            .iter()
            .map(|&[x, _]| x)
            .fold(f64::INFINITY, |a, b| a.min(b));
        let x_max = self
            .polygon
            .vertices
            .iter()
            .map(|&[x, _]| x)
            .fold(f64::NEG_INFINITY, |a, b| a.max(b));
        let y_min = self
            .polygon
            .vertices
            .iter()
            .map(|&[_, y]| y)
            .fold(f64::INFINITY, |a, b| a.min(b));
        let y_max = self
            .polygon
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
        let df = current_lf.clone().collect()?;

        let filtered_lf = df.filter(&boolean_chunked_series)?.lazy();

        Ok(filtered_lf)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HistogramCuts {
    pub cuts: Vec<Cut>,
    pub x_column: String,
    pub y_column: String,
}

impl Default for HistogramCuts {
    fn default() -> Self {
        HistogramCuts {
            cuts: vec![],
            x_column: "".to_string(),
            y_column: "".to_string(),
        }
    }
}

impl HistogramCuts {
    pub fn new_cut(&mut self) {
        // make all the polygons not have interactive clicking
        for cut in &mut self.cuts {
            cut.polygon.interactive_clicking = false;
        }

        // get index of the last cut for the default name
        let index = self.cuts.len();
        let default_name = format!("cut_{}", index);
        let new_cut = EguiPolygon::new(&default_name);

        let new_cut = Cut {
            polygon: new_cut,
            x_column: "".to_string(),
            y_column: "".to_string(),
            selected: false,
        };
        self.cuts.push(new_cut);
    }

    fn sycronize_column_names(&mut self) {
        for cut in &mut self.cuts {
            cut.x_column = self.x_column.clone();
            cut.y_column = self.y_column.clone();
        }
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi) {
        for cut in &mut self.cuts {
            cut.polygon.draw(plot_ui);
        }
    }

    pub fn interactive_response(&mut self, plot_response: &egui_plot::PlotResponse<()>) {
        for cuts in &mut self.cuts {
            cuts.polygon.handle_interactions(plot_response);
        }
    }

    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Cuts");
            if ui.button("Add Cut").clicked() {
                self.new_cut();
            }
        });

        ui.horizontal(|ui| {
            ui.label("X Column");
            ui.text_edit_singleline(&mut self.x_column);
        });

        ui.horizontal(|ui| {
            ui.label("Y Column");
            ui.text_edit_singleline(&mut self.y_column);
        });

        self.sycronize_column_names();

        let mut index_to_remove = None;
        for (index, cut) in self.cuts.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                if ui.button("🗙").clicked() {
                    index_to_remove = Some(index);
                }

                ui.separator();

                cut.menu_button(ui);
            });
        }

        if let Some(index) = index_to_remove {
            self.cuts.remove(index);
        }
    }
}
