use log::info;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::io::BufRead;
use std::path::PathBuf;

#[derive(Debug)]
pub enum MassError {
    NotFound,
    Parse,
    ParseInt(std::num::ParseIntError),
    ParseFloat(std::num::ParseFloatError),
}

impl From<std::io::Error> for MassError {
    fn from(_value: std::io::Error) -> Self {
        MassError::NotFound
    }
}

impl From<std::num::ParseIntError> for MassError {
    fn from(value: std::num::ParseIntError) -> Self {
        MassError::ParseInt(value)
    }
}

impl From<std::num::ParseFloatError> for MassError {
    fn from(value: std::num::ParseFloatError) -> Self {
        MassError::ParseFloat(value)
    }
}

impl Display for MassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MassError::NotFound => {
                write!(f, "Could not find and open amdc mass file!")
            }
            MassError::Parse => write!(f, "Unable to parse amdc mass file!"),
            MassError::ParseInt(e) => {
                write!(f, "Unable to parse amdc mass file with error {}", e)
            }
            MassError::ParseFloat(e) => {
                write!(f, "Unable to parse amdc mass file with error {}", e)
            }
        }
    }
}

impl Error for MassError {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NuclearData {
    pub z: u32,
    pub a: u32,
    pub mass: f64,
    pub isotope: String,
    pub element: String,
}

impl Default for NuclearData {
    fn default() -> Self {
        NuclearData {
            z: 0,
            a: 0,
            mass: 0.0,
            isotope: String::from("None"),
            element: String::from("None"),
        }
    }
}

fn generate_nucleus_id(z: &u32, a: &u32) -> u32 {
    if z >= a {
        z * z + z + a
    } else {
        a * a + z
    }
}

const U2MEV: f64 = 931.49410242;
const ELECTRON_MASS: f64 = 0.51099895000; //MeV

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MassMap {
    pub map: HashMap<u32, NuclearData>,
    pub file: PathBuf,
}

impl MassMap {
    pub fn new() -> Result<Self, MassError> {
        let mut map = MassMap {
            map: HashMap::new(),
            file: std::env::current_dir()?.join("etc").join("amdc_2016.txt"),
        };
        info!("Mass file: {:?}", map.file);
        map.init()?;
        Ok(map)
    }

    fn init(&mut self) -> Result<(), MassError> {
        let file = std::fs::File::open(&self.file)?;
        let mut reader = std::io::BufReader::new(file);
        let mut junk = String::new();
        reader.read_line(&mut junk)?;
        reader.read_line(&mut junk)?;
        let lines = reader.lines();

        for line in lines {
            match line {
                Ok(line_str) => {
                    let entries: Vec<&str> = line_str.split_whitespace().collect();
                    let mut data = NuclearData::default();
                    data.z = entries[1].parse()?;
                    data.a = entries[2].parse()?;
                    data.element = String::from(entries[3]);
                    data.isotope = format!("{}{}", data.a, data.element);
                    data.mass =
                        (entries[4].parse::<f64>()? + 1.0e-6 * entries[5].parse::<f64>()?) * U2MEV
                            - (data.z as f64) * ELECTRON_MASS;
                    self.map.insert(generate_nucleus_id(&data.z, &data.a), data);
                }
                Err(_) => return Err(MassError::Parse),
            };
        }

        Ok(())
    }

    pub fn get_data(&self, z: &u32, a: &u32) -> Option<&NuclearData> {
        self.map.get(&generate_nucleus_id(z, a))
    }
}
