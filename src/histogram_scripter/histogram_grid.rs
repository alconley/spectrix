use crate::histoer::histogrammer::Histogrammer;

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct GridConfig {
    pub name: String,
    pub histogram_names: Vec<String>,
}

impl GridConfig {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Container Name:");
                ui.text_edit_singleline(&mut self.name);
            });

            ui.separator();

            let mut to_remove: Option<usize> = None;

            egui::ScrollArea::horizontal()
                .id_source(format!("{}-histogram_names-horizontal_scroll", self.name))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (index, name) in &mut self.histogram_names.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(name.clone());

                                // Remove button
                                if ui.button("X").clicked() {
                                    to_remove = Some(index);
                                }
                            });
                        }

                        if let Some(index) = to_remove {
                            self.histogram_names.remove(index);
                        }
                    });
                });
        });
    }

    pub fn insert_grid_into_histogrammer(&self, histogrammer: &mut Histogrammer) {
        let names = self
            .histogram_names
            .iter()
            .map(|name| name.as_str())
            .collect::<Vec<&str>>();

        let panes = histogrammer.get_panes(names);

        histogrammer.tabs.insert(self.name.to_string(), panes);
    }
}
