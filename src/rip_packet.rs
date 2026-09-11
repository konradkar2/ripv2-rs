use std::net::SocketAddrV4;

use crate::result::{RIP_CMD_REQUEST, RipError, RipResult};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct RipHeader {
    pub command: u8,
    pub version: u8,
    pub padding: u16,
}
pub const RIP_HEADER_SIZE: usize = 4;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RipEntry {
    pub routing_family_id: u16,
    pub route_tag: u16,
    pub ip_address: u32,
    pub subnet_mask: u32,
    pub next_hop: u32,
    pub metric: u32,
}

pub const RIP_ENTRY_REQUEST: RipEntry = RipEntry {
    routing_family_id: 0,
    route_tag: 0,
    ip_address: 0,
    subnet_mask: 0,
    next_hop: 0,
    metric: 16,
};

#[derive(Debug, Clone)]
pub struct RipIfInfo {
    pub if_name: String,
    pub if_index: u32,
}

#[derive(Debug, Clone)]
pub struct RipPacket {
    pub data: RipPacketData,
    pub if_info: RipIfInfo,
    pub source: SocketAddrV4,
}

impl RipPacket {
    pub fn is_request(&self) -> bool {
        return self.data.is_request();
    }
}

#[derive(Debug, Clone)]
pub struct RipPacketData {
    pub header: RipHeader,
    pub entries: Vec<RipEntry>,
}

impl RipPacketData {
    pub fn from_slice(mut data: &[u8]) -> RipResult<RipPacketData> {
        if (data.len() - RIP_HEADER_SIZE) % RIP_ENTRY_SIZE != 0 {
            return Err(RipError::MalformedPacket());
        }

        let header = RipHeader::from_slice(&data[..RIP_HEADER_SIZE])?;
        data = &data[RIP_HEADER_SIZE..];

        let mut entries: Vec<RipEntry> = vec![];

        while data.len() > 0 {
            let entry = RipEntry::from_slice(&data[..RIP_ENTRY_SIZE])?;
            data = &data[RIP_ENTRY_SIZE..];
            entries.push(entry);
        }

        Ok(RipPacketData { header, entries })
    }

    pub fn is_request(&self) -> bool {
        return self.header.command == RIP_CMD_REQUEST
            && self.entries.len() == 1
            && self.entries[0] == RIP_ENTRY_REQUEST;
    }
}

impl RipHeader {
    pub fn from_slice(data: &[u8]) -> RipResult<Self> {
        if data.len() != RIP_HEADER_SIZE {
            return Err(RipError::MalformedPacket());
        }

        Ok(Self {
            command: data[0],
            version: data[1],
            padding: u16::from_be_bytes([data[2], data[3]]),
        })
    }
}

impl RipEntry {
    pub fn to_be(&mut self) {
        self.routing_family_id = self.routing_family_id.to_be();
        self.route_tag = self.route_tag.to_be();
        self.ip_address = self.ip_address.to_be();
        self.subnet_mask = self.subnet_mask.to_be();
        self.next_hop = self.next_hop.to_be();
        self.metric = self.metric.to_be();
    }

    pub fn from_slice(data: &[u8]) -> RipResult<Self> {
        if data.len() != RIP_ENTRY_SIZE {
            return Err(RipError::MalformedPacket());
        }

        Ok(Self {
            routing_family_id: u16::from_be_bytes([data[0], data[1]]),
            route_tag: u16::from_be_bytes([data[2], data[3]]),

            ip_address: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),

            subnet_mask: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),

            next_hop: u32::from_be_bytes([data[12], data[13], data[14], data[15]]),

            metric: u32::from_be_bytes([data[16], data[17], data[18], data[19]]),
        })
    }
}

pub const RIP_ENTRY_SIZE: usize = 20;

const _: [(); std::mem::size_of::<RipHeader>()] = [(); RIP_HEADER_SIZE];
const _: [(); std::mem::size_of::<RipEntry>()] = [(); RIP_ENTRY_SIZE];
