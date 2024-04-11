use egui::{Color32, Stroke};
use egui_plot::{PlotPoint, PlotUi, VLine};

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct EguiFitMarkers {
    pub region_markers: Vec<f64>,
    pub peak_markers: Vec<f64>,
    pub background_markers: Vec<f64>,

    #[serde(skip)]
    pub cursor_position: Option<PlotPoint>,
}

impl EguiFitMarkers {
    /* region functions */
    pub fn add_region_marker(&mut self, x: f64) {
        self.region_markers.push(x);
    }

    pub fn sort_region_markers(&mut self) {
        self.region_markers
            .sort_by(|a, b| a.partial_cmp(b).unwrap());
    }

    pub fn clear_region_markers(&mut self) {
        self.region_markers.clear();
    }

    pub fn draw_region_markers(&mut self, plot_ui: &mut PlotUi) {
        for x in &self.region_markers {
            let color = Color32::BLUE;
            let line = VLine::new(*x).color(color).stroke(Stroke::new(1.0, color));

            plot_ui.vline(line);
        }
    }

    /* peak functions */
    pub fn add_peak_marker(&mut self, x: f64) {
        self.peak_markers.push(x);
    }

    pub fn sort_peak_markers(&mut self) {
        self.peak_markers.sort_by(|a, b| a.partial_cmp(b).unwrap());
    }

    pub fn clear_peak_markers(&mut self) {
        self.peak_markers.clear();
    }

    pub fn draw_peak_markers(&mut self, plot_ui: &mut PlotUi) {
        for x in &self.peak_markers {
            let color = Color32::from_rgb(255, 0, 255);
            let line = VLine::new(*x).color(color).stroke(Stroke::new(1.0, color));

            plot_ui.vline(line);
        }
    }

    pub fn remove_peak_markers_outside_region(&mut self) {
        let mut new_peak_markers: Vec<f64> = Vec::new();
        for &peak in &self.peak_markers {
            if self.region_markers[0] <= peak && peak <= self.region_markers[1] {
                new_peak_markers.push(peak);
            }
        }
        self.peak_markers = new_peak_markers;
    }

    /* background functions */
    pub fn add_background_marker(&mut self, x: f64) {
        self.background_markers.push(x);
    }

    pub fn sort_background_markers(&mut self) {
        self.background_markers
            .sort_by(|a, b| a.partial_cmp(b).unwrap());
    }

    pub fn clear_background_markers(&mut self) {
        self.background_markers.clear();
    }

    pub fn draw_background_markers(&mut self, plot_ui: &mut PlotUi) {
        for x in &self.background_markers {
            let color = Color32::GREEN;
            let line = VLine::new(*x).color(color).stroke(Stroke::new(1.0, color));

            plot_ui.vline(line);
        }
    }

    /* more general functions */
    pub fn delete_closest_marker(&mut self) {
        if let Some(cursor_pos) = self.cursor_position {
            // Combine all markers into one vector with labels to identify their types
            let mut all_markers: Vec<(f64, String)> = Vec::new();
            all_markers.extend(
                self.region_markers
                    .iter()
                    .map(|&x| (x, "region".to_string())),
            );
            all_markers.extend(self.peak_markers.iter().map(|&x| (x, "peak".to_string())));
            all_markers.extend(
                self.background_markers
                    .iter()
                    .map(|&x| (x, "background".to_string())),
            );

            // Find the closest marker to the cursor position
            if let Some((closest_marker, marker_type)) =
                all_markers.iter().min_by(|(x1, _), (x2, _)| {
                    let dist1 = (cursor_pos.x - x1).abs();
                    let dist2 = (cursor_pos.x - x2).abs();
                    dist1.partial_cmp(&dist2).unwrap()
                })
            {
                // Separate the decision-making process from mutation
                let marker_type = marker_type.clone();
                let closest_marker = *closest_marker;
                // Now perform the deletion
                match marker_type.as_str() {
                    "region" => self.delete_marker_from_region(closest_marker),
                    "peak" => self.delete_marker_from_peak(closest_marker),
                    "background" => self.delete_marker_from_background(closest_marker),
                    _ => {}
                }
            }
        }
    }

    // Separate deletion methods for each marker type to avoid mutable borrowing conflicts
    fn delete_marker_from_region(&mut self, marker_to_delete: f64) {
        if let Some(index) = self
            .region_markers
            .iter()
            .position(|&x| x == marker_to_delete)
        {
            self.region_markers.remove(index);
        }
    }

    fn delete_marker_from_peak(&mut self, marker_to_delete: f64) {
        if let Some(index) = self
            .peak_markers
            .iter()
            .position(|&x| x == marker_to_delete)
        {
            self.peak_markers.remove(index);
        }
    }

    fn delete_marker_from_background(&mut self, marker_to_delete: f64) {
        if let Some(index) = self
            .background_markers
            .iter()
            .position(|&x| x == marker_to_delete)
        {
            self.background_markers.remove(index);
        }
    }

    pub fn draw_markers(&mut self, plot_ui: &mut PlotUi) {
        self.draw_peak_markers(plot_ui);
        self.draw_background_markers(plot_ui);
        self.draw_region_markers(plot_ui);
    }

    pub fn interactive_markers(&mut self, ui: &mut egui::Ui) {
        if let Some(cursor_position) = self.cursor_position {
            if ui.input(|i| i.key_pressed(egui::Key::P)) {
                self.add_peak_marker(cursor_position.x);
                self.sort_peak_markers();
            }

            if ui.input(|i| i.key_pressed(egui::Key::B)) {
                self.add_background_marker(cursor_position.x);
                self.sort_background_markers();
            }

            if ui.input(|i| i.key_pressed(egui::Key::R)) {
                // there can only be 2 region markers
                if self.region_markers.len() > 1 {
                    self.clear_region_markers();
                }
                self.add_region_marker(cursor_position.x);
                self.sort_region_markers();
            }

            // if ui.input(|i| i.key_pressed(egui::Key::Minus)) {
            //     self.delete_closest_marker();
            // }
        }
    }
}
