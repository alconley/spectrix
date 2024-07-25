use crate::histoer::histo1d::histogram1d::Histogram;
use crate::histoer::histo2d::histogram2d::Histogram2D;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Pane {
    Histogram(Box<Histogram>),
    Histogram2D(Box<Histogram2D>),
}

impl Pane {
    pub fn ui(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        let hist_name = match self {
            Pane::Histogram(hist) => hist.name.clone(),
            Pane::Histogram2D(hist) => hist.name.clone(),
        };

        let button = egui::Button::new(hist_name)
            .min_size(egui::Vec2::new(ui.available_width(), 0.0))
            .small()
            .frame(false);

        if ui.add(button.sense(egui::Sense::drag())).drag_started() {
            match self {
                Pane::Histogram(hist) => {
                    hist.render(ui);
                }

                Pane::Histogram2D(hist) => {
                    hist.render(ui);
                }
            }

            egui_tiles::UiResponse::DragStarted
        } else {
            match self {
                Pane::Histogram(hist) => {
                    hist.render(ui);
                }

                Pane::Histogram2D(hist) => {
                    hist.render(ui);
                }
            }

            egui_tiles::UiResponse::None
        }
    }
}
