use std::fs::File;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use log::info;
use polars::prelude::*;
use std::sync::{Arc, Mutex};
use tar::Archive;

use super::channel_data::ChannelData;
use super::channel_map::{Board, ChannelMap};
use super::compass_file::CompassFile;
use super::error::EVBError;
use super::event_builder::EventBuilder;
use super::kinematics::{calculate_weights, KineParameters};
use super::nuclear_data::MassMap;
use super::scaler_list::{ScalerEntryUI, ScalerList};
use super::shift_map::{ShiftMap, ShiftMapEntry};
use super::used_size::UsedSize;

//Maximum allowed size for a single dataframe: 8GB
const MAX_USED_SIZE: usize = 8_000_000_000;

#[derive(Debug)]
struct RunParams<'a> {
    pub run_archive_path: PathBuf,
    pub unpack_dir_path: PathBuf,
    pub output_file_path: PathBuf,
    pub scalerlist: Vec<ScalerEntryUI>,
    pub scalerout_file_path: PathBuf,
    pub nuc_map: &'a MassMap,
    pub channel_map: &'a ChannelMap,
    pub shift_map: &'a Option<ShiftMap>,
    pub coincidence_window: f64,
    pub run_number: i32,
}

fn clean_up_unpack_dir(unpack_dir: &Path) -> Result<(), EVBError> {
    for entry in unpack_dir.read_dir()?.flatten() {
        if entry.metadata()?.is_file() {
            std::fs::remove_file(entry.path())?;
        }
    }

    Ok(())
}

fn write_dataframe(data: ChannelData, filepath: &Path) -> Result<(), PolarsError> {
    info!("Writing dataframe to disk at {}", filepath.display());
    let columns: Vec<Series> = data.convert_to_series();
    let mut df = DataFrame::new(columns)?;
    let mut output_file = File::create(filepath)?;
    ParquetWriter::new(&mut output_file).finish(&mut df)?;
    Ok(())
}

fn write_dataframe_fragment(
    data: ChannelData,
    out_dir: &Path,
    run_number: &i32,
    frag_number: &i32,
) -> Result<(), PolarsError> {
    let frag_file_path = out_dir.join(format!("run_{}_{}.parquet", run_number, frag_number));
    write_dataframe(data, &frag_file_path)?;
    Ok(())
}

//Main function which processes a single run archive and writes the resulting event built data to parquet file
fn process_run(
    params: RunParams<'_>,
    k_params: &KineParameters,
    progress: Arc<Mutex<f32>>,
) -> Result<(), EVBError> {
    //Protective, ensure no loose files
    clean_up_unpack_dir(&params.unpack_dir_path)?;

    let archive_file = File::open(&params.run_archive_path)?;
    let mut decompressed_archive = Archive::new(GzDecoder::new(archive_file));
    decompressed_archive.unpack(&params.unpack_dir_path)?;

    let mut scaler_list = Some(ScalerList::new(params.scalerlist));

    //Collect all files from unpack, separate scalers from normal files
    let mut files: Vec<CompassFile<'_>> = vec![];
    let mut total_count: u64 = 0;
    for item in params.unpack_dir_path.read_dir()? {
        let filepath = &item?.path();
        match &mut scaler_list {
            Some(list) => {
                if list.read_scaler(filepath) {
                    continue;
                }
            }
            None => (),
        };

        files.push(CompassFile::new(filepath, params.shift_map)?);
        files.last_mut().unwrap().set_hit_used();
        files.last_mut().unwrap().get_top_hit()?;
        total_count += files.last().unwrap().get_number_of_hits();
    }

    let mut evb = EventBuilder::new(&params.coincidence_window);
    let mut analyzed_data = ChannelData::default();
    let x_weights = calculate_weights(k_params, params.nuc_map);

    let mut earliest_file_index: Option<usize>;

    let mut count: u64 = 0;
    let mut flush_count: u64 = 0;
    let flush_percent = 0.01;
    let flush_val: u64 = ((total_count as f64) * flush_percent) as u64;

    let mut frag_number = 0;

    loop {
        //Bulk of the work ... look for the earliest hit in the file collection
        earliest_file_index = Option::None;
        for i in 0..files.len() {
            if !files[i].is_eof() {
                let hit = files[i].get_top_hit()?;
                if hit.is_default() {
                    continue;
                }

                earliest_file_index = match earliest_file_index {
                    None => Some(i),
                    Some(index) => {
                        if hit.timestamp < files[index].get_top_hit()?.timestamp {
                            Some(i)
                        } else {
                            Some(index)
                        }
                    }
                };
            }
        }

        match earliest_file_index {
            None => break, //This is how we exit, no more hits to be found
            Some(i) => {
                //else we pop the earliest hit off to the event builder
                let hit = files[i].get_top_hit()?;
                evb.push_hit(hit);
                files[i].set_hit_used();
            }
        }

        if evb.is_event_ready() {
            analyzed_data.append_event(evb.get_ready_event(), params.channel_map, x_weights);
            //Check to see if we need to fragment
            if analyzed_data.get_used_size() > MAX_USED_SIZE {
                write_dataframe_fragment(
                    analyzed_data,
                    params.output_file_path.parent().unwrap(),
                    &params.run_number,
                    &frag_number,
                )?;
                //allocate new vector
                analyzed_data = ChannelData::default();
                frag_number += 1;
            }
        }

        //Progress report
        count += 1;
        if count == flush_val {
            flush_count += 1;
            count = 0;

            match progress.lock() {
                Ok(mut prog) => *prog = (flush_count as f64 * flush_percent) as f32,
                Err(_) => return Err(EVBError::Sync),
            };
        }
    }

    if frag_number == 0 {
        write_dataframe(analyzed_data, &params.output_file_path)?;
    } else {
        write_dataframe_fragment(
            analyzed_data,
            params.output_file_path.parent().unwrap(),
            &params.run_number,
            &frag_number,
        )?;
    }
    if let Some(list) = scaler_list {
        list.write_scalers(&params.scalerout_file_path)?
    }

    //To be safe, manually drop all files in unpack dir before deleting all the files
    drop(files);

    clean_up_unpack_dir(&params.unpack_dir_path)?;

    Ok(())
}

pub struct ProcessParams {
    pub archive_dir: PathBuf,
    pub unpack_dir: PathBuf,
    pub output_dir: PathBuf,
    pub channel_map: Vec<Board>,
    pub scaler_list: Vec<ScalerEntryUI>,
    pub shift_map: Vec<ShiftMapEntry>,
    pub coincidence_window: f64,
    pub run_min: i32,
    pub run_max: i32,
}

//Function which handles processing multiple runs, this is what the UI actually calls
pub fn process_runs(
    params: ProcessParams,
    k_params: KineParameters,
    progress: Arc<Mutex<f32>>,
) -> Result<(), EVBError> {
    let channel_map = ChannelMap::new(&params.channel_map);
    let mass_map = MassMap::new()?;
    let shift_map = ShiftMap::new(params.shift_map);

    for run in params.run_min..params.run_max {
        let local_params = RunParams {
            run_archive_path: params.archive_dir.join(format!("run_{}.tar.gz", run)),
            unpack_dir_path: params.unpack_dir.clone(),
            output_file_path: params.output_dir.join(format!("run_{}.parquet", run)),
            scalerlist: params.scaler_list.clone(),
            scalerout_file_path: params.output_dir.join(format!("run_{}_scalers.txt", run)),
            nuc_map: &mass_map,
            channel_map: &channel_map,
            shift_map: &Some(shift_map.clone()),
            coincidence_window: params.coincidence_window,
            run_number: run,
        };

        match progress.lock() {
            Ok(mut prog) => *prog = 0.0,
            Err(_) => return Err(EVBError::Sync),
        };

        //Skip over run if it doesnt exist
        if local_params.run_archive_path.exists() {
            process_run(local_params, &k_params, progress.clone())?;
        }
    }

    Ok(())
}
