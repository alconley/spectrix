#[derive(PartialEq, Debug, Copy, Clone, serde::Serialize, serde::Deserialize, Default)]
pub enum ColorMap {
    #[default]
    Viridis,
    Fast,
    SmoothCoolWarm,
    BentCoolWarm,
    Plasma,
    Blackbody,
    Inferno,
    Kindlmann,
    ExtendedKindlmann,
}

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct ColormapOptions {
    log_norm: bool,
    reverse: bool,
    custom_display_range: bool,
    display_min: u64,
    display_max: u64,
}

impl Default for ColormapOptions {
    fn default() -> Self {
        ColormapOptions {
            log_norm: true,
            reverse: false,
            custom_display_range: false,
            display_min: 0,
            display_max: u64::MAX,
        }
    }
}

impl ColormapOptions {
    pub fn ui(&mut self, ui: &mut egui::Ui, recalculate_image: &mut bool, max_z_range: u64) {
        if ui
            .checkbox(&mut self.log_norm, "Log Z")
            .on_hover_text("Log the Z values. 0 bins will become transparent")
            .changed()
        {
            *recalculate_image = true;
        };
        if ui
            .checkbox(&mut self.reverse, "Reverse")
            .on_hover_text("Reverse the color map intensity values")
            .changed()
        {
            *recalculate_image = true;
        };
        if ui
            .checkbox(&mut self.custom_display_range, "Custom Z Range")
            .on_hover_text(
                "Strictly for visualization purposes. Projections will not be updated...",
            )
            .changed()
        {
            *recalculate_image = true;
        };

        if self.custom_display_range {
            ui.horizontal(|ui| {
                ui.label("Z ");
                let min_z_range = if self.log_norm { 1 } else { 0 };
                if ui
                    .add(
                        egui::widgets::DragValue::new(&mut self.display_min)
                            .speed(1)
                            .prefix("Min:")
                            .range(min_z_range..=max_z_range),
                    )
                    .changed()
                {
                    *recalculate_image = true;
                };
                if ui
                    .add(
                        egui::widgets::DragValue::new(&mut self.display_max)
                            .speed(1)
                            .prefix("Max:")
                            .range(min_z_range..=max_z_range),
                    )
                    .changed()
                {
                    *recalculate_image = true;
                };
            });
        }
    }
}

impl ColorMap {
    pub fn color(
        &self,
        count: u64,
        min_count: u64,
        max_count: u64,
        options: ColormapOptions,
    ) -> egui::Color32 {
        match self {
            ColorMap::Viridis => Self::colormap(viridis(), count, min_count, max_count, options),
            ColorMap::Fast => Self::colormap(fast(), count, min_count, max_count, options),
            ColorMap::SmoothCoolWarm => {
                Self::colormap(smooth_cool_warm(), count, min_count, max_count, options)
            }
            ColorMap::BentCoolWarm => {
                Self::colormap(bent_cool_warm(), count, min_count, max_count, options)
            }
            ColorMap::Plasma => Self::colormap(plasma(), count, min_count, max_count, options),
            ColorMap::Blackbody => {
                Self::colormap(blackbody(), count, min_count, max_count, options)
            }
            ColorMap::Inferno => Self::colormap(inferno(), count, min_count, max_count, options),
            ColorMap::Kindlmann => {
                Self::colormap(kindlmann(), count, min_count, max_count, options)
            }
            ColorMap::ExtendedKindlmann => {
                Self::colormap(extended_kindlmann(), count, min_count, max_count, options)
            }
        }
    }

    pub fn color_maps_ui(&mut self, ui: &mut egui::Ui, recalculate_image: &mut bool) {
        // check to see if the colormap has changed
        let new_colormap = *self;

        ui.vertical(|ui| {
            ui.radio_value(self, ColorMap::Viridis, "Viridis");
            ui.radio_value(self, ColorMap::Fast, "Fast");
            ui.radio_value(self, ColorMap::SmoothCoolWarm, "Smooth Cool Warm");
            ui.radio_value(self, ColorMap::BentCoolWarm, "Bent Cool Warm");
            ui.radio_value(self, ColorMap::Plasma, "Plasma");
            ui.radio_value(self, ColorMap::Blackbody, "Blackbody");
            ui.radio_value(self, ColorMap::Inferno, "Inferno");
            ui.radio_value(self, ColorMap::Kindlmann, "Kindlmann");
            ui.radio_value(self, ColorMap::ExtendedKindlmann, "Extended Kindlmann");
        });

        if new_colormap != *self {
            *recalculate_image = true;
        }
    }

    fn colormap(
        color_data: Vec<(f32, i32, i32, i32)>,
        value: u64,
        min: u64,
        max: u64,
        options: ColormapOptions,
    ) -> egui::Color32 {
        if value == 0 && options.log_norm {
            // Return transparent color for zero values
            return egui::Color32::from_rgba_unmultiplied(0, 0, 0, 0);
        }

        // Handle display range options
        let (display_min, display_max) = if options.custom_display_range {
            (options.display_min, options.display_max)
        } else {
            (min, max)
        };

        // Reverse RGB values while keeping scalar values the same
        let color_data = if options.reverse {
            let mut reversed_rgb = color_data
                .iter()
                .map(|&(_s, r, g, b)| (r, g, b))
                .collect::<Vec<_>>();
            reversed_rgb.reverse();
            color_data
                .iter()
                .enumerate()
                .map(|(i, &(s, _, _, _))| {
                    (s, reversed_rgb[i].0, reversed_rgb[i].1, reversed_rgb[i].2)
                })
                .collect::<Vec<_>>()
        } else {
            color_data
        };

        // Convert min and max to f64 for calculations
        let (min_f64, max_f64) = (display_min as f64, display_max as f64);

        // Handle case where min == max to avoid division by zero
        let normalized: f64 = if max_f64 > min_f64 {
            let value_f64 = value as f64;
            if options.log_norm {
                // Use logarithmic scale
                (value_f64.log10() - min_f64.log10()) / (max_f64.log10() - min_f64.log10())
            } else {
                // Use linear scale
                (value_f64 - min_f64) / (max_f64 - min_f64)
            }
        } else {
            0.0
        }
        .clamp(0.0, 1.0);

        // Interpolate between colors in the colormap based on the scale value
        let mut lower = color_data[0];
        let mut upper = color_data[0];
        for color in color_data.iter().skip(1) {
            if color.0 >= normalized as f32 {
                upper = *color;
                break;
            }
            lower = *color;
        }

        let scale = (normalized as f32 - lower.0) / (upper.0 - lower.0);
        let red = (lower.1 as f32 + scale * (upper.1 as f32 - lower.1 as f32)).clamp(0.0, 255.0);
        let green = (lower.2 as f32 + scale * (upper.2 as f32 - lower.2 as f32)).clamp(0.0, 255.0);
        let blue = (lower.3 as f32 + scale * (upper.3 as f32 - lower.3 as f32)).clamp(0.0, 255.0);

        egui::Color32::from_rgb(red as u8, green as u8, blue as u8)
    }
}

// scalar,RGB_r,RGB_g,RGB_b
fn viridis() -> Vec<(f32, i32, i32, i32)> {
    vec![
        (0.0, 68, 1, 84),
        (0.032_258_064, 71, 13, 96),
        (0.064_516_13, 72, 25, 107),
        (0.096_774_19, 72, 36, 116),
        (0.129_032_25, 71, 46, 124),
        (0.161_290_32, 69, 56, 130),
        (0.193_548_38, 65, 66, 134),
        (0.225_806_44, 62, 75, 137),
        (0.258_064_5, 58, 84, 140),
        (0.290_322_57, 54, 93, 141),
        (0.322_580_64, 50, 101, 142),
        (0.354_838_7, 46, 109, 142),
        (0.387_096_76, 43, 117, 142),
        (0.419_354_83, 40, 125, 142),
        (0.451_612_9, 37, 133, 142),
        (0.483_870_95, 34, 140, 141),
        (0.516_129, 32, 148, 140),
        (0.548_387_1, 30, 156, 137),
        (0.580_645_14, 32, 163, 134),
        (0.612_903_24, 37, 171, 130),
        (0.645_161_3, 45, 178, 125),
        (0.677_419_36, 57, 186, 118),
        (0.709_677_4, 72, 193, 110),
        (0.741_935_5, 88, 199, 101),
        (0.774_193_5, 106, 205, 91),
        (0.806_451_6, 126, 211, 79),
        (0.838_709_65, 146, 215, 66),
        (0.870_967_75, 168, 219, 52),
        (0.903_225_8, 190, 223, 38),
        (0.935_483_9, 212, 226, 27),
        (0.967_741_9, 233, 228, 26),
        (1.0, 253, 231, 37),
    ]
}

fn fast() -> Vec<(f32, i32, i32, i32)> {
    vec![
        (0.0, 14, 14, 120),
        (0.032_258_064, 28, 35, 136),
        (0.064_516_13, 38, 54, 152),
        (0.096_774_19, 47, 73, 168),
        (0.129_032_25, 54, 92, 184),
        (0.161_290_32, 60, 111, 201),
        (0.193_548_38, 68, 130, 213),
        (0.225_806_44, 76, 148, 222),
        (0.258_064_5, 83, 166, 232),
        (0.290_322_57, 90, 185, 241),
        (0.322_580_64, 110, 198, 242),
        (0.354_838_7, 132, 210, 240),
        (0.387_096_76, 151, 221, 237),
        (0.419_354_83, 169, 233, 235),
        (0.451_612_9, 193, 238, 223),
        (0.483_870_95, 218, 240, 205),
        (0.516_129, 233, 236, 184),
        (0.548_387_1, 239, 225, 160),
        (0.580_645_14, 243, 215, 136),
        (0.612_903_24, 243, 201, 119),
        (0.645_161_3, 241, 187, 106),
        (0.677_419_36, 239, 172, 92),
        (0.709_677_4, 236, 157, 79),
        (0.741_935_5, 229, 141, 70),
        (0.774_193_5, 221, 126, 61),
        (0.806_451_6, 214, 110, 52),
        (0.838_709_65, 206, 94, 43),
        (0.870_967_75, 196, 80, 39),
        (0.903_225_8, 184, 67, 37),
        (0.935_483_9, 173, 53, 35),
        (0.967_741_9, 162, 38, 33),
        (1.0, 150, 20, 30),
    ]
}

fn smooth_cool_warm() -> Vec<(f32, i32, i32, i32)> {
    vec![
        (0.0, 59, 76, 192),
        (0.032_258_064, 68, 91, 205),
        (0.064_516_13, 78, 105, 216),
        (0.096_774_19, 88, 118, 226),
        (0.129_032_25, 99, 132, 235),
        (0.161_290_32, 110, 144, 242),
        (0.193_548_38, 121, 156, 248),
        (0.225_806_44, 133, 168, 252),
        (0.258_064_5, 144, 178, 254),
        (0.290_322_57, 155, 188, 255),
        (0.322_580_64, 167, 196, 254),
        (0.354_838_7, 178, 204, 251),
        (0.387_096_76, 188, 210, 247),
        (0.419_354_83, 198, 215, 241),
        (0.451_612_9, 208, 218, 234),
        (0.483_870_95, 217, 220, 226),
        (0.516_129, 225, 219, 215),
        (0.548_387_1, 233, 214, 203),
        (0.580_645_14, 239, 207, 190),
        (0.612_903_24, 243, 200, 178),
        (0.645_161_3, 246, 191, 165),
        (0.677_419_36, 247, 180, 152),
        (0.709_677_4, 247, 169, 139),
        (0.741_935_5, 245, 157, 126),
        (0.774_193_5, 242, 144, 113),
        (0.806_451_6, 237, 130, 101),
        (0.838_709_65, 230, 115, 89),
        (0.870_967_75, 223, 99, 78),
        (0.903_225_8, 214, 82, 67),
        (0.935_483_9, 204, 63, 57),
        (0.967_741_9, 192, 41, 47),
        (1.0, 180, 4, 38),
    ]
}

fn bent_cool_warm() -> Vec<(f32, i32, i32, i32)> {
    vec![
        (0.0, 59, 76, 192),
        (0.032_258_064, 67, 88, 198),
        (0.064_516_13, 76, 99, 202),
        (0.096_774_19, 85, 110, 207),
        (0.129_032_25, 95, 121, 211),
        (0.161_290_32, 105, 132, 215),
        (0.193_548_38, 116, 142, 219),
        (0.225_806_44, 127, 153, 222),
        (0.258_064_5, 139, 164, 225),
        (0.290_322_57, 151, 175, 228),
        (0.322_580_64, 164, 185, 230),
        (0.354_838_7, 177, 196, 233),
        (0.387_096_76, 191, 206, 235),
        (0.419_354_83, 205, 217, 237),
        (0.451_612_9, 219, 227, 239),
        (0.483_870_95, 235, 237, 241),
        (0.516_129, 241, 236, 233),
        (0.548_387_1, 238, 224, 215),
        (0.580_645_14, 236, 211, 198),
        (0.612_903_24, 233, 199, 182),
        (0.645_161_3, 229, 186, 166),
        (0.677_419_36, 226, 173, 151),
        (0.709_677_4, 223, 160, 136),
        (0.741_935_5, 219, 147, 122),
        (0.774_193_5, 215, 134, 109),
        (0.806_451_6, 211, 121, 97),
        (0.838_709_65, 206, 107, 85),
        (0.870_967_75, 201, 93, 74),
        (0.903_225_8, 196, 77, 64),
        (0.935_483_9, 191, 61, 54),
        (0.967_741_9, 186, 41, 46),
        (1.0, 180, 4, 38),
    ]
}

fn plasma() -> Vec<(f32, i32, i32, i32)> {
    vec![
        (0.0, 13, 8, 135),
        (0.032_258_064, 34, 6, 144),
        (0.064_516_13, 50, 5, 151),
        (0.096_774_19, 64, 4, 157),
        (0.129_032_25, 78, 2, 162),
        (0.161_290_32, 91, 1, 165),
        (0.193_548_38, 104, 0, 168),
        (0.225_806_44, 117, 1, 168),
        (0.258_064_5, 129, 4, 167),
        (0.290_322_57, 141, 11, 165),
        (0.322_580_64, 152, 20, 160),
        (0.354_838_7, 163, 29, 154),
        (0.387_096_76, 173, 38, 147),
        (0.419_354_83, 182, 48, 139),
        (0.451_612_9, 191, 57, 132),
        (0.483_870_95, 199, 66, 124),
        (0.516_129, 207, 76, 116),
        (0.548_387_1, 214, 85, 109),
        (0.580_645_14, 221, 94, 102),
        (0.612_903_24, 227, 104, 95),
        (0.645_161_3, 233, 114, 88),
        (0.677_419_36, 238, 124, 81),
        (0.709_677_4, 243, 135, 74),
        (0.741_935_5, 247, 146, 67),
        (0.774_193_5, 250, 157, 59),
        (0.806_451_6, 252, 169, 53),
        (0.838_709_65, 253, 181, 46),
        (0.870_967_75, 253, 194, 41),
        (0.903_225_8, 252, 207, 37),
        (0.935_483_9, 249, 221, 36),
        (0.967_741_9, 245, 235, 39),
        (1.0, 240, 249, 33),
    ]
}

fn blackbody() -> Vec<(f32, i32, i32, i32)> {
    vec![
        (0.0, 0, 0, 0),
        (0.032_258_064, 23, 8, 4),
        (0.064_516_13, 35, 15, 9),
        (0.096_774_19, 47, 19, 13),
        (0.129_032_25, 60, 21, 16),
        (0.161_290_32, 73, 24, 19),
        (0.193_548_38, 87, 26, 21),
        (0.225_806_44, 101, 28, 23),
        (0.258_064_5, 115, 30, 25),
        (0.290_322_57, 130, 31, 27),
        (0.322_580_64, 145, 33, 30),
        (0.354_838_7, 160, 33, 32),
        (0.387_096_76, 176, 34, 34),
        (0.419_354_83, 185, 47, 32),
        (0.451_612_9, 193, 59, 30),
        (0.483_870_95, 202, 71, 27),
        (0.516_129, 210, 82, 22),
        (0.548_387_1, 218, 93, 16),
        (0.580_645_14, 226, 104, 6),
        (0.612_903_24, 229, 118, 9),
        (0.645_161_3, 230, 132, 15),
        (0.677_419_36, 231, 146, 20),
        (0.709_677_4, 232, 159, 25),
        (0.741_935_5, 233, 172, 30),
        (0.774_193_5, 233, 185, 35),
        (0.806_451_6, 233, 198, 40),
        (0.838_709_65, 232, 211, 45),
        (0.870_967_75, 231, 223, 50),
        (0.903_225_8, 236, 233, 89),
        (0.935_483_9, 246, 240, 148),
        (0.967_741_9, 253, 247, 202),
        (1.0, 255, 255, 255),
    ]
}

fn inferno() -> Vec<(f32, i32, i32, i32)> {
    vec![
        (0.0, 0, 0, 4),
        (0.032_258_064, 4, 3, 19),
        (0.064_516_13, 11, 7, 37),
        (0.096_774_19, 22, 11, 57),
        (0.129_032_25, 34, 12, 76),
        (0.161_290_32, 49, 10, 92),
        (0.193_548_38, 63, 10, 102),
        (0.225_806_44, 77, 12, 107),
        (0.258_064_5, 90, 17, 110),
        (0.290_322_57, 103, 22, 110),
        (0.322_580_64, 116, 27, 110),
        (0.354_838_7, 129, 31, 108),
        (0.387_096_76, 142, 36, 105),
        (0.419_354_83, 155, 41, 100),
        (0.451_612_9, 168, 46, 95),
        (0.483_870_95, 181, 51, 88),
        (0.516_129, 193, 58, 81),
        (0.548_387_1, 204, 66, 72),
        (0.580_645_14, 215, 75, 63),
        (0.612_903_24, 224, 85, 54),
        (0.645_161_3, 232, 97, 44),
        (0.677_419_36, 239, 109, 34),
        (0.709_677_4, 245, 123, 23),
        (0.741_935_5, 248, 137, 12),
        (0.774_193_5, 251, 152, 6),
        (0.806_451_6, 252, 168, 13),
        (0.838_709_65, 251, 184, 28),
        (0.870_967_75, 249, 200, 48),
        (0.903_225_8, 246, 216, 71),
        (0.935_483_9, 242, 231, 99),
        (0.967_741_9, 243, 245, 133),
        (1.0, 252, 255, 164),
    ]
}

fn kindlmann() -> Vec<(f32, i32, i32, i32)> {
    vec![
        (0.0, 0, 0, 0),
        (0.032_258_064, 27, 1, 29),
        (0.064_516_13, 36, 3, 55),
        (0.096_774_19, 39, 4, 79),
        (0.129_032_25, 38, 5, 105),
        (0.161_290_32, 31, 6, 133),
        (0.193_548_38, 25, 8, 158),
        (0.225_806_44, 8, 21, 175),
        (0.258_064_5, 8, 46, 165),
        (0.290_322_57, 7, 64, 147),
        (0.322_580_64, 6, 78, 131),
        (0.354_838_7, 6, 90, 118),
        (0.387_096_76, 5, 101, 109),
        (0.419_354_83, 5, 112, 100),
        (0.451_612_9, 6, 122, 89),
        (0.483_870_95, 6, 132, 75),
        (0.516_129, 7, 142, 57),
        (0.548_387_1, 7, 152, 37),
        (0.580_645_14, 8, 162, 15),
        (0.612_903_24, 20, 172, 8),
        (0.645_161_3, 43, 181, 9),
        (0.677_419_36, 74, 188, 9),
        (0.709_677_4, 107, 195, 9),
        (0.741_935_5, 142, 200, 10),
        (0.774_193_5, 177, 203, 10),
        (0.806_451_6, 212, 205, 10),
        (0.838_709_65, 247, 205, 83),
        (0.870_967_75, 251, 211, 169),
        (0.903_225_8, 252, 221, 203),
        (0.935_483_9, 254, 232, 224),
        (0.967_741_9, 254, 244, 240),
        (1.0, 255, 255, 255),
    ]
}

fn extended_kindlmann() -> Vec<(f32, i32, i32, i32)> {
    vec![
        (0.0, 0, 0, 0),
        (0.032_258_064, 26, 1, 31),
        (0.064_516_13, 29, 3, 63),
        (0.096_774_19, 23, 4, 91),
        (0.129_032_25, 14, 5, 115),
        (0.161_290_32, 5, 28, 112),
        (0.193_548_38, 4, 47, 91),
        (0.225_806_44, 4, 59, 75),
        (0.258_064_5, 3, 69, 64),
        (0.290_322_57, 4, 79, 51),
        (0.322_580_64, 4, 89, 33),
        (0.354_838_7, 5, 98, 11),
        (0.387_096_76, 14, 107, 5),
        (0.419_354_83, 39, 114, 5),
        (0.451_612_9, 72, 120, 6),
        (0.483_870_95, 107, 123, 6),
        (0.516_129, 142, 123, 7),
        (0.548_387_1, 188, 115, 9),
        (0.580_645_14, 243, 90, 12),
        (0.612_903_24, 246, 104, 72),
        (0.645_161_3, 248, 118, 104),
        (0.677_419_36, 249, 128, 151),
        (0.709_677_4, 249, 138, 196),
        (0.741_935_5, 250, 146, 235),
        (0.774_193_5, 235, 166, 251),
        (0.806_451_6, 224, 185, 252),
        (0.838_709_65, 223, 199, 252),
        (0.870_967_75, 228, 211, 253),
        (0.903_225_8, 232, 223, 253),
        (0.935_483_9, 234, 235, 254),
        (0.967_741_9, 238, 247, 254),
        (1.0, 255, 255, 255),
    ]
}
