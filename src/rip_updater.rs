use std::io::{self};

use crate::common::*;
use crate::rip_socket::SocketPair;
use crate::ifc::*;
use std::{mem::size_of, slice};

fn as_bytes<T>(value: &T) -> &[u8] {
    unsafe {
        slice::from_raw_parts(
            (value as *const T).cast::<u8>(),
            size_of::<T>(),
        )
    }
}

pub struct RipUpdater {
    
}

impl RipUpdater {
    pub fn rip_send_request_multicast(sockets: &Vec<SocketPair>) -> io::Result<()> {
        let header = RipHeader {
            command: RIP_CMD_REQUEST,
            version: RIP_2_VERSION,
            padding: 0,
        };
        let entry = RipEntry {
            routing_family_id: 0,
            route_tag: 0,
            ip_address: 0,
            subnet_mask: 0,
            next_hop: 0,
            metric: 16u32.to_be(),
        };

        let mut buffer = vec![0; RIP_HEADER_SIZE + RIP_ENTRY_SIZE];
        buffer[..RIP_HEADER_SIZE].copy_from_slice(as_bytes(&header));
        buffer[RIP_HEADER_SIZE..].copy_from_slice(as_bytes(&entry));
        
        for socket_pair in sockets.iter() {
            socket_pair.tx.send_multicast(&buffer)?;
        }

        Ok(())
    }
}