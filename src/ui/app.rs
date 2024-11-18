use crate::util::processer::Processer;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Spectrix {
    processer: Processer,
    left_side_panel_open: bool,
    right_side_panel_open: bool,
}

impl Default for Spectrix {
    fn default() -> Self {
        Self {
            processer: Processer::new(),
            left_side_panel_open: true,
            right_side_panel_open: true,
        }
    }
}

impl Spectrix {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for Spectrix {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle keybinds
        let input = ctx.input(|state| state.clone()); // Get the input state
        if input.key_pressed(egui::Key::Tab) {
            self.left_side_panel_open = !self.left_side_panel_open;
        }
        if input.key_pressed(egui::Key::Tab) {
            self.right_side_panel_open = !self.right_side_panel_open;
        }

        egui::TopBottomPanel::top("spectrix_top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::global_theme_preference_switch(ui);

                ui.heading("Spectrix");

                ui.separator();

                if ui.button("Reset").clicked() {
                    self.processer.reset();
                }
            });
        });

        egui::SidePanel::left("spectrix_left_panel").show_animated(
            ctx,
            self.left_side_panel_open,
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("LeftPanel")
                    .show(ui, |ui| {
                        self.processer.ui(ui);
                    });
            },
        );

        // Secondary left panel for the toggle button
        egui::SidePanel::left("spectrix_toggle_left_panel")
            .resizable(false)
            .show_separator_line(false)
            .min_width(1.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 10.0); // Center the button vertically
                    if ui
                        .small_button(if self.left_side_panel_open {
                            "◀"
                        } else {
                            "▶"
                        })
                        .clicked()
                    {
                        self.left_side_panel_open = !self.left_side_panel_open;
                    }
                });
            });

        egui::SidePanel::right("spectrix_right_panel").show_animated(
            ctx,
            self.right_side_panel_open,
            |ui| {
                self.processer.histogram_script_ui(ui);
            },
        );

        // Secondary left panel for the toggle button
        egui::SidePanel::right("spectrix_toggle_right_panel")
            .resizable(false)
            .show_separator_line(false)
            .min_width(1.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 10.0); // Center the button vertically
                    if ui
                        .small_button(if self.right_side_panel_open {
                            "▶"
                        } else {
                            "◀"
                        })
                        .clicked()
                    {
                        self.right_side_panel_open = !self.right_side_panel_open;
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.processer.histogrammer.ui(ui);
        });
    }
}
