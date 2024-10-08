use polars::prelude::*;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

pub struct LazyFramer {
    pub lazyframe: Option<LazyFrame>,
    pub columns: Vec<String>,
}

impl LazyFramer {
    pub fn new(files: Vec<PathBuf>) -> Self {
        let files_arc: Arc<[PathBuf]> = Arc::from(files);
        let args = ScanArgsParquet::default();
        log::info!("Files {:?}", files_arc);

        match LazyFrame::scan_parquet_files(files_arc, args) {
            Ok(lf) => {
                log::info!("Loaded Parquet files");
                let column_names = Self::get_column_names_from_lazyframe(&lf);

                Self {
                    lazyframe: Some(lf),
                    columns: column_names,
                }
            }
            Err(e) => {
                log::error!("Failed to load Parquet files: {}", e);
                Self {
                    lazyframe: None, // Indicates that loading failed
                    columns: Vec::new(),
                }
            }
        }
    }

    pub fn set_lazyframe(&mut self, lazyframe: LazyFrame) {
        self.lazyframe = Some(lazyframe);
    }

    pub fn get_column_names(&self) -> Vec<String> {
        self.columns.clone()
    }

    pub fn get_column_names_from_lazyframe(lazyframe: &LazyFrame) -> Vec<String> {
        let lf: LazyFrame = lazyframe.clone().limit(1);
        let df: DataFrame = lf.collect().unwrap();
        let columns: Vec<String> = df
            .get_column_names_owned()
            .into_iter()
            .map(|name| name.to_string())
            .collect();

        columns
    }

    pub fn add_column(&mut self, expr: Expr) {
        let lf = self.lazyframe.clone().unwrap().with_column(expr);
        self.lazyframe = Some(lf);
    }

    pub fn save_lazyframe(&mut self, output_path: &PathBuf, scan: bool) -> Result<(), PolarsError> {
        if let Some(ref lf) = self.lazyframe {
            if scan {
                // use sink parquet to save the LazyFrame to a single file
                // this ensures you can save the lazyframe to a single file even if it is larger than memory
                let options = ParquetWriteOptions::default();

                lf.clone().sink_parquet(output_path, options)?;
            } else {
                // use collect to save the LazyFrame to a single file
                // this will load the entire LazyFrame into memory
                let mut df = lf.clone().collect()?;

                // Open a file in write mode at the specified output path
                let file = File::create(output_path).map_err(|e| PolarsError::IO {
                    error: Arc::new(e),
                    msg: None,
                })?;

                // Write the filtered DataFrame to a Parquet file
                ParquetWriter::new(file)
                    .set_parallel(true)
                    .finish(&mut df)?;
            }
        } else {
            log::error!("LazyFrame is not loaded");
        }
        Ok(())
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("LazyFrame", |ui| {
            if ui.button("Save Current LazyFrame").clicked() {
                if let Some(_lf) = &self.lazyframe {
                    let output_path = rfd::FileDialog::new()
                        .add_filter("Parquet Files", &["parquet"])
                        .save_file();

                    if let Some(output_path) = output_path {
                        match self.save_lazyframe(&output_path, true) {
                            Ok(_) => {
                                log::info!("Saved LazyFrame to {:?}", output_path);
                            }
                            Err(e) => {
                                log::error!("Failed to save LazyFrame: {}", e);
                            }
                        }
                    }
                }
            }

            ui.label("Columns:");
            if self.columns.is_empty() {
                ui.label("No columns");
                if let Some(lf) = &self.lazyframe {
                    if ui.button("Get Columns").clicked() {
                        self.columns = Self::get_column_names_from_lazyframe(lf);
                    }
                }
            } else {
                for column in &self.columns {
                    ui.label(column);
                }
            }
        });
    }
}
