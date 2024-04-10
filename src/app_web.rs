use eframe::App;

#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
pub struct EVBApp {
    window: bool,
}

impl EVBApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, window: bool) -> Self {
        Self { window }
    }
}

impl App for EVBApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if self.window {
            egui::Window::new("SE-SPS Event Builder")
                .min_width(200.0)
                .max_width(600.0)
                .show(ctx, |ui| {
                    ui.label(
                        "SPS Eventbuilder is not supported in the browser. Please run it natively.",
                    );
                });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label(
                    "SPS Eventbuilder is not supported in the browser. Please run it natively.",
                );
            });
        }
    }
}
