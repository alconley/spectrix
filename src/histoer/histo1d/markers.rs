use crate::egui_plot_stuff::{egui_line::EguiLine, egui_vertical_line::EguiVerticalLine};
use crate::fitter::common::Calibration;
use egui_plot::{PlotPoint, PlotUi};

use super::histogram1d::Histogram;

impl Histogram {
    pub fn update_background_pair_lines(&mut self) {
        // Extract bin edges and counts **before** modifying anything
        let bin_edges = self.get_bin_edges();
        let bin_counts = self.bins.clone();

        // Extract immutable background marker positions first
        let marker_positions: Vec<(f64, f64)> = self
            .plot_settings
            .markers
            .background_markers
            .iter()
            .map(|bg_pair| (bg_pair.start.x_value, bg_pair.end.x_value))
            .collect();

        // Compute bin indices based on marker positions **before** modifying anything
        let bin_indices: Vec<(usize, usize)> = marker_positions
            .iter()
            .map(|&(start_x, end_x)| {
                let start_bin = self.get_bin_index(start_x).unwrap_or(0);
                let end_bin = self
                    .get_bin_index(end_x)
                    .unwrap_or(self.bins.len().saturating_sub(1));
                (start_bin, end_bin)
            })
            .collect();

        // Now, modify `background_markers` without conflicting borrows
        for (bg_pair, &(start_bin, end_bin)) in self
            .plot_settings
            .markers
            .background_markers
            .iter_mut()
            .zip(bin_indices.iter())
        {
            bg_pair.histogram_line.points.clear(); // Clear previous points

            // Collect the **actual bin edges** and counts in the correct range
            for i in start_bin..=end_bin {
                if i < bin_edges.len() - 1 {
                    // Ensure no out-of-bounds access
                    let x_start = bin_edges[i]; // Start of the bin
                    let x_end = bin_edges[i + 1]; // End of the bin
                    let y = bin_counts[i] as f64; // Bin count

                    // Add both edges of the bin to the histogram line
                    bg_pair.histogram_line.points.push([x_start, y]);
                    bg_pair.histogram_line.points.push([x_end, y]);
                }
            }
        }
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FitMarkers {
    pub region_markers: Vec<EguiVerticalLine>,
    pub peak_markers: Vec<EguiVerticalLine>,
    pub background_markers: Vec<BackgroundPair>,

    #[serde(skip)]
    pub cursor_position: Option<PlotPoint>,

    #[serde(skip)]
    pub manual_marker_position: f64,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackgroundPair {
    pub start: EguiVerticalLine,
    pub end: EguiVerticalLine,
    pub histogram_line: EguiLine,
}

impl BackgroundPair {
    pub fn is_dragging(&self) -> bool {
        self.start.is_dragging || self.end.is_dragging
    }

    pub fn new(start: EguiVerticalLine, end: EguiVerticalLine) -> Self {
        let mut line = EguiLine::new(egui::Color32::from_rgb(0, 200, 0));
        line.name = "Background Pair".to_owned();
        line.reference_fill = true;
        line.fill = 0.0;
        line.width = 0.0;
        line.fill_alpha = 0.05;

        line.points.push([start.x_value, 0.0]);
        line.points.push([end.x_value, 0.0]);

        Self {
            start,
            end,
            histogram_line: line,
        }
    }

    pub fn average_x(&self) -> f64 {
        (self.start.x_value + self.end.x_value) / 2.0
    }

    pub fn draw(&mut self, plot_ui: &mut PlotUi<'_>, calibration: Option<&Calibration>) {
        self.start.draw(plot_ui, calibration);
        self.end.draw(plot_ui, calibration);
        self.histogram_line.draw(plot_ui, calibration);
    }

    pub fn interactive_dragging(
        &mut self,
        plot_response: &egui_plot::PlotResponse<()>,
        calibration: Option<&Calibration>,
    ) {
        self.start.interactive_dragging(plot_response, calibration);
        self.end.interactive_dragging(plot_response, calibration);
    }

    /// Updates the `histogram_line` to match the histogram bins within this background pair
    pub fn update_histogram_line(&mut self, bin_edges: &[f64], bin_counts: &[u32]) {
        let start_x = self.start.x_value;
        let end_x = self.end.x_value;

        let mut line_points = Vec::new();

        for (i, &edge) in bin_edges.iter().enumerate() {
            if edge >= start_x && edge <= end_x {
                let y_value = if i < bin_counts.len() {
                    bin_counts[i] as f64
                } else {
                    0.0
                };
                line_points.push([edge, y_value]);
            }
        }

        // Ensure the last point is included at the end marker
        if let Some(last_edge) = bin_edges.last() {
            if *last_edge <= end_x {
                let last_count = *bin_counts.last().unwrap_or(&0) as f64;
                line_points.push([*last_edge, last_count]);
            }
        }

        self.histogram_line.points = line_points;
    }
}

impl FitMarkers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_dragging(&self) -> bool {
        self.region_markers.iter().any(|m| m.is_dragging)
            || self.peak_markers.iter().any(|m| m.is_dragging)
            || self.background_markers.iter().any(|m| m.is_dragging())
    }

    pub fn add_region_marker(&mut self, x: f64) {
        if self.region_markers.len() >= 2 {
            self.clear_region_markers();
        }

        let mut marker = EguiVerticalLine::new(x, egui::Color32::BLUE);
        marker.width = 0.5;
        marker.name = format!("Region Marker (x={x:.2})");

        self.region_markers.push(marker);

        self.region_markers.sort_by(|a, b| {
            a.x_value
                .partial_cmp(&b.x_value)
                .expect("Region markers should be sortable")
        });
    }

    pub fn add_peak_marker(&mut self, x: f64) {
        let mut marker = EguiVerticalLine::new(x, egui::Color32::from_rgb(225, 0, 255));

        marker.width = 0.5;
        marker.name = format!("Peak Marker (x={x:.2})");

        self.peak_markers.push(marker);
        self.peak_markers.sort_by(|a, b| {
            a.x_value
                .partial_cmp(&b.x_value)
                .expect("Peak markers should be sortable")
        });
    }

    pub fn add_background_pair(&mut self, x: f64, bin_width: f64) {
        let mut marker_start = EguiVerticalLine::new(x, egui::Color32::from_rgb(0, 200, 0));
        let mut marker_end = EguiVerticalLine::new(x, egui::Color32::from_rgb(0, 200, 0));

        marker_start.width = 0.5;
        marker_end.width = 0.5;

        marker_start.name = format!("Background Pair {} Start", self.background_markers.len());
        marker_end.name = format!("Background Pair {} End", self.background_markers.len());

        marker_start.x_value = x;
        marker_end.x_value = x + bin_width;

        let markers = BackgroundPair::new(marker_start, marker_end);
        self.background_markers.push(markers);
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

    pub fn delete_closest_marker(&mut self, cursor_x: f64) {
        let mut all_markers: Vec<(f64, &str)> = vec![];

        all_markers.extend(self.region_markers.iter().map(|x| (x.x_value, "region")));
        all_markers.extend(self.peak_markers.iter().map(|x| (x.x_value, "peak")));
        all_markers.extend(
            self.background_markers
                .iter()
                .map(|x| (x.average_x(), "background")),
        );

        if let Some(&(closest_marker, marker_type)) =
            all_markers.iter().min_by(|(x1, _), (x2, _)| {
                let dist1 = (cursor_x - x1).abs();
                let dist2 = (cursor_x - x2).abs();
                dist1.partial_cmp(&dist2).expect("Comparison failed")
            })
        {
            match marker_type {
                "region" => Self::delete_marker(&mut self.region_markers, closest_marker),
                "peak" => Self::delete_marker(&mut self.peak_markers, closest_marker),
                "background" => {
                    self.background_markers
                        .retain(|x| x.average_x() != closest_marker);
                }
                _ => {}
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

    pub fn get_background_marker_positions(&self) -> Vec<(f64, f64)> {
        // Self::get_marker_positions(&self.background_markers)
        self.background_markers
            .iter()
            .map(|m| (m.start.x_value, m.end.x_value))
            .collect()
    }

    pub fn remove_peak_markers_outside_region(&mut self) {
        self.peak_markers.retain(|peak| {
            self.region_markers
                .first()
                .is_some_and(|start| peak.x_value >= start.x_value)
                && self
                    .region_markers
                    .get(1)
                    .is_some_and(|end| peak.x_value <= end.x_value)
        });
    }

    pub fn draw_all_markers(
        &mut self,
        plot_ui: &mut PlotUi<'_>,
        calibration: Option<&Calibration>,
    ) {
        for marker in &mut self.background_markers {
            marker.draw(plot_ui, calibration);
        }

        for marker in &mut self.region_markers {
            marker.draw(plot_ui, calibration);
        }

        for marker in &mut self.peak_markers {
            marker.draw(plot_ui, calibration);
        }
    }

    pub fn interactive_dragging(
        &mut self,
        plot_response: &egui_plot::PlotResponse<()>,
        calibration: Option<&Calibration>,
    ) {
        for marker in &mut self.background_markers {
            marker.interactive_dragging(plot_response, calibration);
        }

        for marker in &mut self.region_markers {
            marker.interactive_dragging(plot_response, calibration);
        }

        for marker in &mut self.peak_markers {
            marker.interactive_dragging(plot_response, calibration);
        }
    }

    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Markers", |ui| {
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
                        self.add_background_pair(self.manual_marker_position, 1.0);
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

                ui.horizontal(|ui| {
                    ui.label("Clear");

                    if ui.button("All").clicked() {
                        self.clear_background_markers();
                        self.clear_peak_markers();
                        self.clear_region_markers();
                    }

                    if ui.button("Region").clicked() {
                        self.clear_region_markers();
                    }

                    if ui.button("Peaks").clicked() {
                        self.clear_peak_markers();
                    }

                    if ui.button("Background").clicked() {
                        self.clear_background_markers();
                    }
                });
            });

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for marker in &mut self.region_markers {
                    marker.menu_button(ui);
                }

                for marker in &mut self.peak_markers {
                    marker.menu_button(ui);
                }

                for pair in &mut self.background_markers {
                    pair.start.menu_button(ui);
                    pair.end.menu_button(ui);
                    pair.histogram_line.menu_button(ui);
                }
            });
        });
    }
}
