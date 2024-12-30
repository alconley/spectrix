use super::histogram2d::Histogram2D;

impl Histogram2D {
    // Handles the interactive elements of the histogram
    pub fn keybinds(&mut self, ui: &mut egui::Ui) {
        if let Some(_cursor_position) = self.plot_settings.cursor_position {
            if ui.input(|i| i.key_pressed(egui::Key::I)) {
                self.plot_settings.stats_info = !self.plot_settings.stats_info;
            }

            if ui.input(|i| i.key_pressed(egui::Key::C)) {
                self.new_cut();
            };

            if ui.input(|i| i.key_pressed(egui::Key::X)) {
                self.plot_settings.projections.add_x_projection =
                    !self.plot_settings.projections.add_x_projection;
            }

            if ui.input(|i| i.key_pressed(egui::Key::Y)) {
                self.plot_settings.projections.add_y_projection =
                    !self.plot_settings.projections.add_y_projection;
            }

            if ui.input(|i| i.key_pressed(egui::Key::Z)) {
                self.plot_settings.colormap_options.log_norm =
                    !self.plot_settings.colormap_options.log_norm;
                self.plot_settings.recalculate_image = true;
            }

            if ui.input(|i| i.key_pressed(egui::Key::R)) {
                self.plot_settings.colormap_options.reverse =
                    !self.plot_settings.colormap_options.reverse;
                self.plot_settings.recalculate_image = true;
            }

            if ui.input(|i| i.key_pressed(egui::Key::M)) {
                self.plot_settings.colormap.next_colormap();
                self.plot_settings.recalculate_image = true;
            }
        }
    }
}
