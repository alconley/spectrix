use eframe::App;

#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
pub struct MUCApp {
    window: bool,
}

impl MUCApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, window: bool) -> Self {
        Self { window }
    }
}

impl App for MUCApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if self.window {
            egui::Window::new("muc")
                .min_width(200.0)
                .max_width(600.0)
                .show(ctx, |ui| {
                    ui.label(
                        "muc is not supported in the browser yet. Please run it natively.",
                    );
                });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label(
                    "muc is not supported in the browser yet. Please run it natively.",
                );
            });
        }
    }
}
