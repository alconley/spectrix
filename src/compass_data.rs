use super::shift_map::ShiftMap;
use bitflags::bitflags;
use rand::Rng;

bitflags! {
    #[derive(Debug)]
    pub struct CompassDataType: u16 {
        const ENERGY = 0x0001;
        const ENERGY_SHORT = 0x0004;
        const ENERGY_CALIBRATED = 0x0002;
        const WAVES = 0x0008;
        const ALL = Self::ENERGY.bits() | Self::ENERGY_SHORT.bits() | Self::ENERGY_CALIBRATED.bits() | Self::WAVES.bits();
        const NONE = 0x0000;
    }
}

#[derive(Debug, Clone)]
pub struct RawCompassData {
    pub board: u16,
    pub channel: u16,
    pub timestamp: u64,
    pub energy: u16,
    pub energy_calibrated: u64,
    pub energy_short: u16,
}

pub const fn generate_board_channel_uuid(board: &u32, channel: &u32) -> u32 {
    let b = *board;
    let c = *channel;
    if b >= c {
        b * b + b + c
    } else {
        c * c + b
    }
}

#[allow(dead_code)]
pub fn decompose_uuid_to_board_channel(uuid: &u32) -> (u32, u32) {
    let uuid_sqrt = (*uuid as f64).sqrt().floor() as u32;
    let test = uuid - uuid_sqrt * uuid_sqrt;
    if test >= uuid_sqrt {
        (uuid_sqrt, test - uuid_sqrt)
    } else {
        (test, uuid_sqrt)
    }
}

#[derive(Debug, Clone)]
pub struct CompassData {
    pub uuid: u32,
    pub energy: f64,
    pub energy_short: f64,
    pub timestamp: f64,
}

impl CompassData {
    pub fn new(raw: &RawCompassData, shifts: &Option<ShiftMap>) -> Self {
        let mut rng = rand::thread_rng();
        let board = raw.board as u32;
        let channel = raw.channel as u32;
        let id = generate_board_channel_uuid(&board, &channel);
        CompassData {
            uuid: id,
            energy: raw.energy as f64 + rng.gen::<f64>(),
            energy_short: raw.energy_short as f64 + rng.gen::<f64>(),
            timestamp: match shifts {
                Some(map) => raw.timestamp as f64 * 1.0e-3 + map.get_timeshift(&id),
                None => raw.timestamp as f64 * 1.0e-3,
            },
        }
    }

    pub fn is_default(&self) -> bool {
        self.timestamp == 0.0
    }

    #[allow(dead_code)]
    pub fn get_board_channel(&self) -> (u32, u32) {
        decompose_uuid_to_board_channel(&self.uuid)
    }
}

impl Default for CompassData {
    fn default() -> Self {
        CompassData {
            uuid: 0,
            energy: 0.0,
            energy_short: 0.0,
            timestamp: 0.0,
        }
    }
}
