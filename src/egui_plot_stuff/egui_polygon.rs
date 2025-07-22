use egui::{Color32, DragValue, Id, Slider, Stroke, Ui};
use egui_plot::{LineStyle, PlotResponse, PlotUi, Polygon};
use geo::Contains as _;

use crate::egui_plot_stuff::colors::{COLOR_OPTIONS, Rgb};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct EguiPolygon {
    pub draw: bool,
    pub name_in_legend: bool,
    pub name: String,
    pub highlighted: bool,
    pub stroke: Stroke,
    pub width: f32,
    pub fill_color: Color32,
    #[serde(skip)]
    pub style: Option<LineStyle>,
    pub style_length: f32,
    pub vertices: Vec<[f64; 2]>,
    // Use Rgb struct for custom RGB values
    pub color_rgb: Rgb,
    pub stroke_rgb: Rgb,

    pub interactive_clicking: bool,
    pub interactive_dragging: bool,

    #[serde(skip)]
    temp_vertex: Option<Vec<[f64; 2]>>,
    #[serde(skip)]
    pub is_dragging: bool,
    #[serde(skip)]
    dragged_vertex_index: Option<usize>,
}

impl Default for EguiPolygon {
    fn default() -> Self {
        Self {
            draw: true,
            name_in_legend: false,
            name: "Polygon".to_owned(),
            highlighted: false,
            stroke: Stroke::new(1.0, Color32::RED),
            width: 2.0,
            fill_color: Color32::TRANSPARENT,
            style: Some(LineStyle::Solid),
            style_length: 15.0,
            vertices: vec![],
            color_rgb: Rgb::from_color32(Color32::RED),
            stroke_rgb: Rgb::from_color32(Color32::RED),

            interactive_clicking: false,
            interactive_dragging: true,
            temp_vertex: None,
            is_dragging: false,
            dragged_vertex_index: None,
        }
    }
}

impl EguiPolygon {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            interactive_clicking: true,
            ..Default::default()
        }
    }

    fn to_geo_polygon(&self) -> geo::Polygon<f64> {
        let exterior_coords: Vec<(f64, f64)> =
            self.vertices.iter().map(|&arr| arr.into()).collect();
        let exterior_line_string = geo::LineString::from(exterior_coords);
        geo::Polygon::new(exterior_line_string, vec![])
    }

    pub fn is_inside(&self, x: f64, y: f64) -> bool {
        let point = geo::Point::new(x, y);
        let polygon = self.to_geo_polygon();
        polygon.contains(&point)
    }

    pub fn handle_interactions(&mut self, plot_response: &PlotResponse<()>) {
        let pointer_state = plot_response.response.ctx.input(|i| i.pointer.clone());
        if let Some(pointer_pos) = pointer_state.hover_pos() {
            let x_value = plot_response.transform.value_from_position(pointer_pos).x;
            let y_value = plot_response.transform.value_from_position(pointer_pos).y;

            if self.interactive_clicking && self.draw {
                self.temp_vertex = Some(vec![[x_value, y_value]]);
                if plot_response.response.clicked() {
                    self.add_vertex(x_value, y_value);
                }

                if plot_response.response.double_clicked() {
                    self.temp_vertex = None;
                    self.interactive_clicking = false;
                }
            } else {
                self.temp_vertex = None;
            }

            if self.interactive_dragging && self.draw {
                if let Some(hovered_id) = plot_response.hovered_plot_item {
                    if hovered_id == Id::new(self.name.clone()) {
                        self.highlighted = true;

                        //find the closest vertex to the pointer
                        let closest_index = self
                            .vertices
                            .iter()
                            .enumerate()
                            .min_by(|(_, a), (_, b)| {
                                let dist_a = (a[0] - x_value).powi(2) + (a[1] - y_value).powi(2);
                                let dist_b = (b[0] - x_value).powi(2) + (b[1] - y_value).powi(2);
                                dist_a.partial_cmp(&dist_b).expect("Comparison failed")
                            })
                            .map(|(index, _)| index);

                        log::info!(
                            "Closest index: {:?}, (x,y)={:?}",
                            closest_index,
                            self.vertices[closest_index.expect("Closest index should be found")]
                        );

                        if pointer_state.button_pressed(egui::PointerButton::Primary) {
                            self.is_dragging = true;
                            self.dragged_vertex_index = closest_index;
                        }
                    } else {
                        self.highlighted = false;
                    }
                }

                if self.is_dragging {
                    if let Some(index) = self.dragged_vertex_index {
                        self.vertices[index] = [x_value, y_value];
                    }
                    if pointer_state.button_released(egui::PointerButton::Primary) {
                        self.is_dragging = false;
                        self.dragged_vertex_index = None;
                    }
                }
            }
        } else if pointer_state.button_released(egui::PointerButton::Primary) {
            self.is_dragging = false;
        }
    }

    pub fn add_vertex(&mut self, x: f64, y: f64) {
        self.vertices.push([x, y]);
    }

    pub fn clear_vertices(&mut self) {
        self.vertices.clear();
    }

    pub fn draw(&mut self, plot_ui: &mut PlotUi<'_>) {
        if self.draw {
            // draw the temp vertex
            if let Some(temp_vertex) = &self.temp_vertex {
                let temp_vertex_points = egui_plot::Points::new("", temp_vertex.clone())
                    .radius(5.0)
                    .color(self.stroke.color);

                plot_ui.points(temp_vertex_points);
            }

            let mut polygon = Polygon::new("", self.vertices.clone())
                .highlight(self.highlighted)
                .stroke(self.stroke)
                .width(self.width)
                .fill_color(Color32::TRANSPARENT)
                .id(Id::new(self.name.clone()));

            if self.name_in_legend {
                polygon = polygon.name(self.name.clone());
            }

            if self.style.is_some() {
                polygon = polygon.style(self.style.expect("Style should be set"));
            }

            plot_ui.polygon(polygon);

            // if the user can drag the vertices, draw the vertices
            if self.interactive_dragging {
                let vertices_points = egui_plot::Points::new("", self.vertices.clone())
                    .radius(5.0)
                    .color(self.stroke.color)
                    .id(Id::new(self.name.clone()))
                    .highlight(self.highlighted);

                plot_ui.points(vertices_points);
            }
        }
    }

    pub fn menu_button(&mut self, ui: &mut Ui) {
        ui.menu_button(self.name.clone(), |ui| {
            ui.vertical(|ui| {
                ui.text_edit_singleline(&mut self.name);
                ui.checkbox(&mut self.draw, "Draw Polygon");
                ui.checkbox(
                    &mut self.interactive_clicking,
                    "Interactive Adding Vertices",
                );
                ui.checkbox(
                    &mut self.interactive_dragging,
                    "Interactive Dragging Vertices",
                );
                ui.checkbox(&mut self.name_in_legend, "Name in Legend")
                    .on_hover_text("Show in legend");
                ui.checkbox(&mut self.highlighted, "Highlighted");

                ui.add(Slider::new(&mut self.width, 0.0..=10.0).text("Line Width"));

                self.stroke_color_selection_buttons(ui);

                ui.horizontal(|ui| {
                    ui.label("Line Style: ");
                    ui.radio_value(&mut self.style, Some(LineStyle::Solid), "Solid");
                    ui.radio_value(
                        &mut self.style,
                        Some(LineStyle::Dotted {
                            spacing: self.style_length,
                        }),
                        "Dotted",
                    );
                    ui.radio_value(
                        &mut self.style,
                        Some(LineStyle::Dashed {
                            length: self.style_length,
                        }),
                        "Dashed",
                    );
                    ui.add(
                        DragValue::new(&mut self.style_length)
                            .speed(1.0)
                            .range(0.0..=f32::INFINITY)
                            .prefix("Length: "),
                    );
                });
            });

            ui.separator();
            if ui.button("Clear Vertices").clicked() {
                self.clear_vertices();
            }
        });
    }

    pub fn polygon_info_menu_button(&mut self, ui: &mut Ui) {
        ui.menu_button(self.name.clone(), |ui| {
            ui.text_edit_singleline(&mut self.name);

            ui.label("Vertices (X,Y)");
            for (index, vertex) in self.vertices.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("Vertex {index}"));
                    ui.label(format!("({:.2}, {:.2})", vertex[0], vertex[1]));
                });
            }
        });
    }

    pub fn stroke_color_selection_buttons(&mut self, ui: &mut Ui) {
        ui.label("Color");
        ui.horizontal_wrapped(|ui| {
            for &(color, _) in COLOR_OPTIONS {
                if ui.add(egui::Button::new(" ").fill(color)).clicked() {
                    self.stroke.color = color;
                    self.stroke_rgb = Rgb::from_color32(color);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("RGB: ");
            ui.add(
                DragValue::new(&mut self.stroke_rgb.r)
                    .range(0..=255)
                    .prefix("R: "),
            );
            ui.add(
                DragValue::new(&mut self.stroke_rgb.g)
                    .range(0..=255)
                    .prefix("G: "),
            );
            ui.add(
                DragValue::new(&mut self.stroke_rgb.b)
                    .range(0..=255)
                    .prefix("B: "),
            );

            self.stroke.color = self.stroke_rgb.to_color32();
        });
    }
}
