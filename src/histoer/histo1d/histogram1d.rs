use super::plot_settings::PlotSettings;
use crate::egui_plot_stuff::egui_line::EguiLine;
use crate::fitter::fit_handler::Fits;
use egui_extras::{Column, TableBuilder};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Histogram {
    pub name: String,
    pub bins: Vec<u64>,
    pub range: (f64, f64),
    pub overflow: u64,
    pub underflow: u64,
    pub bin_width: f64,
    pub line: EguiLine,
    pub plot_settings: PlotSettings,
    pub fits: Fits,
    pub original_bins: Vec<u64>,
}

impl Histogram {
    // Create a new Histogram with specified min, max, and number of bins
    pub fn new(name: &str, number_of_bins: usize, range: (f64, f64)) -> Self {
        Histogram {
            name: name.to_string(),
            bins: vec![0; number_of_bins],
            range,
            overflow: 0,
            underflow: 0,
            bin_width: (range.1 - range.0) / number_of_bins as f64,
            line: EguiLine {
                name: name.to_string(),
                ..Default::default()
            },
            plot_settings: PlotSettings::default(),
            fits: Fits::new(),
            original_bins: vec![0; number_of_bins],
        }
    }

    pub fn reset(&mut self) {
        self.bins = vec![0; self.original_bins.len()];
        self.original_bins = vec![0; self.original_bins.len()];
        self.plot_settings.rebin_factor = 1;
        self.bin_width = (self.range.1 - self.range.0) / self.bins.len() as f64;
        self.overflow = 0;
        self.underflow = 0;
    }

    pub fn update_line_points(&mut self) {
        self.line.points = self
            .bins
            .iter()
            .enumerate()
            .flat_map(|(index, &count)| {
                let start = self.range.0 + index as f64 * self.bin_width;
                let end = start + self.bin_width;
                let y_value = count as f64;
                vec![[start, y_value], [end, y_value]]
            })
            .collect();
    }

    pub fn draw(&mut self, plot_ui: &mut egui_plot::PlotUi<'_>) {
        // update the histogram and fit lines with the log setting and draw
        let log_y = self.plot_settings.egui_settings.log_y;
        let log_x = self.plot_settings.egui_settings.log_x;

        self.line.log_y = log_y;
        self.line.log_x = log_x;
        self.line.draw(plot_ui);

        self.fits.set_log(log_y, log_x);
        self.fits.draw(plot_ui);

        self.show_stats(plot_ui);

        self.plot_settings.markers.draw_all_markers(plot_ui);
        self.update_background_pair_lines();
        for bg_pair in &mut self.plot_settings.markers.background_markers {
            bg_pair.histogram_line.log_x = log_x;
            bg_pair.histogram_line.log_y = log_y;
        }

        // Check if markers are being dragged
        if self.plot_settings.markers.is_dragging() {
            // Disable dragging if a marker is being dragged
            self.plot_settings.egui_settings.allow_drag = false;
        } else {
            self.plot_settings.egui_settings.allow_drag = true;
        }

        if plot_ui.response().hovered() {
            self.plot_settings.cursor_position = plot_ui.pointer_coordinate();
            self.plot_settings.egui_settings.limit_scrolling = true;
        } else {
            self.plot_settings.cursor_position = None;
        }

        self.custom_plot_manipulation_update(plot_ui);

        // self.plot_settings.egui_settings.y_label = format!("Counts/{:.}", self.bin_width);
    }

    pub fn draw_other_histograms(
        &mut self,
        plot_ui: &mut egui_plot::PlotUi<'_>,
        histograms: &[Histogram],
    ) {
        for histogram in histograms {
            let mut hist = histogram.clone();
            hist.draw(plot_ui);
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        if ui.visuals().dark_mode {
            self.line.set_color(egui::Color32::LIGHT_BLUE);
        } else {
            self.line.set_color(egui::Color32::BLACK);
        }

        self.update_line_points();
        self.keybinds(ui);

        let height = ui.available_height();
        let mut width = ui.available_width();

        let show_stats = self.fits.settings.show_fit_stats;

        // Dynamically build table with 1 or 2 columns
        let mut table = TableBuilder::new(ui);

        if show_stats {
            table = table.column(Column::auto().resizable(true));
        }

        table = table.column(Column::remainder()).vscroll(false);

        table.body(|mut body| {
            body.row(height, |mut row| {
                if show_stats {
                    row.col(|ui| {
                        self.fits.ui(ui, true);
                        width -= ui.available_width(); // assign the difference back to `width`
                    });
                }

                row.col(|ui| {
                    let mut plot = egui_plot::Plot::new(self.name.clone()).width(width);
                    plot = self.plot_settings.egui_settings.apply_to_plot(plot);

                    let (scroll, _pointer_down, _modifiers) = ui.input(|i| {
                        let scroll = i.events.iter().find_map(|e| match e {
                            egui::Event::MouseWheel { delta, .. } => Some(*delta),
                            _ => None,
                        });
                        (scroll, i.pointer.primary_down(), i.modifiers)
                    });

                    let plot_response = plot.show(ui, |plot_ui| {
                        self.draw(plot_ui);

                        if self.plot_settings.progress.is_some() {
                            let y_max = self.bins.iter().max().cloned().unwrap_or(0) as f64;
                            let mut plot_bounds = plot_ui.plot_bounds();
                            plot_bounds.extend_with_y(y_max * 1.1);
                            plot_ui.set_plot_bounds(plot_bounds);
                        }

                        if self.plot_settings.egui_settings.reset_axis {
                            plot_ui.auto_bounds();
                            self.plot_settings.egui_settings.reset_axis = false;
                        }

                        if self.plot_settings.cursor_position.is_some() {
                            if let Some(delta_pos) = scroll {
                                let zoom_factor = if delta_pos.y > 0.0 || delta_pos.x > 0.0 {
                                    1.1
                                } else {
                                    0.9
                                };
                                plot_ui
                                    .zoom_bounds_around_hovered(egui::Vec2::new(zoom_factor, 1.0));
                            }
                        }
                    });

                    plot_response.response.context_menu(|ui| {
                        self.context_menu(ui);
                    });

                    self.plot_settings.interactive_response(&plot_response);
                });
            });
        });
    }

    // pub fn render(&mut self, ui: &mut egui::Ui) {
    //     // if light mode, se the color to black
    //     if ui.visuals().dark_mode {
    //         self.line.set_color(egui::Color32::LIGHT_BLUE);
    //     } else {
    //         self.line.set_color(egui::Color32::BLACK);
    //     }

    //     self.update_line_points(); // Ensure line points are updated for projections
    //     self.keybinds(ui); // Handle interactive elements

    //     let height = ui.available_height();

    //     // egui_extras column
    //     TableBuilder::new(ui)
    //         .column(Column::auto().resizable(true))
    //         .column(Column::remainder())
    //         .body(|mut body| {
    //             body.row(height, |mut row| {
    //                 // Left column: fits/settings

    //                 row.col(|ui| {
    //                     self.fits.ui(ui, self.fits.settings.show_fit_stats);
    //                 });

    //                 // Right column: plot
    //                 row.col(|ui| {
    //                     let mut plot = egui_plot::Plot::new(self.name.clone());

    //                     plot = self.plot_settings.egui_settings.apply_to_plot(plot);

    //                     let (scroll, _pointer_down, _modifiers) = ui.input(|i| {
    //                         let scroll = i.events.iter().find_map(|e| match e {
    //                             egui::Event::MouseWheel {
    //                                 unit: _,
    //                                 delta,
    //                                 modifiers: _,
    //                             } => Some(*delta),
    //                             _ => None,
    //                         });
    //                         (scroll, i.pointer.primary_down(), i.modifiers)
    //                     });

    //                     let plot_response = plot.show(ui, |plot_ui| {
    //                         self.draw(plot_ui);

    //                         if self.plot_settings.progress.is_some() {
    //                             let y_max = self.bins.iter().max().cloned().unwrap_or(0) as f64;
    //                             let mut plot_bounds = plot_ui.plot_bounds();
    //                             plot_bounds.extend_with_y(y_max * 1.1);
    //                             plot_ui.set_plot_bounds(plot_bounds);
    //                         }

    //                         if self.plot_settings.egui_settings.reset_axis {
    //                             plot_ui.auto_bounds();
    //                             self.plot_settings.egui_settings.reset_axis = false;
    //                         }

    //                         if self.plot_settings.cursor_position.is_some() {
    //                             if let Some(delta_pos) = scroll {
    //                                 if delta_pos.y > 0.0 {
    //                                     plot_ui.zoom_bounds_around_hovered(egui::Vec2::new(1.1, 1.0));
    //                                 } else if delta_pos.y < 0.0 {
    //                                     plot_ui.zoom_bounds_around_hovered(egui::Vec2::new(0.9, 1.0));
    //                                 } else if delta_pos.x > 0.0 {
    //                                     plot_ui.zoom_bounds_around_hovered(egui::Vec2::new(1.1, 1.0));
    //                                 } else if delta_pos.x < 0.0 {
    //                                     plot_ui.zoom_bounds_around_hovered(egui::Vec2::new(0.9, 1.0));
    //                                 }
    //                             }
    //                         }
    //                     });

    //                     plot_response.response.context_menu(|ui| {
    //                         self.context_menu(ui);
    //                     });

    //                     self.plot_settings.interactive_response(&plot_response);

    //                 });
    //             })
    //         });
    // }
}
