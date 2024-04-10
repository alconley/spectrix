use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::ParseIntError;
use strum_macros::{AsRefStr, EnumIter};

use super::compass_data::generate_board_channel_uuid;

//Channels to be mapped in the ChannelMap, each variant is the verbatim keyword in the channel map
#[derive(Debug, Clone, Copy, PartialEq, AsRefStr, EnumIter, Serialize, Deserialize)]
pub enum ChannelType {
    //Detector fields -> can be channel mapped
    AnodeFront,
    AnodeBack,
    ScintLeft,
    ScintRight,
    Cathode,
    DelayFrontLeft,
    DelayFrontRight,
    DelayBackLeft,
    DelayBackRight,
    // make sure to update app.rs so the channel map combo box are updated

    //Invalid channel
    None,
}

impl ChannelType {
    fn default() -> Self {
        ChannelType::None // Default type
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub channels: [ChannelType; 16], // Each board has 16 channels
}

impl Default for Board {
    fn default() -> Self {
        Board {
            channels: [ChannelType::default(); 16], // Initialize all channels with the default type
        }
    }
}

#[derive(Debug)]
pub enum ChannelMapError {
    IOError(std::io::Error),
    ParseError(ParseIntError),
    // UnidentifiedChannelError
}

impl From<std::io::Error> for ChannelMapError {
    fn from(e: std::io::Error) -> Self {
        ChannelMapError::IOError(e)
    }
}

impl From<ParseIntError> for ChannelMapError {
    fn from(e: ParseIntError) -> Self {
        ChannelMapError::ParseError(e)
    }
}

impl std::fmt::Display for ChannelMapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelMapError::IOError(x) => {
                write!(f, "Channel map had an error with the input file: {}", x)
            }
            ChannelMapError::ParseError(x) => write!(
                f,
                "Channel map had an error parsing the channel map file: {}",
                x
            ),
            // ChannelMapError::UnidentifiedChannelError => write!(f, "Channel map found an unidentified field in the channel map file")
        }
    }
}

impl std::error::Error for ChannelMapError {}

#[derive(Debug, Clone)]
pub struct ChannelData {
    pub channel_type: ChannelType,
}

impl Default for ChannelData {
    fn default() -> Self {
        ChannelData {
            channel_type: ChannelType::None,
        }
    }
}

#[derive(Debug)]
pub struct ChannelMap {
    map: HashMap<u32, ChannelData>,
}

impl ChannelMap {
    pub fn new(boards: &[Board]) -> ChannelMap {
        let mut cmap = ChannelMap {
            map: HashMap::new(),
        };
        for (board_index, board) in boards.iter().enumerate() {
            for (channel_index, channel) in board.channels.iter().enumerate() {
                let data = ChannelData {
                    channel_type: *channel,
                };

                cmap.map.insert(
                    generate_board_channel_uuid(&(board_index as u32), &(channel_index as u32)),
                    data,
                );
            }
        }
        cmap
    }

    pub fn get_channel_data(&self, uuid: &u32) -> Option<&ChannelData> {
        return self.map.get(uuid);
    }
}
