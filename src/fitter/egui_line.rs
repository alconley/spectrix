use egui::{Color32, DragValue, Slider, Stroke, Ui};
use egui_plot::{Line, PlotPoint, PlotPoints, PlotUi};

const COLOR_OPTIONS: &[(Color32, &str)] = &[
    (Color32::from_rgb(120, 47, 64), "FSU Garnet"), // go noles!
    (Color32::from_rgb(206, 184, 136), "FSU Gold"),
    (Color32::BLACK, "Black"),
    (Color32::DARK_GRAY, "Dark Gray"),
    (Color32::GRAY, "Gray"),
    (Color32::LIGHT_GRAY, "Light Gray"),
    (Color32::WHITE, "White"),
    (Color32::BROWN, "Brown"),
    (Color32::DARK_RED, "Dark Red"),
    (Color32::RED, "Red"),
    (Color32::LIGHT_RED, "Light Red"),
    (Color32::YELLOW, "Yellow"),
    (Color32::LIGHT_YELLOW, "Light Yellow"),
    (Color32::KHAKI, "Khaki"),
    (Color32::DARK_GREEN, "Dark Green"),
    (Color32::GREEN, "Green"),
    (Color32::LIGHT_GREEN, "Light Green"),
    (Color32::DARK_BLUE, "Dark Blue"),
    (Color32::BLUE, "Blue"),
    (Color32::LIGHT_BLUE, "Light Blue"),
];

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn from_color32(color: Color32) -> Self {
        Rgb {
            r: color.r(),
            g: color.g(),
            b: color.b(),
        }
    }

    pub fn to_color32(self) -> Color32 {
        Color32::from_rgb(self.r, self.g, self.b)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EguiLine {
    pub draw: bool,
    pub name: String,
    pub legend: bool,
    pub highlighted: bool,
    pub stroke: Stroke,
    pub width: f32,
    pub color: Color32,
    pub reference_fill: bool,
    pub fill: f32,
    pub log_y: bool,
    pub log_x: bool,
    pub points: Vec<[f64; 2]>,
    // Use Rgb struct for custom RGB values
    pub color_rgb: Rgb,
    pub stroke_rgb: Rgb,
}

impl EguiLine {
    pub fn new(name: String, color: Color32) -> Self {
        EguiLine {
            draw: true,
            name,
            legend: false,
            highlighted: false,
            stroke: Stroke::new(1.0, color),
            width: 3.0,
            color,
            reference_fill: false,
            fill: 0.0,
            log_y: false,
            log_x: false,
            points: vec![],
            color_rgb: Rgb::from_color32(color),
            stroke_rgb: Rgb::from_color32(color),
        }
    }

    pub fn add_point(&mut self, x: f64, y: f64) {
        self.points.push([x, y]);
    }

    pub fn draw(&self, plot_ui: &mut PlotUi) {
        if self.draw {
            let plot_points: Vec<PlotPoint> = self
                .points
                .iter()
                .map(|&[x, y]| {
                    let x = if self.log_x && x > 0.0 {
                        x.log10().max(0.0001)
                    } else {
                        x
                    };
                    let y = if self.log_y && y > 0.0 {
                        y.log10().max(0.0001)
                    } else {
                        y
                    };
                    PlotPoint::new(x, y)
                })
                .collect();

            let mut line = Line::new(PlotPoints::Owned(plot_points))
                .highlight(self.highlighted)
                .stroke(self.stroke)
                .width(self.width)
                .color(self.color);

            if self.legend {
                line = line.name(self.name.clone());
            }

            if self.reference_fill {
                line = line.fill(self.fill);
            }

            plot_ui.line(line);
        }
    }

    pub fn menu_button(&mut self, ui: &mut Ui) {
        ui.menu_button(format!("{} Line", self.name), |ui| {
            ui.vertical(|ui| {
                ui.checkbox(&mut self.draw, "Draw Line");
                ui.checkbox(&mut self.legend, "Legend")
                    .on_hover_text("Show in legend");
                ui.checkbox(&mut self.highlighted, "Highlighted");

                self.color_selection_buttons(ui);
                ui.add(Slider::new(&mut self.width, 0.0..=10.0).text("Line Width"));

                // self.stroke_color_selection_buttons(ui);
                // ui.add(Slider::new(&mut self.stroke.width, 0.0..=10.0).text("Stroke Width"));

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.reference_fill, "Reference Fill")
                        .on_hover_text("Fill the area under the line");

                    if self.reference_fill {
                        ui.add(
                            DragValue::new(&mut self.fill)
                                .speed(1.0)
                                .prefix("Fill Reference: "),
                        );
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
