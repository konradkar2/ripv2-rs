#[derive(Debug)]
pub enum Error {
    InvalidConfiguration(String),
}

pub type Result<T> = std::result::Result<T, Error>;
