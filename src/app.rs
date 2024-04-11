use eframe::egui::{self};
use eframe::App;

use super::processer::Processer;
use super::workspacer::Workspacer;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct MUCApp {
    workspacer: Workspacer,
    processer: Processer,
    window: bool,
}

impl MUCApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, window: bool) -> Self {
        Self {
            workspacer: Workspacer::new(),
            processer: Processer::new(),
            window,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::top("plotter_top_panel").show_inside(ui, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Workspace", |ui| {
                    self.workspacer.workspace_ui(ui);
                });

                if !self.workspacer.selected_files.is_empty() {
                    self.processer.files = self.workspacer.selected_files.clone();
                    self.processer.calculation_ui(ui);
                    // ui.separator();

                    // if ui.button("Calculate histograms").clicked() {
                    //     // add selected files to processer
                    //     self.processer.files = self.workspacer.selected_files.clone();
                    //     info!("Calculating histograms");

                    //     self.processer.calculate_histograms();
                    //     info!("Finished caluclating histograms");
                    // }

                    // ui.separator();
                }
            });
        });

        egui::SidePanel::right("plotter_right_panel").show_inside(ui, |ui| {
            self.processer.select_histograms_ui(ui);
        });

        if self.workspacer.file_selecton {
            egui::SidePanel::left("plotter_left_panel").show_inside(ui, |ui| {
                self.workspacer.file_selection_ui_side_panel(ui);
            });
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.processer.render_histos(ui);
        });
    }
}

impl App for MUCApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if self.window {
            let mut size = ctx.screen_rect().size();
            size.x -= 50.0; // Subtract 50 from the width
            size.y -= 100.0; // Subtract 50 from the height

            egui::Window::new("Plotter").max_size(size).show(ctx, |ui| {
                self.ui(ui);
            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.ui(ui);
            });
        }
    }
}
