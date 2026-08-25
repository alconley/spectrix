use egui::color_picker::{Alpha, color_picker_color32};
use egui::containers::menu::SubMenuButton;
use egui::{Atom, Button, Color32, DragValue, RichText, Slider, Stroke, Ui};
use egui_plot::{Line, PlotPoint, PlotPoints, PlotUi};

use crate::{
    egui_plot_stuff::colors::{COLOR_OPTIONS, Rgb},
    egui_plot_stuff::line_style::SerializableLineStyle,
    fitter::common::Calibration,
};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct EguiLine {
    pub draw: bool,
    pub name_in_legend: bool,
    pub allow_hover: bool,
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

    #[serde(default)]
    pub style: SerializableLineStyle,

    pub style_length: f32,
    pub points: Vec<[f64; 2]>,
    // Use Rgb struct for custom RGB values
    pub color_rgb: Rgb,
    pub stroke_rgb: Rgb,
}

impl Default for EguiLine {
    fn default() -> Self {
        Self {
            draw: true,
            name_in_legend: false,
            allow_hover: true,
            log_y: false,
            log_x: false,
            name: "Line".to_owned(),
            highlighted: false,
            stroke: Stroke::new(1.0, Color32::LIGHT_BLUE),
            width: 1.0,
            color: Color32::LIGHT_BLUE,
            reference_fill: false,
            fill: 0.0,
            fill_alpha: 0.3,
            style: SerializableLineStyle::Solid,
            style_length: 15.0,
            points: vec![],
            color_rgb: Rgb::from_color32(Color32::LIGHT_BLUE),
            stroke_rgb: Rgb::from_color32(Color32::LIGHT_BLUE),
        }
    }
}

impl EguiLine {
    pub fn new(color: Color32) -> Self {
        let line = Self::default();
        Self {
            color,
            color_rgb: Rgb::from_color32(color),
            ..line
        }
    }

    pub fn new_with_points(points: Vec<[f64; 2]>) -> Self {
        Self {
            points,
            ..Self::default()
        }
    }

    pub fn clear_points(&mut self) {
        self.points.clear();
    }

    pub fn add_point(&mut self, x: f64, y: f64) {
        self.points.push([x, y]);
    }

    pub fn set_points(&mut self, points: Vec<[f64; 2]>) {
        self.points = points;
    }

    fn transform_x(&self, x: f64, calibration: Option<&Calibration>) -> Option<f64> {
        let calibrated_x = if let Some(calibration) = calibration {
            calibration.calibrate_checked(x)?
        } else {
            x
        };
        let transformed_x = if self.log_x && calibrated_x > 0.0 {
            calibrated_x.log10().max(0.0001)
        } else {
            calibrated_x
        };
        transformed_x.is_finite().then_some(transformed_x)
    }

    fn transformed_x_extent(&self, calibration: Option<&Calibration>) -> Option<(f64, f64)> {
        let first = self.points.first()?;
        let last = self.points.last()?;
        let first_x = self.transform_x(first[0], calibration)?;
        let last_x = self.transform_x(last[0], calibration)?;
        Some((first_x.min(last_x), first_x.max(last_x)))
    }

    pub fn overlaps_visible_x(
        &self,
        plot_ui: &PlotUi<'_>,
        calibration: Option<&Calibration>,
    ) -> bool {
        let Some((line_min, line_max)) = self.transformed_x_extent(calibration) else {
            // A non-monotonic or partially invalid calibration is handled by the
            // normal point filter. Avoid incorrectly culling it here.
            return !self.points.is_empty();
        };
        let bounds = plot_ui.plot_bounds();
        let visible_min = bounds.min()[0];
        let visible_max = bounds.max()[0];
        !visible_min.is_finite()
            || !visible_max.is_finite()
            || (line_max >= visible_min && line_min <= visible_max)
    }

    pub fn draw(&self, plot_ui: &mut PlotUi<'_>, calibration: Option<&Calibration>) {
        if self.draw {
            let plot_points = self
                .points
                .iter()
                .filter_map(|&[x, y]| {
                    let x = self.transform_x(x, calibration)?;
                    let y = if self.log_y && y > 0.0 {
                        y.log10().max(0.0001)
                    } else {
                        y
                    };
                    if y.is_finite() {
                        Some(PlotPoint::new(x, y))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            if plot_points.len() < 2 {
                return;
            }

            let mut line = Line::new("", PlotPoints::Owned(plot_points))
                .allow_hover(self.allow_hover)
                .highlight(self.highlighted)
                .stroke(self.stroke)
                .width(self.width)
                .color(self.color)
                .id(egui::Id::new(&self.name));

            if self.name_in_legend {
                line = line.name(self.name.clone());
            }

            if self.reference_fill {
                line = line.fill(self.fill);
                line = line.fill_alpha(self.fill_alpha);
            }

            line = line.style(self.style.to_egui(self.style_length));

            plot_ui.line(line);
        }
    }

    pub fn menu_button(&mut self, ui: &mut Ui) {
        ui.label(self.name.clone());
        ui.vertical(|ui| {
            ui.checkbox(&mut self.draw, "Draw Line");
            ui.checkbox(&mut self.name_in_legend, "Name in Legend")
                .on_hover_text("Show in legend");
            ui.checkbox(&mut self.highlighted, "Highlighted");

            // Color automatically changed based of light/dark mode in histogram1d.rs
            let button = Button::new((
                RichText::new("Color").color(self.color),
                Atom::grow(),
                RichText::new(SubMenuButton::RIGHT_ARROW).color(self.color),
            ))
            .fill(self.color);

            SubMenuButton::from_button(button).ui(ui, |ui| {
                ui.spacing_mut().slider_width = 200.0;
                color_picker_color32(ui, &mut self.color, Alpha::Opaque);
            });

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
                ui.radio_value(&mut self.style, SerializableLineStyle::Solid, "Solid");
                ui.radio_value(&mut self.style, SerializableLineStyle::Dotted, "Dotted");
                ui.radio_value(&mut self.style, SerializableLineStyle::Dashed, "Dashed");
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
                        for point in &self.points {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}, {}", point[0], point[1]));
                            });
                        }
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
            for &(color, name) in COLOR_OPTIONS {
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

#[cfg(test)]
mod tests {
    use super::EguiLine;
    use crate::fitter::common::Calibration;

    #[test]
    fn transformed_extent_tracks_calibration_and_log_scale() {
        let mut line = EguiLine::new_with_points(vec![[2.0, 1.0], [8.0, 2.0]]);
        assert_eq!(line.transformed_x_extent(None), Some((2.0, 8.0)));

        let mut calibration = Calibration::default();
        calibration.b.value = 2.0;
        calibration.c.value = 1.0;
        assert_eq!(
            line.transformed_x_extent(Some(&calibration)),
            Some((5.0, 17.0))
        );

        line.log_x = true;
        let extent = line
            .transformed_x_extent(Some(&calibration))
            .expect("valid transformed extent");
        assert!((extent.0 - 5.0_f64.log10()).abs() < 1.0e-12);
        assert!((extent.1 - 17.0_f64.log10()).abs() < 1.0e-12);
    }
}
