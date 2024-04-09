use super::channel_map::{ChannelMap, ChannelType};
#[allow(unused_imports)]
use super::compass_data::{decompose_uuid_to_board_channel, CompassData};
use super::used_size::UsedSize;
use std::collections::BTreeMap;
use std::hash::Hash;

use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumCount, EnumIter};

use polars::prelude::*;

const INVALID_VALUE: f64 = -1.0e6;

#[derive(Debug, Clone, Hash, Eq, PartialOrd, Ord, PartialEq, EnumIter, EnumCount, AsRefStr)]
pub enum ChannelDataField {
    AnodeFrontEnergy,
    AnodeFrontShort,
    AnodeFrontTime,
    AnodeBackEnergy,
    AnodeBackShort,
    AnodeBackTime,
    ScintLeftEnergy,
    ScintLeftShort,
    ScintLeftTime,
    ScintRightEnergy,
    ScintRightShort,
    ScintRightTime,
    CathodeEnergy,
    CathodeShort,
    CathodeTime,
    DelayFrontLeftEnergy,
    DelayFrontLeftShort,
    DelayFrontLeftTime,
    DelayFrontRightEnergy,
    DelayFrontRightShort,
    DelayFrontRightTime,
    DelayBackLeftEnergy,
    DelayBackLeftShort,
    DelayBackLeftTime,
    DelayBackRightEnergy,
    DelayBackRightShort,
    DelayBackRightTime,
    X1,
    X2,
    Xavg,
    Theta,
}

impl ChannelDataField {
    //Returns a list of fields for iterating over
    pub fn get_field_vec() -> Vec<ChannelDataField> {
        ChannelDataField::iter().collect()
    }
}

impl UsedSize for ChannelDataField {
    fn get_used_size(&self) -> usize {
        std::mem::size_of::<ChannelDataField>()
    }
}

#[derive(Debug, Clone)]
pub struct ChannelData {
    //Columns must always come in same order, so use sorted map
    pub fields: BTreeMap<ChannelDataField, Vec<f64>>,
    pub rows: usize,
}

impl Default for ChannelData {
    fn default() -> Self {
        let fields = ChannelDataField::get_field_vec();
        let mut data = ChannelData {
            fields: BTreeMap::new(),
            rows: 0,
        };
        fields.into_iter().for_each(|f| {
            data.fields.insert(f, vec![]);
        });
        data
    }
}

impl UsedSize for ChannelData {
    fn get_used_size(&self) -> usize {
        self.fields.get_used_size()
    }
}

impl ChannelData {
    //To keep columns all same length, push invalid values as necessary
    fn push_defaults(&mut self) {
        for field in self.fields.iter_mut() {
            if field.1.len() < self.rows {
                field.1.push(INVALID_VALUE)
            }
        }
    }

    //Update the last element to the given value
    fn set_value(&mut self, field: &ChannelDataField, value: f64) {
        if let Some(list) = self.fields.get_mut(field) {
            if let Some(back) = list.last_mut() {
                *back = value;
            }
        }
    }

    pub fn append_event(
        &mut self,
        event: Vec<CompassData>,
        map: &ChannelMap,
        weights: Option<(f64, f64)>,
    ) {
        self.rows += 1;
        self.push_defaults();

        let mut dfl_time = INVALID_VALUE;
        let mut dfr_time = INVALID_VALUE;
        let mut dbl_time = INVALID_VALUE;
        let mut dbr_time = INVALID_VALUE;

        for hit in event.iter() {
            //Fill out detector fields using channel map
            let channel_data = match map.get_channel_data(&hit.uuid) {
                Some(data) => data,
                None => continue,
            };
            match channel_data.channel_type {
                ChannelType::ScintLeft => {
                    self.set_value(&ChannelDataField::ScintLeftEnergy, hit.energy);
                    self.set_value(&ChannelDataField::ScintLeftShort, hit.energy_short);
                    self.set_value(&ChannelDataField::ScintLeftTime, hit.timestamp);
                }

                ChannelType::ScintRight => {
                    self.set_value(&ChannelDataField::ScintRightEnergy, hit.energy);
                    self.set_value(&ChannelDataField::ScintRightShort, hit.energy_short);
                    self.set_value(&ChannelDataField::ScintRightTime, hit.timestamp);
                }

                ChannelType::Cathode => {
                    self.set_value(&ChannelDataField::CathodeEnergy, hit.energy);
                    self.set_value(&ChannelDataField::CathodeShort, hit.energy_short);
                    self.set_value(&ChannelDataField::CathodeTime, hit.timestamp);
                }

                ChannelType::DelayFrontRight => {
                    self.set_value(&ChannelDataField::DelayFrontRightEnergy, hit.energy);
                    self.set_value(&ChannelDataField::DelayFrontRightShort, hit.energy_short);
                    self.set_value(&ChannelDataField::DelayFrontRightTime, hit.timestamp);
                    dfr_time = hit.timestamp;
                }

                ChannelType::DelayFrontLeft => {
                    self.set_value(&ChannelDataField::DelayFrontLeftEnergy, hit.energy);
                    self.set_value(&ChannelDataField::DelayFrontLeftShort, hit.energy_short);
                    self.set_value(&ChannelDataField::DelayFrontLeftTime, hit.timestamp);
                    dfl_time = hit.timestamp;
                }

                ChannelType::DelayBackRight => {
                    self.set_value(&ChannelDataField::DelayBackRightEnergy, hit.energy);
                    self.set_value(&ChannelDataField::DelayBackRightShort, hit.energy_short);
                    self.set_value(&ChannelDataField::DelayBackRightTime, hit.timestamp);
                    dbr_time = hit.timestamp;
                }

                ChannelType::DelayBackLeft => {
                    self.set_value(&ChannelDataField::DelayBackLeftEnergy, hit.energy);
                    self.set_value(&ChannelDataField::DelayBackLeftShort, hit.energy_short);
                    self.set_value(&ChannelDataField::DelayBackLeftTime, hit.timestamp);
                    dbl_time = hit.timestamp;
                }

                ChannelType::AnodeFront => {
                    self.set_value(&ChannelDataField::AnodeFrontEnergy, hit.energy);
                    self.set_value(&ChannelDataField::AnodeFrontShort, hit.energy_short);
                    self.set_value(&ChannelDataField::AnodeFrontTime, hit.timestamp);
                }

                ChannelType::AnodeBack => {
                    self.set_value(&ChannelDataField::AnodeBackEnergy, hit.energy);
                    self.set_value(&ChannelDataField::AnodeBackShort, hit.energy_short);
                    self.set_value(&ChannelDataField::AnodeBackTime, hit.timestamp);
                }

                _ => continue,
            }
        }

        //Physics
        let mut x1 = INVALID_VALUE;
        let mut x2 = INVALID_VALUE;
        if dfr_time != INVALID_VALUE && dfl_time != INVALID_VALUE {
            x1 = (dfl_time - dfr_time) * 0.5 * 1.0 / 2.1;
            self.set_value(&ChannelDataField::X1, x1);
        }
        if dbr_time != INVALID_VALUE && dbl_time != INVALID_VALUE {
            x2 = (dbl_time - dbr_time) * 0.5 * 1.0 / 1.98;
            self.set_value(&ChannelDataField::X2, x2);
        }
        if x1 != INVALID_VALUE && x2 != INVALID_VALUE {
            let diff = x2 - x1;
            if diff > 0.0 {
                self.set_value(&ChannelDataField::Theta, (diff / 36.0).atan());
            } else if diff < 0.0 {
                self.set_value(
                    &ChannelDataField::Theta,
                    std::f64::consts::PI + (diff / 36.0).atan(),
                );
            } else {
                self.set_value(&ChannelDataField::Theta, std::f64::consts::PI * 0.5);
            }

            match weights {
                Some(w) => self.set_value(&ChannelDataField::Xavg, w.0 * x1 + w.1 * x2),
                None => self.set_value(&ChannelDataField::Xavg, INVALID_VALUE),
            };
        }
    }

    pub fn convert_to_series(self) -> Vec<Series> {
        let sps_cols: Vec<Series> = self
            .fields
            .into_iter()
            .map(|field| -> Series { Series::new(field.0.as_ref(), field.1) })
            .collect();

        sps_cols
    }
}
