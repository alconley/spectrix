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
        let palette = &self.generation_defaults.cuts.palette;
        palette[self.plot_settings.cuts.len() % palette.len()]
    }

    pub fn new_cut(&mut self) {
        let source_columns = self.plot_settings.cut_source_columns();
        if source_columns.is_empty() {
            log::error!(
                "Cannot add a 1D cut to histogram '{}' because no source columns are available.",
                self.name
            );
            return;
        }

        let visible_range = self.plot_settings.current_plot_bounds.unwrap_or(self.range);

        self.plot_settings
            .cuts
            .push(InteractiveCut1D::new_with_defaults(
                &self.next_cut_name(),
                &source_columns,
                self.range,
                visible_range,
                self.next_cut_color(),
                &self.generation_defaults.cuts,
            ));
    }

    // Handles the context menu for the histogram
    pub fn context_menu(&mut self, ui: &mut egui::Ui) {
        SubMenuButton::new("Line")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                ui.checkbox(&mut self.follow_theme_colors, "Use app theme colors")
                    .on_hover_text(
                        "Use the light/dark colors captured when this histogram was generated.",
                    );
                let previous_color = self.line.color;
                self.line.menu_button(ui);
                if self.line.color != previous_color {
                    self.follow_theme_colors = false;
                }
            });

        SubMenuButton::new("Settings")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                if let Some(raw_x) = self.plot_settings.settings_ui(ui) {
                    self.add_background_marker_at(raw_x);
                }
            });

        SubMenuButton::new("Export")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                if ui
                    .button("Bin Counts [.csv]")
                    .on_hover_text("Export one row per bin: bin_left, bin_right, count.")
                    .clicked()
                {
                    self.export_bin_counts_csv_dialog();
                    ui.close();
                }
            });

        SubMenuButton::new("Cuts")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                let can_add_cut = !self.plot_settings.cut_source_columns().is_empty();

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
                    ui.label("No source columns are available for this histogram.");
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

        let mut manual_guesses_changed = false;
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.fits.settings.show_fit_stats, "Show")
                .on_hover_text("Open the fit panel.");

            SubMenuButton::new("Fits")
                .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
                .ui(ui, |ui| {
                    manual_guesses_changed |= self.fits.fit_context_menu_ui(
                        ui,
                        self.range,
                        &mut self.plot_settings.markers,
                        self.bin_width,
                    );
                });
        });
        if manual_guesses_changed {
            self.invalidate_manual_gaussian_preview();
            self.refresh_manual_peak_guesses();
        }

        SubMenuButton::new("Peak Finder")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                ui.heading("Peak Finder");
                if ui
                    .button("Detect Peaks")
                    .on_hover_text(
                        "Populate editable peak markers using the original detector.\n\
                         Region markers limit the search when both are present.\n\
                         The active fitted background is subtracted when available.\n\
                         Detection never launches a fit. Keybind: O",
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
