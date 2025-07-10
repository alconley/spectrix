use egui::{Color32, DragValue, Slider, Stroke, Ui};
use egui_plot::{Line, LineStyle, PlotPoint, PlotPoints, PlotUi};

use crate::{
    egui_plot_stuff::colors::{Rgb, COLOR_OPTIONS},
    fitter::common::Calibration,
};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EguiLine {
    pub draw: bool,
    pub name_in_legend: bool,
    pub log_y: bool,
    pub log_x: bool,
    pub name: String,
    pub highlighted: bool,
    pub stroke: Stroke,
    pub width: f32,
    pub color: Color32,
    pub reference_fill: bool,
    pub fill: f32,
    pub fill_alpha: f32,

    #[serde(skip)]
    pub style: Option<LineStyle>,

    pub style_length: f32,
    pub points: Vec<[f64; 2]>,
    // Use Rgb struct for custom RGB values
    pub color_rgb: Rgb,
    pub stroke_rgb: Rgb,
}

impl Default for EguiLine {
    fn default() -> Self {
        EguiLine {
            draw: true,
            name_in_legend: false,
            log_y: false,
            log_x: false,
            name: "Line".to_string(),
            highlighted: false,
            stroke: Stroke::new(1.0, Color32::LIGHT_BLUE),
            width: 2.0,
            color: Color32::LIGHT_BLUE,
            reference_fill: false,
            fill: 0.0,
            fill_alpha: 0.3,
            style: Some(LineStyle::Solid),
            style_length: 15.0,
            points: vec![],
            color_rgb: Rgb::from_color32(Color32::LIGHT_BLUE),
            stroke_rgb: Rgb::from_color32(Color32::LIGHT_BLUE),
        }
    }
}

impl EguiLine {
    pub fn new(color: Color32) -> Self {
        let line = EguiLine::default();
        EguiLine {
            color,
            color_rgb: Rgb::from_color32(color),
            ..line
        }
    }

    pub fn new_with_points(points: Vec<[f64; 2]>) -> Self {
        EguiLine {
            points,
            ..EguiLine::default()
        }
    }

    pub fn clear_points(&mut self) {
        self.points.clear();
    }

    pub fn add_point(&mut self, x: f64, y: f64) {
        self.points.push([x, y]);
    }

    pub fn draw(&self, plot_ui: &mut PlotUi<'_>, calibration: Option<&Calibration>) {
        if self.draw {
            let plot_points: Vec<PlotPoint> = self
                .points
                .iter()
                .map(|&[x, y]| {
                    let calibrated_x = if let Some(cal) = calibration {
                        cal.calibrate(x)
                    } else {
                        x
                    };

                    let x = if self.log_x && calibrated_x > 0.0 {
                        calibrated_x.log10().max(0.0001)
                    } else {
                        calibrated_x
                    };

                    let y = if self.log_y && y > 0.0 {
                        y.log10().max(0.0001)
                    } else {
                        y
                    };

                    PlotPoint::new(x, y)
                })
                .collect();

            let mut line = Line::new("", PlotPoints::Owned(plot_points))
                .highlight(self.highlighted)
                .stroke(self.stroke)
                .width(self.width)
                .color(self.color)
                .id(egui::Id::new(self.name.clone()));

            if self.name_in_legend {
                line = line.name(self.name.clone());
            }

            if self.reference_fill {
                line = line.fill(self.fill);
                line = line.fill_alpha(self.fill_alpha);
            }

            if self.style.is_some() {
                line = line.style(self.style.unwrap());
            }

            plot_ui.line(line);
        }
    }

    pub fn menu_button(&mut self, ui: &mut Ui) {
        ui.menu_button(format!("{} Line", self.name), |ui| {
            ui.label(self.name.to_string());
            ui.vertical(|ui| {
                ui.checkbox(&mut self.draw, "Draw Line");
                ui.checkbox(&mut self.name_in_legend, "Name in Legend")
                    .on_hover_text("Show in legend");
                ui.checkbox(&mut self.highlighted, "Highlighted");

                // self.color_selection_buttons(ui);
                // ui.color_edit_button_srgba(&mut self.color); // add this when the bug is fixed
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
                        ui.add(
                            DragValue::new(&mut self.fill_alpha)
                                .speed(0.01)
                                .range(0.0..=1.0)
                                .prefix("Fill Alpha: "),
                        );
                    }
                });

                // ui.horizontal(|ui| {
                //     ui.checkbox(&mut self.log_x, "Log X")
                //         .on_hover_text("Logarithmic scale data on the x-axis");
                //     ui.checkbox(&mut self.log_y, "Log Y")
                //         .on_hover_text("Logarithmic scale data on the y-axis");
                // });

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

                ui.collapsing("Points", |ui| {
                    if ui
                        .button("📋")
                        .on_hover_text("Copy points to clipboard")
                        .clicked()
                    {
                        let points_str = self
                            .points
                            .iter()
                            .map(|point| format!("{}, {}", point[0], point[1]))
                            .collect::<Vec<String>>()
                            .join("\n");
                        ui.ctx().copy_text(points_str);
                    }

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label("X, Y");
                            for point in self.points.iter_mut() {
                                ui.horizontal(|ui| {
                                    ui.label(format!("{}, {}", point[0], point[1]));
                                });
                            }
                        });
                    });
                });
            });
        });
    }

    pub fn set_color(&mut self, color: Color32) {
        self.color = color;
        self.color_rgb = Rgb::from_color32(color);
        self.stroke.color = color;
        self.stroke_rgb = Rgb::from_color32(color);
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
