use egui_plot::PlotUi;

use egui::Color32;

use rfd::FileDialog;

use std::fs::File;
use std::io::{self, Read};

use super::egui_markers::EguiFitMarkers;

use super::fit::Fit;
use crate::histoer::histogram1d::Histogram;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct FitHandler {
    pub histogram: Option<Histogram>,
    pub fits: Vec<Fit>,
    pub current_fit: Option<Fit>,
    pub markers: EguiFitMarkers,
    pub show_fit_stats: bool,
    to_remove_index: Option<usize>,
}

impl FitHandler {
    pub fn interactive_keybinds(&mut self, ui: &mut egui::Ui) {
        // remove the closest marker to the cursor and the fit
        if ui.input(|i| i.key_pressed(egui::Key::Minus)) {
            if let Some(fit) = &mut self.current_fit {
                fit.clear();
            }

            self.markers.delete_closest_marker();
        }

        // function for adding markers
        // Peak markers are added with the 'P' key
        // Background markers are added with the 'B' key
        // Region markers are added with the 'R' key
        self.markers.interactive_markers(ui);

        // fit the histogram with the 'F' key
        if ui.input(|i| i.key_pressed(egui::Key::F)) {
            self.fit();
        }

        // store the fit with the 'S' key
        if ui.input(|i| i.key_pressed(egui::Key::S)) {
            self.store_fit();
        }

        // clear all markers and fits with the 'Backspace' key
        if ui.input(|i| i.key_pressed(egui::Key::Backspace)) {
            self.clear_all();
        }

        // buttons that will be displayed in the ui
        ui.horizontal(|ui| {
            // check to see if there is at least 2 region markers
            if self.markers.region_markers.len() == 2
                && ui
                    .button("Fit")
                    .on_hover_text("Fit the current histogram data. Shortcut: 'F' key")
                    .clicked()
            {
                self.fit();
            }

            if self.current_fit.is_some() {
                if ui
                    .button("Store fit")
                    .on_hover_text("Store the current fit for comparison. Shortcut: 'S' key")
                    .clicked()
                {
                    self.store_fit();
                }

                ui.separator();
            }

            /*
            ui.label("Clear Markers: ").on_hover_text("The closest marker to the cursor can be removed using the '-' key");
            if ui.button("Peak").on_hover_text("Clear peak markers").clicked() {
                self.current_fit = None;
                self.markers.clear_peak_markers();
            }

            if ui.button("Background").on_hover_text("Clear background markers").clicked() {
                self.current_fit = None;
                self.markers.clear_background_markers();
            }

            if ui.button("Region").on_hover_text("Clear region markers").clicked() {
                self.current_fit = None;
                self.markers.clear_region_markers();
            }

            if ui.button("Clear all").on_hover_text("Clear all fits and markers. Shortcut: 'Backspace' key").clicked() {
                self.clear_all();
            }

            ui.separator();

            */

            ui.checkbox(&mut self.show_fit_stats, "Show Fit Stats");
        });

        if self.show_fit_stats {
            ui.separator();

            // Ensure there's a horizontal scroll area to contain both stats sections side by side
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Current Fits
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Current Fit");

                            if ui.button("Load Fit").clicked() {
                                if let Err(e) = self.load_temp_fit() {
                                    eprintln!("Failed to save fit: {}", e);
                                }
                            }

                            if let Some(fit) = &mut self.current_fit {
                                if ui.button("Save Fit").clicked() {
                                    if let Err(e) = fit.save_fit_to_file() {
                                        eprintln!("Failed to save fit: {}", e);
                                    }
                                }
                            }
                        });

                        if let Some(fit) = &mut self.current_fit {
                            fit.fit_ui(ui);
                        } else {
                            ui.label("No fit available");
                        }
                    });

                    ui.separator();

                    // Stored Fits
                    ui.vertical(|ui| {
                        // Display stored fits stats in a vertical layout within the second column

                        ui.horizontal(|ui| {
                            ui.label("Stored Fits");

                            // Add a button for loading the FitHandler state
                            if ui.button("Load Fits").clicked() {
                                match Self::load_fits_from_file() {
                                    Ok(fit_handler) => *self = fit_handler,
                                    Err(e) => eprintln!("Failed to load Fit Handler state: {}", e),
                                }
                            }

                            // Conditionally show the "Save Fit Handler State" button
                            if !self.fits.is_empty() && ui.button("Save Fits").clicked() {
                                if let Err(e) = self.save_fits_to_file() {
                                    eprintln!("Failed to save Fit Handler state: {}", e);
                                }
                            }
                        });

                        self.stored_fit_stats_labels(ui);
                    });
                });
            });
        }
    }

    pub fn draw_fits(&mut self, plot_ui: &mut PlotUi) {
        // draw the current fit
        if let Some(fit) = &mut self.current_fit {
            fit.draw(plot_ui, Color32::BLUE, Color32::from_rgb(255, 0, 255));
        }

        // draw the stored fits
        for fit in &mut self.fits {
            let color = Color32::from_rgb(162, 0, 255);
            fit.draw(plot_ui, color, color);
        }
    }

    fn new_fit(&mut self, histogram: Histogram) {
        let mut fit = Fit::new(histogram, self.markers.clone());

        if let Err(e) = fit.fit_gaussian() {
            eprintln!("Failed to fit gaussian: {}", e);
        }

        self.markers = fit.markers.clone(); // update the makers with the fit markers
        self.current_fit = Some(fit);
    }

    fn fit(&mut self) {
        if let Some(histogram) = self.histogram.clone() {
            self.new_fit(histogram);
        } else {
            eprintln!("No histogram selected for fitting.");
        }
    }

    fn stored_fit_stats_labels(&mut self, ui: &mut egui::Ui) {
        if !self.fits.is_empty() {
            egui::ScrollArea::vertical()
                .id_source("stored_fit_scroll")
                .show(ui, |ui| {
                    egui::Grid::new("stored_fit_stats_grid")
                        .striped(true)
                        .show(ui, |ui| {
                            // Headers
                            ui.label("Fit Index");
                            ui.label("Mean");
                            ui.label("FWHM");
                            ui.label("Area");
                            ui.end_row(); // End the header row

                            // Iterate over stored fits to fill in the grid with fit statistics

                            for (fit_index, fit) in self.fits.iter().enumerate() {
                                // Assuming each fit has a similar structure to current_fit
                                // and contains fit parameters to display
                                if let Some(gaussian_fitter) = &fit.fit {
                                    if let Some(params) = &gaussian_fitter.fit_params {
                                        // Display stats for each parameter set within the fit
                                        for (param_index, param) in params.iter().enumerate() {
                                            ui.label(format!("{}-{}", fit_index, param_index)); // Fit and parameter index
                                            ui.label(format!(
                                                "{:.2} ± {:.2}",
                                                param.mean.0, param.mean.1
                                            )); // Mean
                                            ui.label(format!(
                                                "{:.2} ± {:.2}",
                                                param.fwhm.0, param.fwhm.1
                                            )); // FWHM
                                            ui.label(format!(
                                                "{:.0} ± {:.0}",
                                                param.area.0, param.area.1
                                            )); // Area

                                            if param_index == 0 && ui.button("X").clicked() {
                                                self.to_remove_index = Some(fit_index);
                                                // Mark for removal
                                            }

                                            ui.end_row(); // Move to the next row for the next set of stats
                                        }
                                    }
                                }

                                // if ui.button(format!("Remove Fit #{}", fit_index)).clicked() {
                                //     self.to_remove_index = Some(fit_index); // Mark for removal
                                // }
                            }
                        });
                });

            if let Some(index) = self.to_remove_index {
                self.remove_fit_at_index(index);
                self.to_remove_index = None;
            }
        }
    }

    fn clear_all(&mut self) {
        self.current_fit = None;
        self.markers.clear_background_markers();
        self.markers.clear_peak_markers();
        self.markers.clear_region_markers();
    }

    fn store_fit(&mut self) {
        if let Some(fit) = self.current_fit.take() {
            self.fits.push(fit);
        }
    }

    fn remove_fit_at_index(&mut self, index: usize) {
        if index < self.fits.len() {
            self.fits.remove(index);
        }
    }

    fn save_fits_to_file(&self) -> Result<(), io::Error> {
        if let Some(path) = FileDialog::new()
            .set_title("Save Fit Handler State")
            .add_filter("YAML files", &["yaml"])
            .save_file()
        {
            let file = File::create(path)?;
            serde_yaml::to_writer(file, &self)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        }
        Ok(())
    }

    fn load_fits_from_file() -> Result<Self, io::Error> {
        if let Some(path) = FileDialog::new()
            .set_title("Load Fit Handler State")
            .add_filter("YAML files", &["yaml"])
            .pick_file()
        {
            let mut file = File::open(path)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            let fit_handler: Self = serde_yaml::from_str(&contents)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            return Ok(fit_handler);
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Failed to load Fit Handler state",
        ))
    }

    fn load_temp_fit(&mut self) -> Result<(), io::Error> {
        // Attempt to load a Fit using the static method defined in Fit
        if let Ok(fit) = Fit::load_fit_from_file() {
            // If successful, update the current_fit with the loaded fit
            self.current_fit = Some(fit);
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Failed to load fit",
            ))
        }
    }
}
