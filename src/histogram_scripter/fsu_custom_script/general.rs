#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Calibration {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub bins: usize,
    pub range: (f64, f64),
    pub active: bool,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            a: 0.0,
            b: 1.0,
            c: 0.0,
            bins: 512,
            range: (0.0, 4096.0),
            active: false,
        }
    }
}

impl Calibration {
    pub fn ui(&mut self, ui: &mut egui::Ui, bins: bool) {
        ui.horizontal(|ui| {
            if self.active {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.a)
                            .speed(0.0001)
                            .prefix("a: "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.b)
                            .speed(0.0001)
                            .prefix("b: "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.c)
                            .speed(0.0001)
                            .prefix("c: "),
                    );
                });

                if bins {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.bins)
                                .speed(1)
                                .prefix("Bins: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.range.0)
                                .speed(1)
                                .prefix("Range: (")
                                .suffix(", "),
                        );
                        ui.add(egui::DragValue::new(&mut self.range.1).speed(1).suffix(")"));
                    });
                }
            }
            ui.checkbox(&mut self.active, "Active");
        });
    }

    pub fn new_column(&self, column: &str, alias: &str) -> (String, String) {
        (
            format!(
                "({})*{}**2 + ({})*{} + ({})",
                self.a, column, self.b, column, self.c
            ),
            alias.to_string(),
        )
    }
}

/*************************** CeBrA Custom Struct ***************************/

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct TimeCut {
    pub mean: f64,
    pub low: f64,
    pub high: f64,
    pub bins: usize,
    pub range: (f64, f64),
    pub no_cut_range: (f64, f64),
    pub no_cut_bins: usize,
    pub active: bool,
}

impl Default for TimeCut {
    fn default() -> Self {
        Self {
            mean: 0.0,
            low: -3000.0,
            high: 3000.0,
            bins: 6400,
            range: (-3200.0, 3200.0),
            no_cut_range: (-3200.0, 3200.0),
            no_cut_bins: 6400,
            active: false,
        }
    }
}

impl TimeCut {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("timecut_grid")
            .striped(true)
            .show(ui, |ui| {
                ui.checkbox(&mut self.active, "Active");
                ui.label("Range");
                ui.label("Bins");

                if self.active {
                    ui.label("Mean");
                    ui.label("Low");
                    ui.label("High");
                }

                ui.end_row();

                if self.active {
                    ui.label("Cut");
                    ui.horizontal(|ui| {
                        ui.add_enabled(
                            self.active,
                            egui::DragValue::new(&mut self.range.0)
                                .speed(1)
                                .prefix("(")
                                .suffix(", "),
                        );
                        ui.add_enabled(
                            self.active,
                            egui::DragValue::new(&mut self.range.1).speed(1).suffix(")"),
                        );
                    });

                    ui.add_enabled(self.active, egui::DragValue::new(&mut self.bins).speed(1));
                    ui.add_enabled(self.active, egui::DragValue::new(&mut self.mean).speed(1));
                    ui.add_enabled(self.active, egui::DragValue::new(&mut self.low).speed(1));
                    ui.add_enabled(self.active, egui::DragValue::new(&mut self.high).speed(1));

                    ui.end_row();
                }

                ui.label("No Cut");

                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.no_cut_range.0)
                            .speed(1)
                            .prefix("(")
                            .suffix(", "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.no_cut_range.1)
                            .speed(1)
                            .suffix(")"),
                    );
                });
                ui.add(egui::DragValue::new(&mut self.no_cut_bins).speed(1));
                ui.end_row();
            });
    }
}
