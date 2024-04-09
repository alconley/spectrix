use std::collections::HashMap;
use std::fmt::Display;
use std::num::ParseFloatError;
use std::num::ParseIntError;

use super::compass_data::generate_board_channel_uuid;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub struct ShiftMapEntry {
    pub board_number: u32,
    pub channel_number: u32,
    pub time_shift: f64, // Assuming time shift is a floating-point number; adjust the type as needed
}

#[derive(Debug)]
pub enum ShiftError {
    File(std::io::Error),
    Channel(ParseIntError),
    Timeshift(ParseFloatError),
}

impl Display for ShiftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShiftError::File(x) => write!(f, "ShiftMap had an IO error: {}", x),
            ShiftError::Channel(x) => {
                write!(f, "ShiftMap could not parse board/channel: {}", x)
            }
            ShiftError::Timeshift(x) => write!(f, "ShiftMap could not parse timeshift: {}", x),
        }
    }
}

impl From<std::io::Error> for ShiftError {
    fn from(value: std::io::Error) -> Self {
        ShiftError::File(value)
    }
}

impl From<ParseIntError> for ShiftError {
    fn from(value: ParseIntError) -> Self {
        ShiftError::Channel(value)
    }
}

impl From<ParseFloatError> for ShiftError {
    fn from(value: ParseFloatError) -> Self {
        ShiftError::Timeshift(value)
    }
}

impl std::error::Error for ShiftError {}

#[derive(Debug, Clone)]
pub struct ShiftMap {
    map: HashMap<u32, f64>,
}

impl ShiftMap {
    pub fn new(entries: Vec<ShiftMapEntry>) -> ShiftMap {
        let mut map = HashMap::new();
        for entry in entries {
            let id = generate_board_channel_uuid(&entry.board_number, &entry.channel_number);
            map.insert(id, entry.time_shift);
        }
        ShiftMap { map }
    }

    pub fn get_timeshift(&self, id: &u32) -> f64 {
        if let Some(value) = self.map.get(id) {
            *value
        } else {
            0.0
        }
    }
}
