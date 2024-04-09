use eframe::App;

#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
pub struct EVBApp {}

impl EVBApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {}
    }
}

impl App for EVBApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("SPS Eventbuilder is not supported in the browser. Please run it natively.");
        });
    }
}
