use eframe::App;

#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
pub struct Spectrix {}

impl Spectrix {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl App for Spectrix {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("NAT is not supported in the browser yet. Please run it natively.");
        });
    }
}
