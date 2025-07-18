use find_peaks::Peak;
use find_peaks::PeakFinder;

use super::histogram1d::Histogram;

impl Histogram {
    // Add a function to find peaks
    pub fn find_peaks(&mut self) {
        // // Clear the peak markers
        // self.plot_settings.markers.clear_peak_markers();

        // let region_marker_positions = self.plot_settings.markers.get_region_marker_positions();
        // let mut background_marker_positions =
        //     self.plot_settings.markers.get_background_marker_positions();

        // // Sort background markers
        // background_marker_positions.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // let mut peaks_found_with_background = false;
        // let mut peaks_found_with_region = false;

        // // Determine x_data and y_data before borrowing temp_fit as mutable
        // let (x_data, y_data) = if background_marker_positions.len() >= 2 {
        //     self.fit_background();

        //     // if there are region markers, use the data between them
        //     let (start_x, end_x) = if region_marker_positions.len() == 2 {
        //         peaks_found_with_region = true;
        //         (region_marker_positions[0], region_marker_positions[1])
        //     } else {
        //         peaks_found_with_background = true;
        //         (
        //             background_marker_positions[0],
        //             background_marker_positions[background_marker_positions.len() - 1],
        //         )
        //     };

        //     // Retrieve x_data and y_data without holding a mutable reference
        //     let x_data = self.get_bin_centers_between(start_x, end_x);
        //     let y_data = self.get_bin_counts_between(start_x, end_x);
        //     (Some(x_data), Some(y_data))
        // } else if region_marker_positions.len() == 2 {
        //     let (start_x, end_x) = (region_marker_positions[0], region_marker_positions[1]);
        //     peaks_found_with_region = true;
        //     let counts = self.get_bin_counts_between(start_x, end_x);
        //     (None, Some(counts))
        // } else {
        //     (
        //         None,
        //         Some(self.bins.iter().map(|&count| count as f64).collect()),
        //     )
        // };

        // // If x_data and y_data were retrieved, perform background subtraction
        // let y_data = if let (Some(x_data), Some(y_data)) = (&x_data, &y_data) {
        //     if let Some(temp_fit) = &mut self.fits.temp_fit {
        //         temp_fit.subtract_background(x_data.clone(), y_data.clone())
        //     } else {
        //         log::error!("Failed to fit background");
        //         return;
        //     }
        // } else {
        //     y_data.clone() // Clone to avoid moving y_data
        // };

        // if let Some(y_data) = y_data {
        //     let peaks = self.plot_settings.find_peaks_settings.find_peaks(y_data);
        //     // Add peak markers at detected peaks
        //     for peak in &peaks {
        //         let peak_position = peak.middle_position();
        //         log::info!("Peak at position: {}", peak_position);
        //         // Adjust peak position relative to the first background marker
        //         if peaks_found_with_background {
        //             let adjusted_peak_position =
        //                 self.bin_width * peak_position as f64 + background_marker_positions[0];
        //             self.plot_settings
        //                 .markers
        //                 .add_peak_marker(adjusted_peak_position);
        //         } else if peaks_found_with_region {
        //             let adjusted_peak_position =
        //                 self.bin_width * peak_position as f64 + region_marker_positions[0];
        //             self.plot_settings
        //                 .markers
        //                 .add_peak_marker(adjusted_peak_position);
        //         } else {
        //             let adjusted_peak_position =
        //                 self.bin_width * peak_position as f64 + self.range.0;
        //             self.plot_settings
        //                 .markers
        //                 .add_peak_marker(adjusted_peak_position);
        //         }
        //     }
        // }
    }
}

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
        Self {
            min_height: 20.0,
            max_height: 0.0,
            min_prominence: 1.0,
            max_prominence: 0.0,
            min_difference: 1.0,
            max_difference: 1.0,
            min_plateau_size: 1,
            max_plateau_size: 1,
            min_distance: 5,
            max_distance: 1,

            enable_min_height: true,
            enable_max_height: false,
            enable_min_prominence: true,
            enable_max_prominence: false,
            enable_min_difference: false,
            enable_max_difference: false,
            enable_min_plateau_size: false,
            enable_max_plateau_size: false,
            enable_min_distance: true,
            enable_max_distance: false,
        }
    }
}

impl PeakFindingSettings {
    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Peak Finder Parameters", |ui| {
            ui.heading("Peak Finder Settings");

            if ui.button("Reset").clicked() {
                *self = Self::default();
            }

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_min_height, "Enable Min Height");
                    if self.enable_min_height {
                        ui.add(
                            egui::DragValue::new(&mut self.min_height)
                                .speed(1.0)
                                .range(0.0..=f32::INFINITY),
                        );
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_max_height, "Enable Max Height");
                    if self.enable_max_height {
                        ui.add(
                            egui::DragValue::new(&mut self.max_height)
                                .speed(1.0)
                                .range(0.0..=f32::INFINITY),
                        );
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_min_prominence, "Enable Min Prominence");
                    if self.enable_min_prominence {
                        ui.add(
                            egui::DragValue::new(&mut self.min_prominence)
                                .speed(1.0)
                                .range(0.0..=f32::INFINITY),
                        );
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_max_prominence, "Enable Max Prominence");
                    if self.enable_max_prominence {
                        ui.add(
                            egui::DragValue::new(&mut self.max_prominence)
                                .speed(1.0)
                                .range(0.0..=f32::INFINITY),
                        );
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_min_difference, "Enable Min Difference");
                    if self.enable_min_difference {
                        ui.add(
                            egui::DragValue::new(&mut self.min_difference)
                                .speed(1.0)
                                .range(0.0..=f32::INFINITY),
                        );
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_max_difference, "Enable Max Difference");
                    if self.enable_max_difference {
                        ui.add(
                            egui::DragValue::new(&mut self.max_difference)
                                .speed(1.0)
                                .range(0.0..=f32::INFINITY),
                        );
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_min_plateau_size, "Enable Min Plateau Size");
                    if self.enable_min_plateau_size {
                        ui.add(
                            egui::DragValue::new(&mut self.min_plateau_size)
                                .speed(1.0)
                                .range(0.0..=f32::INFINITY),
                        );
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_max_plateau_size, "Enable Max Plateau Size");
                    if self.enable_max_plateau_size {
                        ui.add(
                            egui::DragValue::new(&mut self.max_plateau_size)
                                .speed(1.0)
                                .range(0.0..=f32::INFINITY),
                        );
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_min_distance, "Enable Min Distance");
                    if self.enable_min_distance {
                        ui.add(
                            egui::DragValue::new(&mut self.min_distance)
                                .speed(1.0)
                                .range(0.0..=f32::INFINITY),
                        );
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.enable_max_distance, "Enable Max Distance");
                    if self.enable_max_distance {
                        ui.add(
                            egui::DragValue::new(&mut self.max_distance)
                                .speed(1.0)
                                .range(0.0..=f32::INFINITY),
                        );
                    }
                });
            });
        });
    }

    pub fn find_peaks(&self, y_data: &[f64]) -> Vec<Peak<f64>> {
        let mut peak_finder = PeakFinder::new(y_data);

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
