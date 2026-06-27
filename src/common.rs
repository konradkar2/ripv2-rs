use std::net::{SocketAddr};
use crate::ifc::RipPacket;

pub const RIP_CMD_REQUEST: u8= 1;
pub const RIP_CMD_RESPONSE: u8 = 2;
pub const RIP_2_VERSION: u8 = 2;
pub const RIP_MULTICAST_ADDR: &str = "224.0.0.9";
pub const RIP_UDP_PORT: u16 = 520;

#[derive(Debug)]
pub struct RipData {
    pub if_name: String,
    pub source_addr: SocketAddr,
    pub packet: RipPacket
}

#[derive(Debug)]
pub enum RipError {
    InvalidArgument(String),
    InvalidConfiguration(String),
    FailedToConfigureInterface{msg: String, if_name: String},
    IoError(String),
    MalformedPacket(),
    InvalidSourceAddress(),
}

impl RipError {
    pub fn to_string(&self) -> String {
        match self {
            RipError::InvalidArgument(e) => format!("Invalid argument: {}", e),
            RipError::InvalidConfiguration(e) => format!("Invalid configuration: {}", e),
            RipError::FailedToConfigureInterface{msg, if_name} => format!("Failed to configure interface: {}, if_name: {}", msg, if_name),
            RipError::IoError(e) => format!("IO error: {}", e),
            RipError::MalformedPacket() => "Malformed packet".to_string(),
            RipError::InvalidSourceAddress() => "Invalid Source Address".to_string(),
        }
    }
}

pub type RipResult<T> = std::result::Result<T, RipError>;
