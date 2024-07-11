use find_peaks::Peak;
use find_peaks::PeakFinder;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeakFindingSettings {
    min_height: f64,
    max_height: f64,
    min_prominence: f64,
    max_prominence: f64,
    min_difference: f64,
    max_difference: f64,
    min_plateau_size: usize,
    max_plateau_size: usize,
    min_distance: usize,
    max_distance: usize,

    enable_min_height: bool,
    enable_max_height: bool,
    enable_min_prominence: bool,
    enable_max_prominence: bool,
    enable_min_difference: bool,
    enable_max_difference: bool,
    enable_min_plateau_size: bool,
    enable_max_plateau_size: bool,
    enable_min_distance: bool,
    enable_max_distance: bool,
}

impl Default for PeakFindingSettings {
    fn default() -> Self {
        PeakFindingSettings {
            min_height: 0.0,
            max_height: 0.0,
            min_prominence: 0.0,
            max_prominence: 0.0,
            min_difference: 1.0,
            max_difference: 1.0,
            min_plateau_size: 1,
            max_plateau_size: 1,
            min_distance: 1,
            max_distance: 1,

            enable_min_height: false,
            enable_max_height: false,
            enable_min_prominence: false,
            enable_max_prominence: false,
            enable_min_difference: false,
            enable_max_difference: false,
            enable_min_plateau_size: false,
            enable_max_plateau_size: false,
            enable_min_distance: false,
            enable_max_distance: false,
        }
    }
}

impl PeakFindingSettings {
    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Peak Finder", |ui| {
            ui.heading("Peak Finder Settings");

            if ui.button("Reset").clicked() {
                *self = PeakFindingSettings::default();
            }

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_min_height, "Enable Min Height");
                    if self.enable_min_height {
                        ui.add(egui::DragValue::new(&mut self.min_height).speed(1.0));
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_max_height, "Enable Max Height");
                    if self.enable_max_height {
                        ui.add(egui::DragValue::new(&mut self.max_height).speed(1.0));
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_min_prominence, "Enable Min Prominence");
                    if self.enable_min_prominence {
                        ui.add(egui::DragValue::new(&mut self.min_prominence).speed(1.0));
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_max_prominence, "Enable Max Prominence");
                    if self.enable_max_prominence {
                        ui.add(egui::DragValue::new(&mut self.max_prominence).speed(1.0));
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_min_difference, "Enable Min Difference");
                    if self.enable_min_difference {
                        ui.add(egui::DragValue::new(&mut self.min_difference).speed(1.0));
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_max_difference, "Enable Max Difference");
                    if self.enable_max_difference {
                        ui.add(egui::DragValue::new(&mut self.max_difference).speed(1.0));
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_min_plateau_size, "Enable Min Plateau Size");
                    if self.enable_min_plateau_size {
                        ui.add(egui::DragValue::new(&mut self.min_plateau_size).speed(1.0));
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_max_plateau_size, "Enable Max Plateau Size");
                    if self.enable_max_plateau_size {
                        ui.add(egui::DragValue::new(&mut self.max_plateau_size).speed(1.0));
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_min_distance, "Enable Min Distance");
                    if self.enable_min_distance {
                        ui.add(egui::DragValue::new(&mut self.min_distance).speed(1.0));
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_max_distance, "Enable Max Distance");
                    if self.enable_max_distance {
                        ui.add(egui::DragValue::new(&mut self.max_distance).speed(1.0));
                    }
                });
            });
        });
    }

    pub fn find_peaks(&self, y_data: Vec<f64>) -> Vec<Peak<f64>> {
        let mut peak_finder = PeakFinder::new(&y_data);

        if self.enable_min_height {
            peak_finder.with_min_height(self.min_height);
        }

        if self.enable_max_height {
            peak_finder.with_max_height(self.max_height);
        }

        if self.enable_min_prominence {
            peak_finder.with_min_prominence(self.min_prominence);
        }

        if self.enable_max_prominence {
            peak_finder.with_max_prominence(self.max_prominence);
        }

        if self.enable_min_difference {
            peak_finder.with_min_difference(self.min_difference);
        }

        if self.enable_max_difference {
            peak_finder.with_max_difference(self.max_difference);
        }

        if self.enable_min_plateau_size {
            peak_finder.with_min_plateau_size(self.min_plateau_size);
        }

        if self.enable_max_plateau_size {
            peak_finder.with_max_plateau_size(self.max_plateau_size);
        }

        if self.enable_min_distance {
            peak_finder.with_min_distance(self.min_distance);
        }

        if self.enable_max_distance {
            peak_finder.with_max_distance(self.max_distance);
        }

        peak_finder.find_peaks()
    }
}
