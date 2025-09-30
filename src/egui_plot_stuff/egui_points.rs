use egui::color_picker::{Alpha, color_picker_color32};
use egui::containers::menu::SubMenuButton;
use egui::{Atom, Button, Color32, DragValue, RichText, Ui};
use egui_plot::{MarkerShape, PlotUi, Points};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EguiPoints {
    pub draw: bool,
    #[serde(skip)]
    pub shape: Option<MarkerShape>,
    pub color: Color32,
    pub filled: bool,
    pub add_stem: bool,
    pub stem_y_reference: f32,
    pub radius: f32,
    pub name: String,
    pub highlighted: bool,
    pub log_y: bool,
    pub name_in_legend: bool,
    pub points: Vec<[f64; 2]>,
    pub uncertainty: Option<f64>,
}

impl Default for EguiPoints {
    fn default() -> Self {
        Self {
            draw: true,
            shape: Some(MarkerShape::Circle),
            color: Color32::LIGHT_BLUE,
            filled: true,
            add_stem: false,
            stem_y_reference: 0.0,
            radius: 3.0,
            name: "Points".to_owned(),
            highlighted: false,
            log_y: false,
            name_in_legend: false,
            points: vec![],
            uncertainty: None,
        }
    }
}

impl EguiPoints {
    pub fn new(name: &str, points: Vec<[f64; 2]>, color: Color32) -> Self {
        Self {
            name: name.to_owned(),
            points,
            color,
            ..Default::default()
        }
    }

    pub fn new_cross_section(name: &str, x: f64, y: f64, uncertainty: f64, color: Color32) -> Self {
        Self {
            name: name.to_owned(),
            points: vec![[x, y]],
            color,
            log_y: true,
            uncertainty: Some(uncertainty),
            ..Default::default()
        }
    }

    pub fn add_point(&mut self, x: f64, y: f64) {
        self.points.push([x, y]);
    }

    pub fn draw(&self, plot_ui: &mut PlotUi<'_>, clamp: bool) {
        if self.draw {
            const EPS: f64 = 1e-12; // clamp in linear space only

            // transform points if log_y
            let mut transformed: Vec<[f64; 2]> = Vec::with_capacity(self.points.len());
            for &[x, y] in &self.points {
                let y_t = if self.log_y {
                    (y.max(EPS)).log10() // <-- clamp in linear, then log
                } else {
                    y
                };
                transformed.push([x, y_t]);
            }

            let mut points = Points::new(self.name.clone(), transformed)
                .highlight(self.highlighted)
                .color(self.color)
                .radius(self.radius)
                .filled(self.filled);

            if self.name_in_legend {
                points = points.name(self.name.clone());
            }

            if self.add_stem {
                let stem = if self.log_y {
                    (f64::from(self.stem_y_reference).max(EPS)).log10() // <-- fix stem too
                } else {
                    f64::from(self.stem_y_reference)
                } as f32;
                points = points.stems(stem);
            }

            if self.shape.is_some() {
                points = points.shape(self.shape.expect("Shape should be set"));
            }

            // Uncertainty bars (vertical).
            if let Some(unc) = self.uncertainty {
                for &[x, y] in &self.points {
                    if !y.is_finite() || !unc.is_finite() {
                        continue;
                    }

                    if self.log_y {
                        // Asymmetric in log space: transform ends separately.
                        let y_hi_lin = (y + unc).max(EPS);

                        // If y - unc <= 0, draw bar down to EPS (bottom of plot)
                        let y_lo_lin = if y - unc > 0.0 { y - unc } else { EPS };

                        let y_lo = y_lo_lin.log10();
                        let y_hi = y_hi_lin.log10();

                        let bar = egui_plot::Line::new(
                            format!("{} Uncertainty Bar", self.name),
                            vec![[x, y_lo], [x, y_hi]],
                        )
                        .color(self.color)
                        .width(self.radius / 3.0);

                        plot_ui.line(bar);
                    } else {
                        // Linear space: symmetric, but clamp at 0.0
                        let y_lo = if clamp { (y - unc).max(0.0) } else { y - unc };
                        // let y_lo = (y - unc).max(0.0);
                        let y_hi = y + unc;

                        let bar = egui_plot::Line::new(
                            format!("{} Uncertainty Bar", self.name),
                            vec![[x, y_lo], [x, y_hi]],
                        )
                        .color(self.color)
                        .width(self.radius / 3.0);

                        plot_ui.line(bar);
                    }
                }
            }

            plot_ui.points(points);
        }
    }

    pub fn menu_button(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.checkbox(&mut self.draw, "Draw Markers");

            ui.checkbox(&mut self.name_in_legend, "Name in Legend")
                .on_hover_text("Show in legend");
            ui.checkbox(&mut self.highlighted, "Highlighted");

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

            ui.checkbox(&mut self.filled, "Filled");

            ui.add(
                DragValue::new(&mut self.radius)
                    .speed(0.1)
                    .prefix("Radius: "),
            );

            ui.horizontal(|ui| {
                ui.checkbox(&mut self.add_stem, "Add Stem");
                ui.add(
                    DragValue::new(&mut self.stem_y_reference)
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

                for point in &mut self.points {
                    ui.label(format!("{}, {}", point[0], point[1]));
                }
            });
        });
    }
}
