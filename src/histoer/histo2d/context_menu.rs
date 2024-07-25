use super::histogram2d::Histogram2D;

impl Histogram2D {
    // Context menu for the plot (when you right-click on the plot)
    pub fn context_menu(&mut self, ui: &mut egui::Ui) {
        self.image.menu_button(ui);
        self.plot_settings.settings_ui(ui, self.bins.max_count);

        ui.separator();

        ui.heading("Rebin");

        let possible_x_factors = self.possible_x_rebin_factors();
        let possible_y_factors = self.possible_y_rebin_factors();

        ui.label("Rebin Factor");

        ui.horizontal_wrapped(|ui| {
            ui.label("X: ");
            for &factor in &possible_x_factors {
                if ui
                    .selectable_label(
                        self.plot_settings.rebin_x_factor == factor,
                        format!("{}", factor),
                    )
                    .clicked()
                {
                    self.plot_settings.rebin_x_factor = factor;
                    self.rebin();
                }
            }
        });

        ui.horizontal_wrapped(|ui| {
            ui.label("Y: ");
            for &factor in &possible_y_factors {
                if ui
                    .selectable_label(
                        self.plot_settings.rebin_y_factor == factor,
                        format!("{}", factor),
                    )
                    .clicked()
                {
                    self.plot_settings.rebin_y_factor = factor;
                    self.rebin();
                }
            }
        });
    }
}
