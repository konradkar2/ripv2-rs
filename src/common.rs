#[derive(Debug)]
pub enum Error {
    InvalidConfiguration(String),
}

impl Error {
    pub fn to_string(&self) -> String {
        match self {
            Error::InvalidConfiguration(e) => format!("Invalid configuration: {}", e)
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
