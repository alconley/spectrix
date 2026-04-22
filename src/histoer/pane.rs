use crate::histoer::histo1d::histogram1d::Histogram;
use crate::histoer::histo2d::histogram2d::Histogram2D;
use crate::histoer::histogrammer::histogram_calculation_active;
use std::sync::{Arc, Mutex};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Pane {
    Histogram(Arc<Mutex<Box<Histogram>>>),
    Histogram2D(Arc<Mutex<Box<Histogram2D>>>),
}

impl Pane {
    pub fn ui(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        let calculation_active = histogram_calculation_active();
        let hist_name = match self {
            Self::Histogram(hist) => hist
                .try_lock()
                .map(|hist| hist.name.clone())
                .unwrap_or_else(|_| {
                    cached_histogram_name(ui, histogram_snapshot_base_id(hist))
                        .unwrap_or_else(|| "Histogram (updating)".to_owned())
                }),
            Self::Histogram2D(hist) => hist
                .try_lock()
                .map(|hist| hist.name.clone())
                .unwrap_or_else(|_| {
                    cached_histogram_name(ui, histogram2d_snapshot_base_id(hist))
                        .unwrap_or_else(|| "2D Histogram (updating)".to_owned())
                }),
        };

        let button = egui::Button::new(hist_name)
            .min_size(egui::Vec2::new(ui.available_width(), 0.0))
            .small()
            .frame(false);

        let drag_started = ui.add(button.sense(egui::Sense::drag())).drag_started();

        match self {
            Self::Histogram(hist) => {
                if let Ok(mut hist_guard) = hist.try_lock() {
                    hist_guard.render(ui);

                    if calculation_active
                        || !has_histogram_snapshot(ui, histogram_snapshot_base_id(hist))
                    {
                        cache_histogram_snapshot(ui, histogram_snapshot_base_id(hist), &hist_guard);
                    }
                } else {
                    render_cached_histogram(ui, histogram_snapshot_base_id(hist));
                }
            }
            Self::Histogram2D(hist) => {
                if let Ok(mut hist_guard) = hist.try_lock() {
                    hist_guard.render(ui);

                    if calculation_active
                        || !has_histogram2d_snapshot(ui, histogram2d_snapshot_base_id(hist))
                    {
                        cache_histogram2d_snapshot(
                            ui,
                            histogram2d_snapshot_base_id(hist),
                            &hist_guard,
                        );
                    }
                } else {
                    render_cached_histogram2d(ui, histogram2d_snapshot_base_id(hist));
                }
            }
        }

        if drag_started {
            egui_tiles::UiResponse::DragStarted
        } else {
            egui_tiles::UiResponse::None
        }
    }
}

fn histogram_snapshot_base_id(hist: &Arc<Mutex<Box<Histogram>>>) -> egui::Id {
    egui::Id::new(("histogram-pane-snapshot", Arc::as_ptr(hist) as usize))
}

fn histogram2d_snapshot_base_id(hist: &Arc<Mutex<Box<Histogram2D>>>) -> egui::Id {
    egui::Id::new(("histogram2d-pane-snapshot", Arc::as_ptr(hist) as usize))
}

fn cached_histogram_name(ui: &egui::Ui, base_id: egui::Id) -> Option<String> {
    ui.data(|data| data.get_temp::<String>(base_id.with("name")))
}

fn has_histogram_snapshot(ui: &egui::Ui, base_id: egui::Id) -> bool {
    ui.data(|data| data.get_temp::<String>(base_id.with("name")))
        .is_some()
}

fn has_histogram2d_snapshot(ui: &egui::Ui, base_id: egui::Id) -> bool {
    ui.data(|data| data.get_temp::<String>(base_id.with("name")))
        .is_some()
}

fn cache_histogram_snapshot(ui: &egui::Ui, base_id: egui::Id, hist: &Histogram) {
    let snapshot = hist.clone();
    let name = snapshot.name.clone();
    ui.data_mut(|data| {
        data.insert_temp(base_id.with("snapshot"), snapshot);
        data.insert_temp(base_id.with("name"), name);
    });
}

fn cache_histogram2d_snapshot(ui: &egui::Ui, base_id: egui::Id, hist: &Histogram2D) {
    let snapshot = hist.clone();
    let name = snapshot.name.clone();
    ui.data_mut(|data| {
        data.insert_temp(base_id.with("snapshot"), snapshot);
        data.insert_temp(base_id.with("name"), name);
    });
}

fn render_cached_histogram(ui: &mut egui::Ui, base_id: egui::Id) {
    if let Some(mut snapshot) = ui.data(|data| data.get_temp::<Histogram>(base_id.with("snapshot")))
    {
        snapshot.render(ui);
    }
}

fn render_cached_histogram2d(ui: &mut egui::Ui, base_id: egui::Id) {
    if let Some(mut snapshot) =
        ui.data(|data| data.get_temp::<Histogram2D>(base_id.with("snapshot")))
    {
        snapshot.render(ui);
    }
}
