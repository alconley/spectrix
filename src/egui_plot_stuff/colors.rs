use egui::Color32;

pub const COLOR_OPTIONS: &[(Color32, &str)] = &[
    (Color32::from_rgb(120, 47, 64), "FSU Garnet"), // go noles!
    (Color32::from_rgb(206, 184, 136), "FSU Gold"),
    (Color32::BLACK, "Black"),
    (Color32::DARK_GRAY, "Dark Gray"),
    (Color32::GRAY, "Gray"),
    (Color32::LIGHT_GRAY, "Light Gray"),
    (Color32::WHITE, "White"),
    (Color32::BROWN, "Brown"),
    (Color32::DARK_RED, "Dark Red"),
    (Color32::RED, "Red"),
    (Color32::LIGHT_RED, "Light Red"),
    (Color32::YELLOW, "Yellow"),
    (Color32::LIGHT_YELLOW, "Light Yellow"),
    (Color32::KHAKI, "Khaki"),
    (Color32::DARK_GREEN, "Dark Green"),
    (Color32::GREEN, "Green"),
    (Color32::LIGHT_GREEN, "Light Green"),
    (Color32::DARK_BLUE, "Dark Blue"),
    (Color32::BLUE, "Blue"),
    (Color32::LIGHT_BLUE, "Light Blue"),
];

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn from_color32(color: Color32) -> Self {
        Rgb {
            r: color.r(),
            g: color.g(),
            b: color.b(),
        }
    }

    pub fn to_color32(self) -> Color32 {
        Color32::from_rgb(self.r, self.g, self.b)
    }
}
