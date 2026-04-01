use super::histogram1d::Histogram;
use super::interactive_cuts::InteractiveCut1D;
use egui::PopupCloseBehavior;
use egui::containers::menu::{MenuConfig, SubMenuButton};

impl Histogram {
    fn next_cut_name(&self) -> String {
        let base_name = format!("{} 1D Cut", self.name);
        let mut next_index = 1;

        while self
            .plot_settings
            .cuts
            .iter()
            .any(|cut| cut.cut.name == format!("{base_name} {next_index}"))
        {
            next_index += 1;
        }

        format!("{base_name} {next_index}")
    }

    fn next_cut_color(&self) -> egui::Color32 {
        const DEFAULT_CUT_COLORS: [egui::Color32; 6] = [
            egui::Color32::RED,
            egui::Color32::GREEN,
            egui::Color32::BLUE,
            egui::Color32::YELLOW,
            egui::Color32::from_rgb(255, 0, 255),
            egui::Color32::from_rgb(0, 255, 255),
        ];

        DEFAULT_CUT_COLORS[self.plot_settings.cuts.len() % DEFAULT_CUT_COLORS.len()]
    }

    pub fn new_cut(&mut self) {
        if self.plot_settings.column_name.trim().is_empty() {
            log::error!(
                "Cannot add a 1D cut to histogram '{}' because no source column is available.",
                self.name
            );
            return;
        }

        let visible_range = self.plot_settings.current_plot_bounds.unwrap_or(self.range);

        self.plot_settings.cuts.push(InteractiveCut1D::new(
            &self.next_cut_name(),
            &self.plot_settings.column_name,
            self.range,
            visible_range,
            self.next_cut_color(),
        ));
    }

    // Handles the context menu for the histogram
    pub fn context_menu(&mut self, ui: &mut egui::Ui) {
        SubMenuButton::new("Line")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                self.line.menu_button(ui);
            });

        SubMenuButton::new("Settings")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                self.plot_settings.settings_ui(ui);
            });

        SubMenuButton::new("Cuts")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                let can_add_cut = !self.plot_settings.column_name.trim().is_empty();

                ui.horizontal(|ui| {
                    ui.heading("Cuts");
                    if ui
                        .add_enabled(can_add_cut, egui::Button::new("+"))
                        .on_disabled_hover_text(
                            "This histogram does not have a source column to build a filter from.",
                        )
                        .clicked()
                    {
                        self.new_cut();
                    }
                });

                if !can_add_cut {
                    ui.label("No source column is available for this histogram.");
                }

                let mut to_remove = None;
                for (index, cut) in self.plot_settings.cuts.iter_mut().enumerate() {
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("X").clicked() {
                            to_remove = Some(index);
                        }

                        cut.menu_ui(ui, self.bin_width);
                    });
                }

                if let Some(index) = to_remove {
                    self.plot_settings.cuts.remove(index);
                }
            });

        SubMenuButton::new("Keybinds Help")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                self.keybinds_ui(ui);
            });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.fits.settings.show_fit_stats, "Show Fit Panel")
                .on_hover_text("Open the fit panel.");
            ui.checkbox(&mut self.fits.settings.fit_panel_popout, "Pop Out")
                .on_hover_text("Open the fit panel in a separate native window when supported.");
        });

        SubMenuButton::new("Fits")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                self.fits.fit_context_menu_ui(ui);
            });

        SubMenuButton::new("Peak Finder")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                ui.heading("Peak Finder");
                if ui
                    .button("Detect Peaks")
                    .on_hover_text(
                        "Find peaks with `find_peaks` using the settings below.\n\
                         If region markers are set, only the data between them is searched.\n\
                         If an active background fit exists, it is subtracted before searching.\n\
                         Keybind: O",
                    )
                    .clicked()
                {
                    self.find_peaks();
                }
                ui.separator();
                self.plot_settings.find_peaks_settings.menu_button(ui);
            });

        SubMenuButton::new("Rebin")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                ui.heading("Rebin");

                let possible_factors = self.possible_rebin_factors();

                ui.label("Rebin Factor:");

                ui.horizontal_wrapped(|ui| {
                    for &factor in &possible_factors {
                        if ui
                            .selectable_label(
                                self.plot_settings.rebin_factor == factor,
                                format!("{factor}"),
                            )
                            .clicked()
                        {
                            self.plot_settings.rebin_factor = factor;
                            self.rebin();
                        }
                    }
                });
            });
    }
}
