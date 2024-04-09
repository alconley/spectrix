use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

const RAW_BINARY: &str = "raw_binary";
const TEMP_BINARY: &str = "temp_binary";
const BUILT: &str = "built";
const SCALERS: &str = "scalers";

#[derive(Debug, Clone)]
pub enum WorkspaceError {
    ParentError,
    SubdirectoryError,
}

impl Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceError::ParentError => write!(
                f,
                "Parent directory given to workspace does not exist and could not be created!"
            ),
            WorkspaceError::SubdirectoryError => write!(
                f,
                "A required subdirectory in workspace does not exist and could not be created!"
            ),
        }
    }
}

impl Error for WorkspaceError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    parent_dir: PathBuf,
}

impl Workspace {
    pub fn new(parent: &Path) -> Result<Self, WorkspaceError> {
        if parent.exists() && parent.is_dir() {
            let ws = Workspace {
                parent_dir: parent.to_path_buf(),
            };
            ws.init_workspace()?;
            return Ok(Workspace {
                parent_dir: parent.to_path_buf(),
            });
        } else if !parent.exists() {
            match fs::create_dir_all(parent) {
                Ok(_) => {
                    return {
                        let ws = Workspace {
                            parent_dir: parent.to_path_buf(),
                        };
                        ws.init_workspace()?;
                        Ok(Workspace {
                            parent_dir: parent.to_path_buf(),
                        })
                    }
                }
                Err(_) => return Err(WorkspaceError::ParentError),
            };
        }
        Err(WorkspaceError::ParentError)
    }

    pub fn get_parent_str(&self) -> &str {
        match self.parent_dir.as_path().to_str() {
            Some(path) => path,
            None => "InvalidParent",
        }
    }

    pub fn get_archive_dir(&self) -> Result<PathBuf, WorkspaceError> {
        let archive_dir = self.parent_dir.join(RAW_BINARY);
        if archive_dir.exists() {
            Ok(archive_dir)
        } else {
            Err(WorkspaceError::SubdirectoryError)
        }
    }

    pub fn get_unpack_dir(&self) -> Result<PathBuf, WorkspaceError> {
        let unpack_dir = self.parent_dir.join(TEMP_BINARY);
        if unpack_dir.exists() {
            Ok(unpack_dir)
        } else {
            Err(WorkspaceError::SubdirectoryError)
        }
    }

    pub fn get_output_dir(&self) -> Result<PathBuf, WorkspaceError> {
        let output_dir = self.parent_dir.join(BUILT);
        if output_dir.exists() {
            Ok(output_dir)
        } else {
            Err(WorkspaceError::SubdirectoryError)
        }
    }

    fn init_workspace(&self) -> Result<(), WorkspaceError> {
        let raw_binary = self.parent_dir.join(RAW_BINARY);
        let temp_binary = self.parent_dir.join(TEMP_BINARY);
        let built = self.parent_dir.join(BUILT);
        let scalers = self.parent_dir.join(SCALERS);

        if !raw_binary.exists() {
            match fs::create_dir(&raw_binary) {
                Ok(_) => (),
                Err(_) => return Err(WorkspaceError::SubdirectoryError),
            };
        }

        if !temp_binary.exists() {
            match fs::create_dir(&temp_binary) {
                Ok(_) => (),
                Err(_) => return Err(WorkspaceError::SubdirectoryError),
            };
        }

        if !built.exists() {
            match fs::create_dir(&built) {
                Ok(_) => (),
                Err(_) => return Err(WorkspaceError::SubdirectoryError),
            };
        }

        if !scalers.exists() {
            match fs::create_dir(&scalers) {
                Ok(_) => (),
                Err(_) => return Err(WorkspaceError::SubdirectoryError),
            };
        }

        Ok(())
    }
}
