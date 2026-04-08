use std::hash::Hash;

pub fn precise_drag_value<'a>(value: &'a mut f64) -> egui::DragValue<'a> {
    egui::DragValue::new(value).max_decimals(15)
}

#[derive(Debug, Clone, Copy)]
pub struct SearchableColumnPickerSize {
    pub closed_width: f32,
    pub open_width: f32,
}

impl Default for SearchableColumnPickerSize {
    fn default() -> Self {
        Self {
            closed_width: 150.0,
            open_width: 360.0,
        }
    }
}

pub fn searchable_column_picker_with_width_ui(
    ui: &mut egui::Ui,
    id_source: impl Hash,
    selected: &mut String,
    available_columns: &[String],
    hint_text: &str,
    enabled: bool,
    size: SearchableColumnPickerSize,
) -> bool {
    let picker_id = ui.make_persistent_id(id_source);
    let search_id = picker_id.with("search");
    let popup_open = egui::ComboBox::is_open(ui.ctx(), picker_id);
    let popup_width = if popup_open {
        size.open_width
    } else {
        size.closed_width
    };
    let selected_text = if selected.trim().is_empty() {
        hint_text.to_owned()
    } else {
        selected.clone()
    };

    let mut changed = false;

    ui.add_enabled_ui(enabled, |ui| {
        egui::ComboBox::from_id_salt(picker_id)
            .selected_text(selected_text)
            .width(popup_width)
            .truncate()
            .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
            .show_ui(ui, |ui| {
                let mut search = ui
                    .data(|data| data.get_temp::<String>(search_id))
                    .unwrap_or_default();

                if ui
                    .add(
                        egui::TextEdit::singleline(&mut search)
                            .hint_text("Search")
                            .desired_width(size.open_width),
                    )
                    .changed()
                {
                    ui.data_mut(|data| data.insert_temp(search_id, search.clone()));
                }

                ui.separator();

                let normalized_query = search.trim().to_lowercase();
                let matches = available_columns
                    .iter()
                    .filter(|column_name| {
                        normalized_query.is_empty()
                            || column_name.to_lowercase().contains(&normalized_query)
                    })
                    .cloned()
                    .collect::<Vec<_>>();

                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
                        if matches.is_empty() {
                            ui.label("No matching names.");
                        } else {
                            for column_name in matches {
                                if ui
                                    .selectable_label(*selected == column_name, &column_name)
                                    .clicked()
                                {
                                    *selected = column_name;
                                    changed = true;
                                    ui.data_mut(|data| data.insert_temp(search_id, String::new()));
                                    ui.close();
                                }
                            }
                        }
                    });

                if !selected.trim().is_empty() && ui.button("Clear").clicked() {
                    selected.clear();
                    changed = true;
                    ui.data_mut(|data| data.insert_temp(search_id, String::new()));
                    ui.close();
                }
            });
    });

    changed
}

pub fn searchable_column_picker_ui(
    ui: &mut egui::Ui,
    id_source: impl Hash,
    selected: &mut String,
    available_columns: &[String],
    hint_text: &str,
    enabled: bool,
) -> bool {
    searchable_column_picker_with_width_ui(
        ui,
        id_source,
        selected,
        available_columns,
        hint_text,
        enabled,
        SearchableColumnPickerSize::default(),
    )
}
