// use super::configure_auxillary_detectors::AuxillaryDetectors;
use super::configure_lazyframes::{LazyFrameInfo, LazyFrames};
use super::manual_histogram_script::manual_add_histograms;

use crate::histoer::histogrammer::Histogrammer;
use polars::prelude::*;

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct HistogramScript {
    pub lazyframe_info: LazyFrameInfo,
}

impl HistogramScript {
    pub fn new() -> Self {
        Self {
            lazyframe_info: LazyFrameInfo::default(),
        }
    }

    pub fn get_lazyframe_info(&mut self) {
        let mut lazyframe_info = LazyFrameInfo::default();

        let lazyframes = LazyFrames::new();
        let main_columns = lazyframes.main_column_names();
        let main_lf_names = lazyframes.main_lfs_names();

        lazyframe_info.lfs = main_lf_names;
        lazyframe_info.columns = main_columns;

        // if self.add_auxillary_detectors {
        //     if let Some(auxillary_detectors) = &self.auxillary_detectors {
        //         let aux_columns = auxillary_detectors.get_column_names();
        //         let aux_lf_names = auxillary_detectors.get_lf_names();

        //         lazyframe_info.lfs.extend(aux_lf_names);
        //         lazyframe_info.columns.extend(aux_columns);
        //     }
        // }

        self.lazyframe_info = lazyframe_info;
    }


    pub fn add_histograms(&mut self, h: &mut Histogrammer, lf: LazyFrame) {
        manual_add_histograms(h, lf);
    }
}
