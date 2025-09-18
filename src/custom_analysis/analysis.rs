use crate::histoer::histogrammer;

use super::se_sps::SPSAnalysis;
use egui::viewport::{ViewportBuilder, ViewportClass, ViewportId};
use std::path::PathBuf;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct AnalysisScripts {
    pub open: bool,
    pub se_sps: SPSAnalysis,
    #[serde(skip)]
    vp_id: Option<ViewportId>,
}

impl AnalysisScripts {
    /// Call this every frame from your app. Set `self.open = true` to (re)open the window.
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        files: &[(PathBuf, bool)],
        histogrammer: &mut histogrammer::Histogrammer,
    ) {
        if !self.open {
            return;
        }

        let id = *self
            .vp_id
            .get_or_insert_with(|| ViewportId::from_hash_of("spectrix.analysis.se_sps"));

        let builder = ViewportBuilder::default()
            .with_title("Analysis")
            .with_inner_size([1100.0, 700.0])
            .with_app_id("Analysis Window");

        let mut requested_close = false;
        ctx.show_viewport_immediate(id, builder, |ctx, _class: ViewportClass| {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.se_sps.ui(ui, files, histogrammer);
            });

            if ctx.input(|i| i.viewport().close_requested()) {
                requested_close = true;
            }
        });

        if requested_close {
            self.open = false;
            self.vp_id = None;
        }
    }
}
