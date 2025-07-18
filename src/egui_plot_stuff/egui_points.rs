use egui::{Color32, DragValue, Ui};
use egui_plot::{MarkerShape, PlotUi, Points};

use crate::egui_plot_stuff::colors::{Rgb, COLOR_OPTIONS};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EguiPoints {
    pub draw: bool,
    pub name_in_legend: bool,
    pub name: String,
    pub points: Vec<[f64; 2]>,
    #[serde(skip)]
    pub shape: Option<MarkerShape>,
    pub highlighted: bool,
    pub color: Color32,
    pub filled: bool,
    pub add_stem: bool,
    pub stems_y_reference: f32,
    pub radius: f32,
    // Use Rgb struct for custom RGB values
    pub color_rgb: Rgb,
}

impl Default for EguiPoints {
    fn default() -> Self {
        EguiPoints {
            draw: true,
            name_in_legend: true,
            name: "Points".to_string(),
            points: vec![],
            shape: Some(MarkerShape::Circle),
            highlighted: false,
            color: Color32::BLUE,
            filled: true,
            add_stem: false,
            stems_y_reference: 0.0,
            radius: 3.0,
            color_rgb: Rgb::from_color32(Color32::BLUE),
        }
    }
}

impl EguiPoints {
    pub fn _new(points: Vec<[f64; 2]>, color: Color32) -> Self {
        let line = EguiPoints::default();
        EguiPoints {
            points,
            color,
            color_rgb: Rgb::from_color32(color),
            ..line
        }
    }

    pub fn _add_point(&mut self, x: f64, y: f64) {
        self.points.push([x, y]);
    }

    pub fn draw(&self, plot_ui: &mut PlotUi) {
        if self.draw {
            let mut points = Points::new(self.points.clone())
                .highlight(self.highlighted)
                .color(self.color)
                .radius(self.radius)
                .filled(self.filled);

            if self.name_in_legend {
                points = points.name(self.name.clone());
            }

            if self.add_stem {
                points = points.stems(self.stems_y_reference);
            }

            if self.shape.is_some() {
                points = points.shape(self.shape.expect("Shape should be set"));
            }

            plot_ui.points(points);
        }
    }

    pub fn menu_button(&mut self, ui: &mut Ui) {
        ui.menu_button(format!("{} Points", self.name), |ui| {
            ui.vertical(|ui| {
                ui.checkbox(&mut self.draw, "Draw Markers");

                ui.checkbox(&mut self.name_in_legend, "Name in Legend")
                    .on_hover_text("Show in legend");
                ui.checkbox(&mut self.highlighted, "Highlighted");

                self.color_selection_buttons(ui);

                ui.checkbox(&mut self.filled, "Filled");

                ui.add(
                    DragValue::new(&mut self.radius)
                        .speed(0.1)
                        .prefix("Radius: "),
                );

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.add_stem, "Add Stem");
                    ui.add(
                        DragValue::new(&mut self.stems_y_reference)
                            .speed(0.1)
                            .prefix("Y: "),
                    );
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label("Marker Shape: ");
                    ui.selectable_value(&mut self.shape, Some(MarkerShape::Circle), "Circle");
                    ui.selectable_value(&mut self.shape, Some(MarkerShape::Diamond), "Diamond");
                    ui.selectable_value(&mut self.shape, Some(MarkerShape::Square), "Square");
                    ui.selectable_value(&mut self.shape, Some(MarkerShape::Cross), "Cross");
                    ui.selectable_value(&mut self.shape, Some(MarkerShape::Plus), "Plus");
                    ui.selectable_value(&mut self.shape, Some(MarkerShape::Up), "Up");
                    ui.selectable_value(&mut self.shape, Some(MarkerShape::Down), "Down");
                    ui.selectable_value(&mut self.shape, Some(MarkerShape::Left), "Left");
                    ui.selectable_value(&mut self.shape, Some(MarkerShape::Right), "Right");
                    ui.selectable_value(&mut self.shape, Some(MarkerShape::Asterisk), "Asterisk");
                });

                ui.collapsing("Points", |ui| {
                    ui.label("Points: X, Y");

                    for point in self.points.iter_mut() {
                        ui.label(format!("{}, {}", point[0], point[1]));
                    }
                });
            });
        });
    }

    pub fn color_selection_buttons(&mut self, ui: &mut Ui) {
        ui.label("Line Color");

        ui.horizontal_wrapped(|ui| {
            for &(color, name) in COLOR_OPTIONS.iter() {
                if ui
                    .add(egui::Button::new(" ").fill(color))
                    .on_hover_text(name)
                    .clicked()
                {
                    self.color = color;
                    self.color_rgb = Rgb::from_color32(color);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("RGB: ");
            ui.add(
                DragValue::new(&mut self.color_rgb.r)
                    .clamp_range(0..=255)
                    .prefix("R: "),
            );
            ui.add(
                DragValue::new(&mut self.color_rgb.g)
                    .clamp_range(0..=255)
                    .prefix("G: "),
            );
            ui.add(
                DragValue::new(&mut self.color_rgb.b)
                    .clamp_range(0..=255)
                    .prefix("B: "),
            );

            self.color = self.color_rgb.to_color32();
        });
    }
}
