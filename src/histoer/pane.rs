use crate::histoer::histo1d::histogram1d::Histogram;
use crate::histoer::histo2d::histogram2d::Histogram2D;
use std::sync::{Arc, Mutex};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Pane {
    Histogram(Arc<Mutex<Box<Histogram>>>),
    Histogram2D(Arc<Mutex<Box<Histogram2D>>>),
}

impl Pane {
    pub fn ui(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        let hist_name = match self {
            Self::Histogram(hist) => hist.lock().expect("Failed to lock histogram").name.clone(),
            Self::Histogram2D(hist) => hist
                .lock()
                .expect("Failed to lock 2D histogram")
                .name
                .clone(),
        };

        let button = egui::Button::new(hist_name)
            .min_size(egui::Vec2::new(ui.available_width(), 0.0))
            .small()
            .frame(false);

        if ui.add(button.sense(egui::Sense::drag())).drag_started() {
            match self {
                Self::Histogram(hist) => {
                    hist.lock().expect("Failed to lock histogram").render(ui);
                }

                Self::Histogram2D(hist) => {
                    hist.lock().expect("Failed to lock 2D histogram").render(ui);
                }
            }

            egui_tiles::UiResponse::DragStarted
        } else {
            match self {
                Self::Histogram(hist) => {
                    hist.lock().expect("Failed to lock histogram").render(ui);
                }

                Self::Histogram2D(hist) => {
                    hist.lock().expect("Failed to lock 2D histogram").render(ui);
                }
            }

            egui_tiles::UiResponse::None
        }
    }
}
