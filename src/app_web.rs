use eframe::App;

#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
pub struct MUCApp {}

impl MUCApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl App for MUCApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("muc is not supported in the browser yet. Please run it natively.");
        });
    }
}
