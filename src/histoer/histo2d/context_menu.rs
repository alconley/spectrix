use super::histogram2d::Histogram2D;
use super::projections::ProjectionAxisSettings;
use crate::histoer::cuts::Cut2D;

use egui::Color32;
use egui::PopupCloseBehavior;
use egui::containers::menu::{MenuConfig, SubMenuButton};

impl Histogram2D {
    fn next_cut_name(&self, x_column: &str, y_column: &str) -> String {
        let base_name = Cut2D::default_name(x_column, y_column);
        let mut next_index = 1;

        while self
            .plot_settings
            .cuts
            .iter()
            .any(|cut| cut.polygon.name == format!("{base_name} {next_index}"))
        {
            next_index += 1;
        }

        format!("{base_name} {next_index}")
    }

    fn next_cut_color(&self) -> Color32 {
        const DEFAULT_CUT_COLORS: [Color32; 6] = [
            Color32::RED,
            Color32::GREEN,
            Color32::BLUE,
            Color32::YELLOW,
            Color32::from_rgb(255, 0, 255),
            Color32::from_rgb(0, 255, 255),
        ];

        DEFAULT_CUT_COLORS[self.plot_settings.cuts.len() % DEFAULT_CUT_COLORS.len()]
    }

    pub fn context_menu(&mut self, ui: &mut egui::Ui) {
        SubMenuButton::new("Image")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                self.image.menu_button(ui);
            });

        self.plot_settings.settings_ui(
            ui,
            self.bins.max_count,
            ProjectionAxisSettings {
                axis_range: (self.range.x.min, self.range.x.max),
                bin_width: self.bins.x_width,
            },
            ProjectionAxisSettings {
                axis_range: (self.range.y.min, self.range.y.max),
                bin_width: self.bins.y_width,
            },
        );

        SubMenuButton::new("Cuts")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                let cuts_available = self.plot_settings.cuts_available();

                ui.horizontal(|ui| {
                    ui.heading("Cuts");

                    if ui
                        .add_enabled(cuts_available, egui::Button::new("+"))
                        .on_disabled_hover_text(self.plot_settings.cuts_unavailable_reason())
                        .clicked()
                    {
                        self.new_cut();
                    }
                });

                if !cuts_available {
                    ui.label(self.plot_settings.cuts_unavailable_reason());
                }

                ui.add_enabled_ui(cuts_available, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("X: ");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.plot_settings.x_column)
                                .hint_text("X Column Name"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Y: ");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.plot_settings.y_column)
                                .hint_text("Y Column Name"),
                        );
                    });
                });

                let mut to_remove = None;

                for (index, cut) in self.plot_settings.cuts.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.button("X").clicked() {
                            to_remove = Some(index);
                        }

                        ui.separator();

                        ui.add_enabled_ui(cuts_available, |ui| {
                            cut.ui(ui);
                        });
                    });
                }

                if let Some(index) = to_remove {
                    self.plot_settings.cuts.remove(index);
                }
            });

        SubMenuButton::new("Rebin")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
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
                                format!("{factor}"),
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
                                format!("{factor}"),
                            )
                            .clicked()
                        {
                            self.plot_settings.rebin_y_factor = factor;
                            self.rebin();
                        }
                    }
                });
            });
    }

    pub fn new_cut(&mut self) {
        if !self.plot_settings.cuts_available() {
            log::warn!(
                "Cannot add a 2D cut to histogram '{}' because it has multiple source pairs.",
                self.name
            );
            return;
        }

        for cut in &mut self.plot_settings.cuts {
            cut.polygon.interactive_clicking = false;
            cut.polygon.interactive_dragging = false;
        }

        let mut cut = Cut2D {
            x_column: self.plot_settings.x_column.clone(),
            y_column: self.plot_settings.y_column.clone(),
            ..Default::default()
        };
        cut.polygon.name = self.next_cut_name(&cut.x_column, &cut.y_column);
        cut.polygon.set_color(self.next_cut_color());

        cut.polygon.interactive_clicking = true;
        self.plot_settings.cuts.push(cut);
    }
}
