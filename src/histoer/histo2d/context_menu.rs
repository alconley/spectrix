use super::histogram2d::Histogram2D;
use crate::histoer::cuts::Cut2D;

impl Histogram2D {
    // Context menu for the plot (when you right-click on the plot)
    pub fn context_menu(&mut self, ui: &mut egui::Ui) {
        self.image.menu_button(ui);
        self.plot_settings.settings_ui(ui, self.bins.max_count);

        ui.horizontal(|ui| {
            ui.heading("Cuts");

            if ui.button("+").clicked() {
                self.new_cut();
            }
        });

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

        let mut to_remove = None;

        for (index, cut) in self.plot_settings.cuts.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                if ui.button("X").clicked() {
                    to_remove = Some(index);
                }

                ui.separator();

                cut.ui(ui);
            });
        }

        if let Some(index) = to_remove {
            self.plot_settings.cuts.remove(index);
        }

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
    }

    pub fn new_cut(&mut self) {
        for cut in &mut self.plot_settings.cuts {
            cut.polygon.interactive_clicking = false;
            cut.polygon.interactive_dragging = false;
        }

        let mut cut = Cut2D {
            x_column: self.plot_settings.x_column.clone(),
            y_column: self.plot_settings.y_column.clone(),
            ..Default::default()
        };
        cut.polygon.name = format!("Cut {}", self.plot_settings.cuts.len());

        cut.polygon.interactive_clicking = true;
        self.plot_settings.cuts.push(cut);
    }
}
