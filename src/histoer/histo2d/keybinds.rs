use super::histogram2d::Histogram2D;

impl Histogram2D {
    // Handles the interactive elements of the histogram
    pub fn keybinds(&mut self, ui: &mut egui::Ui) {
        if let Some(_cursor_position) = self.plot_settings.cursor_position {
            if ui.input(|i| i.key_pressed(egui::Key::I)) {
                self.plot_settings.stats_info = !self.plot_settings.stats_info;
            }

            if ui.input(|i| i.key_pressed(egui::Key::C)) {
                self.plot_settings.cuts.new_cut();
            }

            if ui.input(|i| i.key_pressed(egui::Key::X)) {
                self.plot_settings.projections.add_x_projection =
                    !self.plot_settings.projections.add_x_projection;
            }

            if ui.input(|i| i.key_pressed(egui::Key::Y)) {
                self.plot_settings.projections.add_y_projection =
                    !self.plot_settings.projections.add_y_projection;
            }
        }
    }
}
