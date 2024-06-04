use egui::{Color32, Stroke};
use egui_plot::{PlotPoint, PlotUi, VLine};

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EguiFitMarkers {
    pub region_markers: Vec<f64>,
    pub peak_markers: Vec<f64>,
    pub background_markers: Vec<f64>,

    #[serde(skip)]
    pub cursor_position: Option<PlotPoint>,

    #[serde(skip)]
    pub manual_marker_position: f64,
}

impl EguiFitMarkers {
    pub fn new() -> Self {
        Self::default()
    }

    // General marker functions
    fn add_marker(markers: &mut Vec<f64>, x: f64) {
        markers.push(x);
        markers.sort_by(|a, b| a.partial_cmp(b).unwrap());
    }

    fn clear_markers(markers: &mut Vec<f64>) {
        markers.clear();
    }

    fn draw_markers(markers: &[f64], color: Color32, plot_ui: &mut PlotUi) {
        for &x in markers {
            let line = VLine::new(x).color(color).stroke(Stroke::new(1.0, color));
            plot_ui.vline(line);
        }
    }

    fn delete_marker(markers: &mut Vec<f64>, marker_to_delete: f64) {
        if let Some(index) = markers.iter().position(|&x| x == marker_to_delete) {
            markers.remove(index);
        }
    }

    pub fn delete_closest_marker(&mut self) {
        if let Some(cursor_pos) = self.cursor_position {
            let mut all_markers: Vec<(f64, &str)> = vec![];

            all_markers.extend(self.region_markers.iter().map(|&x| (x, "region")));
            all_markers.extend(self.peak_markers.iter().map(|&x| (x, "peak")));
            all_markers.extend(self.background_markers.iter().map(|&x| (x, "background")));

            if let Some(&(closest_marker, marker_type)) =
                all_markers.iter().min_by(|(x1, _), (x2, _)| {
                    let dist1 = (cursor_pos.x - x1).abs();
                    let dist2 = (cursor_pos.x - x2).abs();
                    dist1.partial_cmp(&dist2).unwrap()
                })
            {
                match marker_type {
                    "region" => Self::delete_marker(&mut self.region_markers, closest_marker),
                    "peak" => Self::delete_marker(&mut self.peak_markers, closest_marker),
                    "background" => {
                        Self::delete_marker(&mut self.background_markers, closest_marker)
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn remove_peak_markers_outside_region(&mut self) {
        self.peak_markers.retain(|&peak| {
            self.region_markers.first().copied().unwrap_or(0.0) <= peak
                && peak <= self.region_markers.get(1).copied().unwrap_or(0.0)
        });
    }

    pub fn draw_all_markers(&self, plot_ui: &mut PlotUi) {
        Self::draw_markers(&self.peak_markers, Color32::from_rgb(255, 0, 255), plot_ui);
        Self::draw_markers(&self.background_markers, Color32::GREEN, plot_ui);
        Self::draw_markers(&self.region_markers, Color32::BLUE, plot_ui);
    }

    pub fn interactive_markers(&mut self, ui: &mut egui::Ui) {
        if let Some(cursor_position) = self.cursor_position {
            if ui.input(|i| i.key_pressed(egui::Key::P)) {
                Self::add_marker(&mut self.peak_markers, cursor_position.x);
            }

            if ui.input(|i| i.key_pressed(egui::Key::B)) {
                Self::add_marker(&mut self.background_markers, cursor_position.x);
            }

            if ui.input(|i| i.key_pressed(egui::Key::R)) {
                if self.region_markers.len() > 1 {
                    Self::clear_markers(&mut self.region_markers);
                }
                Self::add_marker(&mut self.region_markers, cursor_position.x);
            }

            if ui.input(|i| i.key_pressed(egui::Key::Minus)) {
                self.delete_closest_marker();
            }

            if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                Self::clear_markers(&mut self.region_markers);
                Self::clear_markers(&mut self.peak_markers);
                Self::clear_markers(&mut self.background_markers);
            }
        }
    }

    pub fn context_menu_marker_interactions(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Markers", |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.manual_marker_position)
                            .speed(1.0)
                            .prefix("Marker Position: "),
                    );

                    ui.horizontal(|ui| {
                        if ui.button("Peak").clicked() {
                            Self::add_marker(&mut self.peak_markers, self.manual_marker_position);
                        }

                        ui.separator();

                        if ui.button("Background").clicked() {
                            Self::add_marker(
                                &mut self.background_markers,
                                self.manual_marker_position,
                            );
                        }

                        ui.separator();

                        if ui.button("Region").clicked() {
                            if self.region_markers.len() > 1 {
                                Self::clear_markers(&mut self.region_markers);
                            }
                            Self::add_marker(&mut self.region_markers, self.manual_marker_position);
                        }
                    });

                    ui.separator();

                    if ui.button("Clear all markers").clicked() {
                        Self::clear_markers(&mut self.region_markers);
                        Self::clear_markers(&mut self.peak_markers);
                        Self::clear_markers(&mut self.background_markers);
                    }

                    ui.separator();

                    ui.label("Region Markers");
                    for &x in &self.region_markers {
                        ui.label(format!("{:.2}", x));
                    }

                    ui.label("Peak Markers");
                    for &x in &self.peak_markers {
                        ui.label(format!("{:.2}", x));
                    }

                    ui.label("Background Markers");
                    for &x in &self.background_markers {
                        ui.label(format!("{:.2}", x));
                    }
                });
            });
        });
    }
}
