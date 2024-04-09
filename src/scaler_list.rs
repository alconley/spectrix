use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::compass_file::CompassFile;

const INVALID_SCALER_PATTERN: &str = "InvalidScalerPattern";
const INVALID_SCALER_NAME: &str = "InvalidScaler";
const INVALID_SCALER_VALUE: u64 = 0;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub struct ScalerEntryUI {
    pub file_pattern: String,
    pub scaler_name: String,
}

#[derive(Debug, Clone)]
struct Scaler {
    pub file_pattern: String,
    pub name: String,
    pub value: u64,
}

impl Default for Scaler {
    fn default() -> Self {
        Scaler {
            file_pattern: INVALID_SCALER_PATTERN.to_string(),
            name: INVALID_SCALER_NAME.to_string(),
            value: INVALID_SCALER_VALUE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScalerList {
    list: Vec<Scaler>,
}

impl ScalerList {
    // Adjusted to take a vector of ScalerEntryUI
    pub fn new(entries: Vec<ScalerEntryUI>) -> ScalerList {
        let scalers = entries
            .into_iter()
            .map(|entry| {
                Scaler {
                    file_pattern: entry.file_pattern,
                    name: entry.scaler_name,
                    value: 0, // Assuming initial value is always 0
                }
            })
            .collect();

        ScalerList { list: scalers }
    }

    //Check if file is a scaler, read counts if yes
    pub fn read_scaler(&mut self, filepath: &Path) -> bool {
        for scaler in self.list.iter_mut() {
            match filepath.file_name() {
                Some(file_name) => {
                    if file_name
                        .to_str()
                        .expect("Could not parse file name at ScalerList::read_scaler")
                        .starts_with(&scaler.file_pattern)
                    {
                        if let Ok(compass_rep) = CompassFile::new(filepath, &None) {
                            scaler.value = compass_rep.get_number_of_hits();
                            return true;
                        }
                    } else {
                        continue;
                    }
                }
                None => continue,
            };
        }

        false
    }

    pub fn write_scalers(&self, filepath: &Path) -> Result<(), std::io::Error> {
        let file = File::create(filepath)?;
        let mut writer = BufWriter::new(file);

        // writer.write("SPS Scaler Data\n".as_bytes())?;
        // for scaler in &self.list {
        //     writer.write(format!("{} {}\n", scaler.name, scaler.value).as_bytes())?;
        // }

        writer.write_all("SPS Scaler Data\n".as_bytes())?;
        for scaler in &self.list {
            writer.write_all(format!("{} {}\n", scaler.name, scaler.value).as_bytes())?;
        }
        Ok(())
    }
}
