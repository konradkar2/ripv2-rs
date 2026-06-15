pub const RIP_CMD_REQUEST: u8= 1;
pub const RIP_CMD_RESPONSE: u8 = 2;
pub const RIP_2_VERSION: u8 = 2;
pub const RIP_MULTICAST_ADDR: &str = "224.0.0.9";
pub const RIP_UDP_PORT: u16 = 520;

#[derive(Debug)]
pub enum Error {
    InvalidArgument(String),
    InvalidConfiguration(String),
    FailedToConfigureInterface{msg: String, if_name: String},
    IoError(String),
}

impl Error {
    pub fn to_string(&self) -> String {
        match self {
            Error::InvalidArgument(e) => format!("Invalid argument: {}", e),
            Error::InvalidConfiguration(e) => format!("Invalid configuration: {}", e),
            Error::FailedToConfigureInterface{msg, if_name} => format!("Failed to configure interface: {}, if_name: {}", msg, if_name),
            Error::IoError(e) => format!("IO error: {}", e),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
