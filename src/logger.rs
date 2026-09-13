use std::path::PathBuf;

use flexi_logger::{FileSpec, Logger, LoggerHandle, WriteMode};

use crate::result::{RipError, RipResult};

const DEFAULT_LOG_LEVEL: &str = "info";

pub struct LoggerConfig {
    pub handle: LoggerHandle,
    pub log_file_path: Option<PathBuf>,
}

pub fn init_logger(log_file_path: Option<PathBuf>) -> RipResult<LoggerConfig> {
    let logger = Logger::try_with_env_or_str(DEFAULT_LOG_LEVEL)
        .map_err(|err| RipError::IoError(err.to_string()))?;

    match log_file_path {
        Some(log_file_path) => {
            let file_spec = FileSpec::try_from(log_file_path.clone())
                .map_err(|err| RipError::IoError(err.to_string()))?;
            let handle = logger
                .log_to_file(file_spec)
                .o_append(false)
                .write_mode(WriteMode::Direct)
                .start()
                .map_err(|err| RipError::IoError(err.to_string()))?;

            Ok(LoggerConfig {
                handle,
                log_file_path: Some(log_file_path),
            })
        }
        None => {
            let handle = logger
                .log_to_stderr()
                .start()
                .map_err(|err| RipError::IoError(err.to_string()))?;

            Ok(LoggerConfig {
                handle,
                log_file_path: None,
            })
        }
    }
}
