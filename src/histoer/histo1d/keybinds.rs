use super::histogram1d::Histogram;

fn consume_plot_key(ui: &mut egui::Ui, key: egui::Key) -> bool {
    ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key))
}

impl Histogram {
    // Handles the interactive elements of the histogram
    pub fn keybinds(&mut self, ui: &mut egui::Ui) {
        self.plot_settings.markers.cursor_position = self.plot_settings.cursor_position;

        if let Some(cursor_position) = self.plot_settings.cursor_position {
            let cursor_x_raw = self.display_x_to_raw_x(cursor_position.x);
            let add_peak = consume_plot_key(ui, egui::Key::P);
            let add_background = consume_plot_key(ui, egui::Key::B);
            let add_cut = consume_plot_key(ui, egui::Key::C);
            let add_region = consume_plot_key(ui, egui::Key::R);
            let remove_closest = consume_plot_key(ui, egui::Key::Minus);
            let remove_all = consume_plot_key(ui, egui::Key::Delete);
            let fit_background = consume_plot_key(ui, egui::Key::G);
            let fit_gaussians = consume_plot_key(ui, egui::Key::F);
            let store_fit = consume_plot_key(ui, egui::Key::S);
            let toggle_stats = consume_plot_key(ui, egui::Key::I);
            let toggle_log_y = consume_plot_key(ui, egui::Key::L);
            let toggle_auto_y = consume_plot_key(ui, egui::Key::Y);
            let detect_peaks = consume_plot_key(ui, egui::Key::O);

            if add_peak {
                self.plot_settings.markers.add_peak_marker(cursor_x_raw);
            }

            if add_background {
                self.plot_settings
                    .markers
                    .add_background_pair(cursor_x_raw, self.bin_width);
            }

            if add_cut {
                self.new_cut();
            }

            if add_region {
                if self.plot_settings.markers.region_markers.len() >= 2 {
                    self.plot_settings.markers.clear_region_markers();
                }
                self.plot_settings.markers.add_region_marker(cursor_x_raw);
            }

            if remove_closest {
                self.plot_settings
                    .markers
                    .delete_closest_marker(cursor_x_raw);
            }

            if remove_all {
                self.plot_settings.markers.clear_background_markers();
                self.plot_settings.markers.clear_peak_markers();
                self.plot_settings.markers.clear_region_markers();
            }

            if remove_closest || remove_all {
                self.fits.remove_temp_fits();
            }

            if fit_background {
                self.fit_background();
            }

            if fit_gaussians {
                self.fit_gaussians();
            }

            if store_fit {
                self.fits.store_temp_fit();
            }

            if toggle_stats {
                self.plot_settings.stats_info = !self.plot_settings.stats_info;
            }

            if toggle_log_y {
                self.plot_settings.egui_settings.log_y = !self.plot_settings.egui_settings.log_y;
            }

            if toggle_auto_y {
                self.plot_settings.auto_fit_y_to_visible_range =
                    !self.plot_settings.auto_fit_y_to_visible_range;
            }

            if detect_peaks {
                self.find_peaks();
            }
        }
    }

    // create a ui function to show the keybinds in the context menu
    pub fn keybinds_ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
        .id_salt("keybinds_scroll")
        .max_height(300.0)
        .show(ui, |ui| {
            ui.heading("Keybinds");
            ui.separator();
            ui.label("Markers");
            ui.label("P: Add Marker");
            ui.label("B: Add Background Marker");
            ui.label("R: Add Region Marker");
            ui.label("-: Remove Marker Closest to Cursor");
            ui.label("Delete: Remove All Markers & Temp Fits");
            ui.label("Left click/Drag to Move Marker").on_hover_text("Markers can be dragged to new positions with the left clicking and dragingong when hovered over center point");
            ui.separator();
            ui.label("Cuts");
            ui.label("C: Add 1D Cut");
            ui.label("Left click/Drag cut lines or the region between them")
                .on_hover_text("The cut span can be moved by dragging between the vertical lines while keeping the width fixed.");
            ui.separator();
            ui.label("Fitting");
            ui.label("G: Fit Background").on_hover_text("Fit a linear background using the background markers");
            ui.label("F: Fit Gaussians").on_hover_text("Fit gaussians at the peak markers give some region with a linear background");
            ui.label("S: Store Fit").on_hover_text("Store the current fit as a permanent fit which can be saved and loaded later");
            ui.separator();
            ui.label("Peak Finder");
            ui.label("O: Detect Peaks").on_hover_text("Populate editable peak markers with the original peak finder. This never starts a fit.");
            ui.separator();
            ui.label("Plot");
            ui.label("I: Toggle Stats");
            ui.label("L: Toggle Log Y");
            ui.label("Y: Toggle Auto Fit Y");

        });
    }
}
