use std::cmp::Ordering;
use std::hash::Hash;

pub fn precise_drag_value<'a>(value: &'a mut f64) -> egui::DragValue<'a> {
    egui::DragValue::new(value).max_decimals(15)
}

fn compare_numeric_chunks(left: &str, right: &str) -> Ordering {
    let left_trimmed = left.trim_start_matches('0');
    let right_trimmed = right.trim_start_matches('0');

    let left_significant = if left_trimmed.is_empty() {
        "0"
    } else {
        left_trimmed
    };
    let right_significant = if right_trimmed.is_empty() {
        "0"
    } else {
        right_trimmed
    };

    left_significant
        .len()
        .cmp(&right_significant.len())
        .then_with(|| left_significant.cmp(right_significant))
        .then_with(|| left.len().cmp(&right.len()))
}

fn next_chunk(value: &str, start: usize) -> (&str, usize, bool) {
    if start >= value.len() {
        return ("", start, false);
    }

    let remaining = &value[start..];
    let mut end = start;
    let first_is_digit = remaining
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit());

    for character in remaining.chars() {
        if character.is_ascii_digit() != first_is_digit {
            break;
        }
        end += character.len_utf8();
    }

    (&value[start..end], end, first_is_digit)
}

fn natural_sort_cmp(left: &str, right: &str) -> Ordering {
    let mut left_index = 0usize;
    let mut right_index = 0usize;

    while left_index < left.len() && right_index < right.len() {
        let (left_chunk, next_left_index, left_is_digit) = next_chunk(left, left_index);
        let (right_chunk, next_right_index, right_is_digit) = next_chunk(right, right_index);

        let ordering = match (left_is_digit, right_is_digit) {
            (true, true) => compare_numeric_chunks(left_chunk, right_chunk),
            _ => left_chunk
                .to_ascii_lowercase()
                .cmp(&right_chunk.to_ascii_lowercase())
                .then_with(|| left_chunk.cmp(right_chunk)),
        };

        if ordering != Ordering::Equal {
            return ordering;
        }

        left_index = next_left_index;
        right_index = next_right_index;
    }

    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn natural_sorted_matches(available_columns: &[String], normalized_query: &str) -> Vec<String> {
    let mut matches = available_columns
        .iter()
        .filter(|column_name| column_name_matches_query(column_name, normalized_query))
        .cloned()
        .collect::<Vec<_>>();

    matches.sort_by(|left, right| natural_sort_cmp(left, right));
    matches
}

fn wildcard_match(pattern: &str, candidate: &str) -> bool {
    let pattern = pattern.as_bytes();
    let candidate = candidate.as_bytes();
    let mut pattern_index = 0usize;
    let mut candidate_index = 0usize;
    let mut star_index = None;
    let mut star_candidate_index = 0usize;

    while candidate_index < candidate.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?'
                || pattern[pattern_index] == candidate[candidate_index])
        {
            pattern_index += 1;
            candidate_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star_index = Some(pattern_index);
            pattern_index += 1;
            star_candidate_index = candidate_index;
        } else if let Some(saved_star_index) = star_index {
            pattern_index = saved_star_index + 1;
            star_candidate_index += 1;
            candidate_index = star_candidate_index;
        } else {
            return false;
        }
    }

    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }

    pattern_index == pattern.len()
}

fn column_name_matches_query(column_name: &str, normalized_query: &str) -> bool {
    if normalized_query.is_empty() {
        return true;
    }

    let normalized_column_name = column_name.to_lowercase();
    if normalized_query.contains('*') || normalized_query.contains('?') {
        wildcard_match(normalized_query, &normalized_column_name)
    } else {
        normalized_column_name.contains(normalized_query)
    }
}

fn natural_sorted_selected(selected: &[String]) -> Vec<String> {
    let mut selected_columns = selected.to_vec();
    selected_columns.sort_by(|left, right| natural_sort_cmp(left, right));
    selected_columns
}

fn update_multi_selection(selected: &mut Vec<String>, column_name: &str, is_selected: bool) {
    if is_selected {
        if !selected
            .iter()
            .any(|selected_name| selected_name == column_name)
        {
            selected.push(column_name.to_owned());
        }
    } else {
        selected.retain(|selected_name| selected_name != column_name);
    }
}

fn select_all_multi_matches(selected: &mut Vec<String>, column_names: &[String]) -> bool {
    let mut changed = false;

    for column_name in column_names {
        if !selected
            .iter()
            .any(|selected_name| selected_name == column_name)
        {
            selected.push(column_name.clone());
            changed = true;
        }
    }

    changed
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
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
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
                let matches = natural_sorted_matches(available_columns, &normalized_query);

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

pub fn searchable_multi_column_picker_with_width_ui(
    ui: &mut egui::Ui,
    id_source: impl Hash,
    selected: &mut Vec<String>,
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
    let selected_text = match selected.len() {
        0 => hint_text.to_owned(),
        1 => selected[0].clone(),
        2 | 3 => selected.join(", "),
        count => format!("{count} columns selected"),
    };

    let mut changed = false;

    ui.add_enabled_ui(enabled, |ui| {
        egui::ComboBox::from_id_salt(picker_id)
            .selected_text(selected_text)
            .width(popup_width)
            .truncate()
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
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
                let matches = natural_sorted_matches(available_columns, &normalized_query);
                let selected_columns = natural_sorted_selected(selected);
                let all_matches_selected = matches.iter().all(|column_name| {
                    selected
                        .iter()
                        .any(|selected_name| selected_name == column_name)
                });

                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
                        if !selected_columns.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Selected").small().strong());
                                if ui.small_button("Clear All").clicked() {
                                    selected.clear();
                                    changed = true;
                                }
                            });

                            for column_name in selected_columns {
                                let mut is_selected = true;
                                if ui.checkbox(&mut is_selected, &column_name).changed() {
                                    update_multi_selection(selected, &column_name, is_selected);
                                    changed = true;
                                }
                            }

                            ui.separator();
                        }

                        if matches.is_empty() {
                            ui.label("No matching names.");
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Matches").small().strong());
                                if ui
                                    .add_enabled(
                                        !all_matches_selected,
                                        egui::Button::new("Select All").small(),
                                    )
                                    .clicked()
                                {
                                    changed |= select_all_multi_matches(selected, &matches);
                                }
                            });

                            for column_name in matches {
                                let mut is_selected = selected
                                    .iter()
                                    .any(|selected_name| selected_name == &column_name);
                                if ui.checkbox(&mut is_selected, &column_name).changed() {
                                    update_multi_selection(selected, &column_name, is_selected);
                                    changed = true;
                                }
                            }
                        }
                    });

                if !selected.is_empty() {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(format!("{} selected", selected.len()));
                        if ui.button("Clear").clicked() {
                            selected.clear();
                            changed = true;
                            ui.data_mut(|data| data.insert_temp(search_id, String::new()));
                        }
                    });
                }
            });
    });

    changed
}

pub fn searchable_multi_column_picker_ui(
    ui: &mut egui::Ui,
    id_source: impl Hash,
    selected: &mut Vec<String>,
    available_columns: &[String],
    hint_text: &str,
    enabled: bool,
) -> bool {
    searchable_multi_column_picker_with_width_ui(
        ui,
        id_source,
        selected,
        available_columns,
        hint_text,
        enabled,
        SearchableColumnPickerSize::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_sort_orders_numbered_columns_like_humans_expect() {
        let mut values = vec![
            "S1Ring10Energy".to_owned(),
            "S1Ring2Energy".to_owned(),
            "S1Ring1Energy".to_owned(),
            "S1Ring11Energy".to_owned(),
            "S1Ring0Energy".to_owned(),
        ];

        values.sort_by(|left, right| natural_sort_cmp(left, right));

        assert_eq!(
            values,
            vec![
                "S1Ring0Energy",
                "S1Ring1Energy",
                "S1Ring2Energy",
                "S1Ring10Energy",
                "S1Ring11Energy",
            ]
        );
    }

    #[test]
    fn natural_sorted_matches_keeps_numeric_order_after_filtering() {
        let matches = natural_sorted_matches(
            &[
                "S1Ring10Energy".to_owned(),
                "S1Ring2Energy".to_owned(),
                "S1Ring1Energy".to_owned(),
            ],
            "ring",
        );

        assert_eq!(
            matches,
            vec!["S1Ring1Energy", "S1Ring2Energy", "S1Ring10Energy"]
        );
    }

    #[test]
    fn wildcard_filtering_matches_glob_style_patterns() {
        let matches = natural_sorted_matches(
            &[
                "S1Ring10Time".to_owned(),
                "S1Ring2Time".to_owned(),
                "S1Ring1Time".to_owned(),
                "S1BarTime".to_owned(),
            ],
            "s1ring*time",
        );

        assert_eq!(matches, vec!["S1Ring1Time", "S1Ring2Time", "S1Ring10Time"]);
    }

    #[test]
    fn wildcard_question_mark_matches_single_character() {
        let matches = natural_sorted_matches(
            &[
                "S1Ring1Time".to_owned(),
                "S1Ring10Time".to_owned(),
                "S1Ring2Time".to_owned(),
            ],
            "s1ring?time",
        );

        assert_eq!(matches, vec!["S1Ring1Time", "S1Ring2Time"]);
    }
}
