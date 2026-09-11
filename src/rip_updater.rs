use std::io::{self};
use std::net::{Ipv4Addr, SocketAddrV4};

use socket2 as s2;

use crate::result::*;
use crate::rip_database::RipDatabase;
use crate::rip_ifc::RipIfc;
use crate::rip_packet::*;
use crate::rip_socket::RipSocket;
use std::{mem::size_of, slice};

fn as_bytes<T>(value: &T) -> &[u8] {
    unsafe { slice::from_raw_parts((value as *const T).cast::<u8>(), size_of::<T>()) }
}

fn create_unicast_tx_socket(if_name: &str) -> io::Result<tokio::net::UdpSocket> {
    let socket = s2::Socket::new(s2::Domain::IPV4, s2::Type::DGRAM, Some(s2::Protocol::UDP))?;
    socket.bind_device(Some(if_name.as_bytes()))?;
    socket.set_reuse_port(true)?;

    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, RIP_UDP_PORT);
    socket.bind(&s2::SockAddr::from(addr))?;
    socket.set_nonblocking(true)?;

    let std_socket: std::net::UdpSocket = socket.into();
    tokio::net::UdpSocket::from_std(std_socket)
}

fn build_response_buffer(
    database: &RipDatabase,
    target_if_index: u32,
    changed_only: bool,
) -> Option<Vec<u8>> {
    let mut entries: Vec<RipEntry> = database
        .get_routes_for_advertisement(target_if_index, changed_only)
        .collect();

    if entries.is_empty() {
        return None;
    }

    let header = RipHeader {
        command: RIP_CMD_RESPONSE,
        version: RIP_2_VERSION,
        padding: 0,
    };

    let mut buffer = vec![0; RIP_HEADER_SIZE + entries.len() * RIP_ENTRY_SIZE];
    buffer[..RIP_HEADER_SIZE].copy_from_slice(as_bytes::<RipHeader>(&header));

    for (index, entry) in entries.iter_mut().enumerate() {
        entry.to_be();
        let offset = RIP_HEADER_SIZE + index * RIP_ENTRY_SIZE;
        buffer[offset..offset + RIP_ENTRY_SIZE].copy_from_slice(as_bytes::<RipEntry>(entry));
    }

    Some(buffer)
}

pub struct RipUpdater {}

impl RipUpdater {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn rip_send_request_multicast(socket: &RipSocket) -> io::Result<()> {
        let header = RipHeader {
            command: RIP_CMD_REQUEST,
            version: RIP_2_VERSION,
            padding: 0,
        };
        let mut entry = RIP_ENTRY_REQUEST;
        entry.to_be();

        let mut buffer = vec![0; RIP_HEADER_SIZE + RIP_ENTRY_SIZE];
        buffer[..RIP_HEADER_SIZE].copy_from_slice(as_bytes::<RipHeader>(&header));
        buffer[RIP_HEADER_SIZE..].copy_from_slice(as_bytes::<RipEntry>(&entry));

        socket.send_multicast(&buffer).await?;

        Ok(())
    }

    pub async fn rip_send_response_unicast(
        &self,
        database: &RipDatabase,
        target: &SocketAddrV4,
        target_if_info: &RipIfInfo,
    ) -> io::Result<()> {
        let changed_only = false;
        let Some(buffer) = build_response_buffer(database, target_if_info.if_index, changed_only)
        else {
            return Ok(());
        };

        let socket = create_unicast_tx_socket(&target_if_info.if_name)?;
        let sentn = socket.send_to(&buffer, *target).await?;
        if sentn != buffer.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "failed to send full RIP response",
            ));
        }

        Ok(())
    }

    pub async fn rip_send_advertisement_multicast(
        &self,
        database: &RipDatabase,
        interfaces: &[RipIfc],
        changed_only: bool,
    ) -> io::Result<()> {
        for ifc in interfaces {
            let Some(buffer) = build_response_buffer(database, ifc.tx.if_index, changed_only)
            else {
                continue;
            };

            ifc.tx.send_multicast(&buffer).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/rip_updater_tests.rs"]
mod tests;
