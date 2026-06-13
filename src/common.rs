#[derive(Debug)]
pub enum Error {
    InvalidArgument(String),
    InvalidConfiguration(String),
    OsError(String),
}

impl Error {
    pub fn to_string(&self) -> String {
        match self {
            Error::InvalidArgument(e) => format!("Invalid argument: {}", e),
            Error::InvalidConfiguration(e) => format!("Invalid configuration: {}", e),
            Error::OsError(e) => format!("OsError: {}", e),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
