use super::histoer::histogram1d::Histogram;
use super::histoer::histogram2d::Histogram2D;
use crate::workspacer::Workspacer;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Pane {
    Workspace(Workspacer),
    Histogram(Histogram),
    Histogram2D(Histogram2D),
}

impl Pane {
    pub fn ui(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        match self {
            Pane::Workspace(workspace) => {
                workspace.workspace_ui(ui);
            }

            Pane::Histogram(hist) => {
                hist.render(ui);
            }

            Pane::Histogram2D(hist) => {
                hist.render(ui);
            }
        }
        // if ui
        //     .add(egui::Button::new("").sense(egui::Sense::drag()))
        //     .drag_started()
        // {
        //     egui_tiles::UiResponse::DragStarted
        // } else {
        //     egui_tiles::UiResponse::None
        // }
        egui_tiles::UiResponse::None
    }
}
