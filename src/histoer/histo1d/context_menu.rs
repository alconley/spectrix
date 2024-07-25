use super::histogram1d::Histogram;

impl Histogram {
    // Handles the context menu for the histogram
    pub fn context_menu(&mut self, ui: &mut egui::Ui) {
        self.line.menu_button(ui);
        self.plot_settings.settings_ui(ui);
        self.keybinds_ui(ui);

        self.fits.fit_context_menu_ui(ui);

        // Add find peaks button
        ui.separator();
        ui.heading("Peak Finder");
        if ui.button("Detect Peaks")
            .on_hover_text("Takes the settings (adjust below) and finds peaks in the spectrum\nIf there are background markers, it will fit a background before it finds the peaks in between the min and max values. Likewise for region markers.\nKeybind: o").clicked() {
            self.find_peaks();
        }
        self.plot_settings.find_peaks_settings.menu_button(ui);

        ui.separator();
        ui.heading("Rebin");

        let possible_factors = self.possible_rebin_factors();

        ui.label("Rebin Factor:");

        ui.horizontal_wrapped(|ui| {
            for &factor in &possible_factors {
                if ui
                    .selectable_label(
                        self.plot_settings.rebin_factor == factor,
                        format!("{}", factor),
                    )
                    .clicked()
                {
                    self.plot_settings.rebin_factor = factor;
                    self.rebin();
                }
            }
        });
    }
}
