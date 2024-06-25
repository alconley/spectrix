use crate::egui_plot_stuff::egui_vertical_line::EguiVerticalLine;
use egui_plot::{PlotPoint, PlotUi};

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EguiFitMarkers {
    pub region_markers: Vec<EguiVerticalLine>,
    pub peak_markers: Vec<EguiVerticalLine>,
    pub background_markers: Vec<EguiVerticalLine>,

    #[serde(skip)]
    pub cursor_position: Option<PlotPoint>,

    #[serde(skip)]
    pub manual_marker_position: f64,
}

impl EguiFitMarkers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_region_marker(&mut self, x: f64) {
        if self.region_markers.len() >= 2 {
            self.clear_region_markers();
        }

        let mut marker = EguiVerticalLine::new(x, egui::Color32::BLUE);

        marker.name = format!("Region Marker (x={:.2})", x);

        self.region_markers.push(marker);
        self.region_markers
            .sort_by(|a, b| a.x_value.partial_cmp(&b.x_value).unwrap());
    }

    pub fn add_peak_marker(&mut self, x: f64) {
        let mut marker = EguiVerticalLine::new(x, egui::Color32::from_rgb(255, 0, 255));

        marker.name = format!("Peak Marker (x={:.2})", x);

        self.peak_markers.push(marker);
        self.peak_markers
            .sort_by(|a, b| a.x_value.partial_cmp(&b.x_value).unwrap());
    }

    pub fn add_background_marker(&mut self, x: f64) {
        let mut marker = EguiVerticalLine::new(x, egui::Color32::GREEN);

        marker.name = format!("Background Marker (x={:.2})", x);

        self.background_markers.push(marker);
        self.background_markers
            .sort_by(|a, b| a.x_value.partial_cmp(&b.x_value).unwrap());
    }

    pub fn clear_region_markers(&mut self) {
        self.region_markers.clear();
    }

    pub fn clear_peak_markers(&mut self) {
        self.peak_markers.clear();
    }

    pub fn clear_background_markers(&mut self) {
        self.background_markers.clear();
    }

    fn delete_marker(markers: &mut Vec<EguiVerticalLine>, marker_to_delete: f64) {
        if let Some(index) = markers.iter().position(|x| x.x_value == marker_to_delete) {
            markers.remove(index);
        }
    }

    pub fn delete_closest_marker(&mut self) {
        if let Some(cursor_pos) = self.cursor_position {
            let mut all_markers: Vec<(f64, &str)> = vec![];

            all_markers.extend(self.region_markers.iter().map(|x| (x.x_value, "region")));
            all_markers.extend(self.peak_markers.iter().map(|x| (x.x_value, "peak")));
            all_markers.extend(
                self.background_markers
                    .iter()
                    .map(|x| (x.x_value, "background")),
            );

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

    fn get_marker_positions(markers: &[EguiVerticalLine]) -> Vec<f64> {
        markers.iter().map(|m| m.x_value).collect()
    }

    pub fn get_region_marker_positions(&self) -> Vec<f64> {
        Self::get_marker_positions(&self.region_markers)
    }

    pub fn get_peak_marker_positions(&self) -> Vec<f64> {
        Self::get_marker_positions(&self.peak_markers)
    }

    pub fn get_background_marker_positions(&self) -> Vec<f64> {
        Self::get_marker_positions(&self.background_markers)
    }

    pub fn remove_peak_markers_outside_region(&mut self) {
        self.peak_markers.retain(|peak| {
            self.region_markers
                .first()
                .map_or(false, |start| peak.x_value >= start.x_value)
                && self
                    .region_markers
                    .get(1)
                    .map_or(false, |end| peak.x_value <= end.x_value)
        });
    }

    pub fn draw_all_markers(&mut self, plot_ui: &mut PlotUi) {
        for marker in &mut self.background_markers {
            marker.draw(plot_ui);
        }

        for marker in &mut self.region_markers {
            marker.draw(plot_ui);
        }

        for marker in &mut self.peak_markers {
            marker.draw(plot_ui);
        }
    }

    pub fn interactive_markers(&mut self, ui: &mut egui::Ui) {
        if let Some(cursor_position) = self.cursor_position {
            if ui.input(|i| i.key_pressed(egui::Key::P)) {
                self.add_peak_marker(cursor_position.x);
            }

            if ui.input(|i| i.key_pressed(egui::Key::B)) {
                self.add_background_marker(cursor_position.x);
            }

            if ui.input(|i| i.key_pressed(egui::Key::R)) {
                if self.region_markers.len() >= 2 {
                    self.clear_region_markers();
                }
                self.add_region_marker(cursor_position.x);
            }

            if ui.input(|i| i.key_pressed(egui::Key::Minus)) {
                self.delete_closest_marker();
            }

            if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                self.clear_background_markers();
                self.clear_peak_markers();
                self.clear_region_markers();
            }
        }
    }

    pub fn interactive_dragging(&mut self, plot_response: &egui_plot::PlotResponse<()>) {
        for marker in &mut self.background_markers {
            marker.interactive_dragging(plot_response);
        }

        for marker in &mut self.region_markers {
            marker.interactive_dragging(plot_response);
        }

        for marker in &mut self.peak_markers {
            marker.interactive_dragging(plot_response);
        }
    }

    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
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
                            self.add_peak_marker(self.manual_marker_position);
                        }

                        ui.separator();

                        if ui.button("Background").clicked() {
                            self.add_background_marker(self.manual_marker_position);
                        }

                        ui.separator();

                        if ui.button("Region").clicked() {
                            if self.region_markers.len() > 1 {
                                self.clear_region_markers();
                            }
                            self.add_region_marker(self.manual_marker_position);
                        }
                    });

                    ui.separator();

                    if ui.button("Clear all markers").clicked() {
                        self.clear_background_markers();
                        self.clear_peak_markers();
                        self.clear_region_markers();
                    }
                });

                ui.separator();

                for marker in &mut self.region_markers {
                    marker.menu_button(ui);
                }

                for marker in &mut self.peak_markers {
                    marker.menu_button(ui);
                }

                for marker in &mut self.background_markers {
                    marker.menu_button(ui);
                }
            });
        });
    }
}
