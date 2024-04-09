use super::channel_map::ChannelMapError;
use super::nuclear_data::MassError;
use super::shift_map::ShiftError;
use flate2::DecompressError;
use polars::error::PolarsError;
use std::error::Error;
use std::fmt::Display;

#[derive(Debug)]
pub enum EVBError {
    Compressor(DecompressError),
    Waves,
    File(std::io::Error),
    Parser,
    Channel(ChannelMapError),
    DataFrame(PolarsError),
    MassMap(MassError),
    ShiftMap(ShiftError),
    Sync,
}

impl From<std::io::Error> for EVBError {
    fn from(err: std::io::Error) -> EVBError {
        EVBError::File(err)
    }
}

impl From<DecompressError> for EVBError {
    fn from(err: DecompressError) -> EVBError {
        EVBError::Compressor(err)
    }
}

impl From<ChannelMapError> for EVBError {
    fn from(err: ChannelMapError) -> EVBError {
        EVBError::Channel(err)
    }
}

impl From<PolarsError> for EVBError {
    fn from(err: PolarsError) -> EVBError {
        EVBError::DataFrame(err)
    }
}

impl From<MassError> for EVBError {
    fn from(value: MassError) -> Self {
        EVBError::MassMap(value)
    }
}

impl From<ShiftError> for EVBError {
    fn from(value: ShiftError) -> Self {
        EVBError::ShiftMap(value)
    }
}

impl Display for EVBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EVBError::Compressor(x) => write!(f, "Run had a decompression error: {}", x),
            EVBError::Waves => write!(
                f,
                "Run found a file with waveform data, which is not supported!"
            ),
            EVBError::File(x) => write!(f, "Run had a file I/O error: {}", x),
            EVBError::Parser => write!(f, "Run had an error parsing the data from files"),
            EVBError::Channel(x) => {
                write!(f, "Run had an error occur with the channel map: {}", x)
            }
            EVBError::DataFrame(x) => write!(f, "Run had an error using polars: {}", x),
            EVBError::MassMap(x) => write!(f, "Run had an error with the mass data: {}", x),
            EVBError::ShiftMap(x) => write!(f, "Run had an error with the shift map: {}", x),
            EVBError::Sync => write!(f, "Run was unable to access shared progress resource"),
        }
    }
}

impl Error for EVBError {}
