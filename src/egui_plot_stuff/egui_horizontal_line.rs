use egui::{Color32, DragValue, Id, Slider, Stroke, Ui};
use egui_plot::{HLine, LineStyle, PlotResponse, PlotUi};

use crate::egui_plot_stuff::colors::{Rgb, COLOR_OPTIONS};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EguiHorizontalLine {
    pub draw: bool,
    pub name_in_legend: bool,
    pub name: String,
    pub highlighted: bool,
    pub stroke: Stroke,
    pub width: f32,
    pub color: Color32,

    #[serde(skip)]
    pub style: Option<LineStyle>,
    pub style_length: f32,

    pub y_value: f64,
    // Use Rgb struct for custom RGB values
    pub color_rgb: Rgb,
    pub stroke_rgb: Rgb,

    pub interactive_dragging: bool,
    pub mid_point_radius: f32,

    #[serde(skip)]
    pub is_dragging: bool,
}

impl Default for EguiHorizontalLine {
    fn default() -> Self {
        EguiHorizontalLine {
            draw: true,
            name_in_legend: false,
            name: "Horizontal Line".to_string(),
            highlighted: false,
            stroke: Stroke::new(1.0, Color32::BLUE),
            width: 2.0,
            color: Color32::BLUE,
            style: Some(LineStyle::Solid),
            style_length: 15.0,
            y_value: 0.0,
            color_rgb: Rgb::from_color32(Color32::BLUE),
            stroke_rgb: Rgb::from_color32(Color32::BLUE),
            interactive_dragging: true,
            mid_point_radius: 3.0,
            is_dragging: false,
        }
    }
}

impl EguiHorizontalLine {
    pub fn new(y_value: f64, color: Color32) -> Self {
        let line = EguiHorizontalLine::default();
        EguiHorizontalLine {
            y_value,
            color,
            color_rgb: Rgb::from_color32(color),
            ..line
        }
    }

    pub fn draw(&self, plot_ui: &mut PlotUi) {
        if self.draw {
            let mut line = HLine::new(self.y_value)
                .highlight(self.highlighted)
                .stroke(self.stroke)
                .width(self.width)
                .color(self.color)
                .allow_hover(true)
                .id(Id::new(self.name.clone()));

            if self.name_in_legend {
                line = line.name(self.name.clone());
            }

            if self.style.is_some() {
                line = line.style(self.style.unwrap());
            }

            plot_ui.hline(line);

            if self.interactive_dragging {
                let mid_point_pos: Vec<[f64; 2]> = vec![[
                    (plot_ui.plot_bounds().min()[0] + plot_ui.plot_bounds().max()[0]) / 2.0,
                    self.y_value,
                ]];

                let mid_point = egui_plot::Points::new(mid_point_pos)
                    .color(self.color)
                    .highlight(self.highlighted)
                    .radius(self.mid_point_radius)
                    .id(Id::new(self.name.clone()));

                plot_ui.points(mid_point);
            }
        }
    }

    pub fn interactive_dragging(&mut self, plot_response: &PlotResponse<()>) {
        let pointer_state = plot_response.response.ctx.input(|i| i.pointer.clone());
        if let Some(pointer_pos) = pointer_state.hover_pos() {
            if let Some(hovered_id) = plot_response.hovered_plot_item {
                if hovered_id == Id::new(self.name.clone()) {
                    self.highlighted = true;
                    if pointer_state.button_pressed(egui::PointerButton::Primary) {
                        self.is_dragging = true;
                    }
                } else {
                    self.highlighted = false;
                }
            } else {
                self.highlighted = false;
            }

            if self.is_dragging {
                self.y_value = plot_response.transform.value_from_position(pointer_pos).y;
                if pointer_state.button_released(egui::PointerButton::Primary) {
                    self.is_dragging = false;
                }
            }
        } else if pointer_state.button_released(egui::PointerButton::Primary) {
            self.is_dragging = false;
        }
    }

    pub fn menu_button(&mut self, ui: &mut Ui) {
        ui.menu_button(format!("{} Line", self.name), |ui| {
            ui.label(self.name.to_string());
            ui.vertical(|ui| {
                ui.checkbox(&mut self.draw, "Draw Line");
                ui.add(
                    DragValue::new(&mut self.y_value)
                        .speed(1.0)
                        .prefix("Y Value: "),
                );
                ui.checkbox(&mut self.name_in_legend, "Name in Legend")
                    .on_hover_text("Show in legend");
                ui.checkbox(&mut self.highlighted, "Highlighted");

                self.color_selection_buttons(ui);
                ui.add(Slider::new(&mut self.width, 0.0..=10.0).text("Line Width"));

                // self.stroke_color_selection_buttons(ui);
                // ui.add(Slider::new(&mut self.stroke.width, 0.0..=10.0).text("Stroke Width"));

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

                ui.checkbox(&mut self.interactive_dragging, "Interactive Dragging")
                    .on_hover_text("Enable interactive dragging of the line. Make sure to have a unique name for each line and add the function to the response of the plot.");
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
                    .range(0..=255)
                    .prefix("R: "),
            );
            ui.add(
                DragValue::new(&mut self.color_rgb.g)
                    .range(0..=255)
                    .prefix("G: "),
            );
            ui.add(
                DragValue::new(&mut self.color_rgb.b)
                    .range(0..=255)
                    .prefix("B: "),
            );

            self.color = self.color_rgb.to_color32();
        });
    }

    pub fn stroke_color_selection_buttons(&mut self, ui: &mut Ui) {
        ui.label("Stroke Color");
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
