use egui::{Color32, DragValue, Slider, Stroke, Ui};
use egui_plot::{Polygon, LineStyle, PlotPoints, PlotUi};

use crate::egui_plot_stuff::colors::{Rgb, COLOR_OPTIONS};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
    pub vertices_radius: f32,
    // Use Rgb struct for custom RGB values
    pub color_rgb: Rgb,
    pub stroke_rgb: Rgb,

    pub interactive: bool,
}

impl Default for EguiPolygon {
    fn default() -> Self {
        EguiPolygon {
            draw: true,
            name_in_legend: false,
            name: "Polygon".to_string(),
            highlighted: false,
            stroke: Stroke::new(1.0, Color32::RED),
            width: 2.0,
            fill_color: Color32::TRANSPARENT,
            style: Some(LineStyle::Solid),
            style_length: 15.0,
            vertices: vec![],
            vertices_radius: 5.0,
            color_rgb: Rgb::from_color32(Color32::RED),
            stroke_rgb: Rgb::from_color32(Color32::RED),

            interactive: false,
        }
    }
}

impl EguiPolygon {
    pub fn new(name: &str) -> Self {
        EguiPolygon {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn mouse_interactions(&mut self, plot_ui: &PlotUi) {
        if self.interactive && self.draw {
            let response = plot_ui.response();

            if response.clicked() {
                let pointer_pos = plot_ui.pointer_coordinate().unwrap();
                self.add_vertex(pointer_pos.x, pointer_pos.y);
            }
        }
    }

    pub fn keybinds(&mut self, ui: &mut egui::Ui, cursor_position: Option<egui_plot::PlotPoint>) {
        if let Some(_cursor_position) = cursor_position {
            if ui.input(|i| i.key_pressed(egui::Key::C)) {
                self.draw = !self.draw;
                self.interactive = !self.interactive;
                log::info!("{} Polygon -> Draw: {}, Interactive: {}", self.name, self.draw, self.interactive);
            }

            if ui.input(|i| i.key_pressed(egui::Key::Delete))
                || ui.input(|i| i.key_pressed(egui::Key::Backspace))
            {
                self.clear_vertices();
                log::info!("{} Polygon -> Vertices Cleared", self.name);
            }
        }
    }

    pub fn add_vertex(&mut self, x: f64, y: f64) {
        self.vertices.push([x, y]);
    }

    pub fn clear_vertices(&mut self) {
        self.vertices.clear();
    }

    pub fn draw(&self, plot_ui: &mut PlotUi) {
        if self.draw {
            let vertices: PlotPoints = PlotPoints::new(self.vertices.clone());
            let vertices_points = egui_plot::Points::new(self.vertices.clone())
                .radius(self.vertices_radius)
                .color(self.stroke.color);

            let mut polygon = Polygon::new(vertices)
                .highlight(self.highlighted)
                .stroke(self.stroke)
                .width(self.width)
                .fill_color(Color32::TRANSPARENT);

            if self.name_in_legend {
                polygon = polygon.name(self.name.clone());
            }

            if self.style.is_some() {
                polygon = polygon.style(self.style.unwrap());
            }

            plot_ui.polygon(polygon);
            plot_ui.points(vertices_points); 
        }
    }

    pub fn menu_button(&mut self, ui: &mut Ui) {
        ui.menu_button(format!("{}", self.name), |ui| {
            ui.vertical(|ui| {
                ui.text_edit_singleline(&mut self.name);
                ui.checkbox(&mut self.draw, "Draw Polygon");
                ui.checkbox(&mut self.interactive, "Interactive");
                ui.checkbox(&mut self.name_in_legend, "Name in Legend")
                    .on_hover_text("Show in legend");
                ui.checkbox(&mut self.highlighted, "Highlighted");

                ui.add(
                    DragValue::new(&mut self.vertices_radius)
                        .speed(0.1)
                        .prefix("Vertex Radius: "),
                );

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
                            .clamp_range(0.0..=f32::INFINITY)
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

    pub fn stroke_color_selection_buttons(&mut self, ui: &mut Ui) {
        ui.label("Color");
        ui.horizontal_wrapped(|ui| {
            for &(color, _) in COLOR_OPTIONS.iter() {
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
                    .clamp_range(0..=255)
                    .prefix("R: "),
            );
            ui.add(
                DragValue::new(&mut self.stroke_rgb.g)
                    .clamp_range(0..=255)
                    .prefix("G: "),
            );
            ui.add(
                DragValue::new(&mut self.stroke_rgb.b)
                    .clamp_range(0..=255)
                    .prefix("B: "),
            );

            self.stroke.color = self.stroke_rgb.to_color32();
        });
    }
}
