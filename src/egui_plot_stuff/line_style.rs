use egui_plot::LineStyle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SerializableLineStyle {
    #[default]
    Solid,
    Dotted,
    Dashed,
}

impl SerializableLineStyle {
    pub fn to_egui(self, style_length: f32) -> LineStyle {
        match self {
            Self::Solid => LineStyle::Solid,
            Self::Dotted => LineStyle::Dotted {
                spacing: style_length.max(0.0),
            },
            Self::Dashed => LineStyle::Dashed {
                length: style_length.max(0.0),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SerializableLineStyle;
    use egui_plot::LineStyle;

    #[test]
    fn dotted_style_uses_latest_spacing() {
        assert_eq!(
            SerializableLineStyle::Dotted.to_egui(24.0),
            LineStyle::Dotted { spacing: 24.0 }
        );
    }

    #[test]
    fn dashed_style_uses_latest_length() {
        assert_eq!(
            SerializableLineStyle::Dashed.to_egui(18.0),
            LineStyle::Dashed { length: 18.0 }
        );
    }
}
